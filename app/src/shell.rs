//! The browser window's UI, under the page: the sidebar and the card the page sits in, at Arc's
//! measured sizes (docs/ux-spec.md › Window and › Sidebar). The shell shows what the browser tells
//! it and asks the browser for changes, so the engine stays the one place tabs live.

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Bounds, BoxShadow, Context, DispatchPhase, Entity, FocusHandle, Focusable,
    FontWeight, Hsla, Image, MouseButton, MouseExitEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Rgba, ScrollHandle, SharedString, Size, Subscription, Task, Window, canvas, div, img,
    point, prelude::*, px, rgb, rgba, svg,
};

use crate::text_input::TextInput;

mod assets;
mod browser;
mod clean_link;
mod commands;
mod drag;
mod extensions;
mod favorites;
mod fly_in;
mod footer;
mod keys;
mod header;
mod library;
mod menus;
mod naming;
mod new_space;
mod panes;
mod permission;
mod rows;
pub(crate) mod scaled;
mod sidebar;
mod space_form;
mod split_targets;
mod swipe;
mod tile_theme;

pub use assets::Assets;
pub use browser::{Browser, Folders, Page, Split, WindowButton};
pub use commands::{
    BarCommand, CancelName, Command, CreateSpace, KEY_CONTEXT, KeepName, bar_commands, bind_keys,
    copy_from_bar, keystroke_label, menu_item, pane_added, run_from_bar, shown_tab,
};
pub use drag::dropped_urls;
pub use extensions::{
    Extension, ExtensionAction, ExtensionCommand, menu_item as extension_menu_item,
    pin_menu_item as extension_pin_menu_item,
};
pub use keys::{Holder, Refused, UserKeys, change_keys, rebindable, recorded, reserved, user_keys};
pub use library::{Archived, Download, DownloadAction, DownloadState};
pub use menus::{MenuEntry, new_profile_name};
pub use panes::page_frame;
pub use permission::{Answer, Question};
use clean_link::clean_link;
use commands::ShellHandle;
use fly_in::Flight;
use library::{RECEDE_X, Section};
use menus::MenuAction;
use panes::pane_frames;
use scaled::scaled;
use tile_theme::TileTheme;

/// The docked sidebar's width at first, where the card starts.
pub const SIDEBAR_WIDTH: f32 = 228.;
/// The card's inset from the window's top, right and bottom edges.
pub const CARD_INSET: f32 = 10.;
/// The radius of the card's corners and the page's in it: Arc's, a circle of 12 pixels in its
/// window at 2×.
pub const CARD_RADIUS: f32 = 6.;
/// Between the pages of a split view, as in Arc. The core's is the same.
const SPLIT_GAP: f32 = 10.;
/// The most pages a split view shows, as the core's Space has it.
const MAX_PANES: usize = 3;
/// The glow outside a split view's focused page, in its ring's colour (Arc: ≈ 8 pt).
const FOCUS_GLOW: f32 = 8.;
const FOCUS_GLOW_ALPHA: f32 = 0.5;
// Dragged, the gap between a split view's pages leaves each a tenth of the card, no less than 50 pt
// and no more than 200, and snaps them even within 15 pt, as Chrome's does.
const PANE_LEAST: (f32, f32) = (50., 200.);
const PANE_SNAP: f32 = 15.;

// ⌘S hides and shows the sidebar on Arc's ease-out cubics (docs/ux-spec.md › Motion: Sidebar hide
// and Sidebar show).
const SIDEBAR_HIDE: f32 = 0.11;
const SIDEBAR_SHOW: f32 = 0.1;
// The sidebar's edge drags its width between Ardia's limits (docs/ux-spec.md › Sidebar › Width),
// rubber-banding past them as the Space pages do. Let go past one, it settles there as they settle,
// and a double-click on the edge settles it back at the width it starts at. The edge is the margin
// right of the rows.
const SIDEBAR_MIN: f32 = 160.;
const SIDEBAR_MAX: f32 = 400.;
const SIDEBAR_EDGE: f32 = 6.;

// The sidebar reveal (docs/ux-spec.md › Sidebar reveal): the hidden sidebar slides over the page as
// a panel once the pointer has been at the window's left edge for a moment, and slides away once
// the pointer is past the hold zone, both on Arc's springs. Its window starts at the browser
// window's top-left corner, so a point is at the same place in both.
pub const REVEAL_INSET: f32 = 5.;
const REVEAL_HOLD: f32 = 24.;
const REVEAL_WAIT: Duration = Duration::from_millis(100);
const REVEAL_IN_RESPONSE: f32 = 0.175;
const REVEAL_IN_DAMPING: f32 = 0.8;
const REVEAL_OUT_RESPONSE: f32 = 0.175;
const REVEAL_OUT_DAMPING: f32 = 0.76;
/// The panel's window at rest ends at its edge, and is this much wider while it moves, for the
/// spring's overshoot.
const REVEAL_OVERSHOOT: f32 = 8.;

// Rows make room and close it, a closed tab's row fades and new rows fade in on one spring, Arc's
// (docs/ux-spec.md › Motion: Row motion). A row's hover fill fades out on Row fill change.
const ROW_RESPONSE: f32 = 0.13;
const ROW_DAMPING: f32 = 0.9;
const ROW_FILL: f32 = 0.3;
// A pressed row comes away once the pointer has moved this far (ours), and the rows around a
// dragged one make way critically damped (docs/ux-spec.md › Motion: Reorder).
const DRAG_START: f32 = 4.;
const REORDER_OMEGA: f32 = 90.;
// A dragged row turning into a tile has faded its title by this far through the turn (ours).
const TURN_TITLE_FADE: f32 = 0.5;
// The rows under a folder make room as it opens and close up as it closes on Dia's group spring
// (docs/ux-spec.md › Motion: Group expand).
const FOLD_RESPONSE: f32 = 0.3;
const FOLD_DAMPING: f32 = 0.82;
// The edge under the favorites fades in once the list scrolls past its top and out back there, on
// Dia's Scroll edge curve (docs/ux-spec.md › Motion).
const SCROLL_EDGE_FADE: f32 = 0.1;
// Clear and the URL field's button fade in as the pointer comes onto the sidebar and out as it
// leaves, on the scroll edge's curve. Arc's fade isn't recorded (docs/ux-spec.md › Clear divider).
const TOOLS_FADE: f32 = 0.15;
// Two fingers across the sidebar page between Spaces (docs/ux-spec.md › Spaces › Switching). The
// pages follow at half the fingers' way and rubber-band past the ends. Let go, they settle
// critically damped on the next Space if the last 100 ms moved them faster than a flick, else on
// the nearest, from where the fingers left them and as fast, with Arc's response. A wheel pages
// once per burst of ticks, then rests. AppKit's line is about 10 pt.
// Past the first Space is the Library, which the fingers bring out and put away the same way.
const SWIPE_TRACKING: f32 = 0.5;
const SWIPE_FLICK: f32 = 5.;
const SWIPE_HISTORY: Duration = Duration::from_millis(100);
const SPACE_SETTLE: f32 = 0.19;
const WHEEL_LINE: f32 = 10.;
const WHEEL_BURST: Duration = Duration::from_millis(49);
const WHEEL_REST: Duration = Duration::from_millis(250);

// The header's buttons: their plates' tops and left edges, from the window's, as Arc 1.170's
// accessibility tree has them. The sidebar button is 3 pt past the traffic lights, 13 before back.
const NAV_TOP: f32 = 7.5;
const SIDEBAR_BUTTON_X: f32 = 77.;
const BACK_X: f32 = 122.;
const FORWARD_X: f32 = 156.;
const RELOAD_X: f32 = 190.;
const NAV_SIZE: f32 = 32.;
const NAV_RADIUS: f32 = 6.;
// Header glyphs are drawn in points in a box this big, at Arc's sizes and centres.
const NAV_GLYPH: f32 = 24.;
// A header button shows while its plate ends this far inside the sidebar, so a narrow sidebar loses
// reload, then forward, as Ardia's does.
const NAV_END: f32 = 5.;

const URL_FIELD_TOP: f32 = 46.;
const URL_FIELD_HEIGHT: f32 = 36.;
const URL_FIELD_TEXT_X: f32 = 21.;
// The URL field's button, which shows while the pointer is over the sidebar: 30 pt, as Arc's, 3 pt
// inside the field's end (ours: Arc's frames end 1 pt past where its field would).
const URL_BUTTON_SIZE: f32 = 30.;
const URL_BUTTON_INSET: f32 = 3.;
const URL_BUTTON_GLYPH: f32 = 14.;
// Arc puts favorites between the URL field and the Space's name. This is their gap while there are
// none, which Arc has no layout for.
const LIST_TOP: f32 = URL_FIELD_TOP + URL_FIELD_HEIGHT + 8.;
// The favorites' tiles, over the Space's name (docs/ux-spec.md › Favorites): their height, the gap
// between them and under the last row, the narrowest a row's tiles get before it takes one fewer,
// and their corners.
const TILE_HEIGHT: f32 = 41.;
const TILE_GAP: f32 = 6.;
const TILE_MIN: f32 = 48.;
const TILE_RADIUS: f32 = 10.;
// Favorites taller than this much of the window scroll with the list instead of staying put
// (Ardia), so they never leave the rows too little room.
const TALL_FAVORITES: f32 = 0.4;
// The Space's name, over its tabs. Arc puts its icon at x 13, and Spaces have none yet.
const NAME_HEIGHT: f32 = 44.;
const NAME_X: f32 = 41.;
const ROW_PITCH: f32 = 41.;
const ROW_HEIGHT: f32 = 40.;
const PLATE_HEIGHT: f32 = 36.;
const PLATE_INSET: f32 = 8.;
// What's in a folder sits this far in from it (Arc).
const FOLDER_INDENT: f32 = 16.;
const NEW_FOLDER: &str = "New Folder";
pub const PLATE_RADIUS: f32 = 12.;
const FAVICON_X: f32 = 17.;
const FAVICON_SIZE: f32 = 18.;
// Arc's titles end as far from the sidebar's edge as its favicons start.
const TITLE_END_INSET: f32 = FAVICON_X - PLATE_INSET;
// The engine's favicons are 16 pt, drawn at 2×, so they sit in the middle of Arc's 18 pt slot
// rather than scaled up soft.
const FAVICON_DRAWN: f32 = 16.;
/// An emoji's text size in a favicon's place, for its glyph to fill what a favicon does.
const EMOJI_SIZE: f32 = 13.;
const LOADING_SIZE: f32 = 12.;
// A row narrower than this, such as a split's segment in a narrow sidebar, leaves the spinner out
// (Dia), as it would leave the title no room.
const LOADING_ROW_MIN: f32 = 48.;
// One clockwise turn (docs/ux-spec.md › Motion).
const LOADING_TURN: Duration = Duration::from_millis(1880);
const TITLE_GAP: f32 = 8.;
// The speaker before a title, while its tab plays sound or is muted, and its pressable square.
const SPEAKER_DRAWN: f32 = 16.;
const SPEAKER_SIZE: f32 = 18.;
const SPEAKER_GAP: f32 = 4.;
// Arc's close button, 5 inside the plate's right edge.
const CLOSE_SIZE: f32 = 26.;
const CLOSE_RIGHT: f32 = 5.;
// A split's row (Arc): a segment per tab, all as wide, each after the first behind a line. Its
// favicon is 7 pt after the line, its title 10 pt after the favicon, and it ends 5 pt before the
// next line.
const SEGMENT_FAVICON_X: f32 = 7.;
const SEGMENT_TITLE_GAP: f32 = 10.;
const SEGMENT_END: f32 = 5.;
const SEGMENT_LINE: f32 = 16.;
// Arc's divider line is one device pixel, 18 in from each side of the sidebar. GPUI widens a border
// under one device pixel to one, so borders use this 0.5 pt; a line that's a box's height has to
// ask the window for its pixel, or it's rounded away at 1x.
const HAIRLINE: f32 = 0.5;
const DIVIDER_X: f32 = 18.;
// The Clear divider, under the pinned tabs. Its line's centre is 25 pt above the centre of the first
// row under it. While Clear shows, the line ends 9 pt before it.
const DIVIDER_HEIGHT: f32 = 15.;
const DIVIDER_LINE_Y: f32 = DIVIDER_HEIGHT - (25. - ROW_HEIGHT / 2.);
const DIVIDER_GAP: f32 = 9.;
// The arrow before Clear on a Library card, 8 pt tall and 3 pt before the word (Arc). Lucide's
// arrow-down spans 14 of its 24.
const CLEAR_ARROW: f32 = 8. * 24. / 14.;
const CLEAR_ARROW_INSET: f32 = CLEAR_ARROW * 5. / 24.;
const CLEAR_GAP: f32 = 3.;
const CLEAR_HEIGHT: f32 = 20.;
// How long the toast after a copy shows (ours: Arc's isn't measured), and after a Space is deleted
// (Arc).
const COPIED_TIME: Duration = Duration::from_secs(2);
const DELETED_TIME: Duration = Duration::from_secs(5);
// The footer: the Space switcher between the Library button and the +, which is 8 in from the
// sidebar's right edge (Arc 1.170.0 ax). The switcher keeps 42 from each side, Arc's at 228.
const FOOTER_HEIGHT: f32 = 50.;
const SWITCHER_INSET: f32 = 42.;
const PLUS_INSET: f32 = 8.;
const SPACE_PITCH: f32 = 34.;
const SPACE_HEIGHT: f32 = 34.;
const SPACE_DOT: f32 = 8.;
// The Create a Space form, in Arc's places and type, without its illustration and subtitle, which
// are Arc's art and copy. The field moves up into the subtitle's place, Arc's 35 pt under the text
// above it. Distances to the buttons are from the window's bottom.
const FORM_TITLE_TOP: f32 = 140.;
const FORM_TITLE_HEIGHT: f32 = 20.;
const FORM_FIELD_TOP: f32 = FORM_TITLE_TOP + FORM_TITLE_HEIGHT + 35.;
const FORM_INSET: f32 = 14.;
const FORM_PLATE_HEIGHT: f32 = 36.;
const FORM_RADIUS: f32 = 8.;
// Arc's name starts at x 48, after the Space's icon, which Spaces don't have yet. This is the URL
// field's padding.
const FORM_TEXT_X: f32 = FORM_INSET + URL_FIELD_TEXT_X - PLATE_INSET;
// Arc's Profile and Choose a Theme rows, 42 and 86 under the field's top: a glyph centred 30 in, a
// label at 46.5, and the Space's profile in a pill at 95, 10 after the label, its name 8 in and its
// chevron 6.5 after the name and 10 from the pill's end, which the chevron's box takes 4.5 of.
const FORM_PROFILE_TOP: f32 = FORM_FIELD_TOP + 42.;
const FORM_THEME_TOP: f32 = FORM_FIELD_TOP + 86.;
const FORM_GLYPH_X: f32 = 30.;
const FORM_GLYPH: f32 = 15.;
const FORM_LABEL_X: f32 = 46.5;
const FORM_PILL_X: f32 = 95.;
const FORM_PILL_HEIGHT: f32 = 22.;
const FORM_PILL_LEFT: f32 = 8.;
const FORM_CHEVRON: f32 = 18.;
const FORM_CHEVRON_GAP: f32 = 6.5 - FORM_CHEVRON / 4.;
const FORM_PILL_RIGHT: f32 = 10. - FORM_CHEVRON / 4.;
const CREATE_BOTTOM: f32 = 96.;
const CANCEL_BOTTOM: f32 = 66.5;
const CANCEL_HEIGHT: f32 = 15.;

