//! The sidebar's menus, a Space's, a tab's, a favorite's, a folder's and the plus
//! button's, and what their items do.

use std::time::Instant;

use gpui::{Context, Pixels, Point, SharedString};

use super::{Command, Row, SPACE_COLORS, Shell, UserKeys, user_keys};

/// What an item of a menu does once chosen.
pub(super) type MenuAction = Box<dyn FnOnce(&mut Shell, &mut Context<Shell>)>;

/// An item in a menu the host shows. A line between items has no title.
#[derive(Clone)]
pub struct MenuEntry {
    pub title: SharedString,
    pub enabled: bool,
    /// The keystroke it shows, as SHORTCUTS writes it.
    pub shortcut: Option<String>,
    /// The item whose submenu it's in, by index.
    pub parent: Option<usize>,
    /// Whether it shows a check mark.
    pub checked: bool,
    /// A round swatch of this colour, 0xRRGGBB, before its title.
    pub swatch: Option<u32>,
}

impl MenuEntry {
    pub fn new(title: impl Into<SharedString>, enabled: bool) -> Self {
        MenuEntry {
            title: title.into(),
            enabled,
            shortcut: None,
            parent: None,
            checked: false,
            swatch: None,
        }
    }

    pub(super) fn line() -> Self {
        MenuEntry::new("", false)
    }

    /// In the submenu of the item at `parent`.
    fn under(self, parent: usize) -> Self {
        MenuEntry { parent: Some(parent), ..self }
    }

    pub(super) fn checked(self, checked: bool) -> Self {
        MenuEntry { checked, ..self }
    }

    fn swatch(self, color: u32) -> Self {
        MenuEntry { swatch: Some(color), ..self }
    }

    /// Shows the shortcut that runs `command`.
    fn shortcut(self, command: Command, keys: &UserKeys) -> Self {
        let shortcut = keys.keys(command).first().map(|&key| key.to_string());
        MenuEntry { shortcut, ..self }
    }
}

/// A new profile's name as Chrome numbers them, "Profile 2" after the first, that `names` hasn't.
pub fn new_profile_name(names: &[SharedString]) -> String {
    (names.len() + 1..)
        .map(|number| format!("Profile {number}"))
        .find(|name| !names.iter().any(|each| each == name))
        .unwrap_or_default()
}

impl Shell {
    /// The menu a right-click on a Space's name or dot shows (Arc), with its colour and icon
    /// (ours).
    pub(super) fn space_menu(
        &mut self,
        space: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let has_icon = self.spaces.iter().any(|each| each.id == space && !each.icon.is_empty());
        let mut items = space_menu_head(space, has_icon);
        let color_at = items.len();
        items.push((MenuEntry::new("Color", true), Box::new(|_, _| {})));
        let colors = self.color_items(space);
        items.extend(colors.into_iter().map(|(entry, action)| (entry.under(color_at), action)));
        let at = items.len();
        items.push((MenuEntry::new("Profile", true), Box::new(|_, _| {})));
        let mut profiles = self.profile_items(space);
        let deletes = self.delete_profile_items();
        if !deletes.is_empty() {
            profiles.push((MenuEntry::line(), Box::new(|_, _| {})));
        }
        profiles.extend(deletes);
        items.extend(profiles.into_iter().map(|(entry, action)| (entry.under(at), action)));
        items.push((
            MenuEntry::new("Delete Space", self.spaces.len() > 1),
            Box::new(move |shell, cx| shell.delete_space(space, cx)),
        ));
        self.show_menu(items, position, None, cx);
    }

