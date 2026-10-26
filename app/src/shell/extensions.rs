//! The extensions of the shown Space's profile: the menu bar's Extensions menu lists and runs them,
//! as their toolbar buttons do in Chrome, the pinned ones' actions show at the URL field's end, and
//! their keyboard commands run from the page and the UI (docs/ux-spec.md › Extensions).

use std::sync::Arc;

use gpui::{
    Action, AnyElement, App, Bounds, Context, FontWeight, Image, Keystroke, MenuItem, MouseButton,
    MouseDownEvent, Pixels, Point, SharedString, div, img, point, prelude::*, px, rgb, rgba, size,
    svg,
};

use super::{
    MenuAction, MenuEntry, NAV, NAV_HOVERED, NAV_RADIUS, PLATE_INSET, PRESSED, Shell,
    ShellHandle, URL_BUTTON_GLYPH, URL_BUTTON_INSET, URL_BUTTON_SIZE, URL_FIELD_TEXT_X,
    URL_FIELD_TOP, UserKeys, user_keys,
};

/// The room the URL field keeps for the host before it shows fewer actions (Ardia).
const HOST_ROOM: f32 = 84.;
/// An action's icon, 16 pt as in Chrome's toolbar, on its button's 30 pt pitch (Dia: 30.25).
const ACTION_ICON: f32 = 16.;
/// An action that doesn't act on the tab is dimmed, as in Chrome.
const ACTION_DIMMED: f32 = 0.4;
/// The badge over the icon's bottom: Chrome's grey when the extension gives no colour, white text.
const BADGE_COLOR: u32 = 0x5f6368;
const BADGE_HEIGHT: f32 = 11.;
const BADGE_TEXT: f32 = 8.5;

/// An extension of a profile's, one Chrome's extensions page lists.
#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub id: SharedString,
    pub name: SharedString,
    pub enabled: bool,
    /// Its action shows at the URL field's end.
    pub pinned: bool,
    /// The page its options are on, or "" for none.
    pub options: SharedString,
    pub commands: Vec<ExtensionCommand>,
}

/// One of an extension's keyboard commands that has a key.
#[derive(Clone, PartialEq, Debug)]
pub struct ExtensionCommand {
    /// Its name in the manifest, or "_execute_action" for its action.
    pub name: SharedString,
    pub description: SharedString,
    /// Its key, as the browser's shortcuts write them: "shift-cmd-y".
    pub keystroke: SharedString,
    /// The key is the one the extension suggested, not one the user set.
    pub suggested: bool,
}

/// A pinned extension's action as the window shows it, for the tab shown.
#[derive(Clone)]
pub struct ExtensionAction {
    pub id: SharedString,
    pub title: SharedString,
    pub icon: Option<Arc<Image>>,
    pub badge: SharedString,
    /// 0xAARRGGBB, or 0 for Chrome's.
    pub badge_color: u32,
    /// It acts on the tab.
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct RunExtension(SharedString);

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct PinExtension(SharedString);

/// The menu bar's item that runs `extension`.
pub fn menu_item(extension: &Extension) -> MenuItem {
    MenuItem::action(extension.name.clone(), RunExtension(extension.id.clone()))
}

/// The menu bar's item that pins `extension`'s action or unpins it, checked while it's pinned.
pub fn pin_menu_item(extension: &Extension) -> MenuItem {
    MenuItem::action(extension.name.clone(), PinExtension(extension.id.clone()))
        .checked(extension.pinned)
}

pub(super) fn bind(cx: &mut App) {
    // While the UI has the keyboard. The page hands its keys over before it sees them.
    cx.intercept_keystrokes(|event, _, cx| {
        if crate::dialog::asking(cx) {
            return;
        }
        let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) else {
            return;
        };
        let keystroke = keystroke_text(&event.keystroke);
        let ran = shell.update(cx, |shell, cx| shell.run_extension_key(&keystroke, "ui", cx));
        if ran.unwrap_or(false) {
            cx.stop_propagation();
        }
    })
    .detach();
    cx.on_action(|run: &RunExtension, cx| {
        if crate::dialog::asking(cx) {
            return;
        }
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell.update(cx, |shell, cx| shell.run_extension(&run.0, cx)).ok();
        }
    });
    cx.on_action(|pin: &PinExtension, cx| {
        if crate::dialog::asking(cx) {
            return;
        }
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell.update(cx, |shell, cx| shell.toggle_pinned(&pin.0, cx)).ok();
        }
    });
}

