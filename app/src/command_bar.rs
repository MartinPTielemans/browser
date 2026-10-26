//! The command bar. GPUI can't draw over the page's native view, so the bar is a window of its
//! own that the host shows over the browser window, as its child. Under its input it lists the
//! core's suggestions for what's typed, and the window grows down to fit them. While the first is
//! selected, the input completes what's typed to it, as Chrome's address bar does. Opened from the
//! URL field, it grows out of the field, as Dia's does (docs/ux-spec.md › Command bar). Tab on a
//! row that hints a search engine, or a space after the engine's keyword, searches only with that
//! engine until ⌫ leaves, as Chrome's keyword mode does. What it asks of the core and of its window
//! goes through `Host`.

use std::{rc::Rc, time::Instant};

use gpui::{
    App, Bounds, Context, Entity, Focusable, Global, KeyBinding, Pixels, SharedString,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, actions, div, point, prelude::*, px, rgb, rgba, size,
};

use crate::{
    shell,
    text_input::{self, PasteAndGo, PasteAndSearch, TextInput},
};

mod calculator;

actions!(
    command_bar,
    [
        Go, GoBeside, GoBehind, GoDotCom, Search, Dismiss, Next, Previous, First, Last, Remove,
        EnterSite, LeaveSite
    ]
);

const WIDTH: f32 = 640.;
/// The input's row.
const HEIGHT: f32 = 54.;
const ROW_HEIGHT: f32 = 36.;
/// Around the rows, so a selected row's corners clear the bar's edges.
const ROW_INSET: f32 = 4.;
/// The text's inset, the URL field's, so the text stays where it was as the bar opens out of it.
const TEXT_X: f32 = 13.;
const RADIUS: f32 = 16.;
/// Over a new pane of a split view, the bar stays this far inside the card's right edge (Arc).
const BESIDE_END: f32 = 77.;
const ROW_RADIUS: f32 = 10.;
/// How long the bar takes to grow out of the URL field, in seconds.
const GROW: f32 = 0.13;
// Dia's dark panel (docs/ux-spec.md › Colour tokens).
const FILL: u32 = 0x2d2d2d;
const BORDER: u32 = 0x787d_8652;
const SELECTED: u32 = 0x5e5a5d;
const HOVERED: u32 = 0x787d_8652;
const PLACEHOLDER: u32 = 0xffff_ff87;
const SECONDARY: u32 = 0xeff4_ff99;
const CARET: u32 = 0x4a77d4;
const SELECTION: u32 = 0x787d_86b3;
/// A command's shortcut chip (ours).
const CHIP: u32 = 0xffff_ff14;
const PLACEHOLDER_TEXT: &str = "Search or enter a URL";

/// One of the core's suggestions for what's typed.
#[derive(Default)]
pub struct Suggestion {
    pub url: String,
    pub text: SharedString,
    /// Shown after the text, dimmer, unless it's empty.
    pub detail: SharedString,
    /// The tab already at the page, which the row switches to, or 0.
    pub tab: u64,
    /// What completes the text typed to this row, for the first row only.
    pub completion: String,
    /// Whether ⇧⌦ can take it out of the profile's history.
    pub removable: bool,
    /// The browser command the row runs instead, and its shortcut.
    pub command: Option<shell::Command>,
    pub shortcut: Option<SharedString>,
    /// What a sum typed comes to, which the row copies instead.
    pub result: Option<SharedString>,
    /// The keyword of the search engine whose site search Tab enters on this row, and its name.
    pub keyword: String,
    pub site: SharedString,
}

impl From<shell::BarCommand> for Suggestion {
    fn from(command: shell::BarCommand) -> Self {
        Suggestion {
            text: command.name,
            command: Some(command.command),
            shortcut: command.shortcut,
            ..Suggestion::default()
        }
    }
}

