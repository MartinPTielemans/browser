//! The browser window's UI, under the page: the sidebar and the card the page sits in, at Arc's
//! measured sizes (docs/ux-spec.md › Window and › Sidebar). The shell shows what the browser tells
//! it and asks the browser for changes, so the engine stays the one place tabs live.

use std::{collections::HashMap, rc::Rc, time::Instant};

use gpui::{
    Action, App, Bounds, Context, Global, KeyBinding, Pixels, SharedString, Size, WeakEntity,
    Window, div, point, prelude::*, px, rgb, rgba, size,
};

/// The sidebar's width, where the card starts.
pub const SIDEBAR_WIDTH: f32 = 228.;
/// The card's inset from the window's top, right and bottom edges.
pub const CARD_INSET: f32 = 10.;
const CARD_RADIUS: f32 = 6.;

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
// Until the engine reports favicons.
const FAVICON: u32 = 0xffffff1f;

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
            Command::Quit => "quit",
        }
    }
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct Run(Command);

struct ShellHandle(WeakEntity<Shell>);

impl Global for ShellHandle {}

/// Binds the shortcuts for while the UI has the keyboard. The browser hands them over itself while
/// the page has it.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys(
        SHORTCUTS
            .iter()
            .map(|&(key, command, _)| KeyBinding::new(key, Run(command), None)),
    );
    cx.on_action(|run: &Run, cx| {
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell
                .update(cx, |shell, cx| shell.run(run.0, "ui", cx))
                .ok();
        }
    });
}

/// What the shell asks of the browser. The browser answers by updating the shell afterwards, never
/// during the call.
pub trait Browser {
    fn select(&self, tab: u64, cx: &mut App);
    fn close(&self, tab: u64, cx: &mut App);
    /// Opens the command bar, for a new tab or on `editing`'s URL.
    fn command_bar(&self, editing: Option<(u64, SharedString)>, cx: &mut App);
    fn quit(&self, cx: &mut App);
}

#[derive(Default)]
struct Tab {
    title: SharedString,
    url: SharedString,
}

#[derive(Clone, Copy, PartialEq)]
enum Row {
    NewTab,
    Tab(u64),
}

pub struct Shell {
    browser: Rc<dyn Browser>,
    /// The Space's Today tabs, top to bottom.
    today: Vec<u64>,
    tabs: HashMap<u64, Tab>,
    selected: u64,
    hovered: Option<Row>,
    page_fullscreen: bool,
    /// The tab the user asked for and when, until it's shown.
    switching: Option<(u64, Instant)>,
}

impl Shell {
    /// Makes the shell, the one the browser's shortcuts act on.
    pub fn new(browser: Rc<dyn Browser>, cx: &mut Context<Self>) -> Self {
        cx.set_global(ShellHandle(cx.weak_entity()));
        Self {
            browser,
            today: Vec::new(),
            tabs: HashMap::new(),
            selected: 0,
            hovered: None,
            page_fullscreen: false,
            switching: None,
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

    pub fn set_url(&mut self, tab: u64, url: SharedString, cx: &mut Context<Self>) {
        self.tabs.entry(tab).or_default().url = url;
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

    fn url(&self, tab: u64) -> SharedString {
        self.tabs.get(&tab).map(|tab| tab.url.clone()).unwrap_or_default()
    }

    /// A tab's title: the page's, else its host, else "Untitled" (Arc).
    fn title(&self, tab: u64) -> SharedString {
        match self.tabs.get(&tab) {
            Some(tab) if !tab.title.is_empty() => tab.title.clone(),
            Some(tab) if !tab.url.is_empty() => host(&tab.url).into(),
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
            .when(hovered, |label| label.child(close));
        let icon = div()
            .size(px(FAVICON_SIZE))
            .rounded(px(4.))
            .bg(rgba(FAVICON));
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

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card_left = if self.page_fullscreen { 0. } else { SIDEBAR_WIDTH };
        let card_inset = if self.page_fullscreen { 0. } else { CARD_INSET };
        let new_tab = self.new_tab_row(cx);
        let rows: Vec<_> = self.today.iter().map(|&tab| self.tab_row(tab, cx)).collect();
        div()
            .size_full()
            .relative()
            .font_family(".SystemUIFont")
            .bg(rgb(WINDOW))
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
                shell.child(self.url_field()).child(
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
