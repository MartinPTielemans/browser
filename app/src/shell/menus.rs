//! The sidebar's menus, a Space's, a tab's, a favorite's, a folder's and the plus
//! button's, and what their items do.

use gpui::{Context, Hsla, Pixels, Point, Rgba, SharedString, rgb};

use super::{
    Command, MAX_PANES, ProfileChoice, Row, Shell, UserKeys, clean_link,
    undo::{Prop, Value},
    user_keys,
};
use crate::{
    menu::{Highlight, Look, Swatch},
    space_icon::{SpaceIcon, has_icon},
    theme::Appearance,
};

/// ilo's nouns, which stay capitalized in a sentence, as Chrome's Incognito does.
const PROPER_NOUNS: [&str; 3] = ["Space", "Spaces", "Library"];

/// `title`, written in title case, in sentence case, as Chromium's menus and the Windows and GNOME
/// guidelines write theirs off the Mac, so ilo's menus read as the window's others do there.
fn sentence_case(title: &str) -> String {
    let word = |(i, word): (usize, &str)| {
        let noun = word.trim_end_matches('…');
        let acronym = noun.len() > 1 && noun.chars().all(|c| c.is_ascii_uppercase());
        match i == 0 || acronym || PROPER_NOUNS.contains(&noun) {
            true => word.to_string(),
            false => word.to_lowercase(),
        }
    };
    title.split(' ').enumerate().map(word).collect::<Vec<_>>().join(" ")
}

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
    /// Whether it deletes something, which ilo's menu shows in red.
    pub destructive: bool,
    /// Whether the menu opens with its title at the point it's shown at, highlighted, as a pop-up
    /// button's menu opens over the button's title.
    pub over: bool,
    /// The Space it stands for, whose swatch goes before its title.
    pub swatch: Option<Swatch>,
}

impl MenuEntry {
    /// An item ilo names, in the platform's case.
    pub fn new(title: &'static str, enabled: bool) -> Self {
        MenuEntry::named(MenuEntry::label(title), enabled)
    }

    /// `title`, an item ilo names, in the platform's case, also for a pop-up button showing it.
    pub fn label(title: &'static str) -> SharedString {
        match super::OFF_THE_MAC {
            true => sentence_case(title).into(),
            false => title.into(),
        }
    }

