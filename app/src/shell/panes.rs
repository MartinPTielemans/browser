//! A split view's pages in the card, and the gaps between them, which drag to resize
//! them (docs/ux-spec.md › Split view).

use gpui::{
    AnyElement, Bounds, Context, CursorStyle, DispatchPhase, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Size, canvas, div, point, prelude::*, px, size,
};

use super::{CARD_INSET, EXPANDED_OTHER, MAX_PANES, PANE_LEAST, PANE_SNAP, SPLIT_GAP, Shell};

impl Shell {
    /// The gaps between the pages of the split view shown, which drag the pages' shares of the
    /// card and even them on a double-click.
    pub(super) fn pane_gaps(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let panes = self.panes(self.selected);
        if panes.len() < 2 || self.adding_pane.is_some() || self.page_fullscreen {
            return Vec::new();
        }
        let (frame, _) = self.page_frame(self.viewport);
        let stacked = self.stacked(self.selected);
        let frames = pane_frames(frame, panes.len(), stacked, &self.shares(self.selected));
        let cursor = if stacked { CursorStyle::ResizeUpDown } else { CursorStyle::ResizeLeftRight };
        let mut gaps: Vec<AnyElement> = frames
            .windows(2)
            .enumerate()
            .map(|(index, pair)| {
                let gap = match stacked {
                    true => Bounds::from_corners(pair[0].bottom_left(), pair[1].top_right()),
                    false => Bounds::from_corners(pair[0].top_right(), pair[1].bottom_left()),
                };
                div()
                    .id(("pane-gap", index))
                    .absolute()
                    .left(gap.left())
                    .top(gap.top())
                    .w(gap.size.width)
                    .h(gap.size.height)
                    .cursor(cursor)
                    .block_mouse_except_scroll()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                            shell.press_gap(index + 1, event, cx)
                        }),
                    )
                    .into_any_element()
            })
            .collect();
        // The pointer's moves and release anywhere in the window, while a gap is dragged.
        let resizing = self.pane_resize.is_some();
        let shell = cx.weak_entity();
        let listener = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                if resizing {
                    window.set_window_cursor_style(cursor);
                }
                let moved = shell.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture {
                        let at = if stacked { event.position.y } else { event.position.x };
                        moved.update(cx, |shell, cx| shell.drag_gap(at.into(), cx)).ok();
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        shell.update(cx, |shell, cx| shell.let_go_gap(cx)).ok();
                    }
                });
            },
        );
        gaps.push(listener.absolute().size_0().into_any_element());
        gaps
    }

    /// Where the pages of the split view shown start, and one after them would.
    pub(super) fn shown_pane_edges(&self) -> Vec<f32> {
        let (frame, _) = self.page_frame(self.viewport);
        let count = self.panes(self.selected).len();
        pane_edges(frame, count, self.stacked(self.selected), &self.shares(self.selected))
    }

    pub(super) fn press_gap(
        &mut self,
        pane: usize,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        if event.click_count == 2 {
            self.pane_resize = None;
            return self.browser.set_split_shares(self.selected, None, cx);
        }
        let at = match self.stacked(self.selected) {
            true => event.position.y,
            false => event.position.x,
        };
        let edges = self.shown_pane_edges();
        self.pane_resize = edges.get(pane).map(|&edge| (pane, f32::from(at) - edge));
        cx.notify();
    }

    /// The pointer let go of the gap it dragged. The pages are where it left them, and the core,
    /// which laid them out as the gap moved, hears they're still.
    pub(super) fn let_go_gap(&mut self, cx: &mut Context<Self>) {
        self.pane_resize = None;
        self.stop_page_motion(cx);
    }

    /// Moves the start of the page being resized with the pointer, `at` across the window, or
    /// down it stacked, taking from the page before it or giving to it.
    pub(super) fn drag_gap(&mut self, at: f32, cx: &mut Context<Self>) {
        let Some((pane, grab)) = self.pane_resize else {
            return;
        };
        let mut edges = self.shown_pane_edges();
        let count = edges.len() - 1;
        if pane == 0 || pane >= count {
            return;
        }
        let (start, span) = (edges[0], edges[count] - edges[0]);
        let least = ((span - SPLIT_GAP) * 0.1).clamp(PANE_LEAST.0, PANE_LEAST.1).round();
        let low = edges[pane - 1] + least + SPLIT_GAP;
        let high = edges[pane + 1] - SPLIT_GAP - least;
        if low > high {
            return;
        }
        let even = |index: usize| start + (span * index as f32 / count as f32).floor();
        let mut edge = (at - grab).round().clamp(low, high);
        if (edge - even(pane)).abs() < PANE_SNAP {
            edge = even(pane);
        }
        if edge == edges[pane] {
            return;
        }
        edges[pane] = edge;
        // The core lays the pages out as it takes the shares, not in the next frame.
        self.start_page_motion(cx);
        let evened = (0..=count).all(|index| edges[index] == even(index));
        let shares = edge_shares(&edges);
        self.browser.set_split_shares(self.selected, (!evened).then_some(shares.as_slice()), cx);
    }

    /// ⌃+: opens the command bar over a new pane right of the pages shown, which make room for it
    /// in the same frame (Arc). What it opens shows there, in a split view with the tab shown.
    pub(super) fn add_pane(&mut self, cx: &mut Context<Self>) {
        let with = self.selected;
        let count = self.panes(with).len();
        if with == 0 || count >= MAX_PANES {
            return;
        }
        self.adding_pane = Some(with);
        self.lay_out_page(cx);
        let (card, _) = self.card_frames(self.viewport);
        let pane = pane_frames(card, count + 1, self.stacked(with), &[])[count];
        self.browser.split_bar(with, pane, card, cx);
        cx.notify();
    }

    /// A tab menu's Add Split View: ⌃+ beside `tab`, which shows first if it isn't. Agents reach
    /// the same split through `bc_tab_split_with`.
    pub(super) fn add_split_to(&mut self, tab: u64, cx: &mut Context<Self>) {
        if self.panes(self.selected).contains(&tab) {
            return self.add_pane(cx);
        }
        self.split_pending = Some(tab);
        self.select(tab, cx);
    }

    /// Expand Current Split: the focused page takes the card and each other page keeps 220 pt, in
    /// the same frame (Arc). The shares are fractions of the card, so a window resized after keeps
    /// them in proportion, and the command pins the other pages again at the new width. Done
    /// again at the same width it asks the core for nothing.
    pub(super) fn expand_split(&mut self, cx: &mut Context<Self>) {
        let panes = self.panes(self.selected);
        let Some(focused) =
            panes.iter().position(|&tab| tab == self.selected).filter(|_| panes.len() > 1)
        else {
            return;
        };
        let (frame, _) = self.page_frame(self.viewport);
        let length = match self.stacked(self.selected) {
            true => frame.size.height,
            false => frame.size.width,
        };
        let shares = expanded_shares(panes.len(), focused, f32::from(length));
        let now = self.shares(self.selected);
        let now = if now.is_empty() { vec![1. / panes.len() as f64; panes.len()] } else { now };
        if shares.iter().zip(&now).all(|(share, now)| (share - now).abs() < 1e-6) {
            return;
        }
        // Even again, as the core keeps them, where the card is too short for more.
        let even = shares.iter().all(|share| (share - 1. / panes.len() as f64).abs() < 1e-6);
        self.browser.set_split_shares(self.selected, (!even).then_some(shares.as_slice()), cx);
    }

    /// Focuses the split view's page right of the focused one, or left of it, round the ends.
    pub(super) fn focus_pane(&mut self, forward: bool, cx: &mut Context<Self>) {
        let panes = self.panes(self.selected);
        let count = panes.len();
        let Some(at) = panes.iter().position(|&tab| tab == self.selected).filter(|_| count > 1)
        else {
            return;
        };
        let next = if forward { (at + 1) % count } else { (at + count - 1) % count };
        self.select(panes[next], cx);
    }
}

