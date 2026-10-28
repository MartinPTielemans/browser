//! The picture of a page that another window shows, which the window it left draws in its card, and
//! a click on it takes the page back (docs/ux-spec.md › Window › Several windows; ours).

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use web_time::Instant;

use gpui::{
    AnyElement, AsyncApp, Bounds, Context, Corners, MouseButton, Pixels, RenderImage, Task,
    WeakEntity, Window, canvas, div, hsla, point, prelude::*, px, size,
};
use image::{Frame, RgbaImage};

use super::{CARD_RADIUS, Shell, new_space::ease_out, pane_frames};
use crate::elevation::Level;

use crate::tokens::shell::picture::REST;
/// It steps back from the card's size to REST as the page goes to the other window, and comes
/// forward to the card's before the page takes its place. Both ease out.
const STEP_BACK: f32 = 0.2;
const COME_BACK: f32 = 0.12;
use crate::tokens::shell::picture::VEIL;
use crate::tokens::shell::picture::RIM;
/// What a request for a picture is numbered from, above the numbers capture mode's count up from.
const FIRST_REQUEST: u64 = 1 << 63;

static REQUESTS: AtomicU64 = AtomicU64::new(FIRST_REQUEST);

/// Whether the core's answer `request` is for a picture of this kind, not capture mode's.
pub fn is_picture(request: u64) -> bool {
    request >= FIRST_REQUEST
}

/// The core's answer to `request`, a picture of this kind, for whichever of `shells` asked. It
/// reaches them once the update under way, if any, is over: an answer the core gave inside the call
/// that asked would come while that shell is being updated, and a second update of it panics.
pub fn picture_answered(
    shells: Vec<WeakEntity<Shell>>,
    request: u64,
    png: Option<Vec<u8>>,
    cx: &AsyncApp,
) {
    cx.spawn(async move |cx| {
        for shell in shells {
            shell.update(cx, |shell, cx| shell.page_pictured(request, png.clone(), cx)).ok();
        }
    })
    .detach();
}

/// A page's picture, decoded in the BGRA GPUI draws, and its size in pixels.
struct Ready {
    image: Arc<RenderImage>,
    size: (f32, f32),
}

/// The scale of the picture going from `from` to `to`, since `start`.
#[derive(Clone, Copy)]
struct Move {
    start: Instant,
    from: f32,
    to: f32,
    duration: f32,
}

impl Move {
    fn at(&self, now: Instant) -> f32 {
        let t = now.saturating_duration_since(self.start).as_secs_f32() / self.duration;
        self.from + (self.to - self.from) * ease_out(t)
    }

    fn done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start).as_secs_f32() >= self.duration
    }
}

/// What a window shows in place of a page another window shows: the picture of the tab it has
/// selected. At most one per window.
pub(super) struct Picture {
    tab: u64,
    /// The last request made to the core, and whether its answer hasn't come.
    request: u64,
    pending: bool,
    image: Option<Ready>,
    motion: Option<Move>,
    /// Taking the page back, once the picture has grown to the card.
    returning: Option<Task<()>>,
}

impl Picture {
    /// How big it is, as a share of the card, at `now`.
    fn scale(&self, now: Instant) -> f32 {
        self.motion.map_or(REST, |motion| motion.at(now))
    }
}

impl Shell {
    /// The core says another window shows the page of the tab this one has selected, or with 0 that
    /// this one shows a page again.
    pub fn page_elsewhere(&mut self, tab: u64, cx: &mut Context<Self>) {
        if tab == 0 {
            return self.drop_picture(cx);
        }
        // A report that was overtaken by the window's own pick.
        if tab != self.selected || self.picture.as_ref().is_some_and(|picture| picture.tab == tab) {
            return;
        }
        self.drop_picture(cx);
        self.picture = Some(Picture {
            tab,
            request: 0,
            pending: false,
            image: None,
            motion: None,
            returning: None,
        });
        self.ask_picture(cx);
        cx.notify();
    }

    /// Whether the window shows `tab`'s picture, or waits for it, in place of its page.
    pub(super) fn pictured(&self, tab: u64) -> bool {
        self.picture.as_ref().is_some_and(|picture| picture.tab == tab)
    }

