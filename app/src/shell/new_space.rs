//! Past the last Space, fingers pull its page left a little and bring in a circle from the
//! sidebar's right edge, whose ring closes as they pull. Once it closes the circle readies a new
//! Space, and letting go makes one, as New Space… does (Arc on video; docs/ux-spec.md › Spaces).

use std::{f32::consts::PI, time::Instant};

use gpui::{
    BoxShadow, Context, Hsla, PathBuilder, Window, canvas, div, hsla, point, prelude::*, px, size,
};

use super::{Command, Settle, Shell, Swipe};

/// How far fingers pull the last Space's page past its place at most, and how far to ready a new
/// Space (Arc on video: 41 and 38 of a 448 sidebar, at 2×).
pub(super) const NEW_SPACE_PULL: f32 = 20.5;
const NEW_SPACE_READY: f32 = 19.;
/// The circle: across, its ring, and how far inside the sidebar's right edge its centre comes at
/// the full pull, from just past the edge.
const CIRCLE: f32 = 25.;
const RING: f32 = 2.;
const CIRCLE_INSET: f32 = 24.;
/// The + on it: its arms across, and their thickness.
const PLUS: f32 = 10.;
const PLUS_STROKE: f32 = 1.5;
/// Readying, the ring closes in on the cream over 80 ms to a dot, which turns into a cream + on
/// the dark as the circle pops to 1.2× and back over 130 ms.
const CLOSE: f32 = 0.08;
const DOT: f32 = 11.;
const POP: f32 = 0.13;
const POP_SCALE: f32 = 1.2;
/// The new Space's form comes in from this far right, easing out over 170 ms.
const ENTER: f32 = 23.;
const ENTER_TIME: f32 = 0.17;

impl Shell {
    /// Whether `swipe` has pulled the last Space's page far enough to ready a new Space.
    fn pull_ready(&self, swipe: &Swipe) -> bool {
        let last = self.spaces.last().is_some_and(|last| last.id == swipe.from);
        last && !swipe.library && self.swiped_to(swipe) <= -NEW_SPACE_READY
    }

    /// Readies a new Space as the fingers' pull reaches it, with the detent, and lets it go as
    /// they go back.
    pub(super) fn ready_new_space(&mut self, swipe: &mut Swipe, cx: &mut Context<Self>) {
        let ready = self.pull_ready(swipe);
        if ready == swipe.ready.is_some() {
            return;
        }
        swipe.ready = ready.then(Instant::now);
        if ready {
            // check.sh waits for this line.
            eprintln!("browser: new space ready");
            self.browser.detent(cx);
        }
    }

    /// Makes the Space the fingers readied, with the form for naming it. Its form comes in from
    /// just right of its place, and the window's colour goes to the new Space's almost at once.
    pub(super) fn make_pulled_space(&mut self, cx: &mut Context<Self>) {
        let before = self.space;
        self.run(Command::NewSpace, "swipe", cx);
        let Some(space) = self.form.as_ref().map(|form| form.space) else {
            return;
        };
        let start = Instant::now();
        self.settle = Some(Settle {
            space,
            beside: Some(before),
            from: ENTER,
            velocity: 0.,
            start,
        });
        self.form_entered = Some(start);
    }

    /// Where the form is from its place, as it comes in after a pull.
    pub(super) fn form_left(&self) -> f32 {
        let Some(start) = self.form_entered else {
            return 0.;
        };
        let t = (start.elapsed().as_secs_f32() / ENTER_TIME).min(1.);
        ENTER * (1. - t).powi(4)
    }

    /// Asks for frames while the circle readies or the form comes in.
    pub(super) fn step_new_space(&mut self, window: &mut Window) {
        let readying = self.swipe.as_ref().and_then(|swipe| swipe.ready);
        if readying.is_some_and(|at| at.elapsed().as_secs_f32() < CLOSE + POP) {
            window.request_animation_frame();
        }
        match self.form_entered {
            Some(start) if start.elapsed().as_secs_f32() < ENTER_TIME => {
                window.request_animation_frame();
            }
            Some(_) => self.form_entered = None,
            None => {}
        }
    }

