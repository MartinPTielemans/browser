//! The dialog that asks before something hard to take back, such as deleting a Space with its
//! tabs, or before quitting: a card over the window, which dims, as Arc's (docs/ux-spec.md ›
//! Spaces › Renaming and deleting, and Warn before quitting). GPUI can't draw over the page's
//! native view, so it's a window of its own that the host shows over all of the browser window,
//! as its child, with the keyboard. What it asks of its window goes through `Host`. It also asks
//! Chrome's questions, such as whether to add an extension, which wait their turn behind the
//! dialog that shows and which Chrome can take back, and a page's own, such as its alert(), over
//! its tab's card alone, which come and go with the tab (docs/ux-spec.md › Page dialogs).

use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, BoxShadow, Context, Div, Entity, FocusHandle, Focusable, FontWeight, Global,
    Hsla, Image, KeyBinding, MouseButton, Pixels, SharedString, Stateful, Task, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    actions, div, hsla, img, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::{command_bar, shell::cubic_bezier, text_input::TextInput};

actions!(dialog, [Confirm, Cancel, NextField]);

// Arc's, measured at 2× (docs/ux-spec.md › Spaces › Renaming and deleting). The card is centred
// in the window. Its text is inset 38 from its top and left, and its buttons 28 from its right
// and bottom. Arc's has its icon over the title, and ours starts with the title unless what it
// asks about has an icon of its own, such as an extension.
const WIDTH: f32 = 450.;
const RADIUS: f32 = 16.;
const TEXT_INSET: f32 = 38.;
const BUTTON_INSET: f32 = 28.;
const TITLE_LINE: f32 = 22.;
const DETAIL_TOP: f32 = 15.5;
const DETAIL_LINE: f32 = 18.;
/// An icon over the title, such as the extension's, and the gap under it.
const ICON: f32 = 32.;
const ICON_GAP: f32 = 16.;
/// A list under the detail, such as an extension's permission warnings, each with its smaller,
/// dimmer lines under it, indented.
const LIST_TOP: f32 = 10.;
const LIST_GAP: f32 = 6.;
const SUBLINE: f32 = 16.;
const SUBLINE_INDENT: f32 = 12.;
const SUBLINE_TEXT: u32 = 0xffff_ff8c;
const BUTTONS_TOP: f32 = 38.;
/// From the title's line to the buttons when there's no detail, as in Arc's ⌘Q.
const BUTTONS_UNDER_TITLE: f32 = 46.;
/// Between a button at the left, such as "Quit, and don't ask again", and Cancel at least.
const ALSO_GAP: f32 = 16.;
const FILL: u32 = 0x161b2f;
/// A line of white 0.1 inside the card's edge, and one of black 0.5 thick outside it.
const EDGE: u32 = 0xffff_ff1a;
const DETAIL: u32 = 0xffff_ffc7;
/// The window under the card: #151515 at 0.62.
const DIM: u32 = 0x151515;
const DIM_OPACITY: f32 = 0.62;
const BUTTON_HEIGHT: f32 = 36.;
const BUTTON_RADIUS: f32 = 8.;
/// A button's label starts 12 in from its left edge, and its key's chip ends 12 in from its
/// right, 8 after the label.
const LABEL_X: f32 = 12.;
const CHIP_END: f32 = 12.;
const CHIP_GAP: f32 = 8.;
const CANCEL_FILL: u32 = 0x2f305f;
/// Under the pointer it lightens, as white 0.1 over it (Arc 1.170 rec, ⌘Q's dialog).
const CANCEL_HOVERED: u32 = 0x45436f;
const CANCEL_BORDER: u32 = 0x6462a9;
const CANCEL_TEXT: u32 = 0xffff_ffe6;
/// The action, which darkens under the pointer and sits on a ledge of its own colour at 0.2.
const ACTION: u32 = 0xe1492c;
const ACTION_HOVERED: u32 = 0xcc4126;
const ACTION_LEDGE: u32 = 0xe149_2c33;
/// An action that can be taken back, such as Quit, is blue, on a darker ledge. Ours darkens
/// under the pointer as the red does.
const SAFE_ACTION: u32 = 0x3338f2;
const SAFE_ACTION_HOVERED: u32 = 0x2e32da;
const SAFE_ACTION_LEDGE: u32 = 0x393f92;
/// An action that takes nothing back, such as adding an extension, has the Space's accent
/// instead, darker by this under the pointer.
const ACCENT_HOVERED: f32 = 0.05;
const CHIP_HEIGHT: f32 = 20.;
const CHIP_RADIUS: f32 = 5.;
/// White 0.14 over the button, so it lightens with it.
const ESC_FILL: u32 = 0xffff_ff24;
const ESC_TEXT: u32 = 0xbbb9e3;
const RETURN_WIDTH: f32 = 30.;
const RETURN_FILL: u32 = 0xffff_ff1a;
const RETURN_BORDER: u32 = 0xffff_ff45;
const RETURN_GLYPH: f32 = 16.;
/// Arc's motion: over 0.275 s the window dims and the card comes down 32 pt into place,
/// ease-in-out, opaque after the first 0.135 s. It leaves the way it came, fading all the way.
const MOVE: f32 = 0.275;
const FADE_IN: f32 = 0.135;
const DROP: f32 = 32.;
/// Ours, as Arc's dialog has no fields: a page's fields, such as a sign-in's, under the detail,
/// as tall as the buttons and in Cancel's look, and a checkbox under them.
const FIELDS_TOP: f32 = 16.;
const FIELD_GAP: f32 = 8.;
const FIELD_TEXT: u32 = 0xffff_ffe6;
const FIELD_PLACEHOLDER: u32 = 0xffff_ff73;
const CHECK_TOP: f32 = 14.;
const CHECK_BOX: f32 = 16.;
const CHECK_RADIUS: f32 = 4.;
const CHECK_GAP: f32 = 8.;
const CHECK_GLYPH: f32 = 12.;

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
    /// Fields under the detail, such as a prompt's, the first with the keyboard.
    pub fields: Vec<Field>,
    /// A checkbox under them, by its label, such as "Always allow…".
    pub check: Option<SharedString>,
    /// ↩ cancels, and the action takes a click, as for a page opening an app, so a key press the
    /// page timed can't choose it, as Chrome has it.
    pub return_cancels: bool,
}

