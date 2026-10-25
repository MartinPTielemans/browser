//! The browser window's UI, under the page: the sidebar and the card the page sits in, at Arc's
//! measured sizes (docs/ux-spec.md › Window and › Sidebar). The shell shows what the browser tells
//! it and asks the browser for changes, so the engine stays the one place tabs live.

use std::{
    borrow::Cow,
    cell::Cell,
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Action, Animation, AnimationExt, AnyElement, App, AssetSource, Bounds, BoxShadow, Context,
    CursorStyle,
    DispatchPhase, Entity, FocusHandle, Focusable, FontWeight, Global, Hsla, Image, KeyBinding,
    MenuItem, MouseButton, MouseDownEvent, MouseExitEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Rgba, ScrollDelta, ScrollEvent, ScrollHandle, SharedString, Size, Subscription, Task,
    TouchPhase,
    Transformation, WeakEntity, Window, actions, canvas, div, img, linear_color_stop,
    hsla, linear_gradient, percentage, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::text_input::TextInput;

mod library;

pub use library::{Archived, Download, DownloadAction, DownloadState};
use library::Section;

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
// The rows under a folder make room as it opens and close up as it closes on Dia's group spring
// (docs/ux-spec.md › Motion: Group expand).
const FOLD_RESPONSE: f32 = 0.3;
const FOLD_DAMPING: f32 = 0.82;
// Two fingers across the sidebar page between Spaces (docs/ux-spec.md › Spaces › Switching). The
// pages follow at half the fingers' way and rubber-band past the ends. Let go, they settle
// critically damped on the next Space if the last 100 ms moved them faster than a flick, else on
// the nearest. A wheel pages once per burst of ticks, then rests. AppKit's line is about 10 pt.
const SWIPE_TRACKING: f32 = 0.5;
const SWIPE_FLICK: f32 = 5.;
const SWIPE_HISTORY: Duration = Duration::from_millis(100);
const SPACE_SETTLE: f32 = 0.25;
const WHEEL_LINE: f32 = 10.;
const WHEEL_BURST: Duration = Duration::from_millis(49);
const WHEEL_REST: Duration = Duration::from_millis(250);

// The header's buttons: their plates' tops and left edges, from the window's. The sidebar button's
// is a pitch before back's, as Arc's is.
const NAV_TOP: f32 = 7.5;
const SIDEBAR_BUTTON_X: f32 = 88.;
const BACK_X: f32 = 122.;
const FORWARD_X: f32 = 156.;
const RELOAD_X: f32 = 190.;
const NAV_SIZE: f32 = 32.;
const NAV_RADIUS: f32 = 6.;
const NAV_GLYPH: f32 = 16.;
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
// The engine's favicons are 16 pt, drawn at 2×, so they sit in the middle of Arc's 18 pt slot
// rather than scaled up soft.
const FAVICON_DRAWN: f32 = 16.;
const LOADING_SIZE: f32 = 12.;
// One clockwise turn (docs/ux-spec.md › Motion).
const LOADING_TURN: Duration = Duration::from_millis(1880);
const TITLE_GAP: f32 = 8.;
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
// Arc's divider line: one device pixel at 2x, 18 in from each side of the sidebar.
const HAIRLINE: f32 = 0.5;
const DIVIDER_X: f32 = 18.;
// The Clear divider, under the pinned tabs. Its line's centre is 25 pt above the centre of the first
// row under it. While Clear shows, the line ends 7.5 pt before Clear's arrow.
const DIVIDER_HEIGHT: f32 = 15.;
const DIVIDER_LINE_Y: f32 = DIVIDER_HEIGHT - (25. - ROW_HEIGHT / 2.);
const DIVIDER_GAP: f32 = 7.5;
// Clear's arrow, 8 pt tall and 3 pt before its label (Arc). Lucide's arrow-down spans 14 of its 24.
const CLEAR_ARROW: f32 = 8. * 24. / 14.;
const CLEAR_ARROW_INSET: f32 = CLEAR_ARROW * 5. / 24.;
const CLEAR_GAP: f32 = 3.;
const CLEAR_HEIGHT: f32 = 20.;
// How long the toast after a copy shows (ours: Arc's isn't measured).
const COPIED_TIME: Duration = Duration::from_secs(2);
// The footer's Space switcher, between where Arc has its Home and + buttons.
const FOOTER_HEIGHT: f32 = 50.;
const SWITCHER_LEFT: f32 = 42.;
const SWITCHER_RIGHT: f32 = 186.;
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
const CLEAR: u32 = 0xffffff73;
// The toast: white 0.1 over the window, opaque over the rows it covers (ours: Arc's isn't
// measured).
const TOAST: u32 = 0xffffff1a;
// A tab whose page isn't loaded fades its favicon (ours: Arc's alpha isn't measured).
const SLEEPING_FAVICON: f32 = 0.5;
// A favorite's tile: white 0.10 with a white 0.14 edge at rest (Dia), and the one shown white 0.20
// with Dia's default selection ring, white 0.45, until tiles are themed by their icons. Under the
// pointer it's white 0.15 (ours: Dia's isn't measured).
const TILE: f32 = 0.1;
const TILE_EDGE: u32 = 0xffff_ff24;
const TILE_HOVERED: f32 = 0.15;
const TILE_RING: u32 = 0xffff_ff73;
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
// Space in the dark blue Space's accent, labelled white 0.27 until there's a name. Its label with a
// name, and the name itself, are white (ours: Arc's aren't captured).
const FORM_TEXT: u32 = 0xffffff;
const FORM_PLATE: u32 = 0xffffff13;
const FORM_PLACEHOLDER: u32 = 0xffffff8c;
const CREATE: u32 = 0x25278a;
const CREATE_LABEL_DISABLED: u32 = 0xffffff45;
// The reveal panel's 0.5 pt light border, white 0.15 (ours: Arc's colour isn't measured).
const REVEAL_BORDER: u32 = 0xffffff26;
// The panel draws the window's buttons itself, since AppKit's stay in the browser window, under
// it: their centres from the panel's corner, as the docked sidebar's are from the window's, their
// size, and their colours, sampled from AppKit's in a capture of the browser window. The glyphs
// show while the pointer is over any of them, as AppKit's do.
const WINDOW_BUTTONS: [(f32, u32); 3] = [(20., 0xe0604f), (43., 0xf3bf12), (66., 0x58bb18)];
const WINDOW_BUTTON_Y: f32 = 24.;
const WINDOW_BUTTON_SIZE: f32 = 14.;
const WINDOW_BUTTON_GLYPH: u32 = 0x00000080;

/// What an item of a tab's menu does once chosen.
type MenuAction = Box<dyn FnOnce(&mut Shell, &mut Context<Shell>)>;

/// A browser command, from a key or the sidebar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    NewTab,
    EditUrl,
    CloseTab,
    /// Reopens the Today tab closed last.
    ReopenClosedTab,
    NextTab,
    PreviousTab,
    /// The tab shown before this one, and on while ⌃ is held.
    RecentTab,
    /// The same walk, backwards.
    RecentTabBack,
    /// The tab at this index in sidebar order.
    SelectTab(usize),
    SelectLastTab,
    /// Pins the selected tab, or unpins it.
    TogglePin,
    /// Makes a folder at the bottom of the pinned tabs, and names it.
    NewFolder,
    /// Closes every folder of the Space shown.
    CollapseFolders,
    /// Loads the selected pinned tab's pinned URL.
    BackToPinnedUrl,
    /// Closes the Today tabs of the Space shown, but the one shown.
    Clear,
    /// Brings back the tabs the last Clear closed.
    UndoClear,
    /// Copies the selected tab's URL.
    CopyUrl,
    /// Copies a Markdown link to the selected tab: its title, then its URL.
    CopyMarkdownLink,
    Find,
    FindNext,
    FindPrevious,
    Back,
    Forward,
    Reload,
    /// Reloads without the cache.
    ForceReload,
    Stop,
    /// Zooms the page's site to 100 %.
    ActualSize,
    ZoomIn,
    ZoomOut,
    /// Opens the page's source in a tab under it.
    ViewSource,
    /// Asks where to save the page, and saves it.
    SavePage,
    /// Shows print preview for the page.
    Print,
    /// Opens the page's DevTools, or closes them.
    DevTools,
    /// Opens the page's DevTools picking an element.
    InspectElements,
    /// Opens the page's DevTools at the console, or closes them there.
    JavaScriptConsole,
    NewSpace,
    /// The Space at this index in the switcher.
    SelectSpace(usize),
    NextSpace,
    PreviousSpace,
    /// Hides the docked sidebar, or shows it.
    ToggleSidebar,
    /// Opens the Library, or closes it.
    ViewLibrary,
    /// Opens the Library on the archive.
    ViewArchive,
    ClearArchive,
    /// Opens the Library on the downloads, or closes it there.
    ViewDownloads,
    /// Opens the command bar over a new pane beside the pages shown, for a split view.
    AddSplit,
    /// Closes the split view's focused page.
    CloseSplitPane,
    /// Takes the split view's focused page out of it, to show alone.
    SeparatePane,
    /// Focuses the split view's page right of the focused one, or left of it.
    NextPane,
    PreviousPane,
    Quit,
}

/// The browser's shortcuts (docs/ux-spec.md › Keyboard): the keystroke, with modifiers in the order
/// ctrl, alt, shift, cmd, its command, and whether it's reserved. As in Chrome, reserved ones reach
/// the browser before the page sees them, and the others only if the page doesn't keep them.
const SHORTCUTS: &[(&str, Command, bool)] = &[
    ("cmd-t", Command::NewTab, true),
    ("cmd-w", Command::CloseTab, true),
    ("shift-cmd-t", Command::ReopenClosedTab, true),
    ("cmd-q", Command::Quit, true),
    ("alt-cmd-down", Command::NextTab, true),
    ("alt-cmd-up", Command::PreviousTab, true),
    ("ctrl-tab", Command::RecentTab, true),
    ("ctrl-shift-tab", Command::RecentTabBack, true),
    ("alt-cmd-right", Command::NextSpace, true),
    ("alt-cmd-left", Command::PreviousSpace, true),
    ("cmd-l", Command::EditUrl, false),
    ("cmd-d", Command::TogglePin, false),
    ("cmd-enter", Command::BackToPinnedUrl, false),
    ("shift-cmd-k", Command::Clear, false),
    ("cmd-z", Command::UndoClear, false),
    ("shift-cmd-c", Command::CopyUrl, false),
    ("alt-shift-cmd-c", Command::CopyMarkdownLink, false),
    ("cmd-f", Command::Find, false),
    ("cmd-g", Command::FindNext, false),
    ("shift-cmd-g", Command::FindPrevious, false),
    ("cmd-1", Command::SelectTab(0), false),
    ("cmd-2", Command::SelectTab(1), false),
    ("cmd-3", Command::SelectTab(2), false),
    ("cmd-4", Command::SelectTab(3), false),
    ("cmd-5", Command::SelectTab(4), false),
    ("cmd-6", Command::SelectTab(5), false),
    ("cmd-7", Command::SelectTab(6), false),
    ("cmd-8", Command::SelectTab(7), false),
    ("cmd-9", Command::SelectLastTab, false),
    ("cmd-[", Command::Back, false),
    ("cmd-]", Command::Forward, false),
    ("cmd-left", Command::Back, false),
    ("cmd-right", Command::Forward, false),
    ("cmd-r", Command::Reload, false),
    ("shift-cmd-r", Command::ForceReload, false),
    ("cmd-.", Command::Stop, false),
    ("cmd-0", Command::ActualSize, false),
    // The first is the one the menu shows. ⌘+ is ⇧⌘= on most layouts, which a page names either
    // way, and ⌘= is Chrome's hidden second ⌘+.
    ("cmd-+", Command::ZoomIn, false),
    ("cmd-=", Command::ZoomIn, false),
    ("shift-cmd-=", Command::ZoomIn, false),
    ("shift-cmd-+", Command::ZoomIn, false),
    ("cmd--", Command::ZoomOut, false),
    ("alt-cmd-u", Command::ViewSource, false),
    ("shift-cmd-s", Command::SavePage, false),
    ("cmd-p", Command::Print, false),
    ("alt-cmd-i", Command::DevTools, false),
    ("alt-cmd-c", Command::InspectElements, false),
    ("alt-cmd-j", Command::JavaScriptConsole, false),
    ("cmd-s", Command::ToggleSidebar, false),
    ("shift-cmd-l", Command::ViewLibrary, false),
    ("shift-cmd-j", Command::ViewDownloads, false),
    // ⌃+ is ⌃⇧= on most layouts, as ⌘+ is.
    ("ctrl-+", Command::AddSplit, false),
    ("ctrl-shift-=", Command::AddSplit, false),
    ("ctrl-shift-+", Command::AddSplit, false),
    ("ctrl-shift--", Command::CloseSplitPane, false),
    ("ctrl-shift-_", Command::CloseSplitPane, false),
    ("ctrl-shift-]", Command::NextPane, false),
    ("ctrl-shift-}", Command::NextPane, false),
    ("ctrl-shift-[", Command::PreviousPane, false),
    ("ctrl-shift-{", Command::PreviousPane, false),
    ("escape", Command::Stop, false),
    ("ctrl-1", Command::SelectSpace(0), false),
    ("ctrl-2", Command::SelectSpace(1), false),
    ("ctrl-3", Command::SelectSpace(2), false),
    ("ctrl-4", Command::SelectSpace(3), false),
    ("ctrl-5", Command::SelectSpace(4), false),
    ("ctrl-6", Command::SelectSpace(5), false),
    ("ctrl-7", Command::SelectSpace(6), false),
    ("ctrl-8", Command::SelectSpace(7), false),
    ("ctrl-9", Command::SelectSpace(8), false),
];

/// The commands the command bar finds by name: Arc's bar's names where they're known, else its
/// menus' (docs/ux-spec.md › Command bar › Rows).
const BAR_COMMANDS: &[(&str, Command)] = &[
    ("Clear Today", Command::Clear),
    ("Clear Archive", Command::ClearArchive),
    ("Collapse All Folders", Command::CollapseFolders),
    ("New Folder", Command::NewFolder),
    ("Pin/Unpin Tab", Command::TogglePin),
    ("Back to Pinned URL", Command::BackToPinnedUrl),
    ("Archive Tab", Command::CloseTab),
    ("Restore Last Closed Tab", Command::ReopenClosedTab),
    ("Copy URL", Command::CopyUrl),
    ("Copy URL as Markdown", Command::CopyMarkdownLink),
    ("Find", Command::Find),
    ("Go Back", Command::Back),
    ("Go Forward", Command::Forward),
    ("Refresh the Page", Command::Reload),
    ("Force Refresh the Page", Command::ForceReload),
    ("Stop Loading", Command::Stop),
    ("Zoom In", Command::ZoomIn),
    ("Zoom Out", Command::ZoomOut),
    ("Zoom to Actual Size", Command::ActualSize),
    ("Close this Split Pane", Command::CloseSplitPane),
    ("Separate Page from Split View", Command::SeparatePane),
    ("Show/Hide Sidebar", Command::ToggleSidebar),
    ("Save Page As", Command::SavePage),
    ("Print", Command::Print),
    ("View Source", Command::ViewSource),
    ("Developer Tools", Command::DevTools),
    ("Inspect Elements", Command::InspectElements),
    ("JavaScript Console", Command::JavaScriptConsole),
    ("New Space", Command::NewSpace),
    ("Next Space", Command::NextSpace),
    ("Previous Space", Command::PreviousSpace),
    ("Next Tab", Command::NextTab),
    ("Previous Tab", Command::PreviousTab),
    ("View Library", Command::ViewLibrary),
    ("View Archive", Command::ViewArchive),
    ("View Downloads", Command::ViewDownloads),
];

/// The most commands the command bar lists at once.
const BAR_COMMANDS_SHOWN: usize = 4;

/// A command the command bar offers for what's typed.
pub struct BarCommand {
    pub name: SharedString,
    pub command: Command,
    /// The shortcut that runs it, as a menu shows it.
    pub shortcut: Option<SharedString>,
    /// Whether its name starts with what's typed, three letters or more, which puts it above
    /// the core's suggestions.
    pub strong: bool,
}

/// The commands that do something now whose names match `typed`, best first, as Arc's bar
/// matches: the name starting with it, then a word of it, then its letters in order from a word's
/// start. Nothing for under two letters.
pub fn bar_commands(typed: &str, cx: &App) -> Vec<BarCommand> {
    let typed = typed.trim().to_lowercase();
    let shell = cx.try_global::<ShellHandle>().and_then(|handle| handle.0.upgrade());
    let Some(shell) = shell.filter(|_| typed.chars().count() >= 2) else {
        return Vec::new();
    };
    let shell = shell.read(cx);
    let mut found: Vec<(u8, BarCommand)> = BAR_COMMANDS
        .iter()
        .filter(|&&(_, command)| shell.offers(command))
        .filter_map(|&(name, command)| {
            let score = match_score(name, &typed)?;
            let shortcut = SHORTCUTS.iter().find(|&&(_, each, _)| each == command);
            Some((
                score,
                BarCommand {
                    name: name.into(),
                    command,
                    shortcut: shortcut.map(|&(key, ..)| keystroke_label(key).into()),
                    strong: score == 3 && typed.chars().count() >= 3,
                },
            ))
        })
        .collect();
    found.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    found.into_iter().take(BAR_COMMANDS_SHOWN).map(|(_, command)| command).collect()
}

/// How well `name` matches `typed`, which is lowercase: 3 if the name starts with it, 2 if a word
/// of it does, 1 if its letters are in it in order from a word's start, else none.
fn match_score(name: &str, typed: &str) -> Option<u8> {
    let name = name.to_lowercase();
    let words: Vec<&str> = name
        .char_indices()
        .filter(|&(at, _)| at == 0 || name[..at].ends_with([' ', '/']))
        .map(|(at, _)| &name[at..])
        .collect();
    let first = typed.chars().next()?;
    let in_order = |word: &str| {
        let mut letters = word.chars();
        typed.chars().all(|letter| letters.any(|other| other == letter))
    };
    if name.starts_with(typed) {
        Some(3)
    } else if words.iter().any(|word| word.starts_with(typed)) {
        Some(2)
    } else {
        words.iter().any(|word| word.starts_with(first) && in_order(word)).then_some(1)
    }
}

