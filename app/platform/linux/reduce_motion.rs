//! Linux's Reduce Motion: the desktop's own setting, read through the settings portal, which a
//! Flatpak reaches too, as the Mac's and Windows' hosts read theirs (host_reduce_motion). GNOME
//! keeps it as animations turned off, KDE as an animation speed of 0. A desktop with neither, or no
//! portal, leaves the motion on.

use gpui::App;
use zbus::zvariant::OwnedValue;

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const SETTINGS: &str = "org.freedesktop.portal.Settings";

/// Reads the setting away from the UI and gives GPUI the answer once it comes.
pub fn read(cx: &mut App) {
    cx.spawn(async move |cx| {
        if let Some(reduce) = reduced().await {
            cx.update(|cx| crate::set_reduce_motion(reduce, cx));
        }
    })
    .detach();
}

async fn reduced() -> Option<bool> {
    let connection = zbus::Connection::session().await.ok()?;
    let read = async |namespace: &str, key: &str| -> Option<OwnedValue> {
        let body = &(namespace, key);
        let reply = connection.call_method(Some(PORTAL), PATH, Some(SETTINGS), "ReadOne", body);
        reply.await.ok()?.body().deserialize::<OwnedValue>().ok()
    };
    if let Some(on) = read("org.gnome.desktop.interface", "enable-animations").await {
        return bool::try_from(on).ok().map(|on| !on);
    }
    let speed = read("org.kde.kdeglobals.KDE", "AnimationDurationFactor").await?;
    f64::try_from(speed).ok().map(|factor| factor == 0.)
}
