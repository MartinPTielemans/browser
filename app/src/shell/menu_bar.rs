//! What the menu bar shows that changes with the window, and the window's title, which the Window
//! menu lists it by (docs/ux-spec.md › Menus).

use gpui::{Context, SharedString};

use super::{Extension, Shell, menus::space_menu_label};
use crate::theme::Appearance;

/// What the key window's menu bar shows that changes with it.
#[derive(Clone, Default, PartialEq)]
pub struct MenuBar {
    /// The Spaces' names, each after its icon if it has one, for the Spaces menu.
    pub spaces: Vec<SharedString>,
    /// The shown Space's profile's turned-on extensions, for the Extensions menu.
    pub extensions: Vec<Extension>,
    pub sidebar_hidden: bool,
    /// Whether the tab shown is pinned, or None with none shown.
    pub pinned: Option<bool>,
    /// Spaces › Change Profile: each profile by id and name, whether the Space shown can change to
    /// it and whether it's in it, then whether New Profile can be chosen. None in a private window.
    pub profiles: Option<(Vec<ProfileChoice>, bool)>,
    /// View › Appearance's choice, which it checks.
    pub appearance: Appearance,
    /// Whether the window stays on top of other apps' windows, or None where the platform can't.
    pub on_top: Option<bool>,
}

/// A profile a Space can be in, as its menus list it.
#[derive(Clone, PartialEq, Debug)]
pub struct ProfileChoice {
    pub profile: u64,
    pub name: SharedString,
    pub enabled: bool,
    pub checked: bool,
}

impl MenuBar {
    /// View's item for ⌘S, by what it does now, as Arc's.
    pub fn sidebar_title(&self) -> &'static str {
        if self.sidebar_hidden { "Show Sidebar" } else { "Hide Sidebar" }
    }

    /// Tabs' item for ⌘D, by the tab shown, as Arc's, which names both with none shown.
    pub fn pin_title(&self) -> &'static str {
        match self.pinned {
            Some(true) => "Unpin Tab",
            Some(false) => "Pin Tab",
            None => "Pin/Unpin Tab",
        }
    }
}

impl Shell {
    /// What the menu bar shows for this window now.
    fn menu_bar(&self) -> MenuBar {
        let profile = self.shown_space().map_or(0, |space| space.profile);
        // A private window lists neither Spaces nor extensions: its one Space isn't one of them,
        // and Chrome keeps extensions out of a private session.
        let extensions = (self.extensions.get(&profile).into_iter().flatten())
            .filter(|extension| extension.enabled && !self.private)
            .cloned()
            .collect();
        let spaces = (self.spaces.iter())
            .filter(|_| !self.private)
            .map(|space| space_menu_label(&space.icon, &space.name))
            .collect();
        let pinned = (self.selected != 0).then(|| self.pinned().contains(&self.selected));
        MenuBar {
            spaces,
            extensions,
            sidebar_hidden: self.sidebar_hidden,
            pinned,
            profiles: self.profile_choices(self.space).filter(|_| !self.private),
            appearance: self.app_appearance,
            on_top: self.browser.can_stay_on_top().then_some(self.on_top),
        }
    }

    /// Gives the menu bar what it shows now, once that changes, so most tab switches and page
    /// updates leave the menus as they are.
    pub(super) fn update_menus(&mut self, cx: &mut Context<Self>) {
        let bar = self.menu_bar();
        if bar == self.menus {
            return;
        }
        if bar.extensions != self.menus.extensions {
            // check.sh waits for this line.
            let ids: Vec<&str> = bar.extensions.iter().map(|each| each.id.as_ref()).collect();
            log!("browser: extensions menu {}", ids.join(" "));
        }
        self.browser.set_menus(&bar, cx);
        self.menus = bar;
    }

    /// Gives the menu bar this window's menus again, with the keys the commands have now.
    pub(super) fn refresh_menus(&mut self, cx: &mut Context<Self>) {
        // The menu bar is the key window's, so its Undo and Redo name this window's steps.
        self.send_edit_titles(true, cx);
        self.menus = self.menu_bar();
        self.browser.set_menus(&self.menus, cx);
    }

