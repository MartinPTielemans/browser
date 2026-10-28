//! A browser's wheel events, cut into gestures as AppKit's phases cut a trackpad's: the site's
//! demo learns from them where fingers land and lift, which no browser says. After the fingers
//! lift, AppKit's momentum carries on in deltas that fall away smoothly and steadily, each a
//! little under the last, where the fingers' own wobble from event to event. So the fingers lift
//! where the deltas start falling that smoothly, and land again where they rise out of the fall,
//! turn back, or come after a pause. Speeds come from the events' own times, as a browser busy
//! drawing hands on several summed into one, late.

use std::collections::VecDeque;
use std::time::Duration;
use web_time::Instant;

/// Longer than a trackpad's 8 to 16 ms between events, and shorter than a hand takes to swipe
/// again.
pub(super) const GAP: Duration = Duration::from_millis(60);
/// What a gesture's speed is taken over: two frames at 60 Hz, so summed events even out.
const WINDOW: Duration = Duration::from_millis(34);
/// Falls in a row, each to between FALL and all of the speed before, and for at least FALLING,
/// that read as momentum. The falls may differ by no more than STEADY, which a hand's wobble does.
const FALLS: usize = 4;
const FALL: f32 = 0.75;
const FALLING: Duration = Duration::from_millis(25);
const STEADY: f32 = 0.02;
/// Whole points round away the steady falls under this many, so the falls are taken over HOLD
/// instead: the speed not rising once, falling, but no faster than e^(−t/GENTLE), as no momentum
/// does and a hand stopping or turning does, and by now
/// at most SLOWED of the gesture's fastest, over at least HELD events. A hand's wobble rises
/// within that many.
const WHOLE: f32 = 8.;
const HOLD: Duration = Duration::from_millis(50);
const HELD: usize = 6;
const GENTLE: Duration = Duration::from_millis(150);
const SLOWED: f32 = 0.9;
/// Momentum falls as much in each half of HOLD, where a hand braking for a turn falls faster and
/// faster: neither half may fall more than CURVE times the other, give or take a whole point over
/// WINDOW, which blurs the speed by that much.
const CURVE: f32 = 2.;
/// Out of momentum, a speed this much over its slowest, and by this many points a millisecond, is
/// fingers again: a tail's whole points wobble a point, never by half.
const RISE: f32 = 1.5;
const RISE_SPEED: f32 = 0.05;
/// After a gap, a gesture going on at under this much of its speed paused: a busy page hands on a
/// gesture that kept moving at its speed, summed.
const PAUSED: f32 = 0.5;

#[derive(Default)]
pub(super) struct WheelGestures {
    state: State,
    /// The gesture's last events, newest last.
    recent: VecDeque<Wheel>,
    /// The fingers' speed at each of their last events, over HOLD, over WINDOW and since the event
    /// before, and the fastest they went.
    speeds: VecDeque<(Instant, f32, f32)>,
    peak: f32,
}

#[derive(Clone, Copy)]
struct Wheel {
    at: Instant,
    /// Points across or along, whichever is larger, with its sign.
    delta: f32,
    /// Since the event before, and its speed over that, in points a millisecond.
    since: Duration,
    speed: f32,
}

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    Idle,
    Fingers,
    /// The momentum's direction and its slowest speed yet.
    Momentum { sign: f32, slowest: f32 },
}

/// What a wheel event says of the fingers, ahead of its own delta, which moves them or, after
/// they've lifted, is momentum.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Cut {
    pub lifted: bool,
    pub landed: bool,
}

