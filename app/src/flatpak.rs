//! Updates on Linux, where ilo is the Flatpak on Flathub (docs/ux-spec.md › Updates): Flatpak's
//! portal says when Flathub has a newer build, ilo installs it beside the one running, without
//! waiting for the software centre, and offers to restart into it. Outside a Flatpak, as in a dev
//! build, there's no portal to ask, and no updates.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use futures_lite::StreamExt;
use gpui::{App, Global};
use zbus::zvariant::{Fd, OwnedObjectPath, OwnedValue, Value};

use crate::dialog::{self, Question, Tone};

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
/// What a restart runs in the newest build: `sh -c RESTART sh <lock> <command…>` waits for the
/// profile's lock to go, up to 30 s so one a crash left doesn't hold it back, as Chromium takes
/// over one whose process has gone, then runs the command.
const RESTART: &str = r#"lock=$1; shift; i=0
while [ -L "$lock" ] && [ $i -lt 300 ]; do sleep 0.1; i=$((i + 1)); done
exec "$@""#;

/// The Flatpak's command (`command:` in app/platform/linux/flatpak/app.ilo.browser.yml), which
/// starts chrome through cobalt and zypak. The running process's own argv starts /app/ilo/chrome,
/// which would come up without the sandbox zypak hands it.
const COMMAND: &str = "ilo";

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

    /// Starts the newest build through the Flatpak's command, once this one has let go of its
    /// profile, whose `lock` Chromium holds while it runs: a second copy would hand its launch to
    /// this one and end. This one quits next. Without this launch's arguments: cobalt adds its
    /// flags again, and the session brings back the pages a launch's URLs opened.
    pub async fn restart(&self, lock: &Path) -> zbus::Result<()> {
        let mut command = vec!["/bin/sh".to_string(), "-c".into(), RESTART.into(), "sh".into()];
        command.push(lock.to_string_lossy().into_owned());
        command.push(COMMAND.into());
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

/// Where the update stands as the user is told, once the portal has answered, and what Restart
/// runs: the portal's start of the newest build, and the quit.
#[derive(Default)]
struct Told {
    state: Option<State>,
    restart: Option<Rc<dyn Fn(&mut App)>>,
}

impl Global for Told {}

/// The portal answered: Restart runs `restart`.
pub fn on_restart(restart: impl Fn(&mut App) + 'static, cx: &mut App) {
    cx.default_global::<Told>().restart = Some(Rc::new(restart));
}

/// The update is at `state` now, which once a build is ready offers to restart into it.
pub fn changed(state: State, cx: &mut App) {
    let ready = state == State::Ready;
    cx.default_global::<Told>().state = Some(state);
    if ready {
        check(cx);
    }
}

/// Check for Updates…. Flatpak's portal can't be asked to look now, as it looks every half hour
/// by itself, so this says where the update stands, or offers the restart once a build is ready.
pub fn check(cx: &mut App) {
    let told = cx.default_global::<Told>();
    let question = question(told.state.as_ref());
    let restart = told.restart.clone();
    dialog::pose(
        question,
        Box::new(move |_, cx| {
            if let Some(restart) = restart {
                restart(cx)
            }
        }),
        cx,
    );
}

/// What Check for Updates… asks or says at `state`, None before the portal has answered.
pub fn question(state: Option<&State>) -> Question {
    let (message, detail) = match state {
        Some(State::Ready) => {
            return Question {
                action: "Restart".into(),
                cancel: "Later".into(),
                ..notice(
                    concat!("A new version of ", env!("APP_NAME"), " is ready"),
                    concat!("Restart ", env!("APP_NAME"), " to use it.").into(),
                )
            };
        }
        Some(State::Installing) => (
            concat!(env!("APP_NAME"), " is updating"),
            "It will offer to restart once the new version is in.".to_string(),
        ),
        Some(State::Failed(error)) => {
            (concat!(env!("APP_NAME"), " couldn’t update"), error.clone())
        }
        Some(State::Current) | None => (
            "No update yet",
            concat!(env!("APP_NAME"), " looks for one every half hour while it’s open.").into(),
        ),
    };
    notice(message, detail)
}

/// A dialog that only tells, with OK.
fn notice(message: &str, detail: String) -> Question {
    Question {
        icon: None,
        message: message.to_string().into(),
        detail: detail.into(),
        list: Vec::new(),
        action: "".into(),
        cancel: "OK".into(),
        tone: Tone::Accent,
        also: None,
        fields: Vec::new(),
        check: None,
        return_cancels: false,
    }
}

#[cfg(test)]
mod tests;
