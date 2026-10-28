//! When the hover card shows, swaps and goes, on a host that records what it asks of its window,
//! where it goes, and what its pictures do to it.

use std::cell::RefCell;

use gpui::{TestAppContext, point};

use super::*;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Host for Recorder {
    fn show(&self, left: f32, top: f32, width: f32, height: f32) {
        self.0.borrow_mut().push(format!("show {left} {top} {width} {height}"));
    }

    fn hide(&self) {
        self.0.borrow_mut().push("hide".into());
    }
}

/// What the card told the shell, in order.
type Told = Rc<RefCell<Vec<String>>>;

fn open_card(cx: &mut TestAppContext) -> (Rc<Recorder>, Told) {
    let host = Rc::new(Recorder::default());
    cx.update(|cx| open(host.clone(), cx)).expect("no card window");
    (host, Told::default())
}

fn window() -> Size<Pixels> {
    size(px(1000.), px(700.))
}

/// The row of the tab `tab`, the sidebar's rows being 36 apart from 100 down.
fn row(tab: u64) -> Anchor {
    let top = 100. + 36. * tab as f32;
    Anchor { row: Bounds::new(point(px(8.), px(top)), size(px(224.), px(34.))), window: window() }
}

fn card(tab: u64, pictured: bool) -> Card {
    Card {
        tab,
        title: format!("Tab {tab}").into(),
        host: "example.com".into(),
        icon: Icon::Favicon(None),
        pictured,
        actions: vec![Action::Pin, Action::Favorite, Action::Split],
    }
}

fn rest_on(cx: &mut TestAppContext, told: &Told, card: Card) {
    let told = told.clone();
    let events = Rc::new(move |tab: u64, event: Event, _: &mut App| {
        told.borrow_mut().push(match event {
            Event::Shown => format!("shown {tab}"),
            Event::Act(action) => format!("{} {tab}", action.label()),
        })
    });
    let anchor = row(card.tab);
    cx.update(|cx| hover(Hover::Rest(Rest { card, anchor, events }), cx));
    cx.run_until_parked();
}

fn ask(cx: &mut TestAppContext, what: Hover) {
    cx.update(|cx| hover(what, cx));
    cx.run_until_parked();
}

fn after(cx: &mut TestAppContext, host: &Recorder, time: Duration) -> Vec<String> {
    cx.executor().advance_clock(time);
    cx.run_until_parked();
    host.0.take()
}

/// Where the host is told a card of `tab` goes, `height` tall.
fn shown(tab: u64, height: f32) -> String {
    let (left, top) = place(row(tab), height);
    format!("show {left} {top} {WIDTH} {height}")
}

/// A card with a picture's box but no picture yet.
fn waiting_height() -> f32 {
    height(picture_height(true, None))
}

fn card_window(cx: &mut TestAppContext) -> WindowHandle<HoverCard> {
    cx.update(|cx| cx.global::<CardWindow>().0)
}

/// A picture `width` by `height` pixels, of no colour.
fn picture(width: u32, height: u32) -> PagePicture {
    PagePicture::from_bgra(vec![0; (width * height * 4) as usize], width, height).unwrap()
}

#[gpui::test]
fn a_card_shows_once_the_pointer_has_rested_on_its_row(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    let before = DWELL - Duration::from_millis(10);
    assert!(after(cx, &host, before).is_empty(), "a passing pointer opens nothing");
    assert_eq!(after(cx, &host, Duration::from_millis(10)), [shown(1, waiting_height())]);
    assert_eq!(*told.borrow(), ["shown 1"]);
}

#[gpui::test]
fn the_pointer_leaving_before_the_dwell_opens_nothing(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    ask(cx, Hover::Left(1));
    assert!(after(cx, &host, DWELL * 2).is_empty());
    assert!(told.borrow().is_empty());
}

#[gpui::test]
fn resting_again_on_the_same_row_keeps_its_dwell(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL / 2);
    rest_on(cx, &told, card(1, true));
    assert_eq!(after(cx, &host, DWELL / 2), [shown(1, waiting_height())]);
}

#[gpui::test]
fn the_next_row_swaps_in_at_once_while_a_card_shows(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    // The pointer comes onto the next row before the one it left hears so, as GPUI may order them.
    rest_on(cx, &told, card(2, true));
    ask(cx, Hover::Left(1));
    assert_eq!(host.0.take(), [shown(2, waiting_height())]);
    assert!(after(cx, &host, LEAVE * 4).is_empty(), "leaving the old row leaves the new card");
    assert_eq!(*told.borrow(), ["shown 1", "shown 2"]);
}

#[gpui::test]
fn the_next_row_swaps_in_at_once_after_the_old_one_is_left(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    ask(cx, Hover::Left(1));
    rest_on(cx, &told, card(2, true));
    assert_eq!(host.0.take(), [shown(2, waiting_height())]);
    assert!(after(cx, &host, LEAVE * 4).is_empty());
}

#[gpui::test]
fn leaving_the_rows_closes_the_card_after_a_moment(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    ask(cx, Hover::Left(1));
    assert!(after(cx, &host, LEAVE - Duration::from_millis(10)).is_empty());
    assert_eq!(after(cx, &host, Duration::from_millis(10)), ["hide"]);
    // The next row waits for its dwell again.
    rest_on(cx, &told, card(2, true));
    assert!(host.0.take().is_empty());
    assert_eq!(after(cx, &host, DWELL), [shown(2, waiting_height())]);
}

