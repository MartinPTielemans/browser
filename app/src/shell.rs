//! The browser window's UI, under the page: the sidebar and the card the page sits in, at Arc's
//! measured sizes (docs/ux-spec.md › Window and › Sidebar). The shell shows what the browser tells
//! it and asks the browser for changes, so the engine stays the one place tabs live.

use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Action, Animation, AnimationExt, AnyElement, App, AssetSource, Bounds, Context, CursorStyle,
    DispatchPhase, Entity, FocusHandle, Focusable, FontWeight, Global, Hsla, Image, KeyBinding,
    MenuItem, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Rgba,
    ScrollDelta, ScrollEvent, SharedString, Size, Subscription, Task, TouchPhase, Transformation,
    WeakEntity, Window, actions, canvas, div, img, linear_color_stop, linear_gradient, percentage,
    point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::text_input::TextInput;

/// The docked sidebar's width at first, where the card starts.
pub const SIDEBAR_WIDTH: f32 = 228.;
/// The card's inset from the window's top, right and bottom edges.
pub const CARD_INSET: f32 = 10.;
/// The radius of the card's corners and the page's in it: Arc's, a circle of 12 pixels in its
/// window at 2×.
pub const CARD_RADIUS: f32 = 6.;

// ⌘S hides the sidebar along Arc's curve, an ease-out cubic of 100 ms, and shows it on Dia's
// spring (docs/ux-spec.md › Motion: Sidebar hide, and Sidebar hide/show, *interim*).
const SIDEBAR_HIDE: f32 = 0.1;
const SIDEBAR_SHOW_RESPONSE: f32 = 0.215;
const SIDEBAR_SHOW_DAMPING: f32 = 0.79;
// The sidebar's edge drags its width between Ardia's limits (docs/ux-spec.md › Sidebar › Width),
// rubber-banding past them as the Space pages do. Let go past one, it settles there as they settle,
// and a double-click on the edge settles it back at the width it starts at. The edge is the margin
// right of the rows.
const SIDEBAR_MIN: f32 = 160.;
const SIDEBAR_MAX: f32 = 400.;
const SIDEBAR_EDGE: f32 = 6.;

// The sidebar reveal (docs/ux-spec.md › Sidebar reveal): the hidden sidebar slides over the page as
// a panel when the pointer reaches the window's left edge, on Arc's spring, and slides away on an
// ease-in-out once the pointer is past the hold zone. Its window starts at the browser window's
// top-left corner, so a point is at the same place in both.
pub const REVEAL_INSET: f32 = 5.;
const REVEAL_HOLD: f32 = 24.;
// Fitted to Arc's recording (3.5 pt past rest at 120 ms); Ardia's 0.185 s / 0.77 overshoots 5 pt.
const REVEAL_IN_RESPONSE: f32 = 0.14;
const REVEAL_IN_DAMPING: f32 = 0.8;
const REVEAL_OUT: f32 = 0.15;
/// The panel's window at rest ends at its edge, and is this much wider while it moves, for the
/// spring's overshoot.
const REVEAL_OVERSHOOT: f32 = 8.;

