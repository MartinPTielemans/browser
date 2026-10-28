//! A host for the shell's tests, which records what the shell asks of the core and answers
//! nothing, so a test plays the core's reports itself.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use gpui::{App, Bounds, Pixels, Point, SharedString, Size, WeakEntity};

use super::{
    Answer, BarTab, Browser, DownloadAction, MenuBar, MenuEntry, PageReply, ShownPageDialog,
    browser::WindowButton,
};

/// What the core shares between its windows, as it keeps them: the Spaces deleted and the archive's
/// removals, 25 of each at most, the newest last. Clears are each window's own.
#[derive(Default)]
pub(super) struct Core {
    ticket: Cell<u64>,
    /// The Spaces deleted.
    pub(super) deleted: RefCell<Vec<u64>>,
    /// The archive removals, each by its id and the tabs it took.
    pub(super) removals: RefCell<Vec<(u64, usize)>>,
}

impl Core {
    /// As many as the core keeps.
    const KEPT: usize = 25;

    fn ticket(&self) -> u64 {
        self.ticket.set(self.ticket.get() + 1);
        9000 + self.ticket.get()
    }

    fn removed(&self, tabs: usize) -> u64 {
        let id = self.ticket();
        let mut removals = self.removals.borrow_mut();
        removals.push((id, tabs));
        if removals.len() > Self::KEPT {
            removals.remove(0);
        }
        id
    }
}

#[derive(Default)]
pub(super) struct Recorder {
    /// The core this window shares with the others a test makes with `Recorder::beside`.
    pub(super) core: Rc<Core>,
    /// This window's Clears, by id, the last ten.
    cleared: RefCell<Vec<u64>>,
    calls: RefCell<Vec<String>>,
    /// How many detents the trackpad was asked for.
    pub(super) detents: Cell<usize>,
    /// How many tabs and folders it was asked to move.
    pub(super) moves: Cell<usize>,
    /// The Spaces it was asked to show, in order.
    pub(super) shown: RefCell<Vec<u64>>,
    /// The menus it was asked to show, each item by its title and whether it's enabled.
    pub(super) menus: RefCell<Vec<Vec<(SharedString, bool)>>>,
    /// The last menus it was asked to show, item by item.
    pub(super) entries: RefCell<Vec<MenuEntry>>,
    /// The look of each of ilo's own menus it was asked to show, as `menus` has them.
    pub(super) menu_looks: RefCell<Vec<crate::menu::Look>>,
    /// Whether each of ilo's own menus was to show in the reveal's panel, as `menus` has them.
    pub(super) menus_in_reveal: RefCell<Vec<bool>>,
    /// What the menu bar was last asked to show, and the window's title.
    pub(super) menu_bar: RefCell<MenuBar>,
    pub(super) window_title: RefCell<String>,
    /// The share services it lists for any page.
    pub(super) share_services: RefCell<Vec<SharedString>>,
    /// What the last menu does with the item chosen, as `choose` picks it.
    answer: RefCell<Option<Box<dyn FnOnce(Option<usize>, &mut App)>>>,
    /// Each time the shell opened the Site Controls: the tab, and where they hang from.
    pub(super) site_controls: RefCell<Vec<(u64, crate::site_controls::Place)>>,
    /// The ids of the extensions the Site Controls were given to list, each time.
    pub(super) site_controls_extensions: RefCell<Vec<Vec<SharedString>>>,
    /// Each time the shell opened the command bar: the URL field's box it opened over, if any.
    pub(super) command_bars: RefCell<Vec<Option<Bounds<Pixels>>>>,
    /// The command bar shows, until a test plays the app's report that it was dismissed.
    pub(super) bar_open: Cell<bool>,
    /// What moved the pages, in order: "layout" for the page's frame, "shares" for a split's, and
    /// "moving" and "still" for what the shell told the core about the motion.
    pub(super) page_events: RefCell<Vec<&'static str>>,
    /// Where each tooltip it was asked for hangs from, in order.
    pub(super) anchors: RefCell<Vec<crate::tooltip::Anchor>>,
    /// Where the shell last put the link target's plate, in the window.
    pub(super) link_frame: Cell<Option<Bounds<Pixels>>>,
    /// What the shell did to tabs, folders and Spaces that ⌘Z puts back, in order, such as
    /// `close 5` and `reopen 5 as 1001`.
    edits: RefCell<Vec<String>>,
    /// The id the next tab or folder it makes gets.
    made: Cell<u64>,
    /// The URL each pinned tab or favorite goes back to, and the name each tab was given.
    pub(super) pinned_urls: RefCell<HashMap<u64, String>>,
    pub(super) names: RefCell<HashMap<u64, String>>,
    /// The words of the toasts with no Undo button it was asked to show.
    pub(super) toasts: RefCell<Vec<String>>,
    /// The words of the toasts with an Undo button it was asked to show, and what the last one's
    /// button does.
    pub(super) undo_toasts: RefCell<Vec<String>>,
    pub(super) undo_toast: RefCell<Option<Rc<dyn Fn(&mut App)>>>,
    /// The Edit menu's Undo and Redo titles it was last given.
    pub(super) edit_titles: RefCell<(Option<String>, Option<String>)>,
    /// The platform has Apple's engine for a tab's page.
    /// The platform has Safari's engine for a tab's page.
    pub(super) system_engine: Cell<bool>,
    /// The shell it answers each picture asked of it for with this picture, at once, inside the
    /// call that asked, as a core that answers inline would.
    pub(super) pictures: RefCell<Option<(WeakEntity<super::Shell>, super::PagePicture)>>,
    /// The answers to page dialogs it was given, with what was typed.
    pub(super) replies: RefCell<Vec<PageReply>>,
}

