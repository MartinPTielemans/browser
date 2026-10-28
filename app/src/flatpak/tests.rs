//! The updater against a fake of Flatpak's portal, served on a D-Bus connection of its own, which
//! says what Flatpak's would and notes what it's asked.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures_lite::future::{block_on, zip};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue, Value};

use super::*;

const MONITOR_PATH: &str = "/org/freedesktop/portal/Flatpak/update_monitor/test/1";

type Calls = Arc<Mutex<Vec<String>>>;

struct FakePortal {
    calls: Calls,
    /// Whether the monitor's Update fails, as with no network.
    failing: bool,
}

#[zbus::interface(name = "org.freedesktop.portal.Flatpak")]
impl FakePortal {
    async fn create_update_monitor(
        &self,
        _options: HashMap<String, OwnedValue>,
        #[zbus(object_server)] server: &zbus::ObjectServer,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        self.calls.lock().unwrap().push("CreateUpdateMonitor".into());
        let monitor = FakeMonitor { calls: self.calls.clone(), failing: self.failing };
        server.at(MONITOR_PATH, monitor).await?;
        Ok(OwnedObjectPath::try_from(MONITOR_PATH).unwrap())
    }

    async fn spawn(
        &self,
        cwd: Vec<u8>,
        argv: Vec<Vec<u8>>,
        fds: HashMap<u32, OwnedFd>,
        envs: HashMap<String, String>,
        flags: u32,
        _options: HashMap<String, OwnedValue>,
    ) -> u32 {
        let text = |bytes: &[u8]| {
            assert_eq!(bytes.last(), Some(&0), "each byte string ends with a NUL");
            String::from_utf8_lossy(&bytes[..bytes.len() - 1]).into_owned()
        };
        let argv: Vec<String> = argv.iter().map(|arg| text(arg)).collect();
        let call = format!("Spawn cwd={:?} flags={flags} fds={} envs={} {argv:?}", text(&cwd), fds.len(), envs.len());
        self.calls.lock().unwrap().push(call);
        4242
    }
}

struct FakeMonitor {
    calls: Calls,
    failing: bool,
}

#[zbus::interface(name = "org.freedesktop.portal.Flatpak.UpdateMonitor")]
impl FakeMonitor {
    /// Installs at once, as Flatpak's does over a while, telling how it went.
    async fn update(
        &self,
        parent_window: String,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.calls.lock().unwrap().push(format!("Update {parent_window:?}"));
        let status = |status: u32| HashMap::from([("status", Value::from(status))]);
        Self::progress(&emitter, status(RUNNING)).await?;
        if self.failing {
            let mut info = status(ERROR);
            info.insert("error_message", Value::from("Couldn't reach Flathub"));
            Self::progress(&emitter, info).await?;
        } else {
            Self::progress(&emitter, status(DONE)).await?;
        }
        Ok(())
    }