/// Where the page goes in a window whose content is `viewport`: in the card, whose left edge is at
/// `card_left`, or filling the window for a full-screen page, at 0. The Library pushes the card
/// `push` further right at the same width, past the window's edge.
pub fn page_frame(viewport: Size<Pixels>, card_left: f32, push: f32) -> Bounds<Pixels> {
    if card_left == 0. {
        return Bounds::new(point(px(0.), px(0.)), viewport);
    }
    Bounds::new(
        point(px(card_left + push), px(CARD_INSET)),
        size(
            viewport.width - px(card_left + CARD_INSET),
            viewport.height - px(2. * CARD_INSET),
        ),
    )
}

/// The pages of a split view of `count` in `frame`, in their order, where the core puts them:
/// SPLIT_GAP apart, side by side or stacked, as `pane_edges` has them.
pub(super) fn pane_frames(
    frame: Bounds<Pixels>,
    count: usize,
    stacked: bool,
    shares: &[f64],
) -> Vec<Bounds<Pixels>> {
    let edges = pane_edges(frame, count, stacked, shares);
    (0..count)
        .map(|index| {
            let (from, to) = (px(edges[index]), px(edges[index + 1] - SPLIT_GAP));
            match stacked {
                true => Bounds::from_corners(point(frame.left(), from), point(frame.right(), to)),
                false => Bounds::from_corners(point(from, frame.top()), point(to, frame.bottom())),
            }
        })
        .collect()
}

