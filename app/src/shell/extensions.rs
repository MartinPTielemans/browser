//! The extensions of the shown Space's profile, which the menu bar's Extensions menu lists and runs
//! as their toolbar buttons do in Chrome (docs/ux-spec.md › Extensions).

use gpui::{Action, App, Bounds, Context, MenuItem, SharedString, point, px};

use super::{PLATE_INSET, Shell, ShellHandle};

/// An extension of a profile's, one Chrome's extensions page lists.
#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub id: SharedString,
    pub name: SharedString,
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct RunExtension(SharedString);

/// The menu bar's item that runs `extension`.
pub fn menu_item(extension: &Extension) -> MenuItem {
    MenuItem::action(extension.name.clone(), RunExtension(extension.id.clone()))
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
}

impl Shell {
    /// `profile`'s extensions, by name.
    pub fn set_extensions(
        &mut self,
        profile: u64,
        extensions: Vec<Extension>,
        cx: &mut Context<Self>,
    ) {
        // check.sh reads this line.
        let ids: Vec<&str> = extensions.iter().map(|extension| extension.id.as_ref()).collect();
        log!("browser: extensions {profile} {}", ids.join(" "));
        self.extensions.insert(profile, extensions);
        self.update_extensions_menu(cx);
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
    /// Chrome. Its popup hangs under the URL field, where Arc keeps the sidebar's tools, or from
    /// the page's top left corner while the sidebar is hidden.
    pub fn run_extension(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected == 0 {
            return;
        }
        let anchor = self.url_field_bounds().unwrap_or_else(|| {
            let corner = point(px(self.card_left() + PLATE_INSET), px(PLATE_INSET));
            Bounds::new(corner, Default::default())
        });
        let ran = self.browser.run_extension(id, anchor, cx);
        // check.sh waits for this line.
        log!("browser: run extension {id} {}", if ran { "ran" } else { "refused" });
    }
}