    /// An item named by what it stands for, such as a Space's name, a page's title or Chromium's
    /// item, as written.
    pub fn named(title: impl Into<SharedString>, enabled: bool) -> Self {
        MenuEntry {
            title: title.into(),
            enabled,
            shortcut: None,
            parent: None,
            checked: false,
            destructive: false,
            over: false,
            swatch: None,
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

    pub(super) fn destructive(self) -> Self {
        MenuEntry { destructive: true, ..self }
    }

    fn swatch(self, swatch: Swatch) -> Self {
        MenuEntry { swatch: Some(swatch), ..self }
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
        let items = self.space_items(space);
        self.show_menu(items, position, None, cx);
    }

    /// The menu a right-click on the sidebar's empty space in `space`'s page shows: New Tab, as
    /// ⌘T, then the Space's own menu (ours: Arc's isn't recorded, #433).
    pub(super) fn empty_space_menu(
        &mut self,
        space: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let new_tab = MenuEntry::new("New Tab", true).shortcut(Command::NewTab, user_keys(cx));
        let mut items: Vec<(MenuEntry, MenuAction)> = vec![(
            new_tab,
            // Into the Space it was asked in, which a swipe may not have shown yet.
            Box::new(move |shell, cx| {
                shell.show_space(space, cx);
                shell.run(Command::NewTab, "sidebar", cx)
            }),
        )];
        // A private window's Space has nothing to change.
        if !self.private {
            items.push((MenuEntry::line(), Box::new(|_, _| {})));
            // Its Profile submenu's items name their parent by where it is, now further down.
            let after = items.len();
            items.extend(self.space_items(space).into_iter().map(|(entry, action)| {
                let parent = entry.parent.map(|parent| parent + after);
                (MenuEntry { parent, ..entry }, action)
            }));
        }
        self.show_menu(items, position, None, cx);
    }

    /// The Space's menu's items, for its name, its dot and the sidebar's empty space.
    fn space_items(&self, space: u64) -> Vec<(MenuEntry, MenuAction)> {
        let entry = self.spaces.iter().find(|each| each.id == space);
        let icon = entry.is_some_and(|each| has_icon(&each.icon));
        let theme = entry.is_some_and(|each| each.color.is_some() || each.theme.is_some());
        let mut items = space_menu_head(space, icon, theme);
        let at = items.len();
        items.push((MenuEntry::new("Profile", true), Box::new(|_, _| {})));
        let mut profiles = self.profile_items(space);
        let deletes = self.delete_profile_items();
        if !deletes.is_empty() {
            profiles.push((MenuEntry::line(), Box::new(|_, _| {})));
        }
        profiles.extend(deletes);
        items.extend(profiles.into_iter().map(|(entry, action)| (entry.under(at), action)));
        items.push((MenuEntry::line(), Box::new(|_, _| {})));
        items.push((
            MenuEntry::new("Delete Space", self.spaces.len() > 1).destructive(),
            Box::new(move |shell, cx| shell.delete_space(space, cx)),
        ));
        items
    }

    /// Shows `entries` in ilo's own menu at `position` in the sidebar, in the window or the
    /// reveal's panel, in the Space's look, and calls `chosen` once it closes.
    pub(super) fn context_menu(
        &mut self,
        entries: Vec<MenuEntry>,
        position: Point<Pixels>,
        chosen: Box<dyn FnOnce(Option<usize>, &mut Shell, &mut Context<Shell>)>,
        cx: &mut Context<Self>,
    ) {
        self.context_menu_in(entries, position, self.sidebar_hidden, chosen, cx);
    }

    /// `context_menu` at `position` in the window, or in the reveal's panel if `in_reveal`.
    pub(super) fn context_menu_in(
        &mut self,
        entries: Vec<MenuEntry>,
        position: Point<Pixels>,
        in_reveal: bool,
        chosen: Box<dyn FnOnce(Option<usize>, &mut Shell, &mut Context<Shell>)>,
        cx: &mut Context<Self>,
    ) {
        let look = self.menu_look();
        self.menu_open = true;
        let shell = cx.weak_entity();
        self.browser.context_menu(
            entries,
            position,
            in_reveal,
            look,
            Box::new(move |index, cx| {
                shell
                    .update(cx, |shell, cx| {
                        shell.menu_open = false;
                        chosen(index, shell, cx);
                    })
                    .ok();
            }),
            cx,
        );
    }

    /// The look of the window's menus: light only where the Space's theme asks for it, as the rest
    /// of the window is dark, with the Space's accent under the row the pointer or the keys are on.
    pub fn menu_look(&self) -> Look {
        let light = self.theme_of(self.space).is_some_and(|theme| theme.appearance == Appearance::Light);
        Look { light: light && !self.private, highlight: Highlight::of(&self.palette_of(self.space)) }
    }

    /// Shows `items` at `position` in ilo's menu, or with the one at `over` there, as a pop-up
    /// button's, and does what the one chosen does.
    pub(super) fn show_menu(
        &mut self,
        items: Vec<(MenuEntry, MenuAction)>,
        position: Point<Pixels>,
        over: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.show_menu_in(items, position, over, self.sidebar_hidden, cx);
    }

    /// `show_menu` in the window, or in the reveal's panel if `in_reveal`.
    pub(super) fn show_menu_in(
        &mut self,
        items: Vec<(MenuEntry, MenuAction)>,
        position: Point<Pixels>,
        over: Option<usize>,
        in_reveal: bool,
        cx: &mut Context<Self>,
    ) {
        let entries = (items.iter().enumerate())
            .map(|(at, (entry, _))| MenuEntry { over: Some(at) == over, ..entry.clone() })
            .collect();
        self.context_menu_in(
            entries,
            position,
            in_reveal,
            Box::new(move |chosen, shell, cx| {
                if let Some((_, action)) = chosen.and_then(|at| items.into_iter().nth(at)) {
                    action(shell, cx)
                }
            }),
            cx,
        );
    }

    /// The profiles a Space can be in, its own checked, which only an empty Space can change to, as
    /// its tabs' pages are in its own; and a new one.
    pub(super) fn profile_items(&self, space: u64) -> Vec<(MenuEntry, MenuAction)> {
        let Some((choices, new)) = self.profile_choices(space) else {
            return Vec::new();
        };
        let mut items: Vec<(MenuEntry, MenuAction)> = Vec::new();
        for choice in choices {
            let profile = choice.profile;
            items.push((
                MenuEntry::named(choice.name, choice.enabled).checked(choice.checked),
                Box::new(move |shell, cx| shell.browser.set_space_profile(space, profile, cx)),
            ));
        }
        items.push((MenuEntry::line(), Box::new(|_, _| {})));
        items.push((
            MenuEntry::new("New Profile", new),
            Box::new(move |shell, cx| shell.new_profile(space, cx)),
        ));
        items
    }

    /// The profiles the Space can be in, as its menu and Spaces › Change Profile list them, and
    /// whether it can go in a new one: only an empty Space changes profile.
    pub(super) fn profile_choices(&self, space: u64) -> Option<(Vec<ProfileChoice>, bool)> {
        let entry = self.spaces.iter().find(|each| each.id == space)?;
        let (current, empty) = (entry.profile, entry.pinned.is_empty() && entry.today.is_empty());
        let choices = (self.profiles.iter())
            .map(|(profile, name)| ProfileChoice {
                profile: *profile,
                name: name.clone(),
                enabled: empty || *profile == current,
                checked: *profile == current,
            })
            .collect();
        Some((choices, empty))
    }

    /// Spaces › Change Profile's choice: puts the Space shown in `profile`, or with none in a new
    /// one, as its menu's Profile items do. Agents reach the same through `bc_space_set_profile`.
    pub(super) fn change_profile(&mut self, profile: Option<u64>, cx: &mut Context<Self>) {
        let Some((choices, new)) = self.profile_choices(self.space) else {
            return;
        };
        match profile {
            Some(profile) if choices.iter().any(|each| each.profile == profile && each.enabled) => {
                self.browser.set_space_profile(self.space, profile, cx)
            }
            None if new => self.new_profile(self.space, cx),
            _ => {}
        }
    }

    /// A Delete item for each profile nothing is in.
    fn delete_profile_items(&self) -> Vec<(MenuEntry, MenuAction)> {
        // The one the browser started with stays, as the browser needs one.
        (self.profiles.iter().skip(1))
            .filter(|(profile, _)| {
                !self.spaces.iter().any(|each| each.profile == *profile)
                    && !self.favorite_holders.values().any(|each| each == profile)
            })
            .map(|(profile, name)| {
                let (profile, name) = (*profile, name.clone());
                let entry = MenuEntry::named(format!("Delete “{name}”"), true).destructive();
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
    pub(super) fn move_to_space(&mut self, tab: u64, space: u64, cx: &mut Context<Self>) {
        let profile_of =
            |space| self.spaces.iter().find(|each| each.id == space).map(|each| each.profile);
        let to = profile_of(space);
        // The tab's own Space, which isn't the one shown once New Space… shows the new one.
        let from = (self.spaces.iter())
            .find(|each| each.pinned.contains(&tab) || each.today.contains(&tab))
            .map_or(self.space, |each| each.id);
        if profile_of(from) == to || !self.browser.warn_moving_profiles(cx) {
            return self.send_to_space(tab, space, cx);
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
                        shell.send_to_space(tab, space, cx);
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
        self.end_hovers(cx);
        let row = Row::Tab(self.row_of(tab));
        let keys = user_keys(cx);
        let items = if self.favorites.contains(&tab) {
            self.favorite_menu(keys, tab)
        } else if self.is_folder(tab) {
            self.folder_menu(tab)
        } else if self.pinned().contains(&tab) {
            self.pinned_tab_menu(keys, tab)
        } else {
            self.today_tab_menu(keys, tab, position)
        };
        let entries = items.iter().map(|(entry, ..)| entry.clone()).collect();
        self.context_menu(
            entries,
            position,
            Box::new(move |chosen, shell, cx| {
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
            replace_pinned_url(tab, "Replace Favorite URL with Current", page.away_from_pinned_url),
            (
                MenuEntry::new("Duplicate Tab", true),
                "duplicate-tab",
                Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
            ),
            (
                MenuEntry::new("Remove from Favorites", !self.agents_space_shown()),
                "remove-from-favorites",
                // To the top of Today.
                Box::new(move |shell, cx| shell.move_tab_to(tab, false, 0, 0, cx)),
            ),
        ];
        items.push((
            MenuEntry::new("Change Icon…", true),
            "change-icon",
            Box::new(move |shell, cx| shell.change_icon(tab, cx)),
        ));
        if has_icon(&page.icon) {
            items.push(remove_icon(tab));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // One closed already has nothing to close.
        items.push((
            MenuEntry::new("Close Tab", !page.unloaded).shortcut(Command::CloseTab, keys),
            "close-tab",
            Box::new(move |shell, cx| shell.close_tab(tab, false, cx)),
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
                MenuEntry::new("Delete Folder", true).destructive(),
                "delete-folder",
                Box::new(move |shell, cx| shell.delete_folder_now(folder, cx)),
            ),
        ]
    }

    /// A pinned tab's menu (ours: Arc's isn't recorded).
    fn pinned_tab_menu(
        &self,
        keys: &UserKeys,
        tab: u64,
    ) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let page = self.page(tab);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![
            (
                MenuEntry::new("Unpin Tab", true).shortcut(Command::TogglePin, keys),
                "toggle-pin",
                Box::new(move |shell, cx| shell.pin_tab(tab, false, cx)),
            ),
            (
                MenuEntry::new("Add to Favorites", true),
                "add-to-favorites",
                Box::new(move |shell, cx| shell.favorite_tab(tab, usize::MAX, cx)),
            ),
            (
                MenuEntry::new("Back to Pinned URL", page.away_from_pinned_url)
                    .shortcut(Command::BackToPinnedUrl, keys),
                "back-to-pinned-url",
                Box::new(move |shell, cx| shell.browser.back_to_pinned_url(tab, cx)),
            ),
        ];
        // Arc's Pinned URL › Replace Pinned URL with Current and Edit….
        let pinned_url = items.len();
        items.push((MenuEntry::new("Pinned URL", true), "", Box::new(|_, _| {})));
        let (entry, name, action) =
            replace_pinned_url(tab, "Replace Pinned URL with Current", page.away_from_pinned_url);
        items.push((entry.under(pinned_url), name, action));
        items.push((
            MenuEntry::new("Edit…", true).under(pinned_url),
            "edit-pinned-url",
            Box::new(move |shell, cx| shell.edit_pinned_url(tab, cx)),
        ));
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
        if self.panes(tab).len() > 1 {
            items.push(separate(tab));
        }
        // Each other Space, in the switcher's order.
        let others = self.spaces_to_move_to(false);
        let move_to = items.len();
        items.push((MenuEntry::new("Move to Space", !others.is_empty()), "", Box::new(|_, _| {})));
        for (space, name, _) in others {
            items.push((
                MenuEntry::named(name, true).swatch(self.space_swatch(space)).under(move_to),
                "move-to-space",
                Box::new(move |shell, cx| shell.move_to_space(tab, space, cx)),
            ));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // One closed already has nothing to close.
        items.push((
            MenuEntry::new("Close Tab", !page.unloaded).shortcut(Command::CloseTab, keys),
            "close-tab",
            Box::new(move |shell, cx| shell.close_tab(tab, false, cx)),
        ));
        items
    }

    /// Share as the system can: a submenu of its services where it lists them, as the Mac does;
    /// else one Share… that opens its share sheet, as on Windows, which lists none; else nothing,
    /// as on Linux, which has neither, so the menu never holds a Share that leads nowhere. Dimmed
    /// for a page with no link.
    fn push_share(
        &self,
        items: &mut Vec<(MenuEntry, &'static str, MenuAction)>,
        tab: u64,
        link: &str,
        at: Point<Pixels>,
    ) {
        let services = match link.is_empty() {
            true => Vec::new(),
            false => self.browser.share_services(link),
        };
        if services.is_empty() {
            if self.browser.has_share_sheet() {
                let (link, title) = (link.to_string(), self.title(tab));
                items.push((
                    MenuEntry::new("Share…", !link.is_empty()),
                    "share",
                    Box::new(move |shell, cx| {
                        shell.browser.show_share_sheet(tab, &link, &title, at, cx)
                    }),
                ));
            }
            return;
        }
        let share = items.len();
        items.push((MenuEntry::new("Share", true), "", Box::new(|_, _| {})));
        for (at, service) in services.into_iter().enumerate() {
            let (link, title) = (link.to_string(), self.title(tab));
            items.push((
                MenuEntry::named(service, true).under(share),
                "share",
                Box::new(move |shell, cx| shell.browser.share(&link, &title, at, cx)),
            ));
        }
    }

    /// A Today tab's menu, as Arc's (docs/ux-spec.md › Tab list), with Remove Icon once it has
    /// one and Separate from Split View in a split, ours. Arc's Customize Page… is Boosts, which
    /// the browser hasn't. Each item's agent path is the core call it makes, but Copy Link's and
    /// Share's: the clipboard and the system's share services are the user's, outside the
    /// browser, so agents get neither.
    fn today_tab_menu(
        &self,
        keys: &UserKeys,
        tab: u64,
        position: Point<Pixels>,
    ) -> Vec<(MenuEntry, &'static str, MenuAction)> {
        let page = self.page(tab);
        let panes = self.panes(tab);
        let last = panes[panes.len() - 1];
        let below = self.today().iter().skip_while(|&&id| id != last).count() > 1;
        let link = clean_link(&page.url);
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![(
            MenuEntry::new("Copy Link", !link.is_empty()).shortcut(Command::CopyUrl, keys),
            "copy-link",
            Box::new(move |shell, cx| shell.copy_link(tab, cx)),
        )];
        self.push_share(&mut items, tab, &link, position);
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        items.push((
            MenuEntry::new("Change Icon…", true),
            "change-icon",
            Box::new(move |shell, cx| shell.change_icon(tab, cx)),
        ));
        if has_icon(&page.icon) {
            items.push(remove_icon(tab));
        }
        items.push((
            MenuEntry::new("Rename…", true),
            "rename-tab",
            Box::new(move |shell, cx| shell.rename_tab(tab, cx)),
        ));
        // Dimmed while the tab is silent, as Arc's; a muted one unmutes.
        let mute = if page.muted { "Unmute" } else { "Mute" };
        items.push((
            MenuEntry::new(mute, page.audible || page.muted),
            "toggle-mute",
            Box::new(move |shell, cx| shell.toggle_mute(tab, cx)),
        ));
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        items.push((
            MenuEntry::new("Add Split View", panes.len() < MAX_PANES),
            "add-split",
            Box::new(move |shell, cx| shell.add_split_to(tab, cx)),
        ));
        if panes.len() > 1 {
            items.push(separate(tab));
        }
        items.push((
            MenuEntry::new("Duplicate", true),
            "duplicate-tab",
            Box::new(move |shell, cx| shell.browser.duplicate(tab, cx)),
        ));
        // A private window's tabs stay in it. The others: every one of the user's Spaces, the
        // tab's checked, then a new one.
        if !self.private {
            let move_to = items.len();
            items.push((MenuEntry::new("Move to", true), "", Box::new(|_, _| {})));
            for (space, name, shown) in self.spaces_to_move_to(true) {
                let swatch = self.space_swatch(space);
                items.push((
                    MenuEntry::named(name, true).checked(shown).swatch(swatch).under(move_to),
                    "move-to-space",
                    Box::new(move |shell, cx| {
                        if !shown {
                            shell.move_to_space(tab, space, cx)
                        }
                    }),
                ));
            }
            items.push((MenuEntry::line().under(move_to), "", Box::new(|_, _| {})));
            items.push((
                MenuEntry::new("New Space…", true).under(move_to),
                "new-space",
                Box::new(move |shell, cx| shell.new_space_with(Some(tab), cx)),
            ));
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        // Closing a Today tab archives it.
        items.push((
            MenuEntry::new("Archive Tab", true),
            "close-tab",
            Box::new(move |shell, cx| shell.close_tab(tab, false, cx)),
        ));
        items.push((
            MenuEntry::new("Archive Tabs Below", below),
            "close-tabs-below",
            Box::new(move |shell, cx| shell.close_tabs_below(tab, cx)),
        ));
        items
    }

    /// The user's Spaces a tab can move to, by id and name in the switcher's order, and whether
    /// each is the one shown, which `with_shown` lists too. None moves into an agent's.
    fn spaces_to_move_to(&self, with_shown: bool) -> Vec<(u64, SharedString, bool)> {
        (self.spaces.iter())
            .filter(|space| space.agent.is_empty() && (with_shown || space.id != self.space))
            .map(|space| (space.id, space.name.clone(), space.id == self.space))
            .collect()
    }

    /// `space`'s swatch: its theme's first two colours, or its one colour going lighter down as
    /// the window's gradient does, with the untinted palette's accent for a Space without one.
    pub(super) fn space_swatch(&self, space: u64) -> Swatch {
        if let Some(theme) = self.theme_of(space) {
            let colors = theme.colors();
            if let [from, to, ..] = colors[..] {
                return Swatch { from, to };
            }
        }
        let entry = self.spaces.iter().find(|each| each.id == space);
        let color = self.theme_of(space).map(|theme| theme.main_color());
        let untinted = || crate::theme::packed(crate::palette::Palette::of(None).accent);
        let color = color.or(entry.and_then(|each| each.color)).unwrap_or_else(untinted);
        let hsla = Hsla::from(rgb(color));
        let lighter = crate::theme::packed(Rgba::from(Hsla { l: (hsla.l + 0.25).min(1.), ..hsla }));
        Swatch { from: color, to: lighter }
    }

    /// Copy Link: copies `tab`'s URL without its tracking parameters, as ⇧⌘C does the tab
    /// shown's.
    fn copy_link(&mut self, tab: u64, cx: &mut Context<Self>) {
        let link = clean_link(&self.page(tab).url);
        if !link.is_empty() {
            self.copy(&link, "Copied Current URL", cx);
        }
    }

    /// The + button's menu, by Arc's menus' names for what it makes (ours: Arc's + menu isn't
    /// recorded).
    pub(super) fn plus_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let items = [("New Space…", Command::NewSpace), ("New Folder…", Command::NewFolder)];
        let entries = items.iter().map(|&(title, _)| MenuEntry::new(title, true)).collect();
        self.context_menu(
            entries,
            position,
            Box::new(move |chosen, shell, cx| {
                if let Some(&(_, command)) = chosen.and_then(|at| items.get(at)) {
                    shell.run(command, "footer", cx);
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
            Box::new(move |shell, cx| {
                shell.change(Prop::SpaceIcon(space), Value::Text(String::new()), cx)
            }),
        ));
    }
    // After the icon and the name, as in Arc's.
    items.push((
        MenuEntry::new("Edit Theme Color…", true),
        Box::new(move |shell, cx| shell.edit_space_theme(space, cx)),
    ));
    // The way back to no theme, and ilo green, which the picker has none of (ours: Arc's isn't
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

/// The menu item that takes a tab out of its split view, to show alone.
fn separate(tab: u64) -> (MenuEntry, &'static str, MenuAction) {
    (
        MenuEntry::new("Separate from Split View", true),
        "separate-pane",
        Box::new(move |shell, cx| shell.separate_tab(tab, cx)),
    )
}

/// The menu item that makes the page a pinned tab or a favorite is at the one it goes back to,
/// dimmed while it's there.
fn replace_pinned_url(
    tab: u64,
    title: &'static str,
    away: bool,
) -> (MenuEntry, &'static str, MenuAction) {
    (
        MenuEntry::new(title, away),
        "replace-pinned-url",
        Box::new(move |shell, cx| {
            let url = shell.page(tab).url.to_string();
            shell.change(Prop::PinnedUrl(tab), Value::Text(url), cx)
        }),
    )
}

/// The menu item that gives a tab back its favicon.
pub(super) fn remove_icon(tab: u64) -> (MenuEntry, &'static str, MenuAction) {
    (
        MenuEntry::new("Remove Icon", true),
        "remove-icon",
        Box::new(move |shell, cx| shell.change(Prop::TabIcon(tab), Value::Text(String::new()), cx)),
    )
}

#[cfg(test)]
mod empty_space_tests;

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext, point, px};

    use super::{MenuEntry, sentence_case, space_menu_head, space_menu_label};
    use crate::shell::{Folders, Page, Shell, drag::detents::shell_with, recorder::Recorder};

    /// Off the Mac, ilo's items are in sentence case, keeping its nouns Space and Library and
    /// acronyms capitalized.
    #[test]
    fn items_off_the_mac_are_in_sentence_case() {
        let cases = [
            ("Add Split View", "Add split view"),
            ("Archive Tabs Below", "Archive tabs below"),
            ("Copy Link", "Copy link"),
            ("Change Icon…", "Change icon…"),
            ("New Space…", "New Space…"),
            ("Move to Space", "Move to Space"),
            ("Open Library…", "Open Library…"),
            ("Back to Pinned URL", "Back to pinned URL"),
            ("In Each Profile's Spaces", "In each profile's Spaces"),
            ("Duplicate", "Duplicate"),
        ];
        for (title, sentence) in cases {
            assert_eq!(sentence_case(title), sentence);
        }
    }

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

    /// A menu as the host logs it: lines left out, a submenu's items after their parent's title,
    /// and the dimmed and checked ones marked.
    fn logged(entries: &[MenuEntry]) -> Vec<String> {
        let parents: Vec<usize> = entries.iter().filter_map(|entry| entry.parent).collect();
        let mark = |entry: &MenuEntry, title: String| match (entry.enabled, entry.checked) {
            (false, _) => format!("{title} (disabled)"),
            (true, true) => format!("{title} (checked)"),
            (true, false) => title,
        };
        (entries.iter().enumerate())
            .filter(|(at, entry)| !entry.title.is_empty() && !parents.contains(at))
            .map(|(_, entry)| {
                let title = match entry.parent {
                    Some(parent) => format!("{} › {}", entries[parent].title, entry.title),
                    None => entry.title.to_string(),
                };
                mark(entry, title)
            })
            .collect()
    }

    /// What a right-click on `tab`'s row shows.
    fn tab_menu(
        shell: &Entity<Shell>,
        host: &Recorder,
        cx: &mut TestAppContext,
        tab: u64,
    ) -> Vec<String> {
        shell.update(cx, |shell, cx| shell.tab_menu(tab, point(px(100.), px(200.)), cx));
        logged(&host.entries.borrow())
    }

    fn page(url: &str, audible: bool, muted: bool) -> Page {
        Page { url: url.into(), audible, muted, ..Page::default() }
    }

    /// A Today tab's menu is Arc's: its link, its look, its place, then archiving, with Move to
    /// listing the Space it's in checked and New Space… under the rest.
    #[gpui::test]
    fn a_today_tabs_menu_is_arcs(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10, 11], Folders::default());
        *host.share_services.borrow_mut() = vec!["Mail".into(), "Messages".into()];
        let link = page("https://example.com/", false, false);
        shell.update(cx, |shell, cx| shell.set_page(10, link, cx));
        assert_eq!(
            tab_menu(&shell, &host, cx, 10),
            [
                "Copy Link",
                "Share › Mail",
                "Share › Messages",
                "Change Icon…",
                "Rename…",
                "Mute (disabled)",
                "Add Split View",
                "Duplicate",
                "Move to › Work (checked)",
                "Move to › New Space…",
                "Archive Tab",
                "Archive Tabs Below",
            ]
        );
        // Copy Link alone shows its key, as in Arc's.
        let entries = host.entries.borrow();
        let keyed: Vec<&str> = (entries.iter())
            .filter(|entry| entry.shortcut.is_some())
            .map(|entry| entry.title.as_ref())
            .collect();
        assert_eq!(keyed, ["Copy Link"]);
    }

    /// The last tab has none below to archive, and a tab with no page has no link to copy or share.
    /// Move to marks each Space with its swatch, and New Space… with none (#320).
    #[gpui::test]
    fn move_to_shows_each_spaces_swatch(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10, 11], Folders::default());
        tab_menu(&shell, &host, cx, 10);
        let entries = host.entries.borrow();
        let move_to = entries.iter().position(|entry| entry.title == "Move to").unwrap();
        let swatches: Vec<(&str, bool)> = (entries.iter())
            .filter(|entry| entry.parent == Some(move_to) && !entry.title.is_empty())
            .map(|entry| (entry.title.as_ref(), entry.swatch.is_some()))
            .collect();
        assert_eq!(swatches, [("Work", true), ("New Space…", false)]);
        let work = shell.read_with(cx, |shell, _| shell.space_swatch(shell.space));
        assert_eq!(entries[move_to + 1].swatch, Some(work));
    }

    #[gpui::test]
    fn the_last_tabs_menu_dims_what_it_cant_do(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[], &[10, 11], Folders::default());
        host.share_sheet.set(true);
        let menu = tab_menu(&shell, &host, cx, 11);
        assert_eq!(menu[..2], ["Copy Link (disabled)", "Share… (disabled)"]);
        assert_eq!(menu.last().unwrap(), "Archive Tabs Below (disabled)");
    }

    /// Where the system lists its share services, as the Mac's does, Share is a submenu of them,
    /// and each shares the page with its service.
    #[gpui::test]
    fn share_lists_the_systems_services(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10, 11], Folders::default());
        *host.share_services.borrow_mut() = vec!["Mail".into(), "Messages".into()];
        host.share_sheet.set(true);
        let link = page("https://example.com/", false, false);
        shell.update(cx, |shell, cx| shell.set_page(10, link, cx));
        let menu = tab_menu(&shell, &host, cx, 10);
        assert_eq!(menu[1..3], ["Share › Mail", "Share › Messages"]);
        host.take();
        cx.update(|cx| host.choose("Messages", cx));
        assert_eq!(host.take(), ["share https://example.com/ with 1"]);
    }

    /// Where the system lists none but has a share sheet, as Windows does, Share… is one item that
    /// opens the sheet for the page, rather than an empty submenu.
    #[gpui::test]
    fn share_opens_the_systems_sheet_when_it_lists_no_services(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10, 11], Folders::default());
        host.share_sheet.set(true);
        let link = page("https://example.com/", false, false);
        shell.update(cx, |shell, cx| shell.set_page(10, link, cx));
        let menu = tab_menu(&shell, &host, cx, 10);
        assert_eq!(menu[..3], ["Copy Link", "Share…", "Change Icon…"]);
        let title = shell.read_with(cx, |shell, _| shell.title(10));
        host.take();
        cx.update(|cx| host.choose("Share…", cx));
        assert_eq!(host.take(), [format!("share sheet 10 https://example.com/ {title}")]);
    }

    /// Where the system has neither, as Linux, the menu has no Share at all.
    #[gpui::test]
    fn share_is_left_out_where_the_system_cant_share(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10, 11], Folders::default());
        let link = page("https://example.com/", false, false);
        shell.update(cx, |shell, cx| shell.set_page(10, link, cx));
        let menu = tab_menu(&shell, &host, cx, 10);
        assert_eq!(menu[..2], ["Copy Link", "Change Icon…"]);
        assert!(!menu.iter().any(|item| item.starts_with("Share")));
    }