impl Recorder {
    /// A host for another window of the same core.
    pub(super) fn beside(other: &Recorder) -> Self {
        Recorder {
            core: other.core.clone(),
            ..Default::default()
        }
    }

    fn record(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }

    /// Records the menu it was asked to show, for `choose` to answer.
    fn menu(
        &self,
        entries: Vec<MenuEntry>,
        _: Point<Pixels>,
        _: Option<usize>,
        _: bool,
        answer: Box<dyn FnOnce(Option<usize>, &mut App)>,
        _: &mut App,
    ) {
        let items = entries.iter().map(|entry| (entry.title.clone(), entry.enabled)).collect();
        self.menus.borrow_mut().push(items);
        *self.entries.borrow_mut() = entries;
        *self.answer.borrow_mut() = Some(answer);
    }

    /// Chooses the item titled `title` in the last menu, as the user's click does.
    pub(super) fn choose(&self, title: &str, cx: &mut App) {
        let at = self.menus.borrow().last().and_then(|items| {
            items.iter().position(|(each, enabled)| each.as_ref() == title && *enabled)
        });
        let answer = self.answer.borrow_mut().take().expect("a menu to choose from");
        answer(Some(at.unwrap_or_else(|| panic!("no {title} in the menu"))), cx);
    }

    /// Closes the last menu without a choice, as Escape or a click outside it does.
    pub(super) fn dismiss(&self, cx: &mut App) {
        let answer = self.answer.borrow_mut().take().expect("a menu to close");
        answer(None, cx);
    }

    /// What the shell asked for since the last time, in order.
    pub(super) fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.calls.borrow_mut())
    }

    fn edit(&self, edit: String) {
        self.edits.borrow_mut().push(edit);
    }

    /// What the shell did that ⌘Z puts back since the last time, in order.
    pub(super) fn take_edits(&self) -> Vec<String> {
        std::mem::take(&mut self.edits.borrow_mut())
    }

    fn make(&self) -> u64 {
        self.made.set(self.made.get() + 1);
        1000 + self.made.get()
    }
}