/// A field in a dialog.
pub struct Field {
    pub placeholder: SharedString,
    /// What it starts with, all selected.
    pub text: SharedString,
    /// It shows dots for what's typed, and won't copy it, as a password field.
    pub secure: bool,
}

/// What the user answered: the action or not, what was in the fields, and the checkbox.
pub struct Reply {
    pub accept: bool,
    pub texts: Vec<String>,
    pub checked: bool,
}

impl std::fmt::Debug for Reply {
    // What was typed, such as a password, stays out of logs and test output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Reply {{ accept: {}, fields: {}, checked: {} }}",
            self.accept,
            self.texts.len(),
            self.checked
        )
    }
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
    /// Shows the dialog's window over all of the browser window, or over `over` in it, a tab's
    /// card, with the keyboard.
    fn show(&self, over: Option<Bounds<Pixels>>);
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

/// Who asked a question.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// The browser's own, such as deleting a Space.
    User,
    /// Chrome's, such as adding an extension, which Chrome can take back.
    Chrome,
    /// A page's, over its tab's card, which goes when the tab does.
    Page,
}

/// A question to show, and what's told of its answer once the dialog has gone.
struct Request {
    question: Question,
    source: Source,
    /// The tab's card a page's question goes over, and the radius of its corners.
    over: Option<(Bounds<Pixels>, f32)>,
    /// A page's question's tab and id, by which it's asked again as its card moves.
    page: Option<(u64, u64)>,
    answered: Box<dyn FnOnce(Reply, &mut App)>,
}

