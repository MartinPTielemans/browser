//! The browser. The core (core/, in Chrome's framework) makes the browser window and puts the page
//! in it, through its C API; GPUI draws the shell in a view inside that window, under the page.
//! Chromium owns the application object and the run loop.

use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_int, c_void},
    rc::Rc,
    sync::Arc,
    time::Instant,
};

use browser::shell::{self, Browser, Command, Page, Shell};
use gpui::{
    AnyWindowHandle, App, ApplicationHandle, AsyncApp, Context, Focusable, Image, ImageFormat, Menu,
    MenuItem, Pixels, SharedString, Size, WeakEntity, Window, WindowOptions, actions, prelude::*,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod command_bar;
mod find_bar;

/// The core's API, generated from core/public/browser_core.h.
#[allow(non_camel_case_types, dead_code)]
mod bc {
    include!(concat!(env!("OUT_DIR"), "/browser_core.rs"));
}

/// The AppKit side, in platform/mac/.
mod platform {
    use std::ffi::c_void;

    unsafe extern "C" {
        pub fn drive_start();
        pub fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
        pub fn host_ui_view() -> *mut c_void;
        pub fn host_show_window(ui_view: *mut c_void);
        pub fn host_add_overlay(overlay_view: *mut c_void);
        pub fn host_show_overlay();
        pub fn host_hide_overlay();
        pub fn host_add_find_bar(find_bar_view: *mut c_void, right: f64, top: f64);
        pub fn host_show_find_bar(keyboard: bool);
        pub fn host_hide_find_bar();
        pub fn host_focus(view: *mut c_void);
    }
}

const WINDOW_SIZE: (f64, f64) = (1280., 800.);
const START_URL: &str = "https://en.wikipedia.org/wiki/Arc_(web_browser)";

actions!(browser, [Quit]);

thread_local! {
    // The core calls back into the UI from Chromium's event handling, outside any GPUI update.
    static UI: RefCell<Option<(AsyncApp, WeakEntity<Shell>, AnyWindowHandle)>> =
        const { RefCell::new(None) };
    // GPUI starts inside Chromium's run loop, from a callback, and lives as long as the process.
    static APP: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    // The browser window. There's one so far.
    static WINDOW: Cell<bc::bc_id> = const { Cell::new(0) };
}

/// Changes the shell, from a core callback.
fn update_shell(change: impl FnOnce(&mut Shell, &mut Context<Shell>)) {
    let ui = UI.with_borrow(|ui| ui.as_ref().map(|(cx, shell, _)| (cx.clone(), shell.clone())));
    if let Some((cx, shell)) = ui {
        cx.update(|cx| shell.update(cx, change).ok());
    }
}

/// Changes the UI outside the shell, from a core callback.
fn update_app(change: impl FnOnce(&mut App)) {
    let cx = UI.with_borrow(|ui| ui.as_ref().map(|(cx, _, _)| cx.clone()));
    if let Some(cx) = cx {
        cx.update(change);
    }
}

/// The window's NSView, for the host.
fn ns_view(window: &Window) -> *mut c_void {
    match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::AppKit(handle)) => handle.ns_view.as_ptr(),
        _ => panic!("the window has no AppKit view"),
    }
}

/// Tells the core where the page goes in a window whose content is `viewport`.
fn set_page_frame(viewport: Size<Pixels>, fullscreen: bool) {
    let frame = shell::page_frame(viewport, fullscreen);
    let frame = bc::bc_rect {
        x: f64::from(frame.origin.x),
        y: f64::from(frame.origin.y),
        width: f64::from(frame.size.width),
        height: f64::from(frame.size.height),
    };
    unsafe { bc::bc_window_set_page_frame(WINDOW.get(), frame) };
}

/// Opens a tab loading `url` at the top of the Space's Today tabs, and shows it.
fn open_tab(url: &str) {
    let Ok(url) = CString::new(url) else {
        return;
    };
    let window = WINDOW.get();
    let tab = unsafe { bc::bc_tab_open(window, url.as_ptr()) };
    if tab != 0 {
        unsafe { bc::bc_window_select(window, tab) };
    }
}

