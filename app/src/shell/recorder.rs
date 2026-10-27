//! A host for the shell's tests, which records what the shell asks of the core and answers
//! nothing, so a test plays the core's reports itself.

use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

use gpui::{App, Bounds, Pixels, Point, SharedString, Size};

use super::{
    Answer, BarTab, Browser, DownloadAction, Extension, MenuEntry, browser::WindowButton,
};

#[derive(Default)]
pub(super) struct Recorder {
    calls: RefCell<Vec<String>>,
    /// How many detents the trackpad was asked for.
    pub(super) detents: Cell<usize>,
    /// How many tabs and folders it was asked to move.
    pub(super) moves: Cell<usize>,
    /// The Spaces it was asked to show, in order.
    pub(super) shown: RefCell<Vec<u64>>,
    /// The menus it was asked to show, each item by its title and whether it's enabled.
    pub(super) menus: RefCell<Vec<Vec<(SharedString, bool)>>>,
    /// What the last menu does with the item chosen, as `choose` picks it.
    answer: RefCell<Option<Box<dyn FnOnce(Option<usize>, &mut App)>>>,
    /// The Spaces the menu bar's Spaces menu lists.
    pub(super) menu_spaces: RefCell<Vec<SharedString>>,
    /// Each time the shell opened the Site Controls: the tab, and where they hang from.
    pub(super) site_controls: RefCell<Vec<(u64, crate::site_controls::Place)>>,
    /// Each time the shell opened the command bar: the URL field's box it opened over, if any.
    pub(super) command_bars: RefCell<Vec<Option<Bounds<Pixels>>>>,
}

impl Recorder {
    fn record(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }

    /// Chooses the item titled `title` in the last menu, as the user's click does.
    pub(super) fn choose(&self, title: &str, cx: &mut App) {
        let at = self.menus.borrow().last().and_then(|items| {
            items.iter().position(|(each, enabled)| each.as_ref() == title && *enabled)
        });
        let answer = self.answer.borrow_mut().take().expect("a menu to choose from");
        answer(Some(at.unwrap_or_else(|| panic!("no {title} in the menu"))), cx);
    }

    /// What the shell asked for since the last time, in order.
    pub(super) fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.calls.borrow_mut())
    }
}

impl Browser for Recorder {
    fn select(&self, tab: u64, _: &mut App) {
        self.record(format!("select {tab}"))
    }

    fn close(&self, tab: u64, _: bool, _: &mut App) {
        self.record(format!("close {tab}"))
    }

    fn close_below(&self, _: u64, _: &mut App) {}

    fn duplicate(&self, _: u64, _: &mut App) {}

    fn move_to_space(&self, _: u64, _: u64, _: &mut App) {}

    fn warn_moving_profiles(&self, _: &mut App) -> bool {
        Default::default()
    }

    fn set_warn_moving_profiles(&self, _: bool, _: &mut App) {}

    fn set_pinned(&self, _: u64, _: bool, _: &mut App) {}

    fn move_to_favorites(&self, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }

    fn move_tab(&self, _: u64, _: bool, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }

    fn open_at(&self, _: &[String], _: bool, _: u64, _: usize, _: &mut App) -> u64 {
        Default::default()
    }

    fn create_folder(&self, _: u64, _: &str, _: u64, _: usize, _: &mut App) -> u64 {
        Default::default()
    }

    fn rename_folder(&self, _: u64, _: &str, _: &mut App) {}

    fn set_folder_open(&self, _: u64, _: bool, _: &mut App) {}

    fn move_folder(&self, _: u64, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }

    fn delete_folder(&self, _: u64, _: &mut App) {}

    fn separate(&self, _: u64, _: &mut App) {}

    fn move_in_split(&self, _: u64, _: usize, _: &mut App) {}

    fn set_split_stacked(&self, _: u64, _: bool, _: &mut App) {}

    fn set_split_shares(&self, _: u64, _: Option<&[f64]>, _: &mut App) {}

    fn split_with(&self, _: u64, _: u64, _: bool, _: &mut App) {}

    fn show_drop_targets(&self, _: bool, _: &mut App) {}

    fn back_to_pinned_url(&self, tab: u64, _: &mut App) {
        self.record(format!("back to pinned url {tab}"))
    }

    fn set_muted(&self, _: u64, _: bool, _: &mut App) {}

    fn clear(&self, _: &mut App) {
        self.record("clear".into())
    }

    fn undo_clear(&self, _: &mut App) -> usize {
        self.record("undo clear".into());
        0
    }

    fn undo_delete_space(&self, _: &mut App) -> bool {
        Default::default()
    }

    fn reopen_closed_tab(&self, _: &mut App) {}

    fn restore_archived(&self, _: u64, _: &mut App) {}