impl Shell {
    /// `profile`'s extensions, by name.
    pub fn set_extensions(
        &mut self,
        profile: u64,
        extensions: Vec<Extension>,
        cx: &mut Context<Self>,
    ) {
        // check.sh reads these lines.
        let ids: Vec<&str> = extensions.iter().map(|extension| extension.id.as_ref()).collect();
        log!("browser: extensions {profile} {}", ids.join(" "));
        let pinned = extensions.iter().filter(|extension| extension.pinned);
        let pinned: Vec<&str> = pinned.map(|extension| extension.id.as_ref()).collect();
        log!("browser: pinned extensions {profile} {}", pinned.join(" "));
        for extension in extensions.iter().filter(|extension| !extension.commands.is_empty()) {
            let keys: Vec<String> = (extension.commands.iter())
                .map(|command| {
                    let suggested = if command.suggested { " suggested" } else { "" };
                    format!("{}={}{suggested}", command.name, command.keystroke)
                })
                .collect();
            log!("browser: extension keys {} {}", extension.id, keys.join(", "));
        }
        self.extensions.insert(profile, extensions);
        self.update_menus(cx);
    }

    /// The pinned actions the window shows, for its tab shown.
    pub fn set_extension_actions(&mut self, actions: Vec<ExtensionAction>, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        let shown: Vec<String> = (actions.iter())
            .map(|action| match action.badge.is_empty() {
                true => action.id.to_string(),
                false => format!("{}:{}", action.id, action.badge),
            })
            .collect();
        log!("browser: extension actions {}", shown.join(" "));
        self.extension_actions = actions;
        cx.notify();
    }

    /// Gives the menu bar the Spaces' names and the turned-on extensions of the shown Space's
    /// profile, once they change, so a switch between Spaces of one profile doesn't rebuild the
    /// menus.
    pub(super) fn update_menus(&mut self, cx: &mut Context<Self>) {
        let profile = self.shown_space().map_or(0, |space| space.profile);
        let shown: Vec<Extension> = (self.extensions.get(&profile).into_iter().flatten())
            .filter(|extension| extension.enabled)
            .cloned()
            .collect();
        let spaces: Vec<SharedString> =
            self.spaces.iter().map(|space| space.name.clone()).collect();
        if shown == self.menu_extensions && spaces == self.menu_spaces {
            return;
        }
        if shown != self.menu_extensions {
            // check.sh waits for this line.
            let ids: Vec<&str> = shown.iter().map(|extension| extension.id.as_ref()).collect();
            log!("browser: extensions menu {}", ids.join(" "));
        }
        self.browser.set_menus(&spaces, &shown, cx);
        self.menu_extensions = shown;
        self.menu_spaces = spaces;
    }

    /// Gives the menu bar the keys the commands have now.
    pub(super) fn refresh_menus(&mut self, cx: &mut Context<Self>) {
        self.browser.set_menus(&self.menu_spaces, &self.menu_extensions, cx);
    }

    /// The name of the extension whose page `url` is, in the shown Space's profile, which the URL
    /// field shows in place of its id, as Chrome's omnibox does.
    pub(super) fn extension_name(&self, url: &str) -> Option<SharedString> {
        let id = url.strip_prefix("chrome-extension://")?.split('/').next()?;
        let profile = self.shown_space()?.profile;
        let extensions = self.extensions.get(&profile)?;
        let extension = extensions.iter().find(|extension| extension.id == id)?;
        Some(extension.name.clone())
    }

    /// Runs the extension's action on the tab shown, as a click on its toolbar button does in
    /// Chrome. Its popup hangs from the action's button if it's shown, or the overflow button it's
    /// behind, else under the URL field, or from the page's top left corner while the sidebar is
    /// hidden.
    pub fn run_extension(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected == 0 {
            return;
        }
        let (shown, overflow) = self.shown_actions();
        let button = (self.extension_actions.iter().position(|action| action.id == id))
            .map(|index| index.min(shown))
            .filter(|&place| place < shown || overflow)
            .filter(|_| self.url_field_bounds().is_some())
            .map(|place| self.action_bounds(place));
        let anchor = button.or(self.url_field_bounds()).unwrap_or_else(|| {
            let corner = point(px(self.card_left() + PLATE_INSET), px(PLATE_INSET));
            Bounds::new(corner, Default::default())
        });
        let ran = self.browser.run_extension(id, anchor, cx);
        // check.sh waits for this line.
        log!("browser: run extension {id} {}", if ran { "ran" } else { "refused" });
    }

