//! The command bar. GPUI can't draw over the page's native view, so the bar is a window of its
//! own that the host shows over the browser window, as its child. Under its input it lists the
//! core's suggestions for what's typed, and the window grows down to fit them.

use std::time::Instant;

use browser::{shell, text_input::TextInput};
use gpui::{
    App, Bounds, Context, Entity, Focusable, Global, KeyBinding, SharedString, TitlebarOptions,
    Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, div, point,
    prelude::*, px, rgb, rgba, size,
};

use crate::{WINDOW, bc, navigate, ns_view, open_tab, platform, typed_url};

actions!(command_bar, [Go, Dismiss, Next, Previous]);

const WIDTH: f32 = 640.;
/// The input's row.
const HEIGHT: f32 = 56.;
const ROW_HEIGHT: f32 = 36.;
/// Around the rows, so a selected row's corners clear the bar's edges.
const ROW_INSET: f32 = 4.;
const SELECTED: u32 = 0x0000_0012;
const HOVERED: u32 = 0x0000_0008;

/// One of the core's suggestions for what's typed.
pub struct Suggestion {
    pub url: String,
    pub text: SharedString,
    /// Shown after the text, dimmer, unless it's empty.
    pub detail: SharedString,
    /// The tab already at the page, which the row switches to, or 0.
    pub tab: u64,
}

pub struct CommandBar {
    input: Entity<TextInput>,
    /// The tab the bar loads in, or none for a new tab.
    editing: Option<u64>,
    /// The text the core was last asked to suggest for, and when.
    asked: String,
    asked_at: Instant,
    /// The core's suggestions, and the text they're for.
    rows: Vec<Suggestion>,
    suggested: String,
    /// The row ↩ takes.
    selected: usize,
}

struct CommandBarWindow(WindowHandle<CommandBar>);

impl Global for CommandBarWindow {}

/// Opens the command bar's window, hidden, over the browser window. Opening it ahead of time
/// keeps window setup off the path from the shortcut to the bar on screen.
pub fn open(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Go, Some("CommandBar")),
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
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        let input = cx.new(|cx| TextInput::new("Search or enter a URL", cx));
        window.focus(&input.focus_handle(cx), cx);
        cx.new(|cx| {
            cx.observe(&input, |bar: &mut CommandBar, _, cx| bar.edited(cx))
                .detach();
            CommandBar {
                input,
                editing: None,
                asked: String::new(),
                asked_at: Instant::now(),
                rows: Vec::new(),
                suggested: String::new(),
                selected: 0,
            }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
                unsafe { platform::host_add_overlay(view) };
                cx.set_global(CommandBarWindow(window));
            }
        }
        Err(error) => eprintln!("couldn't open the command bar: {error}"),
    }
}

/// Shows the command bar with the keyboard: empty for a new tab, or on `editing`'s URL, selected.
/// It suggests nothing until the text is edited, as Chrome's address bar.
pub fn show(editing: Option<(u64, SharedString)>, cx: &mut App) {
    let Some(window) = cx.try_global::<CommandBarWindow>().map(|bar| bar.0) else {
        return;
    };
    window
        .update(cx, |bar, window, cx| {
            bar.editing = editing.as_ref().map(|(tab, _)| *tab);
            let text = editing.map(|(_, url)| url).unwrap_or_default();
            bar.asked = text.to_string();
            bar.set_rows(String::new(), Vec::new(), cx);
            bar.input.update(cx, |input, cx| input.reset(text, window, cx))
        })
        .ok();
    // The last query's suggestions are of no use now.
    suggest("");
    unsafe { platform::host_show_overlay() };
}

/// The core's suggestions for `text`.
pub fn suggested(text: String, rows: Vec<Suggestion>, cx: &mut App) {
    let Some(window) = cx.try_global::<CommandBarWindow>().map(|bar| bar.0) else {
        return;
    };
    window
        .update(cx, |bar, _, cx| {
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
            bar.set_rows(text, rows, cx);
        })
        .ok();
}

/// Asks the core for its suggestions for `text`.
fn suggest(text: &str) {
    if let Ok(text) = std::ffi::CString::new(text) {
        unsafe { bc::bc_window_suggest(WINDOW.get(), text.as_ptr()) };
    }
}

impl CommandBar {
    fn edited(&mut self, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        if input.composing() || input.text() == self.asked {
            return;
        }
        self.asked = input.text().to_string();
        self.asked_at = Instant::now();
        suggest(&self.asked);
    }

