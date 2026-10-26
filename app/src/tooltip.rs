//! The tooltip: the name Arc shows after a moment over a favorite's tile, which has only an icon,
//! over a folder, and over a sidebar button, with its shortcut's keys on chips after it
//! (docs/ux-spec.md › Tooltips). GPUI can't draw over the page's native view, and a tooltip may
//! cross the sidebar's edge onto it, so as the toast is, it's a window of its own that the host
//! shows over the browser window, as its child, and the mouse goes through it. What it asks of its
//! window goes through `Host`.

use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, Context, FontWeight, Global, Pixels, SharedString, Size, Task, TextRun,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, WindowTextSystem, div, font, point, prelude::*, px, rgb, size,
};

// Ours: Arc's delay and placement under a tile aren't measured, so they're the system's tooltip,
// which shows after half a second, hung under what it names.
/// How long the pointer rests on a tile, row or button before its tooltip shows.
pub const DELAY: Duration = Duration::from_millis(500);
/// How soon after a tooltip went the next shows at once, as the pointer crosses the gap between
/// two tiles.
pub const GRACE: Duration = DELAY;
/// How long it takes to fade in and out. Arc's fades out; how fast isn't recorded (ours).
pub const FADE: Duration = Duration::from_millis(150);
/// How far below the tile or row it names the tooltip's top is, or above it if it doesn't fit
/// below. The gap between two tiles, so it reads as the tile's own and the tile's edge shows.
pub const GAP: f32 = 6.;
/// How far right of a footer button's plate the tooltip beside it starts (Arc).
pub const BESIDE: f32 = 3.5;
// Arc's, from the Library button's: a capsule 5 taller than its chips each way, its 12 pt text 10
// in and its last chip 10 from its end, 161 wide for View Library ⇧⌘L.
const HEIGHT: f32 = 28.;
/// Its window, wide enough for a long name, which truncates past it.
pub const WINDOW_WIDTH: f32 = 320.;
pub const WINDOW_HEIGHT: f32 = HEIGHT;
const TEXT_X: f32 = 10.;
const END: f32 = 10.;
const TEXT_SIZE: f32 = 12.;
/// A key's chip: 18 square, wider for a key named in words, 8 after the text and 4 apart (Arc).
const CHIP: f32 = 18.;
const CHIP_RADIUS: f32 = 4.;
const CHIP_PADDING: f32 = 3.;
const CHIP_AFTER_TEXT: f32 = 8.;
const CHIP_GAP: f32 = 4.;
const CHIP_TEXT_SIZE: f32 = 10.;
// Arc's capture was mid-fade at about 0.23 over the sidebar, as the + under its end shows. Taken
// back to full, its plate is Arc's blue at a third of its brightness, its chips and text the blue
// at a sixth and an eighth over white, and the chips' glyphs the blue itself, #1F2AFF, as the
// Library's + is.
const FILL: u32 = 0x0a0d53;
const TEXT: u32 = 0xe4e7ff;
const CHIP_FILL: u32 = 0xdbdeff;
const CHIP_TEXT: u32 = 0x1f2aff;

/// What the tooltip asks of its window.
pub trait Host {
    /// Shows the tooltip's window with its top left at (`left`, `top`) in the browser window, or
    /// moves it there if it shows. The UI has kept it inside the window.
    fn show(&self, left: f32, top: f32);
    /// Hides it.
    fn hide(&self);
}

/// What a tooltip says: a name, and the keys of the shortcut that does the same, if one does.
#[derive(Clone, Debug, PartialEq)]
pub struct Tip {
    pub text: SharedString,
    pub keys: Vec<SharedString>,
}

impl Tip {
    /// A name alone.
    pub fn named(text: SharedString) -> Self {
        Tip { text, keys: Vec::new() }
    }
}