    /// Runs the keyboard command that `keystroke` is in the shown Space's profile, if there's one,
    /// and says whether it ran: an action runs as a click on its button does, and another command
    /// runs if its extension is listening.
    pub fn run_extension_key(
        &mut self,
        keystroke: &str,
        via: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let profile = self.shown_space().map_or(0, |space| space.profile);
        let extensions = self.extensions.get(&profile).map_or(&[][..], Vec::as_slice);
        let keys = user_keys(cx);
        let Some((id, name)) = (command_for(keystroke, extensions, keys))
            .map(|(extension, command)| (extension.id.clone(), command.name.clone()))
        else {
            return false;
        };
        let ran = match name.as_ref() {
            "_execute_action" => {
                self.run_extension(&id, cx);
                true
            }
            name => self.browser.run_extension_command(&id, name, cx),
        };
        // check.sh waits for this line.
        let result = if ran { "ran" } else { "refused" };
        log!("browser: extension command {id} {name} {result} via {via}");
        ran
    }

    /// Pins the extension's action in the shown Space's profile, or unpins it.
    fn toggle_pinned(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(profile) = self.shown_space().map(|space| space.profile) else {
            return;
        };
        let extensions = self.extensions.get(&profile).into_iter().flatten();
        let pinned = extensions.filter(|extension| extension.id == id).any(|each| each.pinned);
        // check.sh waits for this line.
        log!("browser: {} extension {id}", if pinned { "unpin" } else { "pin" });
        self.browser.set_extension_pinned(profile, id, !pinned, cx);
    }

    /// How many of the actions show, and whether the overflow button follows them, in the places
    /// that leave the host its room with the field's button shown, so they keep their places as
    /// it comes and goes.
    pub(super) fn shown_actions(&self) -> (usize, bool) {
        let field = self.width() - 2. * PLATE_INSET;
        let room = field - (URL_FIELD_TEXT_X - PLATE_INSET) - HOST_ROOM - URL_BUTTON_INSET;
        let places = ((room - URL_BUTTON_SIZE) / URL_BUTTON_SIZE).floor().max(0.) as usize;
        fit_actions(places, self.extension_actions.len())
    }

    /// Place `index`'s button in the window, the places in a row at the URL field's end.
    fn action_bounds(&self, index: usize) -> Bounds<Pixels> {
        let (shown, overflow) = self.shown_actions();
        let end = self.width() - PLATE_INSET - URL_BUTTON_INSET;
        let left = end - (shown + overflow as usize - index) as f32 * URL_BUTTON_SIZE;
        Bounds::new(
            point(px(left), px(URL_FIELD_TOP + URL_BUTTON_INSET)),
            size(px(URL_BUTTON_SIZE), px(URL_BUTTON_SIZE)),
        )
    }

