//! The find bar, over the top right of the page. GPUI can't draw over the page's native view, so
//! each browser window's bar is a window of its own that the host shows over it, as its child,
//! while the tab shown has find in page open. The core keeps each tab's find; the bar shows the
//! shown tab's.

use std::{cell::RefCell, collections::HashMap, ffi::CString, ffi::c_void};

use browser::{
    log,
    shell,
    text_input::TextInput,
};
use gpui::{
    App, Bounds, Context, Entity, Focusable, FontWeight, Global, KeyBinding, SharedString,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, actions, div, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::{bc, platform};

actions!(find_bar, [FindNext, FindPrevious, SelectAll, Close]);

// Arc's bar (docs/ux-spec.md › Find in page): 300 × 32, radius 6, 10 in from the card's top right
// corner, so twice the card's inset from the window's.
const WIDTH: f32 = 300.;
const HEIGHT: f32 = 32.;
const RADIUS: f32 = 6.;
const RIGHT: f32 = shell::CARD_INSET * 2.;
// Arc's fill, a navy unlike its window's colour (one Space captured).
const FILL: u32 = 0x131d32;
const TOP: f32 = shell::CARD_INSET * 2.;
// Arc's buttons are 28 pt, 2 pt inside the bar, previous and next 26 apart, and a 4 pt divider
// before close. The field starts 1 pt after the search glyph's square.
const INSET: f32 = 2.;
const BUTTON: f32 = 28.;
const ARROW_OVERLAP: f32 = 2.;
const DIVIDER_GAP: f32 = 4.;
const FIELD_GAP: f32 = 1.;
const COUNT_GAP: f32 = 2.5;
// Ours: Arc's glyph sizes and colours aren't measured, so the header's.
const GLYPH: f32 = 14.;
const GLYPH_COLOR: u32 = 0xffffff99;
const GLYPH_DISABLED: u32 = 0xffffff42;
const HOVERED: u32 = 0xffffff14;
const DIVIDER: u32 = 0xffffff26;
const DIVIDER_HEIGHT: f32 = 16.;
const TEXT: u32 = 0xffffff;
const COUNT: u32 = 0xffffff99;
// Arc's red for no match.
const NOT_FOUND: u32 = 0xdb1929;
// The command bar's.
const PLACEHOLDER: u32 = 0xffff_ff87;
const CARET: u32 = 0x4a77d4;
const SELECTION: u32 = 0x787d_86b3;

/// A tab's find in page, as the core reports it.
#[derive(Clone, Default)]
pub struct Find {
    pub open: bool,
    pub text: String,
    /// -1 until the page has looked.
    pub matches: i32,
    /// From 1, or 0 for none.
    pub selected: i32,
}

thread_local! {
    // Every tab's. Kept out of GPUI's entities, so a tab closing inside a GPUI update drops its own.
    static FINDS: RefCell<HashMap<u64, Find>> = RefCell::new(HashMap::new());
}

pub struct FindBar {
    /// The UI's view of the browser window the bar is over.
    ui_view: *mut c_void,
    input: Entity<TextInput>,
    /// The tab the window shows, whose find this bar is.
    tab: u64,
    /// What the page was last asked to find, so a change that leaves the text as it was doesn't
    /// search again.
    searched: String,
}

/// Each browser window's bar, by the core's id of the window.
#[derive(Default)]
struct FindBars(HashMap<bc::bc_id, WindowHandle<FindBar>>);

impl Global for FindBars {}

/// Opens the find bar's window for the browser window `browser`, whose UI draws in `ui_view`,
/// hidden, ahead of time as the command bar does.
pub fn open(browser: bc::bc_id, ui_view: *mut c_void, cx: &mut App) {
    // The keys are bound once, for every window's bar.
    if !cx.has_global::<FindBars>() {
        cx.bind_keys([
            KeyBinding::new("enter", FindNext, Some("FindBar")),
            KeyBinding::new("shift-enter", FindPrevious, Some("FindBar")),
            // The browser's ⌘G, ⇧⌘G and ⌘F, for the bar while it has the keyboard.
            KeyBinding::new("cmd-g", FindNext, Some("FindBar")),
            KeyBinding::new("shift-cmd-g", FindPrevious, Some("FindBar")),
            KeyBinding::new("cmd-f", SelectAll, Some("FindBar")),
            KeyBinding::new("escape", Close, Some("FindBar")),
        ]);
        cx.set_global(FindBars::default());
    }
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
        // The bar's rounded shape is drawn, and the shadow takes it.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        let input = cx.new(|cx| {
            TextInput::new("Find in page", cx)
                .placeholder_color(rgba(PLACEHOLDER))
                .caret_color(rgb(CARET))
                .selection_color(rgba(SELECTION))
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.new(|cx| {
            cx.observe(&input, |bar: &mut FindBar, _, cx| bar.edited(cx))
                .detach();
            FindBar {
                ui_view,
                input,
                tab: 0,
                searched: String::new(),
            }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| crate::ns_view(window)) {
                let (right, top) = (f64::from(RIGHT), f64::from(TOP));
                unsafe { platform::host_add_find_bar(ui_view, view, right, top) };
                cx.global_mut::<FindBars>().0.insert(browser, window);
            }
        }
        Err(error) => log!("couldn't open the find bar: {error}"),
    }
}