struct Asked {
    /// The question, its other answer taken out into `also` and `also_chosen`.
    question: Question,
    also: Option<SharedString>,
    also_chosen: Option<Box<dyn FnOnce(&mut App)>>,
    source: Source,
    over: Option<(Bounds<Pixels>, f32)>,
    page: Option<(u64, u64)>,
    fields: Vec<Entity<TextInput>>,
    checked: bool,
    answered: Option<Box<dyn FnOnce(Reply, &mut App)>>,
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
    /// A page's, waiting the same way, after Chrome's.
    next_page: Option<Request>,
}

struct DialogWindow(WindowHandle<Dialog>);

impl Global for DialogWindow {}

/// Opens the dialog's window, hidden, for `host` to show over the browser window, ahead of time
/// as the command bar's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<Dialog>> {
    cx.bind_keys([
        KeyBinding::new("enter", Confirm, Some("Dialog")),
        KeyBinding::new("escape", Cancel, Some("Dialog")),
        KeyBinding::new("tab", NextField, Some("Dialog")),
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
        cx.new(|cx| Dialog {
            host,
            focus: cx.focus_handle(),
            asked: None,
            next: None,
            next_page: None,
        })
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
        fields: Vec::new(),
        check: None,
        return_cancels: false,
    };
    pose(question, confirmed, cx);
}

