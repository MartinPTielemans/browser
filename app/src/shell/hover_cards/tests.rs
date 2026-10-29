//! Which tabs have a hover card and what it has, when it closes, what its buttons do, and when the
//! tabs' pictures are taken, on a host that records what the shell asks.

use std::rc::Rc;

use gpui::{
    Modifiers, MouseButton, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext,
    point, px,
};

use crate::{
    dock_dot::QUIET,
    hover_card::{Action, Event},
    shell::{Folders, Page, Row, Shell, recorder::Recorder},
};

/// A drawn shell with the pinned tab 1, the Today tabs 2, 3 and 4, and the favorite 10, showing 2.
fn with_tabs(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Work".into(), None, 1, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, vec![1], vec![2, 3, 4], Vec::new(), Folders::default(), cx);
        shell.set_favorites(vec![(10, 1)], cx);
        for (tab, title) in [(1, "Mail"), (2, "Docs"), (3, "News"), (4, "Maps"), (10, "Music")] {
            shell.set_title(tab, title.into(), cx);
            let url = format!("https://www.{}.example/page", title.to_lowercase());
            shell.set_page(tab, Page { url: url.into(), ..Default::default() }, cx);
        }
        shell.set_selected(2, cx);
    });
    cx.run_until_parked();
    cx.executor().advance_clock(QUIET * 2);
    cx.run_until_parked();
    host.take();
    (shell, cx, host)
}

/// What the shell asks of the card as the pointer rests on `tab`'s row or tile.
fn rest_on(shell: &gpui::Entity<Shell>, cx: &mut VisualTestContext, host: &Recorder, tab: u64) -> String {
    shell.update(cx, |shell, cx| shell.hover_card(Row::Tab(tab), true, cx));
    cx.run_until_parked();
    let asked = host.take();
    let rests: Vec<_> = asked.iter().filter(|call| call.starts_with("card rest")).collect();
    assert_eq!(rests.len(), 1, "{asked:?}");
    rests[0].clone()
}

#[gpui::test]
fn each_kind_of_tab_has_its_own_buttons(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    assert_eq!(rest_on(&shell, cx, &host, 3), "card rest 3 pin favorite split picture");
    assert_eq!(rest_on(&shell, cx, &host, 1), "card rest 1 unpin favorite split picture");
    // A favorite can't join a split view, as its drag onto the page can't.
    assert_eq!(rest_on(&shell, cx, &host, 10), "card rest 10 unfavorite picture");
}

#[gpui::test]
fn the_tab_shown_has_no_picture(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    let shown = rest_on(&shell, cx, &host, 2);
    assert!(shown.starts_with("card rest 2 ") && shown.ends_with(" none"), "{shown}");
}

#[gpui::test]
fn a_split_shown_full_has_no_split_button(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    shell.update(cx, |shell, cx| {
        let split = crate::shell::Split { tabs: vec![2, 3, 4], ..Default::default() };
        shell.set_tabs(1, vec![1], vec![2, 3, 4, 5], vec![split], Folders::default(), cx);
    });
    cx.run_until_parked();
    host.take();
    shell.update(cx, |shell, cx| shell.set_title(5, "Five".into(), cx));
    assert_eq!(rest_on(&shell, cx, &host, 5), "card rest 5 pin favorite picture");
    // And a split view's row has no card.
    shell.update(cx, |shell, cx| shell.hover_card(Row::Tab(2), true, cx));
    assert!(!host.take().iter().any(|call| call.starts_with("card rest")));
}

#[gpui::test]
fn a_private_window_s_cards_only_split(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) =
        cx.add_window_view(|_, cx| Shell::private_window(browser, 9, "Private".into(), 1, cx));
    shell.update(cx, |shell, cx| {
        shell.set_tabs(9, Vec::new(), vec![20, 21], Vec::new(), Folders::default(), cx);
        shell.set_title(20, "One".into(), cx);
        shell.set_title(21, "Two".into(), cx);
        shell.set_selected(20, cx);
    });
    cx.run_until_parked();
    host.take();
    assert_eq!(rest_on(&shell, cx, &host, 21), "card rest 21 split picture");
}

#[gpui::test]
fn the_pointer_leaving_a_row_lets_its_card_go(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    rest_on(&shell, cx, &host, 3);
    shell.update(cx, |shell, cx| shell.hover_card(Row::Tab(3), false, cx));
    assert_eq!(host.take(), ["card left 3"]);
}

#[gpui::test]
fn a_scroll_or_a_press_closes_the_card(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    let frame = shell.read_with(cx, |shell, _| shell.tip_frames.borrow()[&3]);
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    cx.run_until_parked();
    host.take();
    cx.simulate_event(ScrollEvent {
        position: frame.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-10.))),
        ..Default::default()
    });
    cx.run_until_parked();
    assert!(host.take().contains(&"card close".to_string()), "a scroll");
    cx.simulate_mouse_down(frame.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(host.take().contains(&"card close".to_string()), "a press");
}

