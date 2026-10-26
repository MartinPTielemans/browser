//! The dialog that asks before something hard to take back, such as deleting a Space with its
//! tabs, or before quitting: a card over the window, which dims, as Arc's (docs/ux-spec.md ›
//! Spaces › Renaming and deleting, and Warn before quitting). GPUI can't draw over the page's
//! native view, so it's a window of its own that the host shows over all of the browser window,
//! as its child, with the keyboard. What it asks of its window goes through `Host`.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, BoxShadow, Context, Div, FocusHandle, FontWeight, Global, Hsla, KeyBinding,
    MouseButton, SharedString, Stateful, Task, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, div, hsla, point, prelude::*,
    px, rgb, rgba, size, svg,
};

use crate::shell::cubic_bezier;

actions!(dialog, [Confirm, Cancel]);

// Arc's, measured at 2× (docs/ux-spec.md › Spaces › Renaming and deleting). The card is centred
// in the window. Its text is inset 38 from its top and left, and its buttons 28 from its right
// and bottom. Arc's has its icon over the title, and ours starts with the title.
const WIDTH: f32 = 450.;
const RADIUS: f32 = 16.;
const TEXT_INSET: f32 = 38.;
const BUTTON_INSET: f32 = 28.;
const TITLE_LINE: f32 = 22.;
const DETAIL_TOP: f32 = 15.5;
const DETAIL_LINE: f32 = 18.;
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

/// What a dialog asks.
pub struct Question {
    pub message: String,
    /// Under the message, if any.
    pub detail: String,
    /// The button that does it, which ↩ chooses.
    pub action: &'static str,
    /// Whether the action is hard to take back, which makes it red, else blue.
    pub destructive: bool,
    /// A button at the left for another answer, and what it does once the dialog has gone.
    pub also: Option<(&'static str, Box<dyn FnOnce(&mut App)>)>,
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

struct Asked {
    message: SharedString,
    detail: SharedString,
    action: SharedString,
    destructive: bool,
    also: Option<SharedString>,
    confirmed: Option<Box<dyn FnOnce(&mut App)>>,
    also_chosen: Option<Box<dyn FnOnce(&mut App)>>,
    /// When its window was first on screen, where the motion starts.
    shown: Option<Instant>,
    settled: bool,
    /// Once the user has answered: when, and where the motion was.
    answered: Option<(Instant, Pose)>,
    _leave: Option<Task<()>>,
}

impl Asked {
    /// How long the dialog has been arriving, in seconds.
    fn arrived_for(&self) -> f32 {
        self.shown.map_or(0., |shown| shown.elapsed().as_secs_f32())
    }

    fn pose(&self) -> Pose {
        match self.answered {
            Some((at, from)) => from.leaving(at.elapsed().as_secs_f32()),
            None => Pose::arriving(self.arrived_for()),
        }
    }
}

pub struct Dialog {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    asked: Option<Asked>,
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
        cx.new(|cx| Dialog { host, focus: cx.focus_handle(), asked: None })
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
    let question = Question { message, detail, action, destructive: true, also: None };
    pose(question, confirmed, cx);
}

/// Asks `question`, and calls `confirmed` if the user chooses its action, once the dialog has
/// gone. Nothing while another dialog shows.
pub fn pose(question: Question, confirmed: Box<dyn FnOnce(&mut App)>, cx: &mut App) {
    let Question { message, detail, action, destructive, also } = question;
    let Some(window) = cx.try_global::<DialogWindow>().map(|dialog| dialog.0) else {
        return;
    };
    window
        .update(cx, |dialog, window, cx| {
            if dialog.asked.is_some() {
                return;
            }
            // check.sh reads this line.
            log!("browser: dialog shown {message}");
            let (also, also_chosen) = also.unzip();
            dialog.asked = Some(Asked {
                message: message.into(),
                detail: detail.into(),
                action: action.into(),
                destructive,
                also: also.map(SharedString::from),
                confirmed: Some(confirmed),
                also_chosen,
                shown: None,
                settled: false,
                answered: None,
                _leave: None,
            });
            window.focus(&dialog.focus, cx);
            dialog.host.show();
            cx.notify();
        })
        .ok();
}

impl Dialog {
    /// The user chose the action, the other answer or Cancel. The dialog leaves, and then what
    /// was chosen happens.
    fn answer(&mut self, answer: Answer, cx: &mut Context<Self>) {
        let Some(asked) = &mut self.asked else {
            return;
        };
        if asked.answered.is_some() {
            return;
        }
        asked.answered = Some((Instant::now(), asked.pose()));
        let confirmed = match answer {
            Answer::Action => asked.confirmed.take(),
            Answer::Also => asked.also_chosen.take(),
            Answer::Cancel => None,
        };
        // check.sh reads these lines.
        match answer {
            Answer::Action => log!("browser: dialog confirmed"),
            Answer::Also => {
                log!("browser: dialog chose {}", asked.also.clone().unwrap_or_default())
            }
            Answer::Cancel => log!("browser: dialog cancelled"),
        }
        asked._leave = Some(cx.spawn(async move |dialog, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(MOVE)).await;
            let hidden = dialog.update(cx, |dialog, cx| {
                dialog.asked = None;
                dialog.host.hide();
                // check.sh reads this line.
                log!("browser: dialog hidden");
                cx.notify();
            });
            if let (Ok(()), Some(confirmed)) = (hidden, confirmed) {
                cx.update(confirmed);
            }
        }));
        cx.notify();
    }
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
        let moving = asked.answered.is_some() || asked.arrived_for() < MOVE;
        if moving {
            window.request_animation_frame();
        } else if !asked.settled {
            asked.settled = true;
            // check.sh reads this line.
            log!("browser: dialog settled");
        }
        let cancel = button(
            "dialog-cancel",
            "Cancel".into(),
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
        let (fill, hovered, ledge) = match asked.destructive {
            true => (ACTION, ACTION_HOVERED, rgba(ACTION_LEDGE)),
            false => (SAFE_ACTION, SAFE_ACTION_HOVERED, rgb(SAFE_ACTION_LEDGE).into()),
        };
        let action = button(
            "dialog-confirm",
            asked.action.clone(),
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
                ),
        )
            .bg(rgb(fill))
            .hover(move |style| style.bg(rgb(hovered)))
            .shadow(vec![shadow(ledge.into(), 2., 0., 0.)])
            .text_color(rgb(0xffffff))
            .on_click(cx.listener(|dialog, _, _, cx| dialog.answer(Answer::Action, cx)));
        let detail = (!asked.detail.is_empty()).then(|| {
            div()
                .mt(px(DETAIL_TOP))
                .pr(px(TEXT_INSET - BUTTON_INSET))
                .line_height(px(DETAIL_LINE))
                .text_size(px(14.))
                .text_color(rgba(DETAIL))
                .child(asked.detail.clone())
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
                shadow(hsla(0., 0., 0., 0.56), 16., 16., 0.),
                shadow(hsla(0., 0., 0., 0.9), 0., 0., 0.5),
            ])
            .opacity(pose.opacity)
            .child(
                div()
                    .pr(px(TEXT_INSET - BUTTON_INSET))
                    .line_height(px(TITLE_LINE))
                    .text_size(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0xffffff))
                    .child(asked.message.clone()),
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
                            .child(action),
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
