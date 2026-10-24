//! Phase 1 spike: a GPUI window with a sidebar, and a Chromium page view inside it, in one
//! process whose application object and run loop belong to Chromium.

use std::{
    cell::RefCell,
    ffi::{CString, c_char, c_int, c_void},
};

use gpui::{
    App, AsyncApp, Bounds, Context, KeyBinding, Menu, MenuItem, SharedString,
    TitlebarOptions, WeakEntity, Window, WindowBounds, WindowOptions, actions, div, point,
    prelude::*, px, rgb, rgba, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod command_bar;
mod text_input;

mod host {
    use std::ffi::{c_char, c_int, c_void};

    unsafe extern "C" {
        pub fn spike_host_init(
            argc: c_int,
            argv: *mut *mut c_char,
            on_command: extern "C" fn(c_int),
        ) -> bool;
        pub fn spike_host_create_browser(
            parent_view: *mut c_void,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            url: *const c_char,
        );
        pub fn spike_host_open_plain_window(url: *const c_char);
        pub fn spike_host_load_url(url: *const c_char);
        pub fn spike_host_add_overlay(parent_view: *mut c_void, overlay_view: *mut c_void);
        pub fn spike_host_show_overlay();
        pub fn spike_host_hide_overlay();
        pub fn spike_host_run();
        pub fn spike_host_quit();
    }
}

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
    // Chromium calls back into the UI from its own event handling, outside any GPUI update.
    static SHELL: RefCell<Option<(AsyncApp, WeakEntity<Shell>)>> = const { RefCell::new(None) };
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

/// Runs a browser command, logs it and shows it in the sidebar. `from` says which side had the
/// key: GPUI's key bindings ("ui") or Chromium handing it back ("page").
fn run_command(name: &str, from: &str, cx: &mut App) {
    // check.sh waits for these lines.
    eprintln!("spike: command {name} via {from}");
    let shell = SHELL.with_borrow(|shell| shell.as_ref().map(|(_, shell)| shell.clone()));
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
        "quit" => unsafe { host::spike_host_quit() },
        // The spike has one tab and no find bar, so these only show up in the sidebar.
        _ => {}
    }
}

extern "C" fn on_command(command: c_int) {
    let name = match command {
        1 => "new-tab",
        2 => "edit-url",
        3 => "close-tab",
        4 => "find",
        5 => "quit",
        _ => return,
    };
    let cx = SHELL.with_borrow(|shell| shell.as_ref().map(|(cx, _)| cx.clone()));
    if let Some(cx) = cx {
        cx.update(|cx| run_command(name, "page", cx));
    }
}

fn open_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1280.), px(800.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.), px(14.))),
        }),
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        let shell = cx.new(|_| Shell {
            last_command: "Press ⌘T, ⌘L, ⌘W or ⌘F".into(),
        });
        SHELL.set(Some((cx.to_async(), shell.downgrade())));

        let viewport = window.viewport_size();
        let parent_view = ns_view(window);
        let url = start_url();
        // The parent view isn't flipped, so y is measured from the bottom. The margins are equal,
        // so the page view keeps them as the window resizes.
        unsafe {
            host::spike_host_create_browser(
                parent_view,
                SIDEBAR_WIDTH as f64,
                GAP as f64,
                (f32::from(viewport.width) - SIDEBAR_WIDTH - GAP) as f64,
                (f32::from(viewport.height) - 2. * GAP) as f64,
                url.as_ptr(),
            );
        }

        window.on_window_should_close(cx, |_, _| {
            // Chromium has to close the page before the window goes away.
            unsafe { host::spike_host_quit() };
            false
        });
        shell
    });
    match window.and_then(|window| window.update(cx, |_, window, _| ns_view(window))) {
        Ok(view) => command_bar::open(view, cx),
        Err(error) => {
            eprintln!("couldn't open the window: {error}");
            unsafe { host::spike_host_quit() };
        }
    }
}

fn main() {
    let args: Vec<CString> = std::env::args()
        .filter_map(|arg| CString::new(arg).ok())
        .collect();
    let mut argv: Vec<*mut c_char> = args.iter().map(|arg| arg.as_ptr().cast_mut()).collect();
    // Chromium creates the application object, so this has to come before GPUI starts.
    if !unsafe { host::spike_host_init(argv.len() as c_int, argv.as_mut_ptr(), on_command) } {
        eprintln!("Chromium didn't start");
        std::process::exit(1);
    }
    // The page alone, as the baseline for what GPUI costs it.
    if std::env::var_os("SPIKE_PLAIN").is_some() {
        let url = start_url();
        unsafe {
            host::spike_host_open_plain_window(url.as_ptr());
            host::spike_host_run();
        }
        return;
    }

    let _app = gpui_platform::application().run_embedded(|cx| {
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
        cx.activate(true);
    });

    unsafe { host::spike_host_run() };
}