    /// Titles the window by the tab shown, as the sidebar names it, or by its Space with none
    /// shown (Arc), once that changes. AppKit lists the window in the Window menu and Mission
    /// Control by it; the window doesn't draw it.
    pub fn update_title(&mut self, cx: &mut Context<Self>) {
        let title = match self.selected {
            0 => self.shown_space().map(|space| space.name.clone()).unwrap_or_default(),
            tab => self.title(tab),
        };
        if title != self.window_title {
            // check.sh reads this line.
            log!("browser: window title {title}");
            self.browser.set_window_title(&title, cx);
            self.window_title = title;
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::ProfileChoice;
    use crate::shell::{Command, Folders, drag::detents::shell_with};

    /// Tabs › Pin Tab is named by the tab shown, and with none both ways; View's sidebar item by
    /// what it does now.
    #[gpui::test]
    fn the_menu_bar_names_items_by_what_they_do_now(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10], Folders::default());
        assert_eq!(host.menu_bar.borrow().pin_title(), "Pin/Unpin Tab");
        assert_eq!(host.menu_bar.borrow().sidebar_title(), "Hide Sidebar");
        shell.update(cx, |shell, cx| shell.set_selected(10, cx));
        assert_eq!(host.menu_bar.borrow().pin_title(), "Pin Tab");
        shell.update(cx, |shell, cx| shell.set_selected(1, cx));
        assert_eq!(host.menu_bar.borrow().pin_title(), "Unpin Tab");
        shell.update(cx, |shell, cx| shell.run(Command::ToggleSidebar, "ui", cx));
        assert_eq!(host.menu_bar.borrow().sidebar_title(), "Show Sidebar");
    }

    /// Window › Stay On Top keeps the window over other apps' and checks itself, and again lets it
    /// go back among them.
    #[gpui::test]
    fn stay_on_top_turns_on_and_off_with_its_check(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[], &[10], Folders::default());
        assert_eq!(host.menu_bar.borrow().on_top, Some(false));
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::StayOnTop, "ui", cx));
        assert!(host.take().contains(&"on top true".to_string()));
        assert_eq!(host.menu_bar.borrow().on_top, Some(true));
        shell.update(cx, |shell, cx| shell.run(Command::StayOnTop, "ui", cx));
        assert!(host.take().contains(&"on top false".to_string()));
        assert_eq!(host.menu_bar.borrow().on_top, Some(false));
    }

    /// The window is titled by the tab shown, as the sidebar names it, and by its Space with none.
    #[gpui::test]
    fn the_window_is_titled_by_the_tab_shown(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[], &[10], Folders::default());
        assert_eq!(*host.window_title.borrow(), "Work");
        shell.update(cx, |shell, cx| {
            shell.set_title(10, "Example Domain".into(), cx);
            shell.set_selected(10, cx);
        });
        assert_eq!(*host.window_title.borrow(), "Example Domain");
        shell.update(cx, |shell, cx| shell.set_title(10, "Example".into(), cx));
        assert_eq!(*host.window_title.borrow(), "Example");
    }

    /// Spaces › Change Profile checks the Space's profile and offers the others, and New Profile,
    /// only while the Space has no tabs, as its menu's Profile items do.
    #[gpui::test]
    fn change_profile_offers_what_the_spaces_menu_does(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[], &[10], Folders::default());
        shell.update(cx, |shell, cx| {
            shell.set_profiles(vec![(0, "Default".into()), (5, "Work".into())], cx)
        });
        let choice = |profile, name: &str, enabled, checked| ProfileChoice {
            profile,
            name: name.to_string().into(),
            enabled,
            checked,
        };
        let full = vec![choice(0, "Default", true, true), choice(5, "Work", false, false)];
        assert_eq!(host.menu_bar.borrow().profiles, Some((full, false)));
        shell.update(cx, |shell, cx| {
            shell.set_tabs(1, Vec::new(), Vec::new(), Vec::new(), Folders::default(), cx)
        });
        let empty = vec![choice(0, "Default", true, true), choice(5, "Work", true, false)];
        assert_eq!(host.menu_bar.borrow().profiles, Some((empty, true)));
    }
}
