//! A split view's pages in the card, and the gaps between them, which drag to resize
//! them (docs/ux-spec.md › Split view).

use gpui::{
    AnyElement, Bounds, Context, CursorStyle, DispatchPhase, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Size, canvas, div, point, prelude::*, px, size,
};

use super::{CARD_INSET, MAX_PANES, PANE_LEAST, PANE_SNAP, SPLIT_GAP, Shell};

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
                        shell.update(cx, |shell, _| shell.pane_resize = None).ok();
                    }
                });
            },
        );
        gaps.push(listener.absolute().size_0().into_any_element());
        gaps
    }

    /// Where the pages of the split view shown start, and one after them would.
    fn shown_pane_edges(&self) -> Vec<f32> {
        let (frame, _) = self.page_frame(self.viewport);
        let count = self.panes(self.selected).len();
        pane_edges(frame, count, self.stacked(self.selected), &self.shares(self.selected))
    }

    fn press_gap(&mut self, pane: usize, event: &MouseDownEvent, cx: &mut Context<Self>) {
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

    /// Moves the start of the page being resized with the pointer, `at` across the window, or
    /// down it stacked, taking from the page before it or giving to it.
    fn drag_gap(&mut self, at: f32, cx: &mut Context<Self>) {
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
    use super::{edge_shares, pane_edges};
    use gpui::{Bounds, point, px, size};

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
