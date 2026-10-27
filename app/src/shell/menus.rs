//! The sidebar's menus, a Space's, a tab's, a favorite's, a folder's and the plus
//! button's, and what their items do.

use gpui::{Context, Pixels, Point, SharedString};

use super::{Command, Row, Shell, UserKeys, user_keys};
use crate::space_icon::SpaceIcon;

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
}

impl MenuEntry {
    pub fn new(title: impl Into<SharedString>, enabled: bool) -> Self {
        MenuEntry {
            title: title.into(),
            enabled,
            shortcut: None,
            parent: None,
            checked: false,
        }
    }

    pub fn line() -> Self {
        MenuEntry::new("", false)
    }

    /// In the submenu of the item at `parent`.
    fn under(self, parent: usize) -> Self {
        MenuEntry { parent: Some(parent), ..self }
    }

    pub(super) fn checked(self, checked: bool) -> Self {
        MenuEntry { checked, ..self }
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
    /// The menu a right-click on a Space's name or dot shows (Arc), with its icon (ours).
    pub(super) fn space_menu(
        &mut self,
        space: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        // A private window's Space has nothing to change.
        if self.private {
            return;
        }
        let entry = self.spaces.iter().find(|each| each.id == space);
        let has_icon = entry.is_some_and(|each| !each.icon.is_empty());
        let has_theme = entry.is_some_and(|each| each.color.is_some() || each.theme.is_some());
        let mut items = space_menu_head(space, has_icon, has_theme);
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
                        // The fill goes as the pointer leaving the row takes it.
                        shell.menu_row = None;
                        if shell.hovered != Some(row) {
                            shell.unhover(row);
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
                MenuEntry::new("Remove from Favorites", !self.agents_space_shown()),
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
        // The user's: none moves into an agent's.
        let others: Vec<(u64, SharedString)> = self
            .spaces
            .iter()
            .filter(|space| space.id != self.space && space.agent.is_empty())
            .map(|space| (space.id, space.name.clone()))
            .collect();
        let pin = MenuEntry::new(if pinned { "Unpin Tab" } else { "Pin Tab" }, true);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = Vec::new();
        // A private window's tabs stay in it, and are never kept.
        if !self.private {
            items.push((
                pin.shortcut(Command::TogglePin, keys),
                "toggle-pin",
                Box::new(move |shell, cx| {
                    let pinned = shell.pinned().contains(&tab);
                    shell.browser.set_pinned(tab, !pinned, cx)
                }),
            ));
            items.push((
                MenuEntry::new("Add to Favorites", true),
                "add-to-favorites",
                Box::new(move |shell, cx| shell.browser.move_to_favorites(tab, usize::MAX, cx)),
            ));
        }
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
        if !self.private {
            items.push((
                MenuEntry::new("Move to Space", !others.is_empty()),
                "",
                Box::new(|_, _| {}),
            ));
        }
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

/// The top of a Space's menu, above its profiles: Rename Space, Change Icon…, Remove Icon once it
/// has one, Edit Theme Color…, and Remove Theme Color once it has a colour.
fn space_menu_head(space: u64, has_icon: bool, has_theme: bool) -> Vec<(MenuEntry, MenuAction)> {
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
    // After the icon and the name, as in Arc's.
    items.push((
        MenuEntry::new("Edit Theme Color…", true),
        Box::new(move |shell, cx| shell.edit_space_theme(space, cx)),
    ));
    // The way back to the system's accent, which the picker has none of (ours: Arc's isn't
    // recorded).
    if has_theme {
        items.push((
            MenuEntry::new("Remove Theme Color", true),
            Box::new(move |shell, cx| shell.remove_space_theme(space, cx)),
        ));
    }
    items
}

/// A Space's item in the menu bar: its name, after its emoji if it has one. GPUI's menu items
/// have no image, so a glyph isn't shown.
pub(super) fn space_menu_label(icon: &str, name: &str) -> SharedString {
    match SpaceIcon::parse(icon) {
        SpaceIcon::Emoji(emoji) => format!("{emoji} {name}").into(),
        SpaceIcon::Dot | SpaceIcon::Glyph(_) => name.to_string().into(),
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

    fn titles(has_icon: bool, has_theme: bool) -> Vec<String> {
        let items = space_menu_head(1, has_icon, has_theme);
        items.into_iter().map(|(entry, _)| entry.title.to_string()).collect()
    }

    /// A Space's menu offers Change Icon… always and Remove Icon only once it has an icon, after
    /// Rename, as a tab's menu does.
    #[test]
    fn remove_icon_is_offered_once_the_space_has_one() {
        let theme = "Edit Theme Color…";
        assert_eq!(titles(false, false), ["Rename Space", "Change Icon…", theme]);
        assert_eq!(titles(true, false), ["Rename Space", "Change Icon…", "Remove Icon", theme]);
    }

    /// Remove Theme Color follows Edit Theme Color… only once the Space has a colour, as Remove
    /// Icon follows Change Icon….
    #[test]
    fn remove_theme_color_is_offered_once_the_space_has_one() {
        let titles = titles(false, true);
        assert_eq!(titles[2..], ["Edit Theme Color…", "Remove Theme Color"]);
    }

    /// The menu bar lists a Space by its name, after its emoji if it has one.
    #[test]
    fn the_menu_bar_names_a_space_after_its_emoji() {
        assert_eq!(space_menu_label("", "Work"), "Work");
        assert_eq!(space_menu_label("🙂", "Work"), "🙂 Work");
        assert_eq!(space_menu_label("glyph:star", "Work"), "Work");
    }
}
