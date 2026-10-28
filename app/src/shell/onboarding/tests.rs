//! A new profile's first launch, on a host that records what the shell asks of the core.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{Entity, Focusable, Modifiers, TestAppContext, VisualTestContext, point, px};

use super::{Drawn, END_FADE, INTRO_END, STEP_MOTION, Step};
use crate::shell::{Folders, Shell, commands::Command, recorder::Recorder};
use crate::tokens::shell::onboarding::SKIP_MARGIN;

/// A drawn shell showing one Space with no tab shown, as at a new profile's first launch, with the
/// keyboard in the UI.
fn first_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, Vec::new(), vec![1], Vec::new(), Folders::default(), cx);
    });
    cx.update(|window, cx| window.focus(&shell.focus_handle(cx), cx));
    cx.run_until_parked();
    (shell, cx, host)
}

/// The step the flow shows, if it shows.
fn step(shell: &Entity<Shell>, cx: &VisualTestContext) -> Option<Step> {
    shell.read_with(cx, |shell, _| shell.onboarding.as_ref().map(|onboarding| onboarding.step))
}

/// Starts the flow, and draws it once, as a launch with nothing to bring back does.
fn started(shell: &Entity<Shell>, cx: &mut VisualTestContext) {
    shell.update(cx, |shell, cx| {
        shell.set_welcome(cx);
        shell.start_onboarding(None, cx);
    });
    cx.run_until_parked();
}

/// Moves the step shown, or the flow's end, `seconds` into the past, and draws.
fn later(shell: &Entity<Shell>, cx: &mut VisualTestContext, seconds: f32) {
    shell.update(cx, |shell, cx| {
        let onboarding = shell.onboarding.as_mut().expect("the flow shows");
        let back = |at: Instant| at - Duration::from_secs_f32(seconds);
        onboarding.since = back(onboarding.since);
        onboarding.ending = onboarding.ending.map(back);
        cx.notify();
    });
    cx.run_until_parked();
}

/// The wordmark draws as a hand writes "ilo": the i's stem, the l's, then the ring, each from
/// nothing; the dot lands only once the ring is closed, takes the green after, and the line comes
/// last. A still intro has all of the wordmark from its first frame, only fading in.
#[test]
fn the_intro_draws_the_wordmark_then_its_line() {
    let at = |t| Drawn::at(t, false);
    assert_eq!(at(0.), Drawn { stem_i: 0., stem_l: 0., ring: 0., dot: 0., signal: 0., tagline: 0. });
    let early = at(0.4);
    assert!(early.stem_i > early.stem_l && early.stem_l > 0., "the stems in order: {early:?}");
    assert_eq!(early.ring, 0., "the ring waits for the stems to be under way");
    // A frame after each part ends, which is when the next may start.
    let ring_closed = at(1.11);
    assert_eq!(ring_closed.ring, 1.);
    assert!(ring_closed.dot < 0.1, "the dot lands once the ring closes: {ring_closed:?}");
    let landed = at(1.35);
    assert_eq!(landed.dot, 1.);
    assert!(landed.signal < 0.1, "the dot lands in the ink: {landed:?}");
    let green = at(1.65);
    assert_eq!(green.signal, 1.);
    assert!(green.tagline < 0.1, "it takes the green before the line comes: {green:?}");
    assert_eq!(at(2.05).tagline, 1.);

    let still = Drawn::at(0., true);
    assert_eq!((still.stem_i, still.ring, still.dot, still.signal), (1., 1., 1., 1.));
    assert_eq!(still.tagline, 0., "with Reduce Motion it fades in rather than drawing");
}

/// A launch that brings Spaces back never starts it: the shell has no flow until it's asked for.
#[gpui::test]
fn a_normal_window_has_no_flow(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    assert_eq!(step(&shell, cx), None);
}

