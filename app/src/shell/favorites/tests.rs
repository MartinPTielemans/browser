//! Where the hover cards of the favorites' tiles go, what a click on one asks for, and how a split
//! view among them is one, on a host that records them.

use std::rc::Rc;

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

use crate::{
    hover_card::Anchor,
    shell::{Folders, LIST_TOP, PLATE_INSET, Row, Shell, TILE_HEIGHT, recorder::Recorder},
};

/// A drawn shell with the favorites 10, 11 and 12.
fn with_tiles(
    cx: &mut TestAppContext,
) -> (gpui::Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_favorites(vec![(10, 1), (11, 1), (12, 1)], cx);
        for (tab, title) in [(10, "YouTube"), (11, "X"), (12, "Proton Mail: Sign-in")] {
            shell.set_title(tab, title.into(), cx);
        }
    });
    cx.run_until_parked();
    (shell, cx, host)
}

/// Where the card the shell asked for as the pointer moved onto `tab`'s tile from wherever it was
/// goes from, and the tile's frame.
fn hover_tile(
    shell: &gpui::Entity<Shell>,
    cx: &mut VisualTestContext,
    host: &Recorder,
    tab: u64,
) -> (Anchor, gpui::Bounds<gpui::Pixels>) {
    let frame = shell.read_with(cx, |shell, _| shell.tip_frames.borrow()[&tab]);
    host.card.borrow_mut().take();
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    cx.run_until_parked();
    let rest = host.card.borrow().clone().expect("no card asked for");
    assert_eq!(rest.card.tab, tab);
    (rest.anchor, frame)
}

/// The sidebar's rows' right edge in the window, which a card starts GAP past.
fn rows_right(shell: &gpui::Entity<Shell>, cx: &mut VisualTestContext) -> f32 {
    shell.read_with(cx, |shell, _| {
        f32::from(shell.sidebar_origin.get().x) + shell.width() - PLATE_INSET
    })
}

/// Each tile's card goes beside the sidebar's rows, not over the tiles beside it, its top at the
/// tile's, the pointer coming straight from the last tile's.
#[gpui::test]
fn each_tiles_card_goes_beside_the_rows_at_its_top(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    let right = rows_right(&shell, cx);
    for tab in [10, 11, 12, 11, 10] {
        let (anchor, frame) = hover_tile(&shell, cx, &host, tab);
        let window = shell.read_with(cx, |shell, _| shell.viewport);
        assert_eq!(anchor.window, window);
        assert_eq!(anchor.row.top(), frame.top(), "tile {tab}");
        assert_eq!(f32::from(anchor.row.right()), right, "tile {tab}");
    }
}

/// A tile's card starts at the tile's top, under the URL field, and its frame is the tile's
/// outside on a 1× display as on a 2× one, where GPUI draws the hairline round it 1 pt wide.
#[gpui::test]
fn a_tiles_card_starts_at_its_top_at_1x_and_2x(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    for (scale, tab) in [(1., 10), (2., 11), (1., 12)] {
        cx.simulate_scale_factor_change(scale);
        cx.run_until_parked();
        let (anchor, frame) = hover_tile(&shell, cx, &host, tab);
        assert_eq!(f32::from(anchor.row.top()), LIST_TOP, "tile {tab} at {scale}×");
        assert_eq!(f32::from(frame.size.height), TILE_HEIGHT, "tile {tab} at {scale}×");
    }
}

/// In a sidebar as narrow as it goes, where the third tile wraps to a second row, each tile's card
/// still starts at its own top.
#[gpui::test]
fn a_narrow_sidebars_tiles_have_cards_at_their_own_tops(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    shell.update(cx, |shell, cx| {
        shell.sidebar_width = 164.;
        cx.notify();
    });
    cx.run_until_parked();
    let right = rows_right(&shell, cx);
    for tab in [10, 11, 12] {
        let (anchor, frame) = hover_tile(&shell, cx, &host, tab);
        assert_eq!((anchor.row.top(), f32::from(anchor.row.right())), (frame.top(), right), "tile {tab}");
    }
}

/// A drawn shell with the favorite 10, Docs, and the Today tabs 1 and 2.
fn with_docs(
    cx: &mut TestAppContext,
) -> (gpui::Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Work".into(), None, 0, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, Vec::new(), vec![1, 2], Vec::new(), Folders::default(), cx);
        shell.set_favorites(vec![(10, 0)], cx);
        shell.set_title(10, "Docs".into(), cx);
    });
    cx.run_until_parked();
    (shell, cx, host)
}

fn cards_for(host: &Recorder, tab: u64) -> usize {
    let rest = format!("card rest {tab} ");
    host.take().iter().filter(|call| call.starts_with(&rest)).count()
}