    /// How far past its place the last Space's page is, while fingers or its settle have it there.
    fn pulled(&self) -> Option<f32> {
        if self.form().is_some() {
            return None;
        }
        let (space, x) = *self.pages().first()?;
        let last = self.spaces.last().is_some_and(|last| last.id == space);
        (last && x < 0.).then_some(-x)
    }

    /// The circle at the sidebar's right edge, halfway down, while the last Space's page is pulled.
    pub(super) fn new_space_circle(&self) -> Option<impl IntoElement + use<>> {
        let pulled = self.pulled()?;
        let fill = self.window_fill();
        let dark = Hsla { l: fill.l * 0.6, ..fill };
        let cream = hsla(0., 0., 0.96, 1.).blend(fill.opacity(0.08));
        let readying = self.swipe.as_ref().and_then(|swipe| swipe.ready);
        let t = readying.map(|at| at.elapsed().as_secs_f32());
        let popped = t.map_or(0., |t| ((t - CLOSE) / POP).clamp(0., 1.));
        let across = CIRCLE * (1. + (POP_SCALE - 1.) * (PI * popped).sin());
        let centre = self.width() + CIRCLE / 2.
            - (CIRCLE / 2. + CIRCLE_INSET) * (pulled / NEW_SPACE_PULL).min(1.);
        let disc = div()
            .relative()
            .size(px(across))
            .rounded_full()
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.3),
                offset: point(px(0.), px(1.)),
                blur_radius: px(4.),
                spread_radius: px(0.),
                inset: false,
            }]);
        let disc = match t {
            None => disc
                .bg(cream)
                .child(ring((pulled / NEW_SPACE_READY).min(1.), dark))
                .children(plus(across, PLUS, PLUS_STROKE, dark)),
            // The ring closes in on the cream, then the dot left turns into a +.
            Some(t) if t < CLOSE => {
                let dot = CIRCLE - 2. * RING + (DOT - CIRCLE + 2. * RING) * ease_out(t / CLOSE);
                disc.bg(dark).children(plus(across, dot, dot, cream))
            }
            Some(_) => {
                let way = ease_out(popped);
                let arms = DOT + (PLUS - DOT) * way;
                let stroke = DOT + (PLUS_STROKE - DOT) * way;
                disc.bg(dark).children(plus(across, arms, stroke, cream))
            }
        };
        Some(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(centre - across / 2.))
                .w(px(across))
                .flex()
                .items_center()
                .child(disc),
        )
    }
}

/// The dark ring over the cream's edge, from 12 o'clock anticlockwise `way` of the way round.
fn ring(way: f32, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let radius = px(CIRCLE / 2. - RING / 2.);
            let centre = bounds.center();
            let at = |angle: f32| {
                centre + point(-radius * angle.sin(), -radius * angle.cos())
            };
            let mut path = PathBuilder::stroke(px(RING));
            path.move_to(at(0.));
            // An arc of half the way or less at a time, so a full ring closes.
            let mut done = 0.;
            while done < way {
                let step = (way - done).min(0.5);
                done += step;
                path.arc_to(point(radius, radius), px(0.), false, false, at(2. * PI * done));
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .absolute()
    .size_full()
}

/// A + `arms` across with arms `stroke` thick and round ends, centred on a disc `across` wide. At
/// `arms` = `stroke` it's a dot.
fn plus(across: f32, arms: f32, stroke: f32, color: Hsla) -> [impl IntoElement; 2] {
    let bar = |bar: gpui::Size<f32>| {
        div()
            .absolute()
            .left(px((across - bar.width) / 2.))
            .top(px((across - bar.height) / 2.))
            .w(px(bar.width))
            .h(px(bar.height))
            .rounded(px(stroke / 2.))
            .bg(color)
    };
    [bar(size(arms, stroke)), bar(size(stroke, arms))]
}

fn ease_out(t: f32) -> f32 {
    1. - (1. - t.clamp(0., 1.)).powi(3)
}
