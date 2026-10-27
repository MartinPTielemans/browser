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

    fn take_clicks(&self, takes: bool) {
        self.0.borrow_mut().push(format!("clicks {takes}"));
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

/// Capture's toast has buttons, so the clicks go to it rather than the page under it, until it
/// leaves; a toast without them lets them through again.
#[gpui::test]
fn a_toast_with_buttons_takes_clicks_while_it_shows(cx: &mut TestAppContext) {
    let host = open_toast(cx);
    let save = Button { label: Label::Text("Save".into()), name: "save", run: Rc::new(|_, _| {}) };
    cx.update(|cx| show_with_buttons("Screenshot Copied to Clipboard", vec![save], TIME, cx));
    cx.run_until_parked();
    assert_eq!(host.0.take(), ["clicks true", "show"]);
    assert_eq!(after(cx, &host, 5.), ["clicks false"]);
    assert_eq!(after(cx, &host, LEAVE), ["hide"]);
    show_deleted(cx);
    assert_eq!(host.0.take(), ["show"]);
}

/// VoiceOver says a toast as it comes, wherever its cursor is, as a polite live region.
#[gpui::test]
fn voiceover_says_the_toast_as_it_comes(cx: &mut TestAppContext) {
    use gpui::{VisualTestContext, accesskit::Live};
    let host = Rc::new(Recorder::default());
    let window = cx.update(|cx| open(host.clone(), |_| hsla(0., 0., 0., 1.), cx));
    let mut window = VisualTestContext::from_window(window.expect("no toast window").into(), cx);
    window.simulate_a11y_activation();
    show_deleted(cx);
    let tree = window.a11y_tree().expect("GPUI built a tree");
    let (_, toast) = tree.nodes.iter().find(|(_, node)| node.role() == Role::Status).unwrap();
    assert_eq!(toast.live(), Some(Live::Polite));
    assert_eq!(toast.value(), Some("Space deleted! Use ⌘Z to undo."));
}

/// The Undo a toast offers goes with the undo, however it was done, but another toast stays.
#[gpui::test]
fn an_undo_toast_goes_once_the_undo_is_done_some_other_way(cx: &mut TestAppContext) {
    let host = open_toast(cx);
    cx.update(|cx| show_undo("Removed from Favorites", Rc::new(|_| {}), TIME, cx));
    cx.run_until_parked();
    assert_eq!(host.0.take(), ["clicks true", "show"]);
    cx.update(|cx| hide_undo(cx));
    cx.run_until_parked();
    assert_eq!(host.0.take(), ["clicks false"]);
    assert_eq!(after(cx, &host, LEAVE), ["hide"]);

    show_deleted(cx);
    host.0.take();
    cx.update(|cx| hide_undo(cx));
    assert!(after(cx, &host, 4.9).is_empty(), "Space deleted! stays its five seconds");
}

#[gpui::test]
fn an_undo_toast_runs_its_undo_with_a_click_on_the_button(cx: &mut TestAppContext) {
    let host = open_toast(cx);
    let undone = Rc::new(std::cell::Cell::new(0));
    let counted = undone.clone();
    let undo = Rc::new(move |_: &mut App| counted.set(counted.get() + 1));
    cx.update(|cx| show_undo("Removed from Favorites", undo, TIME, cx));
    cx.run_until_parked();
    host.0.take();
    let run = cx.update(|cx| {
        let window = cx.global::<ToastWindow>().0;
        window.read(cx).unwrap().shown.as_ref().unwrap().buttons[0].run.clone()
    });
    cx.update(|cx| run(point(px(0.), px(0.)), cx));
    assert_eq!(undone.get(), 1);
}
