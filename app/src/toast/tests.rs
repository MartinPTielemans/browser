//! How long the toast stays, on a host that records what the toast asks of its window.

use std::cell::RefCell;

use gpui::{TestAppContext, hsla};

use super::*;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Host for Recorder {
    fn show(&self) {
        self.0.borrow_mut().push("show".into());
    }

    fn hide(&self) {
        self.0.borrow_mut().push("hide".into());
    }
}

const TIME: Duration = Duration::from_secs(5);

fn open_toast(cx: &mut TestAppContext) -> Rc<Recorder> {
    let host = Rc::new(Recorder::default());
    cx.update(|cx| open(host.clone(), |_| hsla(0., 0., 0., 1.), cx)).expect("no toast window");
    host
}

fn show_deleted(cx: &mut TestAppContext) {
    cx.update(|cx| show("Space deleted! Use", Some(("⌘Z", "to undo.")), TIME, cx));
    cx.run_until_parked();
}

/// Moves the clock on by `seconds` and returns what the toast asked of the host meanwhile.
fn after(cx: &mut TestAppContext, host: &Recorder, seconds: f32) -> Vec<String> {
    cx.executor().advance_clock(Duration::from_secs_f32(seconds));
    cx.run_until_parked();
    host.0.take()
}

#[gpui::test]
fn it_leaves_once_its_time_is_up(cx: &mut TestAppContext) {
    let host = open_toast(cx);
    show_deleted(cx);
    assert_eq!(host.0.take(), ["show"]);
    assert!(after(cx, &host, 4.9).is_empty());
    // It hides once it has shrunk and faded away.
    assert!(after(cx, &host, 0.1).is_empty());
    assert_eq!(after(cx, &host, LEAVE), ["hide"]);
}

#[gpui::test]
fn another_toast_starts_the_time_again(cx: &mut TestAppContext) {
    let host = open_toast(cx);
    show_deleted(cx);
    after(cx, &host, 4.);
    show_deleted(cx);
    assert_eq!(host.0.take(), ["show"]);
    // The first would have left at 5 s.
    assert!(after(cx, &host, 4.).is_empty());
    assert_eq!(after(cx, &host, 1. + LEAVE), ["hide"]);
}

#[gpui::test]
fn it_overshoots_its_size_as_arcs_does(_: &mut TestAppContext) {
    let peak = (0..60).map(|frame| Pose::arriving(frame as f32 / 120.).scale).fold(0., f32::max);
    assert!((1.03..1.05).contains(&peak), "peak {peak}");
    assert_eq!(Pose::arriving(0.), Pose { scale: FROM, opacity: 0. });
    assert_eq!(Pose::arriving(1.), Pose { scale: 1., opacity: 1. });
    let gone = Pose::arriving(1.).leaving(LEAVE);
    assert!((gone.scale - FROM).abs() < 1e-3 && gone.opacity < 1e-3, "{gone:?}");
}
