//! The icon picker's Emoji tab: the emoji it lists, in Unicode's order under their groups after the
//! ones used last, the search that narrows them, and the numbers its layout is drawn from. The
//! layout is *interim* until Arc's Emoji tab is captured (docs/ux-spec.md › Spaces › Icon), and
//! these are the numbers a capture changes. The columns, pitch and plates are the Icon tab's.

use std::ops::Range;

use emojis::{Emoji, Group, SkinTone};

use super::COLUMNS;

/// The search field, 44 under the card's top, a segment control's height and the grid's width.
pub const SEARCH_TOP: f32 = 44.;
pub const SEARCH_HEIGHT: f32 = 24.;
pub const SEARCH_RADIUS: f32 = 6.;
pub const SEARCH_PADDING: f32 = 8.;
/// The list starts 6 under the field and ends 6 over the card's bottom, so its rows stop short of
/// the corners' curve.
pub const LIST_TOP: f32 = SEARCH_TOP + SEARCH_HEIGHT + 6.;
pub const LIST_BOTTOM: f32 = 6.;
pub const EMOJI_SIZE: f32 = 20.;
/// A group's header takes a row, its text on the plates' left edge and the row's bottom.
pub const HEADER_TEXT: f32 = 11.;
pub const HEADER_LEFT: f32 = 4.;
pub const HEADER_BOTTOM: f32 = 3.;
/// How many emoji the picker remembers as used last.
pub const RECENTS: usize = 16;

/// The emoji picked last, newest first, kept as one emoji to a line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recents(Vec<String>);

impl Recents {
    /// Reads what `to_text` wrote, leaving out what the table doesn't know.
    pub fn parse(text: &str) -> Self {
        let each = text.lines().map(str::trim).filter(|line| emojis::get(line).is_some());
        Recents(each.take(RECENTS).map(str::to_string).collect())
    }

    pub fn to_text(&self) -> String {
        self.0.iter().map(|emoji| format!("{emoji}\n")).collect()
    }

    /// Puts `emoji` first, once.
    pub fn push(&mut self, emoji: &str) {
        self.0.retain(|each| each != emoji);
        self.0.insert(0, emoji.to_string());
        self.0.truncate(RECENTS);
    }

    fn emojis(&self) -> impl Iterator<Item = &'static Emoji> {
        self.0.iter().filter_map(|emoji| emojis::get(emoji))
    }
}

/// One row of the list: a header, or up to `COLUMNS` emoji, as a range of the grid's items.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Header(&'static str),
    Cells(Range<usize>),
}

/// What the Emoji tab lists for a search: its emoji in order, and the rows they fall in.
#[derive(Default)]
pub struct Grid {
    pub items: Vec<&'static Emoji>,
    pub rows: Vec<Row>,
}

impl Grid {
    /// The recently used, then every group in Unicode's order, for no search; else the emoji whose
    /// name or shortcode has each word searched for, with no headers.
    pub fn new(query: &str, recents: &Recents) -> Self {
        let mut grid = Grid { items: Vec::new(), rows: Vec::new() };
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            grid.section("Recently Used", recents.emojis());
            for group in Group::iter() {
                grid.section(group_name(group), group.emojis().filter(|emoji| is_base(emoji)));
            }
        } else {
            let found = emojis::iter().filter(|emoji| is_base(emoji) && matches(emoji, &words));
            grid.section("", found);
        }
        grid
    }

    fn section(&mut self, title: &'static str, emojis: impl Iterator<Item = &'static Emoji>) {
        let start = self.items.len();
        self.items.extend(emojis);
        if self.items.len() == start {
            return;
        }
        if !title.is_empty() {
            self.rows.push(Row::Header(title));
        }
        for row in (start..self.items.len()).step_by(COLUMNS) {
            self.rows.push(Row::Cells(row..(row + COLUMNS).min(self.items.len())));
        }
    }

    /// The first item that is `emoji`.
    pub fn position(&self, emoji: &str) -> Option<usize> {
        self.items.iter().position(|each| each.as_str() == emoji)
    }

    /// The row `item` is in.
    pub fn row_of(&self, item: usize) -> Option<usize> {
        self.rows.iter().position(|row| matches!(row, Row::Cells(cells) if cells.contains(&item)))
    }

    /// The item a step from `from` lands on: along the items for `across`, which goes on to the
    /// next row at a row's end, or down and up for `down`, keeping the column as far as the row
    /// goes. `from` none steps onto the first item. Stays where it is at either end.
    pub fn step(&self, from: Option<usize>, across: isize, down: isize) -> Option<usize> {
        let Some(from) = from else {
            return (!self.items.is_empty()).then_some(0);
        };
        let last = self.items.len().checked_sub(1)?;
        if across != 0 {
            return Some(from.saturating_add_signed(across).min(last));
        }
        let row = self.row_of(from)?;
        let Row::Cells(here) = &self.rows[row] else {
            return Some(from);
        };
        let column = from - here.start;
        let mut rows: Box<dyn Iterator<Item = &Row>> = if down > 0 {
            Box::new(self.rows[row + 1..].iter())
        } else {
            Box::new(self.rows[..row].iter().rev())
        };
        match rows.find_map(|row| match row {
            Row::Cells(cells) => Some(cells),
            Row::Header(_) => None,
        }) {
            Some(there) => Some((there.start + column).min(there.end - 1)),
            None => Some(from),
        }
    }
}

/// Emoji with no skin tone to pick, or in the one every emoji starts as; the tones come later.
fn is_base(emoji: &Emoji) -> bool {
    emoji.skin_tone().is_none_or(|tone| tone == SkinTone::Default)
}

