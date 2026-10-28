// Adapted from Zed's crates/ui/src/components/context_menu.rs
// (https://github.com/zed-industries/zed) at commit b36b283021c48f1712714b8a0e2ad6ffb578920b:
// the keyboard's walk over the selectable rows and the submenu's open and close. Copyright Zed
// Industries, Inc. Licensed under the GNU General Public License, version 3 or later.

//! ilo's own context menu, the same on every platform (#324): an opaque panel of 24-pt rows, with
//! submenus, keys, type-to-select, press-drag-release and a safe triangle on the way to a
//! submenu. GPUI can't draw over the page's native view, so it's a window of its own that the host
//! shows over the browser window, as its child, with the keyboard, as the command bar is
//! (AGENTS.md › Over the page). What the menu decides is in `Tracking` and `lay_out`, which the
//! tests drive; what it asks of its window goes through `Host`.

use std::{
    rc::Rc,
    time::Duration,
};
use web_time::Instant;

use gpui::{
    App, BoxShadow, Bounds, Context, FocusHandle, Hsla, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Radians, Role, SharedString,
    Task, TitlebarOptions, Transformation, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, div, point, prelude::*, px, rgba, size, svg,
};

use crate::shell::{MenuEntry, keystroke_label};

pub use crate::tokens::menu::*;

/// A release this close to the press that opened the menu leaves it open, as a click.
const DRAG_SLOP: f32 = 4.;
/// The safe triangle ends after the pointer rests this long in it.
const SAFE_TIMEOUT: Duration = Duration::from_millis(300);
/// Type-to-select starts over after this long without a key.
const TYPE_RESET: Duration = Duration::from_millis(1000);
/// The window's room around the panels for their shadow.
const MARGIN_SIDE: f32 = SHADOW_BLUR;
const MARGIN_TOP: f32 = SHADOW_BLUR - SHADOW_Y;
const MARGIN_BOTTOM: f32 = SHADOW_BLUR + SHADOW_Y;

/// The colours of one look, 0xRRGGBBAA.
struct Palette {
    fill: u32,
    rim: u32,
    shadow: u32,
    label: u32,
    secondary: u32,
    line: u32,
    disabled: u32,
    destructive: u32,
}

const DARK: Palette = Palette {
    fill: DARK_FILL,
    rim: DARK_RIM,
    shadow: DARK_SHADOW,
    label: DARK_LABEL,
    secondary: DARK_SECONDARY,
    line: DARK_LINE,
    disabled: DARK_DISABLED,
    destructive: DARK_DESTRUCTIVE,
};

const LIGHT: Palette = Palette {
    fill: LIGHT_FILL,
    rim: LIGHT_RIM,
    shadow: LIGHT_SHADOW,
    label: LIGHT_LABEL,
    secondary: LIGHT_SECONDARY,
    line: LIGHT_LINE,
    disabled: LIGHT_DISABLED,
    destructive: LIGHT_DESTRUCTIVE,
};

/// The highlight and the label on it, which the palette (#335) replaces with the Space's accent
/// by the contrast rule.
fn highlight() -> (u32, u32) {
    (HIGHLIGHT, HIGHLIGHT_LABEL)
}

/// The menu's look: the panel's material is Ink in a dark window and Paper in a light one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    #[default]
    Dark,
    Light,
}

impl Look {
    fn palette(self) -> &'static Palette {
        match self {
            Look::Dark => &DARK,
            Look::Light => &LIGHT,
        }
    }
}

/// What the menu asks of its window. Both are called once the menu's update is over, as putting a
/// window on screen or taking it off can draw it, which would need the menu while it's held.
pub trait Host {
    /// Puts the menu's window over the browser window it's for at `frame` in that window's
    /// content, as it draws now, with the keyboard.
    fn show(&self, frame: Bounds<Pixels>);
    /// Takes it away, and gives the keyboard back to what had it.
    fn hide(&self);
}

/// A menu to show.
pub struct Request {
    pub items: Vec<MenuEntry>,
    /// The pointer, in the browser window's content: the menu's top left goes there.
    pub at: Point<Pixels>,
    /// Where it may go, in the same terms: the screen's work area.
    pub area: Bounds<Pixels>,
    pub look: Look,
    /// Called once it closes, with the index of the item chosen, if any.
    pub chosen: Box<dyn FnOnce(Option<usize>, &mut App)>,
}

/// One level of the menu, the root or a submenu, placed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Level {
    /// The entry whose submenu it is, none for the root.
    parent: Option<usize>,
    items: Vec<usize>,
    bounds: Bounds<Pixels>,
    /// Each item's top, from the panel's top.
    tops: Vec<f32>,
    /// Whether a column for checks comes before the titles.
    checks: bool,
}

impl Level {
    fn row_bounds(&self, at: usize, entries: &[MenuEntry]) -> Bounds<Pixels> {
        let height = if entries[self.items[at]].title.is_empty() { LINE } else { ROW };
        let origin = self.bounds.origin + point(px(PADDING), px(self.tops[at]));
        Bounds::new(origin, size(self.bounds.size.width - px(2. * PADDING), px(height)))
    }
}