/// How a menu shows `keystroke`, as SHORTCUTS writes it: ⇧⌘K for shift-cmd-k.
fn keystroke_label(keystroke: &str) -> String {
    let mut rest = keystroke;
    let mut label = String::new();
    for (modifier, glyph) in [("ctrl-", "⌃"), ("alt-", "⌥"), ("shift-", "⇧"), ("cmd-", "⌘")] {
        if let Some(after) = rest.strip_prefix(modifier) {
            label.push_str(glyph);
            rest = after;
        }
    }
    let key = match rest {
        "enter" => "↩".to_string(),
        "escape" => "⎋".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "tab" => "⇥".to_string(),
        key => key.to_uppercase(),
    };
    label + &key
}

/// Runs `command`, whose row in the command bar was chosen, once the bar has gone.
pub fn run_from_bar(command: Command, cx: &mut App) {
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell
            .update(cx, |shell, cx| {
                shell.command_bar_dismissed(cx);
                shell.run(command, "command bar", cx);
            })
            .ok();
    }
}

/// An item in a menu the host shows. A line between items has no title.
#[derive(Clone)]
pub struct MenuEntry {
    pub title: SharedString,
    pub enabled: bool,
    /// The keystroke it shows, as SHORTCUTS writes it.
    pub shortcut: Option<&'static str>,
    /// The item whose submenu it's in, by index.
    pub parent: Option<usize>,
    /// Whether it shows a check mark.
    pub checked: bool,
    /// A round swatch of this colour, 0xRRGGBB, before its title.
    pub swatch: Option<u32>,
}

impl MenuEntry {
    fn new(title: impl Into<SharedString>, enabled: bool) -> Self {
        MenuEntry {
            title: title.into(),
            enabled,
            shortcut: None,
            parent: None,
            checked: false,
            swatch: None,
        }
    }

    fn line() -> Self {
        MenuEntry::new("", false)
    }

    /// In the submenu of the item at `parent`.
    fn under(self, parent: usize) -> Self {
        MenuEntry { parent: Some(parent), ..self }
    }

    fn checked(self, checked: bool) -> Self {
        MenuEntry { checked, ..self }
    }

    fn swatch(self, color: u32) -> Self {
        MenuEntry { swatch: Some(color), ..self }
    }

    /// Shows the shortcut that runs `command`.
    fn shortcut(self, command: Command) -> Self {
        let shortcut =
            SHORTCUTS.iter().find(|&&(_, each, _)| each == command).map(|&(key, ..)| key);
        MenuEntry { shortcut, ..self }
    }
}

impl Command {
    /// The command `keystroke` runs, if it's a shortcut that is `reserved` or isn't.
    pub fn for_keystroke(keystroke: &str, reserved: bool) -> Option<Command> {
        SHORTCUTS
            .iter()
            .find(|(key, _, is_reserved)| *key == keystroke && *is_reserved == reserved)
            .map(|&(_, command, _)| command)
    }

    fn name(self) -> &'static str {
        match self {
            Command::NewTab => "new-tab",
            Command::EditUrl => "edit-url",
            Command::CloseTab => "close-tab",
            Command::ReopenClosedTab => "reopen-closed-tab",
            Command::NextTab => "next-tab",
            Command::PreviousTab => "previous-tab",
            Command::RecentTab => "recent-tab",
            Command::RecentTabBack => "recent-tab-back",
            Command::SelectTab(_) => "select-tab",
            Command::SelectLastTab => "select-last-tab",
            Command::TogglePin => "toggle-pin",
            Command::NewFolder => "new-folder",
            Command::CollapseFolders => "collapse-folders",
            Command::BackToPinnedUrl => "back-to-pinned-url",
            Command::Clear => "clear",
            Command::UndoClear => "undo-clear",
            Command::CopyUrl => "copy-url",
            Command::CopyMarkdownLink => "copy-markdown-link",
            Command::Find => "find",
            Command::FindNext => "find-next",
            Command::FindPrevious => "find-previous",
            Command::Back => "back",
            Command::Forward => "forward",
            Command::Reload => "reload",
            Command::ForceReload => "force-reload",
            Command::Stop => "stop",
            Command::ActualSize => "actual-size",
            Command::ZoomIn => "zoom-in",
            Command::ZoomOut => "zoom-out",
            Command::ViewSource => "view-source",
            Command::SavePage => "save-page",
            Command::Print => "print",
            Command::DevTools => "devtools",
            Command::InspectElements => "inspect-elements",
            Command::JavaScriptConsole => "javascript-console",
            Command::NewSpace => "new-space",
            Command::SelectSpace(_) => "select-space",
            Command::NextSpace => "next-space",
            Command::PreviousSpace => "previous-space",
            Command::ToggleSidebar => "toggle-sidebar",
            Command::ViewLibrary => "view-library",
            Command::ViewArchive => "view-archive",
            Command::ClearArchive => "clear-archive",
            Command::ViewDownloads => "view-downloads",
            Command::AddSplit => "add-split",
            Command::CloseSplitPane => "close-split-pane",
            Command::SeparatePane => "separate-pane",
            Command::NextPane => "next-pane",
            Command::PreviousPane => "previous-pane",
            Command::Quit => "quit",
        }
    }
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct Run(Command);

actions!(space_form, [CreateSpace]);
actions!(space_name, [KeepName, CancelName]);

struct ShellHandle(WeakEntity<Shell>);

impl Global for ShellHandle {}

/// The key context at the root of the browser's windows, where its shortcuts apply. GPUI ranks a
/// binding with no context above every other, so without one the shortcuts would take keys a
/// focused field binds for itself, such as escape and ⌘←.
pub const KEY_CONTEXT: &str = "Browser";

/// Binds the shortcuts for while the UI has the keyboard. The browser hands them over itself while
/// the page has it.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys(
        SHORTCUTS
            .iter()
            .map(|&(key, command, _)| KeyBinding::new(key, Run(command), Some(KEY_CONTEXT))),
    );
    cx.bind_keys([
        KeyBinding::new("enter", CreateSpace, Some("SpaceForm")),
        KeyBinding::new("enter", KeepName, Some("Naming")),
        KeyBinding::new("escape", CancelName, Some("Naming")),
    ]);
    cx.on_action(|run: &Run, cx| {
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell
                .update(cx, |shell, cx| shell.run(run.0, "ui", cx))
                .ok();
        }
    });
}

/// The command bar is about to show a page in the pane being added, if one is: the pages shown take
/// back its room first, so the split view shows at its place at once.
pub fn pane_added(cx: &mut App) {
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell
            .update(cx, |shell, cx| {
                if shell.adding_pane.take().is_some() {
                    shell.lay_out_page(cx);
                    cx.notify();
                }
            })
            .ok();
    }
}

/// The tab the window shows, or 0.
pub fn shown_tab(cx: &App) -> u64 {
    let shell = cx.try_global::<ShellHandle>().and_then(|handle| handle.0.upgrade());
    shell.map_or(0, |shell| shell.read(cx).selected)
}

/// A menu item that runs `command`, showing its shortcut.
pub fn menu_item(name: &'static str, command: Command) -> MenuItem {
    MenuItem::action(name, Run(command))
}

/// The shell's glyphs, built in. Lucide's (NOTICE), and the loading ring and the window buttons'
/// glyphs, which are ours.
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    ("icons/archive.svg", include_bytes!("../assets/icons/archive.svg")),
    ("icons/arrow-down.svg", include_bytes!("../assets/icons/arrow-down.svg")),
    (
        "icons/arrow-down-to-line.svg",
        include_bytes!("../assets/icons/arrow-down-to-line.svg"),
    ),
    ("icons/arrow-left.svg", include_bytes!("../assets/icons/arrow-left.svg")),
    ("icons/arrow-right.svg", include_bytes!("../assets/icons/arrow-right.svg")),
    ("icons/chevron-down.svg", include_bytes!("../assets/icons/chevron-down.svg")),
    ("icons/chevron-up.svg", include_bytes!("../assets/icons/chevron-up.svg")),
    ("icons/file.svg", include_bytes!("../assets/icons/file.svg")),
    ("icons/folder.svg", include_bytes!("../assets/icons/folder.svg")),
    ("icons/folder-open.svg", include_bytes!("../assets/icons/folder-open.svg")),
    ("icons/globe.svg", include_bytes!("../assets/icons/globe.svg")),
    ("icons/link.svg", include_bytes!("../assets/icons/link.svg")),
    ("icons/loading.svg", include_bytes!("../assets/icons/loading.svg")),
    ("icons/panel-left.svg", include_bytes!("../assets/icons/panel-left.svg")),
    ("icons/rotate-cw.svg", include_bytes!("../assets/icons/rotate-cw.svg")),
    ("icons/search.svg", include_bytes!("../assets/icons/search.svg")),
    ("icons/window-close.svg", include_bytes!("../assets/icons/window-close.svg")),
    ("icons/window-full-screen.svg", include_bytes!("../assets/icons/window-full-screen.svg")),
    ("icons/window-minimize.svg", include_bytes!("../assets/icons/window-minimize.svg")),
    ("icons/x.svg", include_bytes!("../assets/icons/x.svg")),
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

/// What the shell asks of the browser. The browser answers by updating the shell afterwards, never
/// during the call.
pub trait Browser {
    fn select(&self, tab: u64, cx: &mut App);
    /// Closes a Today tab. A pinned tab stays, and only its page goes. A tab in a split view
    /// leaves it, unless `split` closes the rest of the split with it.
    fn close(&self, tab: u64, split: bool, cx: &mut App);
    /// Closes the Today tabs below `tab`, showing `tab` if one of them showed.
    fn close_below(&self, tab: u64, cx: &mut App);
    /// Opens a copy of `tab` with its history, under it in Today, and shows it.
    fn duplicate(&self, tab: u64, cx: &mut App);
    /// Moves `tab` to `space`. The window goes with it if it shows it.
    fn move_to_space(&self, tab: u64, space: u64, cx: &mut App);
    /// Pins `tab` at the bottom of its Space's pinned tabs, or unpins it to the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App);
    /// Makes `tab` the favorite at `index` of the favorites without it, else the last, out of its
    /// Space, or moves it there among them.
    fn move_to_favorites(&self, tab: u64, index: usize, cx: &mut App);
    /// Moves `tab` to `index` in its Space's Today tabs, or in its pinned tabs, inside `folder` if
    /// it isn't 0, pinning or unpinning it, with the rest of its split view. `index` counts tabs,
    /// not rows, without the split's, and folders directly in the pinned tabs or `folder` count
    /// one. A favorite goes to the Space shown.
    fn move_tab(&self, tab: u64, pinned: bool, folder: u64, index: usize, cx: &mut App);
    /// Makes an open folder named `name` at `index` in `folder`, else in the pinned tabs of
    /// `space`, counted as `move_tab` counts, else at the bottom, and returns it.
    fn create_folder(&self, space: u64, name: &str, folder: u64, index: usize, cx: &mut App)
    -> u64;
    fn rename_folder(&self, folder: u64, name: &str, cx: &mut App);
    fn set_folder_open(&self, folder: u64, open: bool, cx: &mut App);
    /// Moves `folder`, with what's in it, to `index` in `to`, else in the pinned tabs, counted as
    /// `move_tab` counts.
    fn move_folder(&self, folder: u64, to: u64, index: usize, cx: &mut App);
    /// Deletes `folder`, leaving what was in it where it was.
    fn delete_folder(&self, folder: u64, cx: &mut App);
    /// Takes `tab` out of its split view.
    fn separate(&self, tab: u64, cx: &mut App);
    /// Loads a pinned tab's pinned URL, unless it's there already.
    fn back_to_pinned_url(&self, tab: u64, cx: &mut App);
    /// Closes the Today tabs of the Space shown, but the one shown.
    fn clear(&self, cx: &mut App);
    /// Brings back the tabs the last clear closed, and returns how many.
    fn undo_clear(&self, cx: &mut App) -> usize;
    /// Reopens the Today tab closed last, and shows it.
    fn reopen_closed_tab(&self, cx: &mut App);
    /// Reopens the archived tab `archived` at the top of the Today tabs of the Space shown, and
    /// shows it.
    fn restore_archived(&self, archived: u64, cx: &mut App);
    fn clear_archive(&self, cx: &mut App);
    /// Does `action` to the download.
    fn act_on_download(&self, download: u64, action: DownloadAction, cx: &mut App);
    /// Takes the downloads that aren't running or paused off the list, keeping their files.
    fn clear_downloads(&self, cx: &mut App);
    /// Looks up the icon the system shows for the download's file, and calls
    /// `Shell::set_download_icon`.
    fn download_icon(&self, download: u64, cx: &mut App);
    /// Looks up the favicon the browser has for `url`, and calls `Shell::set_page_favicon`.
    fn page_favicon(&self, url: &str, cx: &mut App);
    /// The starts of today, yesterday, this week and last week in the user's calendar, in
    /// milliseconds since the Unix epoch.
    fn day_starts(&self) -> [f64; 4];
    fn go_back(&self, tab: u64, cx: &mut App);
    fn go_forward(&self, tab: u64, cx: &mut App);
    fn reload(&self, tab: u64, bypass_cache: bool, cx: &mut App);
    fn stop(&self, tab: u64, cx: &mut App);
    /// Zooms the tab's page in (1), out (-1) or to 100 % (0), for its site.
    fn zoom(&self, tab: u64, step: i32, cx: &mut App);
    /// Opens the tab's source in a tab under it.
    fn view_source(&self, tab: u64, cx: &mut App);
    /// Asks where to save the tab's page, and saves it there.
    fn save_page(&self, tab: u64, cx: &mut App);
    /// Shows print preview for the tab's page.
    fn print(&self, tab: u64, cx: &mut App);
    /// Opens or closes the tab's DevTools (0), opens or closes them at the console (1), or opens
    /// them picking an element (2).
    fn devtools(&self, tab: u64, what: i32, cx: &mut App);
    fn show_space(&self, space: u64, cx: &mut App);
    /// The tab the window shows when it shows `space`, or the one it shows, else 0.
    fn space_tab(&self, space: u64) -> u64;
    /// Makes an empty Space named `name` after the others, and returns it.
    fn create_space(&self, name: &str, cx: &mut App) -> u64;
    fn rename_space(&self, space: u64, name: &str, cx: &mut App);
    /// Names a tab, which shows `name` for its title from then on, or gives it back its page's
    /// title for an empty one.
    fn rename_tab(&self, tab: u64, name: &str, cx: &mut App);
    /// Deletes a Space, closing its tabs.
    fn delete_space(&self, space: u64, cx: &mut App);
    /// Gives a Space a colour, 0xRRGGBB, or none, for the system's accent colour.
    fn set_space_color(&self, space: u64, color: Option<u32>, cx: &mut App);
    /// Shows a menu of `items` at `position` in the window, or in the reveal's panel if
    /// `in_reveal`, as a right-click there does. Then calls `chosen` once it closes, with the index
    /// of the item chosen, if any.
    fn menu(
        &self,
        items: Vec<MenuEntry>,
        position: Point<Pixels>,
        in_reveal: bool,
        chosen: Box<dyn FnOnce(Option<usize>, &mut App)>,
        cx: &mut App,
    );
    /// Asks whether to do what `message` asks, with `detail` under it, and calls `confirmed` if the
    /// user chooses `action` over Cancel.
    fn confirm(
        &self,
        message: String,
        detail: String,
        action: &'static str,
        confirmed: Box<dyn FnOnce(&mut App)>,
        cx: &mut App,
    );
    /// Gives the UI the keyboard, from the page if the page has it.
    fn focus_ui(&self, cx: &mut App);
    /// Lays the page out at `frame` in the window, with corners of `radius`
    /// (`Shell::page_frame`). A split view's pages share it.
    fn lay_out_page(&self, frame: Bounds<Pixels>, radius: f32, cx: &mut App);
    /// The colour of the ring inside a split view's focused page, 0xRRGGBB.
    fn set_focus_color(&self, color: u32, cx: &mut App);
    /// Keeps the find bar and the zoom HUD `right` points further in from the window's right
    /// edge, at the corner of a split view's focused page.
    fn set_page_right(&self, right: f32, cx: &mut App);
    /// Shows the window's close, minimize and zoom buttons, or hides them.
    fn show_window_buttons(&self, shown: bool, cx: &mut App);
    /// Does what a click on the window's close, minimize or zoom button does.
    fn press_window_button(&self, button: WindowButton, cx: &mut App);
    /// Shows the sidebar reveal's window over the page, `width` points wide from the browser
    /// window's left edge and as tall as it. It calls `Shell::pointer_at` as the pointer moves
    /// anywhere in the browser's windows until it's hidden.
    fn show_reveal(&self, width: f32, cx: &mut App);
    /// The docked sidebar's width the last launch kept, if it kept one.
    fn saved_sidebar_width(&self) -> Option<f32>;
    /// Keeps the docked sidebar's width for the next launch.
    fn set_sidebar_width(&self, width: f32, cx: &mut App);
    fn hide_reveal(&self, cx: &mut App);
    /// A detent on the trackpad, as a swipe's nearest Space changes.
    fn detent(&self, cx: &mut App);
    /// Calls `Shell::control_released` once the user lets go of ⌃.
    fn watch_control(&self, cx: &mut App);
    /// Puts `text` on the clipboard.
    fn copy(&self, text: &str, cx: &mut App);
    /// The system's accent colour, as 0xRRGGBB. The app calls `Shell::accent_color_changed` when
    /// it changes.
    fn accent_color(&self) -> u32;
    /// Opens the command bar, for a new tab or on `editing`'s URL: out of the URL field at `field`
    /// in the window, or centred over it.
    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        field: Option<Bounds<Pixels>>,
        cx: &mut App,
    );
    /// Opens the command bar over `pane`, a new pane in the card at `card`, for a page to show
    /// there in a split view with `with`.
    fn split_bar(&self, with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App);
    /// Opens the tab's find bar with the keyboard.
    fn find(&self, tab: u64, cx: &mut App);
    /// Selects the tab's next match, or the one before, opening its find bar if it's closed.
    fn find_next(&self, tab: u64, forward: bool, cx: &mut App);
    fn quit(&self, cx: &mut App);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WindowButton {
    Close,
    Minimize,
    Zoom,
}