/// What the bar asks of the browser: of the core, for suggestions and pages, and of the host, for
/// the bar's window. The browser's are calls to the core's C API; the tests' are recorded.
pub trait Host {
    /// Asks for the suggestions for `text`, the first completing it if `complete`, or the searches
    /// for it in the site search of the engine whose keyword is `site`. They come back through
    /// `suggested`.
    fn suggest(&self, site: Option<&str>, text: &str, complete: bool);
    /// Takes the suggestion going to `url` out of the profile's history. The suggestions come
    /// again without it.
    fn remove_suggestion(&self, url: &str);
    /// The page the address bar goes to for `text` typed in it: the one it names, else a search.
    fn typed_url(&self, text: &str) -> Option<String>;
    /// A search for `text` with the search engine whose keyword is `site`, else the default one,
    /// even if it names a page.
    fn search_url(&self, site: Option<&str>, text: &str) -> Option<String>;
    /// Opens a tab loading `url` at the top of the Space's Today tabs, and shows it unless
    /// `behind`, when it loads only once it's first shown.
    fn open_tab(&self, url: &str, behind: bool);
    fn navigate(&self, tab: u64, url: &str);
    fn select(&self, tab: u64);
    /// Puts `tab` in a split view with `with`, on its right, and shows it there.
    fn split(&self, tab: u64, with: u64, cx: &mut App);
    /// Opens a tab loading `url` in a split view with `with`, on its right, and shows it there.
    fn open_split(&self, url: &str, with: u64, cx: &mut App);
    /// Copies `text`, a sum's result, and says so.
    fn copy(&self, text: &str, cx: &mut App);
    /// Shows the bar's window, `width` wide, with the keyboard: centred over the browser window,
    /// else at `left` and `top` in it.
    fn show(&self, centered: bool, left: f32, top: f32, width: f32);
    fn set_height(&self, height: f32);
    /// Hides the bar's window once the user has chosen something in it.
    fn hide(&self);
    /// Hides the bar's window as the user leaves it without choosing anything.
    fn dismiss(&self);
    /// The bar's shape changed as it grows, so its window's shadow has to follow.
    fn reshape(&self);
}

/// A search engine's site search, which the bar's searches stay in until the user leaves it, as in
/// Chrome's keyword mode.
struct Site {
    keyword: String,
    name: SharedString,
    /// Tab entered it and nothing has been typed since, so leaving puts the keyword back without
    /// the space after it, as Chrome does.
    tabbed: bool,
}

/// Where the bar shows the page chosen.
#[derive(Clone, Copy)]
enum Target {
    NewTab,
    /// The tab whose URL it edits.
    Tab(u64),
    /// A new tab, in a split view with this one, on its right.
    Beside(u64),
}

pub struct CommandBar {
    host: Rc<dyn Host>,
    input: Entity<TextInput>,
    target: Target,
    /// What was typed, which the core was last asked to suggest for, when, and whether it was
    /// allowed to complete it.
    asked: String,
    asked_at: Instant,
    complete: bool,
    /// What the input shows after what was typed, selected, to complete it.
    completion: String,
    /// The input's length and selection start before its last change, which tell a deletion.
    before: (usize, usize),
    /// The core's suggestions, and the text they're for, until the bar asks again.
    rows: Vec<Suggestion>,
    suggested: String,
    /// The row ↩ takes.
    selected: usize,
    /// The site search the bar is in, if any.
    site: Option<Site>,
    /// While the bar grows out of the URL field: the field's box in the bar's window, and when its
    /// window was first on screen, where the grow starts.
    grow: Option<(Bounds<Pixels>, Option<Instant>)>,
}

struct CommandBarWindow(WindowHandle<CommandBar>);

impl Global for CommandBarWindow {}

/// Opens the command bar's window, hidden, for `host` to show over the browser window. Opening it
/// ahead of time keeps window setup off the path from the shortcut to the bar on screen.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<CommandBar>> {
    cx.bind_keys([
        KeyBinding::new("enter", Go, Some("CommandBar")),
        KeyBinding::new("alt-enter", GoBeside, Some("CommandBar")),
        KeyBinding::new("cmd-enter", GoBehind, Some("CommandBar")),
        KeyBinding::new("ctrl-enter", GoDotCom, Some("CommandBar")),
        KeyBinding::new("shift-cmd-enter", Search, Some("CommandBar")),
        KeyBinding::new("escape", Dismiss, Some("CommandBar")),
        KeyBinding::new("down", Next, Some("CommandBar")),
        KeyBinding::new("up", Previous, Some("CommandBar")),
        KeyBinding::new("ctrl-n", Next, Some("CommandBar")),
        KeyBinding::new("ctrl-p", Previous, Some("CommandBar")),
        KeyBinding::new("tab", EnterSite, Some("CommandBar")),
        KeyBinding::new("shift-tab", LeaveSite, Some("CommandBar")),
        KeyBinding::new("pageup", First, Some("CommandBar")),
        KeyBinding::new("pagedown", Last, Some("CommandBar")),
        KeyBinding::new("shift-delete", Remove, Some("CommandBar")),
    ]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(HEIGHT)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Floating,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The panel's shape is drawn, as it grows.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        let input = cx.new(|cx| {
            TextInput::new(PLACEHOLDER_TEXT, cx)
                .placeholder_color(rgba(PLACEHOLDER))
                .caret_color(rgb(CARET))
                .selection_color(rgba(SELECTION))
                .paste_and_go()
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.new(|cx| {
            cx.observe_in(&input, window, |bar: &mut CommandBar, _, window, cx| {
                bar.edited(window, cx)
            })
            .detach();
            CommandBar {
                host,
                input,
                target: Target::NewTab,
                asked: String::new(),
                asked_at: Instant::now(),
                complete: false,
                completion: String::new(),
                before: (0, 0),
                rows: Vec::new(),
                suggested: String::new(),
                selected: 0,
                site: None,
                grow: None,
            }
        })
    });
    match window {
        Ok(window) => {
            cx.set_global(CommandBarWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the command bar: {error}");
            None
        }
    }
}