/// Where a tooltip goes from what it names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    /// Under it, the tooltip's middle at the anchor's x.
    Centre,
    /// Under it, the tooltip's text starting at the anchor's x, so it lines up with the row's
    /// title and its plate hangs outside.
    Text,
    /// Right of it, starting BESIDE past the anchor's x, its right edge, and centred on it
    /// top to bottom, as Arc's is beside the footer's Library button.
    Beside,
}

/// What a tooltip hangs from: the tile, row or button it names, in the browser window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub x: f32,
    pub align: Align,
    /// The tile's, row's or button's top and bottom.
    pub top: f32,
    pub bottom: f32,
    /// The window's size, which the tooltip stays inside.
    pub window: (f32, f32),
}

impl Anchor {
    /// Under a tile or button whose frame is `frame`, centred on it.
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

    /// Right of a button whose plate's frame is `frame`.
    pub fn beside(frame: Bounds<Pixels>, window: Size<Pixels>) -> Self {
        let x = f32::from(frame.right());
        Anchor { x, align: Align::Beside, ..Self::centred(frame, window) }
    }
}

/// Where a tooltip of `size` has its top left: under what it hangs from, or above it with the same
/// gap if there's no room below, or beside it, and inside the window.
pub fn place(anchor: Anchor, (width, height): (f32, f32)) -> (f32, f32) {
    let left = match anchor.align {
        Align::Centre => anchor.x - width / 2.,
        Align::Text => anchor.x - TEXT_X,
        Align::Beside => anchor.x + BESIDE,
    };
    let left = left.min(anchor.window.0 - width).max(0.);
    let below = anchor.bottom + GAP;
    let top = match anchor.align {
        Align::Beside => (anchor.top + anchor.bottom - height) / 2.,
        _ if below + height <= anchor.window.1 => below,
        _ => anchor.top - GAP - height,
    };
    (left, top.min(anchor.window.1 - height).max(0.))
}

/// Where a tooltip's text and chips go, from its left, and how wide it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub width: f32,
    /// How wide the text draws, which is less than it would take if it truncates.
    pub text: f32,
    pub truncated: bool,
    /// Each chip's left and width.
    pub chips: Vec<(f32, f32)>,
}

/// Lays out a tooltip whose text takes `text` across, with chips `chips` wide after it, in its
/// window: a long name truncates, and the keys stay whole.
pub fn layout(text: f32, chips: &[f32]) -> Layout {
    let keys = match chips.len() {
        0 => 0.,
        count => CHIP_AFTER_TEXT + chips.iter().sum::<f32>() + CHIP_GAP * (count - 1) as f32,
    };
    let room = (WINDOW_WIDTH - TEXT_X - keys - END).max(0.);
    let (text, truncated) = (text.min(room), text > room);
    let mut left = TEXT_X + text + CHIP_AFTER_TEXT;
    let chips = (chips.iter())
        .map(|&width| {
            let chip = (left, width);
            left += width + CHIP_GAP;
            chip
        })
        .collect();
    Layout { width: TEXT_X + text + keys + END, text, truncated, chips }
}

/// How wide `text` is in `size` pt of the system's font at `weight`.
fn text_width(
    text: &SharedString,
    size: f32,
    weight: FontWeight,
    system: &WindowTextSystem,
) -> f32 {
    let run = TextRun {
        len: text.len(),
        font: gpui::Font { weight, ..font(".SystemUIFont") },
        color: rgb(TEXT).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(system.shape_line(text.clone(), px(size), &[run], None).width)
}

/// How `tip` lays out: its text, then a chip for each key, as wide as its glyph and its padding
/// but no less than square.
fn measure(tip: &Tip, system: &WindowTextSystem) -> Layout {
    let text = text_width(&tip.text, TEXT_SIZE, FontWeight::NORMAL, system);
    let chips: Vec<f32> = (tip.keys.iter())
        .map(|key| {
            let glyph = text_width(key, CHIP_TEXT_SIZE, FontWeight::SEMIBOLD, system);
            (glyph + 2. * CHIP_PADDING).max(CHIP)
        })
        .collect();
    layout(text, &chips)
}

/// What a tooltip says over something named `name`: nothing if it has no name.
pub fn text_for(name: &str) -> Option<SharedString> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string().into())
}