// Rows make room and close it on Dia's curves, and a closed tab's row fades (docs/ux-spec.md ›
// Motion: Row slide, Row fade). A row's hover fill fades out on Row fill change.
const ROW_SLIDE: f32 = 0.14;
const ROW_FADE: f32 = 0.1;
const ROW_FILL: f32 = 0.3;
// A pressed row comes away once the pointer has moved this far (ours), and the rows around a
// dragged one make way critically damped (docs/ux-spec.md › Motion: Reorder).
const DRAG_START: f32 = 4.;
const REORDER_OMEGA: f32 = 90.;
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
// Arc puts favorites between the URL field and the Space's name. This is their gap while there are
// none, which Arc has no layout for.
const LIST_TOP: f32 = URL_FIELD_TOP + URL_FIELD_HEIGHT + 8.;
// The Space's name, over its tabs. Arc puts its icon at x 13, and Spaces have none yet.
const NAME_HEIGHT: f32 = 44.;
const NAME_X: f32 = 41.;
const ROW_PITCH: f32 = 41.;
const ROW_HEIGHT: f32 = 40.;
const PLATE_HEIGHT: f32 = 36.;
const PLATE_INSET: f32 = 8.;
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
// Dia's ≈ 22 pt hover square, its centre 18 from the plate's right edge as before.
const CLOSE_SIZE: f32 = 22.;
const CLOSE_RIGHT: f32 = 7.;
// The selected plate's inner line, which every plate keeps clear so that nothing moves.
const HAIRLINE: f32 = 0.5;
// The Clear divider, under the pinned tabs. Its line's centre is 25 pt above the centre of the first
// row under it, and it ends 7.5 pt before Clear's arrow.
const DIVIDER_HEIGHT: f32 = 15.;
const DIVIDER_LINE_Y: f32 = DIVIDER_HEIGHT - (25. - ROW_HEIGHT / 2.);
const DIVIDER_GAP: f32 = 7.5;
// Clear's arrow, 8 pt tall and 3 pt before its label (Arc). Lucide's arrow-down spans 14 of its 24.
const CLEAR_ARROW: f32 = 8. * 24. / 14.;
const CLEAR_ARROW_INSET: f32 = CLEAR_ARROW * 5. / 24.;
const CLEAR_GAP: f32 = 3.;
const CLEAR_HEIGHT: f32 = 20.;
// How long the toast after Clear offers Undo (ours: Arc's isn't measured).
const TOAST_TIME: Duration = Duration::from_secs(6);
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
// Dia's tokens (docs/ux-spec.md › Colour tokens). A row is white 0.16 hovered and 0.31 pressed,
// and selected it's #121212 at 0.5 inside a 0.5 pt hairline of white 0.15. Titles are white, at
// 0.78 unless selected.
const TITLE: u32 = 0xffffffff;
const UNSELECTED_TITLE: u32 = 0xffffffc7;
const SELECTED: u32 = 0x1212_1280;
const SELECTED_HAIRLINE: u32 = 0xffffff26;
const HOVERED: u32 = 0xffffff29;
const PRESSED: u32 = 0xffffff4f;
const SECONDARY: u32 = 0xffffff99;
// The close button's square under the pointer: Dia's subtle fill, white 0.12, at the header
// buttons' radius (ours: Dia's fill isn't measured).
const CLOSE_HOVERED: u32 = 0xffffff1f;
const URL_FIELD: u32 = 0xffffff1a;
const NEW_TAB: u32 = 0xffffff8c;
const NEW_TAB_PLUS: u32 = 0xffffff40;
// The Clear divider's line, white 0.08 (Arc), and its button (ours: Arc's colour isn't measured).
const DIVIDER: u32 = 0xffffff14;
const CLEAR: u32 = 0xffffff73;
// The toast: white 0.1 over the window, opaque over the rows it covers (ours: Arc's isn't
// measured).
const TOAST: u32 = 0xffffff1a;
// A tab whose page isn't loaded fades its favicon (ours: Arc's alpha isn't measured).
const SLEEPING_FAVICON: f32 = 0.5;
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
// The reveal panel's 0.5 pt light border: the selected row's hairline (ours: Arc's colour isn't
// measured).
const REVEAL_BORDER: u32 = 0xffffff26;
// The panel draws the window's buttons itself, since AppKit's stay in the browser window, under
// it: their centres from the panel's corner, as the docked sidebar's are from the window's, their
// size, and their colours, sampled from AppKit's in a capture of the browser window. The glyphs
// show while the pointer is over any of them, as AppKit's do.
const WINDOW_BUTTONS: [(f32, u32); 3] = [(20., 0xe0604f), (43., 0xf3bf12), (66., 0x58bb18)];
const WINDOW_BUTTON_Y: f32 = 24.;
const WINDOW_BUTTON_SIZE: f32 = 14.;
const WINDOW_BUTTON_GLYPH: u32 = 0x00000080;

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
    /// Loads the selected pinned tab's pinned URL.
    BackToPinnedUrl,
    /// Closes the Today tabs of the Space shown.
    Clear,
    Find,
    FindNext,
    FindPrevious,
    Back,
    Forward,
    Reload,
    /// Reloads without the cache.
    ForceReload,
    Stop,
    NewSpace,
    /// The Space at this index in the switcher.
    SelectSpace(usize),
    NextSpace,
    PreviousSpace,
    /// Hides the docked sidebar, or shows it.
    ToggleSidebar,
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
    ("cmd-s", Command::ToggleSidebar, false),
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
            Command::BackToPinnedUrl => "back-to-pinned-url",
            Command::Clear => "clear",
            Command::Find => "find",
            Command::FindNext => "find-next",
            Command::FindPrevious => "find-previous",
            Command::Back => "back",
            Command::Forward => "forward",
            Command::Reload => "reload",
            Command::ForceReload => "force-reload",
            Command::Stop => "stop",
            Command::NewSpace => "new-space",
            Command::SelectSpace(_) => "select-space",
            Command::NextSpace => "next-space",
            Command::PreviousSpace => "previous-space",
            Command::ToggleSidebar => "toggle-sidebar",
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
        KeyBinding::new("enter", KeepName, Some("SpaceName")),
        KeyBinding::new("escape", CancelName, Some("SpaceName")),
    ]);
    cx.on_action(|run: &Run, cx| {
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell
                .update(cx, |shell, cx| shell.run(run.0, "ui", cx))
                .ok();
        }
    });
}

/// A menu item that runs `command`, showing its shortcut.
pub fn menu_item(name: &'static str, command: Command) -> MenuItem {
    MenuItem::action(name, Run(command))
}

/// The shell's glyphs, built in. Lucide's (NOTICE), and the loading ring and the window buttons'
/// glyphs, which are ours.
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    ("icons/arrow-down.svg", include_bytes!("../assets/icons/arrow-down.svg")),
    ("icons/arrow-left.svg", include_bytes!("../assets/icons/arrow-left.svg")),
    ("icons/arrow-right.svg", include_bytes!("../assets/icons/arrow-right.svg")),
    ("icons/globe.svg", include_bytes!("../assets/icons/globe.svg")),
    ("icons/loading.svg", include_bytes!("../assets/icons/loading.svg")),
    ("icons/panel-left.svg", include_bytes!("../assets/icons/panel-left.svg")),
    ("icons/rotate-cw.svg", include_bytes!("../assets/icons/rotate-cw.svg")),
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
    /// Closes a Today tab. A pinned tab stays, and only its page goes.
    fn close(&self, tab: u64, cx: &mut App);
    /// Closes the Today tabs below `tab`, showing `tab` if one of them showed.
    fn close_below(&self, tab: u64, cx: &mut App);
    /// Opens a copy of `tab` with its history, under it in Today, and shows it.
    fn duplicate(&self, tab: u64, cx: &mut App);
    /// Moves `tab` to `space`. The window goes with it if it shows it.
    fn move_to_space(&self, tab: u64, space: u64, cx: &mut App);
    /// Pins `tab` at the bottom of its Space's pinned tabs, or unpins it to the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App);
    /// Moves `tab` to `index` in its Space's pinned tabs or Today, pinning or unpinning it.
    fn move_tab(&self, tab: u64, pinned: bool, index: usize, cx: &mut App);
    /// Loads a pinned tab's pinned URL, unless it's there already.
    fn back_to_pinned_url(&self, tab: u64, cx: &mut App);
    /// Closes the Today tabs of the Space shown.
    fn clear(&self, cx: &mut App);
    /// Brings back the tabs the last clear closed.
    fn undo_clear(&self, cx: &mut App);
    /// Reopens the Today tab closed last, and shows it.
    fn reopen_closed_tab(&self, cx: &mut App);
    fn go_back(&self, tab: u64, cx: &mut App);
    fn go_forward(&self, tab: u64, cx: &mut App);
    fn reload(&self, tab: u64, bypass_cache: bool, cx: &mut App);
    fn stop(&self, tab: u64, cx: &mut App);
    fn show_space(&self, space: u64, cx: &mut App);
    /// The tab the window shows when it shows `space`, or the one it shows, else 0.
    fn space_tab(&self, space: u64) -> u64;
    /// Makes an empty Space named `name` after the others, and returns it.
    fn create_space(&self, name: &str, cx: &mut App) -> u64;
    fn rename_space(&self, space: u64, name: &str, cx: &mut App);
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
    /// Lays the page out in a window whose content is `viewport`, with the card's left edge at
    /// `card_left` (`page_frame`).
    fn lay_out_page(&self, viewport: Size<Pixels>, card_left: f32, cx: &mut App);
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
    /// The tab is pinned, and its page isn't the one it was pinned at.
    pub away_from_pinned_url: bool,
}