/// Asks `question`, and calls `confirmed` if the user chooses its action, once the dialog has
/// gone. Nothing while another dialog shows.
pub fn pose(question: Question, confirmed: Box<dyn FnOnce(&mut App)>, cx: &mut App) {
    let answered = Box::new(move |reply: Reply, cx: &mut App| {
        if reply.accept {
            confirmed(cx);
        }
    });
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window
        .update(cx, |dialog, window, cx| {
            if dialog.asked.is_none() {
                let request =
                    Request { question, source: Source::User, over: None, page: None, answered };
                dialog.show(request, window, cx);
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
    let answered = Box::new(move |reply: Reply, cx: &mut App| answered(reply.accept, cx));
    let mut request =
        Some(Request { question, source: Source::Chrome, over: None, page: None, answered });
    window
        .update(cx, |dialog, window, cx| {
            dialog.withdraw(Source::Chrome, cx);
            let request = request.take().expect("asked once");
            if dialog.asked.is_some() {
                dialog.next = Some(request);
            } else {
                dialog.show(request, window, cx);
            }
        })
        .ok();
    if let Some(request) = request {
        (request.answered)(Reply { accept: false, texts: Vec::new(), checked: false }, cx);
    }
}

/// Takes back Chrome's question, unanswered: the dialog leaves if it shows it, and nothing is
/// told of an answer.
pub fn withdraw_chrome(cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window.update(cx, |dialog, _, cx| dialog.withdraw(Source::Chrome, cx)).ok();
}

/// Asks a page's `question`, the dialog `id` of `tab`, over its tab's card, `over` in the window
/// with corners of `radius`, in place of any page's that shows or waits, once any other dialog has
/// gone, and calls `answered` with the answer once the dialog has gone. Asked again, it only moves
/// to `over`, with what was typed. A click on the card's dim does nothing, as the page waits.
pub fn ask_page(
    (tab, id): (u64, u64),
    question: Question,
    over: (Bounds<Pixels>, f32),
    answered: Box<dyn FnOnce(Reply, &mut App)>,
    cx: &mut App,
) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    let page = Some((tab, id));
    let request = Request { question, source: Source::Page, over: Some(over), page, answered };
    window
        .update(cx, |dialog, window, cx| {
            if let Some(asked) = &mut dialog.asked
                && asked.page == page
                && asked.answered_at.is_none()
            {
                asked.over = Some(over);
                dialog.host.show(Some(over.0));
                cx.notify();
                return;
            }
            if let Some(next) = &mut dialog.next_page
                && next.page == page
            {
                next.over = Some(over);
                return;
            }
            dialog.withdraw(Source::Page, cx);
            match dialog.asked.is_some() {
                true => dialog.next_page = Some(request),
                false => dialog.show(request, window, cx),
            }
        })
        .ok();
}

/// Takes back the page's question, unanswered, as its tab no longer shows or the page no longer
/// asks: the dialog leaves if it shows it, and nothing is told of an answer.
pub fn withdraw_page(cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window.update(cx, |dialog, _, cx| dialog.withdraw(Source::Page, cx)).ok();
}

impl Dialog {
    fn show(&mut self, request: Request, window: &mut Window, cx: &mut Context<Self>) {
        let Request { mut question, source, over, page, answered } = request;
        // check.sh reads this line.
        log!("browser: dialog shown {}", question.message);
        let (also, also_chosen) = question.also.take().unzip();
        let fields: Vec<_> = question
            .fields
            .iter()
            .map(|field| {
                cx.new(|cx| {
                    TextInput::new(field.placeholder.clone(), cx)
                        .with_text(field.text.clone())
                        .secure(field.secure)
                        .placeholder_color(rgba(FIELD_PLACEHOLDER))
                        .caret_color(rgb(0xffffff))
                        .selection_color(rgba(ESC_FILL))
                })
            })
            .collect();
        match fields.first() {
            Some(field) => window.focus(&field.focus_handle(cx), cx),
            None => window.focus(&self.focus, cx),
        }
        self.asked = Some(Asked {
            question,
            also,
            also_chosen,
            source,
            over,
            page,
            fields,
            checked: false,
            answered: Some(answered),
            shown: None,
            settled: false,
            answered_at: None,
            _leave: None,
        });
        self.host.show(over.map(|(over, _)| over));
        cx.notify();
    }

    /// Drops the question from `source` that waits, and the one that shows leaves, both untold.
    fn withdraw(&mut self, source: Source, cx: &mut Context<Self>) {
        match source {
            Source::Page => self.next_page = None,
            _ => self.next = None,
        }
        let Some(asked) = &mut self.asked else {
            return;
        };
        if asked.source == source && asked.answered_at.is_none() {
            // check.sh reads this line.
            log!("browser: dialog withdrawn");
            self.leave(None, cx);
        }
    }

    /// Moves the keyboard to the next field, round to the first.
    fn next_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(asked) = &self.asked else {
            return;
        };
        let focused =
            asked.fields.iter().position(|field| field.focus_handle(cx).is_focused(window));
        let next = focused.map_or(0, |focused| (focused + 1) % asked.fields.len().max(1));
        if let Some(field) = asked.fields.get(next) {
            window.focus(&field.focus_handle(cx), cx);
        }
    }

    /// The user clicked the checkbox or its label.
    fn toggle_check(&mut self, cx: &mut Context<Self>) {
        if let Some(asked) = &mut self.asked {
            asked.checked = !asked.checked;
            cx.notify();
        }
    }

    /// The user pressed ↩: the action, unless it takes a click.
    fn confirm(&mut self, cx: &mut Context<Self>) {
        let Some(asked) = &self.asked else {
            return;
        };
        // An input method takes ↩ to finish what it composes.
        if asked.fields.iter().any(|field| field.read(cx).composing()) {
            return;
        }
        match asked.question.return_cancels {
            true => self.answer(Answer::Cancel, cx),
            false => self.answer(Answer::Action, cx),
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
                let reply = Reply {
                    accept: answer == Answer::Action,
                    texts: (asked.fields.iter())
                        .map(|field| field.read(cx).text().to_owned())
                        .collect(),
                    checked: asked.checked,
                };
                (asked.answered.take().map(|answered| (answered, reply)), None)
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
            if let Some((answered, reply)) = answered {
                cx.update(|cx| answered(reply, cx));
            }
            if let Some(also_chosen) = also_chosen {
                cx.update(also_chosen);
            }
            cx.update(show_next);
        }));
        cx.notify();
    }
}

