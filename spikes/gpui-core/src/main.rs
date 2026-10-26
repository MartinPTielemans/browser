//! Phase 2 spike: GPUI on our own engine. The core (core/, in Chrome's framework) makes the
//! browser window and puts the page in it, through its C API; GPUI draws the sidebar in a view
//! inside that window, under the page. Chromium owns the application object and the run loop.

use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_int, c_void},
};

use gpui::{
    AnyWindowHandle, App, ApplicationHandle, AsyncApp, Context, KeyBinding, Menu, MenuItem, Pixels,
    SharedString, Size, WeakEntity, Window, WindowOptions, actions, div, prelude::*, px, rgb, rgba,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod command_bar;
mod text_input;

/// The core's API, generated from core/public/browser_core.h.
#[allow(non_camel_case_types, dead_code)]
mod bc {
    include!(concat!(env!("OUT_DIR"), "/browser_core.rs"));
}

mod host {
    use std::ffi::c_void;

    unsafe extern "C" {
        pub fn spike_drive_start();
        pub fn spike_host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
        pub fn spike_host_ui_view() -> *mut c_void;
        pub fn spike_host_show_window(ui_view: *mut c_void);
        pub fn spike_host_add_overlay(overlay_view: *mut c_void);
        pub fn spike_host_show_overlay();
        pub fn spike_host_hide_overlay();
    }
}

const WINDOW_SIZE: (f64, f64) = (1280., 800.);
const SIDEBAR_WIDTH: f32 = 240.;
const GAP: f32 = 8.;
const START_URL: &str = "https://en.wikipedia.org/wiki/Arc_(web_browser)";

/// The page to open: `SPIKE_URL`, or the Arc article.
fn start_url() -> CString {
    let url = std::env::var("SPIKE_URL").unwrap_or_else(|_| START_URL.to_string());
    CString::new(url).unwrap_or_else(|_| CString::from(c"about:blank"))
}

actions!(spike, [Quit, NewTab, EditUrl, CloseTab, Find]);

thread_local! {
    // The core calls back into the UI from Chromium's event handling, outside any GPUI update.
    static SHELL: RefCell<Option<(AsyncApp, WeakEntity<Shell>, AnyWindowHandle)>> =
        const { RefCell::new(None) };
    // GPUI starts inside Chromium's run loop, from a callback, and lives as long as the process.
    static APP: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    // The spike has one tab.
    static TAB: Cell<bc::bc_id> = const { Cell::new(0) };
    // Whether the tab's page is full screen, as when a video's full-screen button is pressed.
    static PAGE_FULLSCREEN: Cell<bool> = const { Cell::new(false) };
}

struct Shell {
    last_command: SharedString,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let tab = |title: &'static str, selected: bool| {
            div()
                .px_2()
                .py_1p5()
                .rounded_md()
                .text_sm()
                .text_color(rgb(0xffffff))
                .when(selected, |this| this.bg(rgba(0xffffff26)))
                .child(title)
        };
        div()
            .size_full()
            .flex()
            .bg(rgb(0x3b2f63))
            .child(
                div()
                    .w(px(SIDEBAR_WIDTH))
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .pt(px(44.))
                    .px_2()
                    .child(tab("Arc (web browser) - Wikipedia", true))
                    .child(tab("Hacker News", false))
                    .child(tab("GPUI", false))
                    .child(
                        div()
                            .mt_4()
                            .px_2()
                            .text_xs()
                            .text_color(rgba(0xffffffa0))
                            .child(self.last_command.clone()),
                    ),
            )
            // The page view sits on top of this, as a native view.
            .child(div().flex_1())
    }
}

/// The window's NSView, for the host.
fn ns_view(window: &Window) -> *mut c_void {
    match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::AppKit(handle)) => handle.ns_view.as_ptr(),
        _ => panic!("the window has no AppKit view"),
    }
}

/// Shows the tab right of the sidebar, inset by the gap, in a window whose content is `size`. A
/// full-screen page covers the whole window, sidebar included.
fn show_page(size: Size<Pixels>) {
    let tab = TAB.get();
    if tab == 0 {
        return;
    }
    let (width, height) = (f64::from(size.width), f64::from(size.height));
    let (left, gap) = if PAGE_FULLSCREEN.get() {
        (0., 0.)
    } else {
        (SIDEBAR_WIDTH as f64, GAP as f64)
    };
    let frame = bc::bc_rect {
        x: left,
        y: gap,
        width: width - left - gap,
        height: height - 2. * gap,
    };
    unsafe { bc::bc_tab_show(tab, frame) };
}

/// Shows the tab again, after it changed or went full screen.
fn show_page_again() {
    let shell =
        SHELL.with_borrow(|shell| shell.as_ref().map(|(cx, _, window)| (cx.clone(), *window)));
    match shell {
        Some((mut cx, window)) => {
            cx.update_window(window, |_, window, _| show_page(window.viewport_size()))
                .ok();
        }
        // Without the UI, the spike lays the page out for the size the window opened at.
        None => show_page(Size::new(
            px(WINDOW_SIZE.0 as f32),
            px(WINDOW_SIZE.1 as f32),
        )),
    }
}