impl Browser for Recorder {
    fn select(&self, tab: u64, _: &mut App) {
        self.record(format!("select {tab}"))
    }

    fn close(&self, tab: u64, _: bool, _: &mut App) {
        self.edit(format!("close {tab}"));
        self.record(format!("close {tab}"))
    }

    fn close_below(&self, tab: u64, _: &mut App) {
        self.edit(format!("close below {tab}"));
    }

    fn duplicate(&self, _: u64, _: &mut App) {}

    fn move_to_space(&self, tab: u64, space: u64, _: &mut App) {
        self.edit(format!("space {tab} {space}"));
    }

    fn warn_moving_profiles(&self, _: &mut App) -> bool {
        Default::default()
    }

    fn set_warn_moving_profiles(&self, _: bool, _: &mut App) {}

    fn set_pinned(&self, tab: u64, pinned: bool, _: &mut App) {
        self.edit(format!("pinned {tab} {pinned}"));
    }

    fn move_to_favorites(&self, tab: u64, index: usize, _: &mut App) {
        self.edit(format!("favorite {tab} {index}"));
        self.moves.set(self.moves.get() + 1);
    }

    fn move_tab(&self, tab: u64, pinned: bool, folder: u64, index: usize, _: &mut App) {
        self.edit(format!("move {tab} {pinned} {folder} {index}"));
        self.moves.set(self.moves.get() + 1);
    }

    fn open_at(&self, _: &[String], _: bool, _: u64, _: usize, _: &mut App) -> u64 {
        Default::default()
    }

    fn create_folder(&self, space: u64, name: &str, folder: u64, index: usize, _: &mut App) -> u64 {
        let made = self.make();
        self.edit(format!("folder {made} {space} {name} {folder} {index}"));
        made
    }

    fn rename_folder(&self, folder: u64, name: &str, _: &mut App) {
        self.edit(format!("rename folder {folder} {name}"));
    }

    fn set_folder_open(&self, _: u64, _: bool, _: &mut App) {}

    fn move_folder(&self, folder: u64, to: u64, index: usize, _: &mut App) {
        self.edit(format!("move folder {folder} {to} {index}"));
        self.moves.set(self.moves.get() + 1);
    }

    fn delete_folder(&self, folder: u64, _: &mut App) {
        self.edit(format!("delete folder {folder}"));
    }

    fn separate(&self, tab: u64, _: &mut App) {
        self.edit(format!("separate {tab}"));
    }

    fn move_in_split(&self, _: u64, _: usize, _: &mut App) {}

    fn set_split_stacked(&self, _: u64, _: bool, _: &mut App) {}

    fn set_split_shares(&self, tab: u64, shares: Option<&[f64]>, _: &mut App) {
        self.page_events.borrow_mut().push("shares");
        let shares = shares.map_or_else(
            || "even".to_string(),
            |shares| shares.iter().map(f64::to_string).collect::<Vec<_>>().join(" "),
        );
        self.record(format!("split shares {tab} {shares}"))
    }

    fn set_split_expanded(&self, tab: u64, expanded: bool, _: &mut App) {
        self.record(format!("split expanded {tab} {expanded}"))
    }

    fn split_with(&self, tab: u64, with: u64, left: bool, _: &mut App) {
        self.edit(format!("split {tab} {with} {left}"));
    }

    fn show_drop_targets(&self, _: bool, _: &mut App) {}

    fn back_to_pinned_url(&self, tab: u64, _: &mut App) {
        self.record(format!("back to pinned url {tab}"))
    }

    fn set_muted(&self, _: u64, _: bool, _: &mut App) {}

    fn clear(&self, _: &mut App) -> u64 {
        let id = self.core.ticket();
        self.edit(format!("clear as {id}"));
        self.record("clear".into());
        let mut cleared = self.cleared.borrow_mut();
        cleared.push(id);
        if cleared.len() > 10 {
            cleared.remove(0);
        }
        id
    }

