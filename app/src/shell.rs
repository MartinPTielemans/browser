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
    Action, Animation, AnimationExt, App, AssetSource, Bounds, Context, FocusHandle, Focusable,
    Global, Image, KeyBinding, MouseButton, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    SharedString, Size, Transformation, WeakEntity, Window, div, img, percentage, point,
    prelude::*, px, rgb, rgba, size, svg,
};

/// The sidebar's width, where the card starts.
pub const SIDEBAR_WIDTH: f32 = 228.;
/// The card's inset from the window's top, right and bottom edges.
pub const CARD_INSET: f32 = 10.;
const CARD_RADIUS: f32 = 6.;

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
// Arc puts favorites, the Space's name and pinned tabs between the URL field and "+ New Tab". This
// is their gap while there are none, which Arc has no layout for.
const LIST_TOP: f32 = URL_FIELD_TOP + URL_FIELD_HEIGHT + 8.;
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
// The header's glyphs (docs/ux-spec.md › Header and URL field): white 0.60, and 0.26 for back or
// forward with nowhere to go, over a hover plate of white 0.077.
const NAV: u32 = 0xffffff99;
const NAV_DISABLED: u32 = 0xffffff42;
const NAV_HOVERED: u32 = 0xffffff14;

/// A browser command, from a key or the sidebar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    NewTab,
    EditUrl,
    CloseTab,
    NextTab,
    PreviousTab,
    /// The tab at this index in sidebar order.
    SelectTab(usize),
    SelectLastTab,
    Find,
    Back,
    Forward,
    Reload,
    /// Reloads without the cache.
    ForceReload,
    Stop,
    Quit,
}

/// The browser's shortcuts (docs/ux-spec.md › Keyboard): the keystroke, with modifiers in the order
/// ctrl, alt, shift, cmd, its command, and whether it's reserved. As in Chrome, reserved ones reach
/// the browser before the page sees them, and the others only if the page doesn't keep them.
const SHORTCUTS: &[(&str, Command, bool)] = &[
    ("cmd-t", Command::NewTab, true),
    ("cmd-w", Command::CloseTab, true),
    ("cmd-q", Command::Quit, true),
    ("alt-cmd-down", Command::NextTab, true),
    ("alt-cmd-up", Command::PreviousTab, true),
    ("cmd-l", Command::EditUrl, false),
    ("cmd-f", Command::Find, false),
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
            Command::NextTab => "next-tab",
            Command::PreviousTab => "previous-tab",
            Command::SelectTab(_) => "select-tab",
            Command::SelectLastTab => "select-last-tab",
            Command::Find => "find",
            Command::Back => "back",
            Command::Forward => "forward",
            Command::Reload => "reload",
            Command::ForceReload => "force-reload",
            Command::Stop => "stop",
            Command::Quit => "quit",
        }
    }
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct Run(Command);

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
    cx.on_action(|run: &Run, cx| {
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell
                .update(cx, |shell, cx| shell.run(run.0, "ui", cx))
                .ok();
        }
    });
}

/// The shell's glyphs, built in. Lucide's (NOTICE).
pub struct Assets;

const ICONS: &[(&str, &[u8])] = &[
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
    fn close(&self, tab: u64, cx: &mut App);
    fn go_back(&self, tab: u64, cx: &mut App);
    fn go_forward(&self, tab: u64, cx: &mut App);
    fn reload(&self, tab: u64, bypass_cache: bool, cx: &mut App);
    fn stop(&self, tab: u64, cx: &mut App);
    /// Opens the command bar, for a new tab or on `editing`'s URL.
    fn command_bar(&self, editing: Option<(u64, SharedString)>, cx: &mut App);
    fn quit(&self, cx: &mut App);
}

/// Where a tab's page is, as the browser reports it.
#[derive(Clone, Default)]
pub struct Page {
    pub url: SharedString,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub loading: bool,
}

