//! The command bar. GPUI can't draw over the page's native view, so the bar is a window of its
//! own that the host shows over the browser window, as its child. Under its input it lists the
//! core's suggestions for what's typed, and the window grows down to fit them. While the first is
//! selected, the input completes what's typed to it, as Chrome's address bar does. Opened from the
//! URL field, it grows out of the field, as Dia's does (docs/ux-spec.md › Command bar).

use std::time::Instant;

use browser::{shell, text_input::TextInput};
use gpui::{
    App, Bounds, Context, Entity, Focusable, Global, KeyBinding, Pixels, SharedString,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, actions, div, point, prelude::*, px, rgb, rgba, size,
};

use crate::{
    WINDOW, bc, navigate, ns_view, open_split, open_tab, platform, split_tab, typed_url,
    update_shell,
};

actions!(command_bar, [Go, GoBeside, Dismiss, Next, Previous]);

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

/// One of the core's suggestions for what's typed.
pub struct Suggestion {
    pub url: String,
    pub text: SharedString,
    /// Shown after the text, dimmer, unless it's empty.
    pub detail: SharedString,
    /// The tab already at the page, which the row switches to, or 0.
    pub tab: u64,
    /// What completes the text typed to this row, for the first row only.
    pub completion: String,
    /// The browser command the row runs instead, and its shortcut.
    pub command: Option<shell::Command>,
    pub shortcut: Option<SharedString>,
}

impl From<shell::BarCommand> for Suggestion {
    fn from(command: shell::BarCommand) -> Self {
        Suggestion {
            url: String::new(),
            text: command.name,
            detail: SharedString::default(),
            tab: 0,
            completion: String::new(),
            command: Some(command.command),
            shortcut: command.shortcut,
        }
    }
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
    /// While the bar grows out of the URL field: the field's box in the bar's window, and when it
    /// started.
    grow: Option<(Bounds<Pixels>, Instant)>,
}

struct CommandBarWindow(WindowHandle<CommandBar>);

impl Global for CommandBarWindow {}

/// Opens the command bar's window, hidden, over the browser window. Opening it ahead of time
/// keeps window setup off the path from the shortcut to the bar on screen.
pub fn open(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Go, Some("CommandBar")),
        KeyBinding::new("alt-enter", GoBeside, Some("CommandBar")),
        KeyBinding::new("escape", Dismiss, Some("CommandBar")),
        KeyBinding::new("down", Next, Some("CommandBar")),
        KeyBinding::new("up", Previous, Some("CommandBar")),
        KeyBinding::new("ctrl-n", Next, Some("CommandBar")),
        KeyBinding::new("ctrl-p", Previous, Some("CommandBar")),
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
            TextInput::new("Search or enter a URL", cx)
                .placeholder_color(rgba(PLACEHOLDER))
                .caret_color(rgb(CARET))
                .selection_color(rgba(SELECTION))
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.new(|cx| {
            cx.observe_in(&input, window, |bar: &mut CommandBar, _, window, cx| {
                bar.edited(window, cx)
            })
            .detach();
            CommandBar {
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
                grow: None,
            }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
                unsafe { platform::host_add_overlay(view, dismissed) };
                cx.set_global(CommandBarWindow(window));
            }
        }
        Err(error) => eprintln!("couldn't open the command bar: {error}"),
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
    if reset(target, text, grow, cx) {
        let (left, top) = origin.map_or((0., 0.), |origin| (origin.x.into(), origin.y.into()));
        unsafe { platform::host_show_overlay(origin.is_none(), left, top, f64::from(WIDTH)) };
    }
}

/// Shows the command bar, empty, over `pane`, a split view's new pane in the card at `card`, for
/// a page to show there beside `with`. It's centred over the pane, as far into the card as it
/// needs to be, and as far down the window as it opens for a new tab.
pub fn show_split(with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App) {
    if reset(Target::Beside(with), SharedString::default(), None, cx) {
        let left = pane.center().x - px(WIDTH / 2.);
        let left = left.min(card.right() - px(BESIDE_END + WIDTH)).max(card.left());
        let top = (card.top() + card.bottom()) / 5.;
        unsafe { platform::host_show_overlay(false, left.into(), top.into(), f64::from(WIDTH)) };
    }
}

/// Puts the bar to `target` with `text` in its input, growing out of `grow` in its window if
/// that's given. False if there's no bar.
fn reset(target: Target, text: SharedString, grow: Option<Bounds<Pixels>>, cx: &mut App) -> bool {
    let Some(window) = cx.try_global::<CommandBarWindow>().map(|bar| bar.0) else {
        return false;
    };
    window
        .update(cx, |bar, window, cx| {
            bar.grow = grow.map(|grow| (grow, Instant::now()));
            bar.target = target;
            bar.asked = text.to_string();
            bar.complete = false;
            bar.completion.clear();
            bar.before = (text.len(), 0);
            bar.set_rows(String::new(), Vec::new(), window, cx);
            bar.input.update(cx, |input, cx| input.reset(text, window, cx))
        })
        .ok();
    // The last query's suggestions are of no use now.
    suggest("", false);
    true
}

/// The user left the bar without choosing anything in it.
extern "C" fn dismissed() {
    update_shell(|shell, cx| shell.command_bar_dismissed(cx));
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
            if text != bar.asked {
                return;
            }
            // check.sh reads these lines.
            let urls: Vec<String> = rows
                .iter()
                .map(|row| match row.tab {
                    0 => row.url.clone(),
                    tab => format!("{} tab {tab}", row.url),
                })
                .collect();
            eprintln!(
                "browser: command bar suggestions \"{text}\" in {:.1} ms: {}",
                bar.asked_at.elapsed().as_secs_f64() * 1000.,
                urls.join(", ")
            );
            let rows = bar.with_commands(&text, rows, cx);
            bar.set_rows(text, rows, window, cx);
        })
        .ok();
}

