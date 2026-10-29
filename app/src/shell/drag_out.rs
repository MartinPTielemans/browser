//! A tab's row dragged out of its window (docs/ux-spec.md › Dragging out of the list): onto
//! another window's list, which marks where it would drop, or anywhere else.

use gpui::{Bounds, Context, Pixels, Point, px};

use super::{Place, Shell};

impl Shell {
    /// The dragged tab is at `position` in the window. Past the window's edge it's out, for another
    /// window or one of its own: its ghost waits where it left the list, the line and the page's
    /// targets go, and the browser has the list of the window under it mark where it would drop.
    /// Returns whether it's out. A folder, a favorite and a full-screen or private window's tab stay.
    pub(super) fn drag_out(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let tab = drag.tab;
        let leaves = matches!(drag.from, Place::List(..))
            && !self.is_folder(tab)
            && !self.full_screen
            && !self.private;
        let window = Bounds::new(Point::default(), self.viewport);
        // A window not yet drawn has no edge to pass.
        let out = leaves && self.viewport.width > px(0.) && !window.contains(&position);
        let was = drag.out.is_some();
        if !out {
            if was {
                if let Some(drag) = &mut self.drag {
                    drag.out = None;
                }
                // check.sh waits for this line.
                log!("browser: dragged {tab} back in");
                self.browser.drag_tab_out(tab, None, cx);
            }
            return false;
        }
        let Some(drag) = &mut self.drag else {
            return false;
        };
        drag.moving = true;
        drag.out = Some(position);
        drag.line = None;
        if drag.over_page.take().is_some() {
            log!("browser: drop targets gone");
        }
        if !was {
            // check.sh waits for this line.
            log!("browser: dragged {tab} out");
        }
        self.browser.drag_tab_out(tab, Some(position), cx);
        cx.notify();
        true
    }

    /// The pointer let go of a tab out of the window: the browser takes it where it was let go.
    /// Returns whether it was out.
    pub(super) fn drop_out(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((tab, at)) = self.drag.as_ref().and_then(|drag| Some((drag.tab, drag.out?))) else {
            return false;
        };
        // The release that ends the drag isn't a click.
        self.dragged = true;
        self.browser.drop_tab_out(tab, at, cx);
        self.settle_drag();
        cx.notify();
        true
    }

    /// Another window's tab, dragged out of its list, is at `position` in this window: over the
    /// list, a line marks where it would drop, as for links dragged in. Returns whether it's over
    /// the list.
    pub fn drag_tab_in(&mut self, tab: u64, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        self.drag_over_list(tab, position, cx)
    }

    /// Another window's tab was let go at `position` in this window. Over the list, its row lands
    /// where the line marks once the browser reports it there, and this returns where: in the
    /// pinned tabs or Today, in the folder, else 0, at the index, counted as `Browser::move_tab`
    /// counts.
    pub fn drop_tab_in(
        &mut self,
        tab: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> Option<(bool, u64, usize)> {
        if !self.drag_over_list(tab, position, cx) {
            return None;
        }
        let place = self.incoming_place();
        match (place, &mut self.drag) {
            // Let go at its own row, which the browser reports no move for.
            (Some(_), Some(drag)) if drag.to != drag.from || drag.depth != drag.from_depth => {
                drag.dropped = true;
            }
            _ => self.settle_drag(),
        }
        if let Some((pinned, folder, index)) = place {
            // check.sh waits for these lines.
            match (pinned, folder) {
                (false, _) => log!("browser: dropped tab {tab} in today {index}"),
                (true, 0) => log!("browser: dropped tab {tab} in pinned {index}"),
                (true, _) => log!("browser: dropped tab {tab} in folder {folder} {index}"),
            }
        }
        cx.notify();
        place
    }
}

#[cfg(test)]
mod tests;