// The window's layers over its material (docs/ux-spec.md › Window material): a tint of black 0.4,
// then the Space's gradient, its stops at 0.36 (0.12 for a neutral colour) in a layer at 0.5.
const TINT: u32 = 0x0000_0066;
const GRADIENT_ALPHA: f32 = 0.18;
const NEUTRAL_GRADIENT_ALPHA: f32 = 0.06;
// A colour under this HSL saturation is neutral (ours: Dia's line isn't measured).
const NEUTRAL: f32 = 0.1;
// The colours a Space's menu offers: Dia's hue families, in its order. Its pink is measured, ≈
// #BF556A; the others are ours, at the pink's HSL saturation and lightness.
const SPACE_COLORS: [(&str, u32); 8] = [
    ("Gray", 0x8a8a8a),
    ("Red", 0xbf5c55),
    ("Orange", 0xbf8355),
    ("Yellow", 0xbfa655),
    ("Green", 0x55bf81),
    ("Blue", 0x5583bf),
    ("Pink", 0xbf556a),
    ("Purple", 0x8655bf),
];
// What stands in for the material where the window's look has to be opaque, under rows it covers
// or in the reveal's panel over the page: the material over grey 20, under the tint (ours).
const MATERIAL_STAND_IN: u32 = 0x131313;
// Dia's card, #121212 at 0.5: what shows before a page first paints.
const CARD: u32 = 0x1212_1280;
// Arc's row tokens (docs/ux-spec.md › Tab rows): the tab shown is white 0.2, a row under the
// pointer 0.12 and a pressed one 0.065, with no line or shadow in dark mode. Titles are Dia's,
// white, at 0.78 unless selected (ours: Arc's aren't measured).
const TITLE: u32 = 0xffffffff;
const UNSELECTED_TITLE: u32 = 0xffffffc7;
const SELECTED: u32 = 0xffff_ff33;
const HOVERED: u32 = 0xffff_ff1f;
const PRESSED: u32 = 0xffff_ff11;
const SECONDARY: u32 = 0xffffff99;
// The close button's square under the pointer: Dia's subtle fill, white 0.12, at the header
// buttons' radius (ours: Dia's fill isn't measured).
const CLOSE_HOVERED: u32 = 0xffffff1f;
const URL_FIELD: u32 = 0xffffff1a;
const NEW_TAB: u32 = 0xffffff8c;
const NEW_TAB_PLUS: u32 = 0xffffff40;
// The Clear divider's line, white 0.15 (Arc), and its button (ours: Arc's colour isn't measured).
const DIVIDER: u32 = 0xffffff26;
// The edge under the favorites while the list is scrolled: Dia's separator, white 0.09.
const SCROLL_EDGE: u32 = 0xffffff17;
const CLEAR: u32 = 0xffffff73;
// A tab whose page isn't loaded fades its favicon (ours: Arc's alpha isn't measured).
const SLEEPING_FAVICON: f32 = 0.5;
// A favorite's tile: white 0.10 with a white 0.14 edge at rest (Dia), and under the pointer white
// 0.15 (ours: Dia's isn't measured). The one shown is themed by its icon (tile_theme.rs), and
// black with Dia's default selection ring, white 0.45, where the icon gives no theme.
const TILE: f32 = 0.1;
const TILE_EDGE: u32 = 0xffff_ff24;
const TILE_HOVERED: f32 = 0.15;
const TILE_RING: u32 = 0xffff_ff73;
const TILE_UNTHEMED: u32 = 0x0000_00ff;
// The header's glyphs (docs/ux-spec.md › Header and URL field): white 0.60, and 0.26 for back or
// forward with nowhere to go, over a hover plate of white 0.077.
const NAV: u32 = 0xffffff99;
const NAV_DISABLED: u32 = 0xffffff42;
const NAV_HOVERED: u32 = 0xffffff14;
// The Space's name, white 0.56, and the switcher's dots: white 0.5 for the Space shown and 0.19
// for the others (Arc), 0.3 under the pointer (ours: Arc's isn't measured).
const SPACE_NAME: u32 = 0xffffff8f;
const SPACE_SHOWN: u32 = 0xffffff80;
const SPACE_OTHER: u32 = 0xffffff30;
const SPACE_HOVERED: u32 = 0xffffff4d;
// The form (Arc): a white title, plates of white 0.075, a placeholder of white 0.55, and Create
// Space in Arc's blue with a white label, all of it at 0.4 until there's a name.
const FORM_TEXT: u32 = 0xffffff;
const FORM_PLATE: u32 = 0xffffff13;
const FORM_PLACEHOLDER: u32 = 0xffffff8c;
// The rows' glyphs are white 0.5, and the profile's pill white 0.1 over its row.
const FORM_GLYPH_COLOR: u32 = 0xffffff80;
const FORM_PILL: u32 = 0xffffff1a;
const CREATE: u32 = 0x2129fb;
const CREATE_UNNAMED: f32 = 0.4;
// Create sends the form away from the bottom up, as Arc does (docs/ux-spec.md › Creating): the
// buttons fade, then the rows, the field and the title fade as they sink, each over 150 ms.
const FORM_LEAVE: f32 = 0.15;
const FORM_LEAVE_ROWS: f32 = 0.04;
const FORM_LEAVE_FIELD: f32 = 0.08;
const FORM_LEAVE_TITLE: f32 = 0.13;
const FORM_SINK: f32 = 20.;
// The reveal panel's radius and its 0.5 pt light border, as in a capture of Arc at 1×: a circle of
// 12, and one pixel of white 0.27 over the panel, which half a pixel of white 0.54 makes.
const REVEAL_RADIUS: f32 = 12.;
const REVEAL_BORDER: u32 = 0xffffff8a;
// The panel draws the window's buttons itself, since AppKit's stay in the browser window, under
// it: their centres from the panel's corner, as the docked sidebar's are from the window's, their
// size, and their colours, sampled from AppKit's in a capture of the browser window. The glyphs
// show while the pointer is over any of them, as AppKit's do.
const WINDOW_BUTTONS: [(f32, u32); 3] = [(20., 0xe0604f), (43., 0xf3bf12), (66., 0x58bb18)];
const WINDOW_BUTTON_Y: f32 = 24.;
const WINDOW_BUTTON_SIZE: f32 = 14.;
const WINDOW_BUTTON_GLYPH: u32 = 0x00000080;

/// What ⌘Z undoes: the last of what the user did that it can.
#[derive(Clone, Copy)]
enum Undo {
    Clear,
    DeleteSpace,
}

#[derive(Default)]
struct Tab {
    title: SharedString,
    page: Page,
    /// None for a page without one.
    favicon: Option<Arc<Image>>,
    /// What the favicon (by its id) themes the tile with when the tab is a favorite shown, made
    /// the first time that's drawn.
    theme: RefCell<Option<(u64, Option<Rc<TileTheme>>)>>,
}

struct Space {
    id: u64,
    name: SharedString,
    /// 0xRRGGBB, if it has one.
    color: Option<u32>,
    /// The profile its tabs open in, whose favorites it shows.
    profile: u64,
    /// Its pinned tabs and folders and its Today tabs, each top to bottom.
    pinned: Vec<u64>,
    today: Vec<u64>,
    splits: Vec<Split>,
    folders: Folders,
    /// The tab it shows, or showed last.
    shown: u64,
}

impl Space {
    /// The tabs and folders of its pinned rows and its Today rows: each split's first stands for
    /// the split, and what's in a closed folder has none, but the tab shown's (Arc).
    fn rows(&self) -> (Vec<u64>, Vec<u64>) {
        let later = |tab: &u64| self.splits.iter().any(|split| split.tabs[1..].contains(tab));
        let rows = |tabs: &[u64]| tabs.iter().copied().filter(|tab| !later(tab)).collect();
        let shown = self.panes(self.shown)[0];
        let rows_shown = |row: &u64| !self.shut(*row) || *row == shown;
        let pinned = self.pinned.iter().copied().filter(|row| !later(row) && rows_shown(row));
        (pinned.collect(), rows(&self.today))
    }

    /// Whether `row` is in a closed folder.
    fn shut(&self, row: u64) -> bool {
        self.holders(row).any(|folder| !self.is_open(folder))
    }

    fn is_folder(&self, row: u64) -> bool {
        self.folders.folders.contains_key(&row)
    }

    fn is_open(&self, folder: u64) -> bool {
        self.folders.folders.get(&folder).is_some_and(|&(_, open)| open)
    }

    /// The folder holding `row`, else 0.
    fn folder_of(&self, row: u64) -> u64 {
        self.folders.parents.get(&row).copied().unwrap_or(0)
    }

    /// The folders `row` is in, innermost first.
    fn holders(&self, row: u64) -> impl Iterator<Item = u64> + '_ {
        std::iter::successors(Some(self.folder_of(row)), |&folder| Some(self.folder_of(folder)))
            .take_while(|&folder| folder != 0)
    }

    fn depth(&self, row: u64) -> usize {
        self.holders(row).count()
    }

    /// Its pinned tabs and folders as check.sh reads them: each folder followed by `+` if it's
    /// open, else `-`, and what's in it in brackets.
    fn outline(&self, folder: u64) -> String {
        let rows = self.pinned.iter().filter(|&&row| self.folder_of(row) == folder);
        let rows = rows.map(|&row| match self.is_folder(row) {
            true => {
                let open = if self.is_open(row) { '+' } else { '-' };
                format!("{row}{open}[{}]", self.outline(row))
            }
            false => row.to_string(),
        });
        rows.collect::<Vec<_>>().join(" ")
    }

    /// The tabs shown with `tab`, left to right: its split's, else `tab` alone.
    fn panes(&self, tab: u64) -> Vec<u64> {
        let split = self.splits.iter().find(|split| split.tabs.contains(&tab));
        split.map_or_else(|| vec![tab], |split| split.tabs.clone())
    }

    /// Whether `tab`'s split view is stacked.
    fn stacked(&self, tab: u64) -> bool {
        self.splits.iter().any(|split| split.stacked && split.tabs.contains(&tab))
    }

    /// The shares of `tab`'s split view, or none while they're even.
    fn shares(&self, tab: u64) -> Vec<f64> {
        let split = self.splits.iter().find(|split| split.tabs.contains(&tab));
        split.map_or_else(Vec::new, |split| split.shares.clone())
    }

    /// Where its rows are in the list, from its top.
    fn slots(&self) -> HashMap<Slot, f32> {
        let (pinned, today) = self.rows();
        slots(&pinned, &today)
    }
}

/// The Create a Space form, which a Space New Space made shows until it's named, cancelled or has
/// a tab.
struct SpaceForm {
    space: u64,
    /// The Space shown before, which Cancel shows again.
    from: u64,
    name: Entity<TextInput>,
}

/// What the renaming field names.
#[derive(Clone, Copy, PartialEq)]
enum Naming {
    Space(u64),
    Tab(u64),
    Folder(u64),
    /// A tab's icon, in a field in its place.
    Icon(u64),
}