/// Makes a Space after the others.
fn create_space(name: &str) -> bc::bc_id {
    let name = CString::new(name).unwrap_or_default();
    unsafe { bc::bc_space_create(name.as_ptr()) }
}

/// Loads `url` in `tab`.
fn navigate(tab: u64, url: &str) {
    if let Ok(url) = CString::new(url) {
        unsafe { bc::bc_tab_navigate(tab, url.as_ptr()) };
    }
}

/// The page the address bar goes to for `text` typed in it: the one it names, else a search.
fn typed_url(text: &str) -> Option<String> {
    let text = CString::new(text).ok()?;
    // Long enough for most, so the core reads what was typed once.
    let mut url = vec![0u8; 2048];
    let length = unsafe { bc::bc_typed_url(text.as_ptr(), url.as_mut_ptr().cast(), url.len()) };
    if length >= url.len() {
        url.resize(length + 1, 0);
        unsafe { bc::bc_typed_url(text.as_ptr(), url.as_mut_ptr().cast(), url.len()) };
    }
    url.truncate(length);
    (length > 0).then(|| String::from_utf8_lossy(&url).into_owned())
}

/// The shell's requests, as calls to the core. The core reports back once each call has returned.
struct Engine {
    /// GPUI's view, which has the keyboard when the UI does.
    view: *mut c_void,
}

impl Browser for Engine {
    fn select(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_window_select(WINDOW.get(), tab) };
    }

    fn close(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_close(tab) };
    }

    fn set_pinned(&self, tab: u64, pinned: bool, _: &mut App) {
        unsafe { bc::bc_tab_set_pinned(tab, pinned) };
    }

    fn back_to_pinned_url(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_back_to_pinned_url(tab) };
    }

    fn clear(&self, _: &mut App) {
        unsafe { bc::bc_window_clear(WINDOW.get()) };
    }

    fn undo_clear(&self, _: &mut App) {
        unsafe { bc::bc_window_undo_clear(WINDOW.get()) };
    }

    fn go_back(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_go_back(tab) };
    }

    fn go_forward(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_go_forward(tab) };
    }

    fn reload(&self, tab: u64, bypass_cache: bool, _: &mut App) {
        unsafe { bc::bc_tab_reload(tab, bypass_cache) };
    }

    fn stop(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_stop(tab) };
    }

    fn show_space(&self, space: u64, _: &mut App) {
        unsafe { bc::bc_window_show_space(WINDOW.get(), space) };
    }

    fn create_space(&self, name: &str, _: &mut App) -> u64 {
        create_space(name)
    }

    fn rename_space(&self, space: u64, name: &str, _: &mut App) {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_space_set_name(space, name.as_ptr()) };
    }

    fn delete_space(&self, space: u64, _: &mut App) {
        unsafe { bc::bc_space_delete(space) };
    }

    fn focus_ui(&self, _: &mut App) {
        unsafe { platform::host_focus(self.view) };
    }

    fn command_bar(&self, editing: Option<(u64, SharedString)>, cx: &mut App) {
        command_bar::show(editing, cx);
    }

    fn find(&self, tab: u64, cx: &mut App) {
        find_bar::show(tab, cx);
    }

    fn find_next(&self, tab: u64, forward: bool, cx: &mut App) {
        find_bar::find_next(tab, forward, cx);
    }

    fn quit(&self, _: &mut App) {
        unsafe { bc::bc_quit() };
    }
}

/// A key the page didn't keep, or one it never sees, as a keystroke in GPUI's terms, with the
/// modifiers in the order shell::SHORTCUTS writes them.
fn keystroke(key: &bc::bc_key) -> String {
    let name = unsafe { CStr::from_ptr(key.key) }.to_string_lossy();
    let mut keystroke = String::new();
    for (flag, modifier) in [
        (bc::BC_MODIFIER_CONTROL, "ctrl-"),
        (bc::BC_MODIFIER_ALT, "alt-"),
        (bc::BC_MODIFIER_SHIFT, "shift-"),
        (bc::BC_MODIFIER_META, "cmd-"),
    ] {
        if key.modifiers & flag != 0 {
            keystroke.push_str(modifier);
        }
    }
    let key = match name.as_ref() {
        "ArrowDown" => "down".to_string(),
        "ArrowUp" => "up".to_string(),
        "ArrowLeft" => "left".to_string(),
        "ArrowRight" => "right".to_string(),
        name => name.to_lowercase(),
    };
    keystroke + &key
}

