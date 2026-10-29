//! The note on how to leave full screen, which the window shows as it goes full screen and again
//! for each Space shown there, and where the page goes as a page's full screen ends after the
//! window left it, on a host that records what the shell asks.

use std::rc::Rc;

use gpui::{
    AppContext, Bounds, Entity, Pixels, Size, TestAppContext, VisualTestContext, point, px, size,
};

use crate::shell::{
    Command, ESCAPE_AGAIN, EscapePriority, Folders, Shell, change_keys, recorder::Recorder,
};

/// Spaces 1 and 2, with tabs 10 and 20, Space 1 shown.
fn two_spaces(cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "Space 1".into(), None, 0, "".into(), "".into(), "".into()),
                (2, "Space 2".into(), None, 0, "".into(), "".into(), "".into()),
            ],
            cx,
        );
        shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
        shell.set_tabs(2, Vec::new(), vec![20], Vec::new(), Folders::default(), cx);
        shell.set_space(1, cx);
    });
    (shell, host)
}

/// What the shell asked of the note since the last time.
fn notes(host: &Recorder) -> Vec<String> {
    host.take().into_iter().filter(|call| call.starts_with("full screen note")).collect()
}

#[gpui::test]
fn the_note_shows_again_for_each_space_in_full_screen(cx: &mut TestAppContext) {
    let (shell, host) = two_spaces(cx);
    notes(&host);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    assert_eq!(notes(&host), ["full screen note shown"]);
    shell.update(cx, |shell, cx| shell.set_space(2, cx));
    assert_eq!(notes(&host), ["full screen note shown"]);
    // The same Space again isn't another.
    shell.update(cx, |shell, cx| shell.set_space(2, cx));
    assert!(notes(&host).is_empty());
    shell.update(cx, |shell, cx| shell.set_full_screen(false, cx));
    assert_eq!(notes(&host), ["full screen note hidden"]);
    shell.update(cx, |shell, cx| shell.set_space(1, cx));
    assert!(notes(&host).is_empty());
}

const ZOOM: &str = "window button zoom";

#[gpui::test]
fn a_second_escape_leaves_full_screen(cx: &mut TestAppContext) {
    let (shell, host) = two_spaces(cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    host.take();
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "ui", cx));
    assert!(!host.take().contains(&ZOOM.to_string()), "one press left full screen");
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "ui", cx));
    assert_eq!(host.take(), [ZOOM]);

    // Pressed again too late, it counts as a first press.
    shell.update(cx, |shell, cx| {
        shell.run(Command::Stop, "ui", cx);
        shell.escaped = shell.escaped.map(|then| then - ESCAPE_AGAIN);
        shell.run(Command::Stop, "ui", cx);
    });
    assert!(!host.take().contains(&ZOOM.to_string()), "a slow second press left full screen");

    shell.update(cx, |shell, cx| {
        shell.set_full_screen(false, cx);
        shell.run(Command::Stop, "ui", cx);
        shell.run(Command::Stop, "ui", cx);
    });
    assert!(!host.take().contains(&ZOOM.to_string()));
}

/// The page has the first press, and the browser takes the second before the page can.
#[gpui::test]
fn the_page_has_the_first_escape(cx: &mut TestAppContext) {
    let (shell, host) = two_spaces(cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    host.take();
    let first = shell.update(cx, |shell, cx| shell.escape_before_page(cx));
    assert!(!first, "the browser took the page's first ⎋");
    // The page didn't use it, and hands it back.
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "page", cx));
    assert!(!host.take().contains(&ZOOM.to_string()), "one press left full screen");
    let second = shell.update(cx, |shell, cx| shell.escape_before_page(cx));
    assert!(second, "the browser left the second ⎋ to the page");
    assert_eq!(host.take(), [ZOOM]);
}