/// The field a Space's name or a tab's title turns into for renaming it, or a tab's icon for
/// changing it.
struct Renaming {
    naming: Naming,
    name: Entity<TextInput>,
    /// Keeps the name once the field loses GPUI's focus, as Finder does.
    _blur: Option<Subscription>,
    /// Keeps an icon as soon as it's typed or picked.
    _edit: Option<Subscription>,
}

#[derive(Clone, Copy, PartialEq)]
enum Row {
    NewTab,
    Tab(u64),
}

/// A place in the list that moves as tabs come and go.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Slot {
    Tab(u64),
    Divider,
    NewTab,
}

/// Tabs' rows new in the list, fading in since `start`: a new tab's plate, or whole rows when
/// several come at once, as after Undo.
struct Opened {
    tabs: Vec<u64>,
    /// Whether whole rows fade in, else only a new tab's plate.
    whole: bool,
    start: Instant,
}

impl Opened {
    /// How far they've faded in, or None once they have.
    fn done(&self) -> Option<f32> {
        row_spring(self.start.elapsed().as_secs_f32())
    }
}

/// A closed tab's row, fading where it was.
struct Closing {
    title: SharedString,
    favicon: Option<Arc<Image>>,
    /// The emoji it had in place of its favicon, or empty.
    icon: SharedString,
    /// A folder's row's, if it was open.
    folder: Option<bool>,
    /// How far in from a tab's place it was, in its folders.
    indent: f32,
    top: f32,
    start: Instant,
}

/// How a row slides to its place.
#[derive(Clone, Copy)]
enum Curve {
    /// Making room for a tab's row or closing it (Row motion).
    Slide,
    /// Making way for a dragged row (Reorder).
    Reorder,
    /// Making room as a folder opens, or closing it up as it closes (Group expand).
    Fold,
}

impl Curve {
    /// How far along the slide is `t` seconds in, or None once it's done.
    fn at(self, t: f32) -> Option<f32> {
        match self {
            Curve::Slide => row_spring(t),
            Curve::Reorder => {
                let left = (1. + REORDER_OMEGA * t) * (-REORDER_OMEGA * t).exp();
                (left > 0.01).then_some(1. - left)
            }
            Curve::Fold => spring(t, FOLD_RESPONSE, FOLD_DAMPING),
        }
    }
}

/// Where a dragged tab is or would go: among the favorites, or in the pinned tabs or Today, at an
/// index of them without it.
#[derive(Clone, Copy, PartialEq)]
enum Place {
    Favorite(usize),
    List(bool, usize),
}

/// A tab's row or tile the pointer pressed, which follows the pointer once it has moved far enough.
/// Over the favorites it's drawn as a tile, and over the list as a row. Or links or files dragged
/// over the list from the page or another app, which the rows make room for in the same way.
struct Drag {
    /// For links or files, INCOMING until they're dropped, then the first one's tab.
    tab: u64,
    incoming: bool,
    /// Where the pointer pressed, and the sidebar's top-left corner in that window.
    press: Point<Pixels>,
    origin: Point<Pixels>,
    /// Where the pointer is now, from the sidebar's corner.
    pointer: Point<f32>,
    /// Where the pointer holds the tab: down from its row's top, and from its tile's centre.
    grab: (f32, Point<f32>),
    /// The row's top in the list when it was pressed. A favorite's is 0.
    top: f32,
    /// Whether the tab has come away, and how far up and down the list its row goes: between the
    /// list's first and last places.
    moving: bool,
    span: (f32, f32),
    from: Place,
    to: Place,
    /// How many folders it was in, and would be in where it goes.
    from_depth: usize,
    depth: usize,
    /// How far the other rows are drawn from their places to make room for it there, and whether
    /// the divider stays, which goes once the last Today tab would leave.
    room: HashMap<Slot, f32>,
    divider: bool,
    /// Let go, until the browser reports the lists it made.
    dropped: bool,
    /// Over the page, where it can join the page shown in a split view.
    over_page: Option<split_targets::OverPage>,
    /// Its turn between its row and its tile, and whether the pointer let go mid-turn, which
    /// holds the drop until it has turned.
    shape: Shape,
    held: bool,
}

/// How far a dragged tab has turned from its row into its tile as it crossed into or out of the
/// favorites, on the Reorder spring the tiles make room on (ours, docs/ux-spec.md › Favorites).
#[derive(Clone, Copy)]
struct Shape {
    /// Where it turned from, whether it's turning into a tile, and since when, until it has.
    from: f32,
    tile: bool,
    start: Option<Instant>,
}

impl Shape {
    fn new(tile: bool) -> Self {
        Shape { from: if tile { 1. } else { 0. }, tile, start: None }
    }

    /// How far from its row to its tile it is at `now`: 0 a row, 1 a tile.
    fn at(&self, now: Instant) -> f32 {
        let to = if self.tile { 1. } else { 0. };
        let done = self.start.and_then(|start| {
            Curve::Reorder.at(now.duration_since(start).as_secs_f32())
        });
        done.map_or(to, |done| self.from + (to - self.from) * done)
    }

    /// Turns it toward its tile or its row from where it is at `now`, so a crossing back mid-turn
    /// reverses with no jump.
    fn turn(&mut self, tile: bool, now: Instant) {
        if tile != self.tile {
            *self = Shape { from: self.at(now), tile, start: Some(now) };
        }
    }

    /// Whether it's still turning at `now`.
    fn turning(&self, now: Instant) -> bool {
        self.start.is_some_and(|start| {
            Curve::Reorder.at(now.duration_since(start).as_secs_f32()).is_some()
        })
    }
}

/// Two fingers on the trackpad over the sidebar, which may page between Spaces.
struct Swipe {
    /// The Space shown when they came down.
    from: u64,
    /// Whether they came down in the Library, or as it moved, so they move it.
    library: bool,
    /// Across the sidebar, to page, or along it, to scroll the list. None until they first move.
    across: Option<bool>,
    /// How far they've taken the page of the Space they began at, before the rubber band.
    way: f32,
    /// Where that page was over the last SWIPE_HISTORY, for how fast it's let go.
    samples: VecDeque<(Instant, f32)>,
    /// The Space nearest its place, for the detent as it changes.
    nearest: u64,
    /// Whether they've pulled the last Space far enough to ready a new Space, and when that last
    /// changed.
    ready: bool,
    turned: Option<Instant>,
}

/// A Space's page settling on its place, after a swipe or a switch.
struct Settle {
    space: u64,
    /// The page it pushes out: the Space shown before a switch. After a swipe it's the Space next
    /// to it on the side it's coming from.
    beside: Option<u64>,
    /// How far from its place it started, and how fast it was going.
    from: f32,
    velocity: f32,
    start: Instant,
}

impl Settle {
    /// How far the page is from its place, or None once it's there.
    fn offset(&self) -> Option<f32> {
        let omega = std::f32::consts::TAU / SPACE_SETTLE;
        let t = self.start.elapsed().as_secs_f32();
        let decay = (-omega * t).exp();
        let rise = self.velocity + omega * self.from;
        let offset = (self.from + rise * t) * decay;
        let speed = (self.velocity - omega * rise * t) * decay;
        (offset.abs() > 0.25 || speed.abs() > 10.).then_some(offset)
    }
}

pub struct Shell {
    browser: Rc<dyn Browser>,
    /// Holds GPUI's focus while nothing in the shell takes it, so the shortcuts have their context.
    focus_handle: FocusHandle,
    /// In the switcher's order.
    spaces: Vec<Space>,
    /// The Space the window shows.
    space: u64,
    form: Option<SpaceForm>,
    /// The form Create sent away, and when, until it's gone and its Space shows its rows.
    leaving_form: Option<(SpaceForm, Instant)>,
    /// The form's field takes GPUI's focus at the next draw, which is when the shell has a window.
    focus_form: bool,
    renaming: Option<Renaming>,
    /// The same for the renaming field.
    focus_renaming: bool,
    /// The pinned tabs and folders changed and haven't been drawn since.
    outline_changed: bool,
    /// The favorites of the shown Space's profile, left to right, which every Space in it shows
    /// over its own tabs.
    favorites: Vec<u64>,
    /// Every profile's favorites, and each one's profile.
    all_favorites: Vec<u64>,
    favorite_profiles: HashMap<u64, u64>,
    /// The profiles, by id and name, the one the browser started with first.
    profiles: Vec<(u64, SharedString)>,
    /// The tabs of every Space, and the favorites.
    tabs: HashMap<u64, Tab>,
    selected: u64,
    /// Tabs of every Space, shown most recently first.
    recent: Vec<u64>,
    /// ⌃ is held after a ⌃Tab, which showed this tab. The tabs a walk shows don't count as shown
    /// until ⌃ goes up, and then its last one does, though a click may have shown another since.
    walked: Option<u64>,
    hovered: Option<Row>,
    /// The tab whose menu is up, whose row keeps its hover fill until the menu closes.
    menu_row: Option<u64>,
    /// Rows the pointer left, whose hover fill is fading, and since when.
    unhovered: Vec<(Row, Instant)>,
    /// Rows on their way to their place: how far from it each started, and when.
    slides: HashMap<Slot, (f32, Instant, Curve)>,
    /// The user opened or closed a folder, whose rows move once the browser reports it.
    folding: bool,
    /// The same for tiles, by their left, top and width, and for the list's top as the favorites
    /// take a row more or fewer.
    tile_slides: HashMap<u64, ([f32; 3], Instant, Curve)>,
    list_slide: Option<(f32, Instant, Curve)>,
    /// The list's scroll, and the sidebar's top-left corner in the window the last press was in,
    /// for where the pointer is in them.
    list_scroll: ScrollHandle,
    sidebar_origin: Rc<Cell<Point<Pixels>>>,
    /// Whether the list is past its top, and the edge under the favorites' fade since it last
    /// went past or back, from how far in it was then.
    scrolled: bool,
    scroll_edge_fade: Option<(Instant, f32)>,
    /// Whether Clear and the URL field's button were fading in or shown at the last frame, the
    /// fade since it last changed, from how far in they were then, and whether they were drawn
    /// when the last line was logged.
    tools_on: bool,
    tools_fade: Option<(Instant, f32)>,
    drawn_tools: bool,
    /// Whether the tiles scrolled with the list in the last frame.
    tiles_scrolled_last: bool,
    /// Tabs' new rows, which the rows sliding down to make room come out from beneath.
    opened: Option<Opened>,
    closing: Vec<Closing>,
    /// The tabs Clear is closing, whose rows go in one frame.
    clearing: Vec<u64>,
    /// ⌘T's command bar is open, and "New Tab" has the selected plate in place of the tab shown
    /// (Arc).
    new_tab: bool,
    drag: Option<Drag>,
    /// A drag just ended, so the release isn't a click on the row.
    dragged: bool,
    swipe: Option<Swipe>,
    settle: Option<Settle>,
    /// The last swipe paged, so its momentum, which comes without a touch, doesn't scroll the list.
    swiped: bool,
    /// A wheel's ticks across the sidebar: their sum and the last one's time, and when they last
    /// paged.
    wheel: Option<(f32, Instant)>,
    wheel_paged: Option<Instant>,
    /// The tab whose favicon is under the pointer, which is Back to Pinned URL while it's away.
    hovered_favicon: Option<u64>,
    /// The row and favicon hovered when the rows were last drawn, and whether that row has a ✕,
    /// for the line check.sh waits for.
    drawn_hover: (Option<Row>, Option<u64>, bool),
    undo: Option<Undo>,
    /// Whether the pointer is over the docked sidebar, which shows Clear and the URL field's button
    /// only then.
    sidebar_hovered: bool,
    /// The hover changed since the sidebar was last drawn.
    hover_changed: bool,
    page_fullscreen: bool,
    /// Whether ⌘S has hidden the sidebar.
    sidebar_hidden: bool,
    /// Whether the window's own buttons show (`window_buttons_shown`).
    window_buttons: bool,
    /// The docked sidebar's width, at rest.
    sidebar_width: f32,
    /// While the sidebar's edge is dragged: how far right of the edge the pointer pressed it, and
    /// the width the pointer has it at, before the rubber band.
    resize: Option<(f32, f32)>,
    /// While the gap before a split view's page is dragged: that page's index, and how far past
    /// its start the pointer pressed the gap.
    pane_resize: Option<(usize, f32)>,
    /// While the width settles on `sidebar_width`: the width it started from, and when.
    width_motion: Option<(f32, Instant)>,
    /// While the sidebar moves: where the card's left edge started from, and when.
    sidebar_motion: Option<(f32, Instant)>,
    /// Whether the hidden sidebar is out over the page, or on its way.
    revealed: bool,
    /// While the reveal panel moves: how far out it started (`reveal_out`), and when.
    reveal_motion: Option<(f32, Instant)>,
    /// The pointer is at the window's left edge, waiting to bring the sidebar out.
    reveal_wait: Option<Task<()>>,
    /// The Copy URL button shows a checkmark until this one-shot timer ends.
    url_copied: Option<Task<()>>,
    /// Whether the button had the checkmark when last drawn, for the line check.sh waits for.
    drawn_copied: bool,
    /// The window's content size at the last draw, and the frame and corner radius the page was
    /// last laid out at.
    viewport: Size<Pixels>,
    /// The window's device pixels in a point, kept from render as `viewport` is.
    scale: f32,
    page_laid: Option<(Bounds<Pixels>, f32)>,
    /// While the command bar is open for a pane beside the tab shown, that tab. The pages shown
    /// make room on their right for the pane (Arc).
    adding_pane: Option<u64>,
    /// The colour last given for a split view's focused page.
    focus_color: Option<u32>,
    /// The colour last given for a page before its first frame.
    page_color: Option<u32>,
    /// The window over the page that draws the drop targets shows.
    drop_targets_shown: bool,
    /// How far in from the page frame's top right corner the focused page's is, as last given.
    pane_corner: (f32, f32),
    /// Every Space's archived tabs, newest first, and the favicons asked for their URLs.
    archive: Vec<Archived>,
    page_favicons: HashMap<SharedString, Option<Arc<Image>>>,
    /// The profile's downloads, newest first, and the file icons asked for them.
    downloads: Vec<Download>,
    download_icons: HashMap<u64, Option<Arc<Image>>>,
    /// Every download a report has listed, whether one has come, and the new download's icon on its
    /// way to the Library button.
    downloads_seen: HashSet<u64>,
    downloads_read: bool,
    flight: Option<Flight>,
    /// Each profile's extensions, and those the menu bar lists, the shown Space's turned on.
    extensions: HashMap<u64, Vec<Extension>>,
    menu_extensions: Vec<Extension>,
    /// The names of the Spaces the menu bar lists.
    menu_spaces: Vec<SharedString>,
    /// The pinned actions the window shows, for its tab shown.
    extension_actions: Vec<ExtensionAction>,
    /// The Library's section shown last, which it opens on.
    section: Section,
    /// The Library, from when it opens until it has closed.
    library: Option<library::Library>,
    library_open: bool,
    /// While the Library's width goes to another section's: the width it left, and when.
    library_resize: Option<(f32, Instant)>,
    /// While the card moves for the Library on its own: how far out it started (`library_out`),
    /// how fast, and when.
    library_motion: Option<(f32, f32, Instant)>,
    /// While the sidebar under the Library fades: its opacity when it started, and when.
    sidebar_fade: Option<(f32, Instant)>,
    /// While the circle goes back after a pull made a Space.
    circle_exit: Option<new_space::CircleExit>,
    /// What the tabs' pages ask, such as for the camera: only shown tabs ask.
    questions: HashMap<u64, Question>,
    /// Where the question's card was drawn last, which the log says when it changes.
    question_drawn: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The tab or Space the user asked for and when, until it's shown.
    switching: Option<(u64, Instant)>,
    /// A press on the window's background, until the pointer moves and it moves the window.
    background_pressed: bool,
    /// The system's accent colour, as 0xRRGGBB.
    accent_color: u32,
}

