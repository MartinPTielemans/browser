//! The dialog that asks before something hard to take back, such as deleting a Space with its
//! tabs, or before quitting: a card over the window, which dims, as Arc's (docs/ux-spec.md ›
//! Spaces › Renaming and deleting, and Warn before quitting). GPUI can't draw over the page's
//! native view, so it's a window of its own that the host shows over all of the browser window,
//! as its child, with the keyboard. What it asks of its window goes through `Host`. It also asks
//! Chrome's questions, such as whether to add an extension, which wait their turn behind the
//! dialog that shows and which Chrome can take back.

use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, BoxShadow, Context, Div, FocusHandle, FontWeight, Global, Hsla, Image,
    KeyBinding, MouseButton, SharedString, Stateful, Task, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions,
    div, hsla, img, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::{command_bar, shell::cubic_bezier};

actions!(dialog, [Confirm, Cancel]);

use crate::tokens::dialog::WIDTH;
use crate::tokens::dialog::RADIUS;
use crate::tokens::dialog::TEXT_INSET;
use crate::tokens::dialog::BUTTON_INSET;
use crate::tokens::dialog::TITLE_LINE;
use crate::tokens::dialog::DETAIL_TOP;
use crate::tokens::dialog::DETAIL_LINE;
use crate::tokens::dialog::ICON;
use crate::tokens::dialog::ICON_GAP;
use crate::tokens::dialog::LIST_TOP;
use crate::tokens::dialog::LIST_GAP;
use crate::tokens::dialog::SUBLINE;
use crate::tokens::dialog::SUBLINE_INDENT;
use crate::tokens::dialog::SUBLINE_TEXT;
use crate::tokens::dialog::BUTTONS_TOP;
use crate::tokens::dialog::BUTTONS_UNDER_TITLE;
use crate::tokens::dialog::ALSO_GAP;
use crate::tokens::dialog::FILL;
use crate::tokens::dialog::EDGE;
use crate::tokens::dialog::DETAIL;
use crate::tokens::dialog::DIM;
use crate::tokens::dialog::DIM_OPACITY;
use crate::tokens::dialog::BUTTON_HEIGHT;
use crate::tokens::dialog::BUTTON_RADIUS;
use crate::tokens::dialog::LABEL_X;
use crate::tokens::dialog::CHIP_END;
use crate::tokens::dialog::CHIP_GAP;
use crate::tokens::dialog::CANCEL_FILL;
use crate::tokens::dialog::CANCEL_HOVERED;
use crate::tokens::dialog::CANCEL_BORDER;
use crate::tokens::dialog::CANCEL_TEXT;
use crate::tokens::dialog::ACTION;
use crate::tokens::dialog::ACTION_HOVERED;
use crate::tokens::dialog::ACTION_LEDGE;
use crate::tokens::dialog::SAFE_ACTION;
use crate::tokens::dialog::SAFE_ACTION_HOVERED;
use crate::tokens::dialog::SAFE_ACTION_LEDGE;
use crate::tokens::dialog::ACCENT_HOVERED;
use crate::tokens::dialog::CHIP_HEIGHT;
use crate::tokens::dialog::CHIP_RADIUS;
use crate::tokens::dialog::ESC_FILL;
use crate::tokens::dialog::ESC_TEXT;
use crate::tokens::dialog::RETURN_WIDTH;
use crate::tokens::dialog::RETURN_FILL;
use crate::tokens::dialog::RETURN_BORDER;
use crate::tokens::dialog::RETURN_GLYPH;
use crate::tokens::dialog::TEXT;
use crate::tokens::dialog::SHADOW;
use crate::tokens::dialog::OUTLINE;
/// Arc's motion: over 0.275 s the window dims and the card comes down 32 pt into place,
/// ease-in-out, opaque after the first 0.135 s. It leaves the way it came, fading all the way.
const MOVE: f32 = 0.275;
const FADE_IN: f32 = 0.135;
const DROP: f32 = 32.;

/// What a dialog asks.
pub struct Question {
    /// Shown over the message, such as the extension's.
    pub icon: Option<Arc<Image>>,
    pub message: SharedString,
    /// Under the message, or nothing if empty.
    pub detail: SharedString,
    /// Lines listed under the detail, each with its smaller lines under it.
    pub list: Vec<(SharedString, Vec<SharedString>)>,
    /// The button that does it, which ↩ chooses. With none, only Cancel answers.
    pub action: SharedString,
    pub cancel: SharedString,
    pub tone: Tone,
    /// A button at the left for another answer, and what it does once the dialog has gone.
    pub also: Option<(SharedString, Box<dyn FnOnce(&mut App)>)>,
}