    /// The window selected `tab` or none: the picture of any other tab goes.
    pub(super) fn selected_for_picture(&mut self, tab: u64, cx: &mut Context<Self>) {
        if !self.pictured(tab) {
            self.drop_picture(cx);
        }
    }

    /// The window is in front: its picture is of the page as it is now.
    pub(super) fn refresh_picture(&mut self, cx: &mut Context<Self>) {
        let seen = self.picture.as_ref().is_some_and(|picture| {
            picture.image.is_some() && !picture.pending && picture.returning.is_none()
        });
        if seen {
            self.ask_picture(cx);
        }
    }

    fn ask_picture(&mut self, cx: &mut Context<Self>) {
        let Some(picture) = &mut self.picture else {
            return;
        };
        picture.request = REQUESTS.fetch_add(1, Ordering::Relaxed);
        picture.pending = true;
        let (tab, request) = (picture.tab, picture.request);
        self.browser.page_picture(tab, request, cx);
    }

    fn drop_picture(&mut self, cx: &mut Context<Self>) {
        let Some(picture) = self.picture.take() else {
            return;
        };
        if let Some(ready) = picture.image {
            self.stale_pictures.push(ready.image);
        }
        if picture.pending {
            self.browser.page_picture_end(picture.tab, cx);
        }
        cx.notify();
    }

    /// The core answers `request` with the picture as a PNG, or none.
    pub fn page_pictured(&mut self, request: u64, png: Option<Vec<u8>>, cx: &mut Context<Self>) {
        let Some(picture) = self.picture.as_mut().filter(|picture| picture.request == request)
        else {
            return;
        };
        picture.pending = false;
        let tab = picture.tab;
        self.browser.page_picture_end(tab, cx);
        let Some(png) = png else {
            return;
        };
        // Decoded away from the main thread, as a big window's page is millions of pixels.
        let decoded = cx.background_spawn(async move { decode(&png) });
        cx.spawn(async move |shell, cx| {
            let decoded = decoded.await;
            shell.update(cx, |shell, cx| shell.picture_decoded(request, decoded, cx)).ok();
        })
        .detach();
    }

    fn picture_decoded(&mut self, request: u64, decoded: Option<Ready>, cx: &mut Context<Self>) {
        let window = self.browser.window();
        let Some(picture) = self.picture.as_mut().filter(|picture| picture.request == request)
        else {
            return;
        };
        let Some(ready) = decoded else {
            return;
        };
        // The page steps back as it goes, the first time; a fresh picture of it replaces the old
        // where it is.
        if picture.image.is_none() {
            picture.motion =
                Some(Move { start: Instant::now(), from: 1., to: REST, duration: STEP_BACK });
        }
        if let Some(old) = picture.image.replace(ready) {
            self.stale_pictures.push(old.image);
        }
        // check.sh waits for this line.
        log!("browser: window {window} pictures tab {}", picture.tab);
        cx.notify();
    }