/// Whether each of `words` (lowercase) is in the emoji's name or one of its shortcodes.
fn matches(emoji: &Emoji, words: &[String]) -> bool {
    let name = emoji.name().to_lowercase();
    words.iter().all(|word| {
        name.contains(word.as_str())
            || emoji.shortcodes().any(|code| code.replace('_', " ").contains(word.as_str()))
    })
}

fn group_name(group: Group) -> &'static str {
    match group {
        Group::SmileysAndEmotion => "Smileys & Emotion",
        Group::PeopleAndBody => "People & Body",
        Group::AnimalsAndNature => "Animals & Nature",
        Group::FoodAndDrink => "Food & Drink",
        Group::TravelAndPlaces => "Travel & Places",
        Group::Activities => "Activities",
        Group::Objects => "Objects",
        Group::Symbols => "Symbols",
        Group::Flags => "Flags",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_groups_come_in_unicodes_order_after_the_recents() {
        let recents = Recents::parse("🦊\n🌱\n");
        let grid = Grid::new("", &recents);
        let headers: Vec<_> = grid
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::Header(title) => Some(*title),
                Row::Cells(_) => None,
            })
            .collect();
        assert_eq!(headers[..3], ["Recently Used", "Smileys & Emotion", "People & Body"]);
        assert_eq!(headers.last(), Some(&"Flags"));
        assert_eq!(grid.items[0].as_str(), "🦊");
        assert_eq!(grid.items[1].as_str(), "🌱");
        // 😀 grinning face, Unicode's first, is the first of the Smileys.
        assert_eq!(grid.items[2].as_str(), "😀");
    }

    #[test]
    fn no_recents_leave_no_header() {
        let grid = Grid::new("", &Recents::default());
        assert_eq!(grid.rows[0], Row::Header("Smileys & Emotion"));
    }

    #[test]
    fn the_grid_has_no_skin_tones_and_no_components() {
        let grid = Grid::new("", &Recents::default());
        assert!(grid.items.len() > 1_500, "{}", grid.items.len());
        assert!(grid.position("👍").is_some());
        assert!(grid.position("👍🏽").is_none());
        assert!(grid.items.iter().all(|emoji| !emoji.name().contains("skin tone")));
    }

    #[test]
    fn a_search_matches_names_and_shortcodes() {
        let grid = Grid::new("fox", &Recents::default());
        assert_eq!(grid.items[0].as_str(), "🦊");
        let grid = Grid::new("  GRINNING  face ", &Recents::default());
        assert_eq!(grid.items[0].as_str(), "😀");
        // A shortcode that isn't in the name.
        let grid = Grid::new("+1", &Recents::default());
        assert!(grid.position("👍").is_some());
        assert!(Grid::new("zzzzzz", &Recents::default()).items.is_empty());
        // Found emoji have no headers.
        assert!(grid.rows.iter().all(|row| matches!(row, Row::Cells(_))));
    }

    #[test]
    fn rows_hold_eight_and_cover_every_item_once() {
        let grid = Grid::new("", &Recents::parse("🦊"));
        let mut next = 0;
        for row in &grid.rows {
            if let Row::Cells(cells) = row {
                assert_eq!(cells.start, next);
                assert!(cells.len() <= COLUMNS);
                next = cells.end;
            }
        }
        assert_eq!(next, grid.items.len());
    }

    #[test]
    fn steps_go_along_rows_and_keep_the_column_up_and_down() {
        let grid = Grid::new("smil", &Recents::default());
        let last = grid.items.len() - 1;
        assert_eq!(grid.step(None, 1, 0), Some(0));
        assert_eq!(grid.step(Some(0), 1, 0), Some(1));
        assert_eq!(grid.step(Some(0), -1, 0), Some(0));
        assert_eq!(grid.step(Some(last), 1, 0), Some(last));
        assert_eq!(grid.step(Some(3), 0, 1), Some(3 + COLUMNS));
        assert_eq!(grid.step(Some(3 + COLUMNS), 0, -1), Some(3));
        assert_eq!(grid.step(Some(3), 0, -1), Some(3));
        assert_eq!(grid.step(Some(last), 0, 1), Some(last));
        // Down from a column the last row doesn't reach lands on its last item.
        let first_of_last_row = last - last % COLUMNS;
        if first_of_last_row >= COLUMNS && last % COLUMNS < COLUMNS - 1 {
            assert_eq!(grid.step(Some(first_of_last_row - 1), 0, 1), Some(last));
        }
    }

    #[test]
    fn steps_skip_the_headers_between_groups() {
        let grid = Grid::new("", &Recents::default());
        let people = grid.rows.iter().position(|row| *row == Row::Header("People & Body")).unwrap();
        let Row::Cells(last_smileys) = &grid.rows[people - 1] else { panic!("not a row") };
        let Row::Cells(first_people) = &grid.rows[people + 1] else { panic!("not a row") };
        assert_eq!(grid.step(Some(last_smileys.start), 0, 1), Some(first_people.start));
        assert_eq!(grid.step(Some(first_people.start), 0, -1), Some(last_smileys.start));
    }

    #[test]
    fn a_pick_goes_first_once_and_the_list_keeps_sixteen() {
        let mut recents = Recents::default();
        recents.push("🦊");
        recents.push("🌱");
        recents.push("🦊");
        assert_eq!(recents.to_text(), "🦊\n🌱\n");
        for emoji in emojis::iter().take(30) {
            recents.push(emoji.as_str());
        }
        assert_eq!(recents.0.len(), RECENTS);
        assert_eq!(Recents::parse(&recents.to_text()), recents);
    }

    #[test]
    fn recents_leave_out_what_the_table_doesnt_know() {
        assert_eq!(Recents::parse("🦊\nabc\n\n日\n"), Recents(vec!["🦊".to_string()]));
    }
}
