//! The link preview: a link clicked in a pinned tab or a favorite, to another site than the tab's,
//! opens over the window in a card rather than taking the tab away from its own site (Arc's Peek,
//! docs/ux-spec.md › Phase 4 features). The page is a real one, in a tab of no Space, in its
//! native view, which the host puts over this window's card. ⌘↩ or the card's button keeps it as a
//! Today tab, as it is. GPUI can't draw over the page's native view, so the dim and the card are a
//! window of their own that the host shows over the browser window, as its child, as the
//! dialog's are. What it asks of its window and the core goes through `Host`.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, BoxShadow, Bounds, Context, FocusHandle, Global, Hsla, KeyBinding, MouseButton, Pixels,
    Point, SharedString, Task, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, actions, div, hsla, point, prelude::*, px, rgb, rgba,
    size, svg,
};

use crate::shell::{
    NAV, NAV_HOVERED, NAV_RADIUS, PRESSED, TITLE, URL_BUTTON_GLYPH, URL_BUTTON_INSET,
    URL_BUTTON_SIZE, cubic_bezier, scaled::scaled,
};

actions!(link_preview, [Close, Keep]);

// Ours, as nothing of Arc's preview is recorded yet (docs/ux-spec.md › Measurement plan, 8).
/// The header over the page, a button's height, as the side panel's.
const HEADER: f32 = URL_BUTTON_SIZE + 2. * URL_BUTTON_INSET;
const HEADER_TEXT: f32 = 13.;
const HEADER_LEFT: f32 = 12.;
/// The card's frame around the page at its sides and bottom.
const FRAME: f32 = 6.;
/// How far in from the window's page the card sits, a share of the page's smaller side, within
/// these.
const INSET_SHARE: f32 = 0.05;
const INSET_MIN: f32 = 16.;
const INSET_MAX: f32 = 48.;
/// The page's corners, at least, where the window's page has none, as when it's full screen.
const MIN_RADIUS: f32 = 6.;
const FILL: u32 = 0x1e1e1e;
const EDGE: u32 = 0xffff_ff1a;
/// The dialog's dim, Arc's measured one: what's under the card waits.
const DIM: u32 = 0x151515;
const DIM_OPACITY: f32 = 0.62;
/// The dialog's measured motion, from the link: it grows in over 0.275 s from half its size,
/// opaque after the first 0.135 s, as the window dims. It shrinks back into the link in 0.2 s,
/// as fast as an overlay grows in, and grows into the tab's place in 0.275 s.
const OPEN: f32 = 0.275;
const FADE_IN: f32 = 0.135;
const FROM_SCALE: f32 = 0.5;
const CLOSE: f32 = 0.2;
const KEEP: f32 = 0.275;

/// What the preview asks of its window and the core.
pub trait Host {
    /// Puts `tab`'s page in the preview's window at `page`, from its top left, with corners of
    /// `radius`, with the keyboard.
    fn place(&self, tab: u64, page: Bounds<Pixels>, radius: f32);
    /// Shows the preview's window over all of the browser window `window`, as it draws now, with
    /// the keyboard.
    fn show(&self, window: u64);
    /// It drew a frame of its motion, with the page at `page` and `opacity`, which the page's view
    /// follows.
    fn drew(&self, page: Bounds<Pixels>, opacity: f32);
    /// It stops taking the keyboard and clicks, which the browser window gets back, as it leaves.
    fn release(&self);
    /// Hides its window, once it has left.
    fn hide(&self);
    /// Closes the preview's page, whose tab is `tab`.
    fn close(&self, tab: u64);
    /// Makes `tab` a Today tab of the Space `window` shows, and shows it there, as it is.
    fn keep(&self, window: u64, tab: u64);
}

/// What a key in the preview does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyAction {
    Close,
    Keep,
}

