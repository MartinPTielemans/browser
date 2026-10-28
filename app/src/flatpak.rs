//! Updates on Linux, where ilo is the Flatpak on Flathub (docs/ux-spec.md › Updates): Flatpak's
//! portal says when Flathub has a newer build, ilo installs it beside the one running, without
//! waiting for the software centre, and offers to restart into it. Outside a Flatpak, as in a dev
//! build, there's no portal to ask, and no updates.

use std::collections::HashMap;
use std::path::Path;

use futures_lite::StreamExt;
use zbus::zvariant::{Fd, OwnedObjectPath, OwnedValue, Value};

const PORTAL: &str = "org.freedesktop.portal.Flatpak";
const PATH: &str = "/org/freedesktop/portal/Flatpak";
const MONITOR: &str = "org.freedesktop.portal.Flatpak.UpdateMonitor";
/// Spawn's flag for the newest build installed, rather than the one running.
const LATEST_VERSION: u32 = 2;
// The update monitor's Progress statuses.
const RUNNING: u32 = 0;
const EMPTY: u32 = 1;
const DONE: u32 = 2;
const ERROR: u32 = 3;
/// What a restart runs in the newest build: `sh -c RESTART sh <lock> <argv…>` waits for the
/// profile's lock to go, up to 30 s so one a crash left doesn't hold it back, as Chromium takes
/// over one whose process has gone, then runs argv.
const RESTART: &str = r#"lock=$1; shift; i=0
while [ -L "$lock" ] && [ $i -lt 300 ]; do sleep 0.1; i=$((i + 1)); done
exec "$@""#;

/// Whether this is the Flatpak, which Flatpak marks with this file.
pub fn in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// Where the update stands, as the user is told of it.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// None found.
    Current,
    Installing,
    /// Installed beside the running build, for the next launch.
    Ready,
    Failed(String),
}

/// What the portal says.
#[derive(Debug, PartialEq)]
pub enum Event {
    /// Flatpak found a build: `local` installed and `remote` on Flathub, against the running one.
    Available { running: String, local: String, remote: String },
    Progress { status: u32, error: String },
}

impl State {
    /// The state after `event`, and whether to install the build it announces.
    pub fn after(&self, event: &Event) -> (State, bool) {
        match event {
            // The software centre has installed it already, so a restart is all it needs.
            Event::Available { running, local, .. } if local != running => (State::Ready, false),
            Event::Available { local, remote, .. } if remote != local => (State::Installing, true),
            Event::Available { .. } => (self.clone(), false),
            Event::Progress { status: RUNNING, .. } => (State::Installing, false),
            Event::Progress { status: EMPTY, .. } if *self == State::Installing => {
                (State::Current, false)
            }
            Event::Progress { status: DONE, .. } => (State::Ready, false),
            Event::Progress { status: ERROR, error } => (State::Failed(error.clone()), false),
            Event::Progress { .. } => (self.clone(), false),
        }
    }
}

impl Event {
    fn read(message: &zbus::Message) -> Option<Event> {
        let header = message.header();
        let info: HashMap<String, OwnedValue> = message.body().deserialize().ok()?;
        let text = |key: &str| {
            info.get(key).and_then(|value| value.downcast_ref::<&str>().ok()).unwrap_or("").to_string()
        };
        match header.member()?.as_str() {
            "UpdateAvailable" => Some(Event::Available {
                running: text("running-commit"),
                local: text("local-commit"),
                remote: text("remote-commit"),
            }),
            "Progress" => Some(Event::Progress {
                status: info.get("status")?.downcast_ref::<u32>().ok()?,
                error: text("error_message"),
            }),
            _ => None,
        }
    }
}

/// The portal's update monitor for this instance.
#[derive(Clone)]
pub struct Portal {
    connection: zbus::Connection,
    monitor: OwnedObjectPath,
}

/// What the update monitor says, from before anything could have been said.
pub struct Events(zbus::MessageStream);

impl Portal {
    /// The session's portal, which in a Flatpak is Flatpak's.
    pub async fn open_session() -> zbus::Result<(Portal, Events)> {
        Portal::open(zbus::Connection::session().await?).await
    }

    pub async fn open(connection: zbus::Connection) -> zbus::Result<(Portal, Events)> {
        let options: HashMap<&str, Value> = HashMap::new();
        let reply = connection
            .call_method(Some(PORTAL), PATH, Some(PORTAL), "CreateUpdateMonitor", &(options,))
            .await?;
        let monitor: OwnedObjectPath = reply.body().deserialize()?;
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(MONITOR)?
            .path(monitor.clone())?
            .build();
        let events = zbus::MessageStream::for_match_rule(rule, &connection, None).await?;
        Ok((Portal { connection, monitor }, Events(events)))
    }

    /// Installs the newest build beside the running one; Progress says how it goes.
    pub async fn update(&self) -> zbus::Result<()> {
        let options: HashMap<&str, Value> = HashMap::new();
        let monitor = self.monitor.as_ref();
        self.connection.call_method(Some(PORTAL), monitor, Some(MONITOR), "Update", &("", options)).await?;
        Ok(())
    }

    /// Starts the newest build with `argv`, once this one has let go of its profile, whose
    /// `lock` Chromium holds while it runs: a second copy would hand its launch to this one and
    /// end. This one quits next.
    pub async fn restart(&self, lock: &Path, argv: &[String]) -> zbus::Result<()> {
        let mut command = vec!["/bin/sh".to_string(), "-c".into(), RESTART.into(), "sh".into()];
        command.push(lock.to_string_lossy().into_owned());
        command.extend(argv.iter().cloned());
        // Byte strings, ended with a NUL, as the portal reads them.
        let bytes = |text: &str| [text.as_bytes(), b"\0"].concat();
        let argv: Vec<Vec<u8>> = command.iter().map(|arg| bytes(arg)).collect();
        let fds: HashMap<u32, Fd> = HashMap::new();
        let envs: HashMap<&str, &str> = HashMap::new();
        let options: HashMap<&str, Value> = HashMap::new();
        let body = (bytes(""), argv, fds, envs, LATEST_VERSION, options);
        self.connection.call_method(Some(PORTAL), PATH, Some(PORTAL), "Spawn", &body).await?;
        Ok(())
    }
}

/// Follows the update monitor until the portal goes, installing each build it announces, and
/// tells `changed` each new state.
pub async fn watch(portal: &Portal, mut events: Events, mut changed: impl FnMut(&State)) {
    let mut state = State::Current;
    while let Some(message) = events.0.next().await {
        let Some(event) = message.ok().as_ref().and_then(Event::read) else {
            continue;
        };
        let (next, install) = state.after(&event);
        if next != state {
            state = next;
            changed(&state);
        }
        if install && let Err(error) = portal.update().await {
            state = State::Failed(error.to_string());
            changed(&state);
        }
    }
}

#[cfg(test)]
mod tests;