/// Runs the shortcut `key` is, if it's one of the page shortcuts that are `reserved` or aren't.
fn run_page_shortcut(key: *const bc::bc_key, reserved: bool) -> bool {
    let Some(command) = Command::for_keystroke(&keystroke(unsafe { &*key }), reserved) else {
        return false;
    };
    let mut ran = false;
    update_shell(|shell, cx| {
        shell.run(command, "page", cx);
        ran = true;
    });
    ran
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
/// page.
fn open_browser_window(ui_view: *mut c_void) -> bool {
    let (width, height) = WINDOW_SIZE;
    let (mut x, mut y) = (0., 0.);
    unsafe { platform::host_centered(width, height, &mut x, &mut y) };
    let bounds = bc::bc_rect {
        x,
        y,
        width,
        height,
    };
    let window = unsafe { bc::bc_window_open(bounds, ui_view) };
    if window == 0 {
        return false;
    }
    WINDOW.set(window);
    true
}

/// Shows the Spaces as the user left them, else a first Space with the first tab. A page given in
/// BROWSER_URL opens on top either way.
fn restore_spaces() {
    let window = WINDOW.get();
    let start = Instant::now();
    let restored = unsafe { bc::bc_window_restore(window) };
    if restored {
        // check.sh reads this line.
        eprintln!(
            "browser: restored in {:.1} ms",
            start.elapsed().as_secs_f64() * 1000.
        );
    } else {
        unsafe { bc::bc_window_show_space(window, create_space("Space 1")) };
    }
    match std::env::var("BROWSER_URL") {
        Ok(url) => open_tab(&url),
        Err(_) if !restored => open_tab(START_URL),
        Err(_) => {}
    }
}

/// Opens the browser window, the shell inside it under the page, and its Spaces.
fn open_window(cx: &mut App) {
    let ui_view = unsafe { platform::host_ui_view() };
    if !open_browser_window(ui_view) {
        return;
    }
    gpui_macos::open_next_window_in(ui_view);
    let window = cx.open_window(WindowOptions::default(), |window, cx| {
        let shell = cx.new(|cx| {
            // The page keeps its place in the card as the window resizes.
            cx.observe_window_bounds(window, |shell: &mut Shell, window, _| {
                set_page_frame(window.viewport_size(), shell.page_fullscreen())
            })
            .detach();
            Shell::new(Rc::new(Engine { view: ns_view(window) }), cx)
        });
        window.focus(&shell.focus_handle(cx), cx);
        UI.set(Some((
            cx.to_async(),
            shell.downgrade(),
            Window::window_handle(window),
        )));
        shell
    });
    match window {
        Ok(window) => {
            window
                .update(cx, |_, window, _| set_page_frame(window.viewport_size(), false))
                .ok();
            unsafe { platform::host_show_window(ui_view) };
            command_bar::open(cx);
            find_bar::open(cx);
            // Once the page frame is set, so the tab shown loads at its size.
            restore_spaces();
        }
        Err(error) => {
            eprintln!("couldn't open the window: {error}");
            unsafe { bc::bc_quit() };
        }
    }
}

unsafe extern "C" fn started(_: *mut c_void) {
    let app = gpui_platform::application().with_assets(shell::Assets).run_embedded(|cx| {
        shell::bind_keys(cx);
        browser::text_input::bind_keys(cx);
        cx.on_action(|_: &Quit, _| unsafe { bc::bc_quit() });
        cx.set_menus([
            Menu {
                name: "Browser".into(),
                items: vec![MenuItem::action("Quit Browser", Quit)],
                disabled: false,
            },
            Menu {
                name: "Spaces".into(),
                items: vec![
                    shell::menu_item("New Space…", Command::NewSpace),
                    MenuItem::separator(),
                    shell::menu_item("Next Space", Command::NextSpace),
                    shell::menu_item("Previous Space", Command::PreviousSpace),
                    MenuItem::separator(),
                    shell::menu_item("Clear Unpinned Tabs", Command::Clear),
                ],
                disabled: false,
            },
        ]);
        open_window(cx);
    });
    APP.set(Some(app));
}

// There's one window, so closing it quits, as its close button does in Chrome's last window.
unsafe extern "C" fn window_closed(_: *mut c_void, _: bc::bc_id) {
    eprintln!("browser: window closed");
    unsafe { bc::bc_quit() };
}

unsafe extern "C" fn stopping(_: *mut c_void) {
    eprintln!("browser: stopping");
}

unsafe extern "C" fn spaces_changed(_: *mut c_void) {
    let count = unsafe { bc::bc_spaces(std::ptr::null_mut(), 0) };
    let mut ids = vec![0; count];
    let count = unsafe { bc::bc_spaces(ids.as_mut_ptr(), ids.len()) };
    ids.truncate(count);
    let spaces = ids
        .into_iter()
        .map(|space| {
            let length = unsafe { bc::bc_space_name(space, std::ptr::null_mut(), 0) };
            let mut name = vec![0u8; length + 1];
            unsafe { bc::bc_space_name(space, name.as_mut_ptr().cast(), name.len()) };
            name.truncate(length);
            (space, String::from_utf8_lossy(&name).into_owned().into())
        })
        .collect();
    update_shell(|shell, cx| shell.set_spaces(spaces, cx));
}

unsafe extern "C" fn space_changed(_: *mut c_void, space: bc::bc_id) {
    let list = |copy: unsafe extern "C" fn(bc::bc_id, *mut bc::bc_id, usize) -> usize| {
        let count = unsafe { copy(space, std::ptr::null_mut(), 0) };
        let mut tabs = vec![0; count];
        let count = unsafe { copy(space, tabs.as_mut_ptr(), tabs.len()) };
        tabs.truncate(count);
        tabs
    };
    let (pinned, today) = (list(bc::bc_space_pinned), list(bc::bc_space_today));
    update_shell(|shell, cx| shell.set_tabs(space, pinned, today, cx));
}

unsafe extern "C" fn window_space_shown(_: *mut c_void, window: bc::bc_id, space: bc::bc_id) {
    if window == WINDOW.get() {
        update_shell(|shell, cx| shell.set_space(space, cx));
    }
}

unsafe extern "C" fn window_selected(_: *mut c_void, window: bc::bc_id, tab: bc::bc_id) {
    if window == WINDOW.get() {
        update_shell(|shell, cx| {
            shell.set_selected(tab, cx);
            find_bar::tab_shown(tab, cx);
        });
    }
}

// check.sh reads these lines. Test pages report what reached them through their title.
unsafe extern "C" fn tab_loaded(_: *mut c_void, tab: bc::bc_id, url: *const c_char) {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy();
    eprintln!("browser: loaded {tab} {url}");
}

unsafe extern "C" fn tab_state_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    state: *const bc::bc_tab_state,
) {
    let state = unsafe { &*state };
    let page = Page {
        url: unsafe { CStr::from_ptr(state.url) }
            .to_string_lossy()
            .into_owned()
            .into(),
        can_go_back: state.can_go_back,
        can_go_forward: state.can_go_forward,
        loading: state.loading,
        unloaded: state.unloaded,
        away_from_pinned_url: state.away_from_pinned_url,
    };
    update_shell(|shell, cx| shell.set_page(tab, page, cx));
}