/// Where each level of a menu goes, and the window that holds them with their shadows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    levels: Vec<Level>,
    pub(crate) frame: Bounds<Pixels>,
}

impl Layout {
    /// The level that's the submenu of `entry`.
    fn level_of(&self, entry: usize) -> Option<usize> {
        self.levels.iter().position(|level| level.parent == Some(entry))
    }
}

/// How wide a level's panel is for titles and shortcuts `width_of` measures, between the
/// minimum and the maximum, where longer titles end in "…".
fn panel_width(entries: &[MenuEntry], items: &[usize], checks: bool, width_of: &dyn Fn(&str) -> f32) -> f32 {
    let widest = (items.iter().map(|&at| &entries[at]))
        .filter(|entry| !entry.title.is_empty())
        .map(|entry| {
            let trailing = match &entry.shortcut {
                Some(keys) => GAP + width_of(&keystroke_label(keys)),
                None => 0.,
            };
            width_of(&entry.title) + trailing
        })
        .fold(0., f32::max);
    let submenus = items.iter().any(|&at| entries.iter().any(|each| each.parent == Some(at)));
    let lead = if checks { CHECK + GAP } else { 0. };
    let chevron = if submenus { GAP + CHEVRON } else { 0. };
    (2. * (PADDING + ROW_X) + lead + widest + chevron).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// `origin` moved inside `area` by EDGE for a panel of `size`, as far as it fits.
fn clamped(origin: Point<Pixels>, size: (f32, f32), area: Bounds<Pixels>) -> Point<Pixels> {
    let (left, top) = (f32::from(area.left()) + EDGE, f32::from(area.top()) + EDGE);
    let (right, bottom) = (f32::from(area.right()) - EDGE, f32::from(area.bottom()) - EDGE);
    let x = f32::from(origin.x).min(right - size.0).max(left);
    let y = f32::from(origin.y).min(bottom - size.1).max(top);
    point(px(x), px(y))
}

/// Lays `entries` out as a menu with its top left at `at`, flipping up or left where `area` has
/// no room below or to the right, as AppKit's does, with each submenu beside the row that opens
/// it, flipping left and sliding up to fit.
pub(crate) fn lay_out(
    entries: &[MenuEntry],
    at: Point<Pixels>,
    area: Bounds<Pixels>,
    width_of: &dyn Fn(&str) -> f32,
) -> Layout {
    let parents: Vec<Option<usize>> = std::iter::once(None)
        .chain((0..entries.len()).filter(|&at| entries.iter().any(|e| e.parent == Some(at))).map(Some))
        .collect();
    let mut levels: Vec<Level> = Vec::new();
    for parent in parents {
        let items: Vec<usize> = (0..entries.len()).filter(|&at| entries[at].parent == parent).collect();
        let mut tops = Vec::with_capacity(items.len());
        let mut y = PADDING;
        for &item in &items {
            tops.push(y);
            y += if entries[item].title.is_empty() { LINE } else { ROW };
        }
        let checks = items.iter().any(|&item| entries[item].checked);
        let size = (panel_width(entries, &items, checks, width_of), y + PADDING);
        let origin = match parent {
            None => {
                let (x, y) = (f32::from(at.x), f32::from(at.y));
                let x = if x + size.0 > f32::from(area.right()) - EDGE { x - size.0 } else { x };
                let y = if y + size.1 > f32::from(area.bottom()) - EDGE { y - size.1 } else { y };
                clamped(point(px(x), px(y)), size, area)
            }
            Some(parent) => {
                // A submenu's parent's level comes before it, as an entry's parent comes first.
                let Some(above) = levels.iter().find(|level| level.items.contains(&parent)) else {
                    continue;
                };
                let row = above.row_bounds(above.items.iter().position(|&i| i == parent).unwrap_or(0), entries);
                let right = f32::from(above.bounds.right()) - OVERLAP;
                let x = if right + size.0 > f32::from(area.right()) - EDGE {
                    f32::from(above.bounds.left()) + OVERLAP - size.0
                } else {
                    right
                };
                clamped(point(px(x), row.origin.y - px(PADDING)), size, area)
            }
        };
        let bounds = Bounds::new(origin, gpui::size(px(size.0), px(size.1)));
        levels.push(Level { parent, items, bounds, tops, checks });
    }
    let union = (levels.iter().skip(1)).fold(levels[0].bounds, |all, level| all.union(&level.bounds));
    let frame = Bounds::from_corners(
        union.origin - point(px(MARGIN_SIDE), px(MARGIN_TOP)),
        union.bottom_right() + point(px(MARGIN_SIDE), px(MARGIN_BOTTOM)),
    );
    Layout { levels, frame }
}

/// What's under a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    /// An item's row, by level and entry.
    Row(usize, usize),
    /// A level's panel, between rows or on a line.
    Panel(usize),
    Outside,
}

