//! The dialog's keys and clicks, on a host that records what the dialog asks of its window.

use std::{cell::RefCell, rc::Rc};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px, size};

use super::*;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Host for Recorder {
    fn show(&self) {
        self.0.borrow_mut().push("show".into());
    }

    fn hide(&self) {
        self.0.borrow_mut().push("hide".into());
    }
}

/// The dialog, asking to delete a Space. The action records itself on the host when it runs.
struct Asking {
    cx: VisualTestContext,
    host: Rc<Recorder>,
}

impl Asking {
    fn new(cx: &mut TestAppContext) -> Self {
        let host = Rc::new(Recorder::default());
        let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the dialog");
        let deleted = host.clone();
        cx.update(|cx| {
            let delete = Box::new(move |_: &mut App| deleted.0.borrow_mut().push("delete".into()));
            ask("Delete “Work”?".into(), "Its 3 tabs close too.".into(), "Delete", delete, cx);
        });
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.simulate_resize(size(px(800.), px(600.)));
        cx.run_until_parked();
        Asking { cx, host }
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    fn click(&mut self, x: f32, y: f32) {
        self.cx.simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// What the dialog asked of the host since the last time, once it has had time to leave.
    fn calls(&mut self) -> Vec<String> {
        self.cx.executor().advance_clock(Duration::from_secs_f32(MOVE));
        self.cx.run_until_parked();
        self.host.0.take()
    }
}

#[gpui::test]
fn escape_cancels(cx: &mut TestAppContext) {
    let mut dialog = Asking::new(cx);
    dialog.keys("escape");
    assert_eq!(dialog.calls(), ["show", "hide"]);
}

#[gpui::test]
fn return_deletes_once_the_dialog_has_gone(cx: &mut TestAppContext) {
    let mut dialog = Asking::new(cx);
    dialog.keys("enter");
    assert_eq!(dialog.host.0.borrow().as_slice(), ["show"]);
    // A second ↩ while it leaves does nothing more.
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["show", "hide", "delete"]);
}

#[gpui::test]
fn a_click_around_the_card_cancels(cx: &mut TestAppContext) {
    let mut dialog = Asking::new(cx);
    // The card's middle, and then the dimmed window above it.
    dialog.click(400., 300.);
    assert_eq!(dialog.calls(), ["show"]);
    dialog.click(400., 40.);
    assert_eq!(dialog.calls(), ["hide"]);
}

#[gpui::test]
fn an_ask_while_one_shows_is_dropped(cx: &mut TestAppContext) {
    let mut dialog = Asking::new(cx);
    dialog.cx.cx.update(|cx| {
        let clear = Box::new(|_: &mut App| panic!("the second dialog ran"));
        ask("Clear the archive?".into(), String::new(), "Clear Archive", clear, cx);
    });
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["show", "hide", "delete"]);
}

/// The dialog asking to quit, with "Quit, and don't ask again" at the left. Each answer records
/// itself on the host when it runs.
fn asking_to_quit(cx: &mut TestAppContext) -> Asking {
    let host = Rc::new(Recorder::default());
    let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the dialog");
    let (quit, always) = (host.clone(), host.clone());
    cx.update(|cx| {
        let question = Question {
            icon: None,
            message: "Quit Browser?".into(),
            detail: "".into(),
            list: Vec::new(),
            action: "Quit".into(),
            cancel: "Cancel".into(),
            tone: Tone::Accent,
            also: Some((
                "Quit, and don’t ask again".into(),
                Box::new(move |_: &mut App| always.0.borrow_mut().push("always".into())),
            )),
        };
        pose(question, Box::new(move |_| quit.0.borrow_mut().push("quit".into())), cx);
    });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    Asking { cx, host }
}

#[gpui::test]
fn return_quits_and_escape_doesnt(cx: &mut TestAppContext) {
    let mut dialog = asking_to_quit(cx);
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["show", "hide", "quit"]);
    let mut dialog = asking_to_quit(cx);
    dialog.keys("escape");
    assert_eq!(dialog.calls(), ["show", "hide"]);
}

