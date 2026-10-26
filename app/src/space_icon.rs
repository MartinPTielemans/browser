//! A Space's icon as the core keeps it: an emoji, or `glyph:` and the name of one of the icon
//! picker's glyphs, or nothing for the dot (docs/ux-spec.md › Spaces › Icon).

use unicode_segmentation::UnicodeSegmentation;

/// What a Space keeps before a glyph's name.
const GLYPH: &str = "glyph:";

/// The icon picker's glyphs in Arc's order, eight to a row: the name a Space keeps for each, and
/// Phosphor's drawing of it (NOTICE). The names are ours, so a glyph can be redrawn without
/// changing what Spaces keep.
pub const GLYPHS: [(&str, &str); 56] = [
    ("star", "icons/space/star-fill.svg"),
    ("bookmark", "icons/space/bookmark-simple-fill.svg"),
    ("heart", "icons/space/heart-fill.svg"),
    ("flag", "icons/space/flag-fill.svg"),
    ("bolt", "icons/space/lightning-fill.svg"),
    ("triangle", "icons/space/triangle-fill.svg"),
    ("asterisk", "icons/space/asterisk-bold.svg"),
    ("bell", "icons/space/bell-fill.svg"),
    ("lightbulb", "icons/space/lightbulb-fill.svg"),
    ("shapes", "icons/space/shapes-fill.svg"),
    ("squares", "icons/space/squares-four-fill.svg"),
    ("dots", "icons/space/dots-nine-bold.svg"),
    ("layers", "icons/space/stack-fill.svg"),
    ("database", "icons/space/database-fill.svg"),
    ("trays", "icons/space/cards-three-fill.svg"),
    ("two-squares", "icons/space/copy-fill.svg"),
    ("folder", "icons/space/folder-fill.svg"),
    ("tray", "icons/space/tray-fill.svg"),
    ("calendar", "icons/space/calendar-dots-fill.svg"),
    ("envelope", "icons/space/envelope-simple-fill.svg"),
    ("checkbox", "icons/space/check-square-fill.svg"),
    ("page", "icons/space/file-fill.svg"),
    ("book", "icons/space/book-open-fill.svg"),
    ("chat", "icons/space/chat-circle-dots-fill.svg"),
    ("people", "icons/space/users-fill.svg"),
    ("terminal", "icons/space/terminal-window-fill.svg"),
    ("tools", "icons/space/wrench-fill.svg"),
    ("square", "icons/space/square-fill.svg"),
    ("egg", "icons/space/egg-fill.svg"),
    ("circle", "icons/space/circle-fill.svg"),
    ("moon", "icons/space/moon-fill.svg"),
    ("sun", "icons/space/sun-fill.svg"),
    ("planet", "icons/space/planet-fill.svg"),
    ("leaf", "icons/space/leaf-fill.svg"),
    ("cloud", "icons/space/cloud-fill.svg"),
    ("paw", "icons/space/paw-print-fill.svg"),
    ("basket", "icons/space/basket-fill.svg"),
    ("gift", "icons/space/gift-fill.svg"),
    ("bed", "icons/space/bed-fill.svg"),
    ("fork-knife", "icons/space/fork-knife-fill.svg"),
    ("dumbbell", "icons/space/barbell-fill.svg"),
    ("airplane", "icons/space/airplane-tilt-fill.svg"),
    ("music", "icons/space/music-note-fill.svg"),
    ("palette", "icons/space/palette-fill.svg"),
    ("video", "icons/space/video-camera-fill.svg"),
    ("bandage", "icons/space/bandaids-fill.svg"),
    ("code", "icons/space/code-simple-bold.svg"),
    ("baseball", "icons/space/baseball-fill.svg"),
    ("cloud-outline", "icons/space/cloud-bold.svg"),
    ("map", "icons/space/map-trifold-fill.svg"),
    ("campfire", "icons/space/campfire-fill.svg"),
    ("pizza", "icons/space/pizza-fill.svg"),
    ("skull", "icons/space/skull-fill.svg"),
    ("receipt", "icons/space/receipt-fill.svg"),
    ("thumbs-up", "icons/space/thumbs-up-fill.svg"),
    ("train", "icons/space/train-simple-fill.svg"),
];

/// A Space's icon, read from what the core keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpaceIcon<'a> {
    /// The dot, for no icon, or a glyph this build doesn't have, such as one a later build added.
    Dot,
    Emoji(&'a str),
    /// The glyph at this index in `GLYPHS`.
    Glyph(usize),
}

impl<'a> SpaceIcon<'a> {
    pub fn parse(icon: &'a str) -> Self {
        match icon.strip_prefix(GLYPH) {
            Some(name) => GLYPHS
                .iter()
                .position(|(each, _)| *each == name)
                .map_or(SpaceIcon::Dot, SpaceIcon::Glyph),
            None if icon.is_empty() => SpaceIcon::Dot,
            None => SpaceIcon::Emoji(icon),
        }
    }

    /// The glyph's drawing, if it's a glyph.
    pub fn glyph_path(self) -> Option<&'static str> {
        match self {
            SpaceIcon::Glyph(index) => Some(GLYPHS[index].1),
            _ => None,
        }
    }
}

/// The first character typed or picked into an icon's field: a whole emoji, a flag or a family as
/// one, else nothing.
pub fn first_character(text: &str) -> &str {
    text.trim().graphemes(true).next().unwrap_or_default()
}

/// What a Space keeps for the glyph at `index` in `GLYPHS`.
pub fn glyph(index: usize) -> String {
    format!("{GLYPH}{}", GLYPHS[index].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_glyph_round_trips_through_what_a_space_keeps() {
        for index in 0..GLYPHS.len() {
            assert_eq!(SpaceIcon::parse(&glyph(index)), SpaceIcon::Glyph(index));
        }
        assert_eq!(glyph(4), "glyph:bolt");
    }

    #[test]
    fn an_unknown_glyph_draws_the_dot() {
        assert_eq!(SpaceIcon::parse("glyph:unicorn"), SpaceIcon::Dot);
        assert_eq!(SpaceIcon::parse("glyph:"), SpaceIcon::Dot);
        assert_eq!(SpaceIcon::parse(""), SpaceIcon::Dot);
    }

    #[test]
    fn anything_else_is_an_emoji() {
        assert_eq!(SpaceIcon::parse("🦊"), SpaceIcon::Emoji("🦊"));
    }

    /// The first character typed or picked is the icon, a whole emoji even when it's several
    /// code points; an emptied field is none, which takes the icon away.
    #[test]
    fn the_first_character_is_the_icon() {
        assert_eq!(first_character("🙂"), "🙂");
        assert_eq!(first_character("🙂🚀"), "🙂");
        assert_eq!(first_character("👨‍👩‍👧 family"), "👨‍👩‍👧");
        assert_eq!(first_character("🇳🇱🇯🇵"), "🇳🇱");
        assert_eq!(first_character("ab"), "a");
        assert_eq!(first_character(""), "");
        assert_eq!(first_character("  "), "");
    }

    #[test]
    fn every_name_is_kept_once() {
        let mut names: Vec<_> = GLYPHS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), GLYPHS.len());
    }
}