/// Shows the command bar with the keyboard: empty for a new tab, or on `editing`'s URL, selected.
/// It suggests nothing until the text is edited, as Chrome's address bar. It grows out of the URL
/// field at `field` in the browser window, with its input centred on the field and its left edge
/// at the field's, or shows centred.
pub fn show(editing: Option<(u64, SharedString)>, field: Option<Bounds<Pixels>>, cx: &mut App) {
    let origin = field.map(|field| point(field.left(), field.center().y - px(HEIGHT / 2.)));
    let grow = field.zip(origin);
    let grow = grow.map(|(field, origin)| Bounds::new(field.origin - origin, field.size));
    let target = editing.as_ref().map_or(Target::NewTab, |&(tab, _)| Target::Tab(tab));
    let text = editing.map(|(_, url)| url).unwrap_or_default();
    if let Some(host) = reset(target, text, grow, cx) {
        let (left, top) = origin.map_or((0., 0.), |origin| (origin.x.into(), origin.y.into()));
        host.show(origin.is_none(), left, top, WIDTH);
    }
}

/// Shows the command bar, empty, over `pane`, a split view's new pane in the card at `card`, for
/// a page to show there beside `with`. It's centred over the pane, as far into the card as it
/// needs to be, and as far down the pane as it opens down the window for a new tab.
pub fn show_split(with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App) {
    if let Some(host) = reset(Target::Beside(with), SharedString::default(), None, cx) {
        let left = pane.center().x - px(WIDTH / 2.);
        let left = left.min(card.right() - px(BESIDE_END + WIDTH)).max(card.left());
        let top = (card.top() + card.bottom()) / 5. + (pane.top() - card.top());
        host.show(false, left.into(), top.into(), WIDTH);
    }
}

/// Puts the bar to `target` with `text` in its input, growing out of `grow` in its window if
/// that's given. Returns its host, or none if there's no bar.
fn reset(
    target: Target,
    text: SharedString,
    grow: Option<Bounds<Pixels>>,
    cx: &mut App,
) -> Option<Rc<dyn Host>> {
    let window = cx.try_global::<CommandBarWindow>()?.0;
    window
        .update(cx, |bar, window, cx| {
            bar.grow = grow.map(|grow| (grow, None));
            bar.target = target;
            bar.site_left(cx);
            bar.asked = text.to_string();
            bar.complete = false;
            bar.completion.clear();
            bar.before = (text.len(), 0);
            bar.set_rows(String::new(), Vec::new(), window, cx);
            bar.input.update(cx, |input, cx| input.reset(text, window, cx));
            // The last query's suggestions are of no use now.
            bar.host.suggest(None, "", false);
            bar.host.clone()
        })
        .ok()
}

/// How far the bar has grown out of the URL field, from 0 to 1 on an exponential ease-out, or none
/// once it's done (docs/ux-spec.md › Motion › Panel grow).
fn grow_progress(start: Instant) -> Option<f32> {
    let t = start.elapsed().as_secs_f32() / GROW;
    (t < 1.).then(|| 1. - 2f32.powf(-10. * t))
}

/// The core's suggestions for `text`.
pub fn suggested(text: String, rows: Vec<Suggestion>, cx: &mut App) {
    let Some(window) = cx.try_global::<CommandBarWindow>().map(|bar| bar.0) else {
        return;
    };
    window
        .update(cx, |bar, window, cx| {
            // Older text's come after the bar has asked about newer.
            if text != bar.request() {
                return;
            }
            // check.sh reads these lines.
            let urls: Vec<String> = rows
                .iter()
                .map(|row| {
                    let tab = match row.tab {
                        0 => String::new(),
                        tab => format!(" tab {tab}"),
                    };
                    let site = match row.keyword.as_str() {
                        "" => String::new(),
                        keyword => format!(" site {keyword}"),
                    };
                    format!("{}{tab}{site}", row.url)
                })
                .collect();
            log!(
                "browser: command bar suggestions \"{text}\" in {:.1} ms: {}",
                bar.asked_at.elapsed().as_secs_f64() * 1000.,
                urls.join(", ")
            );
            // A site search lists only its searches.
            let rows = match bar.site {
                Some(_) => rows,
                None => with_calculation(&text, bar.with_commands(&text, rows, cx)),
            };
            bar.set_rows(bar.asked.clone(), rows, window, cx);
        })
        .ok();
}