/// The bar over the browser window `browser`.
fn window(browser: bc::bc_id, cx: &App) -> Option<WindowHandle<FindBar>> {
    cx.try_global::<FindBars>().and_then(|bars| bars.0.get(&browser).copied())
}

/// Opens `tab`'s find bar over the browser window `browser`, or selects its text if it's open, and
/// gives it the keyboard.
pub fn show(browser: bc::bc_id, tab: u64, cx: &mut App) {
    let Some(window) = window(browser, cx).filter(|_| tab != 0) else {
        return;
    };
    let text = open_find(tab);
    FINDS.with_borrow_mut(|finds| {
        let find = finds.entry(tab).or_default();
        find.open = true;
        find.text = text.clone();
    });
    let ui_view = window
        .update(cx, |bar, window, cx| {
            bar.tab = tab;
            bar.searched = text.clone();
            bar.input
                .update(cx, |input, cx| input.reset(text.into(), window, cx));
            bar.ui_view
        });
    if let Ok(ui_view) = ui_view {
        unsafe { platform::host_show_find_bar(ui_view, true) };
    }
}

/// Goes to `tab`'s next match, or the one before, opening its find bar over the browser window
/// `browser` if it's closed, as Chrome's ⌘G does.
pub fn find_next(browser: bc::bc_id, tab: u64, forward: bool, cx: &mut App) {
    let open = FINDS.with_borrow(|finds| finds.get(&tab).is_some_and(|find| find.open));
    if !open {
        show(browser, tab, cx);
    }
    if let Some(bar) = window(browser, cx).and_then(|window| window.read(cx).ok()) {
        let text = bar.input.read(cx).text();
        if !text.is_empty() {
            find(tab, text, forward);
        }
    }
}

/// The browser window `browser` shows `tab`, or no page (0): its bar shows the tab's find if
/// that's open.
pub fn tab_shown(browser: bc::bc_id, tab: u64, cx: &mut App) {
    let Some(window) = window(browser, cx) else {
        return;
    };
    let find = FINDS.with_borrow(|finds| finds.get(&tab).filter(|find| find.open).cloned());
    let changed = window
        .update(cx, |bar, window, cx| {
            if bar.tab == tab {
                return None;
            }
            bar.tab = tab;
            if let Some(find) = &find {
                bar.searched = find.text.clone();
                bar.input.update(cx, |input, cx| {
                    input.reset(find.text.clone().into(), window, cx)
                });
            }
            cx.notify();
            Some(bar.ui_view)
        })
        .ok()
        .flatten();
    if let Some(ui_view) = changed {
        match find {
            Some(_) => unsafe { platform::host_show_find_bar(ui_view, false) },
            None => unsafe { platform::host_hide_find_bar(ui_view) },
        }
    }
}

/// The core reports `tab`'s find.
pub fn changed(tab: u64, find: Find, cx: &mut App) {
    // check.sh reads these lines.
    if find.open {
        log!(
            "browser: find {tab} open {} of {} {}",
            find.selected, find.matches, find.text
        );
    } else {
        log!("browser: find {tab} closed");
    }
    let open = find.open;
    FINDS.with_borrow_mut(|finds| finds.insert(tab, find));
    let windows: Vec<_> = match cx.try_global::<FindBars>() {
        Some(bars) => bars.0.values().copied().collect(),
        None => Vec::new(),
    };
    for window in windows {
        let shown = window
            .update(cx, |bar, _, cx| {
                cx.notify();
                (bar.tab == tab).then_some(bar.ui_view)
            })
            .ok()
            .flatten();
        // A tab going to another page closes its find bar.
        if let Some(ui_view) = shown
            && !open
        {
            unsafe { platform::host_hide_find_bar(ui_view) };
        }
    }
}

/// `tab` closed.
pub fn forget(tab: u64) {
    FINDS.with_borrow_mut(|finds| finds.remove(&tab));
}

/// Opens `tab`'s find in the core and returns its text.
fn open_find(tab: u64) -> String {
    // Long enough for most, so the core opens it once.
    let mut text = vec![0u8; 256];
    let length = unsafe { bc::bc_tab_find_open(tab, text.as_mut_ptr().cast(), text.len()) };
    if length >= text.len() {
        text.resize(length + 1, 0);
        unsafe { bc::bc_tab_find_open(tab, text.as_mut_ptr().cast(), text.len()) };
    }
    text.truncate(length);
    String::from_utf8_lossy(&text).into_owned()
}

fn find(tab: u64, text: &str, forward: bool) {
    if let Ok(text) = CString::new(text) {
        unsafe { bc::bc_tab_find(tab, text.as_ptr(), forward) };
    }
}

