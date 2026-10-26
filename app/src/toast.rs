//! The toast: Arc's note at the top right of the window after something it does without showing
//! it, such as "Space deleted! Use ⌘Z to undo." (docs/ux-spec.md › Toast). GPUI can't draw over
//! the page's native view, so as the zoom HUD is, it's a window of its own that the host shows
//! over the browser window's top right corner, as its child, and the mouse goes through it to the
//! page. What it asks of its window goes through `Host`.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, BoxShadow, Context, FontWeight, Global, Hsla, SharedString, Task,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div, hsla, linear_color_stop, linear_gradient, point, prelude::*, px, rgb,
    rgba, size,
};

use crate::shell::{cubic_bezier, scaled::scaled_from_top, spring};

// Arc's, measured at 2× (docs/ux-spec.md › Toast): 46.5 tall, 4.5 in from the window's right edge
// and 4 down from its top, its text 12.5 in from its left and 13.5 from its right, and a key's
// chip 8 from the words each side of it.
const HEIGHT: f32 = 46.5;
const RIGHT: f32 = 4.5;
const TOP: f32 = 4.;
const TEXT_LEFT: f32 = 12.5;
const TEXT_RIGHT: f32 = 13.5;
const RADIUS: f32 = 10.;
/// Medium, and the key semibold. 13.5 makes Arc's toast 256.5 wide, as measured, where 14 makes it
/// 263.5.
const TEXT_SIZE: f32 = 13.5;
const CHIP_HEIGHT: f32 = 26.;
const CHIP_RADIUS: f32 = 6.;
const CHIP_X: f32 = 9.;
const CHIP_GAP: f32 = 8.;
const CHIP_FILL: u32 = 0xffff_ff1a;
/// A line of black 0.3 around it, half a point thick.
const EDGE: u32 = 0x0000_004d;
/// It darkens from the window's colour at its top to this much of it at its bottom (Arc's goes
/// from (40,22,80) to (31,17,62) in a purple Space).
const SHADE: f32 = 0.78;
/// Its window, wide enough for the longest toast and tall enough for its shadow, at the browser
/// window's top right.
pub const WINDOW_WIDTH: f32 = 420.;
pub const WINDOW_HEIGHT: f32 = 72.;
/// Arc's motion: it grows from 0.7 of its size about the middle of its top edge on a spring of
/// 0.32 s and ζ 0.52, fading in over the first 0.1 s, and leaves the way it came over 0.07 s,
/// speeding up.
const FROM: f32 = 0.7;
const RESPONSE: f32 = 0.32;
const DAMPING: f32 = 0.52;
const FADE_IN: f32 = 0.1;
const LEAVE: f32 = 0.07;

/// What the toast asks of its window.
pub trait Host {
    /// Shows the toast's window at the browser window's top right.
    fn show(&self);
    /// Hides it.
    fn hide(&self);
}

/// How big the toast is drawn, from 0.7 to its size (1) and a little past it, and how opaque.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    scale: f32,
    opacity: f32,
}

impl Pose {
    /// `seconds` after its window was first on screen.
    fn arriving(seconds: f32) -> Self {
        Pose {
            scale: FROM + (1. - FROM) * spring(seconds, RESPONSE, DAMPING).unwrap_or(1.),
            opacity: (seconds / FADE_IN).min(1.),
        }
    }

    /// `seconds` after it started leaving, from where the motion was then.
    fn leaving(self, seconds: f32) -> Self {
        let t = cubic_bezier(0.42, 0., 1., 1., (seconds / LEAVE).min(1.));
        Pose {
            scale: self.scale + (FROM - self.scale) * t,
            opacity: self.opacity * (1. - t),
        }
    }
}

struct Shown {
    text: SharedString,
    /// A key's chip after the text, and the words after it.
    key: Option<(SharedString, SharedString)>,
    /// When its window was first on screen, where the motion starts.
    shown: Option<Instant>,
    /// Once it's leaving: when, and where the motion was.
    leaving: Option<(Instant, Pose)>,
    settled: bool,
    _timer: Task<()>,
}

impl Shown {
    fn pose(&self) -> Pose {
        let arrived = Pose::arriving(self.shown.map_or(0., |shown| shown.elapsed().as_secs_f32()));
        match self.leaving {
            Some((at, from)) => from.leaving(at.elapsed().as_secs_f32()),
            None => arrived,
        }
    }

    fn moving(&self) -> bool {
        match (self.shown, self.leaving) {
            (_, Some(_)) | (None, _) => true,
            (Some(shown), None) => {
                spring(shown.elapsed().as_secs_f32(), RESPONSE, DAMPING).is_some()
            }
        }
    }
}