    fn undo_clear(&self, clear: u64, _: &mut App) -> usize {
        let mut cleared = self.cleared.borrow_mut();
        let Some(at) = cleared.iter().position(|&each| each == clear) else {
            return 0;
        };
        cleared.remove(at);
        self.edit(format!("undo clear {clear}"));
        self.record("undo clear".into());
        1
    }

    fn undo_delete_space(&self, space: u64, _: &mut App) -> bool {
        let mut deleted = self.core.deleted.borrow_mut();
        let Some(at) = deleted.iter().position(|&each| each == space) else {
            return false;
        };
        deleted.remove(at);
        self.edit(format!("undo delete space {space}"));
        true
    }

    fn reopen_closed_tab(&self, _: &mut App) {}

    fn reopen_tab(&self, closed: u64, _: &mut App) -> u64 {
        let made = self.make();
        self.edit(format!("reopen {closed} as {made}"));
        made
    }

    fn pinned_url(&self, tab: u64) -> String {
        self.pinned_urls.borrow().get(&tab).cloned().unwrap_or_default()
    }

    fn set_pinned_url(&self, tab: u64, url: &str, _: &mut App) {
        self.edit(format!("pinned url {tab} {url}"));
    }

    fn tab_name(&self, tab: u64) -> String {
        self.names.borrow().get(&tab).cloned().unwrap_or_default()
    }

    fn undo_archive_remove(&self, removal: u64, _: &mut App) -> usize {
        let mut removals = self.core.removals.borrow_mut();
        let Some(at) = removals.iter().position(|&(id, _)| id == removal) else {
            return 0;
        };
        self.edit(format!("undo archive remove {removal}"));
        removals.remove(at).1
    }

    fn restore_archived(&self, _: u64, _: &mut App) {}

    fn remove_archived(&self, archived: u64, _: &mut App) -> u64 {
        let id = self.core.removed(1);
        self.edit(format!("archive remove {archived} as {id}"));
        id
    }

    fn clear_archive(&self, _: &mut App) -> u64 {
        let id = self.core.removed(2);
        self.edit(format!("archive clear as {id}"));
        id
    }

    fn act_on_download(&self, _: u64, _: DownloadAction, _: &mut App) {}

    fn clear_downloads(&self, _: &mut App) {}

    fn download_icon(&self, _: u64, _: &mut App) {}

    fn file_preview(&self, download: u64, _: &str, _: &mut App) {
        self.record(format!("file preview {download}"))
    }

    fn trash(&self, _: &str, _: &mut App) -> bool {
        Default::default()
    }

    fn drag_file(&self, _: &str, _: &mut App) -> bool {
        Default::default()
    }

    fn page_favicon(&self, _: &str, _: &mut App) {}

    fn day_starts(&self) -> [f64; 4] {
        Default::default()
    }

    fn go_back(&self, _: u64, _: &mut App) {}

    fn go_forward(&self, _: u64, _: &mut App) {}

    fn reload(&self, tab: u64, bypass_cache: bool, _: &mut App) {
        self.record(format!("reload {tab}{}", if bypass_cache { " bypassing cache" } else { "" }))
    }

    fn clear_site(&self, _: u64, _: bool, _: &mut App) {}

    fn answer_permission(&self, tab: u64, answer: Answer, _: &mut App) {
        self.record(format!("answer {tab} {answer:?}"))
    }

    fn can_open_in_safari(&self) -> bool {
        true
    }

    fn open_in_safari(&self, url: &str, _: &mut App) {
        self.record(format!("open in safari {url}"))
    }

    fn can_use_system_engine(&self) -> bool {
        self.system_engine.get()
    }

    fn close_media_note(&self, tab: u64, _: &mut App) {
        self.record(format!("close media note {tab}"));
    }