/// A key the menu takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    Up,
    Down,
    First,
    Last,
    /// Into the highlighted row's submenu.
    Right,
    /// Out of the submenu.
    Left,
    /// Return or space: picks the highlighted row.
    Choose,
    Escape,
    Char(char),
}

/// What the user did to the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Pick(usize),
    Close,
}

/// An open level and the item highlighted in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Open {
    level: usize,
    highlight: Option<usize>,
}

/// A menu from its press until it closes: which levels are open and what's highlighted, from the
/// pointer and the keys.
pub(crate) struct Tracking {
    entries: Vec<MenuEntry>,
    layout: Layout,
    /// Root first, then each submenu open in the one before.
    open: Vec<Open>,
    /// The press that opened the menu, until its release.
    press: Option<Point<Pixels>>,
    /// A button went down on the menu and hasn't come up.
    pressed: bool,
    pointer: Option<Point<Pixels>>,
    /// Since when the pointer has rested in the safe triangle, keeping the submenu open.
    resting: Option<Instant>,
    typed: String,
    typed_at: Option<Instant>,
}

/// Whether `at` is inside the triangle `a`, `b`, `c`.
fn in_triangle(at: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>, c: Point<Pixels>) -> bool {
    let cross = |p: Point<Pixels>, q: Point<Pixels>, r: Point<Pixels>| {
        f32::from(q.x - p.x) * f32::from(r.y - p.y) - f32::from(q.y - p.y) * f32::from(r.x - p.x)
    };
    let (d1, d2, d3) = (cross(a, b, at), cross(b, c, at), cross(c, a, at));
    let negative = d1 < 0. || d2 < 0. || d3 < 0.;
    let positive = d1 > 0. || d2 > 0. || d3 > 0.;
    !(negative && positive)
}

/// `text` lowercased without its accents, for type-to-select.
fn folded(text: &str) -> String {
    const ACCENTED: [(&str, char); 8] = [
        ("àáâãäåā", 'a'),
        ("çćč", 'c'),
        ("èéêëēė", 'e'),
        ("ìíîïī", 'i'),
        ("ñń", 'n'),
        ("òóôõöøō", 'o'),
        ("ùúûüū", 'u'),
        ("ýÿ", 'y'),
    ];
    (text.chars().flat_map(char::to_lowercase))
        .map(|c| ACCENTED.iter().find(|(set, _)| set.contains(c)).map_or(c, |&(_, base)| base))
        .collect()
}

impl Tracking {
    pub(crate) fn new(entries: Vec<MenuEntry>, layout: Layout, at: Point<Pixels>) -> Self {
        Tracking {
            entries,
            layout,
            open: vec![Open { level: 0, highlight: None }],
            press: Some(at),
            pressed: false,
            pointer: Some(at),
            resting: None,
            typed: String::new(),
            typed_at: None,
        }
    }

    fn selectable(&self, entry: usize) -> bool {
        let entry = &self.entries[entry];
        entry.enabled && !entry.title.is_empty()
    }

    /// The item highlighted in each open level, root first.
    #[cfg(test)]
    pub(crate) fn highlights(&self) -> Vec<Option<usize>> {
        self.open.iter().map(|open| open.highlight).collect()
    }

    /// The titles of what's highlighted, root first, for the tests.
    #[cfg(test)]
    fn highlighted(&self) -> Vec<&str> {
        (self.open.iter().filter_map(|open| open.highlight))
            .map(|at| self.entries[at].title.as_ref())
            .collect()
    }

    fn hit(&self, at: Point<Pixels>) -> Hit {
        for open in self.open.iter().rev() {
            let level = &self.layout.levels[open.level];
            if !level.bounds.contains(&at) {
                continue;
            }
            let row = (0..level.items.len()).find(|&row| level.row_bounds(row, &self.entries).contains(&at));
            return match row {
                Some(row) if !self.entries[level.items[row]].title.is_empty() => {
                    Hit::Row(open.level, level.items[row])
                }
                _ => Hit::Panel(open.level),
            };
        }
        Hit::Outside
    }

    /// The pointer is at `at`, which highlights what's under it and opens its submenu, unless it's
    /// on its way to the open submenu (`safe`).
    fn point_at(&mut self, at: Point<Pixels>) {
        let depth = |open: &[Open], level| open.iter().position(|each| each.level == level);
        match self.hit(at) {
            Hit::Row(level, item) => {
                let Some(k) = depth(&self.open, level) else { return };
                let sub = self.layout.level_of(item);
                if sub.is_some() && self.open.get(k + 1).map(|next| next.level) == sub {
                    // Back on the row whose submenu is open: it stays open.
                    self.open.truncate(k + 2);
                    self.open[k + 1].highlight = None;
                    self.open[k].highlight = Some(item);
                    return;
                }
                self.open.truncate(k + 1);
                let selectable = self.selectable(item);
                self.open[k].highlight = selectable.then_some(item);
                if let Some(level) = sub.filter(|_| selectable) {
                    self.open.push(Open { level, highlight: None });
                }
            }
            Hit::Panel(level) => {
                if let Some(k) = depth(&self.open, level) {
                    self.open.truncate(k + 1);
                    self.open[k].highlight = None;
                }
            }
            // A submenu's parent row stays highlighted, as its submenu is open.
            Hit::Outside => {
                if let Some(last) = self.open.last_mut() {
                    last.highlight = None;
                }
            }
        }
    }