/// The clipboard's text as a paste puts it in the input: on one line, trimmed. None if it has none.
fn pasted(cx: &App) -> Option<String> {
    let text = cx.read_from_clipboard()?.text()?.replace('\n', " ");
    Some(text.trim().to_string()).filter(|text| !text.is_empty())
}

/// Puts what `text` comes to, if it's a sum, after the first of `rows`, as Dia's bar does.
fn with_calculation(text: &str, mut rows: Vec<Suggestion>) -> Vec<Suggestion> {
    if let Some(result) = calculator::calculate(text) {
        // check.sh reads this line.
        log!("browser: command bar calculates \"{text}\": {result}");
        let row = Suggestion {
            text: format!("= {result}").into(),
            result: Some(result.into()),
            ..Suggestion::default()
        };
        rows.insert(rows.len().min(1), row);
    }
    rows
}

impl CommandBar {
    fn edited(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        let (text, selection) = (input.text().to_string(), input.selection());
        let (length, start) = std::mem::replace(&mut self.before, (text.len(), selection.start));
        if input.composing() {
            // The input method's text took the completion's place, if it was selected.
            self.completion.clear();
            return;
        }
        let shown = format!("{}{}", self.asked, self.completion);
        // Moving the selection off the completion takes it as typed, as in Chrome.
        if text == shown
            && (self.completion.is_empty() || selection == (self.asked.len()..text.len()))
        {
            return;
        }
        // Chrome completes only typing at the end: not a deletion, a paste, or with the caret
        // elsewhere, and not in a site search.
        let deleted = text.len() < length && selection.start <= start;
        let typing = !deleted && !input.pasted() && selection == (text.len()..text.len());
        let complete = typing && self.site.is_none();
        // A space typed after the keyword the first row hints enters its engine's site search, or
        // an ideographic one, as Japanese input types, as in Chrome.
        let hint = self.rows.first().filter(|row| {
            self.site.is_none()
                && self.suggested == self.asked
                && !row.keyword.is_empty()
                && row.keyword == self.asked
        });
        let spaced = typing && text.strip_suffix([' ', '\u{3000}']) == Some(self.asked.as_str());
        if let Some(row) = hint.filter(|_| spaced) {
            let (keyword, name) = (row.keyword.clone(), row.site.clone());
            return self.enter_site(keyword, name, String::new(), false, window, cx);
        }
        if let Some(site) = self.site.as_mut().filter(|_| !text.is_empty()) {
            site.tabbed = false;
        }
        // While what's typed follows the completion, the rest of it stays until the core answers,
        // so it doesn't flicker.
        let completion = shown
            .get(text.len()..)
            .filter(|_| complete)
            .filter(|_| {
                shown.get(..text.len()).is_some_and(|typed| typed.eq_ignore_ascii_case(&text))
            })
            .unwrap_or_default()
            .to_string();
        self.asked = text;
        self.asked_at = Instant::now();
        self.complete = complete;
        // Rows for the same text asked before are old too, as after ⌫ takes a completion back.
        self.suggested.clear();
        self.completion.clear();
        self.show_completion(completion, window, cx);
        let site = self.site.as_ref().map(|site| site.keyword.as_str());
        self.host.suggest(site, &self.asked, complete);
    }

    /// What the core was last asked for, as its suggestions name it: what was typed, after the
    /// site search's keyword and a space.
    fn request(&self) -> String {
        match &self.site {
            Some(site) => format!("{} {}", site.keyword, self.asked),
            None => self.asked.clone(),
        }
    }

    /// Enters the site search of the engine whose keyword is `keyword`, by Tab if `tabbed`, else
    /// by Space, with `query` in the input.
    fn enter_site(
        &mut self,
        keyword: String,
        name: SharedString,
        query: String,
        tabbed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // check.sh reads this line.
        let by = if tabbed { "tab" } else { "space" };
        log!("browser: command bar site search {keyword} by {by}");
        let placeholder = format!("Search {name}").into();
        self.input.update(cx, |input, cx| input.set_placeholder(placeholder, cx));
        self.site = Some(Site { keyword, name, tabbed });
        self.completion.clear();
        let caret = query.len();
        self.input.update(cx, |input, cx| input.set_text(query.into(), caret, window, cx));
        cx.notify();
    }