#[gpui::test]
fn the_left_button_gives_the_other_answer(cx: &mut TestAppContext) {
    let mut dialog = asking_to_quit(cx);
    // The card is centred in the 800 × 600 window, still 32 above its place as it comes down:
    // 172 tall with no detail, its buttons 107 under its top and the left one 29 in.
    let card = (800. - WIDTH) / 2.;
    let buttons = (600. - 172.) / 2. - DROP + 1. + TEXT_INSET + TITLE_LINE + BUTTONS_UNDER_TITLE;
    dialog.click(card + 1. + BUTTON_INSET + 40., buttons + BUTTON_HEIGHT / 2.);
    assert_eq!(dialog.calls(), ["show", "hide", "always"]);
}

/// Chrome's question whether to add an extension, which records the answer on `host`.
fn add_extension(action: &str, host: &Rc<Recorder>, cx: &mut App) {
    let question = Question {
        icon: None,
        message: "Add \"Check\"?".into(),
        detail: "It can:".into(),
        list: vec![
            ("Read your browsing history".into(), Vec::new()),
            ("Read and change your data on example.org".into(), vec!["example.org".into()]),
        ],
        action: action.to_owned().into(),
        cancel: "Cancel".into(),
        tone: Tone::Accent,
        also: None,
    };
    let host = host.clone();
    let answered = Box::new(move |yes, _: &mut App| {
        host.0.borrow_mut().push(if yes { "yes" } else { "no" }.into());
    });
    ask_chrome(question, answered, cx);
}

/// The dialog, open and asking nothing.
fn opened(cx: &mut TestAppContext) -> (VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the dialog");
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(size(px(800.), px(600.)));
    (cx, host)
}

impl Asking {
    /// The dialog, asking Chrome's question.
    fn chrome(action: &str, cx: &mut TestAppContext) -> Self {
        let (cx, host) = opened(cx);
        cx.cx.update(|cx| add_extension(action, &host, cx));
        cx.run_until_parked();
        Asking { cx, host }
    }
}

#[gpui::test]
fn return_adds_the_extension_once_the_dialog_has_gone(cx: &mut TestAppContext) {
    let mut dialog = Asking::chrome("Add extension", cx);
    dialog.keys("enter");
    assert_eq!(dialog.host.0.borrow().as_slice(), ["show"]);
    assert_eq!(dialog.calls(), ["show", "hide", "yes"]);
}

#[gpui::test]
fn escape_tells_chrome_no(cx: &mut TestAppContext) {
    let mut dialog = Asking::chrome("Add extension", cx);
    dialog.keys("escape");
    assert_eq!(dialog.calls(), ["show", "hide", "no"]);
}

#[gpui::test]
fn without_an_action_only_cancel_answers(cx: &mut TestAppContext) {
    let mut dialog = Asking::chrome("", cx);
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["show"]);
    dialog.keys("escape");
    assert_eq!(dialog.calls(), ["hide", "no"]);
}

#[gpui::test]
fn a_question_chrome_takes_back_leaves_untold(cx: &mut TestAppContext) {
    let mut dialog = Asking::chrome("Add extension", cx);
    dialog.cx.cx.update(withdraw_chrome);
    assert_eq!(dialog.calls(), ["show", "hide"]);
    // ↩ while it leaves answers nothing.
    dialog.keys("enter");
    assert_eq!(dialog.calls(), Vec::<String>::new());
}

#[gpui::test]
fn chromes_question_waits_for_the_dialog_that_shows(cx: &mut TestAppContext) {
    let mut dialog = Asking::new(cx);
    let host = dialog.host.clone();
    dialog.cx.cx.update(|cx| add_extension("Add extension", &host, cx));
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["show", "hide", "delete", "show"]);
    dialog.keys("escape");
    assert_eq!(dialog.calls(), ["hide", "no"]);
}

#[gpui::test]
fn chromes_next_question_takes_the_place_of_the_last(cx: &mut TestAppContext) {
    let mut dialog = Asking::chrome("Add extension", cx);
    let host = dialog.host.clone();
    dialog.cx.cx.update(|cx| add_extension("Re-enable", &host, cx));
    // The first leaves untold, and the second comes up after it.
    assert_eq!(dialog.calls(), ["show", "hide", "show"]);
    dialog.keys("enter");
    assert_eq!(dialog.calls(), ["hide", "yes"]);
}

#[gpui::test]
fn with_no_dialog_chrome_hears_no_at_once(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    cx.update(|cx| add_extension("Add extension", &host, cx));
    assert_eq!(host.0.take(), ["no"]);
}