/// What `keystroke` does in the preview, as the page hands it back: before the page sees it if
/// it's `reserved`, else once the page hasn't kept it. Nothing else of the window's keys acts on
/// it, as they would on the window's own tab.
pub fn key_action(keystroke: &str, reserved: bool) -> Option<KeyAction> {
    match (keystroke, reserved) {
        ("cmd-w", true) => Some(KeyAction::Close),
        ("escape", false) => Some(KeyAction::Close),
        ("cmd-enter", false) => Some(KeyAction::Keep),
        _ => None,
    }
}

/// The site a URL is on, as the header shows it: its host, without "www.".
pub fn site_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    host.strip_prefix("www.").unwrap_or(host)
}

/// Where the card goes over a window's page at `page`.
fn card(page: Bounds<Pixels>) -> Bounds<Pixels> {
    let side = f32::from(page.size.width.min(page.size.height));
    let inset = (side * INSET_SHARE).clamp(INSET_MIN, INSET_MAX);
    Bounds::new(
        page.origin + point(px(inset), px(inset)),
        size(page.size.width - px(2. * inset), page.size.height - px(2. * inset)),
    )
}

/// Where the page goes in `card`: under the header, inside the frame.
fn page_in(card: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        card.origin + point(px(FRAME), px(HEADER)),
        size(card.size.width - px(2. * FRAME), card.size.height - px(HEADER + FRAME)),
    )
}

/// The card with its page at `page`.
fn card_around(page: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        page.origin - point(px(FRAME), px(HEADER)),
        size(page.size.width + px(2. * FRAME), page.size.height + px(HEADER + FRAME)),
    )
}

fn scaled_about(frame: Bounds<Pixels>, scale: f32, about: Point<Pixels>) -> Bounds<Pixels> {
    let at = |edge: Pixels, about: Pixels| about + (edge - about) * scale;
    Bounds::new(
        point(at(frame.origin.x, about.x), at(frame.origin.y, about.y)),
        size(frame.size.width * scale, frame.size.height * scale),
    )
}

fn between(from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    let mix = |a: Pixels, b: Pixels| a + (b - a) * t;
    Bounds::new(
        point(mix(from.origin.x, to.origin.x), mix(from.origin.y, to.origin.y)),
        size(mix(from.size.width, to.size.width), mix(from.size.height, to.size.height)),
    )
}

/// Where the motion is.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Pose {
    /// The card's size about the link, 1 at rest.
    scale: f32,
    /// How far the page has grown into the tab's place, from 0 to 1.
    kept: f32,
    /// The page's opacity, and the card's around it.
    page: f32,
    card: f32,
    /// How far the window under it has dimmed, from 0 to 1.
    dim: f32,
}

impl Pose {
    /// `seconds` after the window was first on screen.
    fn arriving(seconds: f32) -> Self {
        let t = (seconds / OPEN).min(1.);
        let opacity = (seconds / FADE_IN).min(1.);
        Pose {
            scale: FROM_SCALE + (1. - FROM_SCALE) * cubic_bezier(0., 0., 0.58, 1., t),
            kept: 0.,
            page: opacity,
            card: opacity,
            dim: t,
        }
    }

    /// `seconds` after it started leaving back into the link, from `self`.
    fn closing(self, seconds: f32) -> Self {
        let t = (seconds / CLOSE).min(1.);
        let shrink = cubic_bezier(0.42, 0., 1., 1., t);
        Pose {
            scale: self.scale + (FROM_SCALE - self.scale) * shrink,
            page: self.page * (1. - t),
            card: self.card * (1. - t),
            dim: self.dim * (1. - t),
            ..self
        }
    }

    /// `seconds` after it started growing into the tab's place, from `self`.
    fn keeping(self, seconds: f32) -> Self {
        let t = cubic_bezier(0.42, 0., 0.58, 1., (seconds / KEEP).min(1.));
        Pose {
            scale: self.scale + (1. - self.scale) * t,
            kept: self.kept + (1. - self.kept) * t,
            page: self.page + (1. - self.page) * t,
            card: self.card * (1. - t),
            dim: self.dim * (1. - t),
        }
    }
}

