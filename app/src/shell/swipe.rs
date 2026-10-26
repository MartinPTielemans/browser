//! Two fingers or a wheel across the sidebar, paging between Spaces and out to the
//! Library (docs/ux-spec.md › Spaces › Switching), and the list's scroll edge.

use std::{collections::VecDeque, time::Instant};

use gpui::{
    Context, DispatchPhase, Point, ScrollDelta, ScrollEvent, TouchPhase, Window, canvas, div,
    prelude::*, px, rgba,
};

use super::{
    SCROLL_EDGE, SCROLL_EDGE_FADE, SWIPE_FLICK, SWIPE_HISTORY, SWIPE_TRACKING,
    Settle, Shell, Space, Swipe, WHEEL_BURST, WHEEL_LINE, WHEEL_REST, fade_at, rubber_band,
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
        let beside = beside.or_else(|| self.beside(space, x));
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
    /// the Space shown, and past where it opens takes it and the card on, on the Library's band.
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

    /// Shows the first Space as fingers or a wheel leave the Library, which sits left of it, though
    /// it was opened from another (Arc 1.170 rec, three Spaces).
    fn show_first_space(&mut self, cx: &mut Context<Self>) {
        if let Some(first) = self.spaces.first() {
            self.show_space(first.id, cx);
        }
    }

    /// Whether fingers going right from `space` bring out the Library: from the first Space, in the
    /// docked sidebar, of a regular window.
    fn library_beside(&self, space: u64) -> bool {
        let first = self.spaces.first().is_some_and(|first| first.id == space);
        first && !self.sidebar_hidden && !self.page_fullscreen && !self.private
    }

    /// Fingers from the second Space that have brought the first all the way in carry on into the
    /// Library beside it, in the same swipe, as Arc's do (Arc 1.170 rec, three Spaces): the first
    /// Space shows, and the swipe goes on as if it began there.
    fn carry_into_library(&mut self, swipe: &mut Swipe, cx: &mut Context<Self>) {
        let width = self.width();
        let Some(first) = self.spaces.first().map(|space| space.id) else {
            return;
        };
        let second = self.spaces.get(1).is_some_and(|space| space.id == swipe.from);
        let carries = second && !swipe.library && swipe.way >= width && self.form.is_none();
        if !carries || !self.library_beside(first) {
            return;
        }
        (swipe.from, swipe.nearest, swipe.way) = (first, first, swipe.way - width);
        for (_, x) in &mut swipe.samples {
            *x -= width;
        }
        self.show_space(first, cx);
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

    /// The Space whose page comes in beside `space`'s while that's at `x` (`beside`).
    fn beside(&self, space: u64, x: f32) -> Option<u64> {
        let form = self.form.as_ref().map(|form| (form.space, form.from));
        beside(&self.spaces, form, space, x)
    }

    /// The Space whose page is nearest its place while `from`'s is at `x` (`nearest`).
    fn nearest(&self, from: u64, x: f32) -> Option<u64> {
        let form = self.form.as_ref().map(|form| (form.space, form.from));
        nearest(&self.spaces, form, from, x, self.width())
    }

    /// Fingers or a wheel over the sidebar. Across it they page between Spaces, and along it they
    /// scroll the list. Returns whether paging took the event.
    fn scroll(&mut self, event: &ScrollEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // The form pages back to the Space it was made from, but not while another Space shows.
        let form_hidden = self.form.is_some() && self.form().is_none();
        if form_hidden || self.renaming.is_some() || self.drag.is_some() {
            return false;
        }
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => delta.map(f32::from),
            ScrollDelta::Lines(lines) => return self.wheel(lines, window, cx),
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
            TouchPhase::Ended | TouchPhase::Cancelled => return self.release(window, cx),
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
            self.carry_into_library(&mut swipe, cx);
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
    /// on the one nearest its place, keeping the release's speed if it's heading there. Off the
    /// form, that's Cancel, its page going out as the Space it was made from settles.
    fn release(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
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
            if !open {
                self.show_first_space(cx);
            }
            self.move_library(open, out, velocity / self.library_width(), None, cx);
            return true;
        }
        if swipe.ready {
            self.make_pulled_space(x, cx);
            return true;
        }
        let toward = heading(x, velocity, self.width());
        let Some(space) = self.nearest(swipe.from, toward) else {
            return true;
        };
        let from = if space == swipe.from { x } else { x - toward.signum() * self.width() };
        let velocity = if velocity * from < 0. { velocity } else { 0. };
        let cancel = space != swipe.from && self.form().is_some();
        let beside = cancel.then_some(swipe.from);
        self.settle =
            Some(Settle { space, beside, from, velocity, start: now, next_door: false });
        if cancel {
            self.form_going = self.close_form(window, cx);
        } else if space != self.space {
            // check.sh waits for this line.
            log!("browser: command select-space via swipe");
            self.show_space(space, cx);
        }
        cx.notify();
        true
    }

    /// A wheel's ticks across the sidebar page once per burst, then rest. Past the first Space they
    /// open the Library, and back they close it. Back off the form they cancel it.
    fn wheel(&mut self, lines: Point<f32>, window: &mut Window, cx: &mut Context<Self>) -> bool {
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
                if !library {
                    self.show_first_space(cx);
                }
                self.set_library(library, cx);
            } else if !self.library_open
                && let Some(space) = self.nearest(self.space, sum.signum() * self.width())
                && space != self.space
            {
                if self.form().is_some() {
                    self.cancel_space(window, cx);
                    return true;
                }
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
                window.on_mouse_event(move |event: &ScrollEvent, phase, window, cx| {
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    let took = shell.update(cx, |shell, cx| {
                        let over = match in_window {
                            true => event.position.x < px(shell.card_left() + shell.card_push()),
                            false => bounds.contains(&event.position),
                        };
                        (over || shell.swipe.is_some()) && shell.scroll(event, window, cx)
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

/// The Space in `spaces` whose page comes in beside `space`'s while that's at `x`: the one before
/// it for `x` right of its place, else the one after. The form's, `(space, from)`, comes in from
/// the Space it was made from, before it, and has none after.
fn beside(spaces: &[Space], form: Option<(u64, u64)>, space: u64, x: f32) -> Option<u64> {
    if let Some((_, from)) = form.filter(|&(form, _)| form == space) {
        let made_from = spaces.iter().any(|other| other.id == from);
        return (x > 0. && made_from).then_some(from);
    }
    let index = spaces.iter().position(|other| other.id == space)?;
    let index = if x > 0. { index.checked_sub(1)? } else { index + 1 };
    spaces.get(index).map(|other| other.id)
}

/// The Space whose page is nearest its place while `from`'s is at `x`, pages `width` wide: it or
/// the one beside.
fn nearest(
    spaces: &[Space],
    form: Option<(u64, u64)>,
    from: u64,
    x: f32,
    width: f32,
) -> Option<u64> {
    spaces.iter().find(|space| space.id == from)?;
    match (x / width).abs() >= 0.5 {
        true => beside(spaces, form, from, x).or(Some(from)),
        false => Some(from),
    }
}

/// Where a page let go at `x`, going at `velocity` a second, heads: a page `width` that way on a
/// flick, else where it is.
fn heading(x: f32, velocity: f32, width: f32) -> f32 {
    if velocity.abs() >= SWIPE_FLICK { velocity.signum() * width } else { x }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{
        Context, Entity, ScrollDelta, ScrollEvent, TestAppContext, TouchPhase, VisualTestContext,
        Window, point, px,
    };

    use super::{heading, nearest};
    use crate::shell::{Folders, SWIPE_TRACKING, Shell, Space, recorder::Recorder};

    fn spaces(ids: &[u64]) -> Vec<Space> {
        let space = |&id| Space {
            id,
            name: format!("Space {id}").into(),
            color: None,
            icon: Default::default(),
            profile: 0,
            agent: Default::default(),
            pinned: Vec::new(),
            today: Vec::new(),
            splits: Vec::new(),
            folders: Folders::default(),
            shown: 0,
        };
        ids.iter().map(space).collect()
    }

    /// Where fingers let go of a page at `x` going at `velocity` land it, with Space 4's form
    /// made from Space 1, pages 200 wide.
    fn lands(from: u64, x: f32, velocity: f32) -> Option<u64> {
        let spaces = spaces(&[1, 2, 3, 4]);
        nearest(&spaces, Some((4, 1)), from, heading(x, velocity, 200.), 200.)
    }

    /// Back off the form, fingers land on the Space it was made from, not the one before it, past
    /// halfway or on a flick; short of that, and going on past it, they land back on the form.
    #[test]
    fn a_swipe_back_off_the_form_lands_where_it_was_made() {
        assert_eq!(lands(4, 120., 0.), Some(1));
        assert_eq!(lands(4, 20., 300.), Some(1));
        assert_eq!(lands(4, 60., 0.), Some(4));
        assert_eq!(lands(4, -30., -300.), Some(4));
        assert_eq!(lands(4, 120., -300.), Some(4));
    }

    /// Other Spaces page to their neighbours as before, and not past the first and last.
    #[test]
    fn other_spaces_land_beside_them() {
        assert_eq!(lands(2, 120., 0.), Some(1));
        assert_eq!(lands(2, -120., 0.), Some(3));
        assert_eq!(lands(3, -20., -300.), Some(4));
        assert_eq!(lands(1, 120., 0.), Some(1));
    }

    /// A window showing Space `shown` of three, as the core leaves it.
    fn three_spaces(
        cx: &mut TestAppContext,
        shown: u64,
    ) -> (Entity<Shell>, Rc<Recorder>, &mut VisualTestContext) {
        let host = Rc::new(Recorder::default());
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            let space = |id: u64| (id, format!("Space {id}").into(), None, 1, "".into(), "".into());
            shell.set_spaces(vec![space(1), space(2), space(3)], cx);
            shell.set_space(shown, cx);
        });
        cx.run_until_parked();
        (shell, host, cx)
    }

    /// Two fingers across the sidebar, down, moved `way` of the page's way in one go, and up.
    fn swipe(shell: &mut Shell, way: f32, window: &mut Window, cx: &mut Context<Shell>) {
        let fingers = |touch_phase, x| ScrollEvent {
            delta: ScrollDelta::Pixels(point(px(x), px(0.))),
            touch_phase,
            ..Default::default()
        };
        use TouchPhase::{Ended, Moved, Started};
        for (phase, x) in [(Started, 0.), (Moved, way / SWIPE_TRACKING), (Ended, 0.)] {
            shell.scroll(&fingers(phase, x), window, cx);
        }
    }

    /// Fingers taking the Library back land on the first Space, which it sits left of, though it
    /// was opened from the second.
    #[gpui::test]
    fn a_swipe_back_from_the_library_lands_on_the_first_space(cx: &mut TestAppContext) {
        let (shell, host, cx) = three_spaces(cx, 2);
        shell.update(cx, |shell, cx| {
            shell.set_library(true, cx);
            // At rest, open.
            shell.library_motion = None;
        });
        cx.run_until_parked();
        shell.update_in(cx, |shell, window, cx| {
            swipe(shell, -400., window, cx);
            assert!(!shell.library_open, "the swipe left the Library open");
        });
        assert_eq!(*host.shown.borrow(), [1]);
    }

    /// One long swipe from the second of three Spaces brings the first all the way in and carries
    /// on into the Library beside it, as Arc's does, without the fingers letting go between.
    #[gpui::test]
    fn one_swipe_from_the_second_space_carries_on_into_the_library(cx: &mut TestAppContext) {
        let (shell, host, cx) = three_spaces(cx, 2);
        shell.update_in(cx, |shell, window, cx| {
            let way = shell.width() + 0.8 * shell.library_width();
            swipe(shell, way, window, cx);
            assert!(shell.library_open, "the swipe stopped at the first Space");
        });
        assert_eq!(*host.shown.borrow(), [1]);
    }

    /// Fingers let go once the first Space is in land on it with the Library closed, and the next
    /// swipe from there brings the Library out.
    #[gpui::test]
    fn a_swipe_let_go_on_the_first_space_leaves_the_library_to_the_next(
        cx: &mut TestAppContext,
    ) {
        let (shell, host, cx) = three_spaces(cx, 2);
        shell.update_in(cx, |shell, window, cx| {
            swipe(shell, 0.9 * shell.width(), window, cx);
            assert!(!shell.library_open, "the Library came out with the first Space");
            shell.set_space(1, cx);
            swipe(shell, 0.9 * shell.library_width(), window, cx);
            assert!(shell.library_open, "the next swipe left the Library closed");
        });
        assert_eq!(*host.shown.borrow(), [1]);
    }
}
