//! The tooltip: the name Arc shows after a moment over a favorite's tile, which has only an icon,
//! and over a folder (docs/ux-spec.md › Tooltips). GPUI can't draw over the page's native view,
//! and a tooltip may cross the sidebar's edge onto it, so as the toast is, it's a window of its
//! own that the host shows over the browser window, as its child, and the mouse goes through it.
//! What it asks of its window goes through `Host`.

use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, Context, Global, Pixels, SharedString, Size, Task, TextRun, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    WindowTextSystem, div, font, point, prelude::*, px, rgba, size,
};

// Ours: Arc's delay, fade and placement aren't measured, so they're the system's tooltip, which
// shows after half a second with no motion, hung under what it names.
/// How long the pointer rests on a tile or folder before its tooltip shows.
pub const DELAY: Duration = Duration::from_millis(500);
/// How soon after a tooltip went the next shows at once, as the pointer crosses the gap between
/// two tiles.
pub const GRACE: Duration = DELAY;
/// How far below the tile or row it names the tooltip's top is, or above it if it doesn't fit
/// below. The gap between two tiles, so it reads as the tile's own and the tile's edge shows.
pub const GAP: f32 = 6.;
const HEIGHT: f32 = 22.;
/// Its window, wide enough for a long name, which truncates past it.
pub const WINDOW_WIDTH: f32 = 320.;
pub const WINDOW_HEIGHT: f32 = HEIGHT;
const RADIUS: f32 = 5.;
const PADDING: f32 = 7.;
const BORDER: f32 = 0.5;
const TEXT_SIZE: f32 = 12.;
const FILL: u32 = 0x2b2b2bf5;
const EDGE: u32 = 0xffffff26;
const TEXT: u32 = 0xffffffe6;

/// What the tooltip asks of its window.
pub trait Host {
    /// Shows the tooltip's window with its top left at (`left`, `top`) in the browser window, or
    /// moves it there if it shows. The UI has kept it inside the window.
    fn show(&self, left: f32, top: f32);
    /// Hides it.
    fn hide(&self);
}

/// Where a tooltip's text lines up with what it names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    /// The tooltip's middle is at the anchor's x.
    Centre,
    /// The tooltip's text starts at the anchor's x, so it lines up with the row's title and its
    /// plate hangs outside.
    Text,
}

/// What a tooltip hangs from: the tile or row it names, in the browser window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub x: f32,
    pub align: Align,
    /// The tile's or row's top and bottom.
    pub top: f32,
    pub bottom: f32,
    /// The window's size, which the tooltip stays inside.
    pub window: (f32, f32),
}

impl Anchor {
    /// Under a tile whose frame is `frame`, centred on it.
    pub fn centred(frame: Bounds<Pixels>, window: Size<Pixels>) -> Self {
        Anchor {
            x: f32::from(frame.origin.x + frame.size.width / 2.),
            align: Align::Centre,
            top: f32::from(frame.origin.y),
            bottom: f32::from(frame.bottom()),
            window: (window.width.into(), window.height.into()),
        }
    }

    /// Under a row whose frame is `frame`, its text above the row's title, `title` in from the
    /// row's left.
    pub fn titled(frame: Bounds<Pixels>, title: f32, window: Size<Pixels>) -> Self {
        let x = f32::from(frame.origin.x) + title;
        Anchor { x, align: Align::Text, ..Self::centred(frame, window) }
    }
}

/// Where a tooltip of `size` has its top left: under what it hangs from, kept inside the window
/// across, and above it, with the same gap, if there's no room below.
pub fn place(anchor: Anchor, (width, height): (f32, f32)) -> (f32, f32) {
    let left = match anchor.align {
        Align::Centre => anchor.x - width / 2.,
        Align::Text => anchor.x - PADDING - BORDER,
    };
    let left = left.min(anchor.window.0 - width).max(0.);
    let below = anchor.bottom + GAP;
    let top = match below + height <= anchor.window.1 {
        true => below,
        false => (anchor.top - GAP - height).max(0.),
    };
    (left, top)
}

/// How wide a tooltip saying `text` is: the text and its padding, up to its window.
fn width_of(text: &SharedString, system: &WindowTextSystem) -> f32 {
    let run = TextRun {
        len: text.len(),
        font: font(".SystemUIFont"),
        color: rgba(TEXT).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = system.shape_line(text.clone(), px(TEXT_SIZE), &[run], None);
    (f32::from(line.width) + 2. * (PADDING + BORDER)).min(WINDOW_WIDTH)
}

/// What a tooltip says over something named `name`: nothing if it has no name.
pub fn text_for(name: &str) -> Option<SharedString> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string().into())
}

pub struct Tooltip {
    host: Rc<dyn Host>,
    /// For how wide a name is.
    system: Arc<WindowTextSystem>,
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
    let window = cx.open_window(options, |window, cx| {
        let system = window.text_system().clone();
        cx.new(|_| Tooltip { host, system, shown: None, wait: None, went: None })
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

/// The pointer has come onto the tile or row that says `text`, which the tooltip hangs from at
/// `anchor`, wherever the pointer is on it. The tooltip shows after DELAY, one timer that
/// `dismiss` cancels, or at once if another showed less than GRACE ago, as the system's does when
/// the pointer moves from one tile to the next.
pub fn hover(text: SharedString, anchor: Anchor, cx: &mut App) {
    let Some(window) = cx.try_global::<TooltipWindow>().map(|tooltip| tooltip.0) else {
        return;
    };
    window
        .update(cx, |tooltip, _, cx| {
            let now = cx.background_executor().now();
            let soon = tooltip.went.is_some_and(|went| now.duration_since(went) < GRACE);
            if tooltip.shown.is_some() || soon {
                tooltip.wait = None;
                tooltip.show(text, anchor, cx);
                return;
            }
            tooltip.wait = Some(cx.spawn(async move |tooltip, cx| {
                cx.background_executor().timer(DELAY).await;
                tooltip
                    .update(cx, |tooltip, cx| {
                        tooltip.wait = None;
                        tooltip.show(text, anchor, cx)
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
    fn show(&mut self, text: SharedString, anchor: Anchor, cx: &mut Context<Self>) {
        let (left, top) = place(anchor, (width_of(&text, &self.system), HEIGHT));
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
                .border(px(BORDER))
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
