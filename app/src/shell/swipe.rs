//! Two fingers or a wheel across the sidebar, paging between Spaces and out to the
//! Library (docs/ux-spec.md › Spaces › Switching), and the list's scroll edge.

use std::{collections::VecDeque, time::Instant};

use gpui::{
    Context, DispatchPhase, Point, ScrollDelta, ScrollEvent, TouchPhase, Window, canvas, div,
    prelude::*, px, rgba,
};

use super::{
    SCROLL_EDGE, SCROLL_EDGE_FADE, SWIPE_FLICK, SWIPE_HISTORY, SWIPE_TRACKING,
    Settle, Shell, Swipe, WHEEL_BURST, WHEEL_LINE, WHEEL_REST, fade_at, rubber_band,
    library::library_band,
};

impl Shell {
    /// The Spaces' pages the sidebar draws, and where from its left edge: the shown Space's, and
    /// while a swipe or a settle moves one, it and the page beside it.
    pub(super) fn pages(&self) -> Vec<(u64, f32)> {
        let swipe = self.swipe.as_ref().filter(|swipe| {
            swipe.across == Some(true) && self.library_swiped(swipe).is_none()
        });
        let (space, x, beside) = match (swipe, &self.settle) {
            (Some(swipe), _) => (swipe.from, self.swiped_to(swipe), None),
            (None, Some(settle)) => (settle.space, settle.offset().unwrap_or(0.), settle.beside),
            (None, None) => return vec![(self.space, self.held_page().unwrap_or(0.))],
        };
        let beside = beside.or_else(|| {
            let index = self.spaces.iter().position(|other| other.id == space)?;
            let index = if x > 0. { index.checked_sub(1)? } else { index + 1 };
            self.spaces.get(index).map(|other| other.id)
        });
        let mut pages = vec![(space, x)];
        if x != 0.
            && let Some(beside) = beside
        {
            pages.push((beside, x - x.signum() * self.width()));
        }
        pages
    }

    /// Where a swipe has taken the page of the Space it began at: with the fingers, but
    /// rubber-banding past the first and last Spaces, and past one Space either side. Past the
    /// first Space is the Library, its width away. A swipe in the Library goes back that far, to
    /// the Space shown, and past where it opens takes the card on, on the Library's stiffer band.
    pub(super) fn swiped_to(&self, swipe: &Swipe) -> f32 {
        if swipe.library && swipe.way > 0. {
            return self.past_library(swipe.way);
        }
        if swipe.library {
            return rubber_band(swipe.way, -self.library_width(), 0.);
        }
        let index = self.spaces.iter().position(|space| space.id == swipe.from).unwrap_or(0);
        let width = self.width();
        let library = index == 0 && self.library_beside(swipe.from);
        // From the first Space, fingers that brought the Library out and go on meet its band too.
        if library && swipe.way > self.library_width() {
            return self.library_width() + self.past_library(swipe.way - self.library_width());
        }
        let high = match index {
            0 if library => self.library_width(),
            0 => 0.,
            _ => width,
        };
        if index + 1 < self.spaces.len() {
            return rubber_band(swipe.way, -width, high);
        }
        rubber_band(swipe.way, 0., high)
    }

    /// Where fingers `way` past where the Library opens take the page: the card goes on by the
    /// Library's band, and the page's way is the card's over its push.
    fn past_library(&self, way: f32) -> f32 {
        let push = (self.library_width() - self.card_left()).max(1.);
        self.library_width() * library_band(way) / push
    }

    /// Whether fingers going right from `space` bring out the Library: from the first Space, in the
    /// docked sidebar.
    fn library_beside(&self, space: u64) -> bool {
        let first = self.spaces.first().is_some_and(|first| first.id == space);
        first && !self.sidebar_hidden && !self.page_fullscreen
    }

    /// How far out a swipe has taken the Library (`library_out`), while it moves the Library.
    pub(super) fn library_swiped(&self, swipe: &Swipe) -> Option<f32> {
        if swipe.across != Some(true) {
            return None;
        }
        let x = self.swiped_to(swipe);
        match swipe.library {
            true => Some((1. + x / self.library_width()).max(0.)),
            false if x > 0. && self.library_beside(swipe.from) => Some(x / self.library_width()),
            false => None,
        }
    }

    /// The Space whose page is nearest its place while `from`'s is at `x`: it or a neighbour.
    fn nearest(&self, from: u64, x: f32) -> Option<u64> {
        let index = self.spaces.iter().position(|space| space.id == from)? as f32;
        let index = index - (x / self.width()).round().clamp(-1., 1.);
        let index = index.clamp(0., self.spaces.len().saturating_sub(1) as f32) as usize;
        self.spaces.get(index).map(|space| space.id)
    }