    /// Whether the pointer's move from `from` to `to` is on its way to the open submenu: inside
    /// the triangle from where it was to the submenu's two near corners, off the row that opened
    /// it and not yet in the submenu.
    fn safe(&self, from: Point<Pixels>, to: Point<Pixels>) -> bool {
        let [.., parent, sub] = self.open[..] else { return false };
        let level = &self.layout.levels[sub.level];
        if from == to || level.bounds.contains(&to) || parent.highlight != level.parent {
            return false;
        }
        if matches!(self.hit(to), Hit::Row(_, item) if Some(item) == level.parent) {
            return false;
        }
        let beside = &self.layout.levels[parent.level].bounds;
        let x = if level.bounds.left() >= beside.center().x { level.bounds.left() } else { level.bounds.right() };
        in_triangle(to, from, point(x, level.bounds.top()), point(x, level.bounds.bottom()))
    }

    /// The pointer moved to `at` at `now`. Returns whether it's resting in the safe triangle, for
    /// `rested` to be called once SAFE_TIMEOUT has passed.
    pub(crate) fn moved(&mut self, at: Point<Pixels>, now: Instant) -> bool {
        let from = self.pointer.replace(at);
        if from.is_some_and(|from| self.safe(from, at)) {
            self.resting = Some(now);
            return true;
        }
        self.resting = None;
        self.point_at(at);
        false
    }

    /// The pointer has rested since its last move: in the safe triangle for SAFE_TIMEOUT, what's
    /// under it takes the highlight.
    pub(crate) fn rested(&mut self, now: Instant) {
        if self.resting.is_some_and(|since| now.duration_since(since) >= SAFE_TIMEOUT) {
            self.resting = None;
            if let Some(at) = self.pointer {
                self.point_at(at);
            }
        }
    }

    /// A button went down at `at`. Returns whether it's on the menu; outside, it closes it.
    pub(crate) fn pressed(&mut self, at: Point<Pixels>) -> bool {
        self.press = None;
        if self.hit(at) == Hit::Outside {
            return false;
        }
        self.pressed = true;
        self.pointer = Some(at);
        self.resting = None;
        self.point_at(at);
        true
    }

    /// The button came up at `at`: on an item it picks it, as does a release after a drag from
    /// the press that opened the menu, which outside closes it.
    pub(crate) fn released(&mut self, at: Point<Pixels>) -> Outcome {
        let opening = self.press.take();
        let pressed = std::mem::replace(&mut self.pressed, false);
        match opening {
            Some(press) => {
                let (dx, dy) = (f32::from(at.x - press.x), f32::from(at.y - press.y));
                if dx.hypot(dy) <= DRAG_SLOP {
                    return Outcome::Stay;
                }
            }
            None if !pressed => return Outcome::Stay,
            None => {}
        }
        match self.hit(at) {
            Hit::Row(_, item) if self.selectable(item) && self.layout.level_of(item).is_none() => {
                Outcome::Pick(item)
            }
            Hit::Outside if opening.is_some() => Outcome::Close,
            _ => Outcome::Stay,
        }
    }

    /// The selectable items of the deepest level, and its place in `open`.
    fn deepest(&mut self) -> (usize, Vec<usize>) {
        // A submenu the pointer opened has nothing highlighted, and the keys walk its parent.
        if self.open.len() > 1 && self.open.last().is_some_and(|last| last.highlight.is_none()) {
            self.open.pop();
        }
        let k = self.open.len() - 1;
        let items = self.layout.levels[self.open[k].level].items.clone();
        (k, items.into_iter().filter(|&item| self.selectable(item)).collect())
    }

    /// Opens the highlighted row's submenu, if it has one, with its first item highlighted.
    fn open_submenu(&mut self) -> bool {
        let Some(item) = self.open.last().and_then(|last| last.highlight) else { return false };
        let Some(level) = self.layout.level_of(item) else { return false };
        let first = (self.layout.levels[level].items.iter()).copied().find(|&each| self.selectable(each));
        self.open.push(Open { level, highlight: first });
        true
    }

