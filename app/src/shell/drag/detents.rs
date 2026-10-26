//! The trackpad's clicks as a row is dragged, on a host that counts what it's asked.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui::{App, AppContext, Bounds, Pixels, Point, SharedString, TestAppContext, point, px};

use crate::shell::{
    Answer, BarTab, Browser, DownloadAction, Extension, Folders, MenuEntry, ROW_HEIGHT, ROW_PITCH,
    Shell, Slot, TILE_HEIGHT, WindowButton,
};

#[derive(Default)]
struct Host {
    detents: Cell<usize>,
    moves: Cell<usize>,
}

impl Browser for Host {
    fn detent(&self, _: &mut App) {
        self.detents.set(self.detents.get() + 1);
    }
    fn move_tab(&self, _: u64, _: bool, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }
    fn move_folder(&self, _: u64, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }
    fn move_to_favorites(&self, _: u64, _: usize, _: &mut App) {
        self.moves.set(self.moves.get() + 1);
    }

    fn select(&self, _: u64, _: &mut App) {}
    fn close(&self, _: u64, _: bool, _: &mut App) {}
    fn close_below(&self, _: u64, _: &mut App) {}
    fn duplicate(&self, _: u64, _: &mut App) {}
    fn move_to_space(&self, _: u64, _: u64, _: &mut App) {}
    fn warn_moving_profiles(&self, _: &mut App) -> bool {
        false
    }
    fn set_warn_moving_profiles(&self, _: bool, _: &mut App) {}
    fn set_pinned(&self, _: u64, _: bool, _: &mut App) {}
    fn open_at(&self, _: &[String], _: bool, _: u64, _: usize, _: &mut App) -> u64 {
        0
    }
    fn create_folder(&self, _: u64, _: &str, _: u64, _: usize, _: &mut App) -> u64 {
        0
    }
    fn rename_folder(&self, _: u64, _: &str, _: &mut App) {}
    fn set_folder_open(&self, _: u64, _: bool, _: &mut App) {}
    fn delete_folder(&self, _: u64, _: &mut App) {}
    fn separate(&self, _: u64, _: &mut App) {}
    fn move_in_split(&self, _: u64, _: usize, _: &mut App) {}
    fn set_split_stacked(&self, _: u64, _: bool, _: &mut App) {}
    fn set_split_shares(&self, _: u64, _: Option<&[f64]>, _: &mut App) {}
    fn split_with(&self, _: u64, _: u64, _: bool, _: &mut App) {}
    fn show_drop_targets(&self, _: bool, _: &mut App) {}
    fn back_to_pinned_url(&self, _: u64, _: &mut App) {}
    fn set_muted(&self, _: u64, _: bool, _: &mut App) {}
    fn clear(&self, _: &mut App) {}
    fn undo_clear(&self, _: &mut App) -> usize {
        0
    }
    fn undo_delete_space(&self, _: &mut App) -> bool {
        false
    }
    fn reopen_closed_tab(&self, _: &mut App) {}
    fn restore_archived(&self, _: u64, _: &mut App) {}
    fn remove_archived(&self, _: u64, _: &mut App) {}
    fn clear_archive(&self, _: &mut App) {}
    fn act_on_download(&self, _: u64, _: DownloadAction, _: &mut App) {}
    fn clear_downloads(&self, _: &mut App) {}
    fn download_icon(&self, _: u64, _: &mut App) {}
    fn trash(&self, _: &str, _: &mut App) -> bool {
        false
    }
    fn drag_file(&self, _: &str, _: &mut App) -> bool {
        false
    }
    fn page_favicon(&self, _: &str, _: &mut App) {}
    fn day_starts(&self) -> [f64; 4] {
        [0.; 4]
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
        false
    }
    fn run_extension_command(&self, _: &str, _: &str, _: &mut App) -> bool {
        false
    }
    fn set_extension_pinned(&self, _: u64, _: &str, _: bool, _: &mut App) {}
    fn remove_extension(&self, _: u64, _: &str, _: &mut App) -> bool {
        false
    }
    fn open_extension_options(&self, _: &str, _: &mut App) {}
    fn manage_extensions(&self, _: Option<&str>, _: &mut App) {}
    fn view_history(&self, _: &mut App) {}
    fn close_other_window(&self, _: &mut App) -> bool {
        false
    }
    fn show_space(&self, _: u64, _: &mut App) {}
    fn space_tab(&self, _: u64) -> u64 {
        0
    }
    fn create_space(&self, _: &str, _: u64, _: &mut App) -> u64 {
        0
    }
    fn rename_space(&self, _: u64, _: &str, _: &mut App) {}
    fn rename_tab(&self, _: u64, _: &str, _: &mut App) {}
    fn set_icon(&self, _: u64, _: &str, _: &mut App) {}
    fn delete_space(&self, _: u64, _: &mut App) -> bool {
        false
    }
    fn set_space_color(&self, _: u64, _: Option<u32>, _: &mut App) {}
    fn set_space_icon(&self, _: u64, _: &str, _: &mut App) {}
    fn set_space_profile(&self, _: u64, _: u64, _: &mut App) {}
    fn create_profile(&self, _: &str, _: &mut App) -> u64 {
        0
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
        None
    }
    fn set_sidebar_width(&self, _: f32, _: &mut App) {}
    fn hide_reveal(&self, _: &mut App) {}
    fn watch_control(&self, _: &mut App) {}
    fn copy(&self, _: &str, _: &mut App) {}
    fn toast(&self, _: &str, _: Option<(&str, &str)>, _: Duration, _: &mut App) {}
    fn tooltip(&self, _: Option<(SharedString, crate::tooltip::Anchor)>, _: &mut App) {}
    fn accent_color(&self) -> u32 {
        0
    }
    fn command_bar(&self, _: Option<(u64, SharedString)>, _: Vec<BarTab>, _: &mut App) {}
    fn split_bar(&self, _: u64, _: Bounds<Pixels>, _: Bounds<Pixels>, _: &mut App) {}
    fn find(&self, _: u64, _: &mut App) {}
    fn settings(&self, _: &mut App) {}
    fn shortcuts(&self, _: &mut App) {}
    fn find_next(&self, _: u64, _: bool, _: &mut App) {}
    fn jump_to_selection(&self, _: u64, _: &mut App) {}
    fn quit(&self, _: &mut App) {}
    fn new_window(&self, _: &mut App) {}
    fn close_window(&self, _: &mut App) {}
}