/// A middle press closes the row's tab, and its card with it.
#[gpui::test]
fn a_middle_press_closes_the_card(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    let frame = shell.read_with(cx, |shell, _| shell.tip_frames.borrow()[&3]);
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    cx.run_until_parked();
    host.take();
    cx.simulate_mouse_down(frame.center(), MouseButton::Middle, Modifiers::none());
    cx.run_until_parked();
    let asked = host.take();
    assert!(asked.contains(&"card close".to_string()), "{asked:?}");
    assert!(asked.contains(&"close 3".to_string()), "{asked:?}");
}

#[gpui::test]
fn a_rows_menu_closes_the_card(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    rest_on(&shell, cx, &host, 3);
    shell.update(cx, |shell, cx| shell.tab_menu(3, point(px(50.), px(200.)), cx));
    assert!(host.take().contains(&"card close".to_string()));
    // And none rests while it's up.
    shell.update(cx, |shell, cx| shell.hover_card(Row::Tab(4), true, cx));
    assert!(!host.take().iter().any(|call| call.starts_with("card rest")));
}

/// Each button makes the call the row's menu makes, or for Split, the one its drag onto the page
/// makes, beside the tab shown.
#[gpui::test]
fn the_buttons_make_the_menus_calls(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    let act = |cx: &mut VisualTestContext, tab: u64, action: Action| {
        rest_on(&shell, cx, &host, tab);
        let rest = host.card.borrow().clone().unwrap();
        cx.update(|_, cx| (rest.events)(tab, Event::Act(action), cx));
        cx.run_until_parked();
        host.take_edits()
    };
    assert_eq!(act(cx, 3, Action::Pin), ["pinned 3 true"]);
    assert_eq!(act(cx, 1, Action::Unpin), ["pinned 1 false"]);
    assert_eq!(act(cx, 4, Action::Favorite), [format!("favorite 4 {}", usize::MAX)]);
    assert_eq!(act(cx, 10, Action::Unfavorite), ["move 10 false 0 0"]);
    assert_eq!(act(cx, 3, Action::Split), ["split 3 2 false"]);
}

/// While a page shares another tab, the card of a tab Chrome would offer instead has Share This
/// Tab Instead, which has the core switch the share to it; the others' cards don't.
#[gpui::test]
fn a_tab_to_share_instead_offers_it(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    shell.update(cx, |shell, cx| {
        let page = Page { share_instead: true, ..shell.page(3) };
        shell.set_page(3, page, cx);
    });
    assert_eq!(rest_on(&shell, cx, &host, 4), "card rest 4 pin favorite split picture");
    assert_eq!(
        rest_on(&shell, cx, &host, 3),
        "card rest 3 pin favorite split share-instead picture"
    );
    let rest = host.card.borrow().clone().unwrap();
    cx.update(|_, cx| (rest.events)(3, Event::Act(Action::ShareInstead), cx));
    cx.run_until_parked();
    assert!(host.take().contains(&"share instead 3".to_owned()));
}

/// The tabs a run of switches hides are pictured once it's over, never on a switch; the tab shown
/// and an unloaded one aren't.
#[gpui::test]
fn hidden_tabs_are_pictured_once_the_switches_are_quiet(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    shell.update(cx, |shell, cx| {
        let page = Page { unloaded: true, ..shell.page(3) };
        shell.set_page(3, page, cx);
        shell.set_selected(3, cx);
        shell.set_selected(4, cx);
    });
    cx.run_until_parked();
    let pictured = |host: &Recorder| -> Vec<String> {
        host.take().into_iter().filter(|call| call.starts_with("card picture")).collect()
    };
    assert!(pictured(&host).is_empty(), "nothing on the switch");
    cx.executor().advance_clock(QUIET / 2);
    shell.update(cx, |shell, cx| shell.set_selected(1, cx));
    cx.executor().advance_clock(QUIET / 2);
    cx.run_until_parked();
    assert!(pictured(&host).is_empty(), "another switch puts it off");
    cx.executor().advance_clock(QUIET);
    cx.run_until_parked();
    assert_eq!(pictured(&host), ["card picture 2", "card picture 4"]);
}

/// A card showing for a loaded tab in the back pictures it again, as it is now; the tab shown's
/// doesn't.
#[gpui::test]
fn a_card_showing_pictures_its_tab_again(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tabs(cx);
    let mut asked = Vec::new();
    for tab in [3, 2] {
        rest_on(&shell, cx, &host, tab);
        let rest = host.card.borrow().clone().unwrap();
        cx.update(|_, cx| (rest.events)(tab, Event::Shown, cx));
        cx.run_until_parked();
        asked.extend(host.take());
    }
    assert_eq!(asked, ["card picture 3"]);
}
