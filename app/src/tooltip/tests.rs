//! When the tooltip shows and goes, on a host that records what it asks of its window, and where
//! it and its chips go.

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

fn tip(text: &str) -> Tip {
    Tip::named(text.to_string().into())
}

fn hover_on(cx: &mut TestAppContext, text: &str, anchor: Anchor) {
    cx.update(|cx| hover(tip(text), anchor, cx));
    cx.run_until_parked();
}

fn measured(cx: &mut TestAppContext, tip: &Tip) -> Layout {
    let system = cx.update(|cx| WindowTextSystem::new(cx.text_system().clone()));
    measure(tip, &system)
}

fn width(cx: &mut TestAppContext, text: &str) -> f32 {
    measured(cx, &tip(text)).width
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
    // The title starts 35 in from the plate's left, and the text 11 in from the tooltip's.
    let (left, top) = place(folder_row(), SIZE);
    assert_eq!(left + TEXT_X, 8. + 35.);
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
    // 41 tall at the window's bottom: 700 - 27 would be the lowest top.
    let bottom = Anchor::centred(frame(8., 650., 80., 41.), window());
    let (_, top) = place(bottom, SIZE);
    assert_eq!(top, 650. - GAP - HEIGHT);
    // It fits below with exactly no room to spare.
    let snug = Anchor::centred(frame(8., 700. - 41. - GAP - HEIGHT, 80., 41.), window());
    assert_eq!(place(snug, SIZE).1, 700. - HEIGHT);
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

/// The header's back button, and the footer's Library button in a 700 tall window.
fn back_button() -> Bounds<Pixels> {
    frame(122., 7.5, 32., 32.)
}

fn library_button() -> Bounds<Pixels> {
    frame(8., 700. - 41., 32., 32.)
}

#[test]
fn a_footer_buttons_tooltip_is_beside_it_and_centred_on_it() {
    // Arc's: 3.5 past the plate's right edge, its middle at the plate's.
    let (left, top) = place(Anchor::beside(library_button(), window()), SIZE);
    assert_eq!(left, 8. + 32. + BESIDE);
    assert_eq!(top + HEIGHT / 2., 700. - 41. + 16.);
}

#[test]
fn a_tooltip_beside_stays_inside_the_window() {
    // A button whose middle is nearer the window's bottom than half the tooltip's height.
    let low = Anchor::beside(frame(8., 690., 32., 10.), window());
    assert_eq!(place(low, SIZE).1, 700. - HEIGHT);
    // One at the right edge: the tooltip ends at the window's edge.
    let right = Anchor::beside(frame(950., 300., 32., 32.), window());
    assert_eq!(place(right, SIZE).0, 900.);
}

#[test]
fn a_header_buttons_tooltip_is_centred_under_it() {
    let (left, top) = place(Anchor::centred(back_button(), window()), SIZE);
    assert_eq!(left + 50., 122. + 16.);
    assert_eq!(top, 7.5 + 32. + GAP);
}

#[test]
fn a_name_alone_has_its_text_11_in_and_10_before_the_end() {
    let layout = layout(60., &[]);
    let alone = Layout { width: 11. + 60. + 10., text: 60., truncated: false, chips: vec![] };
    assert_eq!(layout, alone);
}

#[test]
fn chips_follow_the_text_on_arcs_pitch() {
    // View Library ⇧⌘L: 8 after the text, 18.5 wide on a pitch of 22, and 10 after the last.
    let layout = layout(70., &[CHIP, CHIP, CHIP]);
    let first = 11. + 70. + 8.;
    assert_eq!(layout.chips, [(first, 18.5), (first + 22., 18.5), (first + 44., 18.5)]);
    assert_eq!(layout.width, first + 44. + 18.5 + 10.);
    assert!(!layout.truncated);
}

#[test]
fn a_wider_chip_moves_the_next() {
    // A key named in words, as Space is, between two glyphs.
    let layout = layout(50., &[CHIP, 40., CHIP]);
    let first = 11. + 50. + 8.;
    assert_eq!(layout.chips, [(first, 18.5), (first + 22., 40.), (first + 22. + 43.5, 18.5)]);
    assert_eq!(layout.width, first + 22. + 43.5 + 18.5 + 10.);
}

#[test]
fn a_long_name_truncates_and_its_keys_stay_whole() {
    let layout = layout(1000., &[CHIP, CHIP]);
    assert_eq!(layout.width, WINDOW_WIDTH);
    assert!(layout.truncated);
    // The chips end 10 from the window's end, and the text stops 8 before them.
    let &(last, width) = layout.chips.last().unwrap();
    assert_eq!(last + width + 10., WINDOW_WIDTH);
    assert_eq!(11. + layout.text + 8., layout.chips[0].0);
}

#[gpui::test]
fn a_keys_chip_is_square_unless_its_name_is_wider(cx: &mut TestAppContext) {
    let keys = |keys: &[&str]| Tip {
        text: "View Library".into(),
        keys: keys.iter().map(|key| key.to_string().into()).collect(),
    };
    let glyphs = measured(cx, &keys(&["⇧", "⌘", "L"]));
    assert!(glyphs.chips.iter().all(|&(_, width)| width == CHIP));
    let words = measured(cx, &keys(&["⌘", "Space"]));
    assert_eq!(words.chips[0].1, CHIP);
    assert!(words.chips[1].1 > CHIP);
    // The same text with keys is wider by its chips, 8 before them and none of its end changes.
    let name = measured(cx, &tip("View Library"));
    assert_eq!(glyphs.width, name.width + 8. + 3. * CHIP + 2. * CHIP_GAP);
}

#[test]
fn the_log_names_the_keys_as_a_menu_does() {
    let tip = Tip { text: "View Library".into(), keys: vec!["⇧".into(), "⌘".into(), "L".into()] };
    assert_eq!(label(&tip), "View Library ⇧⌘L");
    assert_eq!(label(&Tip::named("Docs".into())), "Docs");
}

#[test]
fn arcs_capsule_is_27_tall_and_round_but_toward_its_button() {
    assert_eq!(HEIGHT, 27.);
    assert_eq!(RADIUS, 13.5);
    // Beside a button, and under a tile, the corner toward what it names is the top left.
    let beside = Anchor::beside(library_button(), window());
    assert_eq!(near(beside, HEIGHT), Near::TopLeft);
    assert_eq!(near(tile_a(), HEIGHT), Near::TopLeft);
    // Above it for want of room below, that corner is the bottom left.
    let bottom = Anchor::centred(frame(8., 650., 80., 41.), window());
    assert_eq!(near(bottom, HEIGHT), Near::BottomLeft);
    // A footer button's tooltip never goes above, whatever the window's height.
    let low = Anchor::beside(frame(8., 690., 32., 10.), window());
    assert_eq!(near(low, HEIGHT), Near::TopLeft);
}

#[test]
fn a_tooltip_shows_in_a_frame_and_fades_out_over_270_ms() {
    assert_eq!(FADE_IN, Duration::ZERO);
    assert_eq!(FADE_OUT, Duration::from_millis(270));
    assert_eq!(DELAY, Duration::from_millis(730));
}

#[test]
fn a_chip_is_18_and_a_half_wide_on_a_pitch_of_22() {
    assert_eq!(CHIP, 18.5);
    assert_eq!(CHIP + CHIP_GAP, 22.);
    // Its 16.25 body is centred on the capsule, and the lip hangs 2 under it.
    assert_eq!((HEIGHT - CHIP_BODY) / 2. + CHIP_BODY + CHIP_LIP, 23.625);
}