    /// Fingers or a wheel over the sidebar. Across it they page between Spaces, and along it they
    /// scroll the list. Returns whether paging took the event.
    fn scroll(&mut self, event: &ScrollEvent, cx: &mut Context<Self>) -> bool {
        if self.form.is_some() || self.renaming.is_some() || self.drag.is_some() {
            return false;
        }
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => delta.map(f32::from),
            ScrollDelta::Lines(lines) => return self.wheel(lines, cx),
        };
        match event.touch_phase {
            TouchPhase::Started => {
                self.swiped = false;
                self.swipe = Some(Swipe {
                    from: self.space,
                    library: self.library_open || self.library_motion.is_some(),
                    across: None,
                    way: 0.,
                    samples: VecDeque::new(),
                    nearest: self.space,
                    ready: false,
                    turned: None,
                });
            }
            TouchPhase::Ended | TouchPhase::Cancelled => return self.release(cx),
            TouchPhase::Moved => {}
        }
        let Some(mut swipe) = self.swipe.take() else {
            // A page swipe's momentum, across, doesn't scroll the list.
            return self.swiped && delta.x.abs() > delta.y.abs();
        };
        if swipe.across.is_none() && (delta.x != 0. || delta.y != 0.) {
            let across = delta.x.abs() > delta.y.abs();
            swipe.across = Some(across);
            // check.sh waits for this line.
            log!("browser: swipe {}", if across { "across" } else { "along" });
            // Fingers catch a page, or the Library, that's still settling where it is.
            if across && swipe.library {
                swipe.way = (self.library_out() - 1.) * self.library_width();
                self.library_motion = None;
            } else if across
                && let Some(settle) = self.settle.take()
                && let Some(x) = settle.offset()
            {
                (swipe.from, swipe.way, swipe.nearest) = (settle.space, x, settle.space);
            }
        }
        let across = swipe.across == Some(true);
        if across {
            let library_near = |shell: &Self, swipe: &Swipe| {
                shell.library_swiped(swipe).map(|out| out > 0.5)
            };
            let past_end = |shell: &Self, swipe: &Swipe| {
                shell.library_swiped(swipe).is_some_and(|out| out > 1.)
            };
            let was_near = library_near(self, &swipe);
            let was_past = past_end(self, &swipe);
            swipe.way += SWIPE_TRACKING * delta.x;
            if !was_past && past_end(self, &swipe) {
                // check.sh waits for this line.
                log!("browser: library past its end");
            }
            let (now, x) = (Instant::now(), self.swiped_to(&swipe));
            swipe.samples.push_back((now, x));
            while swipe.samples.front().is_some_and(|&(at, _)| now - at > SWIPE_HISTORY) {
                swipe.samples.pop_front();
            }
            self.ready_new_space(&mut swipe, cx);
            let near = library_near(self, &swipe);
            if near.is_some() {
                if self.library.is_none() {
                    self.show_section(self.section, cx);
                }
                if was_near.is_some() && was_near != near {
                    self.browser.detent(cx);
                }
            } else if let Some(nearest) = self.nearest(swipe.from, x)
                && nearest != swipe.nearest
            {
                swipe.nearest = nearest;
                self.browser.detent(cx);
            }
            cx.notify();
        }
        self.swipe = Some(swipe);
        across
    }

    /// The fingers lifted. The pages settle on the next Space in the direction of a flick, else
    /// on the one nearest its place, keeping the release's speed if it's heading there.
    fn release(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(swipe) = self.swipe.take().filter(|swipe| swipe.across == Some(true)) else {
            return false;
        };
        self.swiped = true;
        let (now, x) = (Instant::now(), self.swiped_to(&swipe));
        let recent = swipe.samples.iter().filter(|&&(at, _)| now - at <= SWIPE_HISTORY);
        let velocity = match (recent.clone().next(), recent.last()) {
            (Some(&(start, from)), Some(&(end, to))) if end > start => {
                (to - from) / (end - start).as_secs_f32()
            }
            _ => 0.,
        };
        if let Some(out) = self.library_swiped(&swipe) {
            let open = if velocity.abs() >= SWIPE_FLICK { velocity > 0. } else { out > 0.5 };
            if open != self.library_open {
                // check.sh waits for this line.
                log!("browser: command view-library via swipe");
            }
            self.move_library(open, out, velocity / self.library_width(), None, cx);
            return true;
        }
        if swipe.ready {
            self.make_pulled_space(x, cx);
            return true;
        }
        let toward = if velocity.abs() >= SWIPE_FLICK {
            velocity.signum() * self.width()
        } else {
            x
        };
        let Some(space) = self.nearest(swipe.from, toward) else {
            return true;
        };
        let index = |id| self.spaces.iter().position(|other| other.id == id).unwrap_or(0) as f32;
        let from = x + (index(space) - index(swipe.from)) * self.width();
        let velocity = if velocity * from < 0. { velocity } else { 0. };
        self.settle = Some(Settle { space, beside: None, from, velocity, start: now });
        if space != self.space {
            // check.sh waits for this line.
            log!("browser: command select-space via swipe");
            self.show_space(space, cx);
        }
        cx.notify();
        true
    }

    /// A wheel's ticks across the sidebar page once per burst, then rest. Past the first Space they
    /// open the Library, and back they close it.
    fn wheel(&mut self, lines: Point<f32>, cx: &mut Context<Self>) -> bool {
        if lines.x.abs() <= lines.y.abs() {
            return false;
        }
        let now = Instant::now();
        if self.wheel_paged.is_some_and(|paged| now - paged < WHEEL_REST) {
            return true;
        }
        let sum = match self.wheel {
            Some((sum, last)) if now - last < WHEEL_BURST => sum + lines.x,
            _ => lines.x,
        };
        self.wheel = Some((sum, now));
        if (sum * WHEEL_LINE).abs() > 1. {
            self.wheel = None;
            self.wheel_paged = Some(now);
            let library = sum > 0. && (self.library_open || self.library_beside(self.space));
            if library != self.library_open {
                // check.sh waits for this line.
                log!("browser: command view-library via wheel");
                self.set_library(library, cx);
            } else if !self.library_open
                && let Some(space) = self.nearest(self.space, sum.signum() * self.width())
                && space != self.space
            {
                // check.sh waits for this line.
                log!("browser: command select-space via wheel");
                self.show_space(space, cx);
            }
        }
        true
    }

    /// Fingers and wheels over the sidebar or the Library, heard ahead of the list, which would
    /// scroll with them. In the browser's window that's left of the card's edge, and in the
    /// reveal's, its panel. The rest of a swipe comes here wherever the pointer goes, as the
    /// sidebar or the Library may slide or fade from under it.
    pub(super) fn scroll_listener(
        &self,
        in_window: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let shell = cx.weak_entity();
        canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                window.on_mouse_event(move |event: &ScrollEvent, phase, _, cx| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    let took = shell.update(cx, |shell, cx| {
                        let over = match in_window {
                            true => event.position.x < px(shell.card_left() + shell.card_push()),
                            false => bounds.contains(&event.position),
                        };
                        (over || shell.swipe.is_some()) && shell.scroll(event, cx)
                    });
                    if took.unwrap_or(false) {
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }

    /// Fades the edge under the favorites in once the list is past its top, and out back there.
    pub(super) fn step_scroll_edge(&mut self, window: &mut Window) {
        let scrolled = self.list_scroll.offset().y < px(0.);
        if scrolled != self.scrolled {
            self.scroll_edge_fade = Some((Instant::now(), self.scroll_edge()));
            self.scrolled = scrolled;
        }
        if let Some((start, _)) = self.scroll_edge_fade {
            if start.elapsed().as_secs_f32() < SCROLL_EDGE_FADE {
                window.request_animation_frame();
            } else {
                self.scroll_edge_fade = None;
                // check.sh waits for these lines.
                log!("browser: scroll edge {}", if scrolled { "shown" } else { "hidden" });
            }
        }
    }

    /// How far the edge under the favorites has faded in.
    fn scroll_edge(&self) -> f32 {
        let to = if self.scrolled { 1. } else { 0. };
        self.scroll_edge_fade.map_or(to, |(start, from)| {
            fade_at(from, to, start.elapsed().as_secs_f32(), SCROLL_EDGE_FADE)
        })
    }

    /// The edge across the sidebar 4 under the favorites, or the URL field without them or while
    /// they scroll with the list, while the rows go under it (Dia).
    pub(super) fn scroll_edge_line(&self) -> Option<impl IntoElement + use<>> {
        let shown = self.scroll_edge();
        (shown > 0.).then(|| {
            div()
                .absolute()
                .top(px(self.pages_top() - 2.))
                .left_0()
                .w(px(self.width()))
                .h(px(1.))
                .bg(rgba(SCROLL_EDGE))
                .opacity(shown)
        })
    }
}
