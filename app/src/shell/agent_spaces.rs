//! An agent's Space in the UI, which shows and switches like the user's, and which none of the
//! user's tabs move into, on a host that records what the shell asks.

use std::rc::Rc;

use gpui::{AppContext, Entity, SharedString, TestAppContext, point, px};

use crate::shell::{Command, Folders, Shell, recorder::Recorder};

/// The user's Space 1 with tab 10, shown, and Claude Code's Space 2 with tab 20, and favorite 30.
fn shell_with_agent(cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "Space 1".into(), None, 0, "".into(), "".into()),
                (2, "Research".into(), None, 0, "".into(), "Claude Code".into()),
            ],
            cx,
        );
        shell.set_favorites(vec![(30, 0)], cx);
        shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
        shell.set_tabs(2, Vec::new(), vec![20], Vec::new(), Folders::default(), cx);
        shell.set_space(1, cx);
    });
    (shell, host)
}

/// The items of the menu a right-click on `tab`'s row shows.
fn tab_menu(
    shell: &Entity<Shell>,
    host: &Recorder,
    cx: &mut TestAppContext,
    tab: u64,
) -> Vec<(SharedString, bool)> {
    shell.update(cx, |shell, cx| shell.tab_menu(tab, point(px(100.), px(200.)), cx));
    host.menus.borrow().last().cloned().unwrap_or_default()
}

fn item(menu: &[(SharedString, bool)], title: &str) -> Option<bool> {
    menu.iter().find(|(each, _)| each == title).map(|&(_, enabled)| enabled)
}

/// The Spaces menu lists the agent's Space among the user's, ⌃2 shows it as it does any second
/// Space, and ⌃1 shows the user's again.
#[gpui::test]
fn an_agents_space_shows_and_switches_like_the_users(cx: &mut TestAppContext) {
    let (shell, host) = shell_with_agent(cx);
    assert_eq!(host.menu_bar.borrow().spaces, ["Space 1", "Research"]);
    shell.update(cx, |shell, cx| shell.run(Command::SelectSpace(1), "ui", cx));
    assert_eq!(*host.shown.borrow(), [2]);
    shell.update(cx, |shell, cx| {
        shell.set_space(2, cx);
        shell.run(Command::SelectSpace(0), "ui", cx);
    });
    assert_eq!(*host.shown.borrow(), [2, 1]);
}

/// A tab's Move to lists the user's Spaces and leaves the agent's out, while a tab of the agent's
/// can come to the user's. A favorite can't be taken out of the favorites into the agent's Space.
#[gpui::test]
fn no_tab_of_the_users_moves_into_an_agents_space(cx: &mut TestAppContext) {
    let (shell, host) = shell_with_agent(cx);
    let menu = tab_menu(&shell, &host, cx, 10);
    assert_eq!(item(&menu, "Space 1"), Some(true));
    assert_eq!(item(&menu, "Research"), None);
    assert_eq!(item(&tab_menu(&shell, &host, cx, 30), "Remove from Favorites"), Some(true));
    shell.update(cx, |shell, cx| shell.set_space(2, cx));
    let menu = tab_menu(&shell, &host, cx, 20);
    assert_eq!(item(&menu, "Move to"), Some(true));
    assert_eq!(item(&menu, "Research"), None);
    assert_eq!(item(&menu, "Space 1"), Some(true));
    assert_eq!(item(&tab_menu(&shell, &host, cx, 30), "Remove from Favorites"), Some(false));
}