impl Shell {
    /// Makes the shell, the one the browser's shortcuts act on.
    pub fn new(browser: Rc<dyn Browser>, cx: &mut Context<Self>) -> Self {
        cx.set_global(ShellHandle(cx.weak_entity()));
        let sidebar_width = browser
            .saved_sidebar_width()
            .map_or(SIDEBAR_WIDTH, |width| width.clamp(SIDEBAR_MIN, SIDEBAR_MAX));
        let accent_color = browser.accent_color();
        Self {
            browser,
            focus_handle: cx.focus_handle(),
            spaces: Vec::new(),
            space: 0,
            form: None,
            leaving_form: None,
            focus_form: false,
            renaming: None,
            focus_renaming: false,
            outline_changed: false,
            favorites: Vec::new(),
            all_favorites: Vec::new(),
            favorite_profiles: HashMap::new(),
            profiles: Vec::new(),
            tabs: HashMap::new(),
            recent: Vec::new(),
            walked: None,
            selected: 0,
            hovered: None,
            menu_row: None,
            unhovered: Vec::new(),
            slides: HashMap::new(),
            folding: false,
            tile_slides: HashMap::new(),
            list_slide: None,
            list_scroll: ScrollHandle::new(),
            sidebar_origin: Rc::default(),
            scrolled: false,
            scroll_edge_fade: None,
            tools_on: false,
            tools_fade: None,
            drawn_tools: false,
            tiles_scrolled_last: false,
            opened: None,
            closing: Vec::new(),
            clearing: Vec::new(),
            new_tab: false,
            drag: None,
            dragged: false,
            swipe: None,
            settle: None,
            swiped: false,
            wheel: None,
            wheel_paged: None,
            hovered_favicon: None,
            drawn_hover: (None, None, false),
            undo: None,
            sidebar_hovered: false,
            hover_changed: false,
            page_fullscreen: false,
            sidebar_hidden: false,
            window_buttons: true,
            sidebar_width,
            resize: None,
            pane_resize: None,
            width_motion: None,
            sidebar_motion: None,
            revealed: false,
            reveal_motion: None,
            reveal_wait: None,
            url_copied: None,
            drawn_copied: false,
            viewport: Size::default(),
            scale: 1.,
            page_laid: None,
            adding_pane: None,
            focus_color: None,
            page_color: None,
            drop_targets_shown: false,
            pane_corner: (0., 0.),
            archive: Vec::new(),
            page_favicons: HashMap::new(),
            downloads: Vec::new(),
            download_icons: HashMap::new(),
            downloads_seen: HashSet::new(),
            downloads_read: false,
            flight: None,
            extensions: HashMap::new(),
            menu_extensions: Vec::new(),
            menu_spaces: Vec::new(),
            extension_actions: Vec::new(),
            section: Section::Archive,
            library: None,
            library_open: false,
            library_resize: None,
            library_motion: None,
            sidebar_fade: None,
            circle_exit: None,
            questions: HashMap::new(),
            question_drawn: Rc::default(),
            switching: None,
            background_pressed: false,
            accent_color,
        }
    }

    /// The pinned tabs of the Space shown.
    pub fn pinned(&self) -> &[u64] {
        self.shown_space().map_or(&[], |space| &space.pinned)
    }

    /// The Today tabs of the Space shown.
    pub fn today(&self) -> &[u64] {
        self.shown_space().map_or(&[], |space| &space.today)
    }

    /// The split views of the Space shown.
    pub fn splits(&self) -> Vec<Split> {
        self.shown_space().map_or_else(Vec::new, |space| space.splits.clone())
    }

    /// The favorites, which every Space shows.
    pub fn favorites(&self) -> &[u64] {
        &self.favorites
    }

    /// The tabs of the rows of the Space shown, pinned and Today (`Space::rows`).
    fn rows(&self) -> (Vec<u64>, Vec<u64>) {
        self.shown_space().map_or_else(Default::default, Space::rows)
    }

    /// Where the rows of the Space shown are in the list.
    fn layout(&self) -> HashMap<Slot, f32> {
        self.shown_space().map_or_else(|| slots(&[], &[]), Space::slots)
    }

    /// The tabs shown with `tab` in the Space shown, in their order: its split's, else `tab` alone.
    fn panes(&self, tab: u64) -> Vec<u64> {
        self.shown_space().map_or_else(|| vec![tab], |space| space.panes(tab))
    }

    /// Whether `tab`'s split view in the Space shown is stacked.
    fn stacked(&self, tab: u64) -> bool {
        self.shown_space().is_some_and(|space| space.stacked(tab))
    }

    /// The shares of `tab`'s split view in the Space shown, or none while they're even.
    fn shares(&self, tab: u64) -> Vec<f64> {
        self.shown_space().map_or_else(Vec::new, |space| space.shares(tab))
    }

    /// The tab whose row is `tab`'s: its split's first.
    fn row_of(&self, tab: u64) -> u64 {
        self.panes(tab)[0]
    }

    fn is_folder(&self, row: u64) -> bool {
        self.shown_space().is_some_and(|space| space.is_folder(row))
    }

    /// How many folders `row` is in, in the Space shown.
    fn depth(&self, row: u64) -> usize {
        self.shown_space().map_or(0, |space| space.depth(row))
    }

    /// What a row says: a folder's name, else the tab's title.
    fn label(&self, row: u64) -> SharedString {
        match self.is_folder(row) {
            true => self.folder_name(row),
            false => self.title(row),
        }
    }

    /// The folders of the Space shown, for the preview.
    pub fn folders(&self) -> Folders {
        self.shown_space().map(|space| space.folders.clone()).unwrap_or_default()
    }

    /// The tab a row shows: the one of its split shown last, else the first.
    fn row_tab(&self, row: u64) -> u64 {
        let panes = self.panes(row);
        self.recent.iter().copied().find(|tab| panes.contains(tab)).unwrap_or(row)
    }

    /// One device pixel, for a line that is a box's height, which GPUI drops under a pixel.
    pub(super) fn hairline(&self) -> Pixels {
        px(1. / self.scale)
    }

    pub fn space(&self) -> u64 {
        self.space
    }

    /// Where the page goes in a window whose content is `viewport`, and the radius of its corners.
    /// While a pane is being added beside the tab shown, the pages shown make room for it.
    pub fn page_frame(&self, viewport: Size<Pixels>) -> (Bounds<Pixels>, f32) {
        let card_left = self.card_left();
        let frame = page_frame(viewport, card_left, self.card_push());
        let radius = if card_left == 0. { 0. } else { CARD_RADIUS };
        match self.adding_pane {
            Some(with) => {
                let count = self.panes(with).len();
                let last = pane_frames(frame, count + 1, self.stacked(with), &[])[count - 1];
                (Bounds::from_corners(frame.origin, last.bottom_right()), radius)
            }
            None => (frame, radius),
        }
    }

    /// Lays the page out at `page_frame`, unless it's there already.
    fn lay_out_page(&mut self, cx: &mut Context<Self>) {
        let frame = self.page_frame(self.viewport);
        if self.page_laid != Some(frame) {
            self.page_laid = Some(frame);
            self.browser.lay_out_page(frame.0, frame.1, cx);
        }
    }

    pub fn selected(&self) -> u64 {
        self.selected
    }

    /// Where the card's left edge is: past the sidebar, or at the inset once ⌘S has hidden it,
    /// and between the two while it moves. 0 while a page is full screen, which takes the window.
    pub fn card_left(&self) -> f32 {
        if self.page_fullscreen {
            return 0.;
        }
        let to = if self.sidebar_hidden { CARD_INSET } else { self.width() };
        let Some((from, start)) = self.sidebar_motion else {
            return to;
        };
        match self.sidebar_progress(start) {
            Some(progress) => from + (to - from) * progress,
            None => to,
        }
    }

    /// The Spaces, by id, name, colour and profile, in the switcher's order.
    pub fn set_spaces(
        &mut self,
        spaces: Vec<(u64, SharedString, Option<u32>, u64)>,
        cx: &mut Context<Self>,
    ) {
        // check.sh reads these lines.
        let ids: Vec<String> = spaces.iter().map(|(id, ..)| id.to_string()).collect();
        log!("browser: spaces {}", ids.join(" "));
        let mut old = std::mem::take(&mut self.spaces);
        self.spaces = spaces
            .into_iter()
            .map(|(id, name, color, profile)| {
                let Some(space) = old.iter_mut().find(|space| space.id == id) else {
                    log!("browser: added space {id} {name}");
                    if let Some(color) = color {
                        log!("browser: colored space {id} {color:06x}");
                    }
                    log!("browser: profiled space {id} {profile}");
                    return Space {
                        id,
                        name,
                        color,
                        profile,
                        pinned: Vec::new(),
                        today: Vec::new(),
                        splits: Vec::new(),
                        folders: Folders::default(),
                        shown: 0,
                    };
                };
                if space.name != name {
                    log!("browser: renamed space {id} {name}");
                }
                if space.color != color {
                    let color = color.map_or("none".to_string(), |color| format!("{color:06x}"));
                    log!("browser: colored space {id} {color}");
                }
                if space.profile != profile {
                    log!("browser: profiled space {id} {profile}");
                }
                Space {
                    id,
                    name,
                    color,
                    profile,
                    pinned: std::mem::take(&mut space.pinned),
                    today: std::mem::take(&mut space.today),
                    splits: std::mem::take(&mut space.splits),
                    folders: std::mem::take(&mut space.folders),
                    shown: space.shown,
                }
            })
            .collect();
        if let Some(form) = &self.form
            && !self.spaces.iter().any(|space| space.id == form.space)
        {
            self.form = None;
        }
        if let Some(Naming::Space(space)) = self.renaming.as_ref().map(|renaming| renaming.naming)
            && !self.spaces.iter().any(|each| each.id == space)
        {
            self.renaming = None;
        }
        if self.show_favorites() {
            self.log_favorites();
        }
        self.update_menus(cx);
        cx.notify();
    }

    /// The profiles, by id and name, the one the browser started with first.
    pub fn set_profiles(&mut self, profiles: Vec<(u64, SharedString)>, cx: &mut Context<Self>) {
        // check.sh waits for these lines.
        for (id, name) in &profiles {
            match self.profiles.iter().find(|(each, _)| each == id) {
                None => log!("browser: added profile {id} {name}"),
                Some((_, was)) if was != name => log!("browser: renamed profile {id} {name}"),
                Some(_) => {}
            }
        }
        for (id, name) in &self.profiles {
            if !profiles.iter().any(|(each, _)| each == id) {
                log!("browser: removed profile {id} {name}");
            }
        }
        self.profiles = profiles;
        cx.notify();
    }