/// A drop that leaves the pointer on a tile, which the drag had it hover, asks for the tile's
/// card once the tile has settled: no hover change comes to ask for it, and not before, as a tile
/// still sliding would put it beside where the tile was.
#[gpui::test]
fn a_tile_under_the_pointer_after_a_drag_has_its_card(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_docs(cx);
    let (x, y) = shell.update(cx, |shell, _| {
        let [left, top, width] = shell.tile_places(1)[0];
        (left + width / 2., shell.tiles_top() + top + TILE_HEIGHT / 2.)
    });
    shell.update(cx, |shell, _| shell.press_tile(10, point(px(x), px(y))));
    shell.update(cx, |shell, cx| shell.drag_to(point(px(x), px(y + 30.)), cx));
    assert!(shell.read_with(cx, |shell, _| shell.dragging().is_some()));
    // The tile is hovered under the drag, which keeps its card.
    shell.update(cx, |shell, cx| shell.hover(Row::Tab(10), true, cx));
    cx.run_until_parked();
    assert_eq!(cards_for(&host, 10), 0);

    shell.update(cx, |shell, cx| {
        assert!(shell.cancel_drag(cx));
        // Its tile is still sliding home.
        shell.tile_slides.insert(10, ([0.; 3], std::time::Instant::now(), crate::shell::Curve::Slide));
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(cards_for(&host, 10), 0, "while it slides");

    shell.update(cx, |shell, cx| {
        shell.tile_slides.clear();
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(cards_for(&host, 10), 1);
    shell.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert_eq!(cards_for(&host, 10), 0, "only once");
}

/// A click on a tile shows its tab and gives its page the keyboard, as a press on a row does.
#[gpui::test]
fn a_click_on_a_tile_gives_its_page_the_keyboard(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    let middle = shell.read_with(cx, |shell, _| shell.tip_frames.borrow()[&11].center());
    cx.simulate_mouse_move(middle, None, Modifiers::none());
    cx.simulate_click(middle, Modifiers::none());
    cx.run_until_parked();
    let calls = host.take();
    assert!(calls.ends_with(&["select 11".into(), "focus page".into()]), "{calls:?}");
}

/// At MOST_FAVORITES no tab becomes another: Add to Favorites is dimmed and leaves them, the hover
/// card has no Add to Favorites, and a row dragged over the tiles stays a row. One short of it, the
/// drag makes it a tile.
#[gpui::test]
fn the_favorites_take_no_more_than_the_most(cx: &mut TestAppContext) {
    use crate::{
        hover_card::Action,
        shell::{
            Place,
            drag::detents::{X, drag_to, press, shell_with},
            favorites::MOST_FAVORITES,
        },
    };

    let (shell, host) = shell_with(cx, &[1], &[2], Folders::default());
    let favorites = |count: u64| (100..100 + count).map(|tab| (tab, 0)).collect::<Vec<_>>();
    let over_tiles = |cx: &mut TestAppContext| {
        press(&shell, cx, 2);
        let top = shell.read_with(cx, |shell, _| shell.tiles_top());
        drag_to(&shell, cx, X, top + TILE_HEIGHT / 2.);
        let to = shell.read_with(cx, |shell, _| shell.drag.as_ref().map(|drag| drag.to));
        shell.update(cx, |shell, cx| shell.cancel_drag(cx));
        to
    };
    shell.update(cx, |shell, cx| shell.set_favorites(favorites(MOST_FAVORITES as u64 - 1), cx));
    assert!(matches!(over_tiles(cx), Some(Place::Favorite(_))));

    shell.update(cx, |shell, cx| shell.set_favorites(favorites(MOST_FAVORITES as u64), cx));
    assert!(matches!(over_tiles(cx), Some(Place::List(..))));
    let actions = shell.read_with(cx, |shell, _| shell.card_actions(1));
    assert_eq!(actions, [Action::Unpin]);
    host.take_edits();
    shell.update(cx, |shell, cx| shell.favorite_tab(1, usize::MAX, cx));
    assert!(host.take_edits().is_empty(), "a thirteenth favorite");
    // A favorite still moves among them.
    shell.update(cx, |shell, cx| shell.favorite_tab(100, 3, cx));
    assert_eq!(host.take_edits(), ["favorite 100 3"]);
}

/// A shell showing a Space with the Today tab 1 and the split view 2 and 3, under the favorites
/// 10, the stacked split view 11 and 12, and 13, as the core reports them.
fn with_split_tile(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, Rc<Recorder>) {
    use crate::shell::{Split, drag::detents::shell_with};

    let (shell, host) = shell_with(cx, &[], &[1, 2, 3], Folders::default());
    shell.update(cx, |shell, cx| {
        let split = Split { tabs: vec![2, 3], ..Split::default() };
        shell.set_tabs(1, Vec::new(), vec![1, 2, 3], vec![split], Folders::default(), cx);
        let split = Split { tabs: vec![11, 12], stacked: true, ..Split::default() };
        shell.set_favorites_split(vec![(10, 0), (11, 0), (12, 0), (13, 0)], vec![split], cx);
    });
    (shell, host)
}

/// A split view among the favorites is one tile, its first tab's, which shows its pages together
/// as they were, and is the tile shown while any of its pages is.
#[gpui::test]
fn a_favorite_split_view_is_one_tile_of_its_pages(cx: &mut TestAppContext) {
    let (shell, _) = with_split_tile(cx);
    shell.update(cx, |shell, cx| {
        assert_eq!(shell.tile_leads(), [10, 11, 13]);
        let mut tiles: Vec<u64> = shell.resting_tiles().into_keys().collect();
        tiles.sort();
        assert_eq!(tiles, [10, 11, 13]);
        assert_eq!((shell.panes(12), shell.stacked(12)), (vec![11, 12], true));
        shell.set_selected(12, cx);
        assert!(shell.tile_selected(11) && !shell.tile_selected(10) && !shell.tile_selected(13));
    });
}

/// The browser counts the favorites' tabs, so a tile dropped past a split view's goes past each
/// of its tabs, and the dragged one's own tabs aren't counted.
#[gpui::test]
fn a_tile_dropped_past_a_split_view_goes_past_its_tabs(cx: &mut TestAppContext) {
    let (shell, _) = with_split_tile(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.favorite_index(13, 2), 3);
        assert_eq!(shell.favorite_index(13, 1), 1);
        assert_eq!(shell.favorite_index(10, 1), 2);
        assert_eq!(shell.favorite_index(11, 1), 1);
        assert_eq!(shell.favorite_index(11, 2), 2);
    });
}

/// A split view's row dragged over the tiles turns into one, and let go after the last tile it
/// asks the browser to make the split a favorite past every favorite's tab.
#[gpui::test]
fn a_split_views_row_dragged_onto_the_tiles_becomes_one_favorite(cx: &mut TestAppContext) {
    use crate::shell::{
        Place,
        drag::detents::{X, drag_to, press},
    };

    let (shell, host) = with_split_tile(cx);
    host.take_edits();
    press(&shell, cx, 2);
    let (x, y) = shell.read_with(cx, |shell, _| {
        let [left, top, width] = shell.tile_places(4)[3];
        (left + width / 2., shell.tiles_top() + top + TILE_HEIGHT / 2.)
    });
    drag_to(&shell, cx, X, y + 60.);
    drag_to(&shell, cx, x, y);
    let to = shell.read_with(cx, |shell, _| shell.drag.as_ref().map(|drag| drag.to));
    assert!(matches!(to, Some(Place::Favorite(3))));
    shell.update(cx, |shell, cx| {
        // Let go once it has turned into a tile.
        if let Some(drag) = shell.drag.as_mut() {
            drag.shape.start = None;
        }
        shell.drop_row(cx)
    });
    assert_eq!(host.take_edits(), ["favorite 2 4"]);
}

/// A favorite's split view stays as it was made: the commands that change a split, the gaps and
/// the split controls have nothing to do there, and no page joins it, until it's back in a Space.
#[gpui::test]
fn a_favorite_split_view_stays_as_it_was_made(cx: &mut TestAppContext) {
    use crate::shell::Command;

    let (shell, _) = with_split_tile(cx);
    let changes = [
        Command::CloseSplitPane,
        Command::SeparatePane,
        Command::SeparateSplit,
        Command::FlipSplit,
        Command::ChangeOrientation,
        Command::ExpandSplit,
        Command::MovePaneLeft,
        Command::MovePaneRight,
    ];
    shell.update(cx, |shell, cx| {
        shell.set_selected(2, cx);
        let offered = |shell: &Shell| changes.iter().filter(|&&each| shell.offers(each)).count();
        // A Space's split has them all but moving its left page left.
        assert_eq!(offered(shell), changes.len() - 1);
        assert!(shell.has_pane_room(2));
        shell.set_selected(12, cx);
        assert_eq!(offered(shell), 0);
        assert!(!shell.has_pane_room(12));
        assert_eq!(shell.split_partner(1), None);
    });
}

/// ⌘W on a page of a favorite's split view closes its pages together, as the split is one
/// favorite. A page of a Space's split closes alone.
#[gpui::test]
fn close_tab_closes_a_favorite_split_views_pages_together(cx: &mut TestAppContext) {
    use crate::shell::Command;

    let (shell, host) = with_split_tile(cx);
    shell.update(cx, |shell, cx| {
        shell.set_selected(12, cx);
        shell.run(Command::CloseTab, "test", cx);
        shell.set_selected(3, cx);
        shell.run(Command::CloseTab, "test", cx);
    });
    assert_eq!(*host.split_closes.borrow(), [12]);
    assert_eq!(host.take_edits(), ["close 12", "close 3"]);
}

/// The most favorites counts tiles, so a split view takes one place among them however many
/// pages it has.
#[gpui::test]
fn a_favorite_split_view_takes_one_place_of_the_most(cx: &mut TestAppContext) {
    use crate::shell::{Split, favorites::MOST_FAVORITES};

    let (shell, _) = with_split_tile(cx);
    shell.update(cx, |shell, cx| {
        let singles = 100..100 + MOST_FAVORITES as u64 - 2;
        let favorites = singles.chain([11, 12]).map(|tab| (tab, 0)).collect::<Vec<_>>();
        let split = Split { tabs: vec![11, 12], ..Split::default() };
        shell.set_favorites_split(favorites.clone(), vec![split.clone()], cx);
        assert!(shell.has_room_for(1));
        let favorites = [favorites, vec![(13, 0)]].concat();
        shell.set_favorites_split(favorites, vec![split], cx);
        assert!(!shell.has_room_for(1));
    });
}