    #[zbus(signal)]
    async fn update_available(
        emitter: &SignalEmitter<'_>,
        info: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn progress(emitter: &SignalEmitter<'_>, info: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
}

/// The app's connection, and the fake's, on either end of a socket.
async fn connect(failing: bool, calls: Calls) -> (zbus::Connection, zbus::Connection) {
    let (ours, theirs) = std::os::unix::net::UnixStream::pair().unwrap();
    let portal = FakePortal { calls, failing };
    let server = zbus::connection::Builder::unix_stream(theirs)
        .server(zbus::Guid::generate())
        .unwrap()
        .p2p()
        .serve_at(PATH, portal)
        .unwrap()
        .build();
    let client = zbus::connection::Builder::unix_stream(ours).p2p().build();
    let (server, client) = zip(server, client).await;
    (client.unwrap(), server.unwrap())
}

/// Says the fake's monitor found `local` installed and `remote` on Flathub, against `running`.
async fn announce(fake: &zbus::Connection, running: &str, local: &str, remote: &str) {
    let monitor = fake.object_server().interface::<_, FakeMonitor>(MONITOR_PATH).await.unwrap();
    let info = HashMap::from([
        ("running-commit", Value::from(running)),
        ("local-commit", Value::from(local)),
        ("remote-commit", Value::from(remote)),
    ]);
    FakeMonitor::update_available(monitor.signal_emitter(), info).await.unwrap();
}

/// The states the updater went through after `announced`, until it settles or the fake is done.
fn states_after(failing: bool, announced: [&str; 3]) -> (Vec<State>, Vec<String>) {
    let calls = Calls::default();
    block_on(async {
        let (ours, fake) = connect(failing, calls.clone()).await;
        let (portal, events) = Portal::open(ours).await.unwrap();
        let states = Arc::new(Mutex::new(Vec::new()));
        let watching = {
            let states = states.clone();
            async move {
                watch(&portal, events, |state| states.lock().unwrap().push(state.clone())).await
            }
        };
        let [running, local, remote] = announced;
        let announcing = async {
            announce(&fake, running, local, remote).await;
            // A turn for the updater's Update and the fake's Progress, then the portal goes.
            let settled = |states: &Vec<State>| {
                matches!(states.last(), Some(State::Ready | State::Failed(_)))
            };
            for _ in 0..200 {
                if settled(&states.lock().unwrap()) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
                futures_lite::future::yield_now().await;
            }
            drop(fake);
        };
        zip(watching, announcing).await;
        let states = states.lock().unwrap().clone();
        (states, calls.lock().unwrap().clone())
    })
}

#[test]
fn a_build_on_flathub_installs_and_is_ready_for_a_restart() {
    let (states, calls) = states_after(false, ["a1", "a1", "b2"]);
    assert_eq!(states, [State::Installing, State::Ready]);
    assert_eq!(calls, ["CreateUpdateMonitor", r#"Update """#]);
}

#[test]
fn a_build_the_software_centre_installed_needs_only_a_restart() {
    let (states, calls) = states_after(false, ["a1", "b2", "b2"]);
    assert_eq!(states, [State::Ready]);
    assert_eq!(calls, ["CreateUpdateMonitor"], "nothing to install");
}

#[test]
fn a_failed_install_says_why() {
    let (states, _) = states_after(true, ["a1", "a1", "b2"]);
    assert_eq!(states, [State::Installing, State::Failed("Couldn't reach Flathub".into())]);
}

#[test]
fn a_restart_starts_the_newest_build_once_the_profile_is_free() {
    let calls = Calls::default();
    block_on(async {
        let (ours, _fake) = connect(false, calls.clone()).await;
        let (portal, _) = Portal::open(ours).await.unwrap();
        let argv = ["/app/ilo/chrome".to_string(), "--enable-features=X".into()];
        portal.restart(Path::new("/home/u/.var/app/ilo/SingletonLock"), &argv).await.unwrap();
    });
    let calls = calls.lock().unwrap();
    let spawn = calls.iter().find(|call| call.starts_with("Spawn")).expect("no Spawn");
    assert!(spawn.starts_with(r#"Spawn cwd="" flags=2 fds=0 envs=0 ["/bin/sh", "-c", "#), "{spawn}");
    assert!(
        spawn.ends_with(
            r#""sh", "/home/u/.var/app/ilo/SingletonLock", "/app/ilo/chrome", "--enable-features=X"]"#
        ),
        "{spawn}"
    );
}

#[test]
fn the_wait_for_the_profile_runs_the_build_with_its_arguments() {
    // The script Spawn runs, here with sh, a lock that goes and echo for the browser.
    let dir = std::env::temp_dir().join(format!("ilo-restart-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lock = dir.join("SingletonLock");
    _ = std::fs::remove_file(&lock);
    std::os::unix::fs::symlink("host-123", &lock).unwrap();
    let remover = {
        let lock = lock.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            std::fs::remove_file(lock).unwrap();
        })
    };
    let started = std::time::Instant::now();
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", RESTART, "sh", lock.to_str().unwrap(), "echo", "started", "a b"])
        .output()
        .unwrap();
    remover.join().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "started a b\n");
    assert!(started.elapsed() >= std::time::Duration::from_millis(300), "it waited for the lock");
    std::fs::remove_dir_all(dir).unwrap();
}