    fn use_system_engine(&self, tab: u64, _: &mut App) {
        self.record(format!("system engine {tab}"))
    }

    fn show_page_dialog(&self, shown: Option<ShownPageDialog>, _: &mut App) {
        match shown {
            Some((tab, dialog, _)) => self.record(format!("page dialog {tab} {}", dialog.id)),
            None => self.record("page dialog none".into()),
        }
    }

    fn answer_page_dialog(&self, tab: u64, id: u64, reply: PageReply, _: &mut App) {
        self.record(format!("answer dialog {tab} {id} {reply:?}"));
        self.replies.borrow_mut().push(reply);
    }

    fn stop(&self, _: u64, _: &mut App) {}

    fn zoom(&self, _: u64, _: i32, _: &mut App) {}

    fn view_source(&self, _: u64, _: &mut App) {}

    fn save_page(&self, _: u64, _: &mut App) {}

    fn close_page_menu(&self, tab: u64, command: Option<i32>, _: &mut App) {
        match command {
            Some(command) => self.record(format!("page menu {tab} runs {command}")),
            None => self.record(format!("page menu {tab} closes")),
        }
    }

    fn print(&self, _: u64, _: &mut App) {}

    fn devtools(&self, _: u64, _: i32, _: &mut App) {}

    fn task_manager(&self, _: &mut App) {}

    fn set_menus(&self, bar: &MenuBar, _: &mut App) {
        *self.menu_bar.borrow_mut() = bar.clone();
    }

    fn set_window_title(&self, title: &str, _: &mut App) {
        *self.window_title.borrow_mut() = title.to_string();
    }

    fn set_edit_titles(&self, undo: Option<&str>, redo: Option<&str>, _: &mut App) {
        *self.edit_titles.borrow_mut() = (undo.map(Into::into), redo.map(Into::into));
    }

    fn run_extension(&self, _: &str, _: Bounds<Pixels>, _: &mut App) -> bool {
        Default::default()
    }

    fn run_extension_command(&self, _: &str, _: &str, _: &mut App) -> bool {
        Default::default()
    }

    fn set_extension_pinned(&self, _: u64, _: &str, _: bool, _: &mut App) {}

    fn remove_extension(&self, _: u64, _: &str, _: &mut App) -> bool {
        Default::default()
    }

    fn open_extension_options(&self, _: &str, _: &mut App) {}

    fn manage_extensions(&self, _: Option<&str>, _: &mut App) {}

    fn add_extension(&self, _: &mut App) {}

    fn lay_out_side_panel(&self, _: Bounds<Pixels>, _: &mut App) {}

    fn close_side_panel(&self, _: &mut App) {}

    fn has_side_panel(&self, _: &str) -> bool {
        false
    }

    fn toggle_side_panel(&self, _: &str, _: &mut App) {}

    fn set_side_panel_width(&self, _: f32, _: &mut App) {}

    fn view_history(&self, _: &mut App) {}

    fn close_other_window(&self, _: &mut App) -> bool {
        Default::default()
    }

    fn show_space(&self, space: u64, _: &mut App) {
        self.shown.borrow_mut().push(space);
    }

    fn space_tab(&self, _: u64) -> u64 {
        Default::default()
    }

    fn create_space(&self, _: &str, _: u64, _: &mut App) -> u64 {
        Default::default()
    }

    fn rename_space(&self, space: u64, name: &str, _: &mut App) {
        self.edit(format!("rename space {space} {name}"));
    }

    fn rename_tab(&self, tab: u64, name: &str, _: &mut App) {
        self.edit(format!("rename tab {tab} {name}"));
        self.names.borrow_mut().insert(tab, name.into());
    }

    fn set_icon(&self, tab: u64, icon: &str, _: &mut App) {
        self.edit(format!("icon {tab} {icon}"));
    }