    /// The shown actions' buttons, for the URL field's end: a click runs one, and a right-click
    /// shows its menu. Then the overflow button, whose menu runs the others.
    pub(super) fn action_buttons(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let (shown, overflow) = self.shown_actions();
        let places = shown + overflow as usize;
        let actions = self.extension_actions.iter().take(shown).enumerate();
        let mut buttons: Vec<AnyElement> = actions
            .map(|(index, action)| {
                let id = action.id.clone();
                let menu_id = action.id.clone();
                let badge = (!action.badge.is_empty()).then(|| {
                    let color = match action.badge_color {
                        0 => rgb(BADGE_COLOR),
                        // 0xAARRGGBB to GPUI's 0xRRGGBBAA.
                        argb => rgba(argb << 8 | argb >> 24),
                    };
                    div()
                        .absolute()
                        .right(px(2.))
                        .bottom(px(3.))
                        .h(px(BADGE_HEIGHT))
                        .min_w(px(BADGE_HEIGHT))
                        .px(px(2.))
                        .rounded(px(3.))
                        .bg(color)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(BADGE_TEXT))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xffffff))
                        .child(action.badge.clone())
                });
                div()
                    .id(SharedString::from(format!("action-{}", action.id)))
                    .absolute()
                    .top(px(URL_BUTTON_INSET))
                    .right(px(URL_BUTTON_INSET + (places - 1 - index) as f32 * URL_BUTTON_SIZE))
                    .size(px(URL_BUTTON_SIZE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(NAV_RADIUS))
                    .block_mouse_except_scroll()
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    .active(|style| style.bg(rgba(PRESSED)))
                    .on_click(cx.listener(move |shell, _, _, cx| shell.run_extension(&id, cx)))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                            shell.action_menu(menu_id.clone(), event.position, cx)
                        }),
                    )
                    .children(action.icon.clone().map(|icon| {
                        let icon = img(icon).size(px(ACTION_ICON));
                        icon.when(!action.enabled, |icon| icon.opacity(ACTION_DIMMED))
                    }))
                    .children(badge)
                    .into_any_element()
            })
            .collect();
        if overflow {
            buttons.push(
                div()
                    .id("actions-overflow")
                    .absolute()
                    .top(px(URL_BUTTON_INSET))
                    .right(px(URL_BUTTON_INSET))
                    .size(px(URL_BUTTON_SIZE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(NAV_RADIUS))
                    .block_mouse_except_scroll()
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    .active(|style| style.bg(rgba(PRESSED)))
                    .on_click(cx.listener(|shell, _, _, cx| shell.overflow_menu(cx)))
                    .child(
                        svg()
                            .path("icons/ellipsis.svg")
                            .size(px(URL_BUTTON_GLYPH))
                            .text_color(rgba(NAV)),
                    )
                    .into_any_element(),
            );
        }
        buttons
    }

    /// The overflow button's menu, under it: the actions that don't fit, by title, each of which
    /// runs its action, its popup hanging from the button.
    fn overflow_menu(&mut self, cx: &mut Context<Self>) {
        let (shown, _) = self.shown_actions();
        let items: Vec<(MenuEntry, MenuAction)> = (self.extension_actions.iter().skip(shown))
            .map(|action| {
                let id = action.id.clone();
                let run: MenuAction = Box::new(move |shell, cx| shell.run_extension(&id, cx));
                (MenuEntry::new(action.title.clone(), true), run)
            })
            .collect();
        let button = self.action_bounds(shown);
        self.show_menu(items, button.bottom_left(), None, cx);
    }

    /// A pinned action's menu, as Chrome's toolbar gives one: Unpin, Options if it has them,
    /// Remove, and Manage Extension, which opens its details on Chrome's extensions page.
    fn action_menu(&mut self, id: SharedString, position: Point<Pixels>, cx: &mut Context<Self>) {
        let profile = self.shown_space().map_or(0, |space| space.profile);
        let Some(extension) = (self.extensions.get(&profile).into_iter().flatten())
            .find(|extension| extension.id == id)
            .cloned()
        else {
            return;
        };
        let (unpin, remove, manage) = (id.clone(), id.clone(), id);
        let mut items: Vec<(MenuEntry, MenuAction)> = vec![(
            MenuEntry::new("Unpin", true),
            Box::new(move |shell, cx| shell.toggle_pinned(&unpin, cx)),
        )];
        if !extension.options.is_empty() {
            let options = extension.options.clone();
            items.push((
                MenuEntry::new("Options", true),
                Box::new(move |shell, cx| shell.browser.open_extension_options(&options, cx)),
            ));
        }
        items.extend([
            (
                MenuEntry::new("Remove", true),
                Box::new(move |shell: &mut Shell, cx: &mut Context<Shell>| {
                    shell.remove_extension(profile, remove, extension.name, cx)
                }) as MenuAction,
            ),
            (
                MenuEntry::new("Manage Extension", true),
                Box::new(move |shell, cx| shell.browser.manage_extensions(Some(&manage), cx)),
            ),
        ]);
        self.show_menu(items, position, None, cx);
    }

    /// Removes the extension from the profile once the user confirms, as Chrome asks first.
    fn remove_extension(
        &mut self,
        profile: u64,
        id: SharedString,
        name: SharedString,
        cx: &mut Context<Self>,
    ) {
        let shell = cx.weak_entity();
        self.browser.confirm(
            format!("Remove “{name}”?"),
            "Its settings and the data it keeps go with it.".into(),
            "Remove",
            Box::new(move |cx| {
                shell
                    .update(cx, |shell, cx| {
                        let removed = shell.browser.remove_extension(profile, &id, cx);
                        // check.sh waits for this line.
                        let result = if removed { "removed" } else { "refused" };
                        log!("browser: remove extension {id} {result}");
                    })
                    .ok();
            }),
            cx,
        );
    }
}

