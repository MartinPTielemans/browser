//! A note that a page needs protected-media playback Chromium doesn't have, such as Netflix's video:
//! a card under the URL field, where a page's question shows, until the tab goes to another site.
//! Safari's engine plays these sites with Apple's DRM, so its one button shows the tab's page there,
//! in the tab, and the profile keeps the site for it: its pages open there from then on, with no
//! note, until the Site Controls turn it off (decision 1). Where the Mac has no engine to give, on
//! macOS 13, and in an agent's Space, whose tabs never switch, it opens the page in Safari instead
//! and leaves the tab as it is. Its ×, shown under the pointer, and escape while the UI has the
//! keyboard close it for the tab until the tab goes to another site, and a page there asking brings
//! it back; the site's own error stays in the page. Agents get no path: it hands the user's page to
//! another engine or app. The core reports it (tab_media_unsupported).

use gpui::{App, Context, FontWeight, KeyBinding, SharedString, actions, div, prelude::*, px, rgba};

use super::{
    KEY_CONTEXT, Shell, ShellHandle, TITLE,
    permission::{BESIDE_CLOSE, BUTTONS_TOP, card_button, card_close, card_listener},
};

/// Ours, as neither Arc nor Dia has protected media to explain: short and plain. It says the video
/// plays here, in this tab, and warns of the sign-in Safari's engine asks for once, as it keeps its
/// own cookies.
const SWITCH_WORDS: &str =
    "This site's video needs Safari's engine. Play it in this tab, and the site may ask you to sign \
     in once more.";
const SWITCH: &str = "Play in This Tab";
const WORDS: &str =
    concat!("This site's video needs a module ", env!("APP_NAME"), " doesn't have yet.");
const OPEN: &str = "Open in Safari";
/// A private window's page leaves it for Safari's own window, which keeps history.
const OPEN_PRIVATE: &str = "Open in Safari, not private";

actions!(drm_note, [CloseNote]);

/// Escape while the UI has the keyboard, over Stop's, which it falls through to unless it closes
/// the note. The page keeps its own escape, which reaches Stop if it hands it back.
pub(super) fn escape_binding() -> KeyBinding {
    KeyBinding::new("escape", CloseNote, Some(KEY_CONTEXT))
}

pub(super) fn bind(cx: &mut App) {
    cx.bind_keys([escape_binding()]);
    cx.on_action(|_: &CloseNote, cx| {
        let shell = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone());
        let closed = shell.and_then(|shell| shell.update(cx, Shell::escape_drm_note).ok());
        if !closed.unwrap_or(false) {
            cx.propagate();
        }
    });
}

impl Shell {
    /// The core reports the protected-media key system `tab`'s page asked for that ilo can't play,
    /// or that there's no note any more (empty).
    pub fn set_media_unsupported(&mut self, tab: u64, key_system: &str, cx: &mut Context<Self>) {
        let changed = match key_system.is_empty() {
            // The tab is at another site, or in Safari's engine: a note there is a new one.
            true => self.drm_closed.remove(&tab) | self.drm_notes.remove(&tab).is_some(),
            false => self.drm_notes.insert(tab, key_system.to_owned().into()).as_deref()
                != Some(key_system),
        };
        if changed {
            cx.notify();
        }
    }

    /// The key system behind the note of the tab shown, if its page needs one and the user hasn't
    /// closed it. A page's question takes the note's place until it's answered, as both hang from
    /// the same place.
    pub(super) fn drm_note(&self) -> Option<&SharedString> {
        let playing = self.page(self.selected).system_engine;
        let hidden = playing || self.drm_closed.contains(&self.selected);
        self.drm_notes.get(&self.selected).filter(|_| !hidden && self.question().is_none())
    }

    /// Closes the note of the tab shown, until the tab goes to another site.
    fn close_drm_note(&mut self, cx: &mut Context<Self>) {
        if self.drm_note().is_none() {
            return;
        }
        // check.sh reads this line.
        log!("browser: drm note {} closed", self.selected);
        self.drm_closed.insert(self.selected);
        cx.notify();
    }

    /// Escape closes the note when it's what the sidebar shows in front, and says whether it did.
    fn escape_drm_note(&mut self, cx: &mut Context<Self>) -> bool {
        let shown = !self.sidebar_hidden || self.revealed;
        let in_front = shown && self.library.is_none() && self.form().is_none();
        if !in_front || self.drm_note().is_none() {
            return false;
        }
        self.close_drm_note(cx);
        true
    }

    /// Whether the note's button shows the page in Safari's engine, in the tab: where the Mac has
    /// one, and not in an agent's Space, whose tabs never switch.
    fn can_switch(&self) -> bool {
        self.browser.can_use_system_engine() && !self.agents_space_shown()
    }