/// Asks the core for its suggestions for `text`, the first of them completing it if `complete`.
fn suggest(text: &str, complete: bool) {
    if let Ok(text) = std::ffi::CString::new(text) {
        unsafe { bc::bc_window_suggest(WINDOW.get(), text.as_ptr(), complete) };
    }
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
        // elsewhere.
        let deleted = text.len() < length && selection.start <= start;
        let complete = !deleted && !input.pasted() && selection == (text.len()..text.len());
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
        suggest(&self.asked, complete);
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
        eprintln!(
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
        eprintln!("browser: command bar commands \"{text}\": {}", names.join(", "));
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
        // The page selected stays selected as more suggestions come for what was asked.
        let selected = self.rows.get(self.selected).filter(|_| text == self.suggested);
        self.selected = selected
            .and_then(|old| {
                rows.iter().position(|row| row.url == old.url && row.command == old.command)
            })
            .unwrap_or(0);
        self.rows = rows;
        self.suggested = text;
        let rows = self.rows.len() as f32;
        let height = HEIGHT + if rows > 0. { rows * ROW_HEIGHT + 2. * ROW_INSET } else { 0. };
        unsafe { platform::host_set_overlay_height(f64::from(height)) };
        self.update_completion(window, cx);
        cx.notify();
    }

    fn go(&mut self, _: &Go, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        // check.sh reads these lines.
        eprintln!("browser: command bar typed {text}");
        // A completion comes from the first row, even before the rows for the last key arrive.
        if (self.suggested == self.asked || !self.completion.is_empty())
            && self.selected < self.rows.len()
        {
            self.take(self.selected, cx);
            return;
        }
        // Before the suggestions for what's typed come, ↩ goes where the first of them will.
        let Some(url) = typed_url(&text) else {
            return;
        };
        eprintln!("browser: command bar go {url}");
        self.show_page(&url, cx);
        unsafe { platform::host_hide_overlay() };
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
            Target::NewTab => open_tab(url),
            Target::Tab(tab) => navigate(tab, url),
            Target::Beside(with) => open_split(url, with, cx),
        }
    }

    /// Goes where row `index` goes: to its tab, else its page, where the bar shows the page
    /// chosen. Beside a tab, its tab moves into the split view.
    fn take(&mut self, index: usize, cx: &mut App) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        // check.sh reads these lines.
        if let Some(command) = row.command {
            eprintln!("browser: command bar runs {}", row.text);
            unsafe { platform::host_hide_overlay() };
            shell::run_from_bar(command, cx);
            return;
        }
        match (row.tab, self.target) {
            (0, _) => {
                eprintln!("browser: command bar go {}", row.url);
                self.show_page(&row.url, cx);
            }
            (tab, Target::Beside(with)) => {
                eprintln!("browser: command bar split with tab {tab}");
                split_tab(tab, with, cx);
            }
            (tab, _) => {
                eprintln!("browser: command bar switch to tab {tab}");
                unsafe { bc::bc_window_select(WINDOW.get(), tab) };
            }
        }
        unsafe { platform::host_hide_overlay() };
    }

    /// Selects the next row, or the one before, wrapping at the ends as Chrome does.
    fn select_next(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.rows.len();
        if count == 0 {
            return;
        }
        self.selected = (self.selected + if forward { 1 } else { count - 1 }) % count;
        // check.sh reads this line.
        let row = &self.rows[self.selected];
        let name = if row.command.is_some() { row.text.as_ref() } else { row.url.as_str() };
        eprintln!("browser: command bar selected {name}");
        self.update_completion(window, cx);
        cx.notify();
    }

    fn dismiss(&mut self, _: &Dismiss, _: &mut Window, _: &mut Context<Self>) {
        unsafe { platform::host_dismiss_overlay() };
    }

    fn row(&self, index: usize, row: &Suggestion, cx: &mut Context<Self>) -> impl IntoElement {
        let dim = |text: SharedString| div().text_color(rgba(SECONDARY)).child(text);
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
            .when_some(row.shortcut.clone(), |line, shortcut| {
                line.child(
                    dim(shortcut)
                        .flex_none()
                        .ml_auto()
                        .px(px(5.))
                        .rounded(px(4.))
                        .bg(rgba(CHIP))
                        .text_size(px(11.)),
                )
            })
    }
}

impl Render for CommandBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // check.sh reads these lines.
        eprintln!("browser: command bar drew \"{}\"", self.input.read(cx).text());
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
        let growing = self.grow.and_then(|(field, start)| Some((field, grow_progress(start)?)));
        let (shape, radius) = match growing {
            Some((field, progress)) => {
                window.request_animation_frame();
                unsafe { platform::host_reshape_overlay() };
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
                    unsafe { platform::host_reshape_overlay() };
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
                .on_action(cx.listener(Self::dismiss))
                .on_action(
                    cx.listener(|bar, _: &Next, window, cx| bar.select_next(true, window, cx)),
                )
                .on_action(
                    cx.listener(|bar, _: &Previous, window, cx| bar.select_next(false, window, cx)),
                )
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
                                .px(px(TEXT_X))
                                .line_height(px(17.))
                                .child(self.input.clone()),
                        )
                        .when(!rows.is_empty(), |bar| {
                            bar.child(div().flex().flex_col().p(px(ROW_INSET)).children(rows))
                        }),
                ),
        )
    }
}