    /// Mute is dimmed while the tab is silent, and a muted tab's menu unmutes it.
    #[gpui::test]
    fn mute_follows_the_tabs_sound(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[], &[10], Folders::default());
        let url = "https://example.com/";
        shell.update(cx, |shell, cx| shell.set_page(10, page(url, true, false), cx));
        assert!(tab_menu(&shell, &host, cx, 10).contains(&"Mute".to_string()));
        shell.update(cx, |shell, cx| shell.set_page(10, page(url, false, true), cx));
        assert!(tab_menu(&shell, &host, cx, 10).contains(&"Unmute".to_string()));
    }

    /// A pinned tab's menu keeps its own items: it unpins, goes back to its URL and closes.
    #[gpui::test]
    fn a_pinned_tabs_menu_keeps_its_items(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10], Folders::default());
        assert_eq!(
            tab_menu(&shell, &host, cx, 1),
            [
                "Unpin Tab",
                "Add to Favorites",
                "Back to Pinned URL (disabled)",
                "Pinned URL › Replace Pinned URL with Current (disabled)",
                "Pinned URL › Edit…",
                "Rename Tab",
                "Change Icon…",
                "Duplicate Tab",
                "Move to Space (disabled)",
                "Close Tab",
            ]
        );
    }

    /// Away from its pinned URL, a pinned tab's Replace Pinned URL with Current makes the page it's
    /// at the one it goes back to, and a favorite's Replace Favorite URL with Current does the same.
    #[gpui::test]
    fn replace_with_current_keeps_the_page_its_at(cx: &mut TestAppContext) {
        let (shell, host) = shell_with(cx, &[1], &[10], Folders::default());
        let away = Page { away_from_pinned_url: true, ..page("https://away.test/", false, false) };
        shell.update(cx, |shell, cx| shell.set_page(1, away.clone(), cx));
        tab_menu(&shell, &host, cx, 1);
        host.take_edits();
        cx.update(|cx| host.choose("Replace Pinned URL with Current", cx));
        assert_eq!(host.take_edits(), ["pinned url 1 https://away.test/"]);
        shell.update(cx, |shell, cx| {
            shell.set_favorites(vec![(20, 0)], cx);
            shell.set_page(20, away, cx)
        });
        let menu = tab_menu(&shell, &host, cx, 20);
        assert_eq!(menu[..2], ["Back to Favorite URL", "Replace Favorite URL with Current"]);
        assert!(menu.contains(&"Change Icon…".to_string()), "{menu:?}");
        cx.update(|cx| host.choose("Replace Favorite URL with Current", cx));
        assert_eq!(host.take_edits(), ["pinned url 20 https://away.test/"]);
    }
}