    /// Shows the page of the tab shown in Safari's engine. The core takes the note away.
    fn use_system_engine(&mut self, cx: &mut Context<Self>) {
        if self.drm_note().is_none() || !self.can_switch() {
            return;
        }
        self.browser.use_system_engine(self.selected, cx);
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
            if self.drm_drawn.take().is_some() {
                // check.sh waits for this line.
                log!("browser: drm note hidden");
            }
            return None;
        }
        let switch = self.can_switch();
        // Where neither is, the note says why and offers nothing.
        let button = if switch {
            Some(
                card_button("drm-note-switch", SWITCH)
                    .on_click(cx.listener(|shell, _, _, cx| shell.use_system_engine(cx))),
            )
        } else {
            self.browser.can_open_in_safari().then(|| {
                let label = if self.private { OPEN_PRIVATE } else { OPEN };
                card_button("drm-note-safari", label)
                    .on_click(cx.listener(|shell, _, _, cx| shell.open_in_safari(cx)))
            })
        };
        let words = if switch { SWITCH_WORDS } else { WORDS };
        // Shown under the pointer, as a row's ×, and always to VoiceOver's cursor, which is no
        // pointer. It takes clicks either way, as the pointer is over the card to click it.
        let close = card_close("drm-note-close")
            .when(!self.a11y, |close| {
                close.opacity(0.).group_hover("drm-note", |style| style.opacity(1.))
            })
            .on_click(cx.listener(|shell, _, _, cx| shell.close_drm_note(cx)));
        Some(
            self.plate("drm-note")
                .group("drm-note")
                .child(card_listener("drm note", &self.drm_drawn, self.selected))
                .child(
                    div()
                        .pr(px(BESIDE_CLOSE))
                        .text_color(rgba(TITLE))
                        .font_weight(FontWeight::MEDIUM)
                        .child(words),
                )
                .children(button.map(|button| div().mt(px(BUTTONS_TOP)).flex().child(button)))
                .child(close),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{AppContext, TestAppContext};

    use crate::shell::{Folders, Page, Question, Shell, bind_keys, recorder::Recorder};

    fn window_with_tabs(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into())], cx);
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
    fn the_button_shows_the_tabs_page_in_safaris_engine_where_the_mac_has_it(
        cx: &mut TestAppContext,
    ) {
        let (shell, host) = window_with_tabs(cx);
        host.system_engine.set(true);
        shell.update(cx, |shell, cx| {
            // Without a note, there's nothing to switch.
            shell.use_system_engine(cx);
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            host.take();
            shell.use_system_engine(cx);
        });
        assert_eq!(host.take(), ["system engine 10"]);
    }

    #[gpui::test]
    fn a_tab_in_safaris_engine_has_no_note(cx: &mut TestAppContext) {
        let (shell, host) = window_with_tabs(cx);
        host.system_engine.set(true);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            let page = Page { system_engine: true, ..shell.page(10) };
            shell.set_page(10, page, cx);
            assert!(shell.drm_note().is_none());
        });
    }

    #[gpui::test]
    fn without_safaris_engine_the_button_switches_nothing(cx: &mut TestAppContext) {
        let (shell, host) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            host.take();
            shell.use_system_engine(cx);
        });
        assert!(host.take().is_empty());
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

    #[gpui::test]
    fn closing_the_note_hides_it_for_that_tab_only(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            shell.set_media_unsupported(11, "com.widevine.alpha", cx);
            shell.close_drm_note(cx);
            assert!(shell.drm_note().is_none());
            shell.set_selected(11, cx);
            assert!(shell.drm_note().is_some(), "closing one tab's note closed another's");
            shell.set_selected(10, cx);
            assert!(shell.drm_note().is_none(), "the note came back with its tab");
        });
    }

    #[gpui::test]
    fn the_site_asking_again_leaves_the_note_closed(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            shell.close_drm_note(cx);
            // Another of the site's pages asks for another system, while the tab is still there.
            shell.set_media_unsupported(10, "com.microsoft.playready", cx);
            assert!(shell.drm_note().is_none());
        });
    }

    #[gpui::test]
    fn the_note_comes_back_once_the_tab_has_been_at_another_site(cx: &mut TestAppContext) {
        let (shell, _) = window_with_tabs(cx);
        shell.update(cx, |shell, cx| {
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            shell.close_drm_note(cx);
            // The core takes the note back as the tab leaves the site, and reports it again once a
            // page there asks.
            shell.set_media_unsupported(10, "", cx);
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
            assert!(shell.drm_note().is_some());
        });
    }

    /// Escape with the UI focused closes the note in front, and with none it stops, as before:
    /// here, in full screen with nothing loading, it leaves full screen.
    #[gpui::test]
    fn escape_closes_the_note_and_otherwise_stops(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        cx.update(bind_keys);
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into())], cx);
            shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
            shell.set_space(1, cx);
            let page = Page { url: "https://www.netflix.com/browse".into(), ..Page::default() };
            shell.set_page(10, page, cx);
            shell.set_selected(10, cx);
            shell.set_full_screen(true, cx);
            shell.set_media_unsupported(10, "com.widevine.alpha", cx);
        });
        cx.update(|window, cx| window.focus(&shell.read(cx).focus_handle.clone(), cx));
        host.take();
        cx.simulate_keystrokes("escape");
        shell.read_with(cx, |shell, _| assert!(shell.drm_note().is_none()));
        assert!(!host.take().contains(&"window button zoom".to_string()), "escape also stopped");
        cx.simulate_keystrokes("escape");
        assert!(host.take().contains(&"window button zoom".to_string()));
    }
}