/// The action's colour, by what it does.
#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    /// Red, as it's hard to take back, such as deleting a Space.
    Destructive,
    /// Blue, as it can be taken back, such as quitting.
    Safe,
    /// The Space's accent, as it takes nothing back, such as adding an extension.
    Accent,
}

/// What the user chose.
#[derive(Clone, Copy, PartialEq)]
enum Answer {
    Action,
    Also,
    Cancel,
}

/// What the dialog asks of its window.
pub trait Host {
    /// Shows the dialog's window over all of the browser window, with the keyboard.
    fn show(&self);
    /// Hides it, and the browser window gets the keyboard back.
    fn hide(&self);
}

/// Where the motion is: how far the window has dimmed and the card has faded in, from 0 to 1,
/// and how far above its place the card is.
#[derive(Clone, Copy)]
struct Pose {
    dim: f32,
    opacity: f32,
    raised: f32,
}

impl Pose {
    /// `seconds` after the dialog was asked for.
    fn arriving(seconds: f32) -> Self {
        let t = (seconds / MOVE).min(1.);
        Pose {
            dim: t,
            opacity: (seconds / FADE_IN).min(1.),
            raised: DROP * (1. - ease_in_out(t)),
        }
    }

    /// `seconds` after the user answered, from where the motion was then.
    fn leaving(self, seconds: f32) -> Self {
        let t = (seconds / MOVE).min(1.);
        Pose {
            dim: self.dim * (1. - t),
            opacity: self.opacity * (1. - t),
            raised: self.raised + (DROP - self.raised) * ease_in_out(t),
        }
    }
}

fn ease_in_out(t: f32) -> f32 {
    cubic_bezier(0.42, 0., 0.58, 1., t)
}

/// A question to show, and what's told of its answer once the dialog has gone.
struct Request {
    question: Question,
    /// Whether Chrome asked it, and can take it back.
    chrome: bool,
    answered: Box<dyn FnOnce(bool, &mut App)>,
}

struct Asked {
    /// The question, its other answer taken out into `also` and `also_chosen`.
    question: Question,
    also: Option<SharedString>,
    also_chosen: Option<Box<dyn FnOnce(&mut App)>>,
    chrome: bool,
    answered: Option<Box<dyn FnOnce(bool, &mut App)>>,
    /// When its window was first on screen, where the motion starts.
    shown: Option<Instant>,
    settled: bool,
    /// Once the user has answered, or Chrome took it back: when, and where the motion was.
    answered_at: Option<(Instant, Pose)>,
    _leave: Option<Task<()>>,
}

impl Asked {
    /// How long the dialog has been arriving, in seconds.
    fn arrived_for(&self) -> f32 {
        self.shown.map_or(0., |shown| shown.elapsed().as_secs_f32())
    }

    fn pose(&self) -> Pose {
        match self.answered_at {
            Some((at, from)) => from.leaving(at.elapsed().as_secs_f32()),
            None => Pose::arriving(self.arrived_for()),
        }
    }
}

pub struct Dialog {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    asked: Option<Asked>,
    /// Chrome's question, waiting for the dialog that shows to go.
    next: Option<Request>,
}

struct DialogWindow(WindowHandle<Dialog>);

impl Global for DialogWindow {}

/// Opens the dialog's window, hidden, for `host` to show over the browser window, ahead of time
/// as the command bar's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<Dialog>> {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm, Some("Dialog")),
        KeyBinding::new("escape", Cancel, Some("Dialog")),
    ]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(WIDTH)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Floating,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The dim and the card are drawn.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| Dialog { host, focus: cx.focus_handle(), asked: None, next: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(DialogWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the dialog's window: {error}");
            None
        }
    }
}

/// Whether the dialog is asking, or leaving after the answer.
pub fn asking(cx: &App) -> bool {
    cx.try_global::<DialogWindow>()
        .and_then(|dialog| dialog.0.read(cx).ok())
        .is_some_and(|dialog| dialog.asked.is_some())
}

