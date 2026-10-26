//! The sidebar's width, which its edge drags; hiding and showing it, and while it's
//! hidden, its reveal over the page (docs/ux-spec.md › Sidebar reveal); and its
//! gradient in the Space's colour.

use std::time::Instant;

use gpui::{
    Context, CursorStyle, DispatchPhase, Hsla, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Rgba, canvas, div, linear_color_stop, linear_gradient, prelude::*, px, rgb,
};

use super::{
    CARD_INSET, GRADIENT_ALPHA, MATERIAL_STAND_IN, NEUTRAL, NEUTRAL_GRADIENT_ALPHA, REVEAL_HOLD,
    REVEAL_INSET, REVEAL_IN_DAMPING, REVEAL_IN_RESPONSE, REVEAL_OUT_DAMPING, REVEAL_OUT_RESPONSE,
    REVEAL_OVERSHOOT, REVEAL_WAIT, SIDEBAR_EDGE, SIDEBAR_HIDE, SIDEBAR_MAX, SIDEBAR_MIN,
    SIDEBAR_SHOW, SIDEBAR_WIDTH, Shell, rubber_band, settle_progress, spring,
};

impl Shell {
    /// The docked sidebar's width as drawn: with the pointer while its edge is dragged, then on its
    /// way to where it rests.
    pub(super) fn width(&self) -> f32 {
        if let Some((_, way)) = self.resize {
            return rubber_band(way, SIDEBAR_MIN, SIDEBAR_MAX).round();
        }
        let Some((from, start)) = self.width_motion else {
            return self.sidebar_width;
        };
        settle_progress(start)
            .map_or(self.sidebar_width, |progress| from + (self.sidebar_width - from) * progress)
    }

