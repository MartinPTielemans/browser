//! The extensions of the shown Space's profile: the menu bar's Extensions menu lists and runs them,
//! as their toolbar buttons do in Chrome, and the pinned ones' actions show at the URL field's end
//! (docs/ux-spec.md › Extensions).

use std::sync::Arc;

use gpui::{
    Action, AnyElement, App, Bounds, Context, FontWeight, Image, MenuItem, MouseButton,
    MouseDownEvent, Pixels, Point, SharedString, div, img, point, prelude::*, px, rgb, rgba, size,
};

use super::{
    MenuAction, MenuEntry, NAV_HOVERED, NAV_RADIUS, PLATE_INSET, PRESSED, Shell, ShellHandle,
    URL_BUTTON_INSET, URL_BUTTON_SIZE, URL_FIELD_TEXT_X, URL_FIELD_TOP,
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
        self.extensions.insert(profile, extensions);
        self.update_extensions_menu(cx);
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

    /// Gives the menu bar the turned-on extensions of the shown Space's profile, once they change,
    /// so a switch between Spaces of one profile doesn't rebuild the menus.
    pub(super) fn update_extensions_menu(&mut self, cx: &mut Context<Self>) {
        let profile = self.shown_space().map_or(0, |space| space.profile);
        let shown: Vec<Extension> = (self.extensions.get(&profile).into_iter().flatten())
            .filter(|extension| extension.enabled)
            .cloned()
            .collect();
        if shown == self.menu_extensions {
            return;
        }
        // check.sh waits for this line.
        let ids: Vec<&str> = shown.iter().map(|extension| extension.id.as_ref()).collect();
        log!("browser: extensions menu {}", ids.join(" "));
        self.browser.set_extensions_menu(&shown, cx);
        self.menu_extensions = shown;
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
    /// Chrome. Its popup hangs from the action's button if it's shown, else under the URL field,
    /// or from the page's top left corner while the sidebar is hidden.
    pub fn run_extension(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected == 0 {
            return;
        }
        let button = (self.extension_actions.iter().take(self.shown_actions()))
            .position(|action| action.id == id)
            .filter(|_| self.url_field_bounds().is_some())
            .map(|index| self.action_bounds(index));
        let anchor = button.or(self.url_field_bounds()).unwrap_or_else(|| {
            let corner = point(px(self.card_left() + PLATE_INSET), px(PLATE_INSET));
            Bounds::new(corner, Default::default())
        });
        let ran = self.browser.run_extension(id, anchor, cx);
        // check.sh waits for this line.
        log!("browser: run extension {id} {}", if ran { "ran" } else { "refused" });
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

    /// How many of the actions show, as many as leave the host its room with the field's button
    /// shown, so they keep their places as it comes and goes. The others are in the Extensions
    /// menu.
    pub(super) fn shown_actions(&self) -> usize {
        let field = self.width() - 2. * PLATE_INSET;
        let room = field - (URL_FIELD_TEXT_X - PLATE_INSET) - HOST_ROOM - URL_BUTTON_INSET;
        let fit = ((room - URL_BUTTON_SIZE) / URL_BUTTON_SIZE).floor().max(0.) as usize;
        fit.min(self.extension_actions.len())
    }

    /// Action `index`'s button in the window, the shown ones in a row at the URL field's end.
    fn action_bounds(&self, index: usize) -> Bounds<Pixels> {
        let end = self.width() - PLATE_INSET - URL_BUTTON_INSET;
        let left = end - (self.shown_actions() - index) as f32 * URL_BUTTON_SIZE;
        Bounds::new(
            point(px(left), px(URL_FIELD_TOP + URL_BUTTON_INSET)),
            size(px(URL_BUTTON_SIZE), px(URL_BUTTON_SIZE)),
        )
    }

    /// The shown actions' buttons, for the URL field's end: a click runs one, and a right-click
    /// shows its menu.
    pub(super) fn action_buttons(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let shown = self.shown_actions();
        let actions = self.extension_actions.iter().take(shown).enumerate();
        actions
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
                    .right(px(URL_BUTTON_INSET + (shown - 1 - index) as f32 * URL_BUTTON_SIZE))
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
            .collect()
    }

    /// A pinned action's menu: Unpin, and Manage Extension, which opens its details on Chrome's
    /// extensions page.
    fn action_menu(&mut self, id: SharedString, position: Point<Pixels>, cx: &mut Context<Self>) {
        let manage = id.clone();
        let items: Vec<(MenuEntry, MenuAction)> = vec![
            (
                MenuEntry::new("Unpin", true),
                Box::new(move |shell, cx| shell.toggle_pinned(&id, cx)),
            ),
            (
                MenuEntry::new("Manage Extension", true),
                Box::new(move |shell, cx| shell.browser.manage_extensions(Some(&manage), cx)),
            ),
        ];
        self.show_menu(items, position, None, cx);
    }
}