/// Asks `message`, with `detail` under it, and calls `confirmed` if the user chooses `action`
/// over Cancel, once the dialog has gone. The action is red, as what it does is hard to take
/// back. Nothing while another dialog shows.
pub fn ask(
    message: String,
    detail: String,
    action: &'static str,
    confirmed: Box<dyn FnOnce(&mut App)>,
    cx: &mut App,
) {
    let question = Question {
        icon: None,
        message: message.into(),
        detail: detail.into(),
        list: Vec::new(),
        action: action.into(),
        cancel: "Cancel".into(),
        tone: Tone::Destructive,
        also: None,
    };
    pose(question, confirmed, cx);
}

/// Asks `question`, and calls `confirmed` if the user chooses its action, once the dialog has
/// gone. Nothing while another dialog shows.
pub fn pose(question: Question, confirmed: Box<dyn FnOnce(&mut App)>, cx: &mut App) {
    let answered = Box::new(move |confirm, cx: &mut App| {
        if confirm {
            confirmed(cx);
        }
    });
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window
        .update(cx, |dialog, window, cx| {
            if dialog.asked.is_none() {
                dialog.show(Request { question, chrome: false, answered }, window, cx);
            }
        })
        .ok();
}

/// Asks Chrome's `question` once the dialog that shows has gone, in place of any of Chrome's
/// that hasn't been answered, and calls `answered` with the answer once the dialog has gone.
/// With no dialog to ask in, it's answered no at once, so that Chrome isn't left waiting.
pub fn ask_chrome(question: Question, answered: Box<dyn FnOnce(bool, &mut App)>, cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        answered(false, cx);
        return;
    };
    let mut request = Some(Request { question, chrome: true, answered });
    window
        .update(cx, |dialog, window, cx| {
            dialog.withdraw(cx);
            let request = request.take().expect("asked once");
            if dialog.asked.is_some() {
                dialog.next = Some(request);
            } else {
                dialog.show(request, window, cx);
            }
        })
        .ok();
    if let Some(request) = request {
        (request.answered)(false, cx);
    }
}

/// Takes back Chrome's question, unanswered: the dialog leaves if it shows it, and nothing is
/// told of an answer.
pub fn withdraw_chrome(cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window.update(cx, |dialog, _, cx| dialog.withdraw(cx)).ok();
}

impl Dialog {
    fn show(&mut self, request: Request, window: &mut Window, cx: &mut Context<Self>) {
        let Request { mut question, chrome, answered } = request;
        // check.sh reads this line.
        log!("browser: dialog shown {}", question.message);
        let (also, also_chosen) = question.also.take().unzip();
        self.asked = Some(Asked {
            question,
            also,
            also_chosen,
            chrome,
            answered: Some(answered),
            shown: None,
            settled: false,
            answered_at: None,
            _leave: None,
        });
        window.focus(&self.focus, cx);
        self.host.show();
        cx.notify();
    }

    /// Drops Chrome's question that waits, and Chrome's that shows leaves, both untold.
    fn withdraw(&mut self, cx: &mut Context<Self>) {
        self.next = None;
        let Some(asked) = &mut self.asked else {
            return;
        };
        if asked.chrome && asked.answered_at.is_none() {
            // check.sh reads this line.
            log!("browser: dialog withdrawn");
            self.leave(None, cx);
        }
    }

    /// The user chose the action, the other answer or Cancel. The dialog leaves, and then what
    /// was chosen happens.
    fn answer(&mut self, answer: Answer, cx: &mut Context<Self>) {
        let Some(asked) = &mut self.asked else {
            return;
        };
        if asked.answered_at.is_some()
            || (answer == Answer::Action && asked.question.action.is_empty())
        {
            return;
        }
        // check.sh reads these lines.
        match answer {
            Answer::Action => log!("browser: dialog confirmed"),
            Answer::Also => {
                log!("browser: dialog chose {}", asked.also.clone().unwrap_or_default())
            }
            Answer::Cancel => log!("browser: dialog cancelled"),
        }
        self.leave(Some(answer), cx);
    }

