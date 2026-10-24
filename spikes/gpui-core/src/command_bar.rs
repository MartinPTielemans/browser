//! The command bar. GPUI can't draw over the page's native view, so the bar is a window of its
//! own that the host shows over the browser window, as its child.

use std::ffi::CString;

use gpui::{
    App, Bounds, Context, Entity, Focusable, Global, KeyBinding, TitlebarOptions, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, div, point, prelude::*, px,
    rgb, size,
};

use crate::{host, load_url, ns_view, text_input::TextInput};

actions!(command_bar, [Go, Dismiss]);

const WIDTH: f32 = 640.;
const HEIGHT: f32 = 56.;

pub struct CommandBar {
    input: Entity<TextInput>,
}

struct CommandBarWindow(WindowHandle<CommandBar>);

impl Global for CommandBarWindow {}

/// Opens the command bar's window, hidden, over the browser window. Opening it ahead of time
/// keeps window setup off the path from the shortcut to the bar on screen.
pub fn open(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Go, Some("CommandBar")),
        KeyBinding::new("escape", Dismiss, Some("CommandBar")),
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
        cx.new(|_| CommandBar { input })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
                unsafe { host::spike_host_add_overlay(view) };
                cx.set_global(CommandBarWindow(window));
            }
        }
        Err(error) => eprintln!("couldn't open the command bar: {error}"),
    }
}

/// Shows the command bar, empty and with the keyboard.
pub fn show(cx: &mut App) {
    let Some(window) = cx.try_global::<CommandBarWindow>().map(|bar| bar.0) else {
        return;
    };
    window
        .update(cx, |bar, window, cx| {
            bar.input.update(cx, |input, cx| input.reset(window, cx))
        })
        .ok();
    unsafe { host::spike_host_show_overlay() };
}

impl CommandBar {
    fn go(&mut self, _: &Go, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim();
        if text.is_empty() {
            return;
        }
        let url = if text.contains("://") {
            text.to_string()
        } else {
            format!("https://{text}")
        };
        // check.sh waits for this line.
        eprintln!("spike: command bar go {url}");
        if let Ok(url) = CString::new(url) {
            load_url(&url);
        }
        unsafe { host::spike_host_hide_overlay() };
    }

    fn dismiss(&mut self, _: &Dismiss, _: &mut Window, _: &mut Context<Self>) {
        unsafe { host::spike_host_hide_overlay() };
    }
}

impl Render for CommandBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // check.sh reads these lines.
        eprintln!("spike: command bar drew \"{}\"", self.input.read(cx).text());
        div()
            .key_context("CommandBar")
            .on_action(cx.listener(Self::go))
            .on_action(cx.listener(Self::dismiss))
            .size_full()
            .flex()
            .items_center()
            .px_4()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x1d1d1f))
            .text_size(px(18.))
            .line_height(px(24.))
            .child(self.input.clone())
    }
}
