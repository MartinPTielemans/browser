//! The browser window's UI, under the page: the sidebar and the card the page sits in, at Arc's
//! measured sizes (docs/ux-spec.md › Window and › Sidebar). The shell shows what the browser tells
//! it and asks the browser for changes, so the engine stays the one place tabs live.

use std::{
    borrow::Cow,
    collections::HashMap,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Action, Animation, AnimationExt, App, AssetSource, Bounds, Context, Entity, FocusHandle,
    Focusable, FontWeight, Global, Image, KeyBinding, MenuItem, MouseButton, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, SharedString, Size, Task, Transformation, WeakEntity, Window,
    actions,
    div, img, percentage, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::text_input::TextInput;

/// The sidebar's width, where the card starts.
pub const SIDEBAR_WIDTH: f32 = 228.;
/// The card's inset from the window's top, right and bottom edges.
pub const CARD_INSET: f32 = 10.;
/// The radius of the card's corners and the page's in it: Arc's, a circle of 12 pixels in its
/// window at 2×.
pub const CARD_RADIUS: f32 = 6.;

// The header's buttons: their plates' tops and left edges, from the window's.
const NAV_TOP: f32 = 7.5;
const BACK_X: f32 = 122.;
const FORWARD_X: f32 = 156.;
const RELOAD_X: f32 = 190.;
const NAV_SIZE: f32 = 32.;
const NAV_RADIUS: f32 = 6.;
const NAV_GLYPH: f32 = 16.;

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
const PLATE_RADIUS: f32 = 12.;
const FAVICON_X: f32 = 17.;
const FAVICON_SIZE: f32 = 18.;
// The engine's favicons are 16 pt, drawn at 2×, so they sit in the middle of Arc's 18 pt slot
// rather than scaled up soft.
const FAVICON_DRAWN: f32 = 16.;
const LOADING_SIZE: f32 = 12.;
// One clockwise turn (docs/ux-spec.md › Motion).
const LOADING_TURN: Duration = Duration::from_millis(1880);
const TITLE_GAP: f32 = 8.;
const CLOSE_SIZE: f32 = 26.;
const CLOSE_X: f32 = 189.;
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

// Arc's dark blue Space, sampled from a capture. It stands in for the window material.
const WINDOW: u32 = 0x262337;
// Dia's card, #121212 at 0.5, over the window: what shows before a page first paints.
const CARD: u32 = 0x1c1a24;
// Arc's tokens (docs/ux-spec.md › Colour tokens), P3 #FAFBFF taken as sRGB.
const TITLE: u32 = 0xdedfff;
const SELECTED: u32 = 0xfafbff33;
const HOVERED: u32 = 0xfafbff1a;
const PRESSED: u32 = 0xfafbff11;
const SECONDARY: u32 = 0xffffff99;
const URL_FIELD: u32 = 0xffffff1a;
const NEW_TAB: u32 = 0xffffff73;
const NEW_TAB_PLUS: u32 = 0xffffff80;
// The Clear divider's line, white 0.08 (Arc), and its button (ours: Arc's colour isn't measured).
const DIVIDER: u32 = 0xffffff14;
const CLEAR: u32 = 0xffffff73;
// The toast: white 0.1 over the window, opaque over the rows it covers (ours: Arc's isn't
// measured).
const TOAST: u32 = 0x3c394b;
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
            Command::Quit => "quit",
        }
    }
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct Run(Command);

actions!(space_form, [CreateSpace]);

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
    cx.bind_keys([KeyBinding::new("enter", CreateSpace, Some("SpaceForm"))]);
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

/// The shell's glyphs, built in. Lucide's (NOTICE).
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
    ("icons/arrow-down.svg", include_bytes!("../assets/icons/arrow-down.svg")),
    ("icons/arrow-left.svg", include_bytes!("../assets/icons/arrow-left.svg")),
    ("icons/arrow-right.svg", include_bytes!("../assets/icons/arrow-right.svg")),
    ("icons/globe.svg", include_bytes!("../assets/icons/globe.svg")),
    ("icons/loading.svg", include_bytes!("../assets/icons/loading.svg")),
    ("icons/rotate-cw.svg", include_bytes!("../assets/icons/rotate-cw.svg")),
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
    /// Pins `tab` at the bottom of its Space's pinned tabs, or unpins it to the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App);
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
    /// Makes an empty Space named `name` after the others, and returns it.
    fn create_space(&self, name: &str, cx: &mut App) -> u64;
    fn rename_space(&self, space: u64, name: &str, cx: &mut App);
    /// Deletes a Space without tabs.
    fn delete_space(&self, space: u64, cx: &mut App);
    /// Gives the UI the keyboard, from the page if the page has it.
    fn focus_ui(&self, cx: &mut App);
    /// Opens the command bar, for a new tab or on `editing`'s URL.
    fn command_bar(&self, editing: Option<(u64, SharedString)>, cx: &mut App);
    /// Opens the tab's find bar with the keyboard.
    fn find(&self, tab: u64, cx: &mut App);
    /// Selects the tab's next match, or the one before, opening its find bar if it's closed.
    fn find_next(&self, tab: u64, forward: bool, cx: &mut App);
    fn quit(&self, cx: &mut App);
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