    /// The dialog leaves, and then what was chosen happens, unless it was taken back.
    fn leave(&mut self, answer: Option<Answer>, cx: &mut Context<Self>) {
        let Some(asked) = &mut self.asked else {
            return;
        };
        asked.answered_at = Some((Instant::now(), asked.pose()));
        let (answered, also_chosen) = match answer {
            Some(Answer::Also) => (None, asked.also_chosen.take()),
            Some(answer) => {
                (asked.answered.take().map(|answered| (answered, answer == Answer::Action)), None)
            }
            None => (None, None),
        };
        asked._leave = Some(cx.spawn(async move |dialog, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(MOVE)).await;
            let hidden = dialog.update(cx, |dialog, cx| {
                dialog.asked = None;
                dialog.host.hide();
                // check.sh reads this line.
                log!("browser: dialog hidden");
                cx.notify();
            });
            if hidden.is_err() {
                return;
            }
            if let Some((answered, confirm)) = answered {
                cx.update(|cx| answered(confirm, cx));
            }
            if let Some(also_chosen) = also_chosen {
                cx.update(also_chosen);
            }
            cx.update(show_next);
        }));
        cx.notify();
    }
}

/// Shows Chrome's question that waits, if no other dialog has come up first.
fn show_next(cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window
        .update(cx, |dialog, window, cx| {
            if dialog.asked.is_none()
                && let Some(request) = dialog.next.take()
            {
                dialog.show(request, window, cx);
            }
        })
        .ok();
}

/// A button with its key's chip after its label.
fn button(id: &'static str, label: SharedString, chip: impl IntoElement) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(BUTTON_HEIGHT))
        .pl(px(LABEL_X))
        .pr(px(CHIP_END))
        .flex()
        .items_center()
        .gap(px(CHIP_GAP))
        .rounded(px(BUTTON_RADIUS))
        // macOS's control font, as Arc's buttons have.
        .text_size(px(13.))
        .child(label)
        .child(chip)
}

/// A key's chip on a button.
fn chip() -> Div {
    div().h(px(CHIP_HEIGHT)).flex().items_center().justify_center().rounded(px(CHIP_RADIUS))
}

fn shadow(color: Hsla, y: f32, blur: f32, spread: f32) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

