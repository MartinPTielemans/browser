//! Telling the core while the page's frame moves on its own, so each step waits for the page to
//! draw at its new size before it goes to the screen, and the edge that doesn't move, with the
//! scrollbar, stays still (issue #166).

use gpui::Context;

use super::Shell;

/// Whether the core was told the page's frame is moving.
#[derive(Default)]
pub(super) struct PageMotion {
    told: bool,
}

impl PageMotion {
    /// Whether to tell the core the page starts moving, before it's laid out for a frame in which
    /// it's `moving`, so the motion's first resize stretches.
    fn starts(&mut self, moving: bool) -> bool {
        let starts = moving && !self.told;
        self.told |= starts;
        starts
    }

    /// Whether to tell the core the page stopped, after it's laid out for a frame in which it's
    /// not `moving`: that layout puts the page at rest, and is the motion's last step.
    fn stops(&mut self, moving: bool) -> bool {
        let stops = !moving && self.told;
        self.told &= !stops;
        stops
    }
}

impl Shell {
    /// Whether the card's left edge is on its way, which resizes the page at every step: the
    /// sidebar hiding or showing, its edge dragged, or its width settling. The Library pushes
    /// the page without resizing it, and the reveal goes over it.
    fn page_moving(&self) -> bool {
        self.sidebar_motion.is_some() || self.width_motion.is_some() || self.resize.is_some()
    }

    /// Lays the page out for this frame, with the core told when the page starts and stops moving.
    /// A second ⌘S mid-motion, or a drag after a settle, keeps it moving without a new start.
    pub(super) fn lay_out_moving_page(&mut self, cx: &mut Context<Self>) {
        let moving = self.page_moving();
        if self.page_motion.starts(moving) {
            let views = self.browser.set_page_moving(true, cx);
            // check.sh waits for these lines.
            log!("browser: page moving {views}");
        }
        self.lay_out_page(cx);
        if self.page_motion.stops(moving) {
            let views = self.browser.set_page_moving(false, cx);
            log!("browser: page still {views}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageMotion;

    /// What the core hears over frames in which the page is moving or not, by frame: true before
    /// the frame's layout, false after it.
    fn heard(frames: &[bool]) -> Vec<(usize, bool)> {
        let mut motion = PageMotion::default();
        let mut heard = Vec::new();
        for (frame, &moving) in frames.iter().enumerate() {
            if motion.starts(moving) {
                heard.push((frame, true));
            }
            if motion.stops(moving) {
                heard.push((frame, false));
            }
        }
        heard
    }

    /// ⌘S: the core hears once that the page moves, before the first step, and once that it
    /// stopped, after the layout that puts it at rest.
    #[test]
    fn a_motion_starts_once_and_stops_once() {
        assert_eq!(heard(&[false, true, true, true, false, false]), [(1, true), (4, false)]);
    }

    /// A second ⌘S mid-motion turns it around without a stop between, and it still stops at the
    /// end; a motion after one ended starts again.
    #[test]
    fn an_interrupted_motion_still_stops() {
        let frames = [true, true, /* ⌘S again */ true, true, false, true, false];
        assert_eq!(heard(&frames), [(0, true), (4, false), (5, true), (6, false)]);
    }

    /// Frames at rest, as a resize of the window, tell the core nothing.
    #[test]
    fn a_page_at_rest_tells_nothing() {
        assert!(heard(&[false, false, false]).is_empty());
    }
}