#[derive(Clone, Copy, PartialEq)]
enum Row {
    NewTab,
    Tab(u64),
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
    /// The tabs of every Space.
    tabs: HashMap<u64, Tab>,
    selected: u64,
    hovered: Option<Row>,
    /// The tab whose favicon is under the pointer, which is Back to Pinned URL while it's away.
    hovered_favicon: Option<u64>,
    toast: Option<Toast>,
    page_fullscreen: bool,
    /// The tab or Space the user asked for and when, until it's shown.
    switching: Option<(u64, Instant)>,
    /// A press on the window's background, until the pointer moves and it moves the window.
    background_pressed: bool,
}

impl Shell {
    /// Makes the shell, the one the browser's shortcuts act on.
    pub fn new(browser: Rc<dyn Browser>, cx: &mut Context<Self>) -> Self {
        cx.set_global(ShellHandle(cx.weak_entity()));
        Self {
            browser,
            focus_handle: cx.focus_handle(),
            spaces: Vec::new(),
            space: 0,
            form: None,
            focus_form: false,
            tabs: HashMap::new(),
            selected: 0,
            hovered: None,
            hovered_favicon: None,
            toast: None,
            page_fullscreen: false,
            switching: None,
            background_pressed: false,
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

    pub fn page_fullscreen(&self) -> bool {
        self.page_fullscreen
    }

    /// The Spaces, by id and name, in the switcher's order.
    pub fn set_spaces(&mut self, spaces: Vec<(u64, SharedString)>, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        let ids: Vec<String> = spaces.iter().map(|(id, _)| id.to_string()).collect();
        eprintln!("browser: spaces {}", ids.join(" "));
        let mut old = std::mem::take(&mut self.spaces);
        self.spaces = spaces
            .into_iter()
            .map(|(id, name)| {
                let Some(space) = old.iter_mut().find(|space| space.id == id) else {
                    eprintln!("browser: added space {id} {name}");
                    return Space {
                        id,
                        name,
                        pinned: Vec::new(),
                        today: Vec::new(),
                    };
                };
                if space.name != name {
                    eprintln!("browser: renamed space {id} {name}");
                }
                Space {
                    id,
                    name,
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
        self.space = space;
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
        entry.pinned = pinned;
        entry.today = today;
        let has_tabs = !entry.pinned.is_empty() || !entry.today.is_empty();
        if space == self.space {
            self.log_tabs();
        }
        // A tab opened in the form's Space makes it a Space like the others, under the name it has.
        if has_tabs && self.form.as_ref().is_some_and(|form| form.space == space) {
            self.form = None;
        }
        // What the shell knew of a closed tab goes with it.
        let spaces = &self.spaces;
        self.tabs.retain(|tab, _| {
            spaces
                .iter()
                .any(|space| space.pinned.contains(tab) || space.today.contains(tab))
        });
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
        cx.notify();
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
        let target = match command {
            Command::NewTab => return self.browser.command_bar(None, cx),
            Command::EditUrl => {
                let editing = (self.selected != 0).then(|| (self.selected, self.url(self.selected)));
                return self.browser.command_bar(editing, cx);
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
            Command::NewSpace => return self.new_space(cx),
            Command::SelectSpace(index) => return self.show_space_at(Some(index), cx),
            Command::NextSpace => return self.show_space_at(space.map(|index| index + 1), cx),
            Command::PreviousSpace => {
                return self.show_space_at(space.and_then(|index| index.checked_sub(1)), cx);
            }
            Command::Find => return self.browser.find(self.selected, cx),
            Command::FindNext => return self.browser.find_next(self.selected, true, cx),
            Command::FindPrevious => return self.browser.find_next(self.selected, false, cx),
            Command::NextTab => position.map(|index| (index + 1) % count),
            Command::PreviousTab => position.map(|index| (index + count - 1) % count),
            Command::SelectTab(index) => (index < count).then_some(index),
            Command::SelectLastTab => count.checked_sub(1),
        };
        if let Some(index) = target {
            self.select(tabs[index], cx);
        }
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
        let below_rows = at.x < px(SIDEBAR_WIDTH) && at.y >= px(LIST_TOP);
        if !self.page_fullscreen && self.form().is_none() && below_rows {
            self.run(Command::NewTab, "sidebar", cx);
        } else {
            window.titlebar_double_click();
        }
    }

    fn page(&self, tab: u64) -> Page {
        self.tabs.get(&tab).map(|tab| tab.page.clone()).unwrap_or_default()
    }

    fn url(&self, tab: u64) -> SharedString {
        self.page(tab).url
    }

    /// A tab's title: the page's, else its host, else "Untitled" (Arc).
    fn title(&self, tab: u64) -> SharedString {
        match self.tabs.get(&tab) {
            Some(tab) if !tab.title.is_empty() => tab.title.clone(),
            Some(tab) if !tab.page.url.is_empty() => host(&tab.page.url).into(),
            _ => "Untitled".into(),
        }
    }

    fn hover(&mut self, row: Row, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            self.hovered = Some(row);
        } else if self.hovered == Some(row) {
            self.hovered = None;
        }
        cx.notify();
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

    /// Back, forward, and reload, which is stop while the page loads.
    fn nav_buttons(&self, cx: &mut Context<Self>) -> [gpui::AnyElement; 3] {
        let page = self.page(self.selected);
        let reload = if page.loading {
            self.nav_button(RELOAD_X, "icons/x.svg", Command::Stop, true, cx)
                .into_any_element()
        } else {
            let shown = self.selected != 0;
            self.nav_button(RELOAD_X, "icons/rotate-cw.svg", Command::Reload, shown, cx)
                .into_any_element()
        };
        [
            self.nav_button(BACK_X, "icons/arrow-left.svg", Command::Back, page.can_go_back, cx)
                .into_any_element(),
            self.nav_button(
                FORWARD_X,
                "icons/arrow-right.svg",
                Command::Forward,
                page.can_go_forward,
                cx,
            )
            .into_any_element(),
            reload,
        ]
    }

    fn url_field(&self) -> impl IntoElement + use<> {
        let url = self.url(self.selected);
        div()
            .absolute()
            .top(px(URL_FIELD_TOP))
            .left(px(PLATE_INSET))
            .w(px(SIDEBAR_WIDTH - 2. * PLATE_INSET))
            .h(px(URL_FIELD_HEIGHT))
            .flex()
            .items_center()
            .pl(px(URL_FIELD_TEXT_X - PLATE_INSET))
            .pr(px(URL_FIELD_TEXT_X - PLATE_INSET))
            .rounded(px(PLATE_RADIUS))
            .block_mouse_except_scroll()
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
        let hovered = self.hovered == Some(row);
        let id = match row {
            Row::NewTab => ("new-tab", 0),
            Row::Tab(tab) => ("tab", tab),
        };
        div()
            .id(id)
            .group("row")
            .flex_none()
            .h(px(ROW_HEIGHT))
            .mb(px(ROW_PITCH - ROW_HEIGHT))
            .px(px(PLATE_INSET))
            .py(px((ROW_HEIGHT - PLATE_HEIGHT) / 2.))
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(row, *hovered, cx)
            }))
            .child(
                div()
                    .id("plate")
                    .size_full()
                    .flex()
                    .items_center()
                    .rounded(px(PLATE_RADIUS))
                    .when(selected, |plate| plate.bg(rgba(SELECTED)))
                    .when(!selected && hovered, |plate| plate.bg(rgba(HOVERED)))
                    .when(!selected, |plate| {
                        plate.group_active("row", |style| style.bg(rgba(PRESSED)))
                    })
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
                    .child(label),
            )
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
        let title = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(px(14.))
            .text_color(rgb(TITLE))
            .child(if back { "Back to Pinned URL".into() } else { self.title(tab) });
        // The close button shows under the pointer, and the title truncates before it.
        let close = div()
            .id(("close", tab))
            .flex_none()
            .ml(px(4.))
            .mr(px(SIDEBAR_WIDTH - PLATE_INSET - CLOSE_X - CLOSE_SIZE))
            .size(px(CLOSE_SIZE))
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
        let icon = match self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()) {
            _ if back => svg()
                .path("icons/arrow-left.svg")
                .size(px(FAVICON_DRAWN))
                .text_color(rgba(NAV))
                .into_any_element(),
            Some(favicon) => img(favicon).size(px(FAVICON_DRAWN)).into_any_element(),
            None => svg()
                .path("icons/globe.svg")
                .size(px(FAVICON_DRAWN))
                .text_color(rgba(SECONDARY))
                .into_any_element(),
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
            .on_click(cx.listener(move |shell, _, _, cx| {
                eprintln!("browser: command select via sidebar");
                shell.select(tab, cx)
            }))
    }

    fn name_row(&self) -> impl IntoElement + use<> {
        let name = self.shown_space().map(|space| space.name.clone()).unwrap_or_default();
        div()
            .id("space-name")
            .flex_none()
            .h(px(NAME_HEIGHT))
            .flex()
            .items_center()
            .pl(px(NAME_X))
            .pr(px(PLATE_INSET))
            .block_mouse_except_scroll()
            .text_size(px(13.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(SPACE_NAME))
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
                .w(px(SIDEBAR_WIDTH - 2. * FORM_INSET))
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
            .w(px(SIDEBAR_WIDTH))
            .bottom_0()
            .child(title)
            .child(field)
            .child(create)
            .child(cancel)
    }

    /// The line over Today, and Clear at its end, where the titles end.
    fn divider(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let clear = div()
            .id("clear")
            .flex_none()
            .ml(px(DIVIDER_GAP - CLEAR_ARROW_INSET))
            .h_full()
            .flex()
            .items_center()
            .gap(px(CLEAR_GAP - CLEAR_ARROW_INSET))
            .block_mouse_except_scroll()
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
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::Clear, "sidebar", cx)));
        div().flex_none().h(px(DIVIDER_HEIGHT)).relative().child(
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
                .w(px(SIDEBAR_WIDTH - 2. * PLATE_INSET))
                .bottom(px(FOOTER_HEIGHT))
                .h(px(PLATE_HEIGHT))
                .flex()
                .items_center()
                .justify_between()
                .px(px(URL_FIELD_TEXT_X - PLATE_INSET))
                .rounded(px(PLATE_RADIUS))
                .block_mouse_except_scroll()
                .bg(rgb(TOAST))
                .text_size(px(13.))
                .text_color(rgb(TITLE))
                .child(format!("Cleared {cleared} {tabs}"))
                .child(undo),
        )
    }

    fn new_tab_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let plus = div()
            .text_size(px(18.))
            .text_color(rgba(NEW_TAB_PLUS))
            .child("+");
        let label = div()
            .text_size(px(14.))
            .text_color(rgba(NEW_TAB))
            .child("New Tab");
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
        let card_left = if self.page_fullscreen { 0. } else { SIDEBAR_WIDTH };
        let card_inset = if self.page_fullscreen { 0. } else { CARD_INSET };
        let new_tab = self.new_tab_row(cx);
        let pinned: Vec<_> = self.pinned().iter().map(|&tab| self.tab_row(tab, cx)).collect();
        let divider = (!self.today().is_empty()).then(|| self.divider(cx));
        let rows: Vec<_> = self.today().iter().map(|&tab| self.tab_row(tab, cx)).collect();
        let nav = self.nav_buttons(cx);
        let switcher = self.switcher(cx);
        let toast = self.toast(cx);
        let form = self.form().map(|form| self.space_form(form, cx));
        div()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .size_full()
            .relative()
            .font_family(".SystemUIFont")
            .bg(rgb(WINDOW))
            // The rows, buttons and fields keep their presses. Everywhere else is the window's
            // background, which moves the window as a title bar does.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, _| shell.background_pressed = true),
            )
            .on_mouse_move(cx.listener(|shell, event: &MouseMoveEvent, window, _| {
                if shell.background_pressed && event.dragging() {
                    shell.background_pressed = false;
                    // check.sh reads this line.
                    eprintln!("browser: window move");
                    window.start_window_move();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseUpEvent, window, cx| {
                    if std::mem::take(&mut shell.background_pressed) && event.click_count == 2 {
                        shell.double_click_background(event.position, window, cx);
                    }
                }),
            )
            // The page view goes over the card, as a native view.
            .child(
                div()
                    .absolute()
                    .left(px(card_left))
                    .top(px(card_inset))
                    .right(px(card_inset))
                    .bottom(px(card_inset))
                    .rounded(px(CARD_RADIUS))
                    .bg(rgb(CARD)),
            )
            .when(!self.page_fullscreen, |shell| {
                let sidebar = shell.children(nav);
                match form {
                    Some(form) => sidebar.child(form),
                    None => sidebar.child(self.url_field()).child(
                        div()
                            .id("today")
                            .absolute()
                            .top(px(LIST_TOP))
                            .left_0()
                            .w(px(SIDEBAR_WIDTH))
                            .bottom(px(FOOTER_HEIGHT))
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .child(self.name_row())
                            .children(pinned)
                            .children(divider)
                            .child(new_tab)
                            .children(rows),
                    ),
                }
                .children(toast)
                .children(switcher)
            })
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

/// Where the page goes in a window whose content is `viewport`: in the card, or filling the window
/// while it's full screen.
pub fn page_frame(viewport: Size<Pixels>, fullscreen: bool) -> Bounds<Pixels> {
    if fullscreen {
        return Bounds::new(point(px(0.), px(0.)), viewport);
    }
    Bounds::new(
        point(px(SIDEBAR_WIDTH), px(CARD_INSET)),
        size(
            viewport.width - px(SIDEBAR_WIDTH + CARD_INSET),
            viewport.height - px(2. * CARD_INSET),
        ),
    )
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