/// The toast after Clear, which offers Undo until it hides.
struct Toast {
    cleared: usize,
    _hide: Task<()>,
}

#[derive(Default)]
struct Tab {
    title: SharedString,
    page: Page,
    /// None for a page without one.
    favicon: Option<Arc<Image>>,
}

struct Space {
    id: u64,
    name: SharedString,
    /// 0xRRGGBB, if it has one.
    color: Option<u32>,
    /// Its pinned tabs and its Today tabs, each top to bottom.
    pinned: Vec<u64>,
    today: Vec<u64>,
}

/// The Create a Space form, which a Space New Space made shows until it's named, cancelled or has
/// a tab.
struct SpaceForm {
    space: u64,
    /// The Space shown before, which Cancel shows again.
    from: u64,
    name: Entity<TextInput>,
}

/// The field the Space's name turns into for renaming it.
struct Renaming {
    space: u64,
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

/// A closed tab's row, fading where it was.
struct Closing {
    title: SharedString,
    favicon: Option<Arc<Image>>,
    top: f32,
    start: Instant,
}

/// How a row slides to its place.
#[derive(Clone, Copy)]
enum Curve {
    /// Making room for a tab's row or closing it (Row slide).
    Slide,
    /// Making way for a dragged row (Reorder).
    Reorder,
}

impl Curve {
    /// How far along the slide is `t` seconds in, or None once it's done.
    fn at(self, t: f32) -> Option<f32> {
        match self {
            Curve::Slide => {
                (t < ROW_SLIDE).then(|| cubic_bezier(0.25, 0.1, 0.25, 1., t / ROW_SLIDE))
            }
            Curve::Reorder => {
                let left = (1. + REORDER_OMEGA * t) * (-REORDER_OMEGA * t).exp();
                (left > 0.01).then_some(1. - left)
            }
        }
    }
}

/// A tab row the pointer pressed, which follows the pointer once it has moved far enough.
struct Drag {
    tab: u64,
    /// Where the pointer pressed the row, and the row's top in the list then.
    press: Point<Pixels>,
    top: f32,
    /// Whether the row has come away, and its top now: the pointer's way from `top`, held between
    /// the list's first and last places.
    moving: bool,
    at: f32,
    /// Where the row was and where it would go: in the pinned tabs or Today, and at which index of
    /// them without it.
    from: (bool, usize),
    to: (bool, usize),
    /// How far the other rows are drawn from their places to make room for it there, and whether
    /// the divider stays, which goes once the last Today tab would be pinned.
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
    /// The tabs of every Space.
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
    /// A tab's new row, which the rows sliding down to make room come out from beneath.
    opened: Option<u64>,
    closing: Vec<Closing>,
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
    /// The card's left edge the page was last laid out at.
    page_left: f32,
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
            tabs: HashMap::new(),
            recent: Vec::new(),
            walking: false,
            selected: 0,
            hovered: None,
            menu_row: None,
            unhovered: Vec::new(),
            slides: HashMap::new(),
            opened: None,
            closing: Vec::new(),
            drag: None,
            dragged: false,
            swipe: None,
            settle: None,
            swiped: false,
            wheel: None,
            wheel_paged: None,
            hovered_favicon: None,
            toast: None,
            page_fullscreen: false,
            sidebar_hidden: false,
            sidebar_width,
            resize: None,
            width_motion: None,
            sidebar_motion: None,
            revealed: false,
            reveal_motion: None,
            page_left: sidebar_width,
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

    pub fn space(&self) -> u64 {
        self.space
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
        if self.sidebar_hidden {
            (t < SIDEBAR_HIDE).then(|| 1. - (1. - t / SIDEBAR_HIDE).powi(3))
        } else {
            spring(t, SIDEBAR_SHOW_RESPONSE, SIDEBAR_SHOW_DAMPING)
        }
    }