/// Where the card and its page are drawn in `pose`, over a window's page at `page`, growing from
/// `link`: the card's frame before its scale about the returned point, and the page's after it.
fn frames(
    pose: Pose,
    page: Bounds<Pixels>,
    link: Point<Pixels>,
) -> (Bounds<Pixels>, Point<Pixels>, Bounds<Pixels>) {
    let rest = card(page);
    let about = point(
        link.x.clamp(rest.left(), rest.right()),
        link.y.clamp(rest.top(), rest.bottom()),
    );
    let card = between(rest, card_around(page), pose.kept);
    let shown = between(page_in(rest), page, pose.kept);
    (card, about, scaled_about(shown, pose.scale, about))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Leave {
    Close,
    Keep,
}

/// The preview of one page, from the click that opens it until it has left.
struct Preview {
    /// The browser window it shows over, and its page's frame and corners there.
    window: u64,
    page: Bounds<Pixels>,
    radius: f32,
    tab: u64,
    url: SharedString,
    /// Where the link was clicked in the browser window.
    link: Point<Pixels>,
    /// When its window was first on screen, where the motion starts.
    shown: Option<Instant>,
    settled: bool,
    /// Once it's leaving: when, how, and where the motion was.
    leaving: Option<(Instant, Leave, Pose)>,
    _leave: Option<Task<()>>,
}

impl Preview {
    fn pose(&self) -> Pose {
        match self.leaving {
            Some((at, Leave::Close, from)) => from.closing(at.elapsed().as_secs_f32()),
            Some((at, Leave::Keep, from)) => from.keeping(at.elapsed().as_secs_f32()),
            None => Pose::arriving(self.shown.map_or(0., |shown| shown.elapsed().as_secs_f32())),
        }
    }

    fn moving(&self) -> bool {
        self.leaving.is_some()
            || self.shown.is_none_or(|shown| shown.elapsed().as_secs_f32() < OPEN)
    }

    /// The page's corners, which the card's follow outside the frame.
    fn radius(&self) -> f32 {
        self.radius.max(MIN_RADIUS)
    }
}

pub struct LinkPreview {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    preview: Option<Preview>,
}

struct LinkPreviewWindow(WindowHandle<LinkPreview>);

impl Global for LinkPreviewWindow {}

/// Opens the preview's window, hidden, for `host` to show over a browser window, ahead of time as
/// the dialog's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<LinkPreview>> {
    cx.bind_keys([
        KeyBinding::new("escape", Close, Some("LinkPreview")),
        KeyBinding::new("cmd-w", Close, Some("LinkPreview")),
        KeyBinding::new("cmd-enter", Keep, Some("LinkPreview")),
    ]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(800.), px(600.)),
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
        // The dim and the card are drawn.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| LinkPreview { host, focus: cx.focus_handle(), preview: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(LinkPreviewWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the link preview's window: {error}");
            None
        }
    }
}

fn update<R>(
    cx: &mut App,
    change: impl FnOnce(&mut LinkPreview, &mut Window, &mut Context<LinkPreview>) -> R,
) -> Option<R> {
    let window = cx.try_global::<LinkPreviewWindow>().map(|preview| preview.0)?;
    window.update(cx, change).ok()
}

/// Shows `tab`'s page, at `url`, over the browser window `window`, whose page is at `page` with
/// corners of that radius, growing from `link` there, where the link was clicked. A preview
/// showing goes at once.
pub fn show(
    window: u64,
    tab: u64,
    url: SharedString,
    (page, radius): (Bounds<Pixels>, f32),
    link: Point<Pixels>,
    cx: &mut App,
) {
    update(cx, |preview, gpui_window, cx| {
        preview.finish();
        // check.sh reads this line.
        log!("browser: link preview {tab} {url}");
        let shown = Preview {
            window,
            page,
            radius,
            tab,
            url,
            link,
            shown: None,
            settled: false,
            leaving: None,
            _leave: None,
        };
        preview.host.place(tab, page_in(card(page)), shown.radius());
        preview.host.show(window);
        preview.preview = Some(shown);
        // The page has the keyboard, and the header's keys are there once a click gives it to the
        // card.
        gpui_window.focus(&preview.focus, cx);
        cx.notify();
    });
}