/// Loads `url` in the tab.
fn load_url(url: &CStr) {
    let tab = TAB.get();
    if tab != 0 {
        unsafe { bc::bc_tab_navigate(tab, url.as_ptr()) };
    }
}

/// Runs a browser command, logs it and shows it in the sidebar. `from` says which side had the
/// key: GPUI's key bindings ("ui") or Chromium handing it back ("page").
fn run_command(name: &str, from: &str, cx: &mut App) {
    // check.sh waits for these lines.
    eprintln!("spike: command {name} via {from}");
    let shell = SHELL.with_borrow(|shell| shell.as_ref().map(|(_, shell, _)| shell.clone()));
    if let Some(shell) = shell {
        shell
            .update(cx, |shell, cx| {
                shell.last_command = format!("{name} via {from}").into();
                cx.notify();
            })
            .ok();
    }
    match name {
        "new-tab" | "edit-url" => command_bar::show(cx),
        "quit" => unsafe { bc::bc_quit() },
        // The spike has one tab and no find bar, so these only show up in the sidebar.
        _ => {}
    }
}

/// The browser's shortcuts while the page has the keyboard: key, command and whether it's reserved.
/// As in Chrome, the reserved ones never reach the page. The others reach it first and come back if
/// it doesn't keep them, so a page like Google Docs can keep ⌘F.
const PAGE_SHORTCUTS: [(&str, &str, bool); 5] = [
    ("t", "new-tab", true),
    ("w", "close-tab", true),
    ("q", "quit", true),
    ("l", "edit-url", false),
    ("f", "find", false),
];

/// Runs the shortcut `key` is, if it's one of the page shortcuts that are `reserved` or aren't.
fn run_page_shortcut(key: *const bc::bc_key, reserved: bool) -> bool {
    let key = unsafe { &*key };
    if key.modifiers != bc::BC_MODIFIER_META {
        return false;
    }
    let name = unsafe { CStr::from_ptr(key.key) }.to_string_lossy();
    let Some(&(_, command, _)) = PAGE_SHORTCUTS
        .iter()
        .find(|(shortcut, _, is_reserved)| *shortcut == name && *is_reserved == reserved)
    else {
        return false;
    };
    let cx = SHELL.with_borrow(|shell| shell.as_ref().map(|(cx, _, _)| cx.clone()));
    match cx {
        Some(cx) => cx.update(|cx| run_command(command, "page", cx)),
        // Without the UI, only quitting does anything.
        None if command == "quit" => unsafe { bc::bc_quit() },
        None => return false,
    }
    true
}

unsafe extern "C" fn tab_key_before_page(
    _: *mut c_void,
    _: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    run_page_shortcut(key, true)
}

unsafe extern "C" fn tab_key_unhandled(
    _: *mut c_void,
    _: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    run_page_shortcut(key, false)
}

/// Opens the browser window centered on the main display, with `ui_view` filling it under the
/// page, and the tab in it.
fn open_browser_window(ui_view: *mut c_void) -> Option<bc::bc_id> {
    let (width, height) = WINDOW_SIZE;
    let (mut x, mut y) = (0., 0.);
    unsafe { host::spike_host_centered(width, height, &mut x, &mut y) };
    let bounds = bc::bc_rect {
        x,
        y,
        width,
        height,
    };
    let window = unsafe { bc::bc_window_open(bounds, ui_view) };
    if window == 0 {
        return None;
    }
    let url = start_url();
    TAB.set(unsafe { bc::bc_tab_open(window, url.as_ptr(), false) });
    Some(window)
}

/// Opens the browser window, and GPUI's view inside it, under the page.
fn open_window(cx: &mut App) {
    let ui_view = unsafe { host::spike_host_ui_view() };
    if open_browser_window(ui_view).is_none() {
        return;
    }
    gpui_macos::open_next_window_in(ui_view);
    let window = cx.open_window(WindowOptions::default(), |window, cx| {
        let shell = cx.new(|cx| {
            // The page keeps its place beside the sidebar as the window resizes.
            cx.observe_window_bounds(window, |_, window, _| show_page(window.viewport_size()))
                .detach();
            Shell {
                last_command: "Press ⌘T, ⌘L, ⌘W or ⌘F".into(),
            }
        });
        SHELL.set(Some((
            cx.to_async(),
            shell.downgrade(),
            Window::window_handle(window),
        )));
        shell
    });
    match window {
        Ok(window) => {
            window
                .update(cx, |_, window, _| show_page(window.viewport_size()))
                .ok();
            unsafe { host::spike_host_show_window(ui_view) };
            command_bar::open(cx);
        }
        Err(error) => {
            eprintln!("couldn't open the window: {error}");
            unsafe { bc::bc_quit() };
        }
    }
}