    /// The Space the window shows. Which of its tabs shows follows in set_selected.
    pub fn set_space(&mut self, space: u64, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        match self.switching {
            Some((asked, start)) if asked == space => {
                self.switching = None;
                log!(
                    "browser: space {space} in {:.1} ms",
                    start.elapsed().as_secs_f64() * 1000.
                );
            }
            _ => log!("browser: space {space}"),
        }
        self.browser.tooltip(None, cx);
        let before = std::mem::replace(&mut self.space, space);
        // A swipe that began in another Space is over, and its momentum with it.
        if self.swipe.as_ref().is_some_and(|swipe| swipe.from != space) {
            self.swipe = None;
            self.swiped = true;
        }
        // A Space shown some other way slides in from its side, pushing the one before out. One a
        // swipe settles on is moving already, and one a pull made shows where it is.
        let pulled = self.pulled_space_shown(space);
        let index = |id| self.spaces.iter().position(|other| other.id == id);
        let width = self.width();
        if !self.settle.as_ref().is_some_and(|settle| settle.space == space)
            && !pulled
            && let (Some(from), Some(to)) = (index(before), index(space))
            && from != to
        {
            self.settle = Some(Settle {
                space,
                beside: Some(before),
                from: if to > from { width } else { -width },
                velocity: 0.,
                start: Instant::now(),
            });
        }
        self.slides.clear();
        self.opened = None;
        self.closing.clear();
        self.clearing.clear();
        self.drag = None;
        if self.show_favorites() {
            self.log_favorites();
        }
        self.log_tabs();
        if self.form().is_some() {
            self.focus_form(cx);
        }
        self.update_menus(cx);
        cx.notify();
    }

    pub fn set_tabs(
        &mut self,
        space: u64,
        pinned: Vec<u64>,
        today: Vec<u64>,
        splits: Vec<Split>,
        folders: Folders,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.spaces.iter_mut().find(|entry| entry.id == space) else {
            return;
        };
        // The rows of the Space shown move from where they were, unless it's being filled.
        let had_tabs = !entry.pinned.is_empty() || !entry.today.is_empty();
        let before = (space == self.space && had_tabs).then(|| entry.slots());
        entry.pinned = pinned;
        entry.today = today;
        entry.splits = splits;
        for (folder, (name, _)) in &folders.folders {
            if entry.folders.folders.get(folder).is_some_and(|(was, _)| was != name) {
                // check.sh waits for this line.
                log!("browser: renamed folder {folder} {name}");
            }
        }
        entry.folders = folders;
        let has_tabs = !entry.pinned.is_empty() || !entry.today.is_empty();
        // After a Clear, the rows left are in their places at once (Arc).
        let cleared = space == self.space && {
            let clearing = self.clearing.len();
            self.clearing.retain(|tab| entry.today.contains(tab));
            self.clearing.len() < clearing
        };
        if space == self.space {
            self.log_tabs();
        }
        if cleared {
            self.slides.clear();
        } else if let Some(mut before) = before {
            // A favorite dragged into the list comes from where the pointer has it.
            if let Some(drag) = self.drag.as_ref().filter(|drag| drag.moving) {
                before.entry(Slot::Tab(drag.tab)).or_insert(drag.top);
            }
            // A folder the user opened or closed moves the rows under it on its own spring.
            let curve = if std::mem::take(&mut self.folding) { Curve::Fold } else { Curve::Slide };
            self.move_rows(before, curve);
        }
        if space == self.space {
            let tiles = self.drawn();
            if self.end_drag(false) {
                self.move_tiles(tiles, Curve::Slide);
            }
        }
        // A tab opened in the form's Space makes it a Space like the others, under the name it has.
        if has_tabs && self.form.as_ref().is_some_and(|form| form.space == space) {
            self.form = None;
        }
        self.forget_closed();
        cx.notify();
    }

    /// Every profile's favorites, left to right, each with its profile.
    pub fn set_favorites(&mut self, favorites: Vec<(u64, u64)>, cx: &mut Context<Self>) {
        let tiles = self.drawn();
        self.all_favorites = favorites.iter().map(|&(tab, _)| tab).collect();
        self.favorite_profiles = favorites.into_iter().collect();
        self.show_favorites();
        self.log_favorites();
        self.end_drag(true);
        // Nothing moves while the Space shown is being filled, as at launch.
        let (pinned, today) = self.rows();
        if pinned.is_empty() && today.is_empty() {
            self.tile_slides.clear();
            self.list_slide = None;
        } else {
            self.move_tiles(tiles, Curve::Slide);
        }
        self.forget_closed();
        cx.notify();
    }

    /// Shows the favorites of the shown Space's profile, or all of them while no Space shows, and
    /// says whether that changed them. Another profile's swap in at once, as the Space they come
    /// with slides in.
    fn show_favorites(&mut self) -> bool {
        let profile = self.shown_space().map(|space| space.profile);
        let profiles = &self.favorite_profiles;
        let shown: Vec<u64> = (self.all_favorites.iter().copied())
            .filter(|tab| profile.is_none_or(|profile| profiles.get(tab) == Some(&profile)))
            .collect();
        let changed = shown != self.favorites;
        self.favorites = shown;
        changed
    }

    fn log_favorites(&self) {
        // check.sh reads these lines.
        let ids: Vec<String> = self.favorites.iter().map(u64::to_string).collect();
        log!("browser: favorites {}", ids.join(" "));
    }

    /// What the shell knew of a closed tab goes with it. The core reports a tab in its new place
    /// before it leaves the old, so one moving between a Space and the favorites stays.
    fn forget_closed(&mut self) {
        let (spaces, favorites) = (&self.spaces, &self.all_favorites);
        let open = |tab: &u64| {
            favorites.contains(tab)
                || spaces
                    .iter()
                    .any(|space| space.pinned.contains(tab) || space.today.contains(tab))
        };
        self.tabs.retain(|tab, _| open(tab));
        self.recent.retain(open);
        if let Some(Naming::Tab(tab)) = self.renaming.as_ref().map(|renaming| renaming.naming)
            && !open(&tab)
        {
            self.renaming = None;
        }
    }

    fn log_tabs(&mut self) {
        // check.sh reads these lines, the pinned one first.
        let ids = |tabs: &[u64]| tabs.iter().map(u64::to_string).collect::<Vec<_>>().join(" ");
        log!("browser: pinned {}", ids(self.pinned()));
        log!("browser: today {}", ids(self.today()));
        let splits = self.shown_space().map_or_else(Vec::new, |space| {
            // A stacked split's tabs are joined with a slash.
            let join = |split: &Split| if split.stacked { "/" } else { "+" };
            space.splits.iter().map(|split| ids(&split.tabs).replace(' ', join(split))).collect()
        });
        log!("browser: splits {}", splits.join(" "));
        self.outline_changed = true;
    }

    /// The sidebar's tabs' rows in its order, the favorites then the Space's pinned and Today rows,
    /// each by its first tab, leaving out folders and what closed ones hide, the tab shown's too,
    /// so each keeps its number.
    fn sidebar_rows(&self) -> Vec<u64> {
        let (pinned, today) = self.rows();
        let space = self.shown_space();
        let shut = |row: u64| space.is_some_and(|space| space.shut(row));
        let pinned = pinned.into_iter().filter(|&row| !self.is_folder(row) && !shut(row)).collect();
        [self.favorites.clone(), pinned, today].concat()
    }

    pub fn set_selected(&mut self, tab: u64, cx: &mut Context<Self>) {
        match self.switching.take() {
            Some((asked, start)) if asked == tab => log!(
                "browser: selected {tab} in {:.1} ms",
                start.elapsed().as_secs_f64() * 1000.
            ),
            _ => log!("browser: selected {tab}"),
        }
        self.selected = tab;
        self.show_in_space(tab);
        self.scroll_to_row(tab);
        if self.walked.is_none() {
            self.shown(tab);
        }
        // check.sh reads this line.
        let panes = self.panes(tab).iter().map(u64::to_string).collect::<Vec<_>>().join(" ");
        log!("browser: panes {panes}");
        self.set_new_tab(false, cx);
        self.adding_pane = None;
        cx.notify();
    }

    /// The user left the command bar without choosing anything in it.
    pub fn command_bar_dismissed(&mut self, cx: &mut Context<Self>) {
        self.set_new_tab(false, cx);
        if self.adding_pane.take().is_some() {
            self.lay_out_page(cx);
            cx.notify();
        }
    }

    /// Gives "New Tab" the selected plate while ⌘T's command bar is open, or gives it back.
    fn set_new_tab(&mut self, new_tab: bool, cx: &mut Context<Self>) {
        if self.new_tab != new_tab {
            self.new_tab = new_tab;
            // check.sh reads this line.
            log!("browser: new tab plate {}", new_tab as u8);
            cx.notify();
        }
    }

    /// Makes `tab` the Space's shown tab, whose row a closed folder keeps, and moves the rows if
    /// that changes them.
    fn show_in_space(&mut self, tab: u64) {
        let space = self.spaces.iter_mut().find(|space| space.id == self.space);
        let Some(space) = space.filter(|space| space.shown != tab) else {
            return;
        };
        // Only a closed folder changes the rows, and a tab switch has nothing more to do.
        if !space.folders.folders.values().any(|&(_, open)| !open) {
            space.shown = tab;
            return;
        }
        let before = space.slots();
        space.shown = tab;
        if space.slots() != before {
            self.move_rows(before, Curve::Slide);
        }
    }

    /// The row with the selected plate: the tab shown's, or its split's.
    fn selected_row(&self) -> Row {
        if self.new_tab { Row::NewTab } else { Row::Tab(self.row_of(self.selected)) }
    }

    fn shown(&mut self, tab: u64) {
        if tab != 0 {
            self.recent.retain(|&other| other != tab);
            self.recent.insert(0, tab);
        }
    }

    /// ⌃ went up, which ends a ⌃Tab walk on the tab it shows. A click ends it too, and the core
    /// may report the tab the click showed before this runs, so the walk's tab counts first.
    pub fn control_released(&mut self, _: &mut Context<Self>) {
        if let Some(walked) = self.walked.take().filter(|tab| self.tabs.contains_key(tab)) {
            self.shown(walked);
        }
        self.shown(self.selected);
    }

    pub fn set_title(&mut self, tab: u64, title: SharedString, cx: &mut Context<Self>) {
        self.tabs.entry(tab).or_default().title = title;
        cx.notify();
    }

    pub fn set_favicon(&mut self, tab: u64, favicon: Option<Arc<Image>>, cx: &mut Context<Self>) {
        self.tabs.entry(tab).or_default().favicon = favicon;
        cx.notify();
    }

    pub fn set_page(&mut self, tab: u64, page: Page, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        let flag = |on| if on { 1 } else { 0 };
        log!(
            "browser: page {tab} back {} forward {} loading {} {}",
            flag(page.can_go_back),
            flag(page.can_go_forward),
            flag(page.loading),
            page.url
        );
        let entry = self.tabs.entry(tab).or_default();
        if page.unloaded != entry.page.unloaded {
            log!("browser: page {tab} unloaded {}", flag(page.unloaded));
        }
        if page.away_from_pinned_url != entry.page.away_from_pinned_url {
            log!("browser: page {tab} away {}", flag(page.away_from_pinned_url));
        }
        if (page.audible, page.muted) != (entry.page.audible, entry.page.muted) {
            let (audible, muted) = (flag(page.audible), flag(page.muted));
            log!("browser: page {tab} audible {audible} muted {muted}");
        }
        if page.icon != entry.page.icon {
            let icon = if page.icon.is_empty() { "none" } else { &page.icon };
            log!("browser: page {tab} icon {icon}");
        }
        entry.page = page;
        cx.notify();
    }

    pub fn set_page_fullscreen(&mut self, fullscreen: bool, cx: &mut Context<Self>) {
        self.page_fullscreen = fullscreen;
        if fullscreen {
            self.conceal_now(cx);
            self.close_library_now();
        }
        cx.notify();
    }

    /// Deletes the Space, asking first if it has tabs, which close with it.
    fn delete_space(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(found) = self.spaces.iter().find(|each| each.id == space) else {
            return;
        };
        let tabs = found.pinned.len() + found.today.len();
        if tabs == 0 {
            self.delete_space_now(space, cx);
            return;
        }
        let shell = cx.weak_entity();
        self.browser.confirm(
            format!("Delete “{}”?", found.name),
            if tabs == 1 {
                "Its tab closes too.".into()
            } else {
                format!("Its {tabs} tabs close too.")
            },
            "Delete",
            Box::new(move |cx| {
                shell.update(cx, |shell, cx| shell.delete_space_now(space, cx)).ok();
            }),
            cx,
        );
    }

    /// Deletes the Space, for ⌘Z to bring back, and says so in the toast (Arc).
    fn delete_space_now(&mut self, space: u64, cx: &mut Context<Self>) {
        if self.browser.delete_space(space, cx) {
            self.undo = Some(Undo::DeleteSpace);
            self.browser.toast("Space deleted! Use", Some(("⌘Z", "to undo.")), DELETED_TIME, cx);
        }
    }