    /// Leaves the site search, if the bar is in one, putting its keyword back before what was
    /// typed, with the caret after it, as Chrome does. Returns whether it was in one.
    fn leave_site(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(site) = self.site_left(cx) else {
            return false;
        };
        // check.sh reads this line.
        log!("browser: command bar leaves site search");
        let space = if site.tabbed { "" } else { " " };
        let prefix = format!("{}{space}", site.keyword);
        let text = format!("{prefix}{}", self.input.read(cx).text());
        self.input.update(cx, |input, cx| input.set_text(text.into(), prefix.len(), window, cx));
        cx.notify();
        true
    }

    /// Takes the bar out of its site search, if it's in one, and returns it.
    fn site_left(&mut self, cx: &mut Context<Self>) -> Option<Site> {
        let site = self.site.take()?;
        self.input.update(cx, |input, cx| input.set_placeholder(PLACEHOLDER_TEXT.into(), cx));
        Some(site)
    }

    /// Tab: enters the site search the selected row hints, keeping what was typed after its first
    /// word, the keyword, as Chrome does. Else it selects the next row.
    fn enter_site_or_next(&mut self, _: &EnterSite, window: &mut Window, cx: &mut Context<Self>) {
        let hint = self.chosen().filter(|row| !row.keyword.is_empty() && self.site.is_none());
        let Some((keyword, name)) = hint.map(|row| (row.keyword.clone(), row.site.clone())) else {
            return self.select_next(true, window, cx);
        };
        let typed = self.typed(cx);
        let query = typed.split_once(char::is_whitespace).map_or("", |(_, rest)| rest.trim_start());
        self.enter_site(keyword, name, query.to_string(), true, window, cx);
    }

    /// ⇧Tab: leaves the site search, else selects the row before.
    fn leave_site_or_previous(
        &mut self,
        _: &LeaveSite,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.leave_site(window, cx) {
            self.select_next(false, window, cx);
        }
    }

    /// ⌫ with the caret at the start of a site search's input leaves it.
    fn backspace(
        &mut self,
        _: &text_input::Backspace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.site.is_some() && self.input.read(cx).selection() == (0..0) {
            self.leave_site(window, cx);
            cx.stop_propagation();
        }
    }

    /// Where ↩ goes for `text` before its suggestions come: the page it names, else a search for
    /// it, or in a site search, a search with that site's engine.
    fn typed_url(&self, text: &str) -> Option<String> {
        match &self.site {
            Some(site) => self.host.search_url(Some(&site.keyword), text),
            None => self.host.typed_url(text),
        }
    }

    /// Shows `completion` after what was typed, selected, or no completion.
    fn show_completion(
        &mut self,
        completion: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if completion == self.completion {
            return;
        }
        self.completion = completion;
        // check.sh reads this line.
        log!(
            "browser: command bar completes \"{}\" with \"{}\"",
            self.asked, self.completion
        );
        let text = format!("{}{}", self.asked, self.completion);
        let typed = self.asked.len();
        self.input.update(cx, |input, cx| input.complete(text.into(), typed, window, cx));
    }