unsafe extern "C" fn started(_: *mut c_void) {
    // The page alone, as the baseline for what GPUI costs it.
    if std::env::var_os("SPIKE_PLAIN").is_some() {
        let ui_view = unsafe { host::spike_host_ui_view() };
        if open_browser_window(ui_view).is_some() {
            let (width, height) = WINDOW_SIZE;
            show_page(Size::new(px(width as f32), px(height as f32)));
            unsafe { host::spike_host_show_window(ui_view) };
        }
        return;
    }
    let app = gpui_platform::application().run_embedded(|cx| {
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-t", NewTab, None),
            KeyBinding::new("cmd-l", EditUrl, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("cmd-f", Find, None),
        ]);
        cx.on_action(|_: &Quit, cx| run_command("quit", "ui", cx));
        text_input::bind_keys(cx);
        cx.on_action(|_: &NewTab, cx| run_command("new-tab", "ui", cx));
        cx.on_action(|_: &EditUrl, cx| run_command("edit-url", "ui", cx));
        cx.on_action(|_: &CloseTab, cx| run_command("close-tab", "ui", cx));
        cx.on_action(|_: &Find, cx| run_command("find", "ui", cx));
        cx.set_menus([Menu {
            name: "Spike".into(),
            items: vec![MenuItem::action("Quit Spike", Quit)],
            disabled: false,
        }]);
        open_window(cx);
    });
    APP.set(Some(app));
}

// The spike has one window, so closing it quits, as its close button does in Chrome's last window.
unsafe extern "C" fn window_closed(_: *mut c_void, _: bc::bc_id) {
    eprintln!("spike: window closed");
    unsafe { bc::bc_quit() };
}

unsafe extern "C" fn tab_closed(_: *mut c_void, _: bc::bc_id) {
    eprintln!("spike: tab closed");
}

unsafe extern "C" fn stopping(_: *mut c_void) {
    eprintln!("spike: stopping");
}

// check.sh reads these lines. Test pages report what reached them through their title.
unsafe extern "C" fn tab_loaded(_: *mut c_void, _: bc::bc_id, url: *const c_char) {
    eprintln!(
        "spike: loaded {}",
        unsafe { CStr::from_ptr(url) }.to_string_lossy()
    );
}

unsafe extern "C" fn tab_fullscreen_changed(_: *mut c_void, tab: bc::bc_id, fullscreen: bool) {
    if tab != TAB.get() || fullscreen == PAGE_FULLSCREEN.get() {
        return;
    }
    PAGE_FULLSCREEN.set(fullscreen);
    show_page_again();
    eprintln!(
        "spike: page {} full screen",
        if fullscreen { "entered" } else { "left" }
    );
}

// The spike has one tab and no list of them, so a page's new tab or popup opens in its place, and
// a background tab stays hidden.
unsafe extern "C" fn tab_opened(
    _: *mut c_void,
    _: bc::bc_id,
    tab: bc::bc_id,
    _: bc::bc_id,
    foreground: bool,
) {
    eprintln!(
        "spike: tab opened in the {}",
        if foreground {
            "foreground"
        } else {
            "background"
        }
    );
    if !foreground {
        return;
    }
    let old = TAB.replace(tab);
    show_page_again();
    if old != 0 {
        unsafe { bc::bc_tab_close(old, false) };
    }
}

unsafe extern "C" fn tab_title_changed(_: *mut c_void, _: bc::bc_id, title: *const c_char) {
    eprintln!(
        "spike: title {}",
        unsafe { CStr::from_ptr(title) }.to_string_lossy()
    );
}

fn main() {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut args: Vec<String> = std::env::args().take(1).collect();
    args.extend([
        // A profile of the spike's own, never the owner's.
        format!("--user-data-dir={home}/Library/Caches/browser-spike-core"),
        // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed build
        // asking for it gets a password prompt, and every request that uses cookies waits.
        "--use-mock-keychain".into(),
        // Chrome would copy the app bundle to check its signature against, which an
        // ad-hoc-signed build doesn't need.
        "--disable-features=MacAppCodeSignClone".into(),
    ]);
    // Switches given to the spike come after, so they override these: Chromium keeps a repeated
    // switch's last value.
    args.extend(std::env::args().skip(1));
    let args: Vec<CString> = args
        .into_iter()
        .filter_map(|arg| CString::new(arg).ok())
        .collect();
    if std::env::var_os("SPIKE_DRIVE").is_some() {
        unsafe { host::spike_drive_start() };
    }
    let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    let host = bc::bc_host {
        started: Some(started),
        stopping: Some(stopping),
        window_closed: Some(window_closed),
        tab_closed: Some(tab_closed),
        tab_opened: Some(tab_opened),
        tab_key_before_page: Some(tab_key_before_page),
        tab_key_unhandled: Some(tab_key_unhandled),
        tab_fullscreen_changed: Some(tab_fullscreen_changed),
        tab_loaded: Some(tab_loaded),
        tab_title_changed: Some(tab_title_changed),
        ..Default::default()
    };
    let code = unsafe { bc::bc_main(argv.len() as c_int, argv.as_mut_ptr(), &host) };
    std::process::exit(code);
}
