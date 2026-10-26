//! A host for the shell's tests, which records what the shell asks of the core and answers
//! nothing, so a test plays the core's reports itself.

use std::{cell::RefCell, time::Duration};

use gpui::{App, Bounds, Pixels, Point, SharedString};

use super::{
    Answer, BarTab, Browser, DownloadAction, Extension, MenuEntry, browser::WindowButton,
};

#[derive(Default)]
pub(super) struct Recorder(RefCell<Vec<String>>);

impl Recorder {
    fn record(&self, call: String) {
        self.0.borrow_mut().push(call);
    }

    /// What the shell asked for since the last time, in order.
    pub(super) fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.0.borrow_mut())
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

    fn move_to_favorites(&self, _: u64, _: usize, _: &mut App) {}

    fn move_tab(&self, _: u64, _: bool, _: u64, _: usize, _: &mut App) {}

    fn open_at(&self, _: &[String], _: bool, _: u64, _: usize, _: &mut App) -> u64 {
        Default::default()
    }

    fn create_folder(&self, _: u64, _: &str, _: u64, _: usize, _: &mut App) -> u64 {
        Default::default()
    }

    fn rename_folder(&self, _: u64, _: &str, _: &mut App) {}

    fn set_folder_open(&self, _: u64, _: bool, _: &mut App) {}

    fn move_folder(&self, _: u64, _: u64, _: usize, _: &mut App) {}

    fn delete_folder(&self, _: u64, _: &mut App) {}

    fn separate(&self, _: u64, _: &mut App) {}

    fn move_in_split(&self, _: u64, _: usize, _: &mut App) {}

    fn set_split_stacked(&self, _: u64, _: bool, _: &mut App) {}

    fn set_split_shares(&self, _: u64, _: Option<&[f64]>, _: &mut App) {}

    fn split_with(&self, _: u64, _: u64, _: bool, _: &mut App) {}

    fn show_drop_targets(&self, _: bool, _: &mut App) {}

    fn back_to_pinned_url(&self, _: u64, _: &mut App) {}

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

    fn set_menus(&self, _: &[SharedString], _: &[Extension], _: &mut App) {}

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

    fn show_space(&self, _: u64, _: &mut App) {}

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

    fn set_space_color(&self, _: u64, _: Option<u32>, _: &mut App) {}

    fn set_space_icon(&self, _: u64, _: &str, _: &mut App) {}

    fn set_space_profile(&self, _: u64, _: u64, _: &mut App) {}

    fn create_profile(&self, _: &str, _: &mut App) -> u64 {
        Default::default()
    }

    fn delete_profile(&self, _: u64, _: &mut App) {}

    fn menu(
        &self,
        _: Vec<MenuEntry>,
        _: Point<Pixels>,
        _: Option<usize>,
        _: bool,
        _: Box<dyn FnOnce(Option<usize>, &mut App)>,
        _: &mut App,
    ) {
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

    fn press_window_button(&self, _: WindowButton, _: &mut App) {}

    fn show_reveal(&self, _: f32, _: &mut App) {}

    fn saved_sidebar_width(&self) -> Option<f32> {
        Default::default()
    }

    fn set_sidebar_width(&self, _: f32, _: &mut App) {}

    fn hide_reveal(&self, _: &mut App) {}

    fn detent(&self, _: &mut App) {}

    fn watch_control(&self, _: &mut App) {}

    fn copy(&self, _: &str, _: &mut App) {}

    fn toast(&self, _: &str, _: Option<(&str, &str)>, _: Duration, _: &mut App) {
        Default::default()
    }

    fn tooltip(&self, _: Option<(SharedString, crate::tooltip::Anchor)>, _: &mut App) {}

    fn accent_color(&self) -> u32 {
        Default::default()
    }

    fn command_bar(&self, _: Option<(u64, SharedString)>, _: Vec<BarTab>, _: &mut App) {
        Default::default()
    }

    fn split_bar(&self, _: u64, _: Bounds<Pixels>, _: Bounds<Pixels>, _: &mut App) {
        Default::default()
    }

    fn find(&self, _: u64, _: &mut App) {}

    fn settings(&self, _: &mut App) {}

    fn shortcuts(&self, _: &mut App) {}

    fn find_next(&self, _: u64, _: bool, _: &mut App) {}

    fn jump_to_selection(&self, _: u64, _: &mut App) {}

    fn quit(&self, _: &mut App) {}

    fn new_window(&self, _: &mut App) {}

    fn close_window(&self, _: &mut App) {}
}