    pub(crate) fn key(&mut self, key: Key, now: Instant) -> Outcome {
        if let Key::Char(c) = key {
            self.type_select(c, now);
            return Outcome::Stay;
        }
        let (k, items) = self.deepest();
        let current = self.open[k].highlight.and_then(|item| items.iter().position(|&each| each == item));
        let to = |at: Option<usize>| at.and_then(|at| items.get(at).copied());
        match key {
            // At the ends they stay, as NSMenu's do.
            Key::Down => {
                let next = current.map_or(0, |at| (at + 1).min(items.len().saturating_sub(1)));
                if let Some(item) = to(Some(next)) {
                    self.open[k].highlight = Some(item);
                }
            }
            Key::Up => {
                let previous = current.map_or(items.len().checked_sub(1), |at| Some(at.saturating_sub(1)));
                if let Some(item) = to(previous) {
                    self.open[k].highlight = Some(item);
                }
            }
            Key::First => self.open[k].highlight = items.first().copied().or(self.open[k].highlight),
            Key::Last => self.open[k].highlight = items.last().copied().or(self.open[k].highlight),
            Key::Right => {
                self.open_submenu();
            }
            Key::Left => {
                if self.open.len() > 1 {
                    self.open.pop();
                }
            }
            Key::Choose => {
                if let Some(item) = self.open[k].highlight
                    && !self.open_submenu()
                {
                    return Outcome::Pick(item);
                }
            }
            Key::Escape => {
                if self.open.len() == 1 {
                    return Outcome::Close;
                }
                self.open.pop();
            }
            Key::Char(_) => {}
        }
        Outcome::Stay
    }

    /// Type-to-select: the first item of the deepest level whose title starts with what's been
    /// typed in the last second, ignoring case and accents, or with the same letter again, the
    /// next one that starts with it.
    fn type_select(&mut self, c: char, now: Instant) {
        if self.typed_at.is_none_or(|at| now.duration_since(at) > TYPE_RESET) {
            self.typed.clear();
        }
        self.typed.push(c);
        self.typed_at = Some(now);
        let (k, items) = self.deepest();
        let typed = folded(&self.typed);
        let mut letters = typed.chars();
        let first = letters.next();
        let repeated = typed.chars().count() > 1 && letters.all(|each| Some(each) == first);
        let starts = |item: &usize, with: &str| folded(&self.entries[*item].title).starts_with(with);
        let found = if repeated {
            let with = first.map(String::from).unwrap_or_default();
            let after = self.open[k].highlight.and_then(|item| items.iter().position(|&each| each == item));
            let start = after.map_or(0, |at| at + 1);
            (items[start..].iter().chain(&items[..start])).find(|item| starts(item, &with)).copied()
        } else {
            items.iter().find(|item| starts(item, &typed)).copied()
        };
        if let Some(item) = found {
            self.open[k].highlight = Some(item);
        }
    }

    /// The enabled item titled `title`, or with "Submenu › Item" the one in that submenu, as the
    /// driver chooses one.
    pub(crate) fn find(&self, title: &str) -> Option<usize> {
        let (parent, title) = match title.split_once(" › ") {
            Some((parent, title)) => (Some(parent), title),
            None => (None, title),
        };
        (0..self.entries.len()).find(|&at| {
            let entry = &self.entries[at];
            entry.enabled
                && entry.title == title
                && self.layout.level_of(at).is_none()
                && parent.is_none_or(|parent| {
                    entry.parent.is_some_and(|up| self.entries[up].title == parent)
                })
        })
    }
}

/// The menu's items as the host's menus log them, for check.sh: lines left out, a submenu's items
/// after their parent's title in place of it, and the dimmed and checked ones marked.
pub(crate) fn described(entries: &[MenuEntry]) -> String {
    let mark = |entry: &MenuEntry, title: String| match (entry.enabled, entry.checked) {
        (false, _) => format!("{title} (disabled)"),
        (true, true) => format!("{title} (checked)"),
        (true, false) => title,
    };
    let mut titles = Vec::new();
    for (at, entry) in entries.iter().enumerate() {
        if entry.parent.is_some() || entry.title.is_empty() {
            continue;
        }
        let children: Vec<&MenuEntry> =
            entries.iter().filter(|each| each.parent == Some(at) && !each.title.is_empty()).collect();
        match children.is_empty() {
            true => titles.push(mark(entry, entry.title.to_string())),
            false => (children.into_iter())
                .for_each(|child| titles.push(mark(child, format!("{} › {}", entry.title, child.title)))),
        }
    }
    titles.join(", ")
}

/// The key `event` is to the menu: arrows, Home and End (⌥↑ and ⌥↓ too), return, space, escape, or
/// a character for type-to-select. A menu has no text to compose, so a character an input method
/// gives a key, such as kana, selects as itself.
pub(crate) fn key_of(event: &KeyDownEvent) -> Option<Key> {
    let keystroke = &event.keystroke;
    let alt = keystroke.modifiers.alt;
    Some(match keystroke.key.as_str() {
        "up" if alt => Key::First,
        "down" if alt => Key::Last,
        "up" => Key::Up,
        "down" => Key::Down,
        "home" | "pageup" => Key::First,
        "end" | "pagedown" => Key::Last,
        "right" => Key::Right,
        "left" => Key::Left,
        "enter" | "space" => Key::Choose,
        "escape" => Key::Escape,
        key => {
            let typed = keystroke.key_char.as_deref().unwrap_or(key);
            let mut chars = typed.chars();
            let c = chars.next().filter(|c| !c.is_control() && chars.next().is_none())?;
            Key::Char(c)
        }
    })
}