impl FindBar {
    /// Finds the text as the user types it, as Chrome's find bar does, once an input method has
    /// committed it.
    fn edited(&mut self, cx: &mut Context<Self>) {
        let input = self.input.read(cx);
        if input.composing() || input.text() == self.searched {
            return;
        }
        self.searched = input.text().to_string();
        find(self.tab, &self.searched, true);
    }

    fn find_next(&mut self, forward: bool, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text();
        if !text.is_empty() {
            find(self.tab, text, forward);
        }
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        let text: SharedString = self.input.read(cx).text().to_string().into();
        self.input
            .update(cx, |input, cx| input.reset(text, window, cx));
    }

    fn close(&mut self, _: &Close, _: &mut Window, _: &mut Context<Self>) {
        unsafe { bc::bc_tab_find_close(self.tab) };
        FINDS.with_borrow_mut(|finds| {
            if let Some(find) = finds.get_mut(&self.tab) {
                find.open = false;
            }
        });
        unsafe { platform::host_hide_find_bar(self.ui_view) };
    }

    /// What the page found for the text, once it has looked: which match of how many, or none.
    fn found(&self) -> Option<(i32, i32)> {
        FINDS.with_borrow(|finds| match finds.get(&self.tab) {
            Some(find) if !find.text.is_empty() && find.matches >= 0 => {
                Some((find.selected, find.matches))
            }
            _ => None,
        })
    }

    /// A 28 pt button with `icon`, which dims while it can't be used.
    fn button(id: &'static str, icon: &'static str, enabled: bool) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .flex_none()
            .size(px(BUTTON))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(RADIUS))
            .when(enabled, |button| button.hover(|style| style.bg(rgba(HOVERED))))
            .child(svg().path(icon).size(px(GLYPH)).text_color(rgba(if enabled {
                GLYPH_COLOR
            } else {
                GLYPH_DISABLED
            })))
    }
}

impl Render for FindBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let found = self.found();
        // Arc's count, "1/12", and its previous and next, or "NOT FOUND" in their place.
        let (status, matches) = match found {
            Some((_, 0)) => ("NOT FOUND".to_string(), None),
            Some((selected, matches)) => (format!("{selected}/{matches}"), Some(matches)),
            None => (String::new(), None),
        };
        // check.sh reads these lines.
        log!(
            "browser: find bar drew \"{}\" {status}",
            self.input.read(cx).text()
        );
        let enabled = matches.is_some();
        let not_found = found.is_some() && !enabled;
        let previous = Self::button("previous", "icons/chevron-up.svg", enabled)
            .when(enabled, |button| {
                button.on_click(cx.listener(|bar, _, _, cx| bar.find_next(false, cx)))
            });
        let next = Self::button("next", "icons/chevron-down.svg", enabled)
            .ml(px(-ARROW_OVERLAP))
            .when(enabled, |button| {
                button.on_click(cx.listener(|bar, _, _, cx| bar.find_next(true, cx)))
            });
        let status = div()
            .flex_none()
            .mr(px(COUNT_GAP))
            .when(not_found, |status| {
                status
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(NOT_FOUND))
            })
            .when(!not_found, |status| status.text_size(px(12.)).text_color(rgba(COUNT)))
            .child(status);
        let divider = div()
            .flex_none()
            .w(px(DIVIDER_GAP))
            .h(px(DIVIDER_HEIGHT))
            .flex()
            .justify_center()
            .child(div().w(px(1.)).h_full().bg(rgba(DIVIDER)));
        let close = Self::button("close", "icons/x.svg", true)
            .on_click(cx.listener(|bar, _, window, cx| bar.close(&Close, window, cx)));
        let search = div()
            .flex_none()
            .size(px(BUTTON))
            .flex()
            .items_center()
            .justify_center()
            .child(svg().path("icons/search.svg").size(px(GLYPH)).text_color(rgba(GLYPH_COLOR)));
        // The browser's shortcuts work here too, below the bar's own keys.
        div().key_context(shell::KEY_CONTEXT).size_full().child(
            div()
                .key_context("FindBar")
                .on_action(cx.listener(|bar, _: &FindNext, _, cx| bar.find_next(true, cx)))
                .on_action(cx.listener(|bar, _: &FindPrevious, _, cx| bar.find_next(false, cx)))
                .on_action(cx.listener(Self::select_all))
                .on_action(cx.listener(Self::close))
                .size_full()
                .flex()
                .items_center()
                .px(px(INSET))
                .rounded(px(RADIUS))
                .bg(rgb(FILL))
                .text_color(rgb(TEXT))
                .text_size(px(13.))
                .line_height(px(18.))
                .child(search)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .ml(px(FIELD_GAP))
                        .overflow_hidden()
                        .child(self.input.clone()),
                )
                .child(status)
                .when(!not_found, |bar| bar.child(previous).child(next))
                .child(divider)
                .child(close),
        )
    }
}
