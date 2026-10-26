//! The tooltip: the name Arc shows after a moment over a favorite's tile, which has only an icon,
//! and over a folder (docs/ux-spec.md › Tooltips). GPUI can't draw over the page's native view,
//! and a tooltip may cross the sidebar's edge onto it, so as the toast is, it's a window of its
//! own that the host shows over the browser window, as its child, and the mouse goes through it.
//! What it asks of its window goes through `Host`.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, Context, Global, SharedString, Task, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, point,
    prelude::*, px, rgba, size,
};

// Ours: Arc's delay, fade and placement aren't measured, so they're the system's tooltip, which
// shows after half a second with no motion, its corner a little below the pointer.
/// How long the pointer rests on a tile or folder before its tooltip shows.
pub const DELAY: Duration = Duration::from_millis(500);
/// How soon after a tooltip went the next shows at once, as the pointer crosses the gap between
/// two tiles.
pub const GRACE: Duration = DELAY;
/// How far below the pointer the tooltip's top is.
pub const BELOW: f32 = 20.;
const HEIGHT: f32 = 22.;
/// Its window, wide enough for a long name, which truncates past it.
pub const WINDOW_WIDTH: f32 = 320.;
pub const WINDOW_HEIGHT: f32 = HEIGHT;
const RADIUS: f32 = 5.;
const PADDING: f32 = 7.;
const TEXT_SIZE: f32 = 12.;
const FILL: u32 = 0x2b2b2bf5;
const EDGE: u32 = 0xffffff26;
const TEXT: u32 = 0xffffffe6;

/// What the tooltip asks of its window.
pub trait Host {
    /// Shows the tooltip's window with its top left at (`left`, `top`) in the browser window, or
    /// moves it there if it shows.
    fn show(&self, left: f32, top: f32);
    /// Hides it.
    fn hide(&self);
}

/// What a tooltip says over something named `name`: nothing if it has no name.
pub fn text_for(name: &str) -> Option<SharedString> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string().into())
}

pub struct Tooltip {
    host: Rc<dyn Host>,
    /// The words showing.
    shown: Option<SharedString>,
    /// The one-shot timer a tooltip waits on. Dropping it cancels it.
    wait: Option<Task<()>>,
    /// When a tooltip last went.
    went: Option<Instant>,
}

struct TooltipWindow(WindowHandle<Tooltip>);

impl Global for TooltipWindow {}

/// Opens the tooltip's window, hidden, for `host` to show over the browser window, ahead of time
/// as the toast's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<Tooltip>> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
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
        window_background: WindowBackgroundAppearance::Transparent,
        // GPUI holds a window that isn't the key one to 30 fps, and this one never is.
        inactive_frame_interval: None,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|_| Tooltip { host, shown: None, wait: None, went: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(TooltipWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the tooltip's window: {error}");
            None
        }
    }
}

/// The pointer has come onto something that says `text`, with the pointer at `at` in the browser
/// window. The tooltip shows after DELAY, one timer that `dismiss` cancels, or at once if another
/// showed less than GRACE ago, as the system's does when the pointer moves from one tile to the
/// next.
pub fn hover(text: SharedString, at: (f32, f32), cx: &mut App) {
    let Some(window) = cx.try_global::<TooltipWindow>().map(|tooltip| tooltip.0) else {
        return;
    };
    window
        .update(cx, |tooltip, _, cx| {
            let place = (at.0, at.1 + BELOW);
            let now = cx.background_executor().now();
            let soon = tooltip.went.is_some_and(|went| now.duration_since(went) < GRACE);
            if tooltip.shown.is_some() || soon {
                tooltip.wait = None;
                tooltip.show(text, place, cx);
                return;
            }
            tooltip.wait = Some(cx.spawn(async move |tooltip, cx| {
                cx.background_executor().timer(DELAY).await;
                tooltip
                    .update(cx, |tooltip, cx| {
                        tooltip.wait = None;
                        tooltip.show(text, place, cx)
                    })
                    .ok();
            }));
        })
        .ok();
}

/// The pointer has left, or a click, a scroll, a Space switch or the window losing the keyboard
/// ends the tooltip: its timer is dropped, and its window goes if it shows.
pub fn dismiss(cx: &mut App) {
    let Some(window) = cx.try_global::<TooltipWindow>().map(|tooltip| tooltip.0) else {
        return;
    };
    window.update(cx, |tooltip, _, cx| tooltip.dismiss(cx)).ok();
}

impl Tooltip {
    fn show(&mut self, text: SharedString, (left, top): (f32, f32), cx: &mut Context<Self>) {
        if self.shown.as_ref() != Some(&text) {
            // check.sh reads this line.
            log!("browser: tooltip shown {text}");
        }
        self.shown = Some(text);
        self.host.show(left, top);
        cx.notify();
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.wait = None;
        if self.shown.take().is_some() {
            self.went = Some(cx.background_executor().now());
            self.host.hide();
            // check.sh reads this line.
            log!("browser: tooltip hidden");
            cx.notify();
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().flex().items_start().font_family(".SystemUIFont");
        let Some(text) = self.shown.clone() else {
            return root;
        };
        root.child(
            div()
                .h(px(HEIGHT))
                .max_w(px(WINDOW_WIDTH))
                .px(px(PADDING))
                .flex()
                .items_center()
                .rounded(px(RADIUS))
                .border(px(0.5))
                .border_color(rgba(EDGE))
                .bg(rgba(FILL))
                .text_size(px(TEXT_SIZE))
                .text_color(rgba(TEXT))
                .child(div().truncate().child(text)),
        )
    }
}

#[cfg(test)]
mod tests;