/// Any key skips the intro to the first step, and so does a click anywhere.
#[gpui::test]
fn any_key_or_click_skips_the_intro(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    started(&shell, cx);
    assert_eq!(step(&shell, cx), Some(Step::Intro));
    cx.simulate_keystrokes("a");
    assert_eq!(step(&shell, cx), Some(Step::Import));

    started(&shell, cx);
    let middle = shell.read_with(cx, |shell, _| shell.page_frame(shell.viewport).0.center());
    cx.simulate_click(middle, Modifiers::none());
    assert_eq!(step(&shell, cx), Some(Step::Import));
}

/// Left alone, the intro holds once it's drawn, then the first step comes.
#[gpui::test]
fn the_intro_goes_on_by_itself(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    started(&shell, cx);
    later(&shell, cx, INTRO_END - 0.5);
    assert_eq!(step(&shell, cx), Some(Step::Intro), "it holds after it's drawn");
    later(&shell, cx, 0.5);
    assert_eq!(step(&shell, cx), Some(Step::Import));
}

/// Esc skips a step, and after the last the flow fades out from over the first Space's welcome,
/// which then shows alone.
#[gpui::test]
fn escape_goes_on_and_the_last_step_leaves_the_flow(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    started(&shell, cx);
    cx.simulate_keystrokes("a");
    later(&shell, cx, STEP_MOTION);
    cx.simulate_keystrokes("escape");
    let ending = shell.read_with(cx, |shell, _| shell.onboarding.as_ref().unwrap().ending);
    assert!(ending.is_some(), "Esc on the last step leaves the flow");
    later(&shell, cx, END_FADE);
    assert_eq!(step(&shell, cx), None);
    assert!(shell.read_with(cx, |shell, _| shell.welcome), "the welcome is under it");
}

/// Return is the step's button, and Skip, at the window's bottom right, goes on too.
#[gpui::test]
fn return_and_skip_go_on_from_a_step(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    for go_on in ["return", "skip"] {
        started(&shell, cx);
        cx.simulate_keystrokes("a");
        later(&shell, cx, STEP_MOTION);
        match go_on {
            "return" => cx.simulate_keystrokes("enter"),
            _ => {
                let viewport = shell.read_with(cx, |shell, _| shell.viewport);
                let skip = point(px(SKIP_MARGIN + 30.), px(SKIP_MARGIN + 10.));
                cx.simulate_click(
                    point(viewport.width - skip.x, viewport.height - skip.y),
                    Modifiers::none(),
                );
            }
        }
        let ending = shell.read_with(cx, |shell, _| shell.onboarding.as_ref().unwrap().ending);
        assert!(ending.is_some(), "{go_on} left the last step");
        later(&shell, cx, END_FADE);
    }
}

/// A page shown covers the window, so it ends the flow at once.
#[gpui::test]
fn a_page_shown_ends_it(cx: &mut TestAppContext) {
    let (shell, cx, _) = first_window(cx);
    started(&shell, cx);
    shell.update(cx, |shell, cx| shell.set_selected(1, cx));
    cx.run_until_parked();
    assert_eq!(step(&shell, cx), None);
}

/// Help › Welcome to ilo plays it again: here when the window shows no page, else in a new window,
/// since GPUI can't draw over a page.
#[gpui::test]
fn welcome_to_ilo_plays_it_again(cx: &mut TestAppContext) {
    let (shell, cx, host) = first_window(cx);
    shell.update(cx, |shell, cx| shell.run(Command::Welcome, "menu", cx));
    cx.run_until_parked();
    assert_eq!(step(&shell, cx), Some(Step::Intro));
    assert!(!host.take().contains(&"welcome window".to_string()), "no new window for it");

    shell.update(cx, |shell, cx| shell.set_selected(1, cx));
    cx.run_until_parked();
    shell.update(cx, |shell, cx| shell.run(Command::Welcome, "menu", cx));
    cx.run_until_parked();
    assert_eq!(step(&shell, cx), None);
    assert!(host.take().contains(&"welcome window".to_string()));
}