pub struct Toast {
    host: Rc<dyn Host>,
    /// The window's colour, which the toast takes.
    fill: Box<dyn Fn(&App) -> Hsla>,
    shown: Option<Shown>,
}

struct ToastWindow(WindowHandle<Toast>);

impl Global for ToastWindow {}

/// Opens the toast's window, hidden, for `host` to show over the browser window, ahead of time as
/// the find bar's is. The toast takes `fill`'s colour.
pub fn open(
    host: Rc<dyn Host>,
    fill: impl Fn(&App) -> Hsla + 'static,
    cx: &mut App,
) -> Option<WindowHandle<Toast>> {
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
        // The toast and its shadow are drawn.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|_| Toast { host, fill: Box::new(fill), shown: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(ToastWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the toast's window: {error}");
            None
        }
    }
}

/// Shows `text`, then `key`'s chip and the words after it if there's a key, for `time` from now,
/// in place of any toast showing.
pub fn show(text: &str, key: Option<(&str, &str)>, time: Duration, cx: &mut App) {
    let Some(window) = cx.try_global::<ToastWindow>().map(|toast| toast.0) else {
        return;
    };
    window
        .update(cx, |toast, _, cx| {
            // check.sh reads this line.
            let words = key.map_or(String::new(), |(key, rest)| format!(" {key} {rest}"));
            log!("browser: toast shown {text}{words}");
            let timer = cx.spawn(async move |toast, cx| {
                cx.background_executor().timer(time).await;
                toast.update(cx, |toast, cx| toast.leave(cx)).ok();
            });
            // One already in place stays there with the new words, and one leaving comes back.
            let shown = toast.shown.take().filter(|shown| shown.leaving.is_none());
            toast.shown = Some(Shown {
                text: text.to_string().into(),
                key: key.map(|(key, rest)| (key.to_string().into(), rest.to_string().into())),
                shown: shown.and_then(|shown| shown.shown),
                leaving: None,
                settled: false,
                _timer: timer,
            });
            toast.host.show();
            cx.notify();
        })
        .ok();
}

impl Toast {
    fn leave(&mut self, cx: &mut Context<Self>) {
        let Some(shown) = &mut self.shown else {
            return;
        };
        shown.leaving = Some((Instant::now(), shown.pose()));
        shown._timer = cx.spawn(async move |toast, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(LEAVE)).await;
            toast
                .update(cx, |toast, cx| {
                    toast.shown = None;
                    toast.host.hide();
                    // check.sh reads this line.
                    log!("browser: toast hidden");
                    cx.notify();
                })
                .ok();
        });
        cx.notify();
    }
}

fn shadow(color: Hsla, y: f32, blur: f32) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset: false,
    }
}

impl Render for Toast {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .size_full()
            .flex()
            .justify_end()
            .items_start()
            .pt(px(TOP))
            .pr(px(RIGHT))
            .font_family(".SystemUIFont");
        let Some(shown) = &mut self.shown else {
            return root;
        };
        // AppKit puts the window on screen some frames after it's asked to, and only then does
        // GPUI draw it, so the motion starts there rather than skip its start.
        if shown.shown.is_none() && window.is_visible() {
            shown.shown = Some(Instant::now());
        }
        if shown.moving() {
            window.request_animation_frame();
        } else if !shown.settled {
            shown.settled = true;
            // check.sh reads this line.
            log!("browser: toast settled");
        }
        let pose = shown.pose();
        let fill = (self.fill)(cx);
        let bottom = Hsla { l: fill.l * SHADE, ..fill };
        let toast = div()
            .h(px(HEIGHT))
            .pl(px(TEXT_LEFT))
            .pr(px(TEXT_RIGHT))
            .flex()
            .items_center()
            .gap(px(CHIP_GAP))
            .rounded(px(RADIUS))
            .border(px(0.5))
            .border_color(rgba(EDGE))
            .bg(linear_gradient(180., linear_color_stop(fill, 0.), linear_color_stop(bottom, 1.)))
            .shadow(vec![shadow(hsla(0., 0., 0., 0.25), 4., 12.)])
            .opacity(pose.opacity)
            .text_size(px(TEXT_SIZE))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xffffff))
            .child(shown.text.clone())
            .children(shown.key.clone().map(|(key, rest)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(CHIP_GAP))
                    .child(
                        div()
                            .h(px(CHIP_HEIGHT))
                            .px(px(CHIP_X))
                            .flex()
                            .items_center()
                            .rounded(px(CHIP_RADIUS))
                            .bg(rgba(CHIP_FILL))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(key),
                    )
                    .child(rest)
            }));
        root.child(scaled_from_top(toast, pose.scale))
    }
}

#[cfg(test)]
mod tests;