#[gpui::test]
fn the_pointer_on_the_card_keeps_it(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    ask(cx, Hover::Left(1));
    let window = card_window(cx);
    cx.update(|cx| window.update(cx, |card, _, cx| card.on_card(true, cx)).unwrap());
    assert!(after(cx, &host, LEAVE * 4).is_empty(), "it stays while the pointer is on it");
    cx.update(|cx| window.update(cx, |card, _, cx| card.on_card(false, cx)).unwrap());
    assert_eq!(after(cx, &host, LEAVE), ["hide"]);
}

#[gpui::test]
fn a_press_a_scroll_or_a_menu_closes_it_at_once(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    ask(cx, Hover::Close);
    assert_eq!(host.0.take(), ["hide"]);
    // And one waiting for its dwell never comes.
    rest_on(cx, &told, card(2, true));
    ask(cx, Hover::Close);
    assert!(after(cx, &host, DWELL * 2).is_empty());
}

#[gpui::test]
fn a_button_closes_the_card_then_tells_the_shell(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    let window = card_window(cx);
    cx.update(|cx| window.update(cx, |card, _, cx| card.press(Action::Favorite, cx)).unwrap());
    assert_eq!(host.0.take(), ["hide"]);
    assert_eq!(*told.borrow(), ["shown 1", "favorite 1"]);
}

#[gpui::test]
fn a_picture_coming_while_its_card_shows_takes_the_page_s_shape(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    let request = cx.update(|cx| asking(1, cx));
    assert!(is_card_request(request));
    rest_on(cx, &told, card(1, true));
    after(cx, &host, DWELL);
    // A page 1200 by 800 points, at 2×, scaled to fit the card's pixels.
    cx.update(|cx| answered(request, Some(picture(528, 352)), cx));
    cx.run_until_parked();
    assert_eq!(host.0.take(), [shown(1, height(INNER * 352. / 528.))]);
    // Its next card has it at once.
    ask(cx, Hover::Close);
    host.0.take();
    rest_on(cx, &told, card(1, true));
    assert_eq!(after(cx, &host, DWELL), [shown(1, height(INNER * 352. / 528.))]);
}

#[gpui::test]
fn an_unloaded_or_closed_tab_s_picture_goes(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    let request = cx.update(|cx| asking(1, cx));
    cx.update(|cx| answered(request, Some(picture(528, 352)), cx));
    cx.update(|cx| forget(1, cx));
    rest_on(cx, &told, card(1, true));
    assert_eq!(after(cx, &host, DWELL), [shown(1, waiting_height())]);
}

#[gpui::test]
fn memory_pressure_takes_every_picture(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    for tab in [1, 2] {
        let request = cx.update(|cx| asking(tab, cx));
        cx.update(|cx| answered(request, Some(picture(528, 352)), cx));
    }
    cx.update(forget_all);
    rest_on(cx, &told, card(2, true));
    assert_eq!(after(cx, &host, DWELL), [shown(2, waiting_height())]);
}

#[gpui::test]
fn the_tab_shown_has_no_picture_box(cx: &mut TestAppContext) {
    let (host, told) = open_card(cx);
    let request = cx.update(|cx| asking(1, cx));
    cx.update(|cx| answered(request, Some(picture(528, 352)), cx));
    rest_on(cx, &told, card(1, false));
    assert_eq!(after(cx, &host, DWELL), [shown(1, height(0.))]);
}

#[test]
fn the_picture_s_box_keeps_to_its_shares() {
    assert_eq!(picture_height(false, Some((528., 352.))), 0.);
    assert_eq!(picture_height(true, None), INNER * PICTURE_DEFAULT);
    // A tall narrow page is cut to the most, a wide one to the least.
    assert_eq!(picture_height(true, Some((300., 600.))), INNER * PICTURE_MOST);
    assert_eq!(picture_height(true, Some((600., 100.))), INNER * PICTURE_LEAST);
    // Its tallest fits the pixels the core is asked for, twice over.
    assert_eq!(PICTURE_PIXELS, ((2. * INNER) as i32, (2. * INNER * PICTURE_MOST) as i32));
    assert!(height(INNER * PICTURE_MOST) + 2. * WINDOW_MARGIN <= WINDOW_HEIGHT);
}

#[test]
fn a_card_goes_beside_its_row_and_stays_inside_the_window() {
    let tall = height(INNER * PICTURE_MOST);
    let (left, top) = place(row(1), tall);
    assert_eq!((left, top), (8. + 224. + GAP, 136.));
    // Near the window's bottom it rises to keep EDGE from it.
    let low = Anchor { row: Bounds::new(point(px(8.), px(660.)), size(px(224.), px(34.))), ..row(0) };
    assert_eq!(place(low, tall).1, 700. - tall - EDGE);
    // In a window too narrow for it, it keeps EDGE from the right.
    let narrow = Anchor { window: size(px(400.), px(700.)), ..row(0) };
    assert_eq!(place(narrow, tall).0, 400. - WIDTH - EDGE);
}
