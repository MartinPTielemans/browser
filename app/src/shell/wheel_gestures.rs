//! A browser's wheel events, cut into gestures as AppKit's phases cut a trackpad's: the site's
//! demo learns from them where fingers land and lift, which no browser says. A trackpad's
//! momentum comes as deltas that fall away for a second or more after the fingers lift, so the
//! fingers lift where the deltas start falling for good, and land again where they rise out of
//! that fall or come after a pause.

use std::time::Duration;
use web_time::Instant;

/// Longer than a trackpad's 16 ms between events, and shorter than a hand takes to swipe again.
pub(super) const GAP: Duration = Duration::from_millis(60);
/// Falls in a row that read as momentum, so a slow swipe's unsteady deltas don't lift the fingers.
const FALLS: u8 = 4;
/// How far under the gesture's largest delta a fall has to be to read as momentum.
const FALLEN: f32 = 0.7;
/// A rise out of momentum this much bigger, and by this many pixels, is fingers again: a decaying
/// tail rounds up and down a pixel, never by half.
const RISE: f32 = 1.5;
const RISE_PX: f32 = 2.;

#[derive(Default)]
pub(super) struct WheelGestures {
    state: State,
    last: Option<Instant>,
}

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    Idle,
    Fingers { peak: f32, prev: f32, falls: u8 },
    Momentum { prev: f32 },
}

/// What a wheel event says of the fingers, ahead of its own delta, which moves them or, after
/// they've lifted, is momentum.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Cut {
    pub lifted: bool,
    pub landed: bool,
}

impl WheelGestures {
    /// A wheel event at `at` whose larger delta is `size` pixels.
    pub(super) fn event(&mut self, size: f32, at: Instant) -> Cut {
        let paused = self.last.is_none_or(|last| at.duration_since(last) > GAP);
        self.last = Some(at);
        let fingers = State::Fingers { peak: size, prev: size, falls: 0 };
        let (state, cut) = match self.state {
            // A pause the quiet timer hasn't told yet lifts the fingers here.
            State::Fingers { .. } if paused => (fingers, Cut { lifted: true, landed: true }),
            State::Idle => (fingers, Cut { landed: true, ..Cut::default() }),
            State::Momentum { .. } if paused => (fingers, Cut { landed: true, ..Cut::default() }),
            State::Fingers { peak, prev, falls } => {
                let falls = if size < prev { falls + 1 } else { 0 };
                if falls >= FALLS && size < FALLEN * peak {
                    (State::Momentum { prev: size }, Cut { lifted: true, ..Cut::default() })
                } else {
                    (State::Fingers { peak: peak.max(size), prev: size, falls }, Cut::default())
                }
            }
            State::Momentum { prev } if size > RISE * prev && size - prev >= RISE_PX => {
                (fingers, Cut { landed: true, ..Cut::default() })
            }
            State::Momentum { .. } => (State::Momentum { prev: size }, Cut::default()),
        };
        self.state = state;
        cut
    }

    /// No event since the last, at `now`: whether that lifts fingers still down.
    pub(super) fn quiet(&mut self, now: Instant) -> bool {
        if self.last.is_some_and(|last| now.duration_since(last) < GAP) {
            return false;
        }
        let lifted = matches!(self.state, State::Fingers { .. });
        self.state = State::Idle;
        lifted
    }
}