    fn shown_space(&self) -> Option<&Space> {
        self.spaces.iter().find(|space| space.id == self.space)
    }

    fn show_space_at(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if let Some(space) = index.and_then(|index| self.spaces.get(index)) {
            self.show_space(space.id, cx);
        }
    }

    fn show_space(&mut self, space: u64, cx: &mut Context<Self>) {
        if space == self.space {
            return;
        }
        self.switching = Some((space, Instant::now()));
        self.browser.show_space(space, cx);
    }

    fn select(&mut self, tab: u64, cx: &mut Context<Self>) {
        if tab == self.selected {
            return;
        }
        self.switching = Some((tab, Instant::now()));
        self.browser.select(tab, cx);
    }

    /// A double-click on the window's background: in the sidebar below the rows it opens a new tab
    /// (Dia), and elsewhere it does what System Settings says a title bar's does.
    fn double_click_background(
        &mut self,
        at: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let below_rows = at.x < px(self.card_left()) && at.y >= px(self.under_tiles());
        let sidebar = !self.page_fullscreen && self.library.is_none() && self.form().is_none();
        if sidebar && below_rows {
            self.run(Command::NewTab, "sidebar", cx);
        } else {
            window.titlebar_double_click();
        }
    }

    pub fn page(&self, tab: u64) -> Page {
        self.tabs.get(&tab).map(|tab| tab.page.clone()).unwrap_or_default()
    }

    fn url(&self, tab: u64) -> SharedString {
        self.page(tab).url
    }

    /// A tab's title: the page's, else its host, else "Untitled" (Arc).
    pub fn title(&self, tab: u64) -> SharedString {
        match self.tabs.get(&tab) {
            Some(tab) if !tab.title.is_empty() => tab.title.clone(),
            Some(tab) if !tab.page.url.is_empty() => match file_name(&tab.page.url) {
                Some(name) => name.into(),
                None => host(&tab.page.url).into(),
            },
            _ => "Untitled".into(),
        }
    }

    /// The pointer entered `row` or left it. A row fills at once, and fades when it's left.
    fn hover(&mut self, row: Row, hovered: bool, cx: &mut Context<Self>) {
        let left = if hovered {
            self.unhovered.retain(|&(old, _)| old != row);
            self.hovered.replace(row).filter(|&old| old != row)
        } else if self.hovered == Some(row) {
            self.hovered.take()
        } else {
            None
        };
        if let Some(left) = left {
            self.unhovered.retain(|&(old, _)| old != left);
            self.unhovered.push((left, Instant::now()));
        }
        cx.notify();
    }

    /// The pointer's moves anywhere in the window, for whether it's over the sidebar, whose rows
    /// and buttons keep it from the sidebar's own hover.
    fn hover_listener(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let shell = cx.weak_entity();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let moved = shell.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture {
                        let over = bounds.contains(&event.position);
                        moved.update(cx, |shell, cx| shell.sidebar_hover(over, cx)).ok();
                    }
                });
                window.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture {
                        shell.update(cx, |shell, cx| shell.sidebar_hover(false, cx)).ok();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }

    /// Drops the list's and the pages' motion that's done, and returns whether any is left.
    fn list_moving(&mut self) -> bool {
        let sliding = !self.slides.is_empty();
        self.slides
            .retain(|_, (_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_some());
        if sliding && self.slides.is_empty() {
            // check.sh waits for this line before it presses a row that moved, or scrolls the
            // list: a row sliding in from above leaves the list short until it settles.
            log!("browser: rows settled");
        }
        let tiling = !self.tile_slides.is_empty();
        self.tile_slides
            .retain(|_, (_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_some());
        if tiling && self.tile_slides.is_empty() {
            // check.sh waits for this line before it presses a tile that moved.
            log!("browser: tiles settled");
        }
        self.list_slide
            .take_if(|(_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_none());
        self.closing.retain(|row| row_spring(row.start.elapsed().as_secs_f32()).is_some());
        self.unhovered.retain(|(_, start)| start.elapsed().as_secs_f32() < ROW_FILL);
        if self.slides.is_empty() {
            self.opened.take_if(|opened| opened.done().is_none());
        }
        if let Some(settle) = self.settle.take_if(|settle| settle.offset().is_none()) {
            // check.sh reads these lines.
            log!("browser: settled space {}", settle.space);
        }
        !(self.slides.is_empty() && self.closing.is_empty() && self.unhovered.is_empty())
            || !self.tile_slides.is_empty()
            || self.list_slide.is_some()
            || self.opened.is_some()
            || self.settle.is_some()
    }

    /// Draws a dragged tab's turn between its row and its tile on, and once it has turned, lands a
    /// drop the pointer let go of mid-turn.
    fn step_drag_shape(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut().filter(|drag| drag.shape.start.is_some()) else {
            return;
        };
        if drag.shape.turning(Instant::now()) {
            window.request_animation_frame();
            return;
        }
        drag.shape.start = None;
        // check.sh waits for these lines before it captures a dragged tab.
        log!("browser: drag shape {}", if drag.shape.tile { "tile" } else { "row" });
        if std::mem::take(&mut drag.held) {
            // Out of render, as the browser may report the lists it made at once.
            cx.defer_in(window, |shell, _, cx| shell.drop_row(cx));
        }
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // check.sh reads this line. It comes as the rows are drawn, so a click it sends next goes
        // to what's drawn.
        if std::mem::take(&mut self.outline_changed) {
            let outline = self.shown_space().map(|space| space.outline(0)).unwrap_or_default();
            log!("browser: outline {outline}");
        }
        // As the outline's, so a click on Clear, which shows only under the pointer, finds it.
        if std::mem::take(&mut self.hover_changed) {
            log!("browser: sidebar hovered {}", self.sidebar_hovered as u8);
        }
        // As the outline's: the ✕ and the way back show only under the pointer, so a click that
        // follows a move waits for this line.
        let close = matches!(self.hovered, Some(Row::Tab(tab)) if self.closable(tab));
        let hover = (self.hovered, self.hovered_favicon, close);
        if hover != self.drawn_hover {
            self.drawn_hover = hover;
            let row = match hover.0 {
                Some(Row::Tab(tab)) => format!("tab {tab}"),
                Some(Row::NewTab) => "new".into(),
                None => "none".into(),
            };
            let favicon = hover.1.map_or("none".into(), |tab| tab.to_string());
            log!("browser: hover {row} close {} favicon {favicon}", close as u8);
        }
        // As the outline's, so a capture of the URL field's button shows the glyph it logged.
        if self.url_copied.is_some() != self.drawn_copied {
            self.drawn_copied = self.url_copied.is_some();
            log!("browser: copy url shows {}", if self.drawn_copied { "check" } else { "link" });
        }
        if std::mem::take(&mut self.focus_form)
            && let Some(form) = self.form()
        {
            window.focus(&form.name.focus_handle(cx), cx);
        }
        // A new folder's field is drawn once the browser reports the folder, and blurs if focused
        // before.
        let drawn = self.renaming.as_ref().is_some_and(|renaming| match renaming.naming {
            Naming::Folder(folder) => self.is_folder(folder),
            _ => true,
        });
        if drawn
            && std::mem::take(&mut self.focus_renaming)
            && let Some(renaming) = &mut self.renaming
        {
            let field = renaming.name.focus_handle(cx);
            window.focus(&field, cx);
            // check.sh waits for this line, which says the field has the keyboard.
            if let Naming::Folder(folder) = renaming.naming {
                log!("browser: rename folder {folder}");
            }
            if let Naming::Icon(tab) = renaming.naming {
                log!("browser: change icon {tab}");
                // A pick from the palette, or a key, is the icon at once, as no ↩ is needed for
                // one character.
                let start = renaming.name.read(cx).text().to_string();
                renaming._edit = Some(cx.observe_in(
                    &renaming.name,
                    window,
                    move |shell, name, window, cx| {
                        let name = name.read(cx);
                        let text = name.text();
                        if !name.composing() && !text.trim().is_empty() && text != start {
                            shell.keep_name(window, cx);
                        }
                    },
                ));
                window.show_character_palette();
            }
            renaming._blur =
                Some(cx.on_blur(&field, window, |shell, window, cx| shell.keep_name(window, cx)));
        }
        if let Some((_, start)) = self.width_motion {
            if settle_progress(start).is_some() {
                window.request_animation_frame();
            } else {
                self.width_motion = None;
                // check.sh waits for these lines.
                log!("browser: sidebar width {}", self.sidebar_width);
            }
        }
        if let Some((_, start)) = self.sidebar_motion {
            if self.sidebar_progress(start).is_some() {
                window.request_animation_frame();
            } else {
                self.sidebar_motion = None;
                // check.sh waits for these lines.
                let state = if self.sidebar_hidden { "hidden" } else { "shown" };
                log!("browser: sidebar {state}");
            }
        }
        self.step_library(window, cx);
        self.step_new_space(window);
        self.step_flight(window);
        if let Some((_, left)) = &self.leaving_form {
            if left.elapsed().as_secs_f32() < FORM_LEAVE_TITLE + FORM_LEAVE {
                window.request_animation_frame();
            } else {
                self.leaving_form = None;
                // check.sh waits for this line.
                log!("browser: space form gone");
            }
        }
        if self.list_moving() {
            window.request_animation_frame();
        }
        self.step_drag_shape(window, cx);
        self.step_scroll_edge(window);
        self.step_tools(window);
        let drop_targets = self.drag.as_ref().is_some_and(|drag| drag.over_page.is_some());
        if drop_targets || self.drop_targets_shown {
            self.drop_targets_shown = drop_targets;
            self.browser.show_drop_targets(drop_targets, cx);
        }
        // The page is laid out at every step, as in Arc.
        let viewport = window.viewport_size();
        if viewport != self.viewport {
            // check.sh waits for this line before it scrolls the list in a resized window.
            let (width, height) = (f32::from(viewport.width), f32::from(viewport.height));
            log!("browser: drawn at {width}x{height}");
        }
        self.viewport = viewport;
        self.scale = window.scale_factor();
        if self.tiles_scroll() != self.tiles_scrolled_last {
            self.tiles_scrolled_last = !self.tiles_scrolled_last;
            // check.sh waits for these lines.
            match self.tiles_scrolled_last {
                true => log!("browser: tiles scroll with the list"),
                false => log!("browser: tiles stay put"),
            }
        }
        self.lay_out_page(cx);
        let card_left = self.card_left();
        let focus_color = self.shown_space().and_then(|space| space.color);
        let focus_color = focus_color.unwrap_or(self.accent_color);
        if self.focus_color != Some(focus_color) {
            self.focus_color = Some(focus_color);
            self.browser.set_focus_color(focus_color, cx);
        }
        // A page shows the card until its first frame, never white (Ardia). Not while the sidebar
        // pages between Spaces, whose colours cross-fade at every step.
        let page_color = packed(self.window_fill().blend(rgba(CARD).into()));
        if self.pages().len() == 1 && self.page_color != Some(page_color) {
            self.page_color = Some(page_color);
            self.browser.set_page_color(page_color, cx);
        }
        // The find bar and the zoom HUD sit at the focused page's corner.
        let (frame, _) = self.page_frame(self.viewport);
        let panes = self.panes(self.selected);
        let focused = panes.iter().position(|&pane| pane == self.selected).unwrap_or(0);
        let (stacked, shares) = (self.stacked(self.selected), self.shares(self.selected));
        let pane = pane_frames(frame, panes.len(), stacked, &shares)[focused];
        let pane_corner =
            (f32::from(frame.right() - pane.right()), f32::from(pane.top() - frame.top()));
        if self.pane_corner != pane_corner {
            self.pane_corner = pane_corner;
            self.browser.set_pane_corner(pane_corner.0, pane_corner.1, cx);
        }
        let window_buttons = self.window_buttons_shown();
        if window_buttons != self.window_buttons {
            self.window_buttons = window_buttons;
            self.browser.show_window_buttons(window_buttons, cx);
        }
        // The Library slides in from the left with the card's edge, over the sidebar receding.
        let library = self.library.is_some() && !self.page_fullscreen;
        let library = library.then(|| self.library_panel(cx));
        // The sidebar leaves with the card's edge, all of it by the time the edge stops.
        let width = self.width();
        let sidebar_left = (card_left - width) * width / (width - CARD_INSET);
        let opacity = self.sidebar_opacity();
        let sidebar_shown = !self.page_fullscreen
            && opacity > 0.
            && (!self.sidebar_hidden || self.sidebar_motion.is_some());
        let sidebar = sidebar_shown.then(|| {
            let scale = self.sidebar_scale();
            let origin = point(px(width + RECEDE_X), self.viewport.height / 2.);
            let cover = self.sidebar_cut().unwrap_or(0.);
            let sidebar = self.sidebar(cx).left(px(sidebar_left - cover));
            let uncovered = div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(cover))
                .right_0()
                .overflow_hidden()
                .child(sidebar.child(self.hover_listener(cx)));
            // Its buttons recede with it, over the Library, as the window's do in Arc. Full screen
            // has none.
            let buttons = self.library_moving() && !window.is_fullscreen();
            let buttons = buttons.then(|| self.window_buttons(cx));
            let receding = div().absolute().size_full().opacity(opacity);
            scaled(receding.child(uncovered).children(buttons), scale, origin)
        });
        let docked = sidebar_shown
            && !self.sidebar_hidden
            && self.sidebar_motion.is_none()
            && self.library.is_none();
        let edge = docked.then(|| self.sidebar_edge(cx));
        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .font_family(".SystemUIFont")
            .bg(rgba(TINT))
            // The rows, buttons and fields keep their presses. Everywhere else is the window's
            // background, which moves the window as a title bar does.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, _| shell.background_pressed = true),
            )
            .on_mouse_move(cx.listener(|shell, event: &MouseMoveEvent, window, cx| {
                if shell.background_pressed && event.dragging() {
                    shell.background_pressed = false;
                    // check.sh reads this line.
                    log!("browser: window move");
                    window.start_window_move();
                }
                shell.pointer_at(event.position.x.into(), cx);
            }))
            .on_scroll(cx.listener(|shell, _, _, cx| shell.browser.tooltip(None, cx)))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseUpEvent, window, cx| {
                    if std::mem::take(&mut shell.background_pressed) && event.click_count == 2 {
                        shell.double_click_background(event.position, window, cx);
                    }
                }),
            )
            .child(div().absolute().size_full().bg(self.gradient(None)))
            // The page views go over the cards, as native views.
            .children(self.cards())
            .children(self.pane_gaps(cx))
            .children(library)
            .children(sidebar)
            .children(edge)
            .child(self.scroll_listener(true, cx))
    }
}