    fn delete_space(&self, space: u64, _: &mut App) -> bool {
        self.edit(format!("delete space {space}"));
        let mut deleted = self.core.deleted.borrow_mut();
        deleted.push(space);
        if deleted.len() > Core::KEPT {
            deleted.remove(0);
        }
        true
    }

    fn set_space_color(&self, space: u64, color: Option<u32>, _: &mut App) {
        self.edit(format!("color {space} {color:?}"));
        let color = color.map_or("none".to_string(), |color| format!("{color:06x}"));
        self.record(format!("space color {space} {color}"))
    }

    fn set_space_icon(&self, space: u64, icon: &str, _: &mut App) {
        self.edit(format!("space icon {space} {icon}"));
    }

    fn set_space_theme(&self, space: u64, theme: &str, _: &mut App) {
        let theme = if theme.is_empty() { "none" } else { theme };
        self.edit(format!("theme {space} {theme}"));
        self.record(format!("space theme {space} {theme}"))
    }

    fn set_space_profile(&self, _: u64, _: u64, _: &mut App) {}

    fn create_profile(&self, _: &str, _: &mut App) -> u64 {
        Default::default()
    }

    fn delete_profile(&self, _: u64, _: &mut App) {}

    fn context_menu(
        &self,
        entries: Vec<MenuEntry>,
        position: Point<Pixels>,
        in_reveal: bool,
        look: crate::menu::Look,
        answer: Box<dyn FnOnce(Option<usize>, &mut App)>,
        cx: &mut App,
    ) {
        self.menu_looks.borrow_mut().push(look);
        self.menus_in_reveal.borrow_mut().push(in_reveal);
        self.menu(entries, position, None, in_reveal, answer, cx);
    }

    fn confirm(
        &self,
        _: String,
        _: String,
        _: &'static str,
        _: Box<dyn FnOnce(&mut App)>,
        _: &mut App,
    ) {
    }

    fn focus_ui(&self, _: &mut App) {}

    fn focus_page(&self, _: &mut App) {
        self.record("focus page".into())
    }

    fn lay_out_page(&self, _: Bounds<Pixels>, _: f32, _: &mut App) {
        self.page_events.borrow_mut().push("layout");
    }

    fn set_focus_color(&self, _: u32, _: &mut App) {}

    fn set_page_color(&self, _: u32, _: &mut App) {}

    fn page_moving(&self, moving: bool, _: &mut App) {
        self.page_events.borrow_mut().push(if moving { "moving" } else { "still" });
    }

    fn set_pane_corner(&self, _: f32, _: f32, _: &mut App) {}

    fn show_window_buttons(&self, _: bool, _: &mut App) {}

    fn press_window_button(&self, button: WindowButton, _: &mut App) {
        if let WindowButton::Zoom = button {
            self.record("window button zoom".into())
        }
    }

    fn show_reveal(&self, _: f32, _: &mut App) {}

    fn saved_sidebar_width(&self) -> Option<f32> {
        Default::default()
    }

    fn set_sidebar_width(&self, _: f32, _: &mut App) {}

    fn hide_reveal(&self, _: &mut App) {}

    fn detent(&self, _: &mut App) {
        self.detents.set(self.detents.get() + 1);
    }

    fn watch_control(&self, _: &mut App) {}

    fn copy(&self, _: &str, _: &mut App) {}

    fn share_services(&self, _: &str) -> Vec<SharedString> {
        self.share_services.borrow().clone()
    }

    fn share(&self, _: &str, _: &str, _: usize, _: &mut App) {}

    fn toast(&self, text: &str, _: Option<(&str, &str)>, _: Duration, _: &mut App) {
        self.toasts.borrow_mut().push(text.into());
    }

    fn toast_undo(&self, text: &str, undo: Rc<dyn Fn(&mut App)>, _: Duration, _: &mut App) {
        self.undo_toasts.borrow_mut().push(text.into());
        *self.undo_toast.borrow_mut() = Some(undo);
    }