/// The command `keystroke` runs among the turned-on `extensions`', as Chrome gives them keys: never
/// one of the browser's reserved shortcuts, and one of its others only if the user set it, as a
/// suggested key that's taken isn't given.
fn command_for<'a>(
    keystroke: &str,
    extensions: &'a [Extension],
    keys: &UserKeys,
) -> Option<(&'a Extension, &'a ExtensionCommand)> {
    if keys.command_for(keystroke, true).is_some() {
        return None;
    }
    let ours = keys.command_for(keystroke, false).is_some();
    extensions.iter().filter(|extension| extension.enabled).find_map(|extension| {
        (extension.commands.iter())
            .find(|command| command.keystroke == keystroke && !(ours && command.suggested))
            .map(|command| (extension, command))
    })
}

/// `keystroke` as the browser's shortcuts write it, with the modifiers in their order.
fn keystroke_text(keystroke: &Keystroke) -> String {
    let modifiers = &keystroke.modifiers;
    let mut text = String::new();
    for (held, name) in [
        (modifiers.control, "ctrl-"),
        (modifiers.alt, "alt-"),
        (modifiers.shift, "shift-"),
        (modifiers.platform, "cmd-"),
    ] {
        if held {
            text.push_str(name);
        }
    }
    text + &keystroke.key
}

/// How many of `count` actions show in `places`, and whether the overflow button takes the last
/// place, for the rest.
fn fit_actions(places: usize, count: usize) -> (usize, bool) {
    match count <= places {
        true => (count, false),
        false => (places.saturating_sub(1), places > 0),
    }
}

#[cfg(test)]
mod tests {
    use super::{Extension, ExtensionCommand, UserKeys, command_for, fit_actions};

    fn extension(id: &str, enabled: bool, commands: &[(&str, &str, bool)]) -> Extension {
        Extension {
            id: id.to_string().into(),
            name: id.to_string().into(),
            enabled,
            pinned: false,
            options: Default::default(),
            commands: (commands.iter())
                .map(|&(name, keystroke, suggested)| ExtensionCommand {
                    name: name.to_string().into(),
                    description: Default::default(),
                    keystroke: keystroke.to_string().into(),
                    suggested,
                })
                .collect(),
        }
    }

    /// A key the user set wins over the browser's, except a reserved one, and a key an extension
    /// suggested never takes one of the browser's.
    #[test]
    fn extension_keys_give_way_to_the_browsers_as_in_chrome() {
        let extensions = [
            extension("off", false, &[("hello", "shift-cmd-y", false)]),
            extension("a", true, &[("hello", "shift-cmd-y", true), ("tab", "cmd-t", false)]),
            extension("b", true, &[("library", "shift-cmd-l", true), ("side", "cmd-s", false)]),
        ];
        let run = |keystroke| {
            command_for(keystroke, &extensions, &UserKeys::default())
                .map(|(extension, command)| format!("{} {}", extension.id, command.name))
        };
        assert_eq!(run("shift-cmd-y").as_deref(), Some("a hello"));
        assert_eq!(run("cmd-t"), None);
        assert_eq!(run("shift-cmd-l"), None);
        assert_eq!(run("cmd-s").as_deref(), Some("b side"));
        assert_eq!(run("alt-shift-p"), None);
    }

    /// Every pinned action has a place to be run from: its own, or the overflow button's.
    #[test]
    fn actions_that_dont_fit_are_behind_the_overflow_button() {
        assert_eq!(fit_actions(2, 0), (0, false));
        assert_eq!(fit_actions(2, 2), (2, false));
        assert_eq!(fit_actions(2, 3), (1, true));
        assert_eq!(fit_actions(1, 5), (0, true));
        // With no place at all, the Extensions menu runs them.
        assert_eq!(fit_actions(0, 1), (0, false));
    }
}
