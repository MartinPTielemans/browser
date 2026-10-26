//! The glyphs the UI draws, built into the app.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

/// The shell's glyphs, built in. Lucide's and, for Spaces' icons, Phosphor's (NOTICE), and the
/// loading ring and the window buttons' glyphs, which are ours.
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    ("icons/archive.svg", include_bytes!("../../assets/icons/archive.svg")),
    ("icons/arrow-down.svg", include_bytes!("../../assets/icons/arrow-down.svg")),
    (
        "icons/arrow-down-to-line.svg",
        include_bytes!("../../assets/icons/arrow-down-to-line.svg"),
    ),
    ("icons/arrow-left.svg", include_bytes!("../../assets/icons/arrow-left.svg")),
    ("icons/arrow-left-right.svg", include_bytes!("../../assets/icons/arrow-left-right.svg")),
    ("icons/arrow-right.svg", include_bytes!("../../assets/icons/arrow-right.svg")),
    ("icons/arrow-up.svg", include_bytes!("../../assets/icons/arrow-up.svg")),
    ("icons/brush-cleaning.svg", include_bytes!("../../assets/icons/brush-cleaning.svg")),
    ("icons/check.svg", include_bytes!("../../assets/icons/check.svg")),
    ("icons/chevron-down.svg", include_bytes!("../../assets/icons/chevron-down.svg")),
    ("icons/chevron-up.svg", include_bytes!("../../assets/icons/chevron-up.svg")),
    ("icons/chevrons-up-down.svg", include_bytes!("../../assets/icons/chevrons-up-down.svg")),
    ("icons/clock.svg", include_bytes!("../../assets/icons/clock.svg")),
    ("icons/code.svg", include_bytes!("../../assets/icons/code.svg")),
    ("icons/command.svg", include_bytes!("../../assets/icons/command.svg")),
    ("icons/cookie.svg", include_bytes!("../../assets/icons/cookie.svg")),
    ("icons/corner-down-left.svg", include_bytes!("../../assets/icons/corner-down-left.svg")),
    ("icons/ellipsis.svg", include_bytes!("../../assets/icons/ellipsis.svg")),
    ("icons/file.svg", include_bytes!("../../assets/icons/file.svg")),
    ("icons/fold-vertical.svg", include_bytes!("../../assets/icons/fold-vertical.svg")),
    ("icons/folder.svg", include_bytes!("../../assets/icons/folder.svg")),
    ("icons/folder-open.svg", include_bytes!("../../assets/icons/folder-open.svg")),
    ("icons/folder-plus.svg", include_bytes!("../../assets/icons/folder-plus.svg")),
    ("icons/globe.svg", include_bytes!("../../assets/icons/globe.svg")),
    ("icons/keyboard.svg", include_bytes!("../../assets/icons/keyboard.svg")),
    ("icons/library.svg", include_bytes!("../../assets/icons/library.svg")),
    ("icons/link.svg", include_bytes!("../../assets/icons/link.svg")),
    ("icons/loading.svg", include_bytes!("../../assets/icons/loading.svg")),
    ("icons/nav-back.svg", include_bytes!("../../assets/icons/nav-back.svg")),
    ("icons/nav-forward.svg", include_bytes!("../../assets/icons/nav-forward.svg")),
    ("icons/nav-plus.svg", include_bytes!("../../assets/icons/nav-plus.svg")),
    ("icons/nav-reload.svg", include_bytes!("../../assets/icons/nav-reload.svg")),
    ("icons/nav-sidebar.svg", include_bytes!("../../assets/icons/nav-sidebar.svg")),
    ("icons/nav-stop.svg", include_bytes!("../../assets/icons/nav-stop.svg")),
    ("icons/paintbrush.svg", include_bytes!("../../assets/icons/paintbrush.svg")),
    ("icons/panel-left.svg", include_bytes!("../../assets/icons/panel-left.svg")),
    ("icons/pencil.svg", include_bytes!("../../assets/icons/pencil.svg")),
    (
        "icons/picture-in-picture-2.svg",
        include_bytes!("../../assets/icons/picture-in-picture-2.svg"),
    ),
    ("icons/pin.svg", include_bytes!("../../assets/icons/pin.svg")),
    ("icons/printer.svg", include_bytes!("../../assets/icons/printer.svg")),
    ("icons/puzzle.svg", include_bytes!("../../assets/icons/puzzle.svg")),
    ("icons/rotate-ccw-clock.svg", include_bytes!("../../assets/icons/rotate-ccw-clock.svg")),
    ("icons/rotate-cw.svg", include_bytes!("../../assets/icons/rotate-cw.svg")),
    ("icons/save.svg", include_bytes!("../../assets/icons/save.svg")),
    ("icons/search.svg", include_bytes!("../../assets/icons/search.svg")),
    ("icons/settings.svg", include_bytes!("../../assets/icons/settings.svg")),
    (
        "icons/space/airplane-tilt-fill.svg",
        include_bytes!("../../assets/icons/space/airplane-tilt-fill.svg"),
    ),
    ("icons/space/asterisk-bold.svg", include_bytes!("../../assets/icons/space/asterisk-bold.svg")),
    ("icons/space/bandaids-fill.svg", include_bytes!("../../assets/icons/space/bandaids-fill.svg")),
    ("icons/space/barbell-fill.svg", include_bytes!("../../assets/icons/space/barbell-fill.svg")),
    ("icons/space/baseball-fill.svg", include_bytes!("../../assets/icons/space/baseball-fill.svg")),
    ("icons/space/basket-fill.svg", include_bytes!("../../assets/icons/space/basket-fill.svg")),
    ("icons/space/bed-fill.svg", include_bytes!("../../assets/icons/space/bed-fill.svg")),
    ("icons/space/bell-fill.svg", include_bytes!("../../assets/icons/space/bell-fill.svg")),
    (
        "icons/space/book-open-fill.svg",
        include_bytes!("../../assets/icons/space/book-open-fill.svg"),
    ),
    (
        "icons/space/bookmark-simple-fill.svg",
        include_bytes!("../../assets/icons/space/bookmark-simple-fill.svg"),
    ),
    (
        "icons/space/calendar-dots-fill.svg",
        include_bytes!("../../assets/icons/space/calendar-dots-fill.svg"),
    ),
    ("icons/space/campfire-fill.svg", include_bytes!("../../assets/icons/space/campfire-fill.svg")),
    (
        "icons/space/cards-three-fill.svg",
        include_bytes!("../../assets/icons/space/cards-three-fill.svg"),
    ),
    (
        "icons/space/chat-circle-dots-fill.svg",
        include_bytes!("../../assets/icons/space/chat-circle-dots-fill.svg"),
    ),
    (
        "icons/space/check-square-fill.svg",
        include_bytes!("../../assets/icons/space/check-square-fill.svg"),
    ),
    ("icons/space/circle-fill.svg", include_bytes!("../../assets/icons/space/circle-fill.svg")),
    ("icons/space/cloud-bold.svg", include_bytes!("../../assets/icons/space/cloud-bold.svg")),
    ("icons/space/cloud-fill.svg", include_bytes!("../../assets/icons/space/cloud-fill.svg")),
    (
        "icons/space/code-simple-bold.svg",
        include_bytes!("../../assets/icons/space/code-simple-bold.svg"),
    ),
    ("icons/space/copy-fill.svg", include_bytes!("../../assets/icons/space/copy-fill.svg")),
    ("icons/space/database-fill.svg", include_bytes!("../../assets/icons/space/database-fill.svg")),
    (
        "icons/space/dots-nine-bold.svg",
        include_bytes!("../../assets/icons/space/dots-nine-bold.svg"),
    ),
    ("icons/space/egg-fill.svg", include_bytes!("../../assets/icons/space/egg-fill.svg")),
    (
        "icons/space/envelope-simple-fill.svg",
        include_bytes!("../../assets/icons/space/envelope-simple-fill.svg"),
    ),
    ("icons/space/file-fill.svg", include_bytes!("../../assets/icons/space/file-fill.svg")),
    ("icons/space/flag-fill.svg", include_bytes!("../../assets/icons/space/flag-fill.svg")),
    ("icons/space/folder-fill.svg", include_bytes!("../../assets/icons/space/folder-fill.svg")),
    (
        "icons/space/fork-knife-fill.svg",
        include_bytes!("../../assets/icons/space/fork-knife-fill.svg"),
    ),
    ("icons/space/gift-fill.svg", include_bytes!("../../assets/icons/space/gift-fill.svg")),
    ("icons/space/heart-fill.svg", include_bytes!("../../assets/icons/space/heart-fill.svg")),
    ("icons/space/leaf-fill.svg", include_bytes!("../../assets/icons/space/leaf-fill.svg")),
    (
        "icons/space/lightbulb-fill.svg",
        include_bytes!("../../assets/icons/space/lightbulb-fill.svg"),
    ),
    (
        "icons/space/lightning-fill.svg",
        include_bytes!("../../assets/icons/space/lightning-fill.svg"),
    ),
    (
        "icons/space/map-trifold-fill.svg",
        include_bytes!("../../assets/icons/space/map-trifold-fill.svg"),
    ),
    ("icons/space/moon-fill.svg", include_bytes!("../../assets/icons/space/moon-fill.svg")),
    (
        "icons/space/music-note-fill.svg",
        include_bytes!("../../assets/icons/space/music-note-fill.svg"),
    ),
    ("icons/space/palette-fill.svg", include_bytes!("../../assets/icons/space/palette-fill.svg")),
    (
        "icons/space/paw-print-fill.svg",
        include_bytes!("../../assets/icons/space/paw-print-fill.svg"),
    ),
    ("icons/space/pizza-fill.svg", include_bytes!("../../assets/icons/space/pizza-fill.svg")),
    ("icons/space/planet-fill.svg", include_bytes!("../../assets/icons/space/planet-fill.svg")),
    ("icons/space/receipt-fill.svg", include_bytes!("../../assets/icons/space/receipt-fill.svg")),
    ("icons/space/shapes-fill.svg", include_bytes!("../../assets/icons/space/shapes-fill.svg")),
    ("icons/space/skull-fill.svg", include_bytes!("../../assets/icons/space/skull-fill.svg")),
    ("icons/space/square-fill.svg", include_bytes!("../../assets/icons/space/square-fill.svg")),
    (
        "icons/space/squares-four-fill.svg",
        include_bytes!("../../assets/icons/space/squares-four-fill.svg"),
    ),
    ("icons/space/stack-fill.svg", include_bytes!("../../assets/icons/space/stack-fill.svg")),
    ("icons/space/star-fill.svg", include_bytes!("../../assets/icons/space/star-fill.svg")),
    ("icons/space/sun-fill.svg", include_bytes!("../../assets/icons/space/sun-fill.svg")),
    (
        "icons/space/terminal-window-fill.svg",
        include_bytes!("../../assets/icons/space/terminal-window-fill.svg"),
    ),
    (
        "icons/space/thumbs-up-fill.svg",
        include_bytes!("../../assets/icons/space/thumbs-up-fill.svg"),
    ),
    (
        "icons/space/train-simple-fill.svg",
        include_bytes!("../../assets/icons/space/train-simple-fill.svg"),
    ),
    ("icons/space/tray-fill.svg", include_bytes!("../../assets/icons/space/tray-fill.svg")),
    ("icons/space/triangle-fill.svg", include_bytes!("../../assets/icons/space/triangle-fill.svg")),
    ("icons/space/users-fill.svg", include_bytes!("../../assets/icons/space/users-fill.svg")),
    (
        "icons/space/video-camera-fill.svg",
        include_bytes!("../../assets/icons/space/video-camera-fill.svg"),
    ),
    ("icons/space/wrench-fill.svg", include_bytes!("../../assets/icons/space/wrench-fill.svg")),
    ("icons/spaces.svg", include_bytes!("../../assets/icons/spaces.svg")),
    ("icons/sliders-horizontal.svg", include_bytes!("../../assets/icons/sliders-horizontal.svg")),
    (
        "icons/square-mouse-pointer.svg",
        include_bytes!("../../assets/icons/square-mouse-pointer.svg"),
    ),
    ("icons/terminal.svg", include_bytes!("../../assets/icons/terminal.svg")),
    ("icons/text-search.svg", include_bytes!("../../assets/icons/text-search.svg")),
    ("icons/trash.svg", include_bytes!("../../assets/icons/trash.svg")),
    ("icons/undo-2.svg", include_bytes!("../../assets/icons/undo-2.svg")),
    ("icons/user.svg", include_bytes!("../../assets/icons/user.svg")),
    ("icons/vibrate.svg", include_bytes!("../../assets/icons/vibrate.svg")),
    ("icons/volume-2.svg", include_bytes!("../../assets/icons/volume-2.svg")),
    ("icons/volume-x.svg", include_bytes!("../../assets/icons/volume-x.svg")),
    ("icons/window-close.svg", include_bytes!("../../assets/icons/window-close.svg")),
    ("icons/window-full-screen.svg", include_bytes!("../../assets/icons/window-full-screen.svg")),
    ("icons/window-minimize.svg", include_bytes!("../../assets/icons/window-minimize.svg")),
    ("icons/x.svg", include_bytes!("../../assets/icons/x.svg")),
    ("icons/zoom-in.svg", include_bytes!("../../assets/icons/zoom-in.svg")),
    ("icons/zoom-out.svg", include_bytes!("../../assets/icons/zoom-out.svg")),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|&(_, icon)| Cow::Borrowed(icon)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|&(name, _)| name.into())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::Assets;
    use gpui::AssetSource;

    /// A glyph that isn't built in draws nothing, with no error to say so.
    #[test]
    fn every_icon_the_ui_names_is_built_in() {
        let mut paths = vec![std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"))];
        let mut named = 0;
        while let Some(path) = paths.pop() {
            if path.is_dir() {
                let entries = std::fs::read_dir(&path).unwrap();
                paths.extend(entries.map(|entry| entry.unwrap().path()));
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (at, _) in text.match_indices("\"icons/") {
                let icon = text[at + 1..].split('"').next().unwrap_or_default();
                // Not this test's own pattern.
                if !icon.ends_with(".svg") {
                    continue;
                }
                assert!(Assets.load(icon).unwrap().is_some(), "{icon}, in {}", path.display());
                named += 1;
            }
        }
        assert!(named > 0);
    }
}