/// Given the browser first in Settings › Shortcuts, one ⎋ leaves, from the page or the UI, and
/// the page has it too only with Use Both. Outside full screen it's the page's.
#[gpui::test]
fn the_browser_can_have_the_first_escape(cx: &mut TestAppContext) {
    let (shell, host) = two_spaces(cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    host.take();
    let first = |priority, cx: &mut TestAppContext| {
        cx.update(|cx| change_keys(cx, |keys| keys.set_escape_priority(priority)));
        shell.update(cx, |shell, cx| shell.escape_before_page(cx))
    };
    assert!(first(EscapePriority::Browser, cx), "the page had the browser's ⎋");
    assert_eq!(host.take(), [ZOOM]);
    assert!(!first(EscapePriority::Both, cx), "the page didn't have ⎋ with Use Both");
    assert_eq!(host.take(), [ZOOM]);
    // The page's ⎋ handed back doesn't leave again.
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "page", cx));
    assert!(!host.take().contains(&ZOOM.to_string()));

    cx.update(|cx| change_keys(cx, |keys| keys.set_escape_priority(EscapePriority::Browser)));
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "ui", cx));
    assert_eq!(host.take(), [ZOOM]);

    shell.update(cx, |shell, cx| shell.set_full_screen(false, cx));
    host.take();
    assert!(!shell.update(cx, |shell, cx| shell.escape_before_page(cx)));
    assert!(host.take().is_empty());
}

const WINDOWED: Size<Pixels> = size(px(1000.), px(700.));
const SCREEN: Size<Pixels> = size(px(1440.), px(900.));

/// A windowed window showing tab 10, 1000 × 700.
fn window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    let host = Rc::new(Recorder::default());
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into(), "".into())], cx);
        shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
        shell.set_space(1, cx);
        shell.set_selected(10, cx);
    });
    cx.simulate_resize(WINDOWED);
    cx.run_until_parked();
    (shell, cx)
}

/// AppKit takes the window into full screen or out, to its size there, but doesn't say so yet.
fn window_goes(full_screen: bool, cx: &mut VisualTestContext) {
    cx.update(|window, _| {
        if window.is_fullscreen() != full_screen {
            window.toggle_fullscreen();
        }
    });
    cx.simulate_resize(if full_screen { SCREEN } else { WINDOWED });
}

/// The core says the page went full screen, or left.
fn page_goes(full_screen: bool, shell: &Entity<Shell>, cx: &mut VisualTestContext) {
    shell.update_in(cx, |shell, window, cx| shell.set_page_fullscreen(full_screen, window, cx));
}

/// Where the page goes at the window's size: None while it's held for the window's resize, else
/// whether it fills the window or is in the card at the sidebar's edge.
fn page(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Option<&'static str> {
    let viewport = cx.update(|window, _| window.viewport_size());
    shell.update(cx, |shell, _| {
        let (frame, _) = shell.window_page_frame(viewport)?;
        let edge = px(shell.width());
        Some(if frame == Bounds::new(point(px(0.), px(0.)), viewport) {
            "fills"
        } else if shell.card_left() == shell.width() && frame.origin.x == edge {
            "at the sidebar"
        } else {
            "elsewhere"
        })
    })
}

/// The page goes full screen in the windowed window, which follows it.
fn page_takes_the_window(shell: &Entity<Shell>, cx: &mut VisualTestContext) {
    page_goes(true, shell, cx);
    assert_eq!(page(shell, cx), None, "the page moved before the window went full screen");
    window_goes(true, cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    assert_eq!(page(shell, cx), Some("fills"));
}

#[gpui::test]
fn a_page_leaving_after_its_window_left_full_screen_brings_the_card_back(cx: &mut TestAppContext) {
    let (shell, cx) = window(cx);
    page_takes_the_window(&shell, cx);
    // The green button, ⌃⌘F or the display going: the window is out before the core says the page
    // is, so no resize is coming to lay the page out.
    window_goes(false, cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(false, cx));
    page_goes(false, &shell, cx);
    assert_eq!(page(&shell, cx), Some("at the sidebar"));

    // The page's full screen takes the window again, and its own exit brings both back.
    page_takes_the_window(&shell, cx);
    page_goes(false, &shell, cx);
    assert_eq!(page(&shell, cx), None, "the page moved before the window's resize");
    window_goes(false, cx);
    assert_eq!(page(&shell, cx), Some("at the sidebar"));
}

#[gpui::test]
fn a_page_leaving_as_its_window_starts_out_waits_for_the_resize(cx: &mut TestAppContext) {
    let (shell, cx) = window(cx);
    page_takes_the_window(&shell, cx);
    // The core lets the page go as AppKit starts taking the window out, while it's still full
    // screen, so the page stays put until the window's resize lays it out, once.
    page_goes(false, &shell, cx);
    assert_eq!(page(&shell, cx), None, "the page moved before the window's resize");
    window_goes(false, cx);
    assert_eq!(page(&shell, cx), Some("at the sidebar"));
    shell.update(cx, |shell, cx| shell.set_full_screen(false, cx));

    // From there, the page's full screen takes the window again.
    page_takes_the_window(&shell, cx);
}
