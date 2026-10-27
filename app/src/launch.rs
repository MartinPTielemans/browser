//! The first window at launch. AppKit hands over the links the launch carried (a link clicked in
//! Mail, `open https://…`) only as it finishes launching the app, which comes after the browser
//! has started and opened its first window. So that window opens as the browser starts, as it does
//! without a link, but goes on screen only once AppKit has finished, with the links' pages already
//! shown over the Spaces it brought back, and once the UI has drawn a frame: its first frame on
//! screen shows the link's page, never the restored one. The core shows a selected tab's page at
//! once, so the UI needn't draw again for it. AppKit finishes whether or not a link came, a few ms
//! after the UI's first frame, so a launch without one waits for no link.

pub struct Launch<W> {
    finished: bool,
    /// A frame has begun, and so has been drawn by the time anything else runs.
    began: bool,
    drawn: bool,
    /// The first window, held off screen.
    window: Option<W>,
    /// The links other apps sent before the first window opened; None after.
    links: Option<Vec<String>>,
}

impl<W> Launch<W> {
    pub const fn new() -> Self {
        Self { finished: false, began: false, drawn: false, window: None, links: Some(Vec::new()) }
    }

    /// Holds the first window off screen until it can show.
    pub fn hold(&mut self, window: W) {
        self.window = Some(window);
    }

    /// AppKit has finished launching the app, after handing over its links. The first window to
    /// show now, if it can.
    pub fn finished(&mut self) -> Option<W> {
        self.finished = true;
        // The frame drew as it began, and AppKit's notice and the links never come inside it.
        self.drawn |= self.began;
        self.release()
    }

    /// The UI has begun a frame.
    pub fn began(&mut self) {
        self.began = true;
    }

    /// The UI has drawn a frame. The first window to show now, if it can.
    pub fn drawn(&mut self) -> Option<W> {
        self.drawn = true;
        self.release()
    }

    /// The first window to show now whatever it waits for, as one that isn't visible draws no
    /// frames.
    pub fn release_now(&mut self) -> Option<W> {
        self.window.take()
    }

    fn release(&mut self) -> Option<W> {
        if self.finished && self.drawn { self.window.take() } else { None }
    }

    /// Links another app sent: kept for the first window while it hasn't opened, else given back
    /// to open in a window now.
    pub fn links(&mut self, urls: Vec<String>) -> Option<Vec<String>> {
        match &mut self.links {
            Some(links) => {
                links.extend(urls);
                None
            }
            None => Some(urls),
        }
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
    fn the_window_waits_for_appkit_to_hand_over_the_links_after_its_first_frame() {
        let mut launch = held();
        assert_eq!(launch.drawn(), None, "AppKit could still hand over a link");
        let link = urls(&["https://a.test/"]);
        assert_eq!(launch.links(link.clone()), Some(link), "it opens in the window, off screen");
        assert_eq!(launch.finished(), Some("window"));
        assert_eq!(launch.release_now(), None);
    }

    #[test]
    fn appkit_finishing_first_waits_for_the_first_frame() {
        let mut launch = held();
        assert_eq!(launch.finished(), None, "nothing drawn yet");
        assert_eq!(launch.drawn(), Some("window"));
    }

    #[test]
    fn links_after_a_frame_began_show_it_at_once() {
        let mut launch = held();
        launch.began();
        assert_eq!(launch.finished(), Some("window"), "the frame was drawn before the links came");
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