/// A shell showing one Space with `pinned` and `today` tabs and `folders`.
fn shell_with(
    cx: &mut TestAppContext,
    pinned: &[u64],
    today: &[u64],
    folders: Folders,
) -> (gpui::Entity<Shell>, Rc<Host>) {
    let host = Rc::new(Host::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Work".into(), None, 0, "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, pinned.to_vec(), today.to_vec(), Vec::new(), folders, cx);
    });
    (shell, host)
}

/// Where in the window `tab`'s row is, down its middle.
fn row_y(shell: &Shell, tab: u64) -> f32 {
    shell.drawn_list_top() + shell.layout()[&Slot::Tab(tab)] + ROW_HEIGHT / 2.
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

const X: f32 = 80.;

/// Presses `tab`'s row, in the middle.
fn press(shell: &gpui::Entity<Shell>, cx: &mut TestAppContext, tab: u64) -> f32 {
    shell.update(cx, |shell, _| {
        let y = row_y(shell, tab);
        shell.press_row(tab, at(X, y));
        y
    })
}

fn drag_to(shell: &gpui::Entity<Shell>, cx: &mut TestAppContext, x: f32, y: f32) {
    shell.update(cx, |shell, cx| shell.drag_to(at(x, y), cx));
}

/// One click for each row a dragged one passes, and none while it stays at a place, on the way
/// back up as on the way down, nor as it's let go.
#[gpui::test]
fn each_row_passed_gives_a_click(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1, 2, 3, 4, 5], Folders::default());
    let y = press(&shell, cx, 1);
    // Coming away from the row is no click, nor moving within its place.
    drag_to(&shell, cx, X, y + 6.);
    drag_to(&shell, cx, X, y + 9.);
    assert_eq!(host.detents.get(), 0);
    drag_to(&shell, cx, X, y + ROW_PITCH);
    assert_eq!(host.detents.get(), 1);
    // Holding still, and moving within the place, add none.
    drag_to(&shell, cx, X, y + ROW_PITCH);
    drag_to(&shell, cx, X, y + ROW_PITCH + 5.);
    drag_to(&shell, cx, X + 30., y + ROW_PITCH - 5.);
    assert_eq!(host.detents.get(), 1);
    drag_to(&shell, cx, X, y + 2. * ROW_PITCH);
    assert_eq!(host.detents.get(), 2);
    // Back up past one.
    drag_to(&shell, cx, X, y + ROW_PITCH);
    assert_eq!(host.detents.get(), 3);
    // Letting go moves the tab and clicks no more.
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert_eq!(host.moves.get(), 1);
    assert_eq!(host.detents.get(), 3);
}