impl Shell {
    /// The cards under the pages shown: a page's own in a split view, so the gaps between them
    /// show the window, with the focused page's glowing in its ring's colour. While a pane is
    /// being added, its card is there, empty.
    fn cards(&self) -> Vec<gpui::Div> {
        let (frame, _) = self.page_frame(self.viewport);
        let panes = self.panes(self.selected);
        let glow = BoxShadow {
            color: Hsla::from(rgb(self.focus_color.unwrap_or(self.accent_color)))
                .opacity(FOCUS_GLOW_ALPHA),
            offset: point(px(0.), px(0.)),
            blur_radius: px(FOCUS_GLOW),
            spread_radius: px(0.),
            inset: false,
        };
        let split = panes.len() > 1;
        let (stacked, shares) = (self.stacked(self.selected), self.shares(self.selected));
        let mut cards: Vec<_> = pane_frames(frame, panes.len(), stacked, &shares)
            .into_iter()
            .zip(&panes)
            .map(|(bounds, &tab)| {
                let focused = split && tab == self.selected;
                card(bounds).when(focused, |card| card.shadow(vec![glow.clone()]))
            })
            .collect();
        if let Some(with) = self.adding_pane {
            let count = self.panes(with).len() + 1;
            let full = page_frame(self.viewport, self.card_left(), self.card_push());
            cards.push(card(pane_frames(full, count, self.stacked(with), &[])[count - 1]));
        }
        cards
    }

    /// The sidebar: the header's buttons, the URL field and the Space's tabs, or the Create a
    /// Space form, over the switcher. Docked, or in the reveal's panel.
    fn sidebar(&self, cx: &mut Context<Self>) -> gpui::Div {
        let sidebar = div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(px(self.width()))
            .children(self.nav_buttons(cx));
        let leaving = self.leaving_form.as_ref().filter(|(form, _)| form.space == self.space);
        let sidebar = match (self.form(), leaving) {
            (Some(form), _) => sidebar.child(self.space_form(form, None, cx)),
            (None, Some((form, left))) => {
                let left = left.elapsed().as_secs_f32();
                sidebar.child(self.space_form(form, Some(left), cx))
            }
            (None, None) => sidebar
                .child(self.url_field(cx))
                .children(self.scroll_edge_line())
                .child(self.space_pages(cx))
                // Over the pages, which they scroll with when they're tall.
                .children(self.tiles(cx))
                .children(self.dragged_tile(cx))
                .children(self.turning_drag())
                .children(self.question_card(cx)),
        };
        sidebar
            .children(self.new_space_circle())
            .child(self.library_button(cx))
            .child(self.plus_button(cx))
            .children(self.switcher(cx))
            .children(self.flying_icon())
            .child(self.press_listener())
    }

    /// The Spaces' pages under the URL field, side by side while they move. Each keeps its own
    /// scroll.
    fn space_pages(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let width = self.width();
        let mut pages = Vec::new();
        for (space, x) in self.pages() {
            let page = match self.spaces.iter().find(|other| other.id == space) {
                _ if space == self.space => self.list(cx).into_any_element(),
                Some(other) => self.still_page(other).into_any_element(),
                None => continue,
            };
            pages.push(
                div()
                    .id(("page", space))
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(x))
                    .w(px(width))
                    .child(page),
            );
        }
        div()
            .absolute()
            .top(px(self.pages_top()))
            .left_0()
            .w(px(self.width()))
            .bottom(px(FOOTER_HEIGHT))
            .overflow_hidden()
            .children(pages)
    }

    /// Another Space's page as it shows once shown, while it slides in or out: its name, and its
    /// rows with the tab it would show selected. It takes no input.
    fn still_page(&self, space: &Space) -> impl IntoElement + use<> {
        let selected = self.browser.space_tab(space.id);
        let row = |tab: u64| {
            let row = row_box().pl(px(PLATE_INSET + space.depth(tab) as f32 * FOLDER_INDENT));
            if let Some((name, open)) = space.folders.folders.get(&tab) {
                let name = title(name.clone(), false).pr(px(TITLE_END_INSET));
                return row.child(plate(folder_icon(*open), name));
            }
            let panes = space.panes(tab);
            if panes.len() > 1 {
                let width = self.segment_width(space.depth(tab), panes.len());
                let segments = panes
                    .iter()
                    .map(|&pane| self.segment(pane, width, pane == tab, pane == selected));
                return row.child(
                    split_plate(segments)
                        .when(panes.contains(&selected), |plate| selected_plate(plate, 1.)),
                );
            }
            let icon = div()
                .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
                .child(self.tab_icon(tab));
            row.child(
                plate(icon, title(self.title(tab), tab == selected).pr(px(TITLE_END_INSET)))
                    .when(tab == selected, |plate| selected_plate(plate, 1.)),
            )
        };
        let (plus, new_tab) = new_tab_label();
        let (pinned, today) = space.rows();
        let room = self.tiles_room();
        div()
            .size_full()
            .flex()
            .flex_col()
            .when(room > 0., |page| page.child(div().flex_none().h(px(room))))
            .child(name_box().child(div().truncate().child(space.name.clone())))
            .children(pinned.into_iter().map(row))
            .when(!today.is_empty(), |page| {
                let clear =
                    self.tools_shown().then(|| clear_label(false).opacity(self.tools_opacity()));
                page.child(divider_box(self.hairline(), clear))
            })
            .child(row_box().child(plate(plus, new_tab)))
            .children(today.into_iter().map(row))
    }

    /// The Space's name, its pinned tabs, the divider, New Tab and its Today tabs, under the URL
    /// field, after room for the tiles when they scroll with it. Closed tabs' rows fade under the
    /// rest, and while rows slide down to make room for a tab's new row, they come out from
    /// beneath it.
    fn list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let room = self.tiles_room();
        let mut rows = vec![(None, self.name_row(cx).into_any_element())];
        let (pinned, today) = self.rows();
        for tab in pinned {
            rows.push((Some(tab), self.list_row(tab, cx)));
        }
        if !today.is_empty() {
            let divider = match self.drag.as_ref().is_some_and(|drag| drag.moving && !drag.divider) {
                true => div().flex_none().h(px(DIVIDER_HEIGHT)).into_any_element(),
                false => self.divider(cx).into_any_element(),
            };
            rows.push((None, divider));
        }
        rows.push((None, self.new_tab_row(cx).into_any_element()));
        for tab in today {
            rows.push((Some(tab), self.list_row(tab, cx)));
        }
        // Over the rest, where the pointer has it, unless it's over the favorites or the page. The
        // system draws links and files dragged in.
        let dragged = self.drag.as_ref().filter(|drag| {
            drag.moving && drag.over_page.is_none() && !drag.incoming
        });
        let dragged = dragged.filter(|drag| matches!(drag.to, Place::List(..)));
        // Mid-turn into or out of a tile, it's drawn over the sidebar instead.
        let dragged = dragged.filter(|drag| drag.shape.start.is_none()).map(|drag| {
            let row = self.tab_row(drag.tab, cx);
            let top = room + self.drag_at(drag);
            div().absolute().top(px(top)).left_0().w(px(self.width())).child(row)
        });
        let under = self
            .opened
            .as_ref()
            .and_then(|opened| {
                rows.iter().position(|&(tab, _)| tab.is_some_and(|tab| opened.tabs.contains(&tab)))
            })
            .map_or(rows.len(), |row| row + 1);
        let under = rows.split_off(under).into_iter().map(|(_, row)| row).collect::<Vec<_>>();
        div()
            .id("today")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            // Fingers across the list page between Spaces instead.
            .restrict_scroll_to_axis()
            .when(room > 0., |list| list.child(div().flex_none().h(px(room))))
            .children(self.closing.iter().map(|row| self.closing_row(row, room)))
            .children(rows.into_iter().map(|(_, row)| row))
            .when(!under.is_empty(), |list| {
                list.child(div().flex_none().flex().flex_col().overflow_hidden().children(under))
            })
            .children(dragged)
            .child(self.drag_listeners(cx))
    }

    /// A tab's row in the list, or the room it leaves while it's dragged.
    fn list_row(&self, tab: u64, cx: &mut Context<Self>) -> AnyElement {
        match self.dragging() == Some(Slot::Tab(tab)) {
            true => div().flex_none().h(px(ROW_PITCH)).into_any_element(),
            false => self.tab_row(tab, cx).into_any_element(),
        }
    }

    /// What the reveal's window shows: the sidebar in a panel, as far out as it is.
    pub fn render_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.reveal_motion.is_some() {
            if self.reveal_step().is_some() {
                window.request_animation_frame();
            } else {
                self.reveal_motion = None;
                // check.sh waits for these lines.
                if self.revealed {
                    log!("browser: reveal shown");
                    // Its window ends at its edge, so the page past it takes clicks again.
                    self.browser.show_reveal(REVEAL_INSET + self.width(), cx);
                } else {
                    log!("browser: reveal hidden");
                    self.browser.hide_reveal(cx);
                }
            }
        }
        if self.list_moving() {
            window.request_animation_frame();
        }
        self.step_drag_shape(window, cx);
        self.step_scroll_edge(window);
        self.step_tools(window);
        let left = REVEAL_INSET - (REVEAL_INSET + self.width()) * (1. - self.reveal_out());
        div()
            .size_full()
            .font_family(".SystemUIFont")
            // A double-click below the rows opens a new tab, as in the docked sidebar.
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseUpEvent, _, cx| {
                    let below_rows = event.position.y >= px(REVEAL_INSET + shell.under_tiles());
                    if event.click_count == 2 && below_rows && shell.form().is_none() {
                        shell.run(Command::NewTab, "sidebar", cx);
                    }
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(REVEAL_INSET))
                    .bottom(px(REVEAL_INSET))
                    .w(px(self.width()))
                    .rounded(px(REVEAL_RADIUS))
                    .border(px(0.5))
                    .border_color(rgba(REVEAL_BORDER))
                    .bg(self.gradient(Some(rgb(MATERIAL_STAND_IN).into())))
                    .child(self.window_buttons(cx))
                    .child(self.sidebar(cx).left_0())
                    .child(self.scroll_listener(false, cx)),
            )
            .into_any_element()
    }

    /// The window's close, minimize and zoom buttons, drawn in the reveal's panel.
    fn window_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let buttons = [
            (WindowButton::Close, "icons/window-close.svg"),
            (WindowButton::Minimize, "icons/window-minimize.svg"),
            (WindowButton::Zoom, "icons/window-full-screen.svg"),
        ];
        let (first, _) = WINDOW_BUTTONS[0];
        let (second, _) = WINDOW_BUTTONS[1];
        div()
            .absolute()
            .left(px(first - WINDOW_BUTTON_SIZE / 2.))
            .top(px(WINDOW_BUTTON_Y - WINDOW_BUTTON_SIZE / 2.))
            .flex()
            .gap(px(second - first - WINDOW_BUTTON_SIZE))
            .group("window-buttons")
            .children(buttons.into_iter().zip(WINDOW_BUTTONS).map(|((button, glyph), (_, colour))| {
                div()
                    .id(glyph)
                    .size(px(WINDOW_BUTTON_SIZE))
                    .rounded_full()
                    .bg(rgb(colour))
                    .block_mouse_except_scroll()
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        // check.sh waits for this line.
                        log!("browser: window button {button:?}");
                        shell.browser.press_window_button(button, cx);
                    }))
                    .child(
                        svg()
                            .path(glyph)
                            .size(px(WINDOW_BUTTON_SIZE))
                            .text_color(rgba(0))
                            .group_hover("window-buttons", |style| {
                                style.text_color(rgba(WINDOW_BUTTON_GLYPH))
                            }),
                    )
            }))
    }
}

/// `color`, opaque, as 0xRRGGBB.
fn packed(color: Hsla) -> u32 {
    let color = Rgba::from(color);
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u32;
    channel(color.r) << 16 | channel(color.g) << 8 | channel(color.b)
}

