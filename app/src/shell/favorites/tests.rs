//! Where the tooltips of the favorites' tiles hang, on a host that records them.

use std::rc::Rc;

use gpui::{Modifiers, TestAppContext, VisualTestContext};

use crate::{
    shell::{Shell, recorder::Recorder},
    tooltip::{Anchor, GAP, place},
};

/// The widths of a short title's tooltip and a long one's, and their height.
const SHORT: f32 = 150.;
const LONG: f32 = 306.;
const HEIGHT: f32 = 27.;

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

/// The anchor of the tooltip the shell asked for as the pointer moved onto `tab`'s tile from
/// wherever it was, and the tile's middle.
fn hover_tile(
    shell: &gpui::Entity<Shell>,
    cx: &mut VisualTestContext,
    host: &Recorder,
    tab: u64,
) -> (Anchor, f32) {
    let frame = shell.read_with(cx, |shell, _| shell.tip_frames.borrow()[&tab]);
    host.anchors.borrow_mut().clear();
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    cx.run_until_parked();
    let anchor = host.anchors.borrow().last().copied().expect("no tooltip asked for");
    (anchor, f32::from(frame.center().x))
}

/// Each tile's tooltip hangs from the tile it names, the pointer coming straight from the last
/// tile's, so the tooltip follows with no frame of the old one's place: its square corner is under
/// the tile's middle, 6 under it, going right from there.
#[gpui::test]
fn each_tiles_tooltip_hangs_from_that_tile(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    for tab in [10, 11, 12, 11, 10] {
        let (anchor, middle) = hover_tile(&shell, cx, &host, tab);
        let (bottom, window) =
            shell.read_with(cx, |shell, _| (shell.tip_frames.borrow()[&tab].bottom(), shell.viewport));
        assert_eq!(anchor.x, middle, "tile {tab}'s tooltip hangs from another place");
        assert_eq!(anchor.window, (f32::from(window.width), f32::from(window.height)));
        let (left, top) = place(anchor, (SHORT, HEIGHT));
        assert_eq!(left, middle, "tile {tab}: its corner isn't under its middle");
        assert_eq!(top, f32::from(bottom) + GAP, "tile {tab}: not 6 under it");
        let (left, _) = place(anchor, (LONG, HEIGHT));
        assert_eq!(left, middle, "tile {tab}: a long title's corner isn't under its middle");
    }
}

/// In a sidebar as narrow as it goes, where the third tile wraps to a second row, the tiles still
/// each hang from their own middle and bottom.
#[gpui::test]
fn a_narrow_sidebars_tiles_hang_from_their_own_middles(cx: &mut TestAppContext) {
    let (shell, cx, host) = with_tiles(cx);
    shell.update(cx, |shell, cx| {
        shell.sidebar_width = 164.;
        cx.notify();
    });
    cx.run_until_parked();
    for tab in [10, 11, 12] {
        let bottom =
            shell.read_with(cx, |shell, _| f32::from(shell.tip_frames.borrow()[&tab].bottom()));
        let (anchor, middle) = hover_tile(&shell, cx, &host, tab);
        assert_eq!((anchor.x, anchor.bottom), (middle, bottom), "tile {tab}");
    }
}