    fn set_rows(&mut self, text: String, rows: Vec<Suggestion>, cx: &mut Context<Self>) {
        // The page selected stays selected as more suggestions come for the same text.
        let selected = self.rows.get(self.selected).filter(|_| text == self.suggested);
        self.selected = selected
            .and_then(|old| rows.iter().position(|row| row.url == old.url))
            .unwrap_or(0);
        self.rows = rows;
        self.suggested = text;
        let rows = self.rows.len() as f32;
        let height = HEIGHT + if rows > 0. { rows * ROW_HEIGHT + 2. * ROW_INSET } else { 0. };
        unsafe { platform::host_set_overlay_height(f64::from(height)) };
        cx.notify();
    }

    fn go(&mut self, _: &Go, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        // check.sh reads these lines.
        eprintln!("browser: command bar typed {text}");
        if self.suggested == self.input.read(cx).text() && self.selected < self.rows.len() {
            self.take(self.selected);
            return;
        }
        // Before the suggestions for what's typed come, ↩ goes where the first of them will.
        let Some(url) = typed_url(&text) else {
            return;
        };
        eprintln!("browser: command bar go {url}");
        match self.editing {
            Some(tab) => navigate(tab, &url),
            None => open_tab(&url),
        }
        unsafe { platform::host_hide_overlay() };
    }

    /// Goes where row `index` goes: to its tab, else its page, in the tab the bar edits or a new
    /// one.
    fn take(&mut self, index: usize) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        // check.sh reads these lines.
        if row.tab != 0 {
            eprintln!("browser: command bar switch to tab {}", row.tab);
            unsafe { bc::bc_window_select(WINDOW.get(), row.tab) };
        } else {
            eprintln!("browser: command bar go {}", row.url);
            match self.editing {
                Some(tab) => navigate(tab, &row.url),
                None => open_tab(&row.url),
            }
        }
        unsafe { platform::host_hide_overlay() };
    }

    /// Selects the next row, or the one before, wrapping at the ends as Chrome does.
    fn select_next(&mut self, forward: bool, cx: &mut Context<Self>) {
        let count = self.rows.len();
        if count == 0 {
            return;
        }
        self.selected = (self.selected + if forward { 1 } else { count - 1 }) % count;
        // check.sh reads this line.
        eprintln!("browser: command bar selected {}", self.rows[self.selected].url);
        cx.notify();
    }

    fn dismiss(&mut self, _: &Dismiss, _: &mut Window, _: &mut Context<Self>) {
        unsafe { platform::host_hide_overlay() };
    }

    fn row(&self, index: usize, row: &Suggestion, cx: &mut Context<Self>) -> impl IntoElement {
        let dim = |text: SharedString| div().opacity(0.5).child(text);
        div()
            .id(("suggestion", index))
            .h(px(ROW_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(12.))
            .rounded(px(10.))
            .map(|line| match index == self.selected {
                true => line.bg(rgba(SELECTED)),
                false => line.hover(|style| style.bg(rgba(HOVERED))),
            })
            .on_click(cx.listener(move |bar, _, _, _| bar.take(index)))
            .child(div().min_w_0().truncate().child(row.text.clone()))
            .when(!row.detail.is_empty(), |line| {
                line.child(dim(format!("— {}", row.detail).into()).min_w_0().truncate())
            })
            .when(row.tab != 0, |line| {
                line.child(dim("Switch to Tab".into()).flex_none().ml_auto())
            })
    }
}

impl Render for CommandBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // check.sh reads these lines.
        eprintln!("browser: command bar drew \"{}\"", self.input.read(cx).text());
        let rows: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.row(index, row, cx).into_any_element())
            .collect();
        // The browser's shortcuts work here too, below the bar's own keys.
        div().key_context(shell::KEY_CONTEXT).size_full().child(
            div()
                .key_context("CommandBar")
                .on_action(cx.listener(Self::go))
                .on_action(cx.listener(Self::dismiss))
                .on_action(cx.listener(|bar, _: &Next, _, cx| bar.select_next(true, cx)))
                .on_action(cx.listener(|bar, _: &Previous, _, cx| bar.select_next(false, cx)))
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(0xffffff))
                .text_color(rgb(0x1d1d1f))
                .child(
                    div()
                        .h(px(HEIGHT))
                        .flex_none()
                        .flex()
                        .items_center()
                        .px_4()
                        .text_size(px(18.))
                        .line_height(px(24.))
                        .child(self.input.clone()),
                )
                .when(!rows.is_empty(), |bar| {
                    bar.child(
                        div()
                            .flex()
                            .flex_col()
                            .p(px(ROW_INSET))
                            .text_size(px(14.))
                            .children(rows),
                    )
                }),
        )
    }
}