/// A card at `bounds` in the window.
fn card(bounds: Bounds<Pixels>) -> gpui::Div {
    div()
        .absolute()
        .left(bounds.left())
        .top(bounds.top())
        .w(bounds.size.width)
        .h(bounds.size.height)
        .rounded(px(CARD_RADIUS))
        .bg(rgba(CARD))
}

/// `way`, but past `low` or `high` only a little further, on the Space pages' rubber band.
fn rubber_band(way: f32, low: f32, high: f32) -> f32 {
    match way {
        way if way > high => high + band(way - high, 255., 0.15),
        way if way < low => low - band(low - way, 255., 0.15),
        way => way,
    }
}

/// How far something goes for `past` beyond its end: `stiffness` of the way at first, and never
/// as far as `limit`.
fn band(past: f32, limit: f32, stiffness: f32) -> f32 {
    limit * (1. - 1. / (stiffness * past / limit + 1.))
}

/// How far something settling critically damped since `start`, as the Space pages do, has come
/// from where it started, or None once it's there.
fn settle_progress(start: Instant) -> Option<f32> {
    let omega_t = std::f32::consts::TAU / SPACE_SETTLE * start.elapsed().as_secs_f32();
    let left = (1. + omega_t) * (-omega_t).exp();
    (left > 0.001).then_some(1. - left)
}

/// How far the list's motion is `t` seconds in, or None once it's done.
fn row_spring(t: f32) -> Option<f32> {
    spring(t, ROW_RESPONSE, ROW_DAMPING)
}

/// Where a spring going from `from` toward `to`, at `speed` a second as it starts, is `t` seconds
/// in, and how fast it's going, or None once it's there.
fn spring_to(
    t: f32,
    from: f32,
    to: f32,
    speed: f32,
    response: f32,
    damping: f32,
) -> Option<(f32, f32)> {
    let omega = std::f32::consts::TAU / response;
    let decay = (-damping * omega * t).exp();
    if decay < 0.001 {
        return None;
    }
    let damped = omega * (1. - damping * damping).sqrt();
    let (away, pull) = (from - to, damping * omega);
    let swing = (speed + pull * away) / damped;
    let (sin, cos) = (damped * t).sin_cos();
    let at = to + decay * (away * cos + swing * sin);
    let going =
        decay * ((swing * damped - pull * away) * cos - (away * damped + pull * swing) * sin);
    Some((at, going))
}

/// Where a spring from 0 to 1, at rest at 0, is `t` seconds in, by SwiftUI's response and damping
/// fraction. None once it's within a thousandth of 1 for good.
pub(crate) fn spring(t: f32, response: f32, damping: f32) -> Option<f32> {
    let omega = std::f32::consts::TAU / response;
    let decay = (-damping * omega * t).exp();
    if decay < 0.001 {
        return None;
    }
    let damped = omega * (1. - damping * damping).sqrt();
    Some(1. - decay * ((damped * t).cos() + damping * omega / damped * (damped * t).sin()))
}

/// Where the list's rows are from its top, by the heights they're drawn at in `list`.
fn slots(pinned: &[u64], today: &[u64]) -> HashMap<Slot, f32> {
    let rows = pinned
        .iter()
        .map(|&tab| (Slot::Tab(tab), ROW_PITCH))
        .chain((!today.is_empty()).then_some((Slot::Divider, DIVIDER_HEIGHT)))
        .chain([(Slot::NewTab, ROW_PITCH)])
        .chain(today.iter().map(|&tab| (Slot::Tab(tab), ROW_PITCH)));
    let mut top = NAME_HEIGHT;
    rows.map(|(slot, height)| {
        top += height;
        (slot, top - height)
    })
    .collect()
}

/// A row of the list, around its plate.
fn row_box() -> gpui::Div {
    div()
        .flex_none()
        .h(px(ROW_HEIGHT))
        .mb(px(ROW_PITCH - ROW_HEIGHT))
        .px(px(PLATE_INSET))
        .py(px((ROW_HEIGHT - PLATE_HEIGHT) / 2.))
}

/// The row with the Space's name, over its tabs.
fn name_box() -> gpui::Div {
    div()
        .flex_none()
        .h(px(NAME_HEIGHT))
        .flex()
        .items_center()
        .pl(px(NAME_X))
        .pr(px(FAVICON_X))
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(SPACE_NAME))
}

/// The divider under the pinned tabs: a line, and `clear` at its end while it shows.
fn divider_box(line: Pixels, clear: Option<impl IntoElement>) -> gpui::Div {
    // Clear ends where the titles do, 17 in from the edge.
    let right = if clear.is_some() { FAVICON_X } else { DIVIDER_X };
    div().flex_none().h(px(DIVIDER_HEIGHT)).child(
        div()
            .absolute()
            .top(px(DIVIDER_LINE_Y - CLEAR_HEIGHT / 2.))
            .left(px(DIVIDER_X))
            .right(px(right))
            .h(px(CLEAR_HEIGHT))
            .flex()
            .items_center()
            .child(div().flex_1().h(line).bg(rgba(DIVIDER)))
            .children(clear),
    )
}

/// The divider's Clear: the word, after an arrow down on a Library card, as in Arc. Arc's sidebar
/// shows the word alone.
fn clear_label(arrow: bool) -> gpui::Div {
    let inset = if arrow { CLEAR_ARROW_INSET } else { 0. };
    let arrow = arrow.then(|| {
        svg().path("icons/arrow-down.svg").size(px(CLEAR_ARROW)).text_color(rgba(CLEAR))
    });
    div()
        .flex_none()
        .ml(px(DIVIDER_GAP - inset))
        .h_full()
        .flex()
        .items_center()
        .gap(px(CLEAR_GAP - inset))
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(CLEAR))
        .children(arrow)
        .child("Clear")
}

/// "+ New Tab": its plus and its label.
fn new_tab_label() -> (gpui::Div, gpui::Div) {
    let plus = div().text_size(px(18.)).text_color(rgba(NEW_TAB_PLUS)).child("+");
    let label = div().text_size(px(14.)).text_color(rgba(NEW_TAB)).child("New Tab");
    (plus, label)
}

/// A row's plate: its icon in the favicon's square, then its label.
fn plate(icon: impl IntoElement, label: impl IntoElement) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .rounded(px(PLATE_RADIUS))
        .child(
            div()
                .flex_none()
                .ml(px(FAVICON_X - PLATE_INSET))
                .mr(px(TITLE_GAP))
                .size(px(FAVICON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .child(icon),
        )
        .child(label)
}

/// A folder's icon, open or closed.
fn folder_icon(open: bool) -> AnyElement {
    svg()
        .path(if open { "icons/folder-open.svg" } else { "icons/folder.svg" })
        .size(px(FAVICON_DRAWN))
        .text_color(rgba(SECONDARY))
        .into_any_element()
}

/// A tab's favicon, or a globe for a page without one.
fn favicon(favicon: Option<Arc<Image>>) -> AnyElement {
    match favicon {
        Some(favicon) => img(favicon).size(px(FAVICON_DRAWN)).into_any_element(),
        None => svg()
            .path("icons/globe.svg")
            .size(px(FAVICON_DRAWN))
            .text_color(rgba(SECONDARY))
            .into_any_element(),
    }
}

/// A split view's plate, holding its segments. Each segment's favicon is as far from its start as
/// from the line before it, so the first's is where a tab's is.
fn split_plate(segments: impl IntoIterator<Item = impl IntoElement>) -> gpui::Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .rounded(px(PLATE_RADIUS))
        .pl(px(FAVICON_X - PLATE_INSET - SEGMENT_FAVICON_X))
        .children(segments)
}

/// The selected tab's plate, at `opacity` while it fades in.
fn selected_plate(plate: gpui::Div, opacity: f32) -> gpui::Div {
    plate.bg(Hsla::from(rgba(SELECTED)).opacity(opacity))
}

/// A tab row's title, cut off at the row's trailing edge.
fn title(title: SharedString, selected: bool) -> gpui::Div {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .text_size(px(14.))
        .text_color(rgba(if selected { TITLE } else { UNSELECTED_TITLE }))
        .child(title)
}

/// How far a fade from `from` to `to` has come `elapsed` seconds into `duration`, on the scroll
/// edge's ease-out curve.
pub(crate) fn fade_at(from: f32, to: f32, elapsed: f32, duration: f32) -> f32 {
    from + (to - from) * cubic_bezier(0.25, 0.46, 0.45, 0.94, (elapsed / duration).min(1.))
}

/// Where a CSS cubic-bezier(`x1`, `y1`, `x2`, `y2`) curve is at `x`, from 0 to 1: the x of each
/// control point, then its y.
pub(crate) fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let at = |t: f32, p1: f32, p2: f32| {
        let u = 1. - t;
        3. * u * u * t * p1 + 3. * u * t * t * p2 + t * t * t
    };
    // x rises with t for every curve CSS allows, so halving finds t to well under a pixel.
    let (mut low, mut high) = (0., 1.);
    for _ in 0..20 {
        let t = (low + high) / 2.;
        if at(t, x1, x2) < x {
            low = t;
        } else {
            high = t;
        }
    }
    at((low + high) / 2., y1, y2)
}

/// The URL field's text for `url`: its host without "www." (Arc), or the URL itself if it has none.
fn host(url: &str) -> &str {
    // A data: URL can hold "://" further in, after characters no scheme has.
    let Some((scheme, rest)) = url.split_once("://") else {
        return url;
    };
    if !scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)) {
        return url;
    }
    let end = rest.find(['/', '?', '#', ':']).unwrap_or(rest.len());
    let host = &rest[..end];
    if host.is_empty() {
        return url;
    }
    host.strip_prefix("www.").unwrap_or(host)
}

/// The name of the file at a `file:` URL, with its escapes undone, which titles its page when the
/// page has no title, as in Chrome's tab strip. None for a folder.
fn file_name(url: &str) -> Option<String> {
    let path = url.strip_prefix("file://")?.split(['?', '#']).next()?;
    let name = path.rsplit('/').next().filter(|name| !name.is_empty())?;
    let mut bytes = Vec::new();
    let mut rest = name.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        let escaped = after.get(..2).and_then(|hex| std::str::from_utf8(hex).ok());
        match escaped.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
            Some(decoded) if byte == b'%' => {
                bytes.push(decoded);
                rest = &after[2..];
            }
            _ => {
                bytes.push(byte);
                rest = after;
            }
        }
    }
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Shape, fade_at, file_name};

    /// A fade reversed partway starts from the opacity it had, so nothing jumps, and ends at the
    /// other end.
    #[test]
    fn a_reversed_fade_starts_where_it_was() {
        let halfway = fade_at(0., 1., 0.05, 0.15);
        assert!(halfway > 0. && halfway < 1.);
        assert!((fade_at(halfway, 0., 0., 0.15) - halfway).abs() < 1e-3);
        assert!(fade_at(halfway, 0., 0.05, 0.15) < halfway);
        assert!(fade_at(halfway, 0., 0.15, 0.15).abs() < 1e-3);
        assert_eq!(fade_at(halfway, 0., 9., 0.15), fade_at(halfway, 0., 0.15, 0.15));
    }

    /// A file's page is titled by its name, as the user wrote it; a folder's and a site's aren't.
    #[test]
    fn a_file_is_titled_by_its_name() {
        let report = "file:///Users/someone/Report%20final.pdf";
        assert_eq!(file_name(report).unwrap(), "Report final.pdf");
        assert_eq!(file_name("file:///tmp/%E6%97%A5%E8%A8%98.txt#top").unwrap(), "日記.txt");
        assert_eq!(file_name("file:///tmp/100%.txt").unwrap(), "100%.txt");
        assert_eq!(file_name("file:///Users/someone/"), None);
        assert_eq!(file_name("https://example.com/notes.txt"), None);
    }

    /// A row turning into a tile is partway there 20 ms in, and turned for good once the spring
    /// has settled.
    #[test]
    fn a_dragged_row_turns_into_a_tile() {
        let start = Instant::now();
        let mut shape = Shape::new(false);
        shape.turn(true, start);
        let partway = shape.at(start + Duration::from_millis(20));
        assert!(partway > 0.3 && partway < 0.8, "20 ms in it's {partway} of the way");
        assert!(shape.turning(start + Duration::from_millis(20)));
        assert!(!shape.turning(start + Duration::from_millis(100)));
        assert_eq!(shape.at(start + Duration::from_millis(100)), 1.);
    }

    /// Crossing back mid-turn turns it back from where it was, with no jump, to a whole row.
    #[test]
    fn crossing_back_reverses_from_where_it_is() {
        let start = Instant::now();
        let mut shape = Shape::new(false);
        shape.turn(true, start);
        let back = start + Duration::from_millis(15);
        let there = shape.at(back);
        shape.turn(false, back);
        assert_eq!(shape.at(back), there);
        let later = shape.at(back + Duration::from_millis(5));
        assert!(later < there && later > 0., "5 ms after turning back it's at {later}");
        assert_eq!(shape.at(back + Duration::from_millis(100)), 0.);
        // Crossing again the way it's going changes nothing.
        let at = back + Duration::from_millis(20);
        let going = shape.at(at);
        shape.turn(false, back + Duration::from_millis(10));
        assert_eq!(shape.at(at), going);
    }
}
