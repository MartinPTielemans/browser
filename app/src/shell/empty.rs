//! The card of a window that shows no page, which is a fresh ⌘N window, a Space with no tab shown
//! and what's left after the last Today tab closes (docs/ux-spec.md › Window › Card, no page).
//! Nothing native covers it, since the core lays out no page view with no tab shown
//! (`Window::ShowNothing`), so GPUI draws ilo's mark and one quiet line on it and takes its clicks.

use std::time::Instant;

use gpui::{Context, MouseButton, Pixels, Point, Stateful, Window, div, prelude::*, px, rgba};

use super::{Command, SECONDARY, Shell, fade_at, keys::user_keys};

/// The mark and the line fade in over this long on the scroll edge's curve, as the hover card's
/// opacity does, from nothing, so a window that shows a page a frame later never shows them.
pub(super) const FADE: f32 = 0.1;
use crate::tokens::shell::empty::WORDS_SIZE;
use crate::tokens::shell::empty::KEYS_SIZE;
use crate::tokens::shell::empty::KEYS_HEIGHT;
use crate::tokens::shell::empty::KEYS_INSET;
use crate::tokens::shell::empty::KEYS_RADIUS;
use crate::tokens::shell::empty::KEYS_FILL;
use crate::tokens::shell::empty::KEYS_GAP;
/// ilo's mark, the app icon's lowercase i (branding/ilo.icon) drawn from its 1024 pt canvas at a
/// sixteenth, so 39 tall: a 184.4 dot 81.9 over a 133.1 × 358.4 stem with round ends.
const MARK_SCALE: f32 = 1. / 16.;
const MARK_DOT: f32 = 184.4 * MARK_SCALE;
const MARK_DOT_GAP: f32 = 81.9 * MARK_SCALE;
const MARK_STEM_WIDTH: f32 = 133.1 * MARK_SCALE;
const MARK_STEM_HEIGHT: f32 = 358.4 * MARK_SCALE;
use crate::tokens::shell::empty::MARK_DOT_FILL;
use crate::tokens::shell::empty::MARK_STEM_FILL;
use crate::tokens::shell::empty::MARK_GAP;

impl Shell {
    /// Starts the fade as the card comes to show no page, and ends it as a page shows.
    pub(super) fn step_empty(&mut self, window: &mut Window) {
        if self.selected != 0 {
            self.empty_since = None;
            self.empty_settled = false;
            return;
        }
        let since = *self.empty_since.get_or_insert_with(Instant::now);
        if since.elapsed().as_secs_f32() < FADE {
            window.request_animation_frame();
        } else if !std::mem::replace(&mut self.empty_settled, true) {
            // check.sh waits for this line before it captures the empty card.
            log!("browser: empty card shown");
        }
    }

    /// How much of the line shows `now`: none with a page shown, else along its fade.
    pub(super) fn empty_opacity(&self, now: Instant) -> f32 {
        let Some(since) = self.empty_since.filter(|_| self.selected == 0) else {
            return 0.;
        };
        fade_at(0., 1., now.saturating_duration_since(since).as_secs_f32(), FADE)
    }

    /// Whether `at` is on the card while it shows no page.
    pub(super) fn empty_card_hit(&self, at: Point<Pixels>) -> bool {
        self.selected == 0 && self.page_frame(self.viewport).0.contains(&at)
    }

    /// ilo's mark over the card's one line, the next step and its key, centred, while no page is
    /// shown. A click anywhere on the card does what that key does: ⌘T's command bar, centred on
    /// the window.
    pub(super) fn empty_card(&self, cx: &mut Context<Self>) -> Option<Stateful<gpui::Div>> {
        if self.selected != 0 {
            return None;
        }
        let (frame, _) = self.page_frame(self.viewport);
        let keys = user_keys(cx).label(Command::NewTab).map(|keys| {
            div()
                .h(px(KEYS_HEIGHT))
                .px(px(KEYS_INSET))
                .flex()
                .items_center()
                .rounded(px(KEYS_RADIUS))
                .bg(rgba(KEYS_FILL))
                .text_size(px(KEYS_SIZE))
                .child(keys)
        });
        let line = div()
            .flex()
            .items_center()
            .gap(px(KEYS_GAP))
            .text_color(rgba(SECONDARY))
            .child(div().text_size(px(WORDS_SIZE)).child("New Tab"))
            .children(keys);
        let mark = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(MARK_DOT_GAP))
            .child(div().size(px(MARK_DOT)).rounded_full().bg(rgba(MARK_DOT_FILL)))
            .child(
                div()
                    .w(px(MARK_STEM_WIDTH))
                    .h(px(MARK_STEM_HEIGHT))
                    .rounded_full()
                    .bg(rgba(MARK_STEM_FILL)),
            );
        let sign = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(MARK_GAP))
            .opacity(self.empty_opacity(Instant::now()))
            .child(mark)
            .child(line);
        Some(
            div()
                .id("empty-card")
                .absolute()
                .left(frame.left())
                .top(frame.top())
                .w(frame.size.width)
                .h(frame.size.height)
                .flex()
                .items_center()
                .justify_center()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|shell, _, _, cx| shell.pressed_bar_target(cx)),
                )
                // On release, so a press that drags the window by its background isn't a click.
                .on_click(cx.listener(|shell, _, _, cx| {
                    shell.clicked_bar_target(Command::NewTab, "card", cx)
                }))
                .child(sign),
        )
    }
}

#[cfg(test)]
mod tests;