    /// Completes what was typed to the first row while it's selected, unless the user has moved
    /// the caret.
    fn update_completion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        let (typed, end) = (self.asked.len(), self.asked.len() + self.completion.len());
        let selection = if self.completion.is_empty() { end..end } else { typed..end };
        if input.composing()
            || input.text() != format!("{}{}", self.asked, self.completion)
            || input.selection() != selection
        {
            return;
        }
        let completion = match self.selected {
            // The core's suggestions for the same text asked without completing it may come.
            _ if !self.complete => String::new(),
            // Rows for older text keep the completion shown, which came from their first.
            0 if self.suggested != self.asked => self.completion.clone(),
            0 => self.rows.first().map(|row| row.completion.clone()).unwrap_or_default(),
            _ => String::new(),
        };
        self.show_completion(completion, window, cx);
    }

    /// Puts the commands found for `text` among the core's `rows`, as Arc's bar does: the best
    /// first if it's strong and the core doesn't complete what's typed, else after the core's
    /// first row, and the others after that. None over a new pane.
    fn with_commands(&self, text: &str, rows: Vec<Suggestion>, cx: &App) -> Vec<Suggestion> {
        if matches!(self.target, Target::Beside(_)) {
            return rows;
        }
        let commands = shell::bar_commands(text, cx);
        if commands.is_empty() {
            return rows;
        }
        // check.sh reads these lines.
        let names: Vec<&str> = commands.iter().map(|command| command.name.as_ref()).collect();
        log!("browser: command bar commands \"{text}\": {}", names.join(", "));
        let first = commands[0].strong && rows.first().is_none_or(|row| row.completion.is_empty());
        let mut commands = commands.into_iter().map(Suggestion::from);
        let mut rows = rows.into_iter();
        let mut merged = Vec::new();
        if first {
            merged.extend(commands.next());
        }
        merged.extend(rows.next());
        merged.extend(commands);
        merged.extend(rows);
        merged
    }

    fn set_rows(
        &mut self,
        text: String,
        rows: Vec<Suggestion>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The page selected stays selected as more suggestions come for what was asked, and when
        // it's taken out, the row that takes its place is, as in Chrome.
        let selected = self.rows.get(self.selected).filter(|_| text == self.suggested);
        self.selected = match selected {
            Some(old) => rows
                .iter()
                .position(|row| row.url == old.url && row.command == old.command)
                .unwrap_or(self.selected.min(rows.len().saturating_sub(1))),
            None => 0,
        };
        self.rows = rows;
        self.suggested = text;
        let rows = self.rows.len() as f32;
        let height = HEIGHT + if rows > 0. { rows * ROW_HEIGHT + 2. * ROW_INSET } else { 0. };
        self.host.set_height(height);
        self.update_completion(window, cx);
        cx.notify();
    }

    /// The row ↩ takes, once there are rows for what's typed. A completion comes from the first
    /// row, even before the rows for the last key arrive.
    fn chosen(&self) -> Option<&Suggestion> {
        let current = self.suggested == self.asked || !self.completion.is_empty();
        self.rows.get(self.selected).filter(|_| current)
    }

    /// What the user typed, without a completion still selected after it.
    fn typed(&self, cx: &App) -> String {
        let input = self.input.read(cx);
        let completing = !self.completion.is_empty()
            && input.selection() == (self.asked.len()..input.text().len());
        let typed = if completing { self.asked.as_str() } else { input.text() };
        typed.trim().to_string()
    }

    fn go(&mut self, _: &Go, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        // check.sh reads these lines.
        log!("browser: command bar typed {text}");
        if self.chosen().is_some() {
            self.take(self.selected, cx);
            return;
        }
        // Before the suggestions for what's typed come, ↩ goes where the first of them will.
        if let Some(url) = self.typed_url(&text) {
            self.go_to(&url, cx);
        }
    }

    /// Paste and Go, from the input's menu: goes where ↩ would for the clipboard's text, as
    /// Chrome does.
    fn paste_and_go(&mut self, _: &PasteAndGo, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = pasted(cx).and_then(|text| self.typed_url(&text)) {
            self.go_to(&url, cx);
        }
    }

    /// Paste and Search: searches for the clipboard's text, even if it names a page, as ⇧⌘↩
    /// does for what's typed.
    fn paste_and_search(&mut self, _: &PasteAndSearch, _: &mut Window, cx: &mut Context<Self>) {
        let site = self.site.as_ref().map(|site| site.keyword.as_str());
        if let Some(url) = pasted(cx).and_then(|text| self.host.search_url(site, &text)) {
            self.go_to(&url, cx);
        }
    }

    /// Shows `url` where the bar shows the page chosen, and closes the bar.
    fn go_to(&self, url: &str, cx: &mut App) {
        // check.sh reads this line.
        log!("browser: command bar go {url}");
        self.show_page(url, cx);
        self.host.hide();
    }

    /// ⌘↩: opens where ↩ would go in a tab behind the one shown, at the top of Today, and the bar
    /// stays open (Arc). A command or a sum's result is taken as ↩ takes it, and over a new pane
    /// ⌘↩ is ↩.
    fn go_behind(&mut self, _: &GoBehind, window: &mut Window, cx: &mut Context<Self>) {
        let command =
            self.chosen().is_some_and(|row| row.command.is_some() || row.result.is_some());
        if command || matches!(self.target, Target::Beside(_)) {
            return self.go(&Go, window, cx);
        }
        let text = self.input.read(cx).text().trim().to_string();
        let url = match self.chosen() {
            Some(row) => Some(row.url.clone()),
            None if text.is_empty() => None,
            None => self.typed_url(&text),
        };
        if let Some(url) = url {
            // check.sh reads this line.
            log!("browser: command bar behind {url}");
            self.host.open_tab(&url, true);
        }
    }

    /// ⌃↩: goes to www.<word>.com for a word typed, as Chrome does, and else where ↩ goes, as in a
    /// site search.
    fn go_dot_com(&mut self, _: &GoDotCom, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.typed(cx);
        let word = !typed.is_empty() && typed.chars().all(|c| c.is_alphanumeric() || c == '-');
        let word = word && self.site.is_none();
        match word.then(|| self.host.typed_url(&format!("www.{typed}.com"))).flatten() {
            Some(url) => self.go_to(&url, cx),
            None => self.go(&Go, window, cx),
        }
    }

    /// ⇧⌘↩: searches for what was typed, even if it names a page, where ↩ goes (Dia).
    fn search(&mut self, _: &Search, _: &mut Window, cx: &mut Context<Self>) {
        let typed = self.typed(cx);
        let site = self.site.as_ref().map(|site| site.keyword.as_str());
        if let Some(url) = self.host.search_url(site, &typed).filter(|_| !typed.is_empty()) {
            self.go_to(&url, cx);
        }
    }

    /// ⇧⌦: takes the selected row's page out of the profile's history, if it came from there, as
    /// Chrome does. The core lists the rows again without it.
    fn remove(&mut self, _: &Remove, _: &mut Window, _: &mut Context<Self>) {
        let Some(row) = self.chosen().filter(|row| row.removable) else {
            return;
        };
        // check.sh reads this line.
        log!("browser: command bar removes {}", row.url);
        self.host.remove_suggestion(&row.url);
    }

    /// ⌥↩: goes where ↩ would, in a new tab in a split view with the tab shown, or the tab the
    /// bar edits, on its right (Arc).
    fn go_beside(&mut self, _: &GoBeside, window: &mut Window, cx: &mut Context<Self>) {
        let with = match self.target {
            Target::Tab(tab) | Target::Beside(tab) => tab,
            Target::NewTab => shell::shown_tab(cx),
        };
        if with != 0 {
            self.target = Target::Beside(with);
        }
        self.go(&Go, window, cx);
    }

    /// Shows `url` where the bar shows the page chosen.
    fn show_page(&self, url: &str, cx: &mut App) {
        match self.target {
            Target::NewTab => self.host.open_tab(url, false),
            Target::Tab(tab) => self.host.navigate(tab, url),
            Target::Beside(with) => self.host.open_split(url, with, cx),
        }
    }

    /// Goes where row `index` goes: to its tab, else its page, where the bar shows the page
    /// chosen. Beside a tab, its tab moves into the split view. A sum's row copies its result.
    fn take(&mut self, index: usize, cx: &mut App) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        if let Some(result) = &row.result {
            // check.sh reads this line.
            log!("browser: command bar copies {result}");
            self.host.hide();
            self.host.copy(result, cx);
            return;
        }
        // check.sh reads these lines.
        if let Some(command) = row.command {
            log!("browser: command bar runs {}", row.text);
            self.host.hide();
            shell::run_from_bar(command, cx);
            return;
        }
        match (row.tab, self.target) {
            (0, _) => return self.go_to(&row.url, cx),
            (tab, Target::Beside(with)) => {
                log!("browser: command bar split with tab {tab}");
                self.host.split(tab, with, cx);
            }
            (tab, _) => {
                log!("browser: command bar switch to tab {tab}");
                self.host.select(tab);
            }
        }
        self.host.hide();
    }

    /// Selects the next row, or the one before, wrapping at the ends as Chrome does.
    fn select_next(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.rows.len();
        if count > 0 {
            self.select((self.selected + if forward { 1 } else { count - 1 }) % count, window, cx);
        }
    }

    /// Selects row `index`, if there is one.
    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.rows.len() {
            return;
        }
        self.selected = index;
        // check.sh reads this line.
        let row = &self.rows[self.selected];
        let named = row.command.is_some() || row.result.is_some();
        let name = if named { row.text.as_ref() } else { row.url.as_str() };
        log!("browser: command bar selected {name}");
        self.update_completion(window, cx);
        cx.notify();
    }

    /// Esc leaves the site search first, then the bar.
    fn dismiss(&mut self, _: &Dismiss, window: &mut Window, cx: &mut Context<Self>) {
        if !self.leave_site(window, cx) {
            self.host.dismiss();
        }
    }

    fn row(&self, index: usize, row: &Suggestion, cx: &mut Context<Self>) -> impl IntoElement {
        let dim = |text: SharedString| div().text_color(rgba(SECONDARY)).child(text);
        let chip = |text: SharedString| {
            dim(text).flex_none().px(px(5.)).rounded(px(4.)).bg(rgba(CHIP)).text_size(px(11.))
        };
        // Tab enters the site search the selected row hints (Chrome's "Search <site>").
        let hint = index == self.selected && self.site.is_none() && !row.keyword.is_empty();
        div()
            .id(("suggestion", index))
            .h(px(ROW_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(TEXT_X - ROW_INSET))
            .rounded(px(ROW_RADIUS))
            .map(|line| match index == self.selected {
                true => line.bg(rgb(SELECTED)),
                false => line.hover(|style| style.bg(rgba(HOVERED))),
            })
            .on_click(cx.listener(move |bar, _, _, cx| bar.take(index, cx)))
            .child(div().min_w_0().truncate().child(row.text.clone()))
            .when(!row.detail.is_empty(), |line| {
                line.child(dim(format!("— {}", row.detail).into()).min_w_0().truncate())
            })
            .when(row.tab != 0, |line| {
                let action = match self.target {
                    Target::Beside(_) => "Add to Split View",
                    _ => "Switch to Tab",
                };
                line.child(dim(action.into()).flex_none().ml_auto())
            })
            .when_some(row.shortcut.clone(), |line, shortcut| line.child(chip(shortcut).ml_auto()))
            .when(hint, |line| {
                let search = dim(format!("Search {}", row.site).into()).flex_none().ml_auto();
                line.child(search).child(chip("Tab".into()))
            })
    }
}

