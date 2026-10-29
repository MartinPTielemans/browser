//! A page's own menu, from a right-click on it: Chrome's, as the core builds it with its
//! extensions' items and spelling suggestions, shown in ilo's menu over the page (#332).

use gpui::{Context, Pixels, Point, SharedString};

use super::{MenuEntry, Shell};

/// A row of a page's menu, as the core gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct PageMenuItem {
    /// What the core runs once it's chosen.
    pub command: i32,
    /// "" for a line between rows.
    pub title: SharedString,
    /// The row whose submenu it's in, which comes before it.
    pub parent: Option<usize>,
    pub enabled: bool,
    pub checked: bool,
}

/// ilo's menu's entries for the page menu's `items`, one for each, so a choice's index is the
/// row's. A row can only be in the submenu of one before it, as the menu lays out a submenu's
/// parent first; one the core says is in another's goes at the top.
fn entries(items: &[PageMenuItem]) -> Vec<MenuEntry> {
    (items.iter().enumerate())
        .map(|(at, item)| MenuEntry {
            parent: item.parent.filter(|&parent| parent < at),
            checked: item.checked,
            ..MenuEntry::named(item.title.clone(), item.enabled && !item.title.is_empty())
        })
        .collect()
}

impl Shell {
    /// Shows the page menu of `tab`, the core's `items`, with its top left at `at` in the window,
    /// and once it closes tells the core the row chosen, or none.
    pub fn show_page_menu(
        &mut self,
        tab: u64,
        items: Vec<PageMenuItem>,
        at: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if items.is_empty() {
            self.browser.close_page_menu(tab, None, cx);
            return;
        }
        let commands: Vec<i32> = items.iter().map(|item| item.command).collect();
        // Over the page, so in the window even while the sidebar is hidden.
        self.context_menu_in(
            entries(&items),
            at,
            false,
            Box::new(move |chosen, shell, cx| {
                let command = chosen.and_then(|at| commands.get(at).copied());
                shell.browser.close_page_menu(tab, command, cx);
            }),
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{AppContext, TestAppContext, point, px};

    use super::*;
    use crate::shell::{Folders, recorder::Recorder};

    fn row(command: i32, title: &str) -> PageMenuItem {
        PageMenuItem { command, title: title.into(), parent: None, enabled: true, checked: false }
    }

    fn line() -> PageMenuItem {
        row(0, "")
    }

    /// Chrome's menu for a misspelt word in a text field, with its Writing Direction submenu.
    fn field_menu() -> Vec<PageMenuItem> {
        vec![
            row(1, "receive"),
            row(2, "Add to Dictionary"),
            line(),
            PageMenuItem { enabled: false, ..row(3, "Cut") },
            row(4, "Copy"),
            row(5, "Paste"),
            line(),
            row(6, "Writing Direction"),
            PageMenuItem { parent: Some(7), checked: true, ..row(7, "Default") },
            PageMenuItem { parent: Some(7), ..row(8, "Left to Right") },
            line(),
            row(9, "Inspect"),
        ]
    }

    fn window_showing_tab(cx: &mut TestAppContext) -> (gpui::Entity<Shell>, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into(), "".into())], cx);
            shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
            shell.set_space(1, cx);
            shell.set_selected(10, cx);
        });
        host.take();
        (shell, host)
    }

    #[test]
    fn rows_become_the_menus_entries_with_their_lines_checks_and_submenus() {
        let entries = entries(&field_menu());
        let titles: Vec<&str> = entries.iter().map(|entry| entry.title.as_ref()).collect();
        assert_eq!(
            titles,
            [
                "receive",
                "Add to Dictionary",
                "",
                "Cut",
                "Copy",
                "Paste",
                "",
                "Writing Direction",
                "Default",
                "Left to Right",
                "",
                "Inspect",
            ]
        );
        assert_eq!(entries[8].parent, Some(7));
        assert_eq!(entries[9].parent, Some(7));
        assert!(entries[8].checked && !entries[9].checked);
        assert!(!entries[3].enabled, "Cut is off with nothing selected");
        assert!(!entries[2].enabled, "a line isn't chosen");
        assert!(entries.iter().all(|entry| entry.shortcut.is_none() && !entry.destructive));
    }

    #[test]
    fn a_row_in_the_submenu_of_one_after_it_goes_at_the_top() {
        let items = vec![PageMenuItem { parent: Some(1), ..row(1, "Early") }, row(2, "Later")];
        assert_eq!(entries(&items)[0].parent, None);
    }

    #[gpui::test]
    fn the_row_chosen_runs_its_command(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| {
            shell.show_page_menu(10, field_menu(), point(px(400.), px(300.)), cx)
        });
        assert_eq!(host.entries.borrow().len(), 12);
        cx.update(|cx| host.choose("receive", cx));
        assert_eq!(host.take(), ["page menu 10 runs 1"]);
    }

    #[gpui::test]
    fn a_row_in_a_submenu_runs_its_own_command(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| {
            shell.show_page_menu(10, field_menu(), point(px(400.), px(300.)), cx)
        });
        cx.update(|cx| host.choose("Left to Right", cx));
        assert_eq!(host.take(), ["page menu 10 runs 8"]);
    }

    #[gpui::test]
    fn the_menu_shows_over_the_page_while_the_sidebar_is_hidden(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| {
            shell.sidebar_hidden = true;
            shell.show_page_menu(10, field_menu(), point(px(400.), px(300.)), cx)
        });
        assert_eq!(*host.menus_in_reveal.borrow(), [false]);
    }

    #[gpui::test]
    #[should_panic(expected = "no Cut in the menu")]
    fn a_disabled_row_cant_be_chosen(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| {
            shell.show_page_menu(10, field_menu(), point(px(400.), px(300.)), cx)
        });
        cx.update(|cx| host.choose("Cut", cx));
    }

    #[gpui::test]
    fn closing_the_menu_without_a_choice_runs_nothing(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| {
            shell.show_page_menu(10, field_menu(), point(px(400.), px(300.)), cx)
        });
        cx.update(|cx| host.dismiss(cx));
        assert_eq!(host.take(), ["page menu 10 closes"]);
    }

    #[gpui::test]
    fn an_empty_menu_closes_at_once(cx: &mut TestAppContext) {
        let (shell, host) = window_showing_tab(cx);
        shell.update(cx, |shell, cx| shell.show_page_menu(10, Vec::new(), point(px(0.), px(0.)), cx));
        assert!(host.menus.borrow().is_empty());
        assert_eq!(host.take(), ["page menu 10 closes"]);
    }
}
