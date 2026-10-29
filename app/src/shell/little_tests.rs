//! A Mini Window's shell on a host that records what it asks of the core: what closes it, what it
//! leaves out, and where ⌥⌘N, ⌘O and its Space menu send their page.

use std::rc::Rc;

use gpui::{AppContext, Entity, TestAppContext};

use crate::shell::{Command, Folders, Page, Shell, recorder::Recorder};

/// A regular window's shell with the user's two Spaces, the first shown.
fn main_window(cx: &mut TestAppContext, host: &Rc<Recorder>) -> Entity<Shell> {
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "One".into(), Some(7), 1, "".into(), "".into(), "".into()),
                (2, "Two".into(), None, 1, "".into(), "".into(), "".into()),
            ],
            cx,
        );
        shell.set_space(1, cx);
    });
    shell
}

/// A Mini Window for the Space One, its own Space 9 holding the page 10 at `url`.
fn little(cx: &mut TestAppContext, url: &str) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let main = main_window(cx, &host);
    let shell = cx.new(|cx| Shell::little_window(host.clone(), 9, 1, 1, Some(&main), cx));
    shell.update(cx, |shell, cx| {
        shell.set_tabs(9, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
        shell.set_selected(10, cx);
        shell.set_page(10, Page { url: url.to_string().into(), ..Page::default() }, cx);
    });
    host.take();
    (shell, host)
}

fn run(shell: &Entity<Shell>, cx: &mut TestAppContext, command: Command) {
    shell.update(cx, |shell, cx| shell.run(command, "test", cx));
}

#[gpui::test]
fn it_belongs_to_its_spaces_look_and_hides_the_sidebar(cx: &mut TestAppContext) {
    let (shell, _) = little(cx, "https://example.com/");
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.little(), Some(1));
        assert!(shell.sidebar_hidden);
        let own = &shell.spaces[0];
        assert_eq!((own.id, own.name.as_ref(), own.color), (9, "One", Some(7)));
        let users: Vec<u64> = shell.spaces.iter().skip(1).map(|space| space.id).collect();
        assert_eq!(users, [1, 2]);
    });
}

#[gpui::test]
fn command_w_closes_the_window_not_the_tab(cx: &mut TestAppContext) {
    let (shell, host) = little(cx, "https://example.com/");
    run(&shell, cx, Command::CloseTab);
    assert_eq!(host.take(), ["close window"]);
}

#[gpui::test]
fn an_unused_escape_closes_it_unless_the_page_is_loading(cx: &mut TestAppContext) {
    let (shell, host) = little(cx, "https://example.com/");
    shell.update(cx, |shell, cx| {
        let page = Page { url: "https://example.com/".into(), loading: true, ..Page::default() };
        shell.set_page(10, page, cx);
    });
    // Escape stops the load first, as ⌘. does.
    assert!(!shell.update(cx, |shell, cx| shell.escape_unused(cx)));
    shell.update(cx, |shell, cx| {
        shell.set_page(10, Page { url: "https://example.com/".into(), ..Page::default() }, cx)
    });
    assert!(shell.update(cx, |shell, cx| shell.escape_unused(cx)));
    assert_eq!(host.take(), ["close window"]);
}

#[gpui::test]
fn an_escape_in_another_window_is_the_pages(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let main = main_window(cx, &host);
    host.take();
    assert!(!main.update(cx, |shell, cx| shell.escape_unused(cx)));
    assert!(host.take().is_empty());
}

#[gpui::test]
fn the_sidebar_spaces_and_split_view_are_left_out(cx: &mut TestAppContext) {
    let (shell, host) = little(cx, "https://example.com/");
    for command in [
        Command::ToggleSidebar,
        Command::NewSpace,
        Command::NextSpace,
        Command::AddSplit,
        Command::ViewLibrary,
        Command::NewTab,
    ] {
        shell.read_with(cx, |shell, _| assert!(!shell.offers(command)));
        run(&shell, cx, command);
    }
    assert!(host.take().is_empty());
    shell.read_with(cx, |shell, _| assert!(shell.sidebar_hidden));
}

#[gpui::test]
fn command_o_moves_the_page_to_its_space_in_the_main_window(cx: &mut TestAppContext) {
    let (shell, host) = little(cx, "https://example.com/");
    shell.read_with(cx, |shell, _| assert!(shell.offers(Command::OpenInMainWindow)));
    run(&shell, cx, Command::OpenInMainWindow);
    assert_eq!(host.take(), ["open in main window 10 1"]);
}

#[gpui::test]
fn open_in_main_window_is_a_mini_windows_alone(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let main = main_window(cx, &host);
    main.read_with(cx, |shell, _| assert!(!shell.offers(Command::OpenInMainWindow)));
}

#[gpui::test]
fn a_new_mini_window_belongs_to_the_space_it_came_from(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let main = main_window(cx, &host);
    main.update(cx, |shell, cx| shell.set_space(2, cx));
    host.take();
    run(&main, cx, Command::NewLittleWindow);
    assert_eq!(host.take(), ["new little window 2"]);
    let (shell, host) = little(cx, "https://example.com/");
    run(&shell, cx, Command::NewLittleWindow);
    assert_eq!(host.take(), ["new little window 1"]);
}

#[gpui::test]
fn picking_a_space_takes_its_look_and_is_remembered_for_the_site(cx: &mut TestAppContext) {
    let (shell, host) = little(cx, "https://example.com/a");
    shell.update(cx, |shell, cx| shell.pick_little_space(2, cx));
    assert_eq!(host.take(), ["pick little space https://example.com/a 2"]);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.little(), Some(2));
        assert_eq!(shell.spaces[0].name.as_ref(), "Two");
    });
}