/// The menu while it shows.
struct Shown {
    tracking: Tracking,
    look: Look,
    /// Where its window's top left is in the browser window's content.
    origin: Point<Pixels>,
    chosen: Option<Box<dyn FnOnce(Option<usize>, &mut App)>>,
}

/// The menu's window's view.
pub struct ContextMenu {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    shown: Option<Shown>,
    /// Where the window stays, when the host can't move it, as the preview's; else it goes where
    /// each menu does.
    fixed: Option<Point<Pixels>>,
    _rest: Option<Task<()>>,
    a11y: bool,
}

struct MenuWindow(WindowHandle<ContextMenu>);

impl gpui::Global for MenuWindow {}

fn options(bounds: Bounds<Pixels>, show: bool) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // The preview's has none, so it hasn't the window buttons the host's takes off.
        titlebar: (!show).then_some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        // A panel hides while its app isn't the active one, which a preview a script started
        // isn't, so the preview's is a plain window over its own.
        kind: if show { WindowKind::Normal } else { WindowKind::Floating },
        show,
        focus: show,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The panels are drawn with their shadows, over nothing.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    }
}

fn open_window(
    host: Rc<dyn Host>,
    bounds: Bounds<Pixels>,
    fixed: Option<Point<Pixels>>,
    cx: &mut App,
) -> Option<WindowHandle<ContextMenu>> {
    let window = cx.open_window(options(bounds, fixed.is_some()), |_, cx| {
        cx.new(|cx| ContextMenu { host, focus: cx.focus_handle(), shown: None, fixed, _rest: None, a11y: false })
    });
    match window {
        Ok(window) => {
            cx.set_global(MenuWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the menu's window: {error}");
            None
        }
    }
}

/// Opens the menu's window, hidden, for `host` to show over a browser window, ahead of time as the
/// command bar's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<ContextMenu>> {
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(MIN_WIDTH), px(MIN_WIDTH)));
    open_window(host, bounds, None, cx)
}

/// Opens the menu's window on screen at `bounds`, over a browser window there whose content is
/// the same, where it stays: for the preview, whose host can't move it.
pub fn open_at(host: Rc<dyn Host>, bounds: Bounds<Pixels>, cx: &mut App) -> Option<WindowHandle<ContextMenu>> {
    open_window(host, bounds, Some(point(px(0.), px(0.))), cx)
}

fn update(cx: &mut App, change: impl FnOnce(&mut ContextMenu, &mut Window, &mut Context<ContextMenu>)) {
    if let Some(window) = cx.try_global::<MenuWindow>().map(|menu| menu.0) {
        window.update(cx, change).ok();
    }
}

/// Shows the menu `request` asks for, closing one that shows.
pub fn show(request: Request, cx: &mut App) {
    let mut request = Some(request);
    update(cx, |menu, window, cx| {
        let Some(request) = request.take() else { return };
        menu.close(None, cx);
        let system = window.text_system().clone();
        let width_of = |text: &str| {
            let text = SharedString::from(text.to_string());
            crate::tooltip::text_width(&text, TEXT, gpui::FontWeight::NORMAL, &system)
        };
        let layout = lay_out(&request.items, request.at, request.area, &width_of);
        // check.sh waits for this line before it chooses.
        log!("browser: menu shown {}", described(&request.items));
        let frame = layout.frame;
        let origin = menu.fixed.unwrap_or(frame.origin);
        menu.shown = Some(Shown {
            tracking: Tracking::new(request.items, layout, request.at),
            look: request.look,
            origin,
            chosen: Some(request.chosen),
        });
        window.focus(&menu.focus, cx);
        // After a hide that `close` asked for, so the new menu stays.
        let host = menu.host.clone();
        cx.defer(move |_| host.show(frame));
        cx.notify();
    });
}

/// What a pointer event the host saw anywhere in the app is, for `pointer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    Moved,
    /// The left button, or another but the right.
    Pressed,
    RightPressed,
    Released,
    Scrolled,
}

/// The host saw `event` at `at` in the menu's window, wherever it happened, while the menu shows.
/// Returns whether the menu takes it, so the window under it never hears it.
pub fn pointer(event: Pointer, at: Point<Pixels>, cx: &mut App) -> bool {
    let mut taken = false;
    update(cx, |menu, _, cx| taken = menu.pointer(event, at, cx));
    taken
}

/// The user left the menu without choosing: another window or app, or the browser window
/// resizing, moving or going full screen.
pub fn dismissed(cx: &mut App) {
    update(cx, |menu, _, cx| menu.close(None, cx));
}

/// Chooses the enabled item titled `title`, as the driver does. Returns whether there was one.
pub fn choose(title: &str, cx: &mut App) -> bool {
    let mut chosen = false;
    update(cx, |menu, _, cx| {
        let found = menu.shown.as_ref().and_then(|shown| shown.tracking.find(title));
        if let Some(item) = found {
            chosen = true;
            menu.close(Some(item), cx);
        }
    });
    chosen
}