#[derive(Default)]
struct Tab {
    title: SharedString,
    page: Page,
    /// None for a page without one.
    favicon: Option<Arc<Image>>,
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
    /// The Space's Today tabs, top to bottom.
    today: Vec<u64>,
    tabs: HashMap<u64, Tab>,
    selected: u64,
    hovered: Option<Row>,
    page_fullscreen: bool,
    /// The tab the user asked for and when, until it's shown.
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
            today: Vec::new(),
            tabs: HashMap::new(),
            selected: 0,
            hovered: None,
            page_fullscreen: false,
            switching: None,
            background_pressed: false,
        }
    }

    pub fn today(&self) -> &[u64] {
        &self.today
    }

    pub fn selected(&self) -> u64 {
        self.selected
    }

    pub fn page_fullscreen(&self) -> bool {
        self.page_fullscreen
    }

    pub fn set_today(&mut self, today: Vec<u64>, cx: &mut Context<Self>) {
        // check.sh reads these lines.
        let ids: Vec<String> = today.iter().map(u64::to_string).collect();
        eprintln!("browser: today {}", ids.join(" "));
        // What the shell knew of a closed tab goes with it.
        self.tabs.retain(|tab, _| today.contains(tab));
        self.today = today;
        cx.notify();
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
        self.tabs.entry(tab).or_default().page = page;
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
        let position = self.today.iter().position(|&tab| tab == self.selected);
        let count = self.today.len();
        // Next and previous wrap around, as in Chrome.
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
            Command::Back => return self.browser.go_back(self.selected, cx),
            Command::Forward => return self.browser.go_forward(self.selected, cx),
            Command::Reload => return self.browser.reload(self.selected, false, cx),
            Command::ForceReload => return self.browser.reload(self.selected, true, cx),
            Command::Stop => return self.browser.stop(self.selected, cx),
            Command::Quit => return self.browser.quit(cx),
            // No find bar yet.
            Command::Find => return,
            Command::NextTab => position.map(|index| (index + 1) % count),
            Command::PreviousTab => position.map(|index| (index + count - 1) % count),
            Command::SelectTab(index) => (index < count).then_some(index),
            Command::SelectLastTab => count.checked_sub(1),
        };
        if let Some(index) = target {
            self.select(self.today[index], cx);
        }
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
        if !self.page_fullscreen && at.x < px(SIDEBAR_WIDTH) && at.y >= px(LIST_TOP) {
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
        let hovered = self.hovered == Some(Row::Tab(tab));
        let loading = self.page(tab).loading;
        let title = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(px(14.))
            .text_color(rgb(TITLE))
            .child(self.title(tab));
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
            .when(!hovered, |label| label.pr(px(PLATE_INSET)))
            .child(title)
            .when(hovered, |label| label.child(close))
            .when(!hovered && loading, |label| label.child(loading_ring(tab)));
        let icon = match self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()) {
            Some(favicon) => img(favicon).size(px(FAVICON_DRAWN)).into_any_element(),
            None => svg()
                .path("icons/globe.svg")
                .size(px(FAVICON_DRAWN))
                .text_color(rgba(SECONDARY))
                .into_any_element(),
        };
        self.row(Row::Tab(tab), icon, label, cx)
            .on_click(cx.listener(move |shell, _, _, cx| {
                eprintln!("browser: command select via sidebar");
                shell.select(tab, cx)
            }))
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card_left = if self.page_fullscreen { 0. } else { SIDEBAR_WIDTH };
        let card_inset = if self.page_fullscreen { 0. } else { CARD_INSET };
        let new_tab = self.new_tab_row(cx);
        let rows: Vec<_> = self.today.iter().map(|&tab| self.tab_row(tab, cx)).collect();
        let nav = self.nav_buttons(cx);
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
                shell.children(nav).child(self.url_field()).child(
                    div()
                        .id("today")
                        .absolute()
                        .top(px(LIST_TOP))
                        .left_0()
                        .w(px(SIDEBAR_WIDTH))
                        .bottom_0()
                        .flex()
                        .flex_col()
                        .overflow_y_scroll()
                        .child(new_tab)
                        .children(rows),
                )
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