    /// Shows `items` at `position`, or the one at `over` there, and does what the one chosen does.
    pub(super) fn show_menu(
        &self,
        items: Vec<(MenuEntry, MenuAction)>,
        position: Point<Pixels>,
        over: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let entries = items.iter().map(|(entry, _)| entry.clone()).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            over,
            self.sidebar_hidden,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| {
                        if let Some((_, action)) = chosen.and_then(|at| items.into_iter().nth(at)) {
                            action(shell, cx)
                        }
                    })
                    .ok();
            }),
            cx,
        );
    }

    /// The colours a Space can have, its own checked: the system's accent, then ours.
    pub(super) fn color_items(&self, space: u64) -> Vec<(MenuEntry, MenuAction)> {
        let color = self.spaces.iter().find(|each| each.id == space).and_then(|each| each.color);
        let mut items: Vec<(MenuEntry, MenuAction)> = vec![(
            MenuEntry::new("System Accent", true)
                .checked(color.is_none())
                .swatch(self.accent_color),
            Box::new(move |shell, cx| shell.browser.set_space_color(space, None, cx)),
        )];
        for (name, each) in SPACE_COLORS {
            items.push((
                MenuEntry::new(name, true).checked(color == Some(each)).swatch(each),
                Box::new(move |shell, cx| shell.browser.set_space_color(space, Some(each), cx)),
            ));
        }
        items
    }

    /// The profiles a Space can be in, its own checked, which only an empty Space can change to, as
    /// its tabs' pages are in its own; and a new one.
    pub(super) fn profile_items(&self, space: u64) -> Vec<(MenuEntry, MenuAction)> {
        let Some(entry) = self.spaces.iter().find(|each| each.id == space) else {
            return Vec::new();
        };
        let (current, empty) = (entry.profile, entry.pinned.is_empty() && entry.today.is_empty());
        let mut items: Vec<(MenuEntry, MenuAction)> = Vec::new();
        for (profile, name) in self.profiles.clone() {
            items.push((
                MenuEntry::new(name, empty || profile == current).checked(profile == current),
                Box::new(move |shell, cx| shell.browser.set_space_profile(space, profile, cx)),
            ));
        }
        items.push((MenuEntry::line(), Box::new(|_, _| {})));
        items.push((
            MenuEntry::new("New Profile", empty),
            Box::new(move |shell, cx| shell.new_profile(space, cx)),
        ));
        items
    }

    /// A Delete item for each profile nothing is in.
    fn delete_profile_items(&self) -> Vec<(MenuEntry, MenuAction)> {
        // The one the browser started with stays, as the browser needs one.
        (self.profiles.iter().skip(1))
            .filter(|(profile, _)| {
                !self.spaces.iter().any(|each| each.profile == *profile)
                    && !self.favorite_profiles.values().any(|each| each == profile)
            })
            .map(|(profile, name)| {
                let (profile, name) = (*profile, name.clone());
                let entry = MenuEntry::new(format!("Delete “{name}”"), true);
                let action: MenuAction =
                    Box::new(move |shell, cx| shell.delete_profile(profile, name, cx));
                (entry, action)
            })
            .collect()
    }

    /// Deletes the profile once the user confirms, as nothing brings it back.
    fn delete_profile(&mut self, profile: u64, name: SharedString, cx: &mut Context<Self>) {
        let shell = cx.weak_entity();
        self.browser.confirm(
            format!("Delete “{name}”?"),
            "Its history, cookies and logins go with it.".into(),
            "Delete",
            Box::new(move |cx| {
                shell.update(cx, |shell, cx| shell.browser.delete_profile(profile, cx)).ok();
            }),
            cx,
        );
    }

    /// Moves the tab to the Space, warning the first time it goes to another profile's, where its
    /// page loads again without this profile's cookies and logins.
    fn move_to_space(&mut self, tab: u64, space: u64, cx: &mut Context<Self>) {
        let profile_of =
            |space| self.spaces.iter().find(|each| each.id == space).map(|each| each.profile);
        let to = profile_of(space);
        if profile_of(self.space) == to || !self.browser.warn_moving_profiles(cx) {
            return self.browser.move_to_space(tab, space, cx);
        }
        let name = (self.profiles.iter())
            .find(|(profile, _)| Some(*profile) == to)
            .map(|(_, name)| name.clone())
            .unwrap_or_default();
        let shell = cx.weak_entity();
        self.browser.confirm(
            format!("Move this tab to the “{name}” profile?"),
            "Its page loads again there, without this profile's cookies and logins, so a site may \
             ask you to sign in."
                .into(),
            "Move",
            Box::new(move |cx| {
                shell
                    .update(cx, |shell, cx| {
                        shell.browser.set_warn_moving_profiles(false, cx);
                        shell.browser.move_to_space(tab, space, cx);
                    })
                    .ok();
            }),
            cx,
        );
    }

    /// Makes a profile, named as Chrome numbers them, and puts the empty Space in it.
    fn new_profile(&mut self, space: u64, cx: &mut Context<Self>) {
        let names: Vec<SharedString> = self.profiles.iter().map(|(_, name)| name.clone()).collect();
        let name = new_profile_name(&names);
        let profile = self.browser.create_profile(&name, cx);
        // check.sh waits for this line.
        log!("browser: create profile {profile} {name}");
        if profile != 0 {
            self.browser.set_space_profile(space, profile, cx);
        }
    }

    /// The menu a right-click on a tab's row shows: Chrome's tab actions that Arc's sidebar has,
    /// with their shortcuts.
    pub(super) fn tab_menu(&mut self, tab: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.browser.tooltip(None, cx);
        let row = Row::Tab(self.row_of(tab));
        let keys = user_keys(cx);
        let items = if self.favorites.contains(&tab) {
            self.favorite_menu(keys, tab)
        } else if self.is_folder(tab) {
            self.folder_menu(tab)
        } else {
            self.space_tab_menu(keys, tab)
        };
        let entries = items.iter().map(|(entry, ..)| entry.clone()).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            None,
            self.sidebar_hidden,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| {
                        // The fill fades as the pointer leaving the row fades it.
                        shell.menu_row = None;
                        if shell.hovered != Some(row) {
                            shell.unhovered.push((row, Instant::now()));
                        }
                        cx.notify();
                        let item = chosen.and_then(|at| items.into_iter().nth(at));
                        if let Some((_, name, action)) = item {
                            log!("browser: command {name} via tab menu");
                            action(shell, cx)
                        }
                    })
                    .ok();
            }),
            cx,
        );
        self.menu_row = Some(self.row_of(tab));
        cx.notify();
    }

    /// A favorite's menu.
    fn favorite_menu(
        &self,
        keys: &UserKeys,
        tab: u64,
    ) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let page = self.page(tab);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![
            (
                MenuEntry::new("Back to Favorite URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl, keys),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ),
            (
                MenuEntry::new("Duplicate Tab", true),
                "duplicate-tab",
                Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
            ),
            (
                MenuEntry::new("Remove from Favorites", true),
                "remove-from-favorites",
                // To the top of Today.
                Box::new(move |shell, cx| shell.browser.move_tab(tab, false, 0, 0, cx)),
            ),
        ];
        // An icon the tab had as a pinned tab.
        if !page.icon.is_empty() {
            items.push(remove_icon(tab));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // One closed already has nothing to close.
        items.push((
            MenuEntry::new("Close Tab", !page.unloaded).shortcut(Command::CloseTab, keys),
            "close-tab",
            Box::new(move |shell, cx| shell.browser.close(tab, false, cx)),
        ));
        items
    }

    /// A folder's menu.
    fn folder_menu(&self, folder: u64) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        vec![
            (
                MenuEntry::new("Rename Folder", true),
                "rename-folder",
                Box::new(move |shell, cx| shell.rename_folder(folder, cx)),
            ),
            (MenuEntry::line(), "", Box::new(|_, _| {})),
            (
                MenuEntry::new("Delete Folder", true),
                "delete-folder",
                Box::new(move |shell, cx| shell.browser.delete_folder(folder, cx)),
            ),
        ]
    }

    /// The menu of a tab in the Space.
    fn space_tab_menu(
        &self,
        keys: &UserKeys,
        tab: u64,
    ) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let pinned = self.pinned().contains(&tab);
        let page = self.page(tab);
        let panes = self.panes(tab);
        let last = panes[panes.len() - 1];
        let below = self.today().iter().skip_while(|&&id| id != last).count() > 1;
        let others: Vec<(u64, SharedString)> = self
            .spaces
            .iter()
            .filter(|space| space.id != self.space)
            .map(|space| (space.id, space.name.clone()))
            .collect();
        let pin = MenuEntry::new(if pinned { "Unpin Tab" } else { "Pin Tab" }, true);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![
            (
                pin.shortcut(Command::TogglePin, keys),
                "toggle-pin",
                Box::new(move |shell, cx| {
                    let pinned = shell.pinned().contains(&tab);
                    shell.browser.set_pinned(tab, !pinned, cx)
                }),
            ),
            (
                MenuEntry::new("Add to Favorites", true),
                "add-to-favorites",
                Box::new(move |shell, cx| shell.browser.move_to_favorites(tab, usize::MAX, cx)),
            ),
        ];
        if pinned {
            items.push((
                MenuEntry::new("Back to Pinned URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl, keys),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ));
        }
        items.push((
            MenuEntry::new("Rename Tab", true),
            "rename-tab",
            Box::new(move |shell, cx| shell.rename_tab(tab, cx)),
        ));
        items.push((
            MenuEntry::new("Change Icon…", true),
            "change-icon",
            Box::new(move |shell, cx| shell.change_icon(tab, cx)),
        ));
        if !page.icon.is_empty() {
            items.push(remove_icon(tab));
        }
        items.push((
            MenuEntry::new("Duplicate Tab", true),
            "duplicate-tab",
            Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
        ));
        if panes.len() > 1 {
            items.push((
                MenuEntry::new("Separate from Split View", true),
                "separate-pane",
                Box::new(move |shell, cx| shell.browser.separate(tab, cx)),
            ));
        }
        // Each other Space, in the switcher's order.
        let move_to = items.len();
        items.push((
            MenuEntry::new("Move to Space", !others.is_empty()),
            "",
            Box::new(|_, _| {}),
        ));
        for (space, name) in others {
            items.push((
                MenuEntry::new(name, true).under(move_to),
                "move-to-space",
                Box::new(move |shell, cx| shell.move_to_space(tab, space, cx)),
            ));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // A pinned tab closed already has nothing to close.
        let closable = !(page.unloaded && pinned);
        items.push((
            MenuEntry::new("Close Tab", closable).shortcut(Command::CloseTab, keys),
            "close-tab",
            Box::new(move |shell, cx| shell.browser.close(tab, false, cx)),
        ));
        if !pinned {
            items.push((
                MenuEntry::new("Close Tabs Below", below),
                "close-tabs-below",
                Box::new(move |shell, cx| shell.browser.close_below(tab, cx)),
            ));
        }
        items
    }

    /// The + button's menu, by Arc's menus' names for what it makes (ours: Arc's + menu isn't
    /// recorded).
    pub(super) fn plus_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let items = [("New Space…", Command::NewSpace), ("New Folder…", Command::NewFolder)];
        let entries = items.iter().map(|&(title, _)| MenuEntry::new(title, true)).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            None,
            self.sidebar_hidden,
            Box::new(move |chosen, cx| {
                if let Some(&(_, command)) = chosen.and_then(|at| items.get(at)) {
                    shell.update(cx, |shell, cx| shell.run(command, "footer", cx)).ok();
                }
            }),
            cx,
        );
    }
}

/// The top of a Space's menu, above its colours: Rename Space, Change Icon…, and Remove Icon once
/// it has one.
fn space_menu_head(space: u64, has_icon: bool) -> Vec<(MenuEntry, MenuAction)> {
    let mut items: Vec<(MenuEntry, MenuAction)> = vec![
        (
            MenuEntry::new("Rename Space", true),
            Box::new(move |shell, cx| shell.rename_space(space, cx)),
        ),
        (
            MenuEntry::new("Change Icon…", true),
            Box::new(move |shell, cx| shell.change_space_icon(space, cx)),
        ),
    ];
    if has_icon {
        items.push((
            MenuEntry::new("Remove Icon", true),
            Box::new(move |shell, cx| shell.browser.set_space_icon(space, "", cx)),
        ));
    }
    items
}

/// A Space's item in the menu bar: its name, after its icon if it has one.
pub(super) fn space_menu_label(icon: &str, name: &str) -> SharedString {
    match icon.is_empty() {
        true => name.to_string().into(),
        false => format!("{icon} {name}").into(),
    }
}

/// The menu item that gives a tab back its favicon.
pub(super) fn remove_icon(tab: u64) -> (MenuEntry, &'static str, MenuAction) {
    (
        MenuEntry::new("Remove Icon", true),
        "remove-icon",
        Box::new(move |shell, cx| shell.browser.set_icon(tab, "", cx)),
    )
}

#[cfg(test)]
mod tests {
    use super::{space_menu_head, space_menu_label};

    fn titles(has_icon: bool) -> Vec<String> {
        let items = space_menu_head(1, has_icon);
        items.into_iter().map(|(entry, _)| entry.title.to_string()).collect()
    }

    /// A Space's menu offers Change Icon… always and Remove Icon only once it has an icon, after
    /// Rename, as a tab's menu does.
    #[test]
    fn remove_icon_is_offered_once_the_space_has_one() {
        assert_eq!(titles(false), ["Rename Space", "Change Icon…"]);
        assert_eq!(titles(true), ["Rename Space", "Change Icon…", "Remove Icon"]);
    }

    /// The menu bar lists a Space by its name, after its icon if it has one.
    #[test]
    fn the_menu_bar_names_a_space_after_its_icon() {
        assert_eq!(space_menu_label("", "Work"), "Work");
        assert_eq!(space_menu_label("🙂", "Work"), "🙂 Work");
    }
}
