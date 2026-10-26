//! A page's question about permissions, such as a site asking for the camera: a card under the URL
//! field, over the sidebar's rows, until the user answers or the page stops asking. Chrome hangs
//! its prompt from the address bar's site icon, which in our layout is the sidebar's URL field.
//! With the sidebar hidden, the reveal brings it out with the question and keeps it out until the
//! question goes. The core keeps each tab's request while another tab is shown and asks again
//! once it's back.

use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, BoxShadow, Context, FontWeight, Pixels, SharedString, canvas, div, hsla, point,
    prelude::*, px, rgba, svg,
};

use super::{
    HOVERED, NAV, NAV_HOVERED, NAV_RADIUS, PLATE_INSET, Shell, TITLE, URL_FIELD, URL_FIELD_HEIGHT,
    URL_FIELD_TOP,
};

/// Ours, as Arc's wasn't recorded: under the URL field, as wide as it, with the URL field's
/// radius and plates. The text is the dialog's detail colour.
const GAP: f32 = 6.;
const PADDING: f32 = 12.;
const RADIUS: f32 = 12.;
const LINE: f32 = 18.;
const DETAIL: u32 = 0xffff_ffc7;
/// A line of white 0.1 inside its edge, as the dialog's card has.
const EDGE: u32 = 0xffff_ff1a;
const BUTTON_HEIGHT: f32 = 28.;
const BUTTON_GAP: f32 = 8.;
const BUTTONS_TOP: f32 = 12.;
const CLOSE: f32 = 22.;
const CLOSE_INSET: f32 = 7.;
const CLOSE_GLYPH: f32 = 12.;

/// What a tab's page asks, as the core reports it.
#[derive(Clone, PartialEq, Debug)]
pub struct Question {
    /// Who asks, as the user should see it.
    pub site: SharedString,
    /// What it asks for, each a phrase that follows "wants to", such as "Use your camera".
    pub permissions: Vec<SharedString>,
}

/// The user's answer to a page's question.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Answer {
    /// The site may, now and whenever it asks again.
    Allow,
    /// It may not, now or again.
    Block,
    /// Not now: it may ask again.
    Dismiss,
}

impl Shell {
    /// The core reports what `tab`'s page asks, or that it no longer asks (None).
    pub fn set_question(&mut self, tab: u64, question: Option<Question>, cx: &mut Context<Self>) {
        match question {
            Some(question) => self.questions.insert(tab, question),
            None => self.questions.remove(&tab),
        };
        // The question needs the sidebar, so a hidden one comes out with it.
        if tab == self.selected && self.question().is_some() && self.can_reveal() {
            self.set_revealed(true, cx);
        }
        cx.notify();
    }

    /// The question of the tab shown, if its page asks one.
    pub(super) fn question(&self) -> Option<&Question> {
        self.questions.get(&self.selected)
    }

    fn answer(&mut self, answer: Answer, cx: &mut Context<Self>) {
        let tab = self.selected;
        if self.questions.remove(&tab).is_some() {
            self.browser.answer_permission(tab, answer, cx);
            cx.notify();
        }
    }

    /// The question of the tab shown, as a card under the URL field.
    pub(super) fn question_card(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let Some(question) = self.question() else {
            // Drawn again, it's logged again.
            self.question_drawn.set(None);
            return None;
        };
        let button = |id: &'static str, label: &'static str, answer: Answer| {
            div()
                .id(id)
                .flex_1()
                .h(px(BUTTON_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .bg(rgba(URL_FIELD))
                .hover(|style| style.bg(rgba(HOVERED)))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(TITLE))
                .child(label)
                .on_click(cx.listener(move |shell, _, _, cx| shell.answer(answer, cx)))
        };
        let close = div()
            .id("question-close")
            .absolute()
            .top(px(CLOSE_INSET))
            .right(px(CLOSE_INSET))
            .size(px(CLOSE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            .on_click(cx.listener(|shell, _, _, cx| shell.answer(Answer::Dismiss, cx)))
            .child(svg().path("icons/x.svg").size(px(CLOSE_GLYPH)).text_color(rgba(NAV)));
        // check.sh clicks the buttons from where the card is, inside its edge, which depends on how
        // its text wraps.
        let tab = self.selected;
        let drawn = self.question_drawn.clone();
        let listener = canvas(
            move |bounds, _, _| logged_at(&drawn, tab, bounds),
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        Some(
            div()
                .id("question")
                .absolute()
                .top(px(URL_FIELD_TOP + URL_FIELD_HEIGHT + GAP))
                .left(px(PLATE_INSET))
                .w(px(self.width() - 2. * PLATE_INSET))
                .p(px(PADDING))
                .rounded(px(RADIUS))
                .bg(self.window_fill())
                .border_1()
                .border_color(rgba(EDGE))
                .shadow(vec![BoxShadow {
                    color: hsla(0., 0., 0., 0.35),
                    offset: point(px(0.), px(6.)),
                    blur_radius: px(18.),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .block_mouse_except_scroll()
                .text_size(px(13.))
                .line_height(px(LINE))
                .child(listener)
                .child(
                    div()
                        .pr(px(CLOSE + CLOSE_INSET - PADDING))
                        .text_color(rgba(TITLE))
                        .font_weight(FontWeight::MEDIUM)
                        // Kept together, so "to" doesn't wrap alone under a long site.
                        .child(format!("{} wants\u{a0}to", question.site)),
                )
                .children(question.permissions.iter().map(|permission| {
                    div().text_color(rgba(DETAIL)).child(permission.clone())
                }))
                .child(
                    div()
                        .mt(px(BUTTONS_TOP))
                        .flex()
                        .gap(px(BUTTON_GAP))
                        .child(button("question-block", "Block", Answer::Block))
                        .child(button("question-allow", "Allow", Answer::Allow)),
                )
                .child(close),
        )
    }
}

/// Logs where the question's card is in its window when that changes.
fn logged_at(drawn: &Rc<Cell<Option<Bounds<Pixels>>>>, tab: u64, bounds: Bounds<Pixels>) {
    if drawn.get() == Some(bounds) {
        return;
    }
    drawn.set(Some(bounds));
    // check.sh reads this line.
    log!(
        "browser: question {tab} at {:.0} {:.0} {:.0} {:.0}",
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height)
    );
}
