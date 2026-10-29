//! Help › Restore Earlier Sidebar: the core backs up each profile's sidebar as it saves the Spaces,
//! and the menu lists the shown Space's profile's backups for the user to bring one back
//! (docs/ux-spec.md › Menus › Help).

use gpui::Context;

use super::{Shell, SidebarBackup};

impl Shell {
    /// The core listed the profile's sidebar backups, newest first.
    pub fn set_sidebar_backups(
        &mut self,
        profile: u64,
        backups: Vec<SidebarBackup>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar_backups.insert(profile, backups);
        self.update_menus(cx);
    }

    /// Brings back the backup taken at `time` once the user confirms, as it replaces the whole
    /// sidebar, though the one it replaces stays in the same menu.
    pub(super) fn restore_sidebar(&mut self, time: u64, cx: &mut Context<Self>) {
        let Some(space) = self.shown_space().filter(|space| space.agent.is_empty()) else {
            return;
        };
        let profile = space.profile;
        let Some(backup) = (self.sidebar_backups.get(&profile).into_iter().flatten())
            .find(|backup| backup.time == time)
        else {
            return;
        };
        // Named only where there's another profile it could be.
        let whose = match self.profiles.iter().find(|(id, _)| *id == profile) {
            Some((_, name)) if self.profiles.len() > 1 => format!("the “{name}” profile's"),
            _ => "this profile's".into(),
        };
        let shell = cx.weak_entity();
        self.browser.confirm(
            format!("Restore the sidebar from {}?", backup.label),
            format!(
                "Its Spaces, with their tabs, take the place of {whose}. The sidebar as it is now \
                 is kept as a backup, in the same menu."
            ),
            "Restore",
            Box::new(move |cx| {
                shell
                    .update(cx, |shell, cx| {
                        // check.sh waits for this line.
                        log!("browser: restore sidebar {time}");
                        shell.browser.restore_sidebar(time, cx);
                    })
                    .ok();
            }),
            cx,
        );
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use crate::shell::{Command, Folders, SidebarBackup, drag::detents::shell_with};

    fn backup(time: u64, label: &str) -> SidebarBackup {
        SidebarBackup { time, label: label.to_string().into() }
    }

    /// Help lists the shown Space's profile's backups, and one chosen is restored once the user
    /// confirms, never before.
    #[gpui::test]
    fn a_backup_chosen_from_help_is_restored_once_confirmed(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10], Folders::default());
        assert!(host.menu_bar.borrow().sidebar_backups.is_empty());
        let backups = vec![backup(2000, "Thursday 29 October, 14:32"), backup(1000, "Wednesday")];
        shell.update(cx, |shell, cx| shell.set_sidebar_backups(0, backups.clone(), cx));
        assert_eq!(host.menu_bar.borrow().sidebar_backups, backups);
        // Another profile's aren't this Space's.
        shell.update(cx, |shell, cx| shell.set_sidebar_backups(7, vec![backup(5, "x")], cx));
        assert_eq!(host.menu_bar.borrow().sidebar_backups, backups);

        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::RestoreSidebar(2000), "ui", cx));
        let (message, detail, confirmed) = host.confirming.borrow_mut().take().unwrap();
        assert_eq!(message, "Restore the sidebar from Thursday 29 October, 14:32?");
        assert!(detail.contains("this profile's"), "{detail}");
        assert!(!host.take().iter().any(|call| call.starts_with("restore sidebar")));
        cx.update(confirmed);
        assert!(host.take().contains(&"restore sidebar 2000".to_string()));

        // One the core no longer lists asks nothing.
        shell.update(cx, |shell, cx| shell.run(Command::RestoreSidebar(3000), "ui", cx));
        assert!(host.confirming.borrow().is_none());
    }
}