/// A drag that comes away and goes back where it began, or is cancelled, clicks only for the
/// rows it passed.
#[gpui::test]
fn cancelling_a_drag_gives_no_click(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1, 2, 3], Folders::default());
    let y = press(&shell, cx, 1);
    drag_to(&shell, cx, X, y + 8.);
    shell.update(cx, |shell, cx| assert!(shell.cancel_drag(cx)));
    assert_eq!(host.detents.get(), 0);
    let y = press(&shell, cx, 2);
    drag_to(&shell, cx, X, y + ROW_PITCH);
    assert_eq!(host.detents.get(), 1);
    shell.update(cx, |shell, cx| assert!(shell.cancel_drag(cx)));
    assert_eq!(host.detents.get(), 1);
    assert_eq!(host.moves.get(), 0);
}

/// Crossing the line between the pinned tabs and Today clicks, as passing a row does.
#[gpui::test]
fn crossing_the_pinned_line_gives_a_click(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[1], &[2, 3], Folders::default());
    let y = press(&shell, cx, 1);
    // Down into Today, past its first and second tab.
    drag_to(&shell, cx, X, y + 3. * ROW_PITCH);
    let down = host.detents.get();
    assert!(down >= 1, "no click crossing into Today");
    drag_to(&shell, cx, X, y + 3. * ROW_PITCH + 3.);
    assert_eq!(host.detents.get(), down);
    // And back up over the line.
    drag_to(&shell, cx, X, y);
    assert!(host.detents.get() > down, "no click crossing back to the pinned tabs");
}

/// The pointer going right takes a row into the folder above, which is a new place for it, so a
/// click; going right again past the folder's depth changes nothing.
#[gpui::test]
fn going_into_a_folder_gives_a_click(cx: &mut TestAppContext) {
    let docs = [(20, ("Docs".into(), true))];
    let folders = Folders { folders: docs.into(), parents: Default::default() };
    let (shell, host) = shell_with(cx, &[20, 21, 22], &[4], folders);
    let y = press(&shell, cx, 22);
    // Up past 21 to the gap under the folder.
    drag_to(&shell, cx, X, y - ROW_PITCH);
    assert_eq!(host.detents.get(), 1);
    // In, a level for each 16 pt.
    drag_to(&shell, cx, X + 20., y - ROW_PITCH);
    assert_eq!(host.detents.get(), 2);
    drag_to(&shell, cx, X + 40., y - ROW_PITCH);
    assert_eq!(host.detents.get(), 2);
    // And out again.
    drag_to(&shell, cx, X - 10., y - ROW_PITCH);
    assert_eq!(host.detents.get(), 3);
}

/// A tile dragged across the favorites clicks at each tile it passes, and again on leaving them
/// for the list.
#[gpui::test]
fn each_tile_passed_gives_a_click(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1, 2], Folders::default());
    shell.update(cx, |shell, cx| shell.set_favorites(vec![(10, 0), (11, 0), (12, 0)], cx));
    // Where the middle of the tile at `index` of the three is.
    let middle = |shell: &gpui::Entity<Shell>, cx: &mut TestAppContext, index: usize| {
        shell.update(cx, |shell, _| {
            let [left, top, width] = shell.tile_places(3)[index];
            (left + width / 2., shell.tiles_top() + top + TILE_HEIGHT / 2.)
        })
    };
    let (x, y) = middle(&shell, cx, 0);
    shell.update(cx, |shell, _| shell.press_tile(10, at(x, y)));
    let (x, y) = middle(&shell, cx, 1);
    drag_to(&shell, cx, x, y);
    assert_eq!(host.detents.get(), 1);
    drag_to(&shell, cx, x + 3., y + 2.);
    assert_eq!(host.detents.get(), 1);
    let (x, y) = middle(&shell, cx, 2);
    drag_to(&shell, cx, x, y);
    assert_eq!(host.detents.get(), 2);
    // Down out of the favorites, into the list.
    let y = shell.update(cx, |shell, _| row_y(shell, 2));
    drag_to(&shell, cx, X, y);
    assert_eq!(host.detents.get(), 3);
}
