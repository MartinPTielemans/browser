//! Naming a Space, a tab or a folder in a field in place of its name, a new folder as
//! it's made; and opening the pickers for a Space's or a tab's icon and a Space's theme.

use crate::tokens::text;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{Context, SharedString, Window, div, point, prelude::*, px, rgb, rgba};

use super::{
    undo::{Prop, Value},
    CancelName, FAVICON_SIZE, FAVICON_X, FOLDER_INDENT, FORM_PLACEHOLDER, FORM_TEXT, KeepName,
    NEW_FOLDER, Naming, PLATE_INSET, Pending, Renaming, SPACE_ICON_SLOT, SPACE_ICON_X, Shell,
    ShellHandle,
};
use crate::{
    icon_picker::{self, Picked, Subject},
    space_icon::has_icon,
    text_input::TextInput,
};

/// How long a picker waits for the name row it points at to stop moving.
const PICKER_WAIT: Duration = Duration::from_secs(2);

impl Shell {
    /// Shows the Space with its name in a field, all selected, to type a new one over.
    pub(super) fn rename_space(&mut self, space: u64, cx: &mut Context<Self>) {
        // A private window's Space is named for what it is.
        if self.private {
            return;
        }
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
        });
        // As for a Space's name.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_renaming = true;
        cx.notify();
    }

    /// Shows the icon picker under the tab's favicon, once its row has stopped moving, with the
    /// sidebar docked.
    pub(super) fn change_icon(&mut self, tab: u64, cx: &mut Context<Self>) {
        if !self.dock(cx) && self.sidebar_hidden {
            self.toggle_sidebar(cx);
        }
        self.picker_pending = Some((Pending::Icon(Subject::Tab(tab)), Instant::now()));
        self.tab_row_drawn.set(None);
        self.name_row_before = None;
        cx.notify();
    }

    /// Shows the Space, and the icon picker under its name once the name row has stopped moving.
    pub(super) fn change_space_icon(&mut self, space: u64, cx: &mut Context<Self>) {
        self.open_picker(space, Pending::Icon(Subject::Space(space)), cx);
    }

    /// Shows the Space, and the theme picker beside its name row once the row has stopped
    /// moving: Arc's Edit Theme Color…, whose picker opened from there isn't recorded.
    pub(super) fn edit_space_theme(&mut self, space: u64, cx: &mut Context<Self>) {
        self.open_picker(space, Pending::Theme(space), cx);
    }

    fn open_picker(&mut self, space: u64, picker: Pending, cx: &mut Context<Self>) {
        if !self.spaces.iter().any(|each| each.id == space) {
            return;
        }
        self.show_space(space, cx);
        // The picker hangs under the sidebar's name row, which the Library covers.
        self.set_library(false, cx);
        // The form names a Space it made, with no name row.
        if self.form.as_ref().is_some_and(|form| form.space == space) {
            return;
        }
        if !self.dock(cx) && self.sidebar_hidden {
            self.toggle_sidebar(cx);
        }
        self.picker_pending = Some((picker, Instant::now()));
        self.name_row_drawn.set(None);
        self.name_row_before = None;
        cx.notify();
    }

    /// Opens the picker asked for once the row it points at has drawn where it drew the frame
    /// before, and the Library has gone, as the sidebar, the Library or a Space's slide may be
    /// moving it. A Space's icon picker points at where its icon goes in the name row, a tab's at
    /// its favicon, and the theme picker at the name row's end. It gives up if the row doesn't
    /// settle in two seconds, such as in a full-screen page, which has no sidebar.
    pub(super) fn open_picker_when_settled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((picker, asked)) = self.picker_pending else {
            return;
        };
        let (drawn, here) = match picker {
            Pending::Icon(Subject::Space(space)) | Pending::Theme(space) => {
                (self.name_row_drawn.get(), space == self.space)
            }
            Pending::Icon(Subject::Tab(_)) => (self.tab_row_drawn.get(), true),
        };
        // Not before the Library has gone and the sidebar has come in, as a frame of either can
        // leave the row where it was.
        let still = self.library.is_none() && self.sidebar_motion.is_none();
        let settled = still && drawn.is_some() && drawn == self.name_row_before && here;
        self.name_row_before = drawn;
        if !settled {
            if asked.elapsed() < PICKER_WAIT {
                window.request_animation_frame();
            } else {
                self.picker_pending = None;
            }
            return;
        }
        self.picker_pending = None;
        // The picker shows over the key window, which may have changed while the row moved.
        let key = cx.try_global::<ShellHandle>().is_some_and(|key| key.0 == cx.weak_entity());
        if !key {
            return;
        }
        let Some(row) = drawn else {
            return;
        };
        let subject = match picker {
            Pending::Icon(subject) => subject,
            Pending::Theme(space) => {
                let anchor = point(f32::from(row.right()), f32::from(row.center().y));
                return self.show_theme_picker(space, anchor, cx);
            }
        };
        let shell = cx.weak_entity();
        let (icon, anchor, picked) = match subject {
            Subject::Space(space) => {
                let Some(icon) = self.spaces.iter().find(|each| each.id == space) else {
                    return;
                };
                let anchor = point(
                    f32::from(row.left()) + SPACE_ICON_X + SPACE_ICON_SLOT / 2.,
                    f32::from(row.center().y),
                );
                let picked: Picked = Rc::new(move |icon, cx| {
                    shell
                        .update(cx, |shell, cx| {
                            shell.change(Prop::SpaceIcon(space), Value::Text(icon.to_string()), cx)
                        })
                        .ok();
                });
                (icon.icon.clone(), anchor, picked)
            }
            Subject::Tab(tab) => {
                // The favicon's middle, in from the row by the folders the tab is in.
                let x = f32::from(row.left())
                    + FAVICON_X
                    + self.depth(tab) as f32 * FOLDER_INDENT
                    + FAVICON_SIZE / 2.;
                let anchor = point(x, f32::from(row.center().y));
                let picked: Picked = Rc::new(move |icon, cx| {
                    shell
                        .update(cx, |shell, cx| {
                            shell.change(Prop::TabIcon(tab), Value::Text(icon.to_string()), cx)
                        })
                        .ok();
                });
                (self.page(tab).icon, anchor, picked)
            }
        };
        let iconless = !has_icon(&icon);
        icon_picker::show(subject, icon, anchor, self.accent_color, picked, cx);
        if let Subject::Space(space) = subject
            && iconless
        {
            self.hold_icon_place(space, cx);
        }
    }

    /// Holds the icon's place in `space`'s name row, which has no icon, with the placeholder, the
    /// name after it, while the icon picker shows for it (Arc), so the arrow points at where the
    /// icon will go.
    pub(super) fn hold_icon_place(&mut self, space: u64, cx: &mut Context<Self>) {
        let Some(picker) = icon_picker::entity(cx) else {
            return;
        };
        let watch = cx.observe(&picker, move |shell, picker, cx| {
            if picker.read(cx).picking().map(|(subject, _)| subject) != Some(Subject::Space(space))
            {
                shell.icon_placeholder = None;
                cx.notify();
            }
        });
        self.icon_placeholder = Some((space, watch));
        cx.notify();
    }

    /// Whether the shown Space's name row holds the icon's place with the placeholder: while the
    /// icon picker shows for it, until it has an icon.
    pub(super) fn holds_icon_place(&self) -> bool {
        self.icon_placeholder.as_ref().is_some_and(|(space, _)| *space == self.space)
            && self.shown_space().is_some_and(|space| !has_icon(&space.icon))
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

    /// Renames the Space, the tab or the folder to what the field holds, and closes the field. A
    /// Space or a folder keeps its name rather than none, and a tab named nothing shows its page's
    /// title again.
    pub(super) fn keep_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let name = renaming.name.read(cx).text().trim().to_string();
        match renaming.naming {
            Naming::Space(space) if !name.is_empty() => {
                self.change(Prop::SpaceName(space), Value::Text(name), cx)
            }
            Naming::Space(_) => {}
            // Left as it was, the title stays the page's, which changes with it.
            Naming::Tab(tab) if *name == *self.title(tab) => {}
            Naming::Tab(tab) => self.change(Prop::TabName(tab), Value::Text(name), cx),
            Naming::Folder(folder) if !name.is_empty() => {
                self.change(Prop::FolderName(folder), Value::Text(name), cx)
            }
            Naming::Folder(_) => {}
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
            .text_size(px(text::BODY))
            .text_color(rgb(FORM_TEXT))
            .child(renaming.name.clone())
    }
}