/// Shows Chrome's question that waits, else a page's, if no other dialog has come up first.
fn show_next(cx: &mut App) {
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window
        .update(cx, |dialog, window, cx| {
            if dialog.asked.is_none()
                && let Some(request) = dialog.next.take().or_else(|| dialog.next_page.take())
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
        let page = asked.source == Source::Page;
        // An alert has only its action.
        let cancel = (!question.cancel.is_empty()).then(|| button(
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
            .on_click(cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Cancel, cx))));
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
        let return_chip = (!question.return_cancels).then(|| {
            chip()
                .w(px(RETURN_WIDTH))
                .border_1()
                .border_color(rgba(RETURN_BORDER))
                .bg(rgba(RETURN_FILL))
                .child(
                    svg()
                        .path("icons/corner-down-left.svg")
                        .size(px(RETURN_GLYPH))
                        .text_color(rgb(0xffffff)),
                )
        });
        let action = (!question.action.is_empty()).then(|| {
            button("dialog-confirm", question.action.clone(), div().children(return_chip))
                // Without its chip, its label is as far in from each side.
                .when(question.return_cancels, |button| button.gap_0().pr(px(LABEL_X)))
            .bg(fill)
            .hover(move |style| style.bg(hovered))
            .shadow(vec![shadow(ledge, 2., 0., 0.)])
            .text_color(rgb(0xffffff))
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
        let fields = (!asked.fields.is_empty()).then(|| {
            div().mt(px(FIELDS_TOP)).flex().flex_col().gap(px(FIELD_GAP)).children(
                asked.fields.iter().map(|field| {
                    div()
                        .h(px(BUTTON_HEIGHT))
                        .px(px(LABEL_X - 1.))
                        .flex()
                        .items_center()
                        .rounded(px(BUTTON_RADIUS))
                        .border_1()
                        .border_color(rgb(CANCEL_BORDER))
                        .bg(rgb(CANCEL_FILL))
                        .text_size(px(13.))
                        .text_color(rgba(FIELD_TEXT))
                        .child(field.clone())
                }),
            )
        });
        let check = question.check.clone().map(|label| {
            let checked = asked.checked;
            div()
                .id("dialog-check")
                .mt(px(CHECK_TOP))
                .flex()
                .items_center()
                .gap(px(CHECK_GAP))
                .text_size(px(13.))
                .text_color(rgba(DETAIL))
                .child(
                    div()
                        .flex_none()
                        .size(px(CHECK_BOX))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(CHECK_RADIUS))
                        .border_1()
                        .border_color(rgb(CANCEL_BORDER))
                        .bg(if checked { fill } else { rgb(CANCEL_FILL).into() })
                        .when(checked, |tick| {
                            tick.child(
                                svg()
                                    .path("icons/check.svg")
                                    .size(px(CHECK_GLYPH))
                                    .text_color(rgb(0xffffff)),
                            )
                        }),
                )
                .child(label)
                .on_click(cx.listener(|dialog, _, _, cx| dialog.toggle_check(cx)))
        });
        let buttons_top = match detail.is_some() || fields.is_some() || check.is_some() {
            true => BUTTONS_TOP,
            false => BUTTONS_UNDER_TITLE,
        };
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
                shadow(hsla(0., 0., 0., 0.56), 16., 16., 0.),
                shadow(hsla(0., 0., 0., 0.9), 0., 0., 0.5),
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
                    .text_color(rgb(0xffffff))
                    .child(question.message.clone()),
            )
            .children(detail)
            .children(fields)
            .children(check)
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
                            .children(cancel)
                            .children(action),
                    ),
            );
        let radius = asked.over.map_or(0., |(_, radius)| radius);
        root.on_action(cx.listener(|dialog, _: &Confirm, _, cx| dialog.confirm(cx)))
            .on_action(cx.listener(|dialog, _: &Cancel, _, cx| dialog.answer(Answer::Cancel, cx)))
            .on_action(
                cx.listener(|dialog, _: &NextField, window, cx| dialog.next_field(window, cx)),
            )
            // A page waits for its answer, so a click on its card's dim does nothing.
            .when(!page, |root| {
                root.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Cancel, cx)),
                )
            })
            // Over a tab's card, the dim has the card's corners.
            .rounded(px(radius))
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla::from(rgb(DIM)).opacity(DIM_OPACITY * pose.dim))
            .child(card)
    }
}

#[cfg(test)]
mod tests;