/// Where a tab's page is, as the browser reports it.
#[derive(Clone, Default)]
pub struct Page {
    pub url: SharedString,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub loading: bool,
    /// The page isn't loaded, as in a tab put back and not yet shown, or a pinned tab closed. It
    /// loads once it's shown.
    pub unloaded: bool,
    /// The tab is pinned or a favorite, and its page isn't the one it was pinned or made a favorite
    /// at.
    pub away_from_pinned_url: bool,
}

/// The toast over the bottom of the list until it hides: what was just done.
struct Toast {
    text: SharedString,
    _hide: Task<()>,
}

#[derive(Default)]
struct Tab {
    title: SharedString,
    page: Page,
    /// None for a page without one.
    favicon: Option<Arc<Image>>,
}

/// A Space's folders in its pinned tabs. What's in each follows it there.
#[derive(Clone, Default)]
pub struct Folders {
    /// Each folder's name, and whether it's open.
    pub folders: HashMap<u64, (SharedString, bool)>,
    /// The folder holding each pinned tab or folder in one.
    pub parents: HashMap<u64, u64>,
}

struct Space {
    id: u64,
    name: SharedString,
    /// 0xRRGGBB, if it has one.
    color: Option<u32>,
    /// Its pinned tabs and folders and its Today tabs, each top to bottom.
    pinned: Vec<u64>,
    today: Vec<u64>,
    /// Its split views, each its tabs left to right, which sit together in their list as one row.
    splits: Vec<Vec<u64>>,
    folders: Folders,
    /// The tab it shows, or showed last.
    shown: u64,
}

impl Space {
    /// The tabs and folders of its pinned rows and its Today rows: each split's first stands for
    /// the split, and what's in a closed folder has none, but the tab shown's (Arc).
    fn rows(&self) -> (Vec<u64>, Vec<u64>) {
        let later = |tab: &u64| self.splits.iter().any(|split| split[1..].contains(tab));
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
        let split = self.splits.iter().find(|split| split.contains(&tab));
        split.cloned().unwrap_or_else(|| vec![tab])
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
}

/// The field a Space's name or a tab's title turns into for renaming it.
struct Renaming {
    naming: Naming,
    name: Entity<TextInput>,
    /// Keeps the name once the field loses GPUI's focus, as Finder does.
    _blur: Option<Subscription>,
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
/// Over the favorites it's drawn as a tile, and over the list as a row.
struct Drag {
    tab: u64,
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
}

/// Two fingers on the trackpad over the sidebar, which may page between Spaces.
struct Swipe {
    /// The Space shown when they came down.
    from: u64,
    /// Across the sidebar, to page, or along it, to scroll the list. None until they first move.
    across: Option<bool>,
    /// How far they've taken the page of the Space they began at, before the rubber band.
    way: f32,
    /// Where that page was over the last SWIPE_HISTORY, for how fast it's let go.
    samples: VecDeque<(Instant, f32)>,
    /// The Space nearest its place, for the detent as it changes.
    nearest: u64,
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
    /// The form's field takes GPUI's focus at the next draw, which is when the shell has a window.
    focus_form: bool,
    renaming: Option<Renaming>,
    /// The same for the renaming field.
    focus_renaming: bool,
    /// The pinned tabs and folders changed and haven't been drawn since.
    outline_changed: bool,
    /// The favorites, left to right, which every Space shows over its own tabs.
    favorites: Vec<u64>,
    /// The tabs of every Space, and the favorites.
    tabs: HashMap<u64, Tab>,
    selected: u64,
    /// Tabs of every Space, shown most recently first.
    recent: Vec<u64>,
    /// ⌃ is held after a ⌃Tab, and the tabs it shows don't count as shown until it goes up.
    walking: bool,
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
    toast: Option<Toast>,
    /// Whether the pointer is over the docked sidebar, which shows Clear and the URL field's button
    /// only then.
    sidebar_hovered: bool,
    page_fullscreen: bool,
    /// Whether ⌘S has hidden the sidebar.
    sidebar_hidden: bool,
    /// The docked sidebar's width, at rest.
    sidebar_width: f32,
    /// While the sidebar's edge is dragged: how far right of the edge the pointer pressed it, and
    /// the width the pointer has it at, before the rubber band.
    resize: Option<(f32, f32)>,
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
    /// The window's content size at the last draw, and the frame and corner radius the page was
    /// last laid out at.
    viewport: Size<Pixels>,
    page_laid: Option<(Bounds<Pixels>, f32)>,
    /// While the command bar is open for a pane beside the tab shown, that tab. The pages shown
    /// make room on their right for the pane (Arc).
    adding_pane: Option<u64>,
    /// The colour last given for a split view's focused page.
    focus_color: Option<u32>,
    /// How far in from the page frame's right edge the focused page's ends, as last given.
    page_right: f32,
    /// Every Space's archived tabs, newest first, and the favicons asked for their URLs.
    archive: Vec<Archived>,
    page_favicons: HashMap<SharedString, Option<Arc<Image>>>,
    /// The profile's downloads, newest first, and the file icons asked for them.
    downloads: Vec<Download>,
    download_icons: HashMap<u64, Option<Arc<Image>>>,
    /// The Library's section shown last, which it opens on.
    section: Section,
    /// The Library, from when it opens until it has closed.
    library: Option<library::Library>,
    library_open: bool,
    /// While the card moves for the Library: how far out it started (`library_out`), and when.
    library_motion: Option<(f32, Instant)>,
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
            focus_form: false,
            renaming: None,
            focus_renaming: false,
            outline_changed: false,
            favorites: Vec::new(),
            tabs: HashMap::new(),
            recent: Vec::new(),
            walking: false,
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
            toast: None,
            sidebar_hovered: false,
            page_fullscreen: false,
            sidebar_hidden: false,
            sidebar_width,
            resize: None,
            width_motion: None,
            sidebar_motion: None,
            revealed: false,
            reveal_motion: None,
            reveal_wait: None,
            viewport: Size::default(),
            page_laid: None,
            adding_pane: None,
            focus_color: None,
            page_right: 0.,
            archive: Vec::new(),
            page_favicons: HashMap::new(),
            downloads: Vec::new(),
            download_icons: HashMap::new(),
            section: Section::Archive,
            library: None,
            library_open: false,
            library_motion: None,
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