    fn hide_undo_toast(&self, _: &mut App) {
        self.undo_toast.borrow_mut().take();
    }

    fn full_screen_note(&self, shown: bool, _: &mut App) {
        self.record(format!("full screen note {}", if shown { "shown" } else { "hidden" }))
    }

    fn hang_note(&self, tab: u64, shown: bool, _: &mut App) {
        self.record(format!("hang note {tab} {}", if shown { "shown" } else { "hidden" }))
    }

    fn tooltip(&self, tip: Option<(crate::tooltip::Tip, crate::tooltip::Anchor)>, _: &mut App) {
        match tip {
            Some((tip, anchor)) => {
                self.anchors.borrow_mut().push(anchor);
                self.record(format!("tooltip {} {}", tip.text, tip.keys.concat()))
            }
            None => self.record("tooltip none".into()),
        }
    }

    fn link_target(
        &self,
        plate: Option<(crate::link_target::Plate, Bounds<Pixels>, crate::link_target::Side)>,
        _: &mut App,
    ) {
        match plate {
            Some((plate, frame, side)) => {
                self.link_frame.set(Some(frame));
                let side = match side {
                    crate::link_target::Side::Left => "left",
                    crate::link_target::Side::Right => "right",
                };
                self.record(format!("link target {} {side}", plate.text()))
            }
            None => self.record("link target none".into()),
        }
    }

    fn command_bar(
        &self,
        _: Option<(u64, SharedString)>,
        _: Vec<BarTab>,
        field: Option<Bounds<Pixels>>,
        _: &mut App,
    ) {
        self.command_bars.borrow_mut().push(field);
        self.bar_open.set(true);
    }

    fn command_bar_open(&self, _: &App) -> bool {
        self.bar_open.get()
    }

    fn split_bar(&self, _: u64, _: Bounds<Pixels>, _: Bounds<Pixels>, _: &mut App) {
        Default::default()
    }

    fn find(&self, _: u64, _: &mut App) {}

    fn site_controls(
        &self,
        tab: u64,
        place: crate::site_controls::Place,
        _: Size<Pixels>,
        extensions: Vec<crate::site_controls::Extension>,
        cx: &mut App,
    ) {
        self.site_controls.borrow_mut().push((tab, place));
        self.site_controls_extensions(tab, extensions, cx);
    }

    fn site_controls_extensions(
        &self,
        _: u64,
        extensions: Vec<crate::site_controls::Extension>,
        _: &mut App,
    ) {
        let ids = extensions.into_iter().map(|extension| extension.id).collect();
        self.site_controls_extensions.borrow_mut().push(ids);
    }

    fn capture(&self, tab: u64, frame: Bounds<Pixels>, _: f32, full_page: bool, _: &mut App) {
        let (x, y) = (f32::from(frame.origin.x), f32::from(frame.origin.y));
        let (width, height) = (f32::from(frame.size.width), f32::from(frame.size.height));
        let what = if full_page { "full page" } else { "portion" };
        self.record(format!("capture {what} {tab} at {x} {y} {width} {height}"));
    }

    fn page_picture(&self, tab: u64, request: u64, cx: &mut App) {
        self.record(format!("picture {tab}"));
        if let Some((shell, picture)) = self.pictures.borrow().clone() {
            super::picture_answered(vec![shell], request, Some(picture), &cx.to_async());
        }
    }

    fn settings(&self, _: &mut App) {}

    fn shortcuts(&self, _: &mut App) {}

    fn find_next(&self, _: u64, _: bool, _: &mut App) {}

    fn jump_to_selection(&self, _: u64, _: &mut App) {}

    fn quit(&self, _: &mut App) {}

    fn new_window(&self, _: &mut App) {
        self.record("new window".into())
    }

    fn close_window(&self, _: &mut App) {
        self.record("close window".into())
    }

    fn new_private_window(&self, _: &mut App) {}
}