impl Render for CommandBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // check.sh reads these lines.
        log!("browser: command bar drew \"{}\"", self.input.read(cx).text());
        let rows: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.row(index, row, cx).into_any_element())
            .collect();
        // The panel's shape grows from the URL field's box to the window's and uncovers the bar,
        // which stays still.
        let size = window.viewport_size();
        let whole = Bounds::new(point(px(0.), px(0.)), size);
        // GPUI draws the window again only once AppKit reports it on screen, some frames after
        // the host's first draw put it there, so the grow starts there rather than skip its start.
        if let Some((_, start @ None)) = &mut self.grow
            && window.is_visible()
        {
            *start = Some(Instant::now());
        }
        let growing = self.grow.and_then(|(field, start)| {
            Some((field, start.map_or(Some(0.), grow_progress)?))
        });
        let (shape, radius) = match growing {
            Some((field, progress)) => {
                window.request_animation_frame();
                self.host.reshape();
                let shape = Bounds::new(
                    field.origin * (1. - progress),
                    gpui::size(
                        field.size.width + (size.width - field.size.width) * progress,
                        field.size.height + (size.height - field.size.height) * progress,
                    ),
                );
                (shape, shell::PLATE_RADIUS + (RADIUS - shell::PLATE_RADIUS) * progress)
            }
            None => {
                if self.grow.take().is_some() {
                    self.host.reshape();
                }
                (whole, RADIUS)
            }
        };
        // The browser's shortcuts work here too, below the bar's own keys.
        div().key_context(shell::KEY_CONTEXT).size_full().child(
            div()
                .key_context("CommandBar")
                .on_action(cx.listener(Self::go))
                .on_action(cx.listener(Self::go_beside))
                .on_action(cx.listener(Self::go_behind))
                .on_action(cx.listener(Self::go_dot_com))
                .on_action(cx.listener(Self::search))
                .on_action(cx.listener(Self::paste_and_go))
                .on_action(cx.listener(Self::paste_and_search))
                .on_action(cx.listener(Self::remove))
                .on_action(cx.listener(Self::dismiss))
                .on_action(cx.listener(Self::enter_site_or_next))
                .on_action(cx.listener(Self::leave_site_or_previous))
                .capture_action(cx.listener(Self::backspace))
                .on_action(
                    cx.listener(|bar, _: &Next, window, cx| bar.select_next(true, window, cx)),
                )
                .on_action(
                    cx.listener(|bar, _: &Previous, window, cx| bar.select_next(false, window, cx)),
                )
                .on_action(cx.listener(|bar, _: &First, window, cx| bar.select(0, window, cx)))
                .on_action(cx.listener(|bar, _: &Last, window, cx| {
                    bar.select(bar.rows.len().saturating_sub(1), window, cx)
                }))
                .absolute()
                .left(shape.left())
                .top(shape.top())
                .w(shape.size.width)
                .h(shape.size.height)
                .overflow_hidden()
                .rounded(px(radius))
                // One device pixel.
                .border(px(1. / window.scale_factor()))
                .border_color(rgba(BORDER))
                .bg(rgb(FILL))
                .text_color(rgb(0xffffff))
                .text_size(px(13.))
                .child(
                    div()
                        .absolute()
                        .left(-shape.left())
                        .top(-shape.top())
                        .w(size.width)
                        .h(size.height)
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(HEIGHT))
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .px(px(TEXT_X))
                                .line_height(px(17.))
                                .when_some(self.site.as_ref(), |line, site| {
                                    line.child(
                                        div()
                                            .flex_none()
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(6.))
                                            .bg(rgba(CHIP))
                                            .child(site.name.clone()),
                                    )
                                })
                                .child(div().flex_1().min_w_0().child(self.input.clone())),
                        )
                        .when(!rows.is_empty(), |bar| {
                            bar.child(div().flex().flex_col().p(px(ROW_INSET)).children(rows))
                        }),
                ),
        )
    }
}

#[cfg(test)]
mod tests;