unsafe extern "C" fn tab_title_changed(_: *mut c_void, tab: bc::bc_id, title: *const c_char) {
    let title = unsafe { CStr::from_ptr(title) }.to_string_lossy().into_owned();
    eprintln!("browser: title {tab} {title}");
    update_shell(|shell, cx| shell.set_title(tab, title.into(), cx));
}

unsafe extern "C" fn tab_favicon_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    png: *const u8,
    size: usize,
) {
    let favicon = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        Arc::new(Image::from_bytes(ImageFormat::Png, png))
    });
    eprintln!("browser: favicon {tab} {size} bytes");
    update_shell(|shell, cx| shell.set_favicon(tab, favicon, cx));
}

unsafe extern "C" fn tab_opened(
    _: *mut c_void,
    _: bc::bc_id,
    tab: bc::bc_id,
    opener: bc::bc_id,
    foreground: bool,
) {
    let side = if foreground { "foreground" } else { "background" };
    eprintln!("browser: tab {tab} opened by {opener} in the {side}");
}

// Called while Chromium closes the tab, which can be inside a GPUI update that asked for it, so
// the shell hears of it from space_changed instead.
unsafe extern "C" fn tab_closed(_: *mut c_void, tab: bc::bc_id) {
    eprintln!("browser: tab {tab} closed");
    find_bar::forget(tab);
}

