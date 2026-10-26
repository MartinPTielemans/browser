//! When the tooltip shows and goes, on a host that records what it asks of its window.

use std::cell::RefCell;

use gpui::{TestAppContext, WindowTextSystem, point, size};

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

fn hover_on(cx: &mut TestAppContext, text: &str, anchor: Anchor) {
    cx.update(|cx| hover(text.to_string().into(), anchor, cx));
    cx.run_until_parked();
}

fn width(cx: &mut TestAppContext, text: &str) -> f32 {
    let system = cx.update(|cx| WindowTextSystem::new(cx.text_system().clone()));
    width_of(&text.to_string().into(), &system)
}

/// Where a tooltip saying `text` goes for `anchor`, as the host is told.
fn placed(cx: &mut TestAppContext, text: &str, anchor: Anchor) -> String {
    let width = width(cx, text);
    let (left, top) = place(anchor, (width, HEIGHT));
    format!("show {left} {top}")
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

fn window() -> Size<Pixels> {
    size(px(1000.), px(700.))
}

/// A frame at (`left`, `top`) in the window, `width` by `height`.
fn frame(left: f32, top: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(left), px(top)), size(px(width), px(height)))
}

/// Two favorites' tiles side by side, and a folder's plate under them.
fn tile_a() -> Anchor {
    Anchor::centred(frame(108., 100., 80., 41.), window())
}

fn tile_b() -> Anchor {
    Anchor::centred(frame(194., 100., 80., 41.), window())
}

fn folder_row() -> Anchor {
    Anchor::titled(frame(8., 300., 220., 36.), 35., window())
}

/// A tooltip 100 wide and `HEIGHT` tall.
const SIZE: (f32, f32) = (100., HEIGHT);

#[test]
fn a_name_is_the_text_and_no_name_is_none() {
    assert_eq!(text_for(" Docs ").unwrap(), "Docs");
    assert_eq!(text_for("Untitled").unwrap(), "Untitled");
    assert!(text_for("").is_none());
    assert!(text_for("  ").is_none());
}

#[gpui::test]
fn it_shows_after_the_delay_under_its_tile(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", tile_a());
    assert!(waiting(cx));
    assert!(after(cx, &host, DELAY - Duration::from_millis(1)).is_empty());
    let shown = after(cx, &host, Duration::from_millis(1));
    assert_eq!(shown, [placed(cx, "Docs", tile_a())]);
}

#[gpui::test]
fn leaving_before_the_delay_cancels_it(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", tile_a());
    after(cx, &host, DELAY / 2);
    cx.update(dismiss);
    // Nothing shows, and nothing is hidden that never showed.
    assert!(after(cx, &host, DELAY * 4).is_empty());
    assert!(!waiting(cx));
}

#[gpui::test]
fn it_goes_when_dismissed_and_leaves_no_timer(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", tile_a());
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
    hover_on(cx, "Docs", tile_a());
    after(cx, &host, DELAY);
    // The pointer leaves the first tile and crosses the gap to the next.
    cx.update(dismiss);
    assert_eq!(after(cx, &host, GRACE / 2), ["hide"]);
    hover_on(cx, "Mail", tile_b());
    assert_eq!(host.0.take(), [placed(cx, "Mail", tile_b())]);
}

#[gpui::test]
fn the_delay_starts_again_after_it_went(cx: &mut TestAppContext) {
    let host = open_tooltip(cx);
    hover_on(cx, "Docs", tile_a());
    after(cx, &host, DELAY);
    cx.update(dismiss);
    assert_eq!(after(cx, &host, GRACE), ["hide"]);
    hover_on(cx, "Mail", tile_b());
    assert!(after(cx, &host, DELAY / 2).is_empty());
    assert_eq!(after(cx, &host, DELAY / 2).len(), 1);
}

#[test]
fn a_tooltip_is_centred_under_its_tile() {
    // The tile spans 108 to 188, so its middle is 148 whichever point of it the pointer is on.
    let (left, top) = place(tile_a(), SIZE);
    assert_eq!((left, top), (148. - 50., 141. + GAP));
    // Its middle is the tile's, for a name of any width.
    let (left, _) = place(tile_a(), (60., HEIGHT));
    assert_eq!(left + 30., 148.);
}

#[test]
fn a_folders_tooltip_has_its_text_over_the_title() {
    // The title starts 35 in from the plate's left, and the text 7.5 in from the tooltip's.
    let (left, top) = place(folder_row(), SIZE);
    assert_eq!(left + PADDING + BORDER, 8. + 35.);
    assert_eq!(top, 336. + GAP);
}

#[test]
fn a_tooltip_stays_inside_the_window_across() {
    // A tile at the right edge: the tooltip ends at the window's edge instead of its middle.
    let right = Anchor::centred(frame(930., 100., 70., 41.), window());
    assert_eq!(place(right, SIZE).0, 900.);
    // And one whose middle is nearer the left than half its width.
    let left = Anchor::centred(frame(0., 100., 40., 41.), window());
    assert_eq!(place(left, SIZE).0, 0.);
    // A folder's text near the right edge.
    let folder = Anchor::titled(frame(900., 300., 100., 36.), 35., window());
    assert_eq!(place(folder, SIZE).0, 900.);
}

#[test]
fn a_tooltip_goes_above_when_there_is_no_room_below() {
    // 41 tall at the window's bottom: 700 - 22 would be the lowest top.
    let bottom = Anchor::centred(frame(8., 650., 80., 41.), window());
    let (_, top) = place(bottom, SIZE);
    assert_eq!(top, 650. - GAP - HEIGHT);
    // It fits below with exactly no room to spare.
    let snug = Anchor::centred(frame(8., 600., 80., 41.), window());
    assert_eq!(place(snug, SIZE).1, 641. + GAP);
    // It never goes past the window's top.
    let short = Anchor::centred(frame(8., 10., 80., 41.), size(px(1000.), px(60.)));
    assert_eq!(place(short, SIZE).1, 0.);
}

#[test]
fn a_frame_gives_the_anchor_its_middle_and_edges() {
    let anchor = Anchor::centred(frame(108., 100., 80., 41.), window());
    assert_eq!((anchor.x, anchor.top, anchor.bottom), (148., 100., 141.));
    assert_eq!(anchor.window, (1000., 700.));
}

#[gpui::test]
fn a_long_name_is_as_wide_as_its_window(cx: &mut TestAppContext) {
    assert_eq!(width(cx, &"x".repeat(400)), WINDOW_WIDTH);
    assert!(width(cx, "Docs") < 100.);
}