/// The browser window `window` laid its page out at `page`, as it resized.
pub fn page_moved(window: u64, (page, radius): (Bounds<Pixels>, f32), cx: &mut App) {
    update(cx, |preview, _, cx| {
        let host = preview.host.clone();
        let Some(shown) = preview.preview.as_mut() else {
            return;
        };
        let moved = (shown.page, shown.radius) != (page, radius);
        if shown.window != window || shown.leaving.is_some() || !moved {
            return;
        }
        shown.page = page;
        shown.radius = radius;
        host.place(shown.tab, page_in(card(page)), shown.radius());
        cx.notify();
    });
}

/// `tab`'s page is at `url` now, as it navigated.
pub fn set_url(tab: u64, url: SharedString, cx: &mut App) {
    update(cx, |preview, _, cx| {
        if let Some(shown) = preview.preview.as_mut().filter(|shown| shown.tab == tab) {
            shown.url = url;
            cx.notify();
        }
    });
}

/// The preview's tab, while one shows.
pub fn tab(cx: &mut App) -> Option<u64> {
    update(cx, |preview, _, _| preview.preview.as_ref().map(|shown| shown.tab)).flatten()
}

/// What `keystroke` from `tab`'s page does, if it's the preview's: it's done, and whether the
/// page loses it. None for another tab's page.
pub fn key(tab: u64, keystroke: &str, reserved: bool, cx: &mut App) -> Option<bool> {
    update(cx, |preview, _, cx| {
        preview.preview.as_ref().filter(|shown| shown.tab == tab)?;
        Some(match key_action(keystroke, reserved) {
            Some(KeyAction::Close) => preview.close(keystroke, cx),
            Some(KeyAction::Keep) => preview.keep(keystroke, cx),
            None => false,
        })
    })
    .flatten()
}

/// The user left the preview for another window. The host has released it already.
pub fn dismissed(cx: &mut App) {
    update(cx, |preview, _, cx| preview.close("dismissed", cx));
}

/// The tab closed, and its preview goes with it at once.
pub fn tab_closed(tab: u64, cx: &mut App) {
    update(cx, |preview, _, cx| {
        if preview.preview.as_ref().is_some_and(|shown| shown.tab == tab) {
            preview.preview = None;
            preview.host.hide();
            cx.notify();
        }
    });
}

impl LinkPreview {
    /// Closes the preview back into its link, giving the browser window the keyboard and the
    /// clicks at once, and its page once it has left. Whether it was showing.
    fn close(&mut self, why: &str, cx: &mut Context<Self>) -> bool {
        self.leave(Leave::Close, why, CLOSE, cx)
    }

    /// Grows the preview into the tab's place, then keeps its page there as a Today tab.
    fn keep(&mut self, why: &str, cx: &mut Context<Self>) -> bool {
        self.leave(Leave::Keep, why, KEEP, cx)
    }

    fn leave(&mut self, how: Leave, why: &str, seconds: f32, cx: &mut Context<Self>) -> bool {
        let Some(shown) = self.preview.as_mut().filter(|shown| shown.leaving.is_none()) else {
            return false;
        };
        shown.leaving = Some((Instant::now(), how, shown.pose()));
        self.host.release();
        // check.sh reads this line.
        let what = if how == Leave::Keep { "keeping" } else { "closing" };
        log!("browser: link preview {what} ({why})");
        let tab = shown.tab;
        shown._leave = Some(cx.spawn(async move |preview, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(seconds)).await;
            preview
                .update(cx, |preview, cx| {
                    // Unless another preview took its place meanwhile.
                    if preview.preview.as_ref().is_some_and(|shown| shown.tab == tab) {
                        preview.finish();
                        cx.notify();
                    }
                })
                .ok();
        }));
        cx.notify();
        true
    }

    /// Ends the preview shown, if any, where its leaving would: its page closed or kept, and its
    /// window hidden.
    fn finish(&mut self) {
        let Some(shown) = self.preview.take() else {
            return;
        };
        match shown.leaving {
            Some((_, Leave::Keep, _)) => {
                self.host.keep(shown.window, shown.tab);
                // check.sh reads this line.
                log!("browser: link preview kept {}", shown.tab);
                self.host.hide();
            }
            _ => {
                self.host.hide();
                self.host.close(shown.tab);
            }
        }
    }
}

