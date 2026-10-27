//! A note that a page needs protected-media playback ilo doesn't have, such as Netflix's video: a
//! card under the URL field, where a page's question shows, until the tab goes to another site.
//! Safari plays these sites with Apple's DRM, so its one button opens the tab's page there and
//! leaves the tab as it is. There's nothing to dismiss: the site's own error stays in the page, and
//! this says why. Agents get no path: it hands the user's page to another app. The core reports it
//! (tab_media_unsupported), and a later step can answer the same report with a download of Widevine.

use gpui::{Context, FontWeight, SharedString, div, prelude::*, px, rgba};

use super::{
    Shell, TITLE,
    permission::{BUTTONS_TOP, card_button, card_listener},
};

/// Ours, as neither Arc nor Dia has protected media to explain: short and plain.
const WORDS: &str =
    concat!("This site's video needs a module ", env!("APP_NAME"), " doesn't have yet.");
const OPEN: &str = "Open in Safari";
/// A private window's page leaves it for Safari's own window, which keeps history.
const OPEN_PRIVATE: &str = "Open in Safari, not private";

impl Shell {
    /// The core reports the protected-media key system `tab`'s page asked for that ilo can't play,
    /// or that there's no note any more (empty).
    pub fn set_media_unsupported(&mut self, tab: u64, key_system: &str, cx: &mut Context<Self>) {
        let changed = match key_system.is_empty() {
            true => self.drm_notes.remove(&tab).is_some(),
            false => self.drm_notes.insert(tab, key_system.to_owned().into()).as_deref()
                != Some(key_system),
        };
        if changed {
            cx.notify();
        }
    }

    /// The key system behind the note of the tab shown, if its page needs one. A page's question
    /// takes the note's place until it's answered, as both hang from the same place.
    pub(super) fn drm_note(&self) -> Option<&SharedString> {
        self.drm_notes.get(&self.selected).filter(|_| self.question().is_none())
    }

    /// Opens the page of the tab shown in Safari. The tab keeps its page, and its note.
    fn open_in_safari(&mut self, cx: &mut Context<Self>) {
        if self.drm_note().is_none() {
            return;
        }
        let url = self.page(self.selected).url;
        self.browser.open_in_safari(&url, cx);
    }

    /// The note of the tab shown, as a card under the URL field.
    pub(super) fn drm_note_card(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if self.drm_note().is_none() {
            // Drawn again, it's logged again.
            self.drm_drawn.set(None);
            return None;
        }
        // Where Safari isn't, the note says why and offers nothing.
        let button = self.browser.can_open_in_safari().then(|| {
            let label = if self.private { OPEN_PRIVATE } else { OPEN };
            div().mt(px(BUTTONS_TOP)).flex().child(
                card_button("drm-note-safari", label)
                    .on_click(cx.listener(|shell, _, _, cx| shell.open_in_safari(cx))),
            )
        });
        Some(
            self.plate("drm-note")
                .child(card_listener("drm note", &self.drm_drawn, self.selected))
                .child(div().text_color(rgba(TITLE)).font_weight(FontWeight::MEDIUM).child(WORDS))
                .children(button),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{AppContext, TestAppContext};

    use crate::shell::{Folders, Page, Question, Shell, recorder::Recorder};

    fn window_with_tabs(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            let space = (1, "Space 1".into(), None, 0, "".into(), "".into(), "".into());
            shell.set_spaces(vec![space], cx);
            shell.set_tabs(1, Vec::new(), vec![10, 11], Vec::new(), Folders::default(), cx);
            shell.set_space(1, cx);
            let page = |url: &str| Page { url: url.to_owned().into(), ..Page::default() };
            shell.set_page(10, page("https://www.netflix.com/browse"), cx);
            shell.set_page(11, page("https://example.com/"), cx);
            shell.set_selected(10, cx);
        });
        (shell, host)
    }

    #[gpui::test]
    fn the_note_shows_for_the_tab_reported_and_only_that_tab(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            assert!(shell.drm_note().is_none());
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            let noted = shell.drm_note().map(|key_system| key_system.to_string());
            assert_eq!(noted.as_deref(), Some("com.widevine.alpha"));
            // Another tab shown has none, and the first has it again once it's back.
            shell.set_selected(11, cx);
            assert!(shell.drm_note().is_none());
            shell.set_selected(10, cx);
            assert!(shell.drm_note().is_some());
        });
    }

    #[gpui::test]
    fn the_note_goes_when_the_core_clears_it(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            // Clearing a tab that has none changes nothing.
            shell.set_media_unsupported(11, "", cx);
            assert!(shell.drm_note().is_some());
            shell.set_media_unsupported(10, "", cx);
            assert!(shell.drm_note().is_none());
        });
    }

    #[gpui::test]
    fn a_page_question_takes_the_notes_place_until_answered(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            let question = Question { site: "netflix.com".into(), permissions: Vec::new() };
            shell.set_question(10, Some(question), cx);
            assert!(shell.drm_note().is_none());
            shell.set_question(10, None, cx);
            assert!(shell.drm_note().is_some());
        });
    }

    #[gpui::test]
    fn the_button_asks_the_host_to_open_the_tabs_page_in_safari(cx: &mut TestAppContext) {
        let (shell, host) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            // Without a note, there's nothing to open.
            shell.open_in_safari(cx);
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            host.take();
            shell.open_in_safari(cx);
        });
        assert_eq!(host.take(), ["open in safari https://www.netflix.com/browse"]);
        // The tab keeps its page and its note.
        shell.update(cx, |shell, _| assert!(shell.drm_note().is_some()));
    }
}
