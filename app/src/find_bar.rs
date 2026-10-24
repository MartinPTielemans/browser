//! The find bar, over the top right of the page. GPUI can't draw over the page's native view, so
//! the bar is a window of its own that the host shows over the browser window, as its child,
//! while the tab shown has find in page open. The core keeps each tab's find; the bar shows the
//! shown tab's.

use std::{cell::RefCell, collections::HashMap, ffi::CString};

use browser::{shell, text_input::TextInput};
use gpui::{
    App, Bounds, Context, Entity, Focusable, Global, KeyBinding, SharedString, TitlebarOptions,
    Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, div, point, prelude::*,
    px, rgb, size,
};

use crate::{bc, platform};

actions!(find_bar, [FindNext, FindPrevious, SelectAll, Close]);

const WIDTH: f32 = 300.;
const HEIGHT: f32 = 44.;
// From the top right corner of the window's content: inside the page's card, as far again from its
// edges as the card is from the window's. Arc's isn't measured yet.
const RIGHT: f32 = shell::CARD_INSET * 2.;
const TOP: f32 = shell::CARD_INSET * 2.;

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
    input: Entity<TextInput>,
    /// The tab the window shows, whose find this bar is.
    tab: u64,
    /// What the page was last asked to find, so a change that leaves the text as it was doesn't
    /// search again.
    searched: String,
}

struct FindBarWindow(WindowHandle<FindBar>);

impl Global for FindBarWindow {}

/// Opens the find bar's window, hidden, over the browser window, ahead of time as the command bar
/// does.
pub fn open(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", FindNext, Some("FindBar")),
        KeyBinding::new("shift-enter", FindPrevious, Some("FindBar")),
        // The browser's ⌘G, ⇧⌘G and ⌘F, for the bar while it has the keyboard.
        KeyBinding::new("cmd-g", FindNext, Some("FindBar")),
        KeyBinding::new("shift-cmd-g", FindPrevious, Some("FindBar")),
        KeyBinding::new("cmd-f", SelectAll, Some("FindBar")),
        KeyBinding::new("escape", Close, Some("FindBar")),
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
        let input = cx.new(|cx| TextInput::new("Find in page", cx));
        window.focus(&input.focus_handle(cx), cx);
        cx.new(|cx| {
            cx.observe(&input, |bar: &mut FindBar, _, cx| bar.edited(cx))
                .detach();
            FindBar {
                input,
                tab: 0,
                searched: String::new(),
            }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| crate::ns_view(window)) {
                unsafe { platform::host_add_find_bar(view, f64::from(RIGHT), f64::from(TOP)) };
                cx.set_global(FindBarWindow(window));
            }
        }
        Err(error) => eprintln!("couldn't open the find bar: {error}"),
    }
}

fn window(cx: &App) -> Option<WindowHandle<FindBar>> {
    cx.try_global::<FindBarWindow>().map(|bar| bar.0)
}

/// Opens `tab`'s find bar, or selects its text if it's open, and gives it the keyboard.
pub fn show(tab: u64, cx: &mut App) {
    let Some(window) = window(cx).filter(|_| tab != 0) else {
        return;
    };
    let text = open_find(tab);
    FINDS.with_borrow_mut(|finds| {
        let find = finds.entry(tab).or_default();
        find.open = true;
        find.text = text.clone();
    });
    window
        .update(cx, |bar, window, cx| {
            bar.tab = tab;
            bar.searched = text.clone();
            bar.input
                .update(cx, |input, cx| input.reset(text.into(), window, cx));
        })
        .ok();
    unsafe { platform::host_show_find_bar(true) };
}

/// Goes to `tab`'s next match, or the one before, opening its find bar if it's closed, as Chrome's
/// ⌘G does.
pub fn find_next(tab: u64, forward: bool, cx: &mut App) {
    let open = FINDS.with_borrow(|finds| finds.get(&tab).is_some_and(|find| find.open));
    if !open {
        show(tab, cx);
    }
    if let Some(bar) = window(cx).and_then(|window| window.read(cx).ok()) {
        let text = bar.input.read(cx).text();
        if !text.is_empty() {
            find(tab, text, forward);
        }
    }
}

/// The window shows `tab`, or no page (0): the bar shows its find if that's open.
pub fn tab_shown(tab: u64, cx: &mut App) {
    let Some(window) = window(cx) else {
        return;
    };
    let find = FINDS.with_borrow(|finds| finds.get(&tab).filter(|find| find.open).cloned());
    let changed = window
        .update(cx, |bar, window, cx| {
            if bar.tab == tab {
                return false;
            }
            bar.tab = tab;
            if let Some(find) = &find {
                bar.searched = find.text.clone();
                bar.input.update(cx, |input, cx| {
                    input.reset(find.text.clone().into(), window, cx)
                });
            }
            cx.notify();
            true
        })
        .unwrap_or(false);
    if changed {
        match find {
            Some(_) => unsafe { platform::host_show_find_bar(false) },
            None => unsafe { platform::host_hide_find_bar() },
        }
    }
}

/// The core reports `tab`'s find.
pub fn changed(tab: u64, find: Find, cx: &mut App) {
    // check.sh reads these lines.
    if find.open {
        eprintln!(
            "browser: find {tab} open {} of {} {}",
            find.selected, find.matches, find.text
        );
    } else {
        eprintln!("browser: find {tab} closed");
    }
    let open = find.open;
    FINDS.with_borrow_mut(|finds| finds.insert(tab, find));
    let shown = window(cx)
        .and_then(|window| {
            window
                .update(cx, |bar, _, cx| {
                    cx.notify();
                    bar.tab == tab
                })
                .ok()
        })
        .unwrap_or(false);
    // A tab going to another page closes its find bar.
    if shown && !open {
        unsafe { platform::host_hide_find_bar() };
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
        unsafe { platform::host_hide_find_bar() };
    }

    /// "n of m" once the page has looked for the text.
    fn count(&self) -> String {
        FINDS.with_borrow(|finds| match finds.get(&self.tab) {
            Some(find) if !find.text.is_empty() && find.matches >= 0 => {
                format!("{} of {}", find.selected, find.matches)
            }
            _ => String::new(),
        })
    }
}

impl Render for FindBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.count();
        // check.sh reads these lines.
        eprintln!(
            "browser: find bar drew \"{}\" {count}",
            self.input.read(cx).text()
        );
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
                .gap_3()
                .px_4()
                // Grey, not white: it stays over the page, and most pages are white.
                .bg(rgb(0xececee))
                .text_color(rgb(0x1d1d1f))
                .text_size(px(14.))
                .line_height(px(20.))
                .child(div().flex_1().overflow_hidden().child(self.input.clone()))
                .child(div().flex_none().text_color(rgb(0x86868b)).child(count)),
        )
    }
}