impl Render for Dialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .key_context("Dialog")
            .track_focus(&self.focus)
            .size_full()
            .font_family(".SystemUIFont");
        let Some(asked) = &mut self.asked else {
            return root;
        };
        // AppKit puts the window on screen some frames after it's asked to, and only then does
        // GPUI draw it, so the motion starts there rather than skip its start.
        if asked.shown.is_none() && window.is_visible() {
            asked.shown = Some(Instant::now());
        }
        let pose = asked.pose();
        let moving = asked.answered_at.is_some() || asked.arrived_for() < MOVE;
        if moving {
            window.request_animation_frame();
        } else if !asked.settled {
            asked.settled = true;
            // check.sh reads this line.
            log!("browser: dialog settled");
        }
        let question = &asked.question;
        let cancel = button(
            "dialog-cancel",
            question.cancel.clone(),
            chip()
                .px(px(6.5))
                .bg(rgba(ESC_FILL))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(ESC_TEXT))
                .child("ESC"),
        )
            // Its border is inside its edge, where Arc's label and chip are measured from.
            .pl(px(LABEL_X - 1.))
            .pr(px(CHIP_END - 1.))
            .border_1()
            .border_color(rgb(CANCEL_BORDER))
            .bg(rgb(CANCEL_FILL))
            .hover(|style| style.bg(rgb(CANCEL_HOVERED)))
            .text_color(rgba(CANCEL_TEXT))
            .on_click(cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Cancel, cx)));
        // A text-only button in Cancel's look, its label as far in from each side.
        let also = asked.also.clone().map(|label| {
            div()
                .id("dialog-also")
                .h(px(BUTTON_HEIGHT))
                .px(px(LABEL_X))
                .flex()
                .items_center()
                .rounded(px(BUTTON_RADIUS))
                .text_size(px(13.))
                .border_1()
                .border_color(rgb(CANCEL_BORDER))
                .bg(rgb(CANCEL_FILL))
                .hover(|style| style.bg(rgb(CANCEL_HOVERED)))
                .text_color(rgba(CANCEL_TEXT))
                .child(label)
                .on_click(cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Also, cx)))
        });
        let (fill, hovered, ledge): (Hsla, Hsla, Hsla) = match question.tone {
            Tone::Destructive => {
                (rgb(ACTION).into(), rgb(ACTION_HOVERED).into(), rgba(ACTION_LEDGE).into())
            }
            Tone::Safe => (
                rgb(SAFE_ACTION).into(),
                rgb(SAFE_ACTION_HOVERED).into(),
                rgb(SAFE_ACTION_LEDGE).into(),
            ),
            Tone::Accent => {
                let accent = command_bar::accent(cx);
                (accent, Hsla { l: accent.l - ACCENT_HOVERED, ..accent }, accent.opacity(0.2))
            }
        };
        let action = (!question.action.is_empty()).then(|| {
            button(
                "dialog-confirm",
                question.action.clone(),
                chip()
                    .w(px(RETURN_WIDTH))
                    .border_1()
                    .border_color(rgba(RETURN_BORDER))
                    .bg(rgba(RETURN_FILL))
                    .child(
                        svg()
                            .path("icons/corner-down-left.svg")
                            .size(px(RETURN_GLYPH))
                            .text_color(rgb(TEXT)),
                    ),
            )
            .bg(fill)
            .hover(move |style| style.bg(hovered))
            .shadow(vec![shadow(ledge, 2., 0., 0.)])
            .text_color(rgb(TEXT))
            .on_click(cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Action, cx)))
        });
        let list = question.list.iter().map(|(line, sublines)| {
            div()
                .flex()
                .flex_col()
                .child(line.clone())
                .children(sublines.iter().map(|subline| {
                    div()
                        .pl(px(SUBLINE_INDENT))
                        .line_height(px(SUBLINE))
                        .text_size(px(12.))
                        .text_color(rgba(SUBLINE_TEXT))
                        .child(subline.clone())
                }))
        });
        let detail = (!question.detail.is_empty() || !question.list.is_empty()).then(|| {
            div()
                .mt(px(DETAIL_TOP))
                .pr(px(TEXT_INSET - BUTTON_INSET))
                .flex()
                .flex_col()
                .line_height(px(DETAIL_LINE))
                .text_size(px(14.))
                .text_color(rgba(DETAIL))
                .when(!question.detail.is_empty(), |text| text.child(question.detail.clone()))
                .when(!question.list.is_empty(), |text| {
                    text.child(
                        div()
                            .when(!question.detail.is_empty(), |list| list.mt(px(LIST_TOP)))
                            .flex()
                            .flex_col()
                            .gap(px(LIST_GAP))
                            .children(list),
                    )
                })
        });
        let buttons_top = if detail.is_some() { BUTTONS_TOP } else { BUTTONS_UNDER_TITLE };
        let card = div()
            .id("dialog")
            // Clicks on the card stay on it, and one around it cancels.
            .occlude()
            .relative()
            .top(px(-pose.raised))
            .w(px(WIDTH))
            .pt(px(TEXT_INSET))
            .pl(px(TEXT_INSET))
            .pr(px(BUTTON_INSET))
            .pb(px(BUTTON_INSET))
            .flex()
            .flex_col()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(rgba(EDGE))
            .bg(rgb(FILL))
            .shadow(vec![
                shadow(hsla(0., 0., 0., SHADOW), 16., 16., 0.),
                shadow(hsla(0., 0., 0., OUTLINE), 0., 0., 0.5),
            ])
            .opacity(pose.opacity)
            .children(
                question.icon.clone().map(|icon| img(icon).size(px(ICON)).mb(px(ICON_GAP))),
            )
            .child(
                div()
                    .pr(px(TEXT_INSET - BUTTON_INSET))
                    .line_height(px(TITLE_LINE))
                    .text_size(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(TEXT))
                    .child(question.message.clone()),
            )
            .children(detail)
            .child(
                div()
                    .mt(px(buttons_top))
                    .flex()
                    .gap(px(ALSO_GAP))
                    .children(also)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .justify_end()
                            .gap(px(CHIP_GAP))
                            .child(cancel)
                            .children(action),
                    ),
            );
        root.on_action(cx.listener(|dialog, _: &Confirm, _, cx| dialog.answer(Answer::Action, cx)))
            .on_action(cx.listener(|dialog, _: &Cancel, _, cx| dialog.answer(Answer::Cancel, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Cancel, cx)),
            )
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla::from(rgb(DIM)).opacity(DIM_OPACITY * pose.dim))
            .child(card)
    }
}

#[cfg(test)]
mod tests;