pub struct Tooltip {
    host: Rc<dyn Host>,
    /// For how wide a name and its keys are.
    system: Arc<WindowTextSystem>,
    /// What shows.
    shown: Option<Tip>,
    /// What the window draws, which stays as it fades out after the tooltip went.
    drawn: Option<(Tip, Layout)>,
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
        cx.new(|_| Tooltip { host, system, shown: None, drawn: None, wait: None, went: None })
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

/// The pointer has come onto the tile, row or button that says `tip`, which the tooltip hangs from
/// at `anchor`, wherever the pointer is on it. The tooltip shows after DELAY, one timer that
/// `dismiss` cancels, or at once if another showed less than GRACE ago, as the system's does when
/// the pointer moves from one tile to the next.
pub fn hover(tip: Tip, anchor: Anchor, cx: &mut App) {
    let Some(window) = cx.try_global::<TooltipWindow>().map(|tooltip| tooltip.0) else {
        return;
    };
    window
        .update(cx, |tooltip, _, cx| {
            let now = cx.background_executor().now();
            let soon = tooltip.went.is_some_and(|went| now.duration_since(went) < GRACE);
            if tooltip.shown.is_some() || soon {
                tooltip.wait = None;
                tooltip.show(tip, anchor, cx);
                return;
            }
            tooltip.wait = Some(cx.spawn(async move |tooltip, cx| {
                cx.background_executor().timer(DELAY).await;
                tooltip
                    .update(cx, |tooltip, cx| {
                        tooltip.wait = None;
                        tooltip.show(tip, anchor, cx)
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
    fn show(&mut self, tip: Tip, anchor: Anchor, cx: &mut Context<Self>) {
        let layout = measure(&tip, &self.system);
        let (left, top) = place(anchor, (layout.width, HEIGHT));
        if self.shown.as_ref() != Some(&tip) {
            // check.sh reads this line.
            log!("browser: tooltip shown {}", label(&tip));
        }
        self.shown = Some(tip.clone());
        self.drawn = Some((tip, layout));
        self.host.show(left, top);
        cx.notify();
    }

    /// The window fades out with what it drew, so this draws nothing new.
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.wait = None;
        if self.shown.take().is_some() {
            self.went = Some(cx.background_executor().now());
            self.host.hide();
            // check.sh reads this line.
            log!("browser: tooltip hidden");
        }
    }
}

/// How the log names `tip`: its text, then its keys as a menu shows them.
fn label(tip: &Tip) -> String {
    match tip.keys.is_empty() {
        true => tip.text.to_string(),
        false => format!("{} {}", tip.text, tip.keys.concat()),
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().font_family(".SystemUIFont");
        let Some((tip, layout)) = self.drawn.clone() else {
            return root;
        };
        let text = div()
            .absolute()
            .left(px(TEXT_X))
            .h_full()
            .flex()
            .items_center()
            .text_size(px(TEXT_SIZE))
            .text_color(rgb(TEXT))
            // Untruncated, it's as wide as it shaped, and a box that wide could round under it.
            .when(layout.truncated, |text| text.w(px(layout.text)).truncate())
            .child(tip.text);
        let chips = tip.keys.into_iter().zip(layout.chips).map(|(key, (left, width))| {
            div()
                .absolute()
                .left(px(left))
                .top(px((HEIGHT - CHIP) / 2.))
                .w(px(width))
                .h(px(CHIP))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(CHIP_RADIUS))
                .bg(rgb(CHIP_FILL))
                .text_size(px(CHIP_TEXT_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(CHIP_TEXT))
                .child(key)
        });
        root.child(
            div()
                .relative()
                .w(px(layout.width))
                .h(px(HEIGHT))
                .rounded(px(HEIGHT / 2.))
                .bg(rgb(FILL))
                .child(text)
                .children(chips),
        )
    }
}

#[cfg(test)]
mod tests;