    /// Hides the docked sidebar, or shows it, from wherever it is.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        // A full-screen page has no sidebar to hide.
        if self.page_fullscreen {
            return;
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
            // Core Animation's ease-in-out.
            (t < REVEAL_OUT).then(|| from * (1. - cubic_bezier(0.42, 0., 0.58, 1., t / REVEAL_OUT)))
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

    /// The window's look, opaque, halfway down: what covers the rows it passes over.
    fn window_fill(&self) -> Hsla {
        Hsla::from(rgb(MATERIAL_STAND_IN)).blend(self.gradient_color(0.5))
    }

    /// The pointer is at `x` in the browser window: at its left edge it brings the hidden sidebar
    /// out over the page, and more than the hold zone past the panel's edge it sends it away.
    pub fn pointer_at(&mut self, x: f32, cx: &mut Context<Self>) {
        if !self.sidebar_hidden || self.sidebar_motion.is_some() || self.page_fullscreen {
            return;
        }
        if x < CARD_INSET {
            self.set_revealed(true, cx);
        } else if x > REVEAL_INSET + self.width() + REVEAL_HOLD {
            self.set_revealed(false, cx);
        }
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
                }
            })
            .collect();
        if let Some(form) = &self.form
            && !self.spaces.iter().any(|space| space.id == form.space)
        {
            self.form = None;
        }
        if let Some(renaming) = &self.renaming
            && !self.spaces.iter().any(|space| space.id == renaming.space)
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
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.spaces.iter_mut().find(|entry| entry.id == space) else {
            return;
        };
        // The rows of the Space shown move from where they were, unless it's being filled.
        let had_tabs = !entry.pinned.is_empty() || !entry.today.is_empty();
        let before = (space == self.space && had_tabs).then(|| slots(&entry.pinned, &entry.today));
        entry.pinned = pinned;
        entry.today = today;
        let has_tabs = !entry.pinned.is_empty() || !entry.today.is_empty();
        if space == self.space {
            self.log_tabs();
        }
        if let Some(before) = before {
            self.move_rows(before);
        }
        // A dragged row lands, or goes back if the lists changed under it.
        if space == self.space {
            self.drag = None;
        }
        // A tab opened in the form's Space makes it a Space like the others, under the name it has.
        if has_tabs && self.form.as_ref().is_some_and(|form| form.space == space) {
            self.form = None;
        }
        // What the shell knew of a closed tab goes with it.
        let spaces = &self.spaces;
        let open = |tab: &u64| {
            spaces
                .iter()
                .any(|space| space.pinned.contains(tab) || space.today.contains(tab))
        };
        self.tabs.retain(|tab, _| open(tab));
        self.recent.retain(open);
        cx.notify();
    }

    fn log_tabs(&self) {
        // check.sh reads these lines, the pinned one first.
        let ids = |tabs: &[u64]| tabs.iter().map(u64::to_string).collect::<Vec<_>>().join(" ");
        eprintln!("browser: pinned {}", ids(self.pinned()));
        eprintln!("browser: today {}", ids(self.today()));
    }

    /// The Space's tabs in the sidebar's order: pinned, then Today.
    fn sidebar_tabs(&self) -> Vec<u64> {
        [self.pinned(), self.today()].concat()
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
        if !self.walking {
            self.shown(tab);
        }
        cx.notify();
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
        }
        cx.notify();
    }

    /// Runs a browser command and logs it. `from` says which side had the key: GPUI's bindings
    /// ("ui"), the browser handing it back ("page"), or the sidebar.
    pub fn run(&mut self, command: Command, from: &str, cx: &mut Context<Self>) {
        // check.sh waits for these lines.
        eprintln!("browser: command {} via {from}", command.name());
        let tabs = self.sidebar_tabs();
        let position = tabs.iter().position(|&tab| tab == self.selected);
        let count = tabs.len();
        let space = self.spaces.iter().position(|space| space.id == self.space);
        // Next and previous wrap around for tabs, as in Chrome, and stop at the ends for Spaces, as
        // in Arc.
        // Escape puts back a dragged row before it stops the page (docs/ux-spec.md › Tab list).
        if matches!(command, Command::Stop) && self.cancel_drag(cx) {
            return;
        }
        // The command bar opens over the page, and the reveal's panel makes way for it.
        if matches!(command, Command::NewTab | Command::EditUrl) {
            self.set_revealed(false, cx);
        }
        let target = match command {
            Command::NewTab => return self.browser.command_bar(None, None, cx),
            Command::EditUrl => {
                let editing = (self.selected != 0).then(|| (self.selected, self.url(self.selected)));
                return self.browser.command_bar(editing, self.url_field_bounds(), cx);
            }
            Command::CloseTab => {
                if self.selected != 0 {
                    self.browser.close(self.selected, cx);
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
            Command::BackToPinnedUrl => return self.browser.back_to_pinned_url(self.selected, cx),
            Command::Clear => return self.clear(cx),
            Command::ReopenClosedTab => return self.browser.reopen_closed_tab(cx),
            Command::Back => return self.browser.go_back(self.selected, cx),
            Command::Forward => return self.browser.go_forward(self.selected, cx),
            Command::Reload => return self.browser.reload(self.selected, false, cx),
            Command::ForceReload => return self.browser.reload(self.selected, true, cx),
            Command::Stop => return self.browser.stop(self.selected, cx),
            Command::Quit => return self.browser.quit(cx),
            Command::ToggleSidebar => return self.toggle_sidebar(cx),
            Command::NewSpace => return self.new_space(cx),
            Command::SelectSpace(index) => return self.show_space_at(Some(index), cx),
            Command::NextSpace => return self.show_space_at(space.map(|index| index + 1), cx),
            Command::PreviousSpace => {
                return self.show_space_at(space.and_then(|index| index.checked_sub(1)), cx);
            }
            Command::Find => return self.browser.find(self.selected, cx),
            Command::FindNext => return self.browser.find_next(self.selected, true, cx),
            Command::FindPrevious => return self.browser.find_next(self.selected, false, cx),
            Command::RecentTab => return self.walk(tabs, true, cx),
            Command::RecentTabBack => return self.walk(tabs, false, cx),
            Command::NextTab => position.map(|index| (index + 1) % count),
            Command::PreviousTab => position.map(|index| (index + count - 1) % count),
            Command::SelectTab(index) => (index < count).then_some(index),
            Command::SelectLastTab => count.checked_sub(1),
        };
        if let Some(index) = target {
            self.select(tabs[index], cx);
        }
    }

    /// ⌃Tab, as in Arc: shows the Space's tab shown before this one at once, with no switcher.
    /// While ⌃ is held, each ⌃Tab goes on to the one shown before that, and ⌃⇧Tab back.
    fn walk(&mut self, tabs: Vec<u64>, forward: bool, cx: &mut Context<Self>) {
        // Then the tabs not shown yet, in the sidebar's order.
        let mut order: Vec<u64> =
            self.recent.iter().copied().filter(|tab| tabs.contains(tab)).collect();
        order.extend(tabs.into_iter().filter(|tab| !self.recent.contains(tab)));
        let count = order.len();
        let next = match order.iter().position(|&tab| tab == self.selected) {
            Some(at) if forward => (at + 1) % count,
            Some(at) => (at + count - 1) % count,
            // With no tab shown, the tab shown last (Arc).
            None if forward && count > 0 => 0,
            None => match count.checked_sub(1) {
                Some(last) => last,
                None => return,
            },
        };
        if order[next] == self.selected {
            return;
        }
        if !self.walking {
            self.walking = true;
            self.browser.watch_control(cx);
        }
        self.select(order[next], cx);
    }

    /// Clears the Space shown, with a toast that offers Undo for a while.
    fn clear(&mut self, cx: &mut Context<Self>) {
        let cleared = self.today().len();
        if cleared == 0 {
            return;
        }
        self.browser.clear(cx);
        // check.sh reads these lines.
        eprintln!("browser: toast cleared {cleared}");
        let hide = cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(TOAST_TIME).await;
            shell.update(cx, |shell, cx| shell.hide_toast(cx)).ok();
        });
        self.toast = Some(Toast {
            cleared,
            _hide: hide,
        });
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
        type Action = Box<dyn FnOnce(&mut Shell, &mut Context<Shell>)>;
        let pinned = self.pinned().contains(&tab);
        let page = self.page(tab);
        let below = self.today().iter().skip_while(|&&id| id != tab).count() > 1;
        let others: Vec<(u64, SharedString)> = self
            .spaces
            .iter()
            .filter(|space| space.id != self.space)
            .map(|space| (space.id, space.name.clone()))
            .collect();
        let pin = MenuEntry::new(if pinned { "Unpin Tab" } else { "Pin Tab" }, true);
        let mut items: Vec<(MenuEntry, &'static str, Action)> = vec![(
            pin.shortcut(Command::TogglePin),
            "toggle-pin",
            Box::new(move |shell, cx| {
                let pinned = shell.pinned().contains(&tab);
                shell.browser.set_pinned(tab, !pinned, cx)
            }),
        )];
        if pinned {
            items.push((
                MenuEntry::new("Back to Pinned URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ));
        }
        items.push((
            MenuEntry::new("Duplicate Tab", true),
            "duplicate-tab",
            Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
        ));
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
            Box::new(move |shell, cx| shell.browser.close(tab, cx)),
        ));
        if !pinned {
            items.push((
                MenuEntry::new("Close Tabs Below", below),
                "close-tabs-below",
                Box::new(move |shell, cx| shell.browser.close_below(tab, cx)),
            ));
        }
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
                        if shell.hovered != Some(Row::Tab(tab)) {
                            shell.unhovered.push((Row::Tab(tab), Instant::now()));
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
        self.menu_row = Some(tab);
        cx.notify();
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
            space,
            name,
            _blur: None,
        });
        // As for the form.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Renames the Space to what the field holds, unless it's empty, and closes the field.
    fn keep_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let name = renaming.name.read(cx).text().trim().to_string();
        if !name.is_empty() {
            self.browser.rename_space(renaming.space, &name, cx);
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
        let below_rows = at.x < px(self.card_left()) && at.y >= px(LIST_TOP);
        if !self.page_fullscreen && self.form().is_none() && below_rows {
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
    /// there slide, a new tab's row is in its place at once, and a closed one fades where it was
    /// (docs/ux-spec.md › Tab rows).
    fn move_rows(&mut self, before: HashMap<Slot, f32>) {
        let now = Instant::now();
        let after = slots(self.pinned(), self.today());
        let slides = after
            .iter()
            .filter_map(|(&slot, &top)| {
                let from = before.get(&slot)? + self.slide(slot) - top;
                (from != 0.).then_some((slot, (from, now, Curve::Slide)))
            })
            .collect();
        let opened = after
            .iter()
            .filter(|(slot, _)| matches!(slot, Slot::Tab(_)) && !before.contains_key(slot))
            .min_by(|(_, a), (_, b)| a.total_cmp(b));
        if let Some((&Slot::Tab(tab), _)) = opened {
            self.opened = Some(tab);
        }
        for (&slot, &top) in &before {
            let Slot::Tab(tab) = slot else { continue };
            if !after.contains_key(&slot) {
                self.closing.push(Closing {
                    title: self.title(tab),
                    favicon: self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()),
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
            return drag.at - drag.top;
        }
        let room = drag.and_then(|drag| drag.room.get(&slot)).copied().unwrap_or(0.);
        room + self.slides.get(&slot).map_or(0., |&(from, start, curve)| {
            curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| from * (1. - done))
        })
    }

    /// The row the pointer drags, once it has come away.
    fn dragging(&self) -> Option<Slot> {
        self.drag.as_ref().filter(|drag| drag.moving).map(|drag| Slot::Tab(drag.tab))
    }

    /// The pointer pressed `tab`'s row at `position`, which may start a drag.
    fn press_row(&mut self, tab: u64, position: Point<Pixels>) {
        if self.drag.as_ref().is_some_and(|drag| drag.dropped) {
            return;
        }
        let pinned = self.pinned().contains(&tab);
        let tabs = if pinned { self.pinned() } else { self.today() };
        let Some(index) = tabs.iter().position(|&other| other == tab) else {
            return;
        };
        let top = slots(self.pinned(), self.today())[&Slot::Tab(tab)];
        self.dragged = false;
        self.drag = Some(Drag {
            tab,
            press: position,
            top,
            moving: false,
            at: top,
            from: (pinned, index),
            to: (pinned, index),
            room: HashMap::new(),
            divider: true,
            dropped: false,
        });
    }

    /// The pointer pressing a row moved to `position`. Past DRAG_START the row comes away and
    /// follows it up and down, and the other rows make room for it at the place, in the pinned
    /// tabs or Today, nearest where it's drawn.
    fn drag_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_ref().filter(|drag| !drag.dropped) else {
            return;
        };
        let moved = position - drag.press;
        if !drag.moving && moved.x.abs().max(moved.y.abs()) < px(DRAG_START) {
            return;
        }
        let (tab, top, to) = (drag.tab, drag.top + f32::from(moved.y), drag.to);
        let others = |tabs: &[u64]| tabs.iter().copied().filter(|&other| other != tab).collect();
        let (pinned, today): (Vec<u64>, Vec<u64>) = (others(self.pinned()), others(self.today()));
        let places = (0..=pinned.len()).map(|index| (true, index));
        let places = places.chain((0..=today.len()).map(|index| (false, index)));
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
        let at = top.clamp(first.min(last), first.max(last));
        let Some((place, _, layout)) =
            layouts.into_iter().min_by(|a, b| (a.1 - at).abs().total_cmp(&(b.1 - at).abs()))
        else {
            return;
        };
        if place != to {
            // The rows that make room, or no longer need to, start from where they're drawn.
            let now = Instant::now();
            let room: HashMap<Slot, f32> = slots(self.pinned(), self.today())
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
            if let Some(drag) = &mut self.drag {
                drag.to = place;
                drag.room = room;
                drag.divider = layout.contains_key(&Slot::Divider);
            }
        }
        if let Some(drag) = &mut self.drag {
            drag.moving = true;
            drag.at = at;
        }
        cx.notify();
    }

    /// The pointer let go of a pressed row. A dragged one goes where it would, and the rows settle
    /// once the browser has moved its tab.
    fn drop_row(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut().filter(|drag| !drag.dropped) else {
            return;
        };
        if !drag.moving {
            self.drag = None;
            return;
        }
        self.dragged = true;
        let (tab, (pinned, index)) = (drag.tab, drag.to);
        if drag.to == drag.from {
            // Nothing moves in the browser, so the rows settle from where they're drawn.
            self.move_rows(slots(self.pinned(), self.today()));
            self.drag = None;
        } else {
            drag.dropped = true;
            // check.sh waits for this line.
            let list = if pinned { "pinned" } else { "today" };
            eprintln!("browser: dragged {tab} to {list} {index}");
            self.browser.move_tab(tab, pinned, index, cx);
        }
        cx.notify();
    }

    /// Puts the row the pointer drags back where it was, and returns whether there was one.
    fn cancel_drag(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.drag.as_ref().is_some_and(|drag| drag.moving && !drag.dropped) {
            return false;
        }
        // The release that ends the drag isn't a click.
        self.dragged = true;
        self.move_rows(slots(self.pinned(), self.today()));
        self.drag = None;
        cx.notify();
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

    /// The pointer's moves and release anywhere in the window, for a pressed row, which only hears
    /// the pointer while it's over the row.
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
        self.slides
            .retain(|_, (_, start, curve)| curve.at(start.elapsed().as_secs_f32()).is_some());
        self.closing.retain(|row| row.start.elapsed().as_secs_f32() < ROW_FADE);
        self.unhovered.retain(|(_, start)| start.elapsed().as_secs_f32() < ROW_FILL);
        if self.slides.is_empty() {
            self.opened = None;
        }
        if let Some(settle) = self.settle.take_if(|settle| settle.offset().is_none()) {
            // check.sh reads these lines.
            eprintln!("browser: settled space {}", settle.space);
        }
        !(self.slides.is_empty() && self.closing.is_empty() && self.unhovered.is_empty())
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
        (docked && self.form().is_none()).then(|| {
            Bounds::new(
                point(px(PLATE_INSET), px(URL_FIELD_TOP)),
                size(px(self.width() - 2. * PLATE_INSET), px(URL_FIELD_HEIGHT)),
            )
        })
    }

    fn url_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let url = self.url(self.selected);
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
            .pr(px(URL_FIELD_TEXT_X - PLATE_INSET))
            .rounded(px(PLATE_RADIUS))
            .block_mouse_except_scroll()
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::EditUrl, "sidebar", cx)))
            .bg(rgba(URL_FIELD))
            .text_size(px(13.))
            .text_color(rgba(SECONDARY))
            .child(div().truncate().child(SharedString::from(host(&url))))
    }

    /// A sidebar row: the plate, with the favicon slot and the label.
    fn row(
        &self,
        row: Row,
        icon: impl IntoElement,
        label: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let selected = matches!(row, Row::Tab(tab) if tab == self.selected);
        let (id, slot) = match row {
            Row::NewTab => (("new-tab", 0), Slot::NewTab),
            Row::Tab(tab) => (("tab", tab), Slot::Tab(tab)),
        };
        // Full under the pointer or with its menu up, and fading once it's left. Full for a dragged
        // row, which the pointer holds, and none for the others then.
        let dragging = self.dragging();
        let menu = self.menu_row.is_some_and(|tab| row == Row::Tab(tab));
        let hover = if let Some(dragged) = dragging {
            (dragged == slot).then_some(1.)
        } else if self.hovered == Some(row) || menu {
            Some(1.)
        } else {
            self.unhovered.iter().find(|&&(old, _)| old == row).map(|(_, start)| {
                let t = (start.elapsed().as_secs_f32() / ROW_FILL).min(1.);
                1. - cubic_bezier(0., 0., 0.58, 1., t)
            })
        };
        row_box()
            .id(id)
            .group("row")
            // A dragged row is drawn over the list, from `list`.
            .top(px(if dragging == Some(slot) { 0. } else { self.slide(slot) }))
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(row, *hovered, cx)
            }))
            .child(
                plate(icon, label)
                    .when(selected, selected_plate)
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

    /// A closed tab's row where it was, fading, without the selected fill it may have had.
    fn closing_row(&self, row: &Closing) -> impl IntoElement + use<> {
        let t = (row.start.elapsed().as_secs_f32() / ROW_FADE).min(1.);
        div()
            .absolute()
            .top(px(row.top))
            .left_0()
            .w(px(self.width()))
            .h(px(ROW_HEIGHT))
            .px(px(PLATE_INSET))
            .py(px((ROW_HEIGHT - PLATE_HEIGHT) / 2.))
            .opacity(1. - cubic_bezier(0., 0., 0.58, 1., t))
            .child(plate(
                favicon(row.favicon.clone()),
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

    fn tab_row(&self, tab: u64, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let page = self.page(tab);
        let loading = page.loading;
        // A pinned tab closed already has nothing to close.
        let closable = !(page.unloaded && self.pinned().contains(&tab));
        let hovered = self.hovered == Some(Row::Tab(tab));
        // Away from its pinned URL, a pinned tab's favicon is a way back.
        let away = page.away_from_pinned_url;
        let back = away && self.hovered_favicon == Some(tab);
        let label = if back { "Back to Pinned URL".into() } else { self.title(tab) };
        let title = title(label, tab == self.selected);
        // The close button shows under the pointer, and the title truncates before it.
        let close = div()
            .id(("close", tab))
            .flex_none()
            .ml(px(4.))
            .mr(px(CLOSE_RIGHT - HAIRLINE))
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
                shell.browser.close(tab, cx);
            }));
        let label = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .when(!(hovered && closable), |label| label.pr(px(PLATE_INSET)))
            .child(title)
            .when(hovered && closable, |label| label.child(close))
            .when(!hovered && loading, |label| label.child(loading_ring(tab)));
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
        self.row(Row::Tab(tab), icon, label, cx)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, _| {
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
            .on_click(cx.listener(move |shell, _, _, cx| {
                if shell.dragged || shell.dragging().is_some() {
                    return;
                }
                eprintln!("browser: command select via sidebar");
                shell.select(tab, cx)
            }))
    }

    fn name_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let space = self.space;
        let row = name_box().id("space-name").block_mouse_except_scroll();
        if let Some(renaming) = self.renaming.as_ref().filter(|renaming| renaming.space == space) {
            return row
                .key_context("SpaceName")
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
        let clear = clear_label()
            .id("clear")
            .block_mouse_except_scroll()
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::Clear, "sidebar", cx)));
        divider_box(clear).top(px(self.slide(Slot::Divider)))
    }

    /// The toast after Clear, over the bottom of the list, with Undo.
    fn toast(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let cleared = self.toast.as_ref()?.cleared;
        let tabs = if cleared == 1 { "tab" } else { "tabs" };
        let undo = div()
            .id("undo")
            .h_full()
            .flex()
            .items_center()
            .font_weight(FontWeight::SEMIBOLD)
            .child("Undo")
            .on_click(cx.listener(|shell, _, _, cx| {
                // check.sh waits for this line.
                eprintln!("browser: command undo-clear via sidebar");
                shell.hide_toast(cx);
                shell.browser.undo_clear(cx);
            }));
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
                .justify_between()
                .px(px(URL_FIELD_TEXT_X - PLATE_INSET))
                .rounded(px(PLATE_RADIUS))
                .block_mouse_except_scroll()
                .bg(self.window_fill().blend(rgba(TOAST).into()))
                .text_size(px(13.))
                .text_color(rgba(TITLE))
                .child(format!("Cleared {cleared} {tabs}"))
                .child(undo),
        )
    }

    fn new_tab_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (plus, label) = new_tab_label();
        self.row(Row::NewTab, plus, label, cx)
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
        if std::mem::take(&mut self.focus_form)
            && let Some(form) = self.form()
        {
            window.focus(&form.name.focus_handle(cx), cx);
        }
        if std::mem::take(&mut self.focus_renaming)
            && let Some(renaming) = &mut self.renaming
        {
            let field = renaming.name.focus_handle(cx);
            window.focus(&field, cx);
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
        if self.list_moving() {
            window.request_animation_frame();
        }
        // The page is laid out at every step, as in Arc.
        let card_left = self.card_left();
        if card_left != self.page_left {
            self.page_left = card_left;
            self.browser.lay_out_page(window.viewport_size(), card_left, cx);
        }
        let card_inset = if self.page_fullscreen { 0. } else { CARD_INSET };
        // The sidebar leaves with the card's edge, all of it by the time the edge stops.
        let width = self.width();
        let sidebar_left = (card_left - width) * width / (width - CARD_INSET);
        let sidebar_shown =
            !self.page_fullscreen && (!self.sidebar_hidden || self.sidebar_motion.is_some());
        let sidebar = sidebar_shown.then(|| self.sidebar(cx).left(px(sidebar_left)));
        let docked = !self.page_fullscreen && !self.sidebar_hidden && self.sidebar_motion.is_none();
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
            // The page view goes over the card, as a native view.
            .child(
                div()
                    .absolute()
                    .left(px(card_left))
                    .top(px(card_inset))
                    .right(px(card_inset))
                    .bottom(px(card_inset))
                    .rounded(px(CARD_RADIUS))
                    .bg(rgba(CARD)),
            )
            .children(sidebar)
            .children(edge)
    }
}