    fn remove_archived(&self, _: u64, _: &mut App) {}

    fn clear_archive(&self, _: &mut App) {}

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

    fn reload(&self, _: u64, _: bool, _: &mut App) {}

    fn clear_site(&self, _: u64, _: bool, _: &mut App) {}

    fn answer_permission(&self, _: u64, _: Answer, _: &mut App) {}

    fn stop(&self, _: u64, _: &mut App) {}

    fn zoom(&self, _: u64, _: i32, _: &mut App) {}

    fn view_source(&self, _: u64, _: &mut App) {}

    fn save_page(&self, _: u64, _: &mut App) {}

    fn print(&self, _: u64, _: &mut App) {}

    fn devtools(&self, _: u64, _: i32, _: &mut App) {}

    fn task_manager(&self, _: &mut App) {}

    fn set_menus(&self, spaces: &[SharedString], _: &[Extension], _: &mut App) {
        *self.menu_spaces.borrow_mut() = spaces.to_vec();
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

    fn rename_space(&self, _: u64, _: &str, _: &mut App) {}

    fn rename_tab(&self, _: u64, _: &str, _: &mut App) {}

    fn set_icon(&self, _: u64, _: &str, _: &mut App) {}

    fn delete_space(&self, _: u64, _: &mut App) -> bool {
        Default::default()
    }

    fn set_space_color(&self, space: u64, color: Option<u32>, _: &mut App) {
        let color = color.map_or("none".to_string(), |color| format!("{color:06x}"));
        self.record(format!("space color {space} {color}"))
    }

    fn set_space_icon(&self, _: u64, _: &str, _: &mut App) {}

    fn set_space_theme(&self, space: u64, theme: &str, _: &mut App) {
        let theme = if theme.is_empty() { "none" } else { theme };
        self.record(format!("space theme {space} {theme}"))
    }

    fn set_space_profile(&self, _: u64, _: u64, _: &mut App) {}

    fn create_profile(&self, _: &str, _: &mut App) -> u64 {
        Default::default()
    }

    fn delete_profile(&self, _: u64, _: &mut App) {}

    fn menu(
        &self,
        entries: Vec<MenuEntry>,
        _: Point<Pixels>,
        _: Option<usize>,
        _: bool,
        answer: Box<dyn FnOnce(Option<usize>, &mut App)>,
        _: &mut App,
    ) {
        let items = entries.into_iter().map(|entry| (entry.title, entry.enabled)).collect();
        self.menus.borrow_mut().push(items);
        *self.answer.borrow_mut() = Some(answer);
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

    fn lay_out_page(&self, _: Bounds<Pixels>, _: f32, _: &mut App) {}

    fn set_focus_color(&self, _: u32, _: &mut App) {}

    fn set_page_color(&self, _: u32, _: &mut App) {}

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

    fn toast(&self, _: &str, _: Option<(&str, &str)>, _: Duration, _: &mut App) {
        Default::default()
    }

    fn full_screen_note(&self, shown: bool, _: &mut App) {
        self.record(format!("full screen note {}", if shown { "shown" } else { "hidden" }))
    }

    fn tooltip(&self, tip: Option<(crate::tooltip::Tip, crate::tooltip::Anchor)>, _: &mut App) {
        match tip {
            Some((tip, _)) => self.record(format!("tooltip {} {}", tip.text, tip.keys.concat())),
            None => self.record("tooltip none".into()),
        }
    }

    fn accent_color(&self) -> u32 {
        Default::default()
    }

    fn command_bar(
        &self,
        _: Option<(u64, SharedString)>,
        _: Vec<BarTab>,
        field: Option<Bounds<Pixels>>,
        _: &mut App,
    ) {
        self.command_bars.borrow_mut().push(field);
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
        _: Vec<crate::site_controls::Extension>,
        _: &mut App,
    ) {
        self.site_controls.borrow_mut().push((tab, place));
    }

    fn capture(&self, tab: u64, frame: Bounds<Pixels>, _: f32, full_page: bool, _: &mut App) {
        let (x, y) = (f32::from(frame.origin.x), f32::from(frame.origin.y));
        let (width, height) = (f32::from(frame.size.width), f32::from(frame.size.height));
        let what = if full_page { "full page" } else { "portion" };
        self.record(format!("capture {what} {tab} at {x} {y} {width} {height}"));
    }

    fn settings(&self, _: &mut App) {}

    fn shortcuts(&self, _: &mut App) {}

    fn find_next(&self, _: u64, _: bool, _: &mut App) {}

    fn jump_to_selection(&self, _: u64, _: &mut App) {}

    fn quit(&self, _: &mut App) {}

    fn new_window(&self, _: &mut App) {}

    fn close_window(&self, _: &mut App) {}

    fn new_private_window(&self, _: &mut App) {}
}