unsafe extern "C" fn tab_find_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    state: *const bc::bc_find_state,
) {
    let state = unsafe { &*state };
    let find = find_bar::Find {
        open: state.open,
        text: unsafe { CStr::from_ptr(state.text) }
            .to_string_lossy()
            .into_owned(),
        matches: state.matches,
        selected: state.selected,
    };
    update_app(|cx| find_bar::changed(tab, find, cx));
}

unsafe extern "C" fn tab_fullscreen_changed(_: *mut c_void, tab: bc::bc_id, fullscreen: bool) {
    let ui = UI.with_borrow(|ui| ui.as_ref().map(|(cx, shell, window)| (cx.clone(), shell.clone(), *window)));
    let Some((cx, shell, window)) = ui else {
        return;
    };
    cx.update(|cx| {
        let shown = shell.read_with(cx, |shell, _| shell.selected() == tab).unwrap_or(false);
        if !shown {
            return;
        }
        shell
            .update(cx, |shell, cx| shell.set_page_fullscreen(fullscreen, cx))
            .ok();
        cx.update_window(window, |_, window, _| {
            set_page_frame(window.viewport_size(), fullscreen)
        })
        .ok();
    });
    eprintln!(
        "browser: page {} full screen",
        if fullscreen { "entered" } else { "left" }
    );
}

fn main() {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut args: Vec<String> = std::env::args().take(1).collect();
    args.extend([
        // A profile of the dev build's own, never the owner's.
        format!("--user-data-dir={home}/Library/Caches/browser-app"),
        // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed build
        // asking for it gets a password prompt, and every request that uses cookies waits.
        "--use-mock-keychain".into(),
        // Chrome would copy the app bundle to check its signature against, which an
        // ad-hoc-signed build doesn't need.
        "--disable-features=MacAppCodeSignClone".into(),
    ]);
    // Switches given to the app come after, so they override these: Chromium keeps a repeated
    // switch's last value.
    args.extend(std::env::args().skip(1));
    let args: Vec<CString> = args
        .into_iter()
        .filter_map(|arg| CString::new(arg).ok())
        .collect();
    if std::env::var_os("BROWSER_DRIVE").is_some() {
        unsafe { platform::drive_start() };
    }
    let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    let host = bc::bc_host {
        started: Some(started),
        stopping: Some(stopping),
        window_closed: Some(window_closed),
        space_changed: Some(space_changed),
        spaces_changed: Some(spaces_changed),
        window_space_shown: Some(window_space_shown),
        window_selected: Some(window_selected),
        tab_opened: Some(tab_opened),
        tab_closed: Some(tab_closed),
        tab_loaded: Some(tab_loaded),
        tab_state_changed: Some(tab_state_changed),
        tab_title_changed: Some(tab_title_changed),
        tab_favicon_changed: Some(tab_favicon_changed),
        tab_fullscreen_changed: Some(tab_fullscreen_changed),
        tab_find_changed: Some(tab_find_changed),
        tab_key_before_page: Some(tab_key_before_page),
        tab_key_unhandled: Some(tab_key_unhandled),
        ..Default::default()
    };
    let code = unsafe { bc::bc_main(argv.len() as c_int, argv.as_mut_ptr(), &host) };
    std::process::exit(code);
}