/// The shares of `count` pages across a card `length` long, or tall stacked, with the `focused`
/// one taking all but what the others keep: EXPANDED_OTHER each, no more than an even share, so
/// the focused page is never the smaller one on a card too short for both.
pub(super) fn expanded_shares(count: usize, focused: usize, length: f32) -> Vec<f64> {
    let span = f64::from(length + SPLIT_GAP);
    let other = f64::from(EXPANDED_OTHER + SPLIT_GAP).min(span / count as f64) / span;
    let mut shares = vec![other; count];
    shares[focused] = 1. - other * (count - 1) as f64;
    shares
}

/// The shares of a split view whose pages start at `edges`, with where one after them would last,
/// as `pane_edges` takes them.
pub(super) fn edge_shares(edges: &[f32]) -> Vec<f64> {
    let span = f64::from(edges[edges.len() - 1] - edges[0]);
    edges.windows(2).map(|pair| f64::from(pair[1] - pair[0]) / span).collect()
}

/// Where each of a split view's `count` pages starts across `frame`, or down it stacked, then where
/// one after them would, as the core has it: each page takes its share of the frame with the gap
/// after it, rounded to the point, else is as wide or as tall as the others to the point.
pub(super) fn pane_edges(
    frame: Bounds<Pixels>,
    count: usize,
    stacked: bool,
    shares: &[f64],
) -> Vec<f32> {
    let (start, length) = match stacked {
        true => (frame.origin.y, frame.size.height),
        false => (frame.origin.x, frame.size.width),
    };
    let span = f32::from(length) + SPLIT_GAP;
    let even = shares.len() != count;
    let mut before = 0.;
    (0..=count)
        .map(|index| {
            let edge = match even {
                true => (span * index as f32 / count as f32).floor(),
                false => (f64::from(span) * before).round() as f32,
            };
            before += shares.get(index).copied().unwrap_or(0.);
            f32::from(start) + edge
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::{edge_shares, expanded_shares, pane_edges, pane_frames};
    use crate::shell::{Command, Folders, Shell, Split, recorder::Recorder};
    use gpui::{
        AppContext, Bounds, Entity, MouseButton, MouseDownEvent, TestAppContext, point, px, size,
    };

    /// A Space whose Today holds `tabs`, all in one split view, with `shares` as the core kept
    /// them, on a host that records what the shell asks; `selected` is focused, in a window
    /// `width` wide.
    fn split_of(
        tabs: &[u64],
        selected: u64,
        shares: &[f64],
        width: f32,
        cx: &mut TestAppContext,
    ) -> (Entity<Shell>, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into())], cx);
            let split = Split { tabs: tabs.to_vec(), stacked: false, shares: shares.to_vec() };
            shell.set_tabs(1, Vec::new(), tabs.to_vec(), vec![split], Folders::default(), cx);
            shell.set_space(1, cx);
            shell.set_selected(selected, cx);
            shell.viewport = size(px(width), px(800.));
        });
        (shell, host)
    }

    /// What the core keeps after the shell's one call: its shares, none while they're even.
    fn asked(host: &Recorder) -> Vec<f64> {
        let calls = host.take();
        assert_eq!(calls.len(), 1, "{calls:?}");
        let shares = calls[0].split(' ').skip(3).collect::<Vec<_>>();
        let shares = shares.into_iter().filter(|share| *share != "even");
        shares.map(|share| share.parse().unwrap()).collect()
    }

    /// The core reports `shares`, as it does after the shell asks.
    fn kept(shell: &Entity<Shell>, tabs: &[u64], shares: &[f64], cx: &mut TestAppContext) {
        shell.update(cx, |shell, cx| {
            let split = Split { tabs: tabs.to_vec(), stacked: false, shares: shares.to_vec() };
            shell.set_tabs(1, Vec::new(), tabs.to_vec(), vec![split], Folders::default(), cx);
        });
    }

    /// The widths of the pages shown, left to right.
    fn widths(shell: &Entity<Shell>, count: usize, cx: &mut TestAppContext) -> Vec<f32> {
        shell.read_with(cx, |shell, _| {
            let (frame, _) = shell.page_frame(shell.viewport);
            let shares = shell.shares(shell.selected);
            let frames = pane_frames(frame, count, false, &shares);
            frames.iter().map(|frame| f32::from(frame.size.width)).collect()
        })
    }

    #[gpui::test]
    fn expanding_gives_the_focused_page_the_card_and_the_other_220(cx: &mut TestAppContext) {
        for width in [900., 1200., 1800.] {
            let (shell, host) = split_of(&[10, 11], 11, &[], width, cx);
            host.take();
            shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
            let shares = asked(&host);
            kept(&shell, &[10, 11], &shares, cx);
            let widths = widths(&shell, 2, cx);
            assert!((widths[0] - 220.).abs() < 1., "at {width} the other is {}", widths[0]);
            let (card, _) = shell.read_with(cx, |shell, _| shell.page_frame(shell.viewport));
            assert!((widths[0] + 10. + widths[1] - f32::from(card.size.width)).abs() < 1.);
        }
    }

    #[gpui::test]
    fn the_page_focused_is_the_one_that_takes_the_card(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10, 11, 12], 11, &[], 1400., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        kept(&shell, &[10, 11, 12], &asked(&host), cx);
        let widths = widths(&shell, 3, cx);
        assert!((widths[0] - 220.).abs() < 1. && (widths[2] - 220.).abs() < 1., "{widths:?}");
        assert!(widths[1] > 600., "{widths:?}");
    }

    #[gpui::test]
    fn expanding_again_changes_nothing(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10, 11], 10, &[], 1200., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        let shares = asked(&host);
        kept(&shell, &[10, 11], &shares, cx);
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        assert_eq!(host.take(), Vec::<String>::new());
        // After a quit the core hands the shares back with the split, and it's still done.
        let (shell, host) = split_of(&[10, 11], 10, &shares, 1200., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        assert_eq!(host.take(), Vec::<String>::new());
        assert!((widths(&shell, 2, cx)[1] - 220.).abs() < 1.);
    }

    /// A double-click on the gap evens the pages, which is the way out, and the command takes
    /// them back.
    #[gpui::test]
    fn a_double_click_on_the_gap_leaves_it(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10, 11], 11, &[], 1200., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        let shares = asked(&host);
        kept(&shell, &[10, 11], &shares, cx);
        shell.update(cx, |shell, cx| {
            let event = MouseDownEvent {
                button: MouseButton::Left,
                position: point(px(600.), px(300.)),
                modifiers: Default::default(),
                click_count: 2,
                first_mouse: false,
            };
            shell.press_gap(1, &event, cx);
        });
        assert_eq!(host.take(), ["split shares 11 even"]);
        kept(&shell, &[10, 11], &[], cx);
        let evened = widths(&shell, 2, cx);
        assert!((evened[0] - evened[1]).abs() <= 1., "{evened:?}");
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        assert_eq!(asked(&host), shares);
    }

    /// The shares are fractions of the card, so a resized window keeps the pages in proportion,
    /// and the command pins the other one at 220 again at the new width.
    #[gpui::test]
    fn expanding_after_a_resize_pins_the_other_page_at_the_new_width(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10, 11], 11, &[], 1600., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        let shares = asked(&host);
        kept(&shell, &[10, 11], &shares, cx);
        shell.update(cx, |shell, _| shell.viewport = size(px(1000.), px(800.)));
        assert!((widths(&shell, 2, cx)[0] - 220.).abs() > 20.);
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        kept(&shell, &[10, 11], &asked(&host), cx);
        assert!((widths(&shell, 2, cx)[0] - 220.).abs() < 1.);
    }

    /// On a card too short for 220 and a larger page, the pages are even, as the core keeps them.
    #[gpui::test]
    fn a_card_too_short_for_it_leaves_the_pages_even(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10, 11], 11, &[0.3, 0.7], 560., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        assert_eq!(host.take(), ["split shares 11 even"]);
        for (count, length) in [(2, 300.), (3, 500.), (3, 1400.)] {
            for focused in 0..count {
                let shares = expanded_shares(count, focused, length);
                assert!((shares.iter().sum::<f64>() - 1.).abs() < 1e-9);
                assert!(shares.iter().all(|share| *share <= shares[focused] + 1e-9));
            }
        }
    }

    #[gpui::test]
    fn a_page_alone_has_nothing_to_expand(cx: &mut TestAppContext) {
        let (shell, host) = split_of(&[10], 10, &[], 1200., cx);
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::ExpandSplit, "ui", cx));
        assert_eq!(host.take(), Vec::<String>::new());
        assert!(!shell.read_with(cx, |shell, _| shell.offers(Command::ExpandSplit)));
    }

    /// A dragged gap's shares put the pages back where it was dragged to, as the core lays them
    /// out with the same rounding, so the cards behind the pages line up with them.
    #[test]
    fn shares_from_a_dragged_gap_lay_the_pages_out_there() {
        for length in [700., 911., 1174., 1503.] {
            let frame = Bounds::new(point(px(238.), px(10.)), size(px(length), px(length)));
            for count in [2, 3] {
                for moved in [-157., -1., 1., 16., 233.] {
                    for stacked in [false, true] {
                        let mut edges = pane_edges(frame, count, stacked, &[]);
                        edges[1] += moved;
                        let shares = edge_shares(&edges);
                        assert_eq!(pane_edges(frame, count, stacked, &shares), edges);
                    }
                }
            }
        }
    }
}