impl WheelGestures {
    /// A wheel event that came at `at`, whose larger delta is `delta` points.
    pub(super) fn event(&mut self, delta: f32, at: Instant) -> Cut {
        let last = self.recent.back().copied();
        // The first event of a gesture is as long as a trackpad's between events.
        let since = last.map_or(Duration::from_millis(8), |last| at.duration_since(last.at));
        let speed = delta / (since.as_secs_f32() * 1000.).max(1.);
        let before = self.speed();
        let paused = last.is_none()
            || since > GAP && speed.abs() < PAUSED * before.abs();
        if paused {
            self.recent.clear();
            self.speeds.clear();
            self.peak = 0.;
        }
        self.recent.push_back(Wheel { at, delta, since, speed });
        while self.recent.len() > FALLS + 1
            && self.recent.front().is_some_and(|wheel| at.duration_since(wheel.at) > WINDOW)
        {
            self.recent.pop_front();
        }
        let now = self.speed();
        // A turn starts the fingers' speeds over.
        if self.speeds.back().is_some_and(|&(_, speed, _)| speed.signum() != now.signum()) {
            self.speeds.clear();
            self.peak = 0.;
        }
        self.speeds.push_back((at, now, speed));
        self.peak = self.peak.max(now.abs());
        while self.speeds.len() > HELD
            && self.speeds.get(1).is_some_and(|&(when, _, _)| at.duration_since(when) >= HOLD)
        {
            self.speeds.pop_front();
        }
        let (state, cut) = match self.state {
            State::Fingers if paused => (State::Fingers, Cut { lifted: true, landed: true }),
            State::Idle => (State::Fingers, Cut { landed: true, ..Cut::default() }),
            State::Momentum { .. } if paused => (State::Fingers, Cut { landed: true, ..Cut::default() }),
            State::Fingers if self.falling() || self.slowing() => {
                let state = State::Momentum { sign: now.signum(), slowest: now.abs() };
                (state, Cut { lifted: true, ..Cut::default() })
            }
            State::Fingers => (State::Fingers, Cut::default()),
            State::Momentum { sign, slowest } => {
                let turned = now.signum() != sign && now.abs() >= RISE_SPEED;
                let rose = now.abs() > RISE * slowest && now.abs() - slowest >= RISE_SPEED;
                if turned || rose {
                    // The fingers' own speed is taken from here on.
                    self.recent.retain(|wheel| wheel.at == at);
                    self.speeds.clear();
                    self.peak = 0.;
                    (State::Fingers, Cut { landed: true, ..Cut::default() })
                } else {
                    (State::Momentum { sign, slowest: slowest.min(now.abs()) }, Cut::default())
                }
            }
        };
        self.state = state;
        cut
    }

    /// The gesture's speed over the last WINDOW, in points a millisecond with its sign.
    fn speed(&self) -> f32 {
        let Some(newest) = self.recent.back() else { return 0. };
        let window = self.recent.iter().filter(|wheel| newest.at.duration_since(wheel.at) < WINDOW);
        let (way, time) = window.fold((0., 0.), |(way, time), wheel| {
            (way + wheel.delta, time + wheel.since.as_secs_f32() * 1000.)
        });
        way / time.max(1.)
    }

    /// Whether the last events fell the way momentum does: each a steady part of the one before,
    /// in one direction.
    fn falling(&self) -> bool {
        if self.recent.len() < FALLS + 1 || self.recent.iter().any(|wheel| wheel.speed == 0.) {
            return false;
        }
        let last = self.recent.iter().rev().take(FALLS + 1).collect::<Vec<_>>();
        if last.iter().any(|wheel| wheel.delta.fract() == 0. && wheel.delta.abs() < WHOLE) {
            return false;
        }
        let span = last[0].at.duration_since(last[FALLS].at);
        let ratios = last.windows(2).map(|pair| pair[0].speed / pair[1].speed);
        let (low, high) = ratios.fold((f32::MAX, f32::MIN), |(low, high), ratio| {
            (low.min(ratio), high.max(ratio))
        });
        let fell = last[0].speed.abs() < last[FALLS].speed.abs();
        span >= FALLING && low >= FALL && high <= 1. && high - low <= STEADY && fell
    }

    /// Whether the fingers' speed has fallen gently and evenly over HOLD, never rising from one
    /// event to the next, as momentum's does in whole points.
    fn slowing(&self) -> bool {
        let (Some(&(then_at, then, _)), Some(&(now_at, now, _))) =
            (self.speeds.front(), self.speeds.back())
        else {
            return false;
        };
        let held = now_at.duration_since(then_at) >= HOLD && self.speeds.len() >= HELD;
        let pairs = self.speeds.iter().zip(self.speeds.iter().skip(1));
        let never_rose = pairs.clone().all(|(&(_, _, a), &(_, _, b))| b.abs() <= a.abs() * 1.001);
        let middle = then_at + HOLD / 2;
        let mid = self.speeds.iter().find(|&&(at, _, _)| at >= middle).map_or(now, |&(_, mid, _)| mid);
        let (then, mid, now) = (then.abs(), mid.abs(), now.abs());
        let (first, second) = ((then / mid).ln(), (mid / now).ln());
        let blur = (1. + 1. / (WINDOW.as_secs_f32() * 1000. * now)).ln();
        let even = first <= CURVE * second + blur && second <= CURVE * first + blur;
        let gentle = (-now_at.duration_since(then_at).as_secs_f32() / GENTLE.as_secs_f32()).exp();
        held && never_rose && even && now < then && now >= gentle * then && now <= SLOWED * self.peak
    }

    /// No event since the last, at `now`: whether that lifts fingers still down.
    pub(super) fn quiet(&mut self, now: Instant) -> bool {
        if self.recent.back().is_some_and(|last| now.duration_since(last.at) < GAP) {
            return false;
        }
        let lifted = matches!(self.state, State::Fingers);
        self.state = State::Idle;
        self.recent.clear();
        self.speeds.clear();
        self.peak = 0.;
        lifted
    }
}