    /// The split views of the Space shown, each its tabs left to right.
    pub fn splits(&self) -> Vec<Vec<u64>> {
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

    /// The tabs shown with `tab` in the Space shown, left to right: its split's, else `tab` alone.
    fn panes(&self, tab: u64) -> Vec<u64> {
        self.shown_space().map_or_else(|| vec![tab], |space| space.panes(tab))
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

    pub fn space(&self) -> u64 {
        self.space
    }

    /// Where the page goes in a window whose content is `viewport`, and the radius of its corners.
    /// While a pane is being added beside the tab shown, the pages shown make room for it.
    pub fn page_frame(&self, viewport: Size<Pixels>) -> (Bounds<Pixels>, f32) {
        let card_left = self.card_left();
        let frame = page_frame(viewport, card_left, self.card_push());
        let radius = if card_left == 0. { 0. } else { CARD_RADIUS };
        match self.adding_pane.map(|with| self.panes(with).len()) {
            Some(count) => {
                let last = pane_frames(frame, count + 1)[count - 1];
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

    /// The docked sidebar's width as drawn: with the pointer while its edge is dragged, then on its
    /// way to where it rests.
    fn width(&self) -> f32 {
        if let Some((_, way)) = self.resize {
            return rubber_band(way, SIDEBAR_MIN, SIDEBAR_MAX).round();
        }
        let Some((from, start)) = self.width_motion else {
            return self.sidebar_width;
        };
        settle_progress(start)
            .map_or(self.sidebar_width, |progress| from + (self.sidebar_width - from) * progress)
    }

    /// The docked sidebar's right edge, which drags its width and puts it back on a double-click.
    fn sidebar_edge(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let width = self.width();
        let cursor = match width {
            width if width <= SIDEBAR_MIN => CursorStyle::ResizeRight,
            width if width >= SIDEBAR_MAX => CursorStyle::ResizeLeft,
            _ => CursorStyle::ResizeLeftRight,
        };
        let resizing = self.resize.is_some();
        let shell = cx.weak_entity();
        div()
            .id("sidebar-edge")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(width - SIDEBAR_EDGE))
            .w(px(SIDEBAR_EDGE))
            .cursor(cursor)
            .block_mouse_except_scroll()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseDownEvent, _, cx| shell.press_edge(event, cx)),
            )
            // The pointer's moves and release anywhere in the window, while the edge is dragged.
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        if resizing {
                            window.set_window_cursor_style(cursor);
                        }
                        let moved = shell.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture {
                                let x = event.position.x.into();
                                moved.update(cx, |shell, cx| shell.drag_edge(x, cx)).ok();
                            }
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                shell.update(cx, |shell, cx| shell.let_go_edge(cx)).ok();
                            }
                        });
                    },
                )
                .absolute()
                .size_0(),
            )
    }

    fn press_edge(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let width = self.width();
        if event.click_count == 2 {
            self.resize = None;
            self.set_width(width, SIDEBAR_WIDTH, cx);
            return;
        }
        self.width_motion = None;
        self.resize = Some((f32::from(event.position.x) - width, width));
        cx.notify();
    }

    fn drag_edge(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some((grab, _)) = self.resize {
            self.resize = Some((grab, x - grab));
            cx.notify();
        }
    }

    fn let_go_edge(&mut self, cx: &mut Context<Self>) {
        if let Some((_, way)) = self.resize.take() {
            let from = rubber_band(way, SIDEBAR_MIN, SIDEBAR_MAX).round();
            self.set_width(from, way.clamp(SIDEBAR_MIN, SIDEBAR_MAX), cx);
        }
    }

    /// Settles the docked sidebar at `to` wide from `from`, and keeps the width for the next
    /// launch.
    fn set_width(&mut self, from: f32, to: f32, cx: &mut Context<Self>) {
        let to = to.round();
        if to != self.sidebar_width {
            self.sidebar_width = to;
            self.browser.set_sidebar_width(to, cx);
        }
        self.width_motion = (from != to).then(|| (from, Instant::now()));
        if self.width_motion.is_none() {
            // check.sh waits for these lines.
            eprintln!("browser: sidebar width {to}");
        }
        cx.notify();
    }

    /// How far the sidebar has moved since `start`, or None once it's there.
    fn sidebar_progress(&self, start: Instant) -> Option<f32> {
        let t = start.elapsed().as_secs_f32();
        let time = if self.sidebar_hidden { SIDEBAR_HIDE } else { SIDEBAR_SHOW };
        (t < time).then(|| 1. - (1. - t / time).powi(3))
    }

    /// Hides the docked sidebar, or shows it, from wherever it is.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        // A full-screen page has no sidebar to hide.
        if self.page_fullscreen {
            return;
        }
        // Ours: the sidebar comes back from the Library as it was.
        if self.library_open {
            return self.set_library(false, cx);
        }
        if self.dock(cx) {
            return;
        }
        let from = self.card_left();
        self.sidebar_hidden = !self.sidebar_hidden;
        self.sidebar_motion = Some((from, Instant::now()));
        // Arc hides the window's buttons with the sidebar.
        if self.sidebar_hidden {
            self.browser.show_window_buttons(false, cx);
        }
        cx.notify();
    }

    /// Docks the sidebar the reveal has out, in one frame (Arc), and returns whether it was out.
    fn dock(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.reveal_shown() {
            return false;
        }
        self.conceal_now(cx);
        self.sidebar_hidden = false;
        self.sidebar_motion = None;
        self.browser.show_window_buttons(true, cx);
        // check.sh waits for this line.
        eprintln!("browser: sidebar shown");
        cx.notify();
        true
    }

    /// Whether the reveal's panel is on screen, moving or not.
    pub fn reveal_shown(&self) -> bool {
        self.revealed || self.reveal_motion.is_some()
    }

    /// How far out the reveal's panel is while it moves: 0 off the window, 1 at rest over the page
    /// and past 1 as it overshoots. None once it's at rest.
    fn reveal_step(&self) -> Option<f32> {
        let (from, start) = self.reveal_motion?;
        let t = start.elapsed().as_secs_f32();
        if self.revealed {
            spring(t, REVEAL_IN_RESPONSE, REVEAL_IN_DAMPING).map(|eased| from + (1. - from) * eased)
        } else {
            spring(t, REVEAL_OUT_RESPONSE, REVEAL_OUT_DAMPING).map(|eased| from * (1. - eased))
        }
    }

    fn reveal_out(&self) -> f32 {
        self.reveal_step().unwrap_or(if self.revealed { 1. } else { 0. })
    }

    /// The system's accent colour changed, which the window's gradient follows.
    pub fn accent_color_changed(&mut self, cx: &mut Context<Self>) {
        self.accent_color = self.browser.accent_color();
        cx.notify();
    }

    /// The gradient down the window, at `y` from its top (0) to its bottom (1): the Space's
    /// colour, going to the same at HSL lightness + 0.25. While the sidebar pages between two
    /// Spaces, it cross-fades from one's to the other's as the pages move.
    fn gradient_color(&self, y: f32) -> Hsla {
        let color_of = |space| {
            // A Space without a colour has the system's accent colour, as in Dia.
            let space = self.spaces.iter().find(|each| each.id == space);
            let color = space.and_then(|space| space.color).unwrap_or(self.accent_color);
            let color = Hsla::from(rgb(color));
            let alpha = if color.s < NEUTRAL { NEUTRAL_GRADIENT_ALPHA } else { GRADIENT_ALPHA };
            Rgba::from(Hsla { l: (color.l + 0.25 * y).min(1.), ..color }.opacity(alpha))
        };
        let pages = self.pages();
        let (space, x) = pages[0];
        let from = color_of(space);
        let Some(&(beside, _)) = pages.get(1) else {
            return from.into();
        };
        let to = color_of(beside);
        let t = (x.abs() / self.width()).min(1.);
        let mix = |from: f32, to: f32| from + (to - from) * t;
        let (r, g, b) = (mix(from.r, to.r), mix(from.g, to.g), mix(from.b, to.b));
        Rgba { r, g, b, a: mix(from.a, to.a) }.into()
    }

    /// The gradient as a background, over `base` where the window's look has to be opaque.
    fn gradient(&self, base: Option<Hsla>) -> gpui::Background {
        let stop = |y| {
            let color = self.gradient_color(y);
            linear_color_stop(base.map_or(color, |base| base.blend(color)), y)
        };
        linear_gradient(180., stop(0.), stop(1.))
    }

    /// The window's look, opaque, halfway down: what covers the rows it passes over, and the find
    /// bar's colour.
    pub fn window_fill(&self) -> Hsla {
        Hsla::from(rgb(MATERIAL_STAND_IN)).blend(self.gradient_color(0.5))
    }

    /// The pointer is at `x` in the browser window: kept at its left edge it brings the hidden
    /// sidebar out over the page, and more than the hold zone past the panel's edge it sends it
    /// away.
    pub fn pointer_at(&mut self, x: f32, cx: &mut Context<Self>) {
        if !self.can_reveal() {
            return;
        }
        // The edge is the card's margin. Left of the window, the pointer has gone to another
        // window or display.
        if !(0. ..CARD_INSET).contains(&x) {
            self.reveal_wait = None;
            if x > REVEAL_INSET + self.width() + REVEAL_HOLD {
                self.set_revealed(false, cx);
            }
        } else if !self.revealed && self.reveal_wait.is_none() {
            // Arc waits a moment, so a pointer passing the edge on its way to another window or
            // display leaves the sidebar where it is.
            self.reveal_wait = Some(cx.spawn(async move |shell, cx| {
                cx.background_executor().timer(REVEAL_WAIT).await;
                shell
                    .update(cx, |shell, cx| {
                        shell.reveal_wait = None;
                        if shell.can_reveal() {
                            shell.set_revealed(true, cx);
                        }
                    })
                    .ok();
            }));
        }
    }

    /// Whether the sidebar is hidden and still, so the reveal can bring it out.
    fn can_reveal(&self) -> bool {
        let still = self.sidebar_motion.is_none() && self.library.is_none();
        self.sidebar_hidden && still && !self.page_fullscreen
    }

    /// Starts the reveal's panel moving out over the page, or away, from wherever it is.
    fn set_revealed(&mut self, revealed: bool, cx: &mut Context<Self>) {
        if revealed == self.revealed {
            return;
        }
        self.reveal_motion = Some((self.reveal_out(), Instant::now()));
        self.revealed = revealed;
        if revealed {
            self.browser.show_reveal(REVEAL_INSET + self.width() + REVEAL_OVERSHOOT, cx);
        }
        cx.notify();
    }

    /// Takes the reveal's panel off screen at once.
    fn conceal_now(&mut self, cx: &mut Context<Self>) {
        if self.reveal_shown() {
            self.revealed = false;
            self.reveal_motion = None;
            self.browser.hide_reveal(cx);
        }
    }

    /// The Spaces, by id and name, in the switcher's order.
    pub fn set_spaces(
        &mut self,
        spaces: Vec<(u64, SharedString, Option<u32>)>,
        cx: &mut Context<Self>,
    ) {
        // check.sh reads these lines.
        let ids: Vec<String> = spaces.iter().map(|(id, ..)| id.to_string()).collect();
        eprintln!("browser: spaces {}", ids.join(" "));
        let mut old = std::mem::take(&mut self.spaces);
        self.spaces = spaces
            .into_iter()
            .map(|(id, name, color)| {
                let Some(space) = old.iter_mut().find(|space| space.id == id) else {
                    eprintln!("browser: added space {id} {name}");
                    if let Some(color) = color {
                        eprintln!("browser: colored space {id} {color:06x}");
                    }
                    return Space {
                        id,
                        name,
                        color,
                        pinned: Vec::new(),
                        today: Vec::new(),
                        splits: Vec::new(),
                        folders: Folders::default(),
                        shown: 0,
                    };
                };
                if space.name != name {
                    eprintln!("browser: renamed space {id} {name}");
                }
                if space.color != color {
                    let color = color.map_or("none".to_string(), |color| format!("{color:06x}"));
                    eprintln!("browser: colored space {id} {color}");
                }
                Space {
                    id,
                    name,
                    color,
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
        cx.notify();
    }

    /// The Space the window shows. Which of its tabs shows follows in set_selected.
    pub fn set_space(&mut self, space: u64, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        match self.switching {
            Some((asked, start)) if asked == space => {
                self.switching = None;
                eprintln!(
                    "browser: space {space} in {:.1} ms",
                    start.elapsed().as_secs_f64() * 1000.
                );
            }
            _ => eprintln!("browser: space {space}"),
        }
        let before = std::mem::replace(&mut self.space, space);
        // A swipe that began in another Space is over, and its momentum with it.
        if self.swipe.as_ref().is_some_and(|swipe| swipe.from != space) {
            self.swipe = None;
            self.swiped = true;
        }
        // A Space shown some other way slides in from its side, pushing the one before out. One a
        // swipe settles on is moving already.
        let index = |id| self.spaces.iter().position(|other| other.id == id);
        let width = self.width();
        if !self.settle.as_ref().is_some_and(|settle| settle.space == space)
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
        self.log_tabs();
        if self.form().is_some() {
            self.focus_form(cx);
        }
        cx.notify();
    }

    pub fn set_tabs(
        &mut self,
        space: u64,
        pinned: Vec<u64>,
        today: Vec<u64>,
        splits: Vec<Vec<u64>>,
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
                eprintln!("browser: renamed folder {folder} {name}");
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

    pub fn set_favorites(&mut self, favorites: Vec<u64>, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        let ids: Vec<String> = favorites.iter().map(u64::to_string).collect();
        eprintln!("browser: favorites {}", ids.join(" "));
        let tiles = self.drawn();
        self.favorites = favorites;
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

    /// What the shell knew of a closed tab goes with it. The core reports a tab in its new place
    /// before it leaves the old, so one moving between a Space and the favorites stays.
    fn forget_closed(&mut self) {
        let (spaces, favorites) = (&self.spaces, &self.favorites);
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
        eprintln!("browser: pinned {}", ids(self.pinned()));
        eprintln!("browser: today {}", ids(self.today()));
        let splits = self.shown_space().map_or_else(Vec::new, |space| {
            space.splits.iter().map(|split| ids(split).replace(' ', "+")).collect()
        });
        eprintln!("browser: splits {}", splits.join(" "));
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
            Some((asked, start)) if asked == tab => eprintln!(
                "browser: selected {tab} in {:.1} ms",
                start.elapsed().as_secs_f64() * 1000.
            ),
            _ => eprintln!("browser: selected {tab}"),
        }
        self.selected = tab;
        self.show_in_space(tab);
        if !self.walking {
            self.shown(tab);
        }
        // check.sh reads this line.
        let panes = self.panes(tab).iter().map(u64::to_string).collect::<Vec<_>>().join(" ");
        eprintln!("browser: panes {panes}");
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
            eprintln!("browser: new tab plate {}", new_tab as u8);
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

    /// ⌃ went up, which ends a ⌃Tab walk on the tab it shows.
    pub fn control_released(&mut self, _: &mut Context<Self>) {
        self.walking = false;
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
        eprintln!(
            "browser: page {tab} back {} forward {} loading {} {}",
            flag(page.can_go_back),
            flag(page.can_go_forward),
            flag(page.loading),
            page.url
        );
        let entry = self.tabs.entry(tab).or_default();
        if page.unloaded != entry.page.unloaded {
            eprintln!("browser: page {tab} unloaded {}", flag(page.unloaded));
        }
        if page.away_from_pinned_url != entry.page.away_from_pinned_url {
            eprintln!("browser: page {tab} away {}", flag(page.away_from_pinned_url));
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

    /// Runs a browser command and logs it. `from` says which side had the key: GPUI's bindings
    /// ("ui"), the browser handing it back ("page"), or the sidebar.
    pub fn run(&mut self, command: Command, from: &str, cx: &mut Context<Self>) {
        // check.sh waits for these lines.
        eprintln!("browser: command {} via {from}", command.name());
        let rows = self.sidebar_rows();
        let position = rows.iter().position(|&row| row == self.row_of(self.selected));
        let count = rows.len();
        let space = self.spaces.iter().position(|space| space.id == self.space);
        // Next and previous wrap around for tabs, as in Chrome, and stop at the ends for Spaces, as
        // in Arc.
        // Escape puts back a dragged row before it stops the page (docs/ux-spec.md › Tab list).
        if matches!(command, Command::Stop) && self.cancel_drag(cx) {
            return;
        }
        // The command bar opens over the page, and the reveal's panel makes way for it.
        if matches!(command, Command::NewTab | Command::EditUrl | Command::AddSplit) {
            self.set_revealed(false, cx);
        }
        let target = match command {
            Command::NewTab => {
                self.set_new_tab(true, cx);
                return self.browser.command_bar(None, None, cx);
            }
            Command::EditUrl => {
                self.set_new_tab(false, cx);
                let editing = (self.selected != 0).then(|| (self.selected, self.url(self.selected)));
                return self.browser.command_bar(editing, self.url_field_bounds(), cx);
            }
            Command::CloseTab => {
                if self.selected != 0 {
                    self.browser.close(self.selected, false, cx);
                }
                return;
            }
            Command::TogglePin => {
                if self.selected != 0 {
                    let pinned = self.pinned().contains(&self.selected);
                    self.browser.set_pinned(self.selected, !pinned, cx);
                }
                return;
            }
            Command::NewFolder => return self.new_folder(cx),
            Command::CollapseFolders => return self.collapse_folders(cx),
            Command::BackToPinnedUrl => return self.browser.back_to_pinned_url(self.selected, cx),
            Command::Clear => return self.clear(cx),
            Command::UndoClear => {
                self.browser.undo_clear(cx);
                return;
            }
            Command::CopyUrl => return self.copy_url(false, cx),
            Command::CopyMarkdownLink => return self.copy_url(true, cx),
            Command::ReopenClosedTab => return self.browser.reopen_closed_tab(cx),
            Command::Back => return self.browser.go_back(self.selected, cx),
            Command::Forward => return self.browser.go_forward(self.selected, cx),
            Command::Reload => return self.browser.reload(self.selected, false, cx),
            Command::ForceReload => return self.browser.reload(self.selected, true, cx),
            Command::Stop => return self.browser.stop(self.selected, cx),
            Command::ActualSize => return self.browser.zoom(self.selected, 0, cx),
            Command::ZoomIn => return self.browser.zoom(self.selected, 1, cx),
            Command::ZoomOut => return self.browser.zoom(self.selected, -1, cx),
            Command::ViewSource => return self.browser.view_source(self.selected, cx),
            Command::SavePage => return self.browser.save_page(self.selected, cx),
            Command::Print => return self.browser.print(self.selected, cx),
            Command::DevTools => return self.browser.devtools(self.selected, 0, cx),
            Command::JavaScriptConsole => return self.browser.devtools(self.selected, 1, cx),
            Command::InspectElements => return self.browser.devtools(self.selected, 2, cx),
            Command::Quit => return self.browser.quit(cx),
            Command::AddSplit => return self.add_pane(cx),
            Command::CloseSplitPane | Command::SeparatePane => {
                if self.panes(self.selected).len() > 1 {
                    match command {
                        Command::CloseSplitPane => self.browser.close(self.selected, false, cx),
                        _ => self.browser.separate(self.selected, cx),
                    }
                }
                return;
            }
            Command::NextPane => return self.focus_pane(true, cx),
            Command::PreviousPane => return self.focus_pane(false, cx),
            Command::ToggleSidebar => return self.toggle_sidebar(cx),
            Command::ViewLibrary => return self.set_library(!self.library_open, cx),
            Command::ViewArchive => return self.view_section(Section::Archive, cx),
            Command::ViewDownloads => return self.view_section(Section::Downloads, cx),
            Command::ClearArchive => return self.clear_archive(cx),
            Command::NewSpace => return self.new_space(cx),
            Command::SelectSpace(index) => return self.show_space_at(Some(index), cx),
            Command::NextSpace => return self.show_space_at(space.map(|index| index + 1), cx),
            Command::PreviousSpace => {
                return self.show_space_at(space.and_then(|index| index.checked_sub(1)), cx);
            }
            Command::Find => return self.browser.find(self.selected, cx),
            Command::FindNext => return self.browser.find_next(self.selected, true, cx),
            Command::FindPrevious => return self.browser.find_next(self.selected, false, cx),
            Command::RecentTab => return self.walk(rows, true, cx),
            Command::RecentTabBack => return self.walk(rows, false, cx),
            Command::NextTab => position.map(|index| (index + 1) % count),
            Command::PreviousTab => position.map(|index| (index + count - 1) % count),
            Command::SelectTab(index) => (index < count).then_some(index),
            Command::SelectLastTab => count.checked_sub(1),
        };
        if let Some(index) = target {
            self.select(self.row_tab(rows[index]), cx);
        }
    }

    /// ⌃+: opens the command bar over a new pane right of the pages shown, which make room for it
    /// in the same frame (Arc). What it opens shows there, in a split view with the tab shown.
    fn add_pane(&mut self, cx: &mut Context<Self>) {
        let with = self.selected;
        let count = self.panes(with).len();
        if with == 0 || count >= MAX_PANES {
            return;
        }
        self.adding_pane = Some(with);
        self.lay_out_page(cx);
        let card = page_frame(self.viewport, self.card_left(), self.card_push());
        let pane = pane_frames(card, count + 1)[count];
        self.browser.split_bar(with, pane, card, cx);
        cx.notify();
    }

    /// Focuses the split view's page right of the focused one, or left of it, round the ends.
    fn focus_pane(&mut self, forward: bool, cx: &mut Context<Self>) {
        let panes = self.panes(self.selected);
        let count = panes.len();
        let Some(at) = panes.iter().position(|&tab| tab == self.selected).filter(|_| count > 1)
        else {
            return;
        };
        let next = if forward { (at + 1) % count } else { (at + count - 1) % count };
        self.select(panes[next], cx);
    }

    /// ⌃Tab, as in Arc: shows the Space's tab shown before this one at once, with no switcher.
    /// While ⌃ is held, each ⌃Tab goes on to the one shown before that, and ⌃⇧Tab back. A split's
    /// row counts once.
    fn walk(&mut self, rows: Vec<u64>, forward: bool, cx: &mut Context<Self>) {
        // Then the rows not shown yet, in the sidebar's order.
        let mut order: Vec<u64> = Vec::new();
        for row in self.recent.iter().map(|&tab| self.row_of(tab)).chain(rows.iter().copied()) {
            if rows.contains(&row) && !order.contains(&row) {
                order.push(row);
            }
        }
        let count = order.len();
        let next = match order.iter().position(|&row| row == self.row_of(self.selected)) {
            Some(at) if forward => (at + 1) % count,
            Some(at) => (at + count - 1) % count,
            // With no tab shown, the tab shown last (Arc).
            None if forward && count > 0 => 0,
            None => match count.checked_sub(1) {
                Some(last) => last,
                None => return,
            },
        };
        let tab = self.row_tab(order[next]);
        if tab == self.selected {
            return;
        }
        if !self.walking {
            self.walking = true;
            self.browser.watch_control(cx);
        }
        self.select(tab, cx);
    }

    /// Clears the Space shown, but the tab shown. Its rows go in one frame, with no toast (Arc).
    fn clear(&mut self, cx: &mut Context<Self>) {
        // A split view shown stays whole.
        let shown = self.panes(self.selected);
        self.clearing = self.today().iter().copied().filter(|tab| !shown.contains(tab)).collect();
        if self.clearing.is_empty() {
            return;
        }
        self.browser.clear(cx);
        // check.sh reads this line.
        eprintln!("browser: cleared {}", self.clearing.len());
    }

    /// Runs `command` for a key the page didn't keep, and returns whether it was used. With no
    /// Clear to undo, ⌘Z goes back, for the Edit menu to give the page.
    pub fn run_for_page(&mut self, command: Command, cx: &mut Context<Self>) -> bool {
        if command != Command::UndoClear {
            self.run(command, "page", cx);
            return true;
        }
        let undone = self.browser.undo_clear(cx);
        if undone > 0 {
            // check.sh waits for this line.
            eprintln!("browser: command undo-clear via page");
        }
        undone > 0
    }

    /// Copies the selected tab's URL, or a Markdown link to it, and says so in the toast.
    fn copy_url(&mut self, markdown: bool, cx: &mut Context<Self>) {
        let url = self.url(self.selected);
        if url.is_empty() {
            return;
        }
        let (text, toast) = match markdown {
            false => (url.to_string(), "Copied URL"),
            true => {
                let title = markdown_text(&self.title(self.selected));
                (format!("[{title}]({url})"), "Copied URL as Markdown")
            }
        };
        self.browser.copy(&text, cx);
        // check.sh reads this line.
        eprintln!("browser: copied {text}");
        self.show_toast(toast.into(), COPIED_TIME, cx);
    }

    /// Shows `text` in the toast for `time`, in place of any toast showing.
    fn show_toast(&mut self, text: SharedString, time: Duration, cx: &mut Context<Self>) {
        let hide = cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(time).await;
            shell.update(cx, |shell, cx| shell.hide_toast(cx)).ok();
        });
        self.toast = Some(Toast { text, _hide: hide });
        cx.notify();
    }

    fn hide_toast(&mut self, cx: &mut Context<Self>) {
        if self.toast.take().is_some() {
            eprintln!("browser: toast hidden");
            cx.notify();
        }
    }

    /// Makes a Space and shows it with the form for naming it, or shows the form already open.
    fn new_space(&mut self, cx: &mut Context<Self>) {
        let space = match &self.form {
            Some(form) => form.space,
            None => {
                // Arc's name for a Space the user doesn't name.
                let name = format!("Space {}", self.spaces.len() + 1);
                let space = self.browser.create_space(&name, cx);
                let name = cx.new(|cx| {
                    TextInput::new("Space name", cx).placeholder_color(rgba(FORM_PLACEHOLDER))
                });
                self.form = Some(SpaceForm {
                    space,
                    from: self.space,
                    name,
                });
                space
            }
        };
        if space == self.space {
            self.focus_form(cx);
        } else {
            self.show_space(space, cx);
        }
    }

    /// The form, if the window shows its Space.
    fn form(&self) -> Option<&SpaceForm> {
        self.form.as_ref().filter(|form| form.space == self.space)
    }

    fn focus_form(&mut self, cx: &mut Context<Self>) {
        // The panel never takes the keyboard, and goes when the pointer leaves it.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_form = true;
        cx.notify();
    }

    /// Names the form's Space, if it has a name, and closes the form.
    fn create_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form() else {
            return;
        };
        let name = form.name.read(cx).text().trim().to_string();
        if name.is_empty() {
            return;
        }
        // check.sh waits for these lines.
        eprintln!("browser: create space {} {name}", form.space);
        self.browser.rename_space(form.space, &name, cx);
        self.form = None;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// Shows the Space the form was opened from and deletes the form's.
    fn cancel_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form.take_if(|form| form.space == self.space) else {
            return;
        };
        eprintln!("browser: cancel space {}", form.space);
        self.show_space(form.from, cx);
        self.browser.delete_space(form.space, cx);
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// The menu a right-click on a Space's name or dot shows (Arc), with its colour (ours).
    fn space_menu(&mut self, space: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        type Action = Box<dyn FnOnce(&mut Shell, &mut Context<Shell>)>;
        let color = self.spaces.iter().find(|each| each.id == space).and_then(|each| each.color);
        let mut items: Vec<(MenuEntry, Action)> = vec![
            (
                MenuEntry::new("Rename Space", true),
                Box::new(move |shell, cx| shell.rename_space(space, cx)),
            ),
            (MenuEntry::new("Color", true), Box::new(|_, _| {})),
            (
                MenuEntry::new("System Accent", true)
                    .under(1)
                    .checked(color.is_none())
                    .swatch(self.accent_color),
                Box::new(move |shell, cx| shell.browser.set_space_color(space, None, cx)),
            ),
        ];
        for (name, each) in SPACE_COLORS {
            items.push((
                MenuEntry::new(name, true).under(1).checked(color == Some(each)).swatch(each),
                Box::new(move |shell, cx| shell.browser.set_space_color(space, Some(each), cx)),
            ));
        }
        items.push((
            MenuEntry::new("Delete Space", self.spaces.len() > 1),
            Box::new(move |shell, cx| shell.delete_space(space, cx)),
        ));
        let entries = items.iter().map(|(entry, _)| entry.clone()).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            self.sidebar_hidden,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| {
                        if let Some((_, action)) = chosen.and_then(|at| items.into_iter().nth(at)) {
                            action(shell, cx)
                        }
                    })
                    .ok();
            }),
            cx,
        );
    }

    /// The menu a right-click on a tab's row shows: Chrome's tab actions that Arc's sidebar has,
    /// with their shortcuts.
    fn tab_menu(&mut self, tab: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        let row = Row::Tab(self.row_of(tab));
        let items = if self.favorites.contains(&tab) {
            self.favorite_menu(tab)
        } else if self.is_folder(tab) {
            self.folder_menu(tab)
        } else {
            self.space_tab_menu(tab)
        };
        let entries = items.iter().map(|(entry, ..)| entry.clone()).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            self.sidebar_hidden,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| {
                        // The fill fades as the pointer leaving the row fades it.
                        shell.menu_row = None;
                        if shell.hovered != Some(row) {
                            shell.unhovered.push((row, Instant::now()));
                        }
                        cx.notify();
                        let item = chosen.and_then(|at| items.into_iter().nth(at));
                        if let Some((_, name, action)) = item {
                            eprintln!("browser: command {name} via tab menu");
                            action(shell, cx)
                        }
                    })
                    .ok();
            }),
            cx,
        );
        self.menu_row = Some(self.row_of(tab));
        cx.notify();
    }

    /// A favorite's menu.
    fn favorite_menu(&self, tab: u64) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let page = self.page(tab);
        vec![
            (
                MenuEntry::new("Back to Favorite URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ),
            (
                MenuEntry::new("Duplicate Tab", true),
                "duplicate-tab",
                Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
            ),
            (
                MenuEntry::new("Remove from Favorites", true),
                "remove-from-favorites",
                // To the top of Today.
                Box::new(move |shell, cx| shell.browser.move_tab(tab, false, 0, 0, cx)),
            ),
            (MenuEntry::line(), "", Box::new(|_, _| {})),
            // One closed already has nothing to close.
            (
                MenuEntry::new("Close Tab", !page.unloaded).shortcut(Command::CloseTab),
                "close-tab",
                Box::new(move |shell, cx| shell.browser.close(tab, false, cx)),
            ),
        ]
    }

    /// A folder's menu.
    fn folder_menu(&self, folder: u64) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        vec![
            (
                MenuEntry::new("Rename Folder", true),
                "rename-folder",
                Box::new(move |shell, cx| shell.rename_folder(folder, cx)),
            ),
            (MenuEntry::line(), "", Box::new(|_, _| {})),
            (
                MenuEntry::new("Delete Folder", true),
                "delete-folder",
                Box::new(move |shell, cx| shell.browser.delete_folder(folder, cx)),
            ),
        ]
    }

    /// The menu of a tab in the Space.
    fn space_tab_menu(&self, tab: u64) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let pinned = self.pinned().contains(&tab);
        let page = self.page(tab);
        let panes = self.panes(tab);
        let last = panes[panes.len() - 1];
        let below = self.today().iter().skip_while(|&&id| id != last).count() > 1;
        let others: Vec<(u64, SharedString)> = self
            .spaces
            .iter()
            .filter(|space| space.id != self.space)
            .map(|space| (space.id, space.name.clone()))
            .collect();
        let pin = MenuEntry::new(if pinned { "Unpin Tab" } else { "Pin Tab" }, true);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![
            (
                pin.shortcut(Command::TogglePin),
                "toggle-pin",
                Box::new(move |shell, cx| {
                    let pinned = shell.pinned().contains(&tab);
                    shell.browser.set_pinned(tab, !pinned, cx)
                }),
            ),
            (
                MenuEntry::new("Add to Favorites", true),
                "add-to-favorites",
                Box::new(move |shell, cx| shell.browser.move_to_favorites(tab, usize::MAX, cx)),
            ),
        ];
        if pinned {
            items.push((
                MenuEntry::new("Back to Pinned URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ));
        }
        items.push((
            MenuEntry::new("Rename Tab", true),
            "rename-tab",
            Box::new(move |shell, cx| shell.rename_tab(tab, cx)),
        ));
        items.push((
            MenuEntry::new("Duplicate Tab", true),
            "duplicate-tab",
            Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
        ));
        if panes.len() > 1 {
            items.push((
                MenuEntry::new("Separate from Split View", true),
                "separate-pane",
                Box::new(move |shell, cx| shell.browser.separate(tab, cx)),
            ));
        }
        // Each other Space, in the switcher's order.
        let move_to = items.len();
        items.push((
            MenuEntry::new("Move to Space", !others.is_empty()),
            "",
            Box::new(|_, _| {}),
        ));
        for (space, name) in others {
            items.push((
                MenuEntry::new(name, true).under(move_to),
                "move-to-space",
                Box::new(move |shell, cx| shell.browser.move_to_space(tab, space, cx)),
            ));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // A pinned tab closed already has nothing to close.
        let closable = !(page.unloaded && pinned);
        items.push((
            MenuEntry::new("Close Tab", closable).shortcut(Command::CloseTab),
            "close-tab",
            Box::new(move |shell, cx| shell.browser.close(tab, false, cx)),
        ));
        if !pinned {
            items.push((
                MenuEntry::new("Close Tabs Below", below),
                "close-tabs-below",
                Box::new(move |shell, cx| shell.browser.close_below(tab, cx)),
            ));
        }
        items
    }

    /// Shows the Space with its name in a field, all selected, to type a new one over.
    fn rename_space(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(name) = self.spaces.iter().find(|each| each.id == space).map(|each| &each.name)
        else {
            return;
        };
        let name = name.clone();
        self.show_space(space, cx);
        // The form names a Space it made.
        if self.form.as_ref().is_some_and(|form| form.space == space) {
            return;
        }
        eprintln!("browser: rename space {space}");
        let name = cx.new(|cx| {
            TextInput::new("Space name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(name)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Space(space),
            name,
            _blur: None,
        });
        // As for the form.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows the tab's title in a field, all selected, to type a name over.
    fn rename_tab(&mut self, tab: u64, cx: &mut Context<Self>) {
        eprintln!("browser: rename tab {tab}");
        let title = self.title(tab);
        let name = cx.new(|cx| {
            TextInput::new("Tab name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(title)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Tab(tab),
            name,
            _blur: None,
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows the folder's name in a field, all selected, to type a new one over.
    fn rename_folder(&mut self, folder: u64, cx: &mut Context<Self>) {
        let name = self.folder_name(folder);
        self.name_folder(folder, name, cx);
    }

    fn name_folder(&mut self, folder: u64, name: SharedString, cx: &mut Context<Self>) {
        let name = cx.new(|cx| {
            TextInput::new("Folder name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(name)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Folder(folder),
            name,
            _blur: None,
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Makes a folder at the bottom of the pinned tabs of the Space shown, with its name in a field
    /// to type over.
    fn new_folder(&mut self, cx: &mut Context<Self>) {
        let name = SharedString::from(NEW_FOLDER);
        let folder = self.browser.create_folder(self.space, &name, 0, usize::MAX, cx);
        if folder != 0 {
            self.name_folder(folder, name, cx);
        }
    }

    /// The folder's name in the Space shown.
    fn folder_name(&self, folder: u64) -> SharedString {
        let space = self.shown_space();
        let folder = space.and_then(|space| space.folders.folders.get(&folder));
        folder.map(|(name, _)| name.clone()).unwrap_or_default()
    }

    /// Closes every folder of the Space shown, all on one fold.
    fn collapse_folders(&mut self, cx: &mut Context<Self>) {
        let Some(space) = self.shown_space() else {
            return;
        };
        let folders = space.folders.folders.keys().copied();
        let open: Vec<u64> = folders.filter(|&folder| space.is_open(folder)).collect();
        // The core reports each, and the first report has them all closed.
        self.folding = !open.is_empty();
        for folder in open {
            self.browser.set_folder_open(folder, false, cx);
        }
    }

    /// Whether `command` would do something now, for the command bar to offer it.
    fn offers(&self, command: Command) -> bool {
        let page = self.page(self.selected);
        let space = self.spaces.iter().position(|space| space.id == self.space);
        match command {
            Command::Back => page.can_go_back,
            Command::Forward => page.can_go_forward,
            Command::Stop => page.loading,
            Command::BackToPinnedUrl => page.away_from_pinned_url,
            Command::CloseSplitPane | Command::SeparatePane => self.panes(self.selected).len() > 1,
            Command::CollapseFolders => self.shown_space().is_some_and(|space| {
                space.folders.folders.keys().any(|&folder| space.is_open(folder))
            }),
            Command::Clear => !self.today().is_empty(),
            Command::NextSpace => space.is_some_and(|index| index + 1 < self.spaces.len()),
            Command::PreviousSpace => space.is_some_and(|index| index > 0),
            Command::NextTab | Command::PreviousTab => self.sidebar_rows().len() > 1,
            Command::NewFolder
            | Command::ReopenClosedTab
            | Command::ToggleSidebar
            | Command::NewSpace
            | Command::ViewLibrary
            | Command::ViewArchive
            | Command::ClearArchive
            | Command::ViewDownloads => true,
            _ => self.selected != 0,
        }
    }

    /// Opens or closes the folder, unless the click ended a drag or renames it.
    fn click_folder(&mut self, folder: u64, cx: &mut Context<Self>) {
        let renaming = self.renaming.as_ref().map(|renaming| renaming.naming);
        if self.dragged || self.dragging().is_some() || renaming == Some(Naming::Folder(folder)) {
            return;
        }
        let open = self.shown_space().is_some_and(|space| space.is_open(folder));
        eprintln!("browser: command toggle-folder via sidebar");
        self.folding = true;
        self.browser.set_folder_open(folder, !open, cx);
    }

    /// Renames the Space, the tab or the folder to what the field holds, and closes the field. A
    /// Space or a folder keeps its name rather than none, and a tab named nothing shows its page's
    /// title again.
    fn keep_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let name = renaming.name.read(cx).text().trim().to_string();
        match renaming.naming {
            Naming::Space(space) if !name.is_empty() => self.browser.rename_space(space, &name, cx),
            Naming::Space(_) => {}
            // Left as it was, the title stays the page's, which changes with it.
            Naming::Tab(tab) if *name == *self.title(tab) => {}
            Naming::Tab(tab) => self.browser.rename_tab(tab, &name, cx),
            Naming::Folder(folder) if !name.is_empty() => {
                self.browser.rename_folder(folder, &name, cx)
            }
            Naming::Folder(_) => {}
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn cancel_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.renaming.take().is_some() {
            eprintln!("browser: rename cancelled");
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// Deletes the Space, asking first if it has tabs, which close with it.
    fn delete_space(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(found) = self.spaces.iter().find(|each| each.id == space) else {
            return;
        };
        let tabs = found.pinned.len() + found.today.len();
        if tabs == 0 {
            self.browser.delete_space(space, cx);
            return;
        }
        let browser = self.browser.clone();
        self.browser.confirm(
            format!("Delete “{}”?", found.name),
            if tabs == 1 {
                "Its tab closes too.".into()
            } else {
                format!("Its {tabs} tabs close too.")
            },
            "Delete",
            Box::new(move |cx| browser.delete_space(space, cx)),
            cx,
        );
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
        let below_rows = at.x < px(self.card_left()) && at.y >= px(self.list_top());
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
            Some(tab) if !tab.page.url.is_empty() => host(&tab.page.url).into(),
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

    /// Starts the list's rows from where they were in `before` to where they are now: rows still
    /// there slide, a new tab's row is in its place at once with its plate fading in, several new
    /// ones fade in whole, and a closed one fades where it was (docs/ux-spec.md › Tab rows).
    fn move_rows(&mut self, before: HashMap<Slot, f32>, curve: Curve) {
        let now = Instant::now();
        let after = self.layout();
        let slides = after
            .iter()
            .filter_map(|(&slot, &top)| {
                let from = before.get(&slot)? + self.slide(slot) - top;
                (from != 0.).then_some((slot, (from, now, curve)))
            })
            .collect();
        let mut opened = after
            .iter()
            .filter_map(|(&slot, &top)| match slot {
                Slot::Tab(tab) if !before.contains_key(&slot) => Some((top, tab)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !opened.is_empty() {
            opened.sort_by(|(a, _), (b, _)| a.total_cmp(b));
            let tabs: Vec<u64> = opened.into_iter().map(|(_, tab)| tab).collect();
            // Rows a folder shows as it opens come whole, as several new tabs do.
            let whole = tabs.len() > 1 || tabs.iter().any(|&tab| self.depth(tab) > 0);
            self.opened = Some(Opened { tabs, whole, start: now });
        }
        for (&slot, &top) in &before {
            let Slot::Tab(tab) = slot else { continue };
            // One gone to the favorites is a tile now.
            if !after.contains_key(&slot) && !self.favorites.contains(&tab) {
                let space = self.shown_space();
                let folder = space.filter(|space| space.is_folder(tab));
                self.closing.push(Closing {
                    title: self.label(tab),
                    favicon: self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()),
                    folder: folder.map(|space| space.is_open(tab)),
                    indent: self.depth(tab) as f32 * FOLDER_INDENT,
                    top: top + self.slide(slot),
                    start: now,
                });
            }
        }
        self.slides = slides;
    }

    /// How far a row is drawn from its place: where the pointer has a dragged one, and others while
    /// they slide there or make room for it.
    fn slide(&self, slot: Slot) -> f32 {
        let drag = self.drag.as_ref().filter(|drag| drag.moving);
        if let Some(drag) = drag.filter(|drag| slot == Slot::Tab(drag.tab)) {
            return self.drag_at(drag) - drag.top;
        }
        let room = drag.and_then(|drag| drag.room.get(&slot)).copied().unwrap_or(0.);
        room + self.slides.get(&slot).map_or(0., |&(from, start, curve)| {
            curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| from * (1. - done))
        })
    }

    /// Where a dragged tab's row is drawn in the list: with the pointer, held between the list's
    /// first and last places.
    fn drag_at(&self, drag: &Drag) -> f32 {
        let (first, last) = drag.span;
        (self.list_y(drag.pointer.y) - drag.grab.0).clamp(first, last)
    }

    /// Where `y`, down from the sidebar's top, is in the list, scrolled as it is.
    fn list_y(&self, y: f32) -> f32 {
        y - self.drawn_list_top() - f32::from(self.list_scroll.offset().y)
    }

    /// The row the pointer drags, once it has come away.
    fn dragging(&self) -> Option<Slot> {
        self.drag.as_ref().filter(|drag| drag.moving).map(|drag| Slot::Tab(drag.tab))
    }

    /// The pointer pressed `tab`'s row at `position`, which may start a drag.
    fn press_row(&mut self, tab: u64, position: Point<Pixels>) {
        let pinned = self.pinned().contains(&tab);
        let (pinned_rows, today_rows) = self.rows();
        let rows = if pinned { pinned_rows } else { today_rows };
        let Some(index) = rows.iter().position(|&other| other == tab) else {
            return;
        };
        let top = self.layout()[&Slot::Tab(tab)];
        let pointer = in_sidebar(position, self.sidebar_origin.get());
        let grab = (self.list_y(pointer.y) - top, point(0., 0.));
        self.press(tab, position, Place::List(pinned, index), top, grab);
    }

    /// The pointer pressed `tab`'s tile at `position`, which may start a drag.
    fn press_tile(&mut self, tab: u64, position: Point<Pixels>) {
        let Some(index) = self.favorites.iter().position(|&other| other == tab) else {
            return;
        };
        let Some(&[left, top, width]) = self.drawn_tiles().get(&tab) else {
            return;
        };
        let pointer = in_sidebar(position, self.sidebar_origin.get());
        let centre = point(left + width / 2., LIST_TOP + top + TILE_HEIGHT / 2.);
        // Its row, over the list, is centred on the pointer.
        let grab = (ROW_HEIGHT / 2., pointer - centre);
        self.press(tab, position, Place::Favorite(index), 0., grab);
    }

    fn press(
        &mut self,
        tab: u64,
        position: Point<Pixels>,
        from: Place,
        top: f32,
        grab: (f32, Point<f32>),
    ) {
        if self.drag.as_ref().is_some_and(|drag| drag.dropped) {
            return;
        }
        let origin = self.sidebar_origin.get();
        self.dragged = false;
        self.drag = Some(Drag {
            tab,
            press: position,
            origin,
            pointer: in_sidebar(position, origin),
            grab,
            top,
            moving: false,
            span: (top, top),
            from,
            to: from,
            from_depth: self.depth(tab),
            depth: self.depth(tab),
            room: HashMap::new(),
            divider: true,
            dropped: false,
        });
    }

    /// The pointer pressing a row or a tile moved to `position`. Past DRAG_START the tab comes away
    /// and follows it. Over the favorites it's a tile, and the tiles make room for it at the place
    /// nearest it. Over the list it's a row, which goes up and down, and the rows make room for it
    /// at the place, in the pinned tabs or Today, nearest where it's drawn. Where that's at the end
    /// of folders or after one, the pointer moved right takes it into them, a row in for each
    /// FOLDER_INDENT. A split view's row stays in the list, and a folder's in the pinned tabs.
    fn drag_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_ref().filter(|drag| !drag.dropped) else {
            return;
        };
        let moved = position - drag.press;
        if !drag.moving && moved.x.abs().max(moved.y.abs()) < px(DRAG_START) {
            return;
        }
        let (tab, to, grab, span) = (drag.tab, drag.to, drag.grab, drag.span);
        let pointer = in_sidebar(position, drag.origin);
        // Rows in, from where it was, by how far the pointer has gone right.
        let moved_in = (pointer.x - in_sidebar(drag.press, drag.origin).x) / FOLDER_INDENT;
        let wants = (drag.from_depth as f32 + moved_in).round().max(0.) as usize;
        let others = |rows: Vec<u64>| rows.into_iter().filter(|&other| other != tab).collect();
        let (pinned, today) = self.rows();
        let (pinned, today): (Vec<u64>, Vec<u64>) = (others(pinned), others(today));
        let favorites = self.favorites.iter().filter(|&&other| other != tab).count();
        let folder = self.is_folder(tab);
        let over_favorites = !folder
            && self.panes(tab).len() == 1
            && pointer.y < self.list_top_with(favorites + 1);
        let mut depth = 0;
        let (place, layout, span) = if over_favorites {
            let centre = pointer - grab.1;
            let places = self.tile_places(favorites + 1);
            let distance = |index: &usize| {
                let [left, top, width] = places[*index];
                let x = left + width / 2. - centre.x;
                let y = LIST_TOP + top + TILE_HEIGHT / 2. - centre.y;
                x * x + y * y
            };
            let index = (0..places.len()).min_by(|a, b| distance(a).total_cmp(&distance(b)));
            (Place::Favorite(index.unwrap_or(0)), slots(&pinned, &today), span)
        } else {
            let fits = |&gap: &usize| self.gap_depths(&pinned, gap, tab).is_some();
            let places = (0..=pinned.len()).filter(fits).map(|index| (true, index));
            let today_places = (0..=today.len()).filter(|_| !folder);
            let places = places.chain(today_places.map(|index| (false, index)));
            let layouts: Vec<_> = places
                .map(|(in_pinned, index)| {
                    let (mut pinned, mut today) = (pinned.clone(), today.clone());
                    let tabs = if in_pinned { &mut pinned } else { &mut today };
                    tabs.insert(index, tab);
                    let layout = slots(&pinned, &today);
                    ((in_pinned, index), layout[&Slot::Tab(tab)], layout)
                })
                .collect();
            let (first, last) = (layouts[0].1, layouts[layouts.len() - 1].1);
            let span = (first.min(last), first.max(last));
            let at = (self.list_y(pointer.y) - grab.0).clamp(span.0, span.1);
            let Some(((in_pinned, index), _, layout)) =
                layouts.into_iter().min_by(|a, b| (a.1 - at).abs().total_cmp(&(b.1 - at).abs()))
            else {
                return;
            };
            if in_pinned && let Some((low, high)) = self.gap_depths(&pinned, index, tab) {
                depth = wants.clamp(low, high);
            }
            (Place::List(in_pinned, index), layout, span)
        };
        if let Some(drag) = &mut self.drag {
            drag.moving = true;
            drag.pointer = pointer;
            drag.span = span;
            drag.depth = depth;
        }
        if place != to {
            // The rows and tiles that make room, or no longer need to, start from where they're
            // drawn.
            let now = Instant::now();
            let room: HashMap<Slot, f32> = self
                .layout()
                .into_iter()
                .filter(|&(slot, _)| slot != Slot::Tab(tab))
                .filter_map(|(slot, top)| {
                    let moved = layout.get(&slot)? - top;
                    (moved != 0.).then_some((slot, moved))
                })
                .collect();
            let old = self.drag.as_ref().map(|drag| drag.room.clone()).unwrap_or_default();
            for slot in old.keys().chain(room.keys()).copied().collect::<Vec<_>>() {
                let from = self.slide(slot) - room.get(&slot).copied().unwrap_or(0.);
                self.slides.insert(slot, (from, now, Curve::Reorder));
            }
            let tiles = self.drawn();
            if let Some(drag) = &mut self.drag {
                drag.to = place;
                drag.room = room;
                drag.divider = layout.contains_key(&Slot::Divider);
            }
            self.move_tiles(tiles, Curve::Reorder);
        }
        cx.notify();
    }

    /// The pointer let go of a pressed row or tile. A dragged tab goes where it would, and the rows
    /// and tiles settle once the browser has moved it.
    fn drop_row(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut().filter(|drag| !drag.dropped) else {
            return;
        };
        if !drag.moving {
            self.drag = None;
            return;
        }
        self.dragged = true;
        let (tab, to, drag_depth) = (drag.tab, drag.to, drag.depth);
        if drag.to == drag.from && drag.depth == drag.from_depth {
            self.settle_drag();
        } else {
            drag.dropped = true;
            match to {
                Place::Favorite(index) => {
                    // check.sh waits for this line.
                    eprintln!("browser: dragged {tab} to favorites {index}");
                    self.browser.move_to_favorites(tab, index, cx);
                }
                Place::List(true, gap) => {
                    let (rows, _) = self.rows();
                    let rows: Vec<u64> = rows.into_iter().filter(|&row| row != tab).collect();
                    let (folder, index) = self.pinned_place(&rows, gap, drag_depth, tab);
                    // check.sh waits for these lines.
                    match folder {
                        0 => eprintln!("browser: dragged {tab} to pinned {gap}"),
                        _ => eprintln!("browser: dragged {tab} to folder {folder} {index}"),
                    }
                    match self.is_folder(tab) {
                        true => self.browser.move_folder(tab, folder, index, cx),
                        false => self.browser.move_tab(tab, true, folder, index, cx),
                    }
                }
                Place::List(false, index) => {
                    // check.sh waits for this line.
                    eprintln!("browser: dragged {tab} to today {index}");
                    // The browser counts tabs, and the rows above may be split views.
                    let (_, rows) = self.rows();
                    let rows = rows.into_iter().filter(|&row| row != tab).take(index);
                    let index = rows.map(|row| self.panes(row).len()).sum();
                    self.browser.move_tab(tab, false, 0, index, cx);
                }
            }
        }
        cx.notify();
    }

    /// The depths a row dropped at `gap` in the pinned rows `rows`, which leave out `dragged`, may
    /// be at: from the row's below, out of the folders it ends, to the row's above, or one in if
    /// that's a folder. None at a gap inside `dragged`, a folder.
    fn gap_depths(&self, rows: &[u64], gap: usize, dragged: u64) -> Option<(usize, usize)> {
        let space = self.shown_space()?;
        let above = gap.checked_sub(1).map(|at| rows[at]);
        let below = rows.get(gap).copied();
        let inside = |row: u64| space.holders(row).any(|folder| folder == dragged);
        if below.is_some_and(inside) {
            return None;
        }
        let low = below.map_or(0, |row| space.depth(row));
        let high = match above {
            None => 0,
            // Not into itself.
            Some(row) if inside(row) => space.depth(dragged),
            Some(row) => space.depth(row) + space.is_folder(row) as usize,
        };
        Some((low, high.max(low)))
    }

    /// Where a row dropped at `gap` in the pinned rows `rows`, without `dragged`, at `depth`
    /// goes: the folder, else 0, and its index among what's directly in that, counted as the
    /// browser counts, without `dragged`.
    fn pinned_place(&self, rows: &[u64], gap: usize, depth: usize, dragged: u64) -> (u64, usize) {
        let Some(space) = self.shown_space() else {
            return (0, gap);
        };
        let above = gap.checked_sub(1).map(|at| rows[at]);
        let folder = match above {
            _ if depth == 0 => 0,
            Some(row) if space.is_folder(row) && depth == space.depth(row) + 1 => row,
            // The folder holding the row above at that depth.
            Some(row) => space.holders(row).nth(space.depth(row) - depth).unwrap_or(0),
            None => 0,
        };
        let before = rows.get(gap).copied().filter(|&row| space.folder_of(row) == folder);
        let panes = space.panes(dragged);
        let index = space
            .pinned
            .iter()
            .take_while(|&&row| Some(row) != before)
            .filter(|&&row| space.folder_of(row) == folder && !panes.contains(&row))
            .count();
        (folder, index)
    }

    /// Puts the tab the pointer drags back where it was, and returns whether there was one.
    fn cancel_drag(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.drag.as_ref().is_some_and(|drag| drag.moving && !drag.dropped) {
            return false;
        }
        // The release that ends the drag isn't a click.
        self.dragged = true;
        self.settle_drag();
        cx.notify();
        true
    }

    /// Ends a drag that moves nothing in the browser: the rows and tiles settle from where they're
    /// drawn.
    fn settle_drag(&mut self) {
        let tiles = self.drawn();
        self.move_rows(self.layout(), Curve::Slide);
        self.drag = None;
        self.move_tiles(tiles, Curve::Slide);
    }

    /// The browser reported its favorites, or else the Space shown's lists. A drag still held ends,
    /// since the lists changed under it. A dragged tab let go lands once it's where it was dropped:
    /// moved in one list, once that list comes, and between them, once it's in the one and out of
    /// the other. Returns whether the drag ended.
    fn end_drag(&mut self, favorites: bool) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let tab = drag.tab;
        let in_favorites = self.favorites.contains(&tab);
        let in_space = self
            .shown_space()
            .is_some_and(|space| space.pinned.contains(&tab) || space.today.contains(&tab));
        let landed = match (drag.from, drag.to) {
            (Place::Favorite(_), Place::Favorite(_)) => favorites,
            (Place::List(..), Place::List(..)) => !favorites,
            (_, Place::Favorite(_)) => in_favorites && !in_space,
            (_, Place::List(..)) => in_space && !in_favorites,
        };
        if drag.dropped && !landed {
            return false;
        }
        self.drag = None;
        true
    }

    /// The Spaces' pages the sidebar draws, and where from its left edge: the shown Space's, and
    /// while a swipe or a settle moves one, it and the page beside it.
    fn pages(&self) -> Vec<(u64, f32)> {
        let swipe = self.swipe.as_ref().filter(|swipe| swipe.across == Some(true));
        let (space, x, beside) = match (swipe, &self.settle) {
            (Some(swipe), _) => (swipe.from, self.swiped_to(swipe), None),
            (None, Some(settle)) => (settle.space, settle.offset().unwrap_or(0.), settle.beside),
            (None, None) => return vec![(self.space, 0.)],
        };
        let beside = beside.or_else(|| {
            let index = self.spaces.iter().position(|other| other.id == space)?;
            let index = if x > 0. { index.checked_sub(1)? } else { index + 1 };
            self.spaces.get(index).map(|other| other.id)
        });
        let mut pages = vec![(space, x)];
        if x != 0.
            && let Some(beside) = beside
        {
            pages.push((beside, x - x.signum() * self.width()));
        }
        pages
    }

    /// Where a swipe has taken the page of the Space it began at: with the fingers, but
    /// rubber-banding past the first and last Spaces, and past one Space either side.
    fn swiped_to(&self, swipe: &Swipe) -> f32 {
        let index = self.spaces.iter().position(|space| space.id == swipe.from).unwrap_or(0);
        let width = self.width();
        let high = if index > 0 { width } else { 0. };
        let low = if index + 1 < self.spaces.len() { -width } else { 0. };
        rubber_band(swipe.way, low, high)
    }

    /// The Space whose page is nearest its place while `from`'s is at `x`: it or a neighbour.
    fn nearest(&self, from: u64, x: f32) -> Option<u64> {
        let index = self.spaces.iter().position(|space| space.id == from)? as f32;
        let index = index - (x / self.width()).round().clamp(-1., 1.);
        let index = index.clamp(0., self.spaces.len().saturating_sub(1) as f32) as usize;
        self.spaces.get(index).map(|space| space.id)
    }

    /// Fingers or a wheel over the sidebar. Across it they page between Spaces, and along it they
    /// scroll the list. Returns whether paging took the event.
    fn scroll(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) -> bool {
        if self.form.is_some() || self.renaming.is_some() || self.drag.is_some() {
            return false;
        }
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => delta.map(f32::from),
            ScrollDelta::Lines(lines) => return self.wheel(lines, cx),
        };
        match event.touch_phase {
            TouchPhase::Started => {
                self.swiped = false;
                self.swipe = Some(Swipe {
                    from: self.space,
                    across: None,
                    way: 0.,
                    samples: VecDeque::new(),
                    nearest: self.space,
                });
            }
            TouchPhase::Ended | TouchPhase::Cancelled => return self.release(cx),
            TouchPhase::Moved => {}
        }
        let Some(mut swipe) = self.swipe.take() else {
            // A page swipe's momentum, across, doesn't scroll the list.
            return self.swiped && delta.x.abs() > delta.y.abs();
        };
        if swipe.across.is_none() && (delta.x != 0. || delta.y != 0.) {
            let across = delta.x.abs() > delta.y.abs();
            swipe.across = Some(across);
            // check.sh waits for this line.
            eprintln!("browser: swipe {}", if across { "across" } else { "along" });
            // Fingers catch a page that's still settling where it is.
            if across
                && let Some(settle) = self.settle.take()
                && let Some(x) = settle.offset()
            {
                (swipe.from, swipe.way, swipe.nearest) = (settle.space, x, settle.space);
            }
        }
        let across = swipe.across == Some(true);
        if across {
            swipe.way += SWIPE_TRACKING * delta.x;
            let (now, x) = (Instant::now(), self.swiped_to(&swipe));
            swipe.samples.push_back((now, x));
            while swipe.samples.front().is_some_and(|&(at, _)| now - at > SWIPE_HISTORY) {
                swipe.samples.pop_front();
            }
            if let Some(nearest) = self.nearest(swipe.from, x)
                && nearest != swipe.nearest
            {
                swipe.nearest = nearest;
                self.browser.detent(cx);
            }
            cx.notify();
        }
        self.swipe = Some(swipe);
        across
    }

    /// The fingers lifted. The pages settle on the next Space in the direction of a flick, else
    /// on the one nearest its place, keeping the release's speed if it's heading there.
    fn release(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(swipe) = self.swipe.take().filter(|swipe| swipe.across == Some(true)) else {
            return false;
        };
        self.swiped = true;
        let (now, x) = (Instant::now(), self.swiped_to(&swipe));
        let recent = swipe.samples.iter().filter(|&&(at, _)| now - at <= SWIPE_HISTORY);
        let velocity = match (recent.clone().next(), recent.last()) {
            (Some(&(start, from)), Some(&(end, to))) if end > start => {
                (to - from) / (end - start).as_secs_f32()
            }
            _ => 0.,
        };
        let toward = if velocity.abs() >= SWIPE_FLICK {
            velocity.signum() * self.width()
        } else {
            x
        };
        let Some(space) = self.nearest(swipe.from, toward) else {
            return true;
        };
        let index = |id| self.spaces.iter().position(|other| other.id == id).unwrap_or(0) as f32;
        let from = x + (index(space) - index(swipe.from)) * self.width();
        let velocity = if velocity * from < 0. { velocity } else { 0. };
        self.settle = Some(Settle { space, beside: None, from, velocity, start: now });
        if space != self.space {
            // check.sh waits for this line.
            eprintln!("browser: command select-space via swipe");
            self.show_space(space, cx);
        }
        cx.notify();
        true
    }

    /// A wheel's ticks across the sidebar page once per burst, then rest.
    fn wheel(&mut self, lines: Point<f32>, cx: &mut Context<Self>) -> bool {
        if lines.x.abs() <= lines.y.abs() {
            return false;
        }
        let now = Instant::now();
        if self.wheel_paged.is_some_and(|paged| now - paged < WHEEL_REST) {
            return true;
        }
        let sum = match self.wheel {
            Some((sum, last)) if now - last < WHEEL_BURST => sum + lines.x,
            _ => lines.x,
        };
        self.wheel = Some((sum, now));
        if (sum * WHEEL_LINE).abs() > 1. {
            self.wheel = None;
            self.wheel_paged = Some(now);
            if let Some(space) = self.nearest(self.space, sum.signum() * self.width())
                && space != self.space
            {
                // check.sh waits for this line.
                eprintln!("browser: command select-space via wheel");
                self.show_space(space, cx);
            }
        }
        true
    }

    /// Fingers and wheels over the sidebar, heard ahead of the list, which would scroll with them.
    fn scroll_listener(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let shell = cx.weak_entity();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                window.on_mouse_event(move |event: &ScrollEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture
                        && bounds.contains(&event.position)
                        && shell.update(cx, |shell, cx| shell.scroll(event, cx)).unwrap_or(false)
                    {
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .size_full()
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

    /// Where the sidebar's top-left corner is in the window a press is in, heard before the row or
    /// tile pressed hears it, for where a drag is in the sidebar.
    fn press_listener(&self) -> impl IntoElement + use<> {
        let origin = self.sidebar_origin.clone();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                window.on_mouse_event(move |_: &MouseDownEvent, phase, _, _| {
                    if phase == DispatchPhase::Capture {
                        origin.set(bounds.origin);
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }

    /// The pointer's moves and release anywhere in the window, for a pressed row or tile, which
    /// only hears the pointer while it's over it.
    fn drag_listeners(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let shell = cx.weak_entity();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let moved = shell.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture {
                        moved.update(cx, |shell, cx| shell.drag_to(event.position, cx)).ok();
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        shell.update(cx, |shell, cx| shell.drop_row(cx)).ok();
                    }
                });
            },
        )
        .absolute()
        .size_0()
    }

    /// Drops the list's and the pages' motion that's done, and returns whether any is left.
    fn list_moving(&mut self) -> bool {
        let sliding = !self.slides.is_empty();
        self.slides
            .retain(|_, (_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_some());
        if sliding && self.slides.is_empty() {
            // check.sh waits for this line before it presses a row that moved.
            eprintln!("browser: rows settled");
        }
        self.tile_slides
            .retain(|_, (_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_some());
        self.list_slide
            .take_if(|(_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_none());
        self.closing.retain(|row| row_spring(row.start.elapsed().as_secs_f32()).is_some());
        self.unhovered.retain(|(_, start)| start.elapsed().as_secs_f32() < ROW_FILL);
        if self.slides.is_empty() {
            self.opened.take_if(|opened| opened.done().is_none());
        }
        if let Some(settle) = self.settle.take_if(|settle| settle.offset().is_none()) {
            // check.sh reads these lines.
            eprintln!("browser: settled space {}", settle.space);
        }
        !(self.slides.is_empty() && self.closing.is_empty() && self.unhovered.is_empty())
            || !self.tile_slides.is_empty()
            || self.list_slide.is_some()
            || self.opened.is_some()
            || self.settle.is_some()
    }

    /// A header button at `x`, with the glyph `icon`, that runs `command`. One that can't dims, as
    /// back does on a page with no history.
    fn nav_button(
        &self,
        x: f32,
        icon: &'static str,
        command: Command,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div()
            .id(command.name())
            .absolute()
            .left(px(x))
            .top(px(NAV_TOP))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .when(enabled, |button| {
                button
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.run(command, "sidebar", cx)
                    }))
            })
            .child(
                svg()
                    .path(icon)
                    .size(px(NAV_GLYPH))
                    .text_color(rgba(if enabled { NAV } else { NAV_DISABLED })),
            )
    }

    /// The sidebar button, back, forward, and reload, which is stop while the page loads, as many
    /// as fit.
    fn nav_buttons(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let page = self.page(self.selected);
        let reload = match page.loading {
            true => (RELOAD_X, "icons/x.svg", Command::Stop, true),
            false => (RELOAD_X, "icons/rotate-cw.svg", Command::Reload, self.selected != 0),
        };
        let buttons = [
            (SIDEBAR_BUTTON_X, "icons/panel-left.svg", Command::ToggleSidebar, true),
            (BACK_X, "icons/arrow-left.svg", Command::Back, page.can_go_back),
            (FORWARD_X, "icons/arrow-right.svg", Command::Forward, page.can_go_forward),
            reload,
        ];
        let width = self.width();
        buttons
            .into_iter()
            .filter(|&(x, ..)| x + NAV_SIZE <= width - NAV_END)
            .map(|(x, icon, command, enabled)| {
                self.nav_button(x, icon, command, enabled, cx).into_any_element()
            })
            .collect()
    }

    /// The URL field's box in the window, while the docked sidebar shows it.
    fn url_field_bounds(&self) -> Option<Bounds<Pixels>> {
        let docked = !self.page_fullscreen && !self.sidebar_hidden && self.sidebar_motion.is_none();
        (docked && self.form().is_none() && self.library.is_none()).then(|| {
            Bounds::new(
                point(px(PLATE_INSET), px(URL_FIELD_TOP)),
                size(px(self.width() - 2. * PLATE_INSET), px(URL_FIELD_HEIGHT)),
            )
        })
    }

    fn url_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let url = self.url(self.selected);
        // Copy URL, at the field's end, where the host stops short of it.
        let copy = (self.tools_shown() && !url.is_empty()).then(|| {
            div()
                .id("copy-url")
                .absolute()
                .top(px(URL_BUTTON_INSET))
                .right(px(URL_BUTTON_INSET))
                .size(px(URL_BUTTON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .block_mouse_except_scroll()
                .hover(|style| style.bg(rgba(NAV_HOVERED)))
                .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::CopyUrl, "sidebar", cx)))
                .child(
                    svg()
                        .path("icons/link.svg")
                        .size(px(URL_BUTTON_GLYPH))
                        .text_color(rgba(NAV)),
                )
        });
        let text_end = match copy {
            Some(_) => URL_BUTTON_INSET + URL_BUTTON_SIZE,
            None => URL_FIELD_TEXT_X - PLATE_INSET,
        };
        div()
            .id("url-field")
            .absolute()
            .top(px(URL_FIELD_TOP))
            .left(px(PLATE_INSET))
            .w(px(self.width() - 2. * PLATE_INSET))
            .h(px(URL_FIELD_HEIGHT))
            .flex()
            .items_center()
            .pl(px(URL_FIELD_TEXT_X - PLATE_INSET))
            .pr(px(text_end))
            .rounded(px(PLATE_RADIUS))
            .block_mouse_except_scroll()
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::EditUrl, "sidebar", cx)))
            .bg(rgba(URL_FIELD))
            .text_size(px(13.))
            .text_color(rgba(SECONDARY))
            .child(div().truncate().child(SharedString::from(host(&url))))
            .children(copy)
    }

    /// Whether the sidebar shows Clear and the URL field's button, which it does while the pointer
    /// is over it (Arc). The reveal's panel shows only under the pointer.
    fn tools_shown(&self) -> bool {
        self.sidebar_hovered || self.sidebar_hidden
    }

    fn sidebar_hover(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if hovered != self.sidebar_hovered {
            self.sidebar_hovered = hovered;
            // check.sh reads this line.
            eprintln!("browser: sidebar hovered {}", hovered as u8);
            cx.notify();
        }
    }

    /// A sidebar row: the plate, with the favicon slot and the label.
    fn row(
        &self,
        row: Row,
        content: gpui::Div,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let selected = row == self.selected_row();
        let (id, slot) = match row {
            Row::NewTab => (("new-tab", 0), Slot::NewTab),
            Row::Tab(tab) => (("tab", tab), Slot::Tab(tab)),
        };
        // Full for a dragged row, which the pointer holds, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(dragged) => (dragged == slot).then_some(1.),
            None => self.hover_fill(row),
        };
        // A new tab's plate fades in, and rows that come together fade in whole.
        let opened = self.opened.as_ref().filter(|opened| {
            matches!(row, Row::Tab(tab) if opened.tabs.contains(&tab))
        });
        let (row_in, plate_in) = opened.map_or((1., 1.), |opened| {
            let done = opened.done().unwrap_or(1.).clamp(0., 1.);
            if opened.whole { (done, 1.) } else { (1., done) }
        });
        // In from its folders, or where a dragged row would go.
        let depth = match (row, &self.drag) {
            (_, Some(drag)) if dragging == Some(slot) => drag.depth,
            (Row::Tab(tab), _) => self.depth(tab),
            (Row::NewTab, _) => 0,
        };
        row_box()
            .pl(px(PLATE_INSET + depth as f32 * FOLDER_INDENT))
            .id(id)
            .group("row")
            .opacity(row_in)
            // A dragged row is drawn over the list, from `list`.
            .top(px(if dragging == Some(slot) { 0. } else { self.slide(slot) }))
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(row, *hovered, cx)
            }))
            .child(
                content
                    .when(selected, |plate| selected_plate(plate, plate_in))
                    .id("plate")
                    .when_some(hover.filter(|_| !selected), |plate, hover| {
                        plate.bg(Hsla::from(rgba(HOVERED)).opacity(hover))
                    })
                    .when(!selected && dragging != Some(slot), |plate| {
                        // GPUI keeps an element's pressed state only once a listener gives it a
                        // hitbox.
                        plate
                            .on_mouse_down(MouseButton::Left, |_, _, _| {})
                            .group_active("row", |style| style.bg(rgba(PRESSED)))
                    })
                    // A dragged row covers the rows it passes, in the colours it has in place.
                    .when(dragging == Some(slot), |plate| {
                        let fill = if selected { SELECTED } else { HOVERED };
                        plate.bg(self.window_fill().blend(rgba(fill).into()))
                    }),
            )
    }

    /// How full `row`'s hover fill is: full under the pointer or with its menu up, and fading once
    /// it's left. None for none.
    fn hover_fill(&self, row: Row) -> Option<f32> {
        if self.hovered == Some(row) || self.menu_row.is_some_and(|tab| row == Row::Tab(tab)) {
            return Some(1.);
        }
        self.unhovered.iter().find(|&&(old, _)| old == row).map(|(_, start)| {
            let t = (start.elapsed().as_secs_f32() / ROW_FILL).min(1.);
            1. - cubic_bezier(0., 0., 0.58, 1., t)
        })
    }

    /// The favorites' tiles under the URL field, which stay put while the Spaces' pages move under
    /// them, and slide to their places as tiles come, go and make room for a dragged tab.
    fn tiles(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let drawn = self.drawn_tiles();
        let dragged = self.tile_ghost().map(|(tab, _)| tab);
        let tiles: Vec<_> = self
            .tile_tabs()
            .into_iter()
            .filter(|&tab| Some(tab) != dragged)
            .map(|tab| {
                let [left, top, width] = drawn[&tab];
                self.tile(tab, cx).absolute().left(px(left)).top(px(top)).w(px(width))
            })
            .collect();
        (!tiles.is_empty()).then(|| {
            div().absolute().top(px(LIST_TOP)).left_0().w(px(self.width())).children(tiles)
        })
    }

    /// A tab dragged over the favorites, as a tile where the pointer has it, over the rest of the
    /// sidebar.
    fn dragged_tile(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let (tab, [left, top, width]) = self.tile_ghost()?;
        Some(self.tile(tab, cx).absolute().left(px(left)).top(px(LIST_TOP + top)).w(px(width)))
    }

    /// The tiles in their order: the favorites, with a dragged tab out of its place, and in the
    /// place it would go to among them.
    fn tile_tabs(&self) -> Vec<u64> {
        let mut tabs = self.favorites.clone();
        if let Some(drag) = self.drag.as_ref().filter(|drag| drag.moving) {
            tabs.retain(|&tab| tab != drag.tab);
            if let Place::Favorite(index) = drag.to {
                tabs.insert(index.min(tabs.len()), drag.tab);
            }
        }
        tabs
    }

    /// Where `count` tiles go, down from the favorites' top: each one's left, top and width. A row
    /// has as many as fit, each as wide as the others, and fewer tiles share it.
    fn tile_places(&self, count: usize) -> Vec<[f32; 3]> {
        let room = self.width() - 2. * PLATE_INSET + TILE_GAP;
        let fit = (room / (TILE_MIN + TILE_GAP)).floor().max(1.) as usize;
        let columns = fit.min(count).max(1);
        let width = room / columns as f32 - TILE_GAP;
        (0..count)
            .map(|index| {
                let (column, row) = ((index % columns) as f32, (index / columns) as f32);
                [PLATE_INSET + column * (width + TILE_GAP), row * (TILE_HEIGHT + TILE_GAP), width]
            })
            .collect()
    }

    /// Where each tile is drawn, down from the favorites' top: at its place or on its way there,
    /// and a dragged one where the pointer has it.
    fn drawn_tiles(&self) -> HashMap<u64, [f32; 3]> {
        let tabs = self.tile_tabs();
        let places = self.tile_places(tabs.len());
        let mut drawn: HashMap<u64, [f32; 3]> = tabs
            .into_iter()
            .zip(places)
            .map(|(tab, place)| {
                let way = self.tile_slides.get(&tab).map_or(0., |&(_, start, curve)| {
                    curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| 1. - done)
                });
                let from = self.tile_slides.get(&tab).map_or([0.; 3], |&(from, ..)| from);
                (tab, std::array::from_fn(|i| place[i] + from[i] * way))
            })
            .collect();
        drawn.extend(self.tile_ghost());
        drawn
    }

    /// A tab dragged over the favorites, and its tile where the pointer has it.
    fn tile_ghost(&self) -> Option<(u64, [f32; 3])> {
        let drag = self.drag.as_ref().filter(|drag| drag.moving)?;
        if !matches!(drag.to, Place::Favorite(_)) {
            return None;
        }
        let width = self.tile_places(self.tile_tabs().len()).first()?[2];
        let centre = drag.pointer - drag.grab.1;
        let top = centre.y - LIST_TOP - TILE_HEIGHT / 2.;
        Some((drag.tab, [centre.x - width / 2., top, width]))
    }

    /// Where the tiles and the list's top are drawn, for motion from there.
    fn drawn(&self) -> (HashMap<u64, [f32; 3]>, f32) {
        (self.drawn_tiles(), self.drawn_list_top())
    }

    /// Starts the tiles and the list's top from where they were drawn `before` to their places.
    fn move_tiles(&mut self, before: (HashMap<u64, [f32; 3]>, f32), curve: Curve) {
        let now = Instant::now();
        let tabs = self.tile_tabs();
        let places = self.tile_places(tabs.len());
        self.tile_slides = tabs
            .into_iter()
            .zip(places)
            .filter_map(|(tab, place)| {
                let was = before.0.get(&tab)?;
                let from: [f32; 3] = std::array::from_fn(|i| was[i] - place[i]);
                (from != [0.; 3]).then_some((tab, (from, now, curve)))
            })
            .collect();
        let from = before.1 - self.list_top();
        self.list_slide = (from != 0.).then_some((from, now, curve));
    }

    /// Where the Space's pages start with `count` tiles: under them, else under the URL field.
    fn list_top_with(&self, count: usize) -> f32 {
        let rows = self.tile_places(count).last().map_or(0., |&[_, top, _]| top + TILE_HEIGHT);
        LIST_TOP + if count > 0 { rows + TILE_GAP } else { 0. }
    }

    /// Where the Space's pages start: under the tiles.
    fn list_top(&self) -> f32 {
        self.list_top_with(self.tile_tabs().len())
    }

    /// Where the Space's pages start as drawn: at list_top, or on their way there.
    fn drawn_list_top(&self) -> f32 {
        self.list_top()
            + self.list_slide.map_or(0., |(from, start, curve)| {
                curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| from * (1. - done))
            })
    }

    /// A favorite's tile: its favicon on a plate, which the tab shown has filled and ringed. A
    /// click shows it, a double-click takes it back to its URL (Dia), and a right-click shows its
    /// menu.
    fn tile(&self, tab: u64, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let selected = tab == self.selected && !self.new_tab;
        // Full for a dragged tile, which covers what it passes, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(dragged) => (dragged == Slot::Tab(tab)).then_some(1.),
            None => self.hover_fill(Row::Tab(tab)),
        };
        let fill = hsla(0., 0., 1., TILE + (TILE_HOVERED - TILE) * hover.unwrap_or(0.));
        let fill = match dragging == Some(Slot::Tab(tab)) {
            true => self.window_fill().blend(if selected { rgba(SELECTED).into() } else { fill }),
            false if selected => rgba(SELECTED).into(),
            false => fill,
        };
        let icon = div()
            .size(px(FAVICON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone())));
        div()
            .id(("tile", tab))
            .h(px(TILE_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(TILE_RADIUS))
            .border(px(HAIRLINE))
            .border_color(rgba(if selected { TILE_RING } else { TILE_EDGE }))
            .bg(fill)
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(Row::Tab(tab), *hovered, cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click showed it.
                    if event.click_count == 2 {
                        eprintln!("browser: command back-to-pinned-url via favorite");
                        return shell.browser.back_to_pinned_url(tab, cx);
                    }
                    shell.press_tile(tab, event.position)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(tab, event.position, cx)
                    }
                }),
            )
            // Unless it ended a drag.
            .on_click(cx.listener(move |shell, _, _, cx| {
                if shell.dragged || shell.dragging().is_some() {
                    return;
                }
                eprintln!("browser: command select via favorite");
                shell.select(tab, cx)
            }))
            .child(icon)
    }

    /// A closed tab's row where it was, without the selected fill it may have had, fading as the
    /// rows below slide over it.
    fn closing_row(&self, row: &Closing) -> impl IntoElement + use<> {
        let done = row_spring(row.start.elapsed().as_secs_f32()).unwrap_or(1.);
        div()
            .absolute()
            .top(px(row.top))
            .left_0()
            .w(px(self.width()))
            .h(px(ROW_HEIGHT))
            .px(px(PLATE_INSET))
            .pl(px(PLATE_INSET + row.indent))
            .py(px((ROW_HEIGHT - PLATE_HEIGHT) / 2.))
            .opacity((1. - done).clamp(0., 1.))
            .child(plate(
                row.folder.map_or_else(|| favicon(row.favicon.clone()), folder_icon),
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .pr(px(PLATE_INSET))
                    .child(title(row.title.clone(), false)),
            ))
    }

    /// A tab's row, its split view's for the split's first tab, or a folder's.
    fn tab_row(&self, tab: u64, cx: &mut Context<Self>) -> AnyElement {
        if self.is_folder(tab) {
            return self.folder_row(tab, cx).into_any_element();
        }
        let panes = self.panes(tab);
        match panes.len() > 1 {
            true => self.split_row(panes, cx).into_any_element(),
            false => self.single_row(tab, cx).into_any_element(),
        }
    }

    /// A folder's row: its icon, open or closed, and its name. A click opens or closes it, a
    /// double-click renames it and a right-click shows its menu.
    fn folder_row(&self, folder: u64, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.shown_space().is_some_and(|space| space.is_open(folder));
        let renaming =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Folder(folder));
        let label = match renaming {
            Some(renaming) => self.naming_field(renaming, cx),
            None => title(self.folder_name(folder), false).pr(px(PLATE_INSET)),
        };
        let renaming = renaming.is_some();
        self.row(Row::Tab(folder), plate(folder_icon(open), label), cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click opened or closed it.
                    if event.click_count == 2 {
                        return shell.rename_folder(folder, cx);
                    }
                    shell.press_row(folder, event.position)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(folder, event.position, cx)
                    }
                }),
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_folder(folder, cx)))
        })
    }

    /// The button that closes `tabs`, which shows under the pointer at a row's end.
    fn close_button(&self, tabs: Vec<u64>, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        div()
            .id(("close", tabs[0]))
            .flex_none()
            .ml(px(4.))
            .mr(px(CLOSE_RIGHT))
            .size(px(CLOSE_SIZE))
            .rounded(px(NAV_RADIUS))
            .hover(|style| style.bg(rgba(CLOSE_HOVERED)))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(16.))
            .text_color(rgba(SECONDARY))
            .child("×")
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                eprintln!("browser: command close-tab via sidebar");
                shell.browser.close(tabs[0], true, cx);
            }))
    }

    /// The field that renames a tab, in place of its title.
    fn naming_field(&self, renaming: &Renaming, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .pr(px(PLATE_INSET))
            .key_context("Naming")
            .on_action(cx.listener(|shell, _: &KeepName, window, cx| shell.keep_name(window, cx)))
            .on_action(
                cx.listener(|shell, _: &CancelName, window, cx| shell.cancel_name(window, cx)),
            )
            .text_size(px(14.))
            .text_color(rgb(FORM_TEXT))
            .child(renaming.name.clone())
    }

    /// A click on `tab`'s row, or its segment of a split view's, shows it, unless it ended a drag.
    fn click_row(&mut self, tab: u64, cx: &mut Context<Self>) {
        let renaming = self.renaming.as_ref().map(|renaming| renaming.naming);
        if self.dragged || self.dragging().is_some() || renaming == Some(Naming::Tab(tab)) {
            return;
        }
        eprintln!("browser: command select via sidebar");
        self.select(tab, cx)
    }

    /// A tab's segment of its split view's row: its favicon and its title, bright if `focused`,
    /// after a line unless it's the `first`.
    fn segment(&self, tab: u64, first: bool, focused: bool) -> gpui::Div {
        let page = self.page(tab);
        let icon = div()
            .flex_none()
            .size(px(FAVICON_SIZE))
            .mr(px(SEGMENT_TITLE_GAP))
            .flex()
            .items_center()
            .justify_center()
            .when(page.unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone())));
        let line = div()
            .absolute()
            .left_0()
            .top(px((PLATE_HEIGHT - SEGMENT_LINE) / 2.))
            .w(px(1.))
            .h(px(SEGMENT_LINE))
            .bg(rgba(DIVIDER));
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .pl(px(SEGMENT_FAVICON_X))
            .pr(px(SEGMENT_END))
            .when(!first, |segment| segment.child(line))
            .child(icon)
            .child(title(self.title(tab), focused))
            .when(page.loading, |segment| segment.child(loading_ring(tab)))
    }

    /// `segment`, which a click shows focused, a double-click renames and a right-click shows the
    /// tab's menu for.
    fn split_segment(
        &self,
        tab: u64,
        first: bool,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        self.segment(tab, first, focused)
            .id(("segment", tab))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click selected the tab (Dia).
                    if event.click_count == 2 {
                        cx.stop_propagation();
                        shell.rename_tab(tab, cx);
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(tab, event.position, cx)
                    }
                }),
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_row(tab, cx)))
    }

    /// A split view's row, for its first tab: a segment per tab, left to right. A click on one
    /// shows the split with its page focused, and the close button closes them all.
    fn split_row(&self, panes: Vec<u64>, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let first = panes[0];
        let row_selected = self.selected_row() == Row::Tab(first);
        let hovered = self.hovered == Some(Row::Tab(first));
        // A pinned tab closed already has nothing to close.
        let pinned = self.pinned().contains(&first);
        let closable = panes.iter().any(|&tab| !(self.page(tab).unloaded && pinned));
        let renaming = self.renaming.as_ref().filter(|renaming| {
            matches!(renaming.naming, Naming::Tab(tab) if panes.contains(&tab))
        });
        let content = match renaming {
            Some(renaming) => {
                let Naming::Tab(tab) = renaming.naming else {
                    unreachable!()
                };
                let icon = favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()));
                plate(icon, self.naming_field(renaming, cx))
            }
            None => {
                let close = (hovered && closable).then(|| self.close_button(panes.clone(), cx));
                let segments: Vec<_> = panes
                    .iter()
                    .enumerate()
                    .map(|(index, &tab)| {
                        let focused = row_selected && tab == self.selected;
                        self.split_segment(tab, index == 0, focused, cx)
                    })
                    .collect();
                split_plate(segments).children(close)
            }
        };
        let renaming = renaming.is_some();
        self.row(Row::Tab(first), content, cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, _| {
                    shell.press_row(first, event.position)
                }),
            )
        })
    }

    fn single_row(&self, tab: u64, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let page = self.page(tab);
        let loading = page.loading;
        // A pinned tab closed already has nothing to close.
        let closable = !(page.unloaded && self.pinned().contains(&tab));
        let hovered = self.hovered == Some(Row::Tab(tab));
        // Away from its pinned URL, a pinned tab's favicon is a way back.
        let away = page.away_from_pinned_url;
        let back = away && self.hovered_favicon == Some(tab);
        let label = if back { "Back to Pinned URL".into() } else { self.title(tab) };
        let title = title(label, Row::Tab(tab) == self.selected_row());
        // The close button shows under the pointer, and the title truncates before it.
        let close = self.close_button(vec![tab], cx);
        let renaming =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Tab(tab));
        let label = match renaming {
            Some(renaming) => self.naming_field(renaming, cx),
            None => div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .when(!(hovered && closable), |label| label.pr(px(PLATE_INSET)))
                .child(title)
                .when(hovered && closable, |label| label.child(close))
                .when(!hovered && loading, |label| label.child(loading_ring(tab))),
        };
        let renaming = renaming.is_some();
        let icon = match back {
            true => svg()
                .path("icons/arrow-left.svg")
                .size(px(FAVICON_DRAWN))
                .text_color(rgba(NAV))
                .into_any_element(),
            false => favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone())),
        };
        let icon = div()
            .id(("favicon", tab))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .when(page.unloaded && !back, |icon| icon.opacity(SLEEPING_FAVICON))
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                if *hovered {
                    shell.hovered_favicon = Some(tab);
                } else if shell.hovered_favicon == Some(tab) {
                    shell.hovered_favicon = None;
                }
                cx.notify();
            }))
            // The row's click selects the tab after this one.
            .when(away, |icon| {
                icon.on_click(cx.listener(move |shell, _, _, cx| {
                    eprintln!("browser: command back-to-pinned-url via sidebar");
                    shell.browser.back_to_pinned_url(tab, cx);
                }))
            })
            .child(icon);
        // The field takes the row's clicks while it's up.
        self.row(Row::Tab(tab), plate(icon, label), cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click selected the tab (Dia).
                    if event.click_count == 2 {
                        return shell.rename_tab(tab, cx);
                    }
                    shell.press_row(tab, event.position)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(tab, event.position, cx)
                    }
                }),
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_row(tab, cx)))
        })
    }

    fn name_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let space = self.space;
        let row = name_box().id("space-name").block_mouse_except_scroll();
        if let Some(renaming) =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Space(space))
        {
            return row
                .key_context("Naming")
                .on_action(
                    cx.listener(|shell, _: &KeepName, window, cx| shell.keep_name(window, cx)),
                )
                .on_action(
                    cx.listener(|shell, _: &CancelName, window, cx| shell.cancel_name(window, cx)),
                )
                .text_color(rgb(FORM_TEXT))
                .child(renaming.name.clone());
        }
        let name = self.shown_space().map(|space| space.name.clone()).unwrap_or_default();
        row.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    shell.rename_space(space, cx);
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                shell.space_menu(space, event.position, cx)
            }),
        )
        .child(div().truncate().child(name))
    }

    /// A dot for each Space, in the footer. Arc draws none while there's only one.
    fn switcher(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.spaces.len() < 2 {
            return None;
        }
        let span = SWITCHER_RIGHT - SWITCHER_LEFT;
        let pitch = SPACE_PITCH.min(span / self.spaces.len() as f32);
        let dots = self.spaces.iter().map(|space| {
            let (id, shown) = (space.id, space.id == self.space);
            div()
                .id(("space", id))
                .group("space")
                .w(px(pitch))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .block_mouse_except_scroll()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    eprintln!("browser: command select-space via sidebar");
                    shell.show_space(id, cx)
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                        shell.space_menu(id, event.position, cx)
                    }),
                )
                .child(
                    div()
                        .size(px(SPACE_DOT))
                        .rounded_full()
                        .bg(rgba(if shown { SPACE_SHOWN } else { SPACE_OTHER }))
                        .when(!shown, |dot| {
                            dot.group_hover("space", |style| style.bg(rgba(SPACE_HOVERED)))
                        }),
                )
        });
        Some(
            div()
                .absolute()
                .left(px(SWITCHER_LEFT))
                .w(px(span))
                .bottom(px((FOOTER_HEIGHT - SPACE_HEIGHT) / 2.))
                .h(px(SPACE_HEIGHT))
                .flex()
                .justify_center()
                .children(dots),
        )
    }

    /// The Create a Space form, in place of the URL field and the Space's tabs.
    fn space_form(&self, form: &SpaceForm, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let named = !form.name.read(cx).text().trim().is_empty();
        let plate = |id: &'static str| {
            div()
                .id(id)
                .absolute()
                .left(px(FORM_INSET))
                .w(px(self.width() - 2. * FORM_INSET))
                .h(px(FORM_PLATE_HEIGHT))
                .flex()
                .items_center()
                .rounded(px(FORM_RADIUS))
                .block_mouse_except_scroll()
        };
        let title = div()
            .absolute()
            .top(px(FORM_TITLE_TOP))
            .w_full()
            .h(px(FORM_TITLE_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(17.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(FORM_TEXT))
            .child("New Space");
        let field = plate("space-name-field")
            .top(px(FORM_FIELD_TOP))
            .px(px(FORM_TEXT_X - FORM_INSET))
            .bg(rgba(FORM_PLATE))
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(FORM_TEXT))
            .child(form.name.clone());
        let create = plate("create-space")
            .bottom(px(CREATE_BOTTOM))
            .justify_center()
            .bg(rgb(CREATE))
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if named {
                rgb(FORM_TEXT)
            } else {
                rgba(CREATE_LABEL_DISABLED)
            })
            .child("Create Space")
            .on_click(cx.listener(|shell, _, window, cx| shell.create_space(window, cx)));
        let cancel = div()
            .absolute()
            .bottom(px(CANCEL_BOTTOM))
            .w_full()
            .h(px(CANCEL_HEIGHT))
            .flex()
            .justify_center()
            .child(
                div()
                    .id("cancel-space")
                    .h_full()
                    .flex()
                    .items_center()
                    .block_mouse_except_scroll()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgba(SECONDARY))
                    .child("Cancel")
                    .on_click(cx.listener(|shell, _, window, cx| shell.cancel_space(window, cx))),
            );
        div()
            .key_context("SpaceForm")
            .on_action(cx.listener(|shell, _: &CreateSpace, window, cx| {
                shell.create_space(window, cx)
            }))
            .absolute()
            .top_0()
            .left_0()
            .w(px(self.width()))
            .bottom_0()
            .child(title)
            .child(field)
            .child(create)
            .child(cancel)
    }

    /// The line over Today, and Clear at its end, where the titles end.
    fn divider(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let clear = self.tools_shown().then(|| {
            clear_label()
                .id("clear")
                .block_mouse_except_scroll()
                .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::Clear, "sidebar", cx)))
        });
        divider_box(clear).top(px(self.slide(Slot::Divider)))
    }

    /// The toast over the bottom of the list.
    fn toast(&self) -> Option<impl IntoElement + use<>> {
        let toast = self.toast.as_ref()?;
        Some(
            div()
                .id("toast")
                .absolute()
                .left(px(PLATE_INSET))
                .w(px(self.width() - 2. * PLATE_INSET))
                .bottom(px(FOOTER_HEIGHT))
                .h(px(PLATE_HEIGHT))
                .flex()
                .items_center()
                .px(px(URL_FIELD_TEXT_X - PLATE_INSET))
                .rounded(px(PLATE_RADIUS))
                .block_mouse_except_scroll()
                .bg(self.window_fill().blend(rgba(TOAST).into()))
                .text_size(px(13.))
                .text_color(rgba(TITLE))
                .child(toast.text.clone()),
        )
    }

    fn new_tab_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (plus, label) = new_tab_label();
        self.row(Row::NewTab, plate(plus, label), cx)
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::NewTab, "sidebar", cx)))
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
            eprintln!("browser: outline {outline}");
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
                eprintln!("browser: rename folder {folder}");
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
                eprintln!("browser: sidebar width {}", self.sidebar_width);
            }
        }
        if let Some((_, start)) = self.sidebar_motion {
            if self.sidebar_progress(start).is_some() {
                window.request_animation_frame();
            } else {
                self.sidebar_motion = None;
                // check.sh waits for these lines.
                let state = if self.sidebar_hidden { "hidden" } else { "shown" };
                eprintln!("browser: sidebar {state}");
                if !self.sidebar_hidden {
                    self.browser.show_window_buttons(true, cx);
                }
            }
        }
        self.step_library(window, cx);
        if self.list_moving() {
            window.request_animation_frame();
        }
        // The page is laid out at every step, as in Arc.
        self.viewport = window.viewport_size();
        self.lay_out_page(cx);
        let card_left = self.card_left();
        let focus_color = self.shown_space().and_then(|space| space.color);
        let focus_color = focus_color.unwrap_or(self.accent_color);
        if self.focus_color != Some(focus_color) {
            self.focus_color = Some(focus_color);
            self.browser.set_focus_color(focus_color, cx);
        }
        // The find bar and the zoom HUD sit at the focused page's corner.
        let (frame, _) = self.page_frame(self.viewport);
        let panes = self.panes(self.selected);
        let focused = panes.iter().position(|&pane| pane == self.selected).unwrap_or(0);
        let pane = pane_frames(frame, panes.len())[focused];
        let page_right = f32::from(frame.right() - pane.right());
        if self.page_right != page_right {
            self.page_right = page_right;
            self.browser.set_page_right(page_right, cx);
        }
        // The Library takes the sidebar's place at once, and the card uncovers it as it moves.
        let library = self.library.is_some() && !self.page_fullscreen;
        let library = library.then(|| self.library_panel(cx));
        // The sidebar leaves with the card's edge, all of it by the time the edge stops.
        let width = self.width();
        let sidebar_left = (card_left - width) * width / (width - CARD_INSET);
        let sidebar_shown = !self.page_fullscreen
            && library.is_none()
            && (!self.sidebar_hidden || self.sidebar_motion.is_some());
        let sidebar = sidebar_shown
            .then(|| self.sidebar(cx).left(px(sidebar_left)).child(self.hover_listener(cx)));
        let docked = sidebar_shown && !self.sidebar_hidden && self.sidebar_motion.is_none();
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
                    eprintln!("browser: window move");
                    window.start_window_move();
                }
                shell.pointer_at(event.position.x.into(), cx);
            }))
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
            .children(library)
            .children(sidebar)
            .children(edge)
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
        let mut cards: Vec<_> = pane_frames(frame, panes.len())
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
            cards.push(card(pane_frames(full, count)[count - 1]));
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
        let sidebar = match self.form() {
            Some(form) => sidebar.child(self.space_form(form, cx)),
            None => sidebar
                .child(self.url_field(cx))
                .children(self.tiles(cx))
                .child(self.space_pages(cx))
                .children(self.dragged_tile(cx)),
        };
        sidebar
            .children(self.toast())
            .child(self.library_button(cx))
            .children(self.switcher(cx))
            .child(self.scroll_listener(cx))
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
            .top(px(self.drawn_list_top()))
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
                let name = title(name.clone(), false).pr(px(PLATE_INSET));
                return row.child(plate(folder_icon(*open), name));
            }
            let panes = space.panes(tab);
            if panes.len() > 1 {
                let segments =
                    panes.iter().map(|&pane| self.segment(pane, pane == tab, pane == selected));
                return row.child(
                    split_plate(segments)
                        .when(panes.contains(&selected), |plate| selected_plate(plate, 1.)),
                );
            }
            let icon = favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()));
            let icon = div()
                .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
                .child(icon);
            row.child(
                plate(icon, title(self.title(tab), tab == selected).pr(px(PLATE_INSET)))
                    .when(tab == selected, |plate| selected_plate(plate, 1.)),
            )
        };
        let (plus, new_tab) = new_tab_label();
        let (pinned, today) = space.rows();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(name_box().child(div().truncate().child(space.name.clone())))
            .children(pinned.into_iter().map(row))
            .when(!today.is_empty(), |page| {
                page.child(divider_box(self.tools_shown().then(clear_label)))
            })
            .child(row_box().child(plate(plus, new_tab)))
            .children(today.into_iter().map(row))
    }

    /// The Space's name, its pinned tabs, the divider, New Tab and its Today tabs, under the URL
    /// field. Closed tabs' rows fade under the rest, and while rows slide down to make room for a
    /// tab's new row, they come out from beneath it.
    fn list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
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
        // Over the rest, where the pointer has it, unless it's over the favorites.
        let dragged = self.drag.as_ref().filter(|drag| drag.moving);
        let dragged = dragged.filter(|drag| matches!(drag.to, Place::List(..))).map(|drag| {
            let row = self.tab_row(drag.tab, cx);
            div().absolute().top(px(self.drag_at(drag))).left_0().w(px(self.width())).child(row)
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
            .children(self.closing.iter().map(|row| self.closing_row(row)))
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
                    eprintln!("browser: reveal shown");
                    // Its window ends at its edge, so the page past it takes clicks again.
                    self.browser.show_reveal(REVEAL_INSET + self.width(), cx);
                } else {
                    eprintln!("browser: reveal hidden");
                    self.browser.hide_reveal(cx);
                }
            }
        }
        if self.list_moving() {
            window.request_animation_frame();
        }
        let left = REVEAL_INSET - (REVEAL_INSET + self.width()) * (1. - self.reveal_out());
        div()
            .size_full()
            .font_family(".SystemUIFont")
            // A double-click below the rows opens a new tab, as in the docked sidebar.
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseUpEvent, _, cx| {
                    let below_rows = event.position.y >= px(REVEAL_INSET + shell.list_top());
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
                    .rounded(px(CARD_RADIUS))
                    .border(px(0.5))
                    .border_color(rgba(REVEAL_BORDER))
                    .bg(self.gradient(Some(rgb(MATERIAL_STAND_IN).into())))
                    .child(self.window_buttons(cx))
                    .child(self.sidebar(cx).left_0()),
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
                        eprintln!("browser: window button {button:?}");
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

/// A loading tab's ring, at the row's trailing edge, turning.
fn loading_ring(tab: u64) -> impl IntoElement {
    svg()
        .path("icons/loading.svg")
        .flex_none()
        .ml(px(4.))
        .size(px(LOADING_SIZE))
        .text_color(rgba(SECONDARY))
        .with_animation(
            ("loading", tab),
            Animation::new(LOADING_TURN).repeat(),
            |ring, turn| ring.with_transformation(Transformation::rotate(percentage(turn))),
        )
}

/// Where the page goes in a window whose content is `viewport`: in the card, whose left edge is at
/// `card_left`, or filling the window for a full-screen page, at 0. The Library pushes the card
/// `push` further right at the same width, past the window's edge.
pub fn page_frame(viewport: Size<Pixels>, card_left: f32, push: f32) -> Bounds<Pixels> {
    if card_left == 0. {
        return Bounds::new(point(px(0.), px(0.)), viewport);
    }
    Bounds::new(
        point(px(card_left + push), px(CARD_INSET)),
        size(
            viewport.width - px(card_left + CARD_INSET),
            viewport.height - px(2. * CARD_INSET),
        ),
    )
}

/// The pages of a split view of `count` in `frame`, left to right, where the core puts them: side
/// by side, SPLIT_GAP apart, each as wide as the others to the point.
fn pane_frames(frame: Bounds<Pixels>, count: usize) -> Vec<Bounds<Pixels>> {
    let (x, span) = (f32::from(frame.origin.x), f32::from(frame.size.width) + SPLIT_GAP);
    let edge = |index: usize| x + (span * index as f32 / count as f32).floor();
    (0..count)
        .map(|index| {
            Bounds::from_corners(
                point(px(edge(index)), frame.top()),
                point(px(edge(index + 1) - SPLIT_GAP), frame.bottom()),
            )
        })
        .collect()
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

/// Where a spring from 0 to 1, at rest at 0, is `t` seconds in, by SwiftUI's response and damping
/// fraction. None once it's within a thousandth of 1 for good.
/// `way`, but past `low` or `high` only a little further, on the Space pages' rubber band.
fn rubber_band(way: f32, low: f32, high: f32) -> f32 {
    let band = |past: f32| 255. * (1. - 1. / (0.15 * past / 255. + 1.));
    match way {
        way if way > high => high + band(way - high),
        way if way < low => low - band(low - way),
        way => way,
    }
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

fn spring(t: f32, response: f32, damping: f32) -> Option<f32> {
    let omega = std::f32::consts::TAU / response;
    let decay = (-damping * omega * t).exp();
    if decay < 0.001 {
        return None;
    }
    let damped = omega * (1. - damping * damping).sqrt();
    Some(1. - decay * ((damped * t).cos() + damping * omega / damped * (damped * t).sin()))
}

/// Where `position` in a window is from the corner, `origin`, of the sidebar it draws.
fn in_sidebar(position: Point<Pixels>, origin: Point<Pixels>) -> Point<f32> {
    let at = position - origin;
    point(f32::from(at.x), f32::from(at.y))
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
        .pr(px(PLATE_INSET))
        .text_size(px(13.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(SPACE_NAME))
}

/// `text` for a Markdown link's text, with what would end it escaped.
fn markdown_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// The divider under the pinned tabs: a line, and `clear` at its end while it shows.
fn divider_box(clear: Option<impl IntoElement>) -> gpui::Div {
    let right = if clear.is_some() { 2. * PLATE_INSET } else { DIVIDER_X };
    div().flex_none().h(px(DIVIDER_HEIGHT)).child(
        div()
            .absolute()
            .top(px(DIVIDER_LINE_Y - CLEAR_HEIGHT / 2.))
            .left(px(DIVIDER_X))
            .right(px(right))
            .h(px(CLEAR_HEIGHT))
            .flex()
            .items_center()
            .child(div().flex_1().h(px(HAIRLINE)).bg(rgba(DIVIDER)))
            .children(clear),
    )
}

/// The divider's Clear: an arrow down and its label.
fn clear_label() -> gpui::Div {
    div()
        .flex_none()
        .ml(px(DIVIDER_GAP - CLEAR_ARROW_INSET))
        .h_full()
        .flex()
        .items_center()
        .gap(px(CLEAR_GAP - CLEAR_ARROW_INSET))
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(CLEAR))
        .child(
            svg()
                .path("icons/arrow-down.svg")
                .size(px(CLEAR_ARROW))
                .text_color(rgba(CLEAR)),
        )
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

/// Where a CSS cubic-bezier(`x1`, `y1`, `x2`, `y2`) curve is at `x`, from 0 to 1: the x of each
/// control point, then its y.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
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
