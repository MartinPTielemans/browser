//! The card of a window that shows no page, which is a fresh ⌘N window, a Space with no tab shown
//! and what's left after the last Today tab closes (docs/ux-spec.md › Window › Card, no page).
//! Nothing native covers it, since the core lays out no page view with no tab shown
//! (`Window::ShowNothing`), so GPUI draws one quiet line on it and takes its clicks.

use std::time::Instant;

use gpui::{Context, Pixels, Point, Stateful, Window, div, prelude::*, px, rgba};

use super::{Command, SECONDARY, Shell, fade_at, keys::user_keys};

/// The line fades in over this long on the scroll edge's curve, as the hover card's opacity does,
/// from nothing, so a window that shows a page a frame later never shows it.
pub(super) const FADE: f32 = 0.1;
/// The sidebar's New Tab row is 14 pt.
const WORDS_SIZE: f32 = 14.;
/// The command bar's key chip, which a command's row ends in: 24 tall, 7 in, white 0.08 on the
/// panel, with its text at 13 pt.
const KEYS_SIZE: f32 = 13.;
const KEYS_HEIGHT: f32 = 24.;
const KEYS_INSET: f32 = 7.;
const KEYS_RADIUS: f32 = 6.5;
const KEYS_FILL: u32 = 0xffff_ff14;
/// From the words to their keys, as the toast sets its chip.
const KEYS_GAP: f32 = 8.;

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

    /// The card's one line, the next step and its key, centred, while no page is shown. A click
    /// anywhere on the card does what that key does: ⌘T's command bar, centred on the window.
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
            .opacity(self.empty_opacity(Instant::now()))
            .child(div().text_size(px(WORDS_SIZE)).child("New Tab"))
            .children(keys);
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
                // On release, so a press that drags the window by its background isn't a click.
                .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::NewTab, "card", cx)))
                .child(line),
        )
    }
}

#[cfg(test)]
mod tests;