impl Render for LinkPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .key_context("LinkPreview")
            .track_focus(&self.focus)
            .on_action(cx.listener(|preview, _: &Close, _, cx| {
                preview.close("escape", cx);
            }))
            .on_action(cx.listener(|preview, _: &Keep, _, cx| {
                preview.keep("cmd-enter", cx);
            }))
            .size_full()
            .font_family(".SystemUIFont");
        let Some(shown) = self.preview.as_mut() else {
            return root;
        };
        // AppKit puts the window on screen some frames after it's asked to, and only then does
        // GPUI draw it, so the motion starts there rather than skip its start.
        if shown.shown.is_none() && window.is_visible() {
            shown.shown = Some(Instant::now());
        }
        let pose = shown.pose();
        let moving = shown.moving();
        let settling = !moving && !shown.settled;
        shown.settled |= settling;
        if moving {
            window.request_animation_frame();
        }
        let (card_frame, about, page) = frames(pose, shown.page, shown.link);
        if moving || settling {
            self.host.drew(page, pose.page);
        }
        if settling {
            // check.sh reads this line.
            log!("browser: link preview settled");
        }
        let radius = shown.radius() + FRAME;
        let site = SharedString::from(site_of(&shown.url).to_string());
        let button = |id: &'static str, icon: &'static str| {
            div()
                .id(id)
                .flex_none()
                .size(px(URL_BUTTON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .hover(|style| style.bg(rgba(NAV_HOVERED)))
                .active(|style| style.bg(rgba(PRESSED)))
                .child(svg().path(icon).size(px(URL_BUTTON_GLYPH)).text_color(rgba(NAV)))
        };
        let header = div()
            .h(px(HEADER))
            .flex()
            .items_center()
            .gap(px(URL_BUTTON_INSET))
            .pl(px(HEADER_LEFT))
            .pr(px(URL_BUTTON_INSET))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(HEADER_TEXT))
                    .text_color(rgba(TITLE))
                    .child(site),
            )
            .child(
                button("link-preview-keep", "icons/maximize.svg").on_click(
                    cx.listener(|preview, _, _, cx| {
                        preview.keep("button", cx);
                    }),
                ),
            )
            .child(button("link-preview-close", "icons/x.svg").on_click(cx.listener(
                |preview, _, _, cx| {
                    preview.close("button", cx);
                },
            )));
        let card = div()
            .id("link-preview-card")
            .absolute()
            .left(card_frame.left())
            .top(card_frame.top())
            .w(card_frame.size.width)
            .h(card_frame.size.height)
            .rounded(px(radius))
            .bg(rgb(FILL))
            .border_1()
            .border_color(rgba(EDGE))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.35),
                offset: point(px(0.), px(8.)),
                blur_radius: px(32.),
                spread_radius: px(0.),
                inset: false,
            }])
            .opacity(pose.card)
            // Clicks on the card stay off the dim under it.
            .occlude()
            .child(header);
        let dim = div()
            .id("link-preview-dim")
            .absolute()
            .size_full()
            .bg(Hsla::from(rgb(DIM)).opacity(DIM_OPACITY * pose.dim))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|preview, _, _, cx| {
                    preview.close("click outside", cx);
                }),
            );
        root.child(dim).child(scaled(card, pose.scale, about))
    }
}

#[cfg(test)]
mod tests;
