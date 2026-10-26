//! The note on how to leave full screen, which the window shows as it goes full screen and again
//! for each Space shown there, on a host that records what the shell asks.

use std::rc::Rc;

use gpui::{AppContext, Entity, TestAppContext};

use crate::shell::{Command, Folders, Shell, recorder::Recorder};

/// Spaces 1 and 2, with tabs 10 and 20, Space 1 shown.
fn two_spaces(cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "Space 1".into(), None, 0, "".into(), "".into()),
                (2, "Space 2".into(), None, 0, "".into(), "".into()),
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

#[gpui::test]
fn escape_leaves_full_screen_when_nothing_loads(cx: &mut TestAppContext) {
    let (shell, host) = two_spaces(cx);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    host.take();
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "ui", cx));
    assert_eq!(host.take(), ["window button zoom"]);
    shell.update(cx, |shell, cx| {
        shell.set_full_screen(false, cx);
        shell.run(Command::Stop, "ui", cx);
    });
    assert!(!host.take().contains(&"window button zoom".to_string()));
}