impl ContextMenu {
    /// Closes the menu, if it shows, and says which item was chosen once the update is over.
    fn close(&mut self, chosen: Option<usize>, cx: &mut Context<Self>) {
        let Some(mut shown) = self.shown.take() else { return };
        self._rest = None;
        let host = self.host.clone();
        cx.defer(move |_| host.hide());
        // check.sh reads this line.
        match chosen {
            Some(item) => log!("browser: menu chose {}", shown.tracking.entries[item].title),
            None => log!("browser: menu closed"),
        }
        if let Some(answer) = shown.chosen.take() {
            cx.defer(move |cx| answer(chosen, cx));
        }
        cx.notify();
    }

    fn outcome(&mut self, outcome: Outcome, cx: &mut Context<Self>) {
        match outcome {
            Outcome::Stay => cx.notify(),
            Outcome::Pick(item) => self.close(Some(item), cx),
            Outcome::Close => self.close(None, cx),
        }
    }

    /// `at` in the window, in the browser window's content.
    fn in_content(&self, at: Point<Pixels>) -> Point<Pixels> {
        self.shown.as_ref().map_or(at, |shown| shown.origin + at)
    }

    fn pointer(&mut self, event: Pointer, at: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let at = self.in_content(at);
        let Some(shown) = self.shown.as_mut() else { return false };
        let tracking = &mut shown.tracking;
        match event {
            Pointer::Moved => {
                if tracking.moved(at, Instant::now()) {
                    self.wait_for_rest(cx);
                }
                cx.notify();
                true
            }
            Pointer::Pressed | Pointer::RightPressed => {
                if tracking.pressed(at) {
                    cx.notify();
                    return true;
                }
                // A click outside closes the menu and goes no further; a right-click goes on, to
                // open the menu of what it's on.
                self.close(None, cx);
                event == Pointer::Pressed
            }
            Pointer::Released => {
                let outcome = tracking.released(at);
                self.outcome(outcome, cx);
                true
            }
            Pointer::Scrolled => {
                if tracking.hit(at) != Hit::Outside {
                    return true;
                }
                self.close(None, cx);
                false
            }
        }
    }

