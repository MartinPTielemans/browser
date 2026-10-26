//! When the tooltip shows and goes, on a host that records what it asks of its window.

use std::cell::RefCell;

use gpui::TestAppContext;

use super::*;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Host for Recorder {
    fn show(&self, left: f32, top: f32) {
        self.0.borrow_mut().push(format!("show {left} {top}"));
    }

    fn hide(&self) {
        self.0.borrow_mut().push("hide".into());
    }
}

fn open_tooltip(cx: &mut TestAppContext) -> Rc<Recorder> {
    let host = Rc::new(Recorder::default());
    cx.update(|cx| open(host.clone(), cx)).expect("no tooltip window");
    host
}

fn hover_on(cx: &mut TestAppContext, text: &str, at: (f32, f32)) {
    cx.update(|cx| hover(text.to_string().into(), at, cx));
    cx.run_until_parked();
}

fn after(cx: &mut TestAppContext, host: &Recorder, time: Duration) -> Vec<String> {
    cx.executor().advance_clock(time);
    cx.run_until_parked();
    host.0.take()
}

/// Whether the delay's timer is running.
fn waiting(cx: &mut TestAppContext) -> bool {
    let window = cx.update(|cx| cx.global::<TooltipWindow>().0);
    cx.update(|cx| window.update(cx, |tooltip, _, _| tooltip.wait.is_some()).unwrap())
}

const A: (f32, f32) = (100., 110.);
const B: (f32, f32) = (190., 110.);

#[test]
fn a_name_is_the_text_and_no_name_is_none() {
    assert_eq!(text_for(" Docs ").unwrap(), "Docs");
    assert_eq!(text_for("Untitled").unwrap(), "Untitled");
    assert!(text_for("").is_none());
    assert!(text_for("  ").is_none());
}

#[gpui::test]
fn it_shows_after_the_delay_below_the_pointer(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", A);
    assert!(waiting(cx));
    assert!(after(cx, &host, DELAY - Duration::from_millis(1)).is_empty());
    let shown = after(cx, &host, Duration::from_millis(1));
    assert_eq!(shown, [format!("show {} {}", A.0, A.1 + BELOW)]);
}

#[gpui::test]
fn leaving_before_the_delay_cancels_it(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", A);
    after(cx, &host, DELAY / 2);
    cx.update(dismiss);
    // Nothing shows, and nothing is hidden that never showed.
    assert!(after(cx, &host, DELAY * 4).is_empty());
    assert!(!waiting(cx));
}

#[gpui::test]
fn it_goes_when_dismissed_and_leaves_no_timer(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", A);
    assert_eq!(after(cx, &host, DELAY).len(), 1);
    cx.update(dismiss);
    assert_eq!(host.0.take(), ["hide"]);
    // A second dismissal has nothing to hide.
    cx.update(dismiss);
    assert!(host.0.take().is_empty());
    assert!(!waiting(cx));
}

#[gpui::test]
fn the_next_tile_shows_at_once(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", A);
    after(cx, &host, DELAY);
    // The pointer leaves the first tile and crosses the gap to the next.
    cx.update(dismiss);
    assert_eq!(after(cx, &host, GRACE / 2), ["hide"]);
    hover_on(cx, "Mail", B);
    assert_eq!(host.0.take(), [format!("show {} {}", B.0, B.1 + BELOW)]);
}

#[gpui::test]
fn the_delay_starts_again_after_it_went(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", A);
    after(cx, &host, DELAY);
    cx.update(dismiss);
    assert_eq!(after(cx, &host, GRACE), ["hide"]);
    hover_on(cx, "Mail", B);
    assert!(after(cx, &host, DELAY / 2).is_empty());
    assert_eq!(after(cx, &host, DELAY / 2).len(), 1);
}