    /// A press on the picture: it grows to the card, then the page comes back by the call a press
    /// on the tab's row makes.
    fn press_picture(&mut self, cx: &mut Context<Self>) {
        let Some(picture) = self.picture.as_mut() else {
            return;
        };
        if picture.returning.is_some() || picture.image.is_none() {
            return;
        }
        let tab = picture.tab;
        let from = picture.scale(Instant::now());
        picture.motion = Some(Move { start: Instant::now(), from, to: 1., duration: COME_BACK });
        // The page is live about as soon as the picture has grown, which is within the 130 ms a
        // click may take to be readable.
        picture.returning = Some(cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(COME_BACK)).await;
            shell.update(cx, |shell, cx| shell.take_page_back(tab, cx)).ok();
        }));
        cx.notify();
    }

    fn take_page_back(&mut self, tab: u64, cx: &mut Context<Self>) {
        if self.pictured(tab) {
            log!("browser: command select via picture");
            self.select(tab, cx);
        }
    }

    /// The card of the page the picture stands for: the focused pane's, in a split view.
    fn picture_card(&self) -> Bounds<Pixels> {
        let (frame, _) = self.page_frame(self.viewport);
        let panes = self.panes(self.selected);
        let focused = panes.iter().position(|&pane| pane == self.selected).unwrap_or(0);
        let (stacked, shares) = (self.stacked(self.selected), self.shares(self.selected));
        pane_frames(frame, panes.len(), stacked, &shares)[focused]
    }

    /// Where the picture is at `now`: fitted to its scale of the card, centred in it.
    pub(super) fn picture_bounds(&self, now: Instant) -> Option<Bounds<Pixels>> {
        let picture = self.picture.as_ref()?;
        let ready = picture.image.as_ref()?;
        Some(fit(self.picture_card(), ready.size, picture.scale(now)))
    }

    /// Steps the motion for a frame, and lets go of the pictures the window has no use for.
    pub(super) fn step_picture(&mut self, window: &mut Window) {
        for image in self.stale_pictures.drain(..) {
            window.drop_image(image).ok();
        }
        let now = Instant::now();
        let id = self.browser.window();
        let Some(picture) = &mut self.picture else {
            return;
        };
        let Some(motion) = picture.motion else {
            return;
        };
        if !motion.done(now) {
            window.request_animation_frame();
        } else if picture.returning.is_none() {
            picture.motion = None;
            // check.sh waits for this line, which says the picture has stopped moving.
            log!("browser: window {id} picture rests");
        }
    }

    /// The picture over the card, which takes a press.
    pub(super) fn picture_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let picture = self.picture.as_ref()?;
        let ready = picture.image.as_ref()?;
        let now = Instant::now();
        let bounds = self.picture_bounds(now)?;
        let scale = picture.scale(now);
        let veil = VEIL * (1. - (scale - REST) / (1. - REST)).clamp(0., 1.);
        let image = ready.image.clone();
        let radius = px(CARD_RADIUS);
        let picture = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let corners = Corners::all(radius);
                window.paint_image(bounds, bounds, corners, image.clone(), 0, false).ok();
            },
        )
        .absolute()
        .size_full();
        Some(
            div()
                .id("page-picture")
                .group("page-picture")
                .absolute()
                .left(bounds.left())
                .top(bounds.top())
                .w(bounds.size.width)
                .h(bounds.size.height)
                .rounded(radius)
                .shadow(Level::Resting.shadow())
                .cursor_pointer()
                // A press here is the picture's, not the window background's, which moves the
                // window.
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|shell, _, _, cx| {
                        cx.stop_propagation();
                        shell.press_picture(cx);
                    }),
                )
                .child(picture)
                // The veil goes in one frame under the pointer, as header buttons light.
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .rounded(radius)
                        .bg(hsla(0., 0., 0., veil))
                        .group_hover("page-picture", |veil| veil.bg(gpui::transparent_black())),
                )
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .rounded(radius)
                        .border(px(RIM))
                        .border_color(Level::Resting.rim()),
                )
                .into_any_element(),
        )
    }
}

/// The largest box of the picture's shape that fits `scale` of `card`, centred in it.
fn fit(card: Bounds<Pixels>, picture: (f32, f32), scale: f32) -> Bounds<Pixels> {
    let (card_width, card_height) = (f32::from(card.size.width), f32::from(card.size.height));
    let (room_width, room_height) = (card_width * scale, card_height * scale);
    let zoom = (room_width / picture.0).min(room_height / picture.1);
    let (width, height) = (picture.0 * zoom, picture.1 * zoom);
    Bounds::new(
        point(
            card.left() + px((card_width - width) / 2.),
            card.top() + px((card_height - height) / 2.),
        ),
        size(px(width), px(height)),
    )
}

fn decode(png: &[u8]) -> Option<Ready> {
    let decoded = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let decoded = decoded.into_rgba8();
    let (width, height) = decoded.dimensions();
    let mut pixels = decoded.into_raw();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let image = RgbaImage::from_raw(width, height, pixels)?;
    Some(Ready {
        image: Arc::new(RenderImage::new(vec![Frame::new(image)])),
        size: (width as f32, height as f32),
    })
}

#[cfg(test)]
mod tests;