    /// Once the pointer has rested in the safe triangle long enough, what's under it takes the
    /// highlight.
    fn wait_for_rest(&mut self, cx: &mut Context<Self>) {
        self._rest = Some(cx.spawn(async move |menu, cx| {
            cx.background_executor().timer(SAFE_TIMEOUT).await;
            menu.update(cx, |menu, cx| {
                if let Some(shown) = menu.shown.as_mut() {
                    shown.tracking.rested(Instant::now());
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    fn key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(shown) = self.shown.as_mut() else { return };
        let modifiers = event.keystroke.modifiers;
        // A shortcut closes the menu, as the menu shows them but doesn't run them.
        if modifiers.platform || modifiers.control {
            return self.close(None, cx);
        }
        if let Some(key) = key_of(event) {
            let outcome = shown.tracking.key(key, Instant::now());
            self.outcome(outcome, cx);
        }
    }

    fn row(&self, shown: &Shown, level: usize, row: usize, lit: Option<usize>, open: bool) -> gpui::AnyElement {
        let tracking = &shown.tracking;
        let palette = shown.look.palette();
        let lay = &tracking.layout.levels[level];
        let item = lay.items[row];
        let entry = &tracking.entries[item];
        let bounds = lay.row_bounds(row, &tracking.entries);
        let top = bounds.origin.y - lay.bounds.origin.y;
        let placed = div().absolute().left(px(PADDING)).top(top).w(bounds.size.width).h(bounds.size.height);
        if entry.title.is_empty() {
            let line = div().mx(px(ROW_X)).h(px(1.)).bg(rgba(palette.line));
            return placed.flex().items_center().child(div().flex_1().child(line)).into_any_element();
        }
        let lit = lit == Some(item);
        let (fill, on_fill) = match entry.destructive {
            true => (DESTRUCTIVE_FILL, DESTRUCTIVE_LABEL),
            false => highlight(),
        };
        let label: Hsla = match (lit, entry.enabled, entry.destructive) {
            (true, ..) => rgba(on_fill).into(),
            (false, false, _) => rgba(palette.disabled).into(),
            (false, true, true) => rgba(palette.destructive).into(),
            (false, true, false) => rgba(palette.label).into(),
        };
        let secondary: Hsla = match (lit, entry.enabled) {
            (true, _) => rgba(on_fill).into(),
            (false, false) => rgba(palette.disabled).into(),
            (false, true) => rgba(palette.secondary).into(),
        };
        let submenu = tracking.layout.level_of(item).is_some();
        let shortcut = entry.shortcut.as_deref().map(keystroke_label);
        let pressed = lit && tracking.pressed;
        let check = lay.checks.then(|| {
            div().w(px(CHECK)).flex_none().when(entry.checked, |slot| {
                slot.child(svg().path("icons/check.svg").size(px(CHECK)).text_color(label))
            })
        });
        let trailing = match (&shortcut, submenu) {
            (_, true) => Some(
                svg()
                    .path("icons/chevron-down.svg")
                    .size(px(CHEVRON))
                    .flex_none()
                    .text_color(secondary)
                    .with_transformation(Transformation::rotate(Radians(-std::f32::consts::FRAC_PI_2)))
                    .into_any_element(),
            ),
            (Some(keys), false) => {
                Some(div().flex_none().text_color(secondary).child(keys.clone()).into_any_element())
            }
            (None, false) => None,
        };
        let a11y = self.a11y;
        let name = entry.title.clone();
        let (enabled, checked, keys) = (entry.enabled, entry.checked, shortcut.clone());
        let radio = checked && level != 0;
        placed
            .id(("menu-item", item))
            .rounded(px(ROW_RADIUS))
            .px(px(ROW_X))
            .flex()
            .items_center()
            .gap(px(GAP))
            .when(lit, |row| row.bg(rgba(fill)))
            .child(div().absolute().inset_0().rounded(px(ROW_RADIUS)).when(pressed, |o| o.bg(rgba(PRESSED))))
            .children(check)
            .child(div().flex_1().min_w_0().truncate().text_color(label).child(entry.title.clone()))
            .children(trailing)
            .when(a11y, |row| {
                let role = match (checked, radio) {
                    (true, true) => Role::MenuItemRadio,
                    (true, false) => Role::MenuItemCheckBox,
                    _ => Role::MenuItem,
                };
                row.role(role)
                    .aria_label(name)
                    .when(checked, |row| row.aria_toggled(gpui::accesskit::Toggled::True))
                    .when_some(keys, |row, keys| row.aria_keyshortcuts(keys))
                    .when(submenu, |row| row.aria_expanded(open))
                    .when(lit, |row| row.aria_active_descendant())
                    .a11y_synthetic_children(move |builder| {
                        let node = builder.parent_node();
                        if !enabled {
                            node.set_disabled();
                        }
                        if submenu {
                            node.set_has_popup(gpui::accesskit::HasPopup::Menu);
                        }
                    })
            })
            .into_any_element()
    }

    fn panel(&self, shown: &Shown, depth: usize) -> gpui::AnyElement {
        let tracking = &shown.tracking;
        let open = tracking.open[depth];
        let level = &tracking.layout.levels[open.level];
        let palette = shown.look.palette();
        let at = level.bounds.origin - shown.origin;
        let next = tracking.open.get(depth + 1).map(|next| next.level);
        let rows = (0..level.items.len()).map(|row| {
            let opens = next.is_some_and(|next| tracking.layout.levels[next].parent == Some(level.items[row]));
            self.row(shown, open.level, row, open.highlight, opens)
        });
        let name = level.parent.map_or(SharedString::from("Menu"), |parent| tracking.entries[parent].title.clone());
        div()
            .id(("menu", open.level))
            .absolute()
            .left(at.x)
            .top(at.y)
            .w(level.bounds.size.width)
            .h(level.bounds.size.height)
            .rounded(px(RADIUS))
            .bg(rgba(palette.fill))
            .border_1()
            .border_color(rgba(palette.rim))
            .shadow(vec![BoxShadow {
                color: rgba(palette.shadow).into(),
                offset: point(px(0.), px(SHADOW_Y)),
                blur_radius: px(SHADOW_BLUR),
                spread_radius: px(0.),
                inset: false,
            }])
            .text_size(px(TEXT))
            .children(rows)
            .when(self.a11y, |panel| panel.role(Role::Menu).aria_label(name))
            .into_any_element()
    }
}

impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.a11y = window.is_a11y_active();
        // Keys go to the last frame's listeners, so a key typed before the window draws a menu it
        // was just given still reaches it.
        let root = div()
            .size_full()
            .font_family(".SystemUIFont")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|menu, event: &KeyDownEvent, _, cx| menu.key_down(event, cx)));
        let Some(shown) = self.shown.as_ref() else { return root };
        let panels: Vec<_> = (0..shown.tracking.open.len()).map(|depth| self.panel(shown, depth)).collect();
        // On a platform whose host doesn't see the pointer first (the preview), the window's own
        // events drive the menu.
        let on = |event: Pointer| {
            cx.listener(move |menu: &mut Self, at: &Point<Pixels>, _: &mut Window, cx: &mut Context<Self>| {
                menu.pointer(event, *at, cx);
            })
        };
        let (moved, pressed, right) = (on(Pointer::Moved), on(Pointer::Pressed), on(Pointer::RightPressed));
        let (released, right_released) = (on(Pointer::Released), on(Pointer::Released));
        root.on_mouse_move(move |event: &MouseMoveEvent, window, cx| moved(&event.position, window, cx))
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
                pressed(&event.position, window, cx)
            })
            .on_mouse_down(MouseButton::Right, move |event: &MouseDownEvent, window, cx| {
                right(&event.position, window, cx)
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, window, cx| {
                released(&event.position, window, cx)
            })
            .on_mouse_up(MouseButton::Right, move |event: &MouseUpEvent, window, cx| {
                right_released(&event.position, window, cx)
            })
            .children(panels)
    }
}

#[cfg(test)]
mod tests;