    /// The docked sidebar's right edge, which drags its width and puts it back on a double-click.
    pub(super) fn sidebar_edge(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let width = self.width();
        let cursor = match width {
            width if width <= SIDEBAR_MIN => CursorStyle::ResizeRight,
            width if width >= SIDEBAR_MAX => CursorStyle::ResizeLeft,
            _ => CursorStyle::ResizeLeftRight,
        };
        let resizing = self.resize.is_some();
        let shell = cx.weak_entity();
        div()
            .id("sidebar-edge")
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(width - SIDEBAR_EDGE))
            .w(px(SIDEBAR_EDGE))
            .cursor(cursor)
            .block_mouse_except_scroll()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseDownEvent, _, cx| shell.press_edge(event, cx)),
            )
            // The pointer's moves and release anywhere in the window, while the edge is dragged.
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        if resizing {
                            window.set_window_cursor_style(cursor);
                        }
                        let moved = shell.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture {
                                let x = event.position.x.into();
                                moved.update(cx, |shell, cx| shell.drag_edge(x, cx)).ok();
                            }
                        });
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                            if phase == DispatchPhase::Capture && event.button == MouseButton::Left
                            {
                                shell.update(cx, |shell, cx| shell.let_go_edge(cx)).ok();
                            }
                        });
                    },
                )
                .absolute()
                .size_0(),
            )
    }

    fn press_edge(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let width = self.width();
        if event.click_count == 2 {
            self.resize = None;
            self.set_width(width, SIDEBAR_WIDTH, cx);
            return;
        }
        self.width_motion = None;
        self.resize = Some((f32::from(event.position.x) - width, width));
        cx.notify();
    }

    fn drag_edge(&mut self, x: f32, cx: &mut Context<Self>) {
        if let Some((grab, _)) = self.resize {
            self.resize = Some((grab, x - grab));
            cx.notify();
        }
    }

    fn let_go_edge(&mut self, cx: &mut Context<Self>) {
        if let Some((_, way)) = self.resize.take() {
            let from = rubber_band(way, SIDEBAR_MIN, SIDEBAR_MAX).round();
            self.set_width(from, way.clamp(SIDEBAR_MIN, SIDEBAR_MAX), cx);
        }
    }

    /// Settles the docked sidebar at `to` wide from `from`, and keeps the width for the next
    /// launch.
    fn set_width(&mut self, from: f32, to: f32, cx: &mut Context<Self>) {
        let to = to.round();
        if to != self.sidebar_width {
            self.sidebar_width = to;
            self.browser.set_sidebar_width(to, cx);
        }
        self.width_motion = (from != to).then(|| (from, Instant::now()));
        if self.width_motion.is_none() {
            // check.sh waits for these lines.
            log!("browser: sidebar width {to}");
        }
        cx.notify();
    }

    /// How far the sidebar has moved since `start`, or None once it's there.
    pub(super) fn sidebar_progress(&self, start: Instant) -> Option<f32> {
        let t = start.elapsed().as_secs_f32();
        let time = if self.sidebar_hidden { SIDEBAR_HIDE } else { SIDEBAR_SHOW };
        (t < time).then(|| 1. - (1. - t / time).powi(3))
    }

    /// Hides the docked sidebar, or shows it, from wherever it is.
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        // A full-screen page has no sidebar to hide.
        if self.page_fullscreen {
            return;
        }
        // Ours: the sidebar comes back from the Library as it was.
        if self.library_open {
            return self.set_library(false, cx);
        }
        if self.dock(cx) {
            return;
        }
        let from = self.card_left();
        self.sidebar_hidden = !self.sidebar_hidden;
        self.sidebar_motion = Some((from, Instant::now()));
        cx.notify();
    }

    /// Whether the window's own buttons show: over the docked sidebar at rest and over the open
    /// Library, as in Arc. The sidebar hides them as it goes and they come back once it's in, and
    /// while the Library moves the sidebar draws them, receding with it.
    pub(super) fn window_buttons_shown(&self) -> bool {
        if self.library_moving() {
            return false;
        }
        self.library_open || (!self.sidebar_hidden && self.sidebar_motion.is_none())
    }

    /// Docks the sidebar the reveal has out, in one frame (Arc), and returns whether it was out.
    pub(super) fn dock(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.reveal_shown() {
            return false;
        }
        self.conceal_now(cx);
        self.sidebar_hidden = false;
        self.sidebar_motion = None;
        // check.sh waits for this line.
        log!("browser: sidebar shown");
        cx.notify();
        true
    }

    /// Whether the reveal's panel is on screen, moving or not.
    pub fn reveal_shown(&self) -> bool {
        self.revealed || self.reveal_motion.is_some()
    }

    /// How far out the reveal's panel is while it moves: 0 off the window, 1 at rest over the page
    /// and past 1 as it overshoots. None once it's at rest.
    pub(super) fn reveal_step(&self) -> Option<f32> {
        let (from, start) = self.reveal_motion?;
        let t = start.elapsed().as_secs_f32();
        if self.revealed {
            spring(t, REVEAL_IN_RESPONSE, REVEAL_IN_DAMPING).map(|eased| from + (1. - from) * eased)
        } else {
            spring(t, REVEAL_OUT_RESPONSE, REVEAL_OUT_DAMPING).map(|eased| from * (1. - eased))
        }
    }

    pub(super) fn reveal_out(&self) -> f32 {
        self.reveal_step().unwrap_or(if self.revealed { 1. } else { 0. })
    }

    /// The system's accent colour changed, which the window's gradient follows.
    pub fn accent_color_changed(&mut self, cx: &mut Context<Self>) {
        self.accent_color = self.browser.accent_color();
        cx.notify();
    }

    /// The gradient down the window, at `y` from its top (0) to its bottom (1): the Space's
    /// colour, going to the same at HSL lightness + 0.25. While the sidebar pages between two
    /// Spaces, it cross-fades from one's to the other's as the pages move.
    fn gradient_color(&self, y: f32) -> Hsla {
        let color_of = |space| {
            // A Space without a colour has the system's accent colour, as in Dia.
            let space = self.spaces.iter().find(|each| each.id == space);
            let color = space.and_then(|space| space.color).unwrap_or(self.accent_color);
            let color = Hsla::from(rgb(color));
            let alpha = if color.s < NEUTRAL { NEUTRAL_GRADIENT_ALPHA } else { GRADIENT_ALPHA };
            Rgba::from(Hsla { l: (color.l + 0.25 * y).min(1.), ..color }.opacity(alpha))
        };
        let pages = self.pages();
        let (space, x) = pages[0];
        let from = color_of(space);
        let Some(&(beside, _)) = pages.get(1) else {
            return from.into();
        };
        let to = color_of(beside);
        let t = (x.abs() / self.width()).min(1.);
        let mix = |from: f32, to: f32| from + (to - from) * t;
        let (r, g, b) = (mix(from.r, to.r), mix(from.g, to.g), mix(from.b, to.b));
        Rgba { r, g, b, a: mix(from.a, to.a) }.into()
    }

    /// The gradient as a background, over `base` where the window's look has to be opaque.
    pub(super) fn gradient(&self, base: Option<Hsla>) -> gpui::Background {
        let stop = |y| {
            let color = self.gradient_color(y);
            linear_color_stop(base.map_or(color, |base| base.blend(color)), y)
        };
        linear_gradient(180., stop(0.), stop(1.))
    }

    /// The window's look, opaque, halfway down: what covers the rows it passes over, and the find
    /// bar's colour.
    pub fn window_fill(&self) -> Hsla {
        Hsla::from(rgb(MATERIAL_STAND_IN)).blend(self.gradient_color(0.5))
    }

    /// The pointer is at `x` in the browser window: kept at its left edge it brings the hidden
    /// sidebar out over the page, and more than the hold zone past the panel's edge it sends it
    /// away.
    pub fn pointer_at(&mut self, x: f32, cx: &mut Context<Self>) {
        if !self.can_reveal() {
            return;
        }
        // The edge is the card's margin. Left of the window, the pointer has gone to another
        // window or display.
        if !(0. ..CARD_INSET).contains(&x) {
            self.reveal_wait = None;
            // The page's question keeps it out until it's answered.
            if x > REVEAL_INSET + self.width() + REVEAL_HOLD && self.question().is_none() {
                self.set_revealed(false, cx);
            }
        } else if !self.revealed && self.reveal_wait.is_none() {
            // Arc waits a moment, so a pointer passing the edge on its way to another window or
            // display leaves the sidebar where it is.
            self.reveal_wait = Some(cx.spawn(async move |shell, cx| {
                cx.background_executor().timer(REVEAL_WAIT).await;
                shell
                    .update(cx, |shell, cx| {
                        shell.reveal_wait = None;
                        if shell.can_reveal() {
                            shell.set_revealed(true, cx);
                        }
                    })
                    .ok();
            }));
        }
    }

    /// Whether the sidebar is hidden and still, so the reveal can bring it out.
    pub(super) fn can_reveal(&self) -> bool {
        let still = self.sidebar_motion.is_none() && self.library.is_none();
        self.sidebar_hidden && still && !self.page_fullscreen
    }

    /// Starts the reveal's panel moving out over the page, or away, from wherever it is.
    pub(super) fn set_revealed(&mut self, revealed: bool, cx: &mut Context<Self>) {
        if revealed == self.revealed {
            return;
        }
        self.reveal_motion = Some((self.reveal_out(), Instant::now()));
        self.revealed = revealed;
        if revealed {
            self.browser.show_reveal(REVEAL_INSET + self.width() + REVEAL_OVERSHOOT, cx);
        }
        cx.notify();
    }

    /// Takes the reveal's panel off screen at once.
    pub(super) fn conceal_now(&mut self, cx: &mut Context<Self>) {
        if self.reveal_shown() {
            self.revealed = false;
            self.reveal_motion = None;
            self.browser.hide_reveal(cx);
        }
    }

    pub(super) fn sidebar_hover(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if hovered != self.sidebar_hovered {
            self.sidebar_hovered = hovered;
            self.hover_changed = true;
            cx.notify();
        }
    }
}
