//! Naming a Space, a tab or a folder in a field in place of its name, a new folder as
//! it's made, and a tab's icon.

use gpui::{Context, SharedString, Window, div, prelude::*, px, rgb, rgba};

use super::{
    CancelName, EMOJI_SIZE, FAVICON_SIZE, FORM_PLACEHOLDER, FORM_TEXT, HOVERED, KeepName,
    NEW_FOLDER, Naming, PLATE_INSET, Renaming, SPACE_ICON_SLOT, SPACE_ICON_TEXT, Shell,
};
use crate::text_input::TextInput;
use unicode_segmentation::UnicodeSegmentation;

impl Shell {
    /// Shows the Space with its name in a field, all selected, to type a new one over.
    pub(super) fn rename_space(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(name) = self.spaces.iter().find(|each| each.id == space).map(|each| &each.name)
        else {
            return;
        };
        let name = name.clone();
        self.show_space(space, cx);
        // The field is in the sidebar, which the Library covers.
        self.set_library(false, cx);
        // The form names a Space it made.
        if self.form.as_ref().is_some_and(|form| form.space == space) {
            return;
        }
        log!("browser: rename space {space}");
        let name = cx.new(|cx| {
            TextInput::new("Space name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(name)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Space(space),
            name,
            _blur: None,
            _edit: None,
        });
        // As for the form.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows the tab's title in a field, all selected, to type a name over.
    pub(super) fn rename_tab(&mut self, tab: u64, cx: &mut Context<Self>) {
        log!("browser: rename tab {tab}");
        let title = self.title(tab);
        let name = cx.new(|cx| {
            TextInput::new("Tab name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(title)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Tab(tab),
            name,
            _blur: None,
            _edit: None,
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows a field in the tab's icon's place, with the emoji it has, if any, selected, and the
    /// system's character palette to pick one from.
    pub(super) fn change_icon(&mut self, tab: u64, cx: &mut Context<Self>) {
        let icon = self.page(tab).icon;
        let name = cx.new(|cx| TextInput::new("", cx).with_text(icon));
        self.renaming = Some(Renaming {
            naming: Naming::Icon(tab),
            name,
            _blur: None,
            _edit: None,
        });
        // As for a tab's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows a field before the Space's name, with the emoji it has, if any, selected, and the
    /// system's character palette to pick one from.
    pub(super) fn change_space_icon(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(icon) = self.spaces.iter().find(|each| each.id == space).map(|each| &each.icon)
        else {
            return;
        };
        let icon = icon.clone();
        self.show_space(space, cx);
        // The field is in the sidebar, which the Library covers.
        self.set_library(false, cx);
        // The form names a Space it made, with no name row.
        if self.form.as_ref().is_some_and(|form| form.space == space) {
            return;
        }
        let name = cx.new(|cx| TextInput::new("", cx).with_text(icon));
        self.renaming = Some(Renaming {
            naming: Naming::SpaceIcon(space),
            name,
            _blur: None,
            _edit: None,
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows the folder's name in a field, all selected, to type a new one over.
    pub(super) fn rename_folder(&mut self, folder: u64, cx: &mut Context<Self>) {
        let name = self.folder_name(folder);
        self.name_folder(folder, name, cx);
    }

    fn name_folder(&mut self, folder: u64, name: SharedString, cx: &mut Context<Self>) {
        let name = cx.new(|cx| {
            TextInput::new("Folder name", cx)
                .placeholder_color(rgba(FORM_PLACEHOLDER))
                .with_text(name)
        });
        self.renaming = Some(Renaming {
            naming: Naming::Folder(folder),
            name,
            _blur: None,
            _edit: None,
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Makes a folder at the bottom of the pinned tabs of the Space shown, with its name in a field
    /// to type over.
    pub(super) fn new_folder(&mut self, cx: &mut Context<Self>) {
        let name = SharedString::from(NEW_FOLDER);
        let folder = self.browser.create_folder(self.space, &name, 0, usize::MAX, cx);
        if folder != 0 {
            self.name_folder(folder, name, cx);
        }
    }

    /// The folder's name in the Space shown.
    pub(super) fn folder_name(&self, folder: u64) -> SharedString {
        let space = self.shown_space();
        let folder = space.and_then(|space| space.folders.folders.get(&folder));
        folder.map(|(name, _)| name.clone()).unwrap_or_default()
    }

    /// Renames the Space, the tab or the folder to what the field holds, or gives the tab or the
    /// Space its icon, and closes the field. A Space or a folder keeps its name rather than none,
    /// and a tab named nothing shows its page's title again.
    pub(super) fn keep_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let name = renaming.name.read(cx).text().trim().to_string();
        match renaming.naming {
            Naming::Space(space) if !name.is_empty() => self.browser.rename_space(space, &name, cx),
            Naming::Space(_) => {}
            // Left as it was, the title stays the page's, which changes with it.
            Naming::Tab(tab) if *name == *self.title(tab) => {}
            Naming::Tab(tab) => self.browser.rename_tab(tab, &name, cx),
            Naming::Folder(folder) if !name.is_empty() => {
                self.browser.rename_folder(folder, &name, cx)
            }
            Naming::Folder(_) => {}
            // The first character typed or picked; none gives the tab back its favicon.
            Naming::Icon(tab) => {
                let icon = first_character(&name);
                if icon != self.page(tab).icon.as_ref() {
                    self.browser.set_icon(tab, icon, cx);
                }
            }
            // As a tab's; none takes the Space's icon away.
            Naming::SpaceIcon(space) => {
                let icon = first_character(&name);
                let current = self.spaces.iter().find(|each| each.id == space);
                if current.is_some_and(|each| icon != each.icon.as_ref()) {
                    self.browser.set_space_icon(space, icon, cx);
                }
            }
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    pub(super) fn cancel_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.renaming.take().is_some() {
            log!("browser: rename cancelled");
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// The field that renames a tab, in place of its title.
    pub(super) fn naming_field(&self, renaming: &Renaming, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .pr(px(PLATE_INSET))
            .key_context("Naming")
            .on_action(cx.listener(|shell, _: &KeepName, window, cx| shell.keep_name(window, cx)))
            .on_action(
                cx.listener(|shell, _: &CancelName, window, cx| shell.cancel_name(window, cx)),
            )
            .text_size(px(14.))
            .text_color(rgb(FORM_TEXT))
            .child(renaming.name.clone())
    }

    /// The field that changes a tab's icon, in its place.
    pub(super) fn icon_field(&self, renaming: &Renaming, cx: &mut Context<Self>) -> gpui::Div {
        self.emoji_field(renaming, FAVICON_SIZE, EMOJI_SIZE, cx)
    }

    /// The field that changes a Space's icon, before its name.
    pub(super) fn space_icon_field(
        &self,
        renaming: &Renaming,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        self.emoji_field(renaming, SPACE_ICON_SLOT, SPACE_ICON_TEXT, cx)
    }

    /// A field `size` square for one emoji of `text` size.
    fn emoji_field(
        &self,
        renaming: &Renaming,
        size: f32,
        text: f32,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        div()
            .size(px(size))
            .flex()
            .items_center()
            .rounded(px(4.))
            .bg(rgba(HOVERED))
            .overflow_hidden()
            .key_context("Naming")
            .on_action(cx.listener(|shell, _: &KeepName, window, cx| shell.keep_name(window, cx)))
            .on_action(
                cx.listener(|shell, _: &CancelName, window, cx| shell.cancel_name(window, cx)),
            )
            .text_size(px(text))
            .text_color(rgb(FORM_TEXT))
            .child(renaming.name.clone())
    }
}

/// The first character typed or picked into an icon's field: a whole emoji, a flag or a family
/// as one, else nothing.
fn first_character(text: &str) -> &str {
    text.trim().graphemes(true).next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::first_character;

    /// The first character typed or picked is the icon, a whole emoji even when it's several
    /// code points; an emptied field is none, which takes the icon away.
    #[test]
    fn the_first_character_is_the_icon() {
        assert_eq!(first_character("🙂"), "🙂");
        assert_eq!(first_character("🙂🚀"), "🙂");
        assert_eq!(first_character("👨‍👩‍👧 family"), "👨‍👩‍👧");
        assert_eq!(first_character("🇳🇱🇯🇵"), "🇳🇱");
        assert_eq!(first_character("ab"), "a");
        assert_eq!(first_character(""), "");
        assert_eq!(first_character("  "), "");
    }
}