impl Shell {
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
            None => sidebar.child(self.url_field(cx)).child(self.space_pages(cx)),
        };
        sidebar
            .children(self.toast(cx))
            .children(self.switcher(cx))
            .child(self.scroll_listener(cx))
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
            .top(px(LIST_TOP))
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
            let icon = favicon(self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()));
            let icon = div()
                .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
                .child(icon);
            row_box().child(
                plate(icon, title(self.title(tab), tab == selected).pr(px(PLATE_INSET)))
                    .when(tab == selected, selected_plate),
            )
        };
        let (plus, new_tab) = new_tab_label();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(name_box().child(div().truncate().child(space.name.clone())))
            .children(space.pinned.iter().map(|&tab| row(tab)))
            .when(!space.today.is_empty(), |page| page.child(divider_box(clear_label())))
            .child(row_box().child(plate(plus, new_tab)))
            .children(space.today.iter().map(|&tab| row(tab)))
    }

    /// The Space's name, its pinned tabs, the divider, New Tab and its Today tabs, under the URL
    /// field. Closed tabs' rows fade under the rest, and while rows slide down to make room for a
    /// tab's new row, they come out from beneath it.
    fn list(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut rows = vec![(None, self.name_row(cx).into_any_element())];
        for &tab in self.pinned() {
            rows.push((Some(tab), self.list_row(tab, cx)));
        }
        if !self.today().is_empty() {
            let divider = match self.drag.as_ref().is_some_and(|drag| drag.moving && !drag.divider) {
                true => div().flex_none().h(px(DIVIDER_HEIGHT)).into_any_element(),
                false => self.divider(cx).into_any_element(),
            };
            rows.push((None, divider));
        }
        rows.push((None, self.new_tab_row(cx).into_any_element()));
        for &tab in self.today() {
            rows.push((Some(tab), self.list_row(tab, cx)));
        }
        // Over the rest, where the pointer has it.
        let dragged = self.drag.as_ref().filter(|drag| drag.moving).map(|drag| {
            let row = self.tab_row(drag.tab, cx);
            div().absolute().top(px(drag.at)).left_0().w(px(self.width())).child(row)
        });
        let under = self
            .opened
            .and_then(|opened| rows.iter().position(|&(tab, _)| tab == Some(opened)))
            .map_or(rows.len(), |row| row + 1);
        let under = rows.split_off(under).into_iter().map(|(_, row)| row).collect::<Vec<_>>();
        div()
            .id("today")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
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
                    let below_rows = event.position.y >= px(REVEAL_INSET + LIST_TOP);
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
/// `card_left`, or filling the window for a full-screen page, at 0.
pub fn page_frame(viewport: Size<Pixels>, card_left: f32) -> Bounds<Pixels> {
    if card_left == 0. {
        return Bounds::new(point(px(0.), px(0.)), viewport);
    }
    Bounds::new(
        point(px(card_left), px(CARD_INSET)),
        size(
            viewport.width - px(card_left + CARD_INSET),
            viewport.height - px(2. * CARD_INSET),
        ),
    )
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

fn spring(t: f32, response: f32, damping: f32) -> Option<f32> {
    let omega = std::f32::consts::TAU / response;
    let decay = (-damping * omega * t).exp();
    if decay < 0.001 {
        return None;
    }
    let damped = omega * (1. - damping * damping).sqrt();
    Some(1. - decay * ((damped * t).cos() + damping * omega / damped * (damped * t).sin()))
}

/// Where a CSS cubic-bezier(`x1`, `y1`, `x2`, `y2`) curve is at `x`, from 0 to 1: the x of each
/// control point, then its y.
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

/// The divider under the pinned tabs: a line, then `clear`.
fn divider_box(clear: impl IntoElement) -> gpui::Div {
    div().flex_none().h(px(DIVIDER_HEIGHT)).child(
        div()
            .absolute()
            .top(px(DIVIDER_LINE_Y - CLEAR_HEIGHT / 2.))
            .left(px(FAVICON_X))
            .right(px(2. * PLATE_INSET))
            .h(px(CLEAR_HEIGHT))
            .flex()
            .items_center()
            .child(div().flex_1().h(px(1.)).bg(rgba(DIVIDER)))
            .child(clear),
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
        .border(px(HAIRLINE))
        .child(
            div()
                .flex_none()
                .ml(px(FAVICON_X - PLATE_INSET - HAIRLINE))
                .mr(px(TITLE_GAP))
                .size(px(FAVICON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .child(icon),
        )
        .child(label)
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

/// The selected tab's plate.
fn selected_plate(plate: gpui::Div) -> gpui::Div {
    plate.bg(rgba(SELECTED)).border_color(rgba(SELECTED_HAIRLINE))
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
