//! The first window at launch. AppKit hands over the links the launch carried (a link clicked in
//! Mail, `open https://…`) only as it finishes launching the app, which comes after the browser
//! has started and opened its first window. So that window opens as the browser starts, as it does
//! without a link, but stays off screen until AppKit has finished and the UI has drawn a frame
//! with the links open over the Spaces it brought back: its first frame on screen shows them,
//! never the restored page. AppKit finishes whether or not a link came, a few ms after the UI's
//! first frame, so a launch without one waits for no link.

pub struct Launch<W> {
    finished: bool,
    /// Counts the links opened in the window while it's held, so a frame drawn before one isn't
    /// taken for a frame that shows it.
    changes: u64,
    /// The `changes` the last frame drawn showed.
    drawn: Option<u64>,
    /// The first window, held off screen.
    window: Option<W>,
    /// The links other apps sent before the first window opened; None after.
    links: Option<Vec<String>>,
}

impl<W> Launch<W> {
    pub const fn new() -> Self {
        Self { finished: false, changes: 0, drawn: None, window: None, links: Some(Vec::new()) }
    }

    /// Holds the first window off screen until it can show.
    pub fn hold(&mut self, window: W) {
        self.window = Some(window);
    }

    pub fn holding(&self) -> bool {
        self.window.is_some()
    }

    /// AppKit has finished launching the app, after handing over its links. The first window to
    /// show now, if it can.
    pub fn finished(&mut self) -> Option<W> {
        self.finished = true;
        self.release()
    }

    /// What a frame starting now draws, for `drawn` once it has.
    pub fn frame(&self) -> u64 {
        self.changes
    }

    /// The UI has drawn the `frame` it started. The first window to show now, if it can.
    pub fn drawn(&mut self, frame: u64) -> Option<W> {
        self.drawn = Some(frame);
        self.release()
    }

    /// The first window to show now whatever it waits for, as one that isn't visible draws no
    /// frames.
    pub fn release_now(&mut self) -> Option<W> {
        self.window.take()
    }

    fn release(&mut self) -> Option<W> {
        let current = self.drawn == Some(self.changes);
        if self.finished && current { self.window.take() } else { None }
    }

    /// Links another app sent: kept for the first window while it hasn't opened, else given back
    /// to open in a window now, with `opened` once they have.
    pub fn links(&mut self, urls: Vec<String>) -> Option<Vec<String>> {
        match &mut self.links {
            Some(links) => {
                links.extend(urls);
                None
            }
            None => Some(urls),
        }
    }

    /// Links have opened in a window, which the held one waits to draw.
    pub fn opened(&mut self) {
        self.changes += 1;
    }

    /// The links for the first window, which takes them as it opens. Later ones go to a window.
    pub fn take_links(&mut self) -> Vec<String> {
        self.links.take().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(urls: &[&str]) -> Vec<String> {
        urls.iter().map(|url| url.to_string()).collect()
    }

    /// A launch whose first window has opened and is held.
    fn held() -> Launch<&'static str> {
        let mut launch = Launch::new();
        assert!(launch.take_links().is_empty());
        launch.hold("window");
        launch
    }

    #[test]
    fn without_a_link_the_window_shows_once_appkit_has_finished_and_a_frame_is_drawn() {
        let mut launch = held();
        let frame = launch.frame();
        assert_eq!(launch.drawn(frame), None, "AppKit could still hand over a link");
        assert_eq!(launch.finished(), Some("window"));
        assert!(!launch.holding());
    }

    #[test]
    fn appkit_finishing_first_waits_for_the_first_frame() {
        let mut launch = held();
        assert_eq!(launch.finished(), None, "nothing drawn yet");
        let frame = launch.frame();
        assert_eq!(launch.drawn(frame), Some("window"));
    }

    #[test]
    fn a_launch_link_waits_for_a_frame_drawn_after_it_opened() {
        let mut launch = held();
        // A frame starts, and the link opens before the frame's drawn is heard.
        let before = launch.frame();
        let link = urls(&["https://a.test/"]);
        assert_eq!(launch.links(link.clone()), Some(link), "it opens in the window, off screen");
        launch.opened();
        assert_eq!(launch.finished(), None);
        assert_eq!(launch.drawn(before), None, "that frame was drawn without the link");
        let after = launch.frame();
        assert_eq!(launch.drawn(after), Some("window"));
    }

    #[test]
    fn a_window_that_draws_no_frames_still_shows() {
        let mut launch = held();
        assert_eq!(launch.release_now(), Some("window"));
        assert_eq!(launch.finished(), None);
    }

    #[test]
    fn links_before_the_first_window_opens_are_its_own() {
        let mut launch = Launch::<&str>::new();
        assert_eq!(launch.links(urls(&["https://a.test/"])), None);
        assert_eq!(launch.links(urls(&["https://b.test/"])), None);
        assert_eq!(launch.take_links(), urls(&["https://a.test/", "https://b.test/"]));
        assert!(launch.take_links().is_empty());
    }
}
