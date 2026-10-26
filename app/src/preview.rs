//! The UI alone, with made-up tabs, for working on how it looks: it needs no engine and rebuilds in
//! seconds. `cargo run --release --bin preview`. The browser's own rules for tabs live in the core,
//! so the ones here are only stand-ins.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use browser::log;
use browser::shell::{
    self, Archived, Browser, Command, Download, DownloadAction, DownloadState, Extension, Folders,
    MenuEntry, Page, Shell, Split, WindowButton,
};
use gpui::{
    App, AppContext, Bounds, Focusable, Pixels, Point, SharedString, TitlebarOptions,
    WeakEntity, WindowBounds, WindowOptions, point, px, size,
};

unsafe extern "C" {
    // platform/mac/host.mm: where a window goes, centered on the main display or, with
    // BROWSER_DISPLAY=builtin, on the Mac's own.
    fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
}

// Two Spaces, so the switcher shows, the second in Dia's pink. The tabs are in the first, the top
// two pinned, the second in a folder.
const SPACES: [(&str, Option<u32>); 2] = [("Space 1", None), ("Reading", Some(0xbf556a))];
const TABS: [(&str, &str); 5] = [
    (
        "Arc (web browser) - Wikipedia",
        "https://en.wikipedia.org/wiki/Arc_(web_browser)",
    ),
    ("Hacker News", "https://news.ycombinator.com/"),
    ("Zed — The editor for what's next", "https://zed.dev/"),
    (
        "Rust Programming Language",
        "https://www.rust-lang.org/",
    ),
    ("", "https://example.com/"),
];

// The favorites, which every Space shows above its tabs.
const FAVORITES: [(&str, &str); 4] = [
    ("Gmail", "https://mail.google.com/"),
    ("Calendar", "https://calendar.google.com/"),
    ("GitHub", "https://github.com/"),
    ("YouTube", "https://www.youtube.com/"),
];

// The archive: tabs closed hours and days ago, newest first.
const ARCHIVE: [(&str, &str, f64); 6] = [
    ("Wikipedia", "https://www.wikipedia.org/", 0.05),
    ("Internet Assigned Numbers Authority", "https://www.iana.org/", 0.2),
    ("W3C", "https://www.w3.org/", 1.1),
    ("The GNU Operating System and the Free Software Movement", "https://www.gnu.org/", 2.5),
    ("Chromium", "https://www.chromium.org/", 6.),
    ("", "https://example.org/", 12.),
];
const DAY: f64 = 86_400_000.;

// The downloads: one running, one paused, one held, and finished ones, newest first, with their
// bytes so far and in all, and how many days ago they started.
const DOWNLOADS: [(&str, &str, i64, i64, DownloadState, bool, f64); 6] = [
    ("zed-0.170.dmg", "https://zed.dev/", 48_200_000, 121_500_000, RUNNING, false, 0.),
    ("rust-book.pdf", "https://doc.rust-lang.org/", 3_100_000, 7_800_000, PAUSED, false, 0.01),
    ("installer.command", "http://example.com/", 2_000, 4_000, RUNNING, true, 0.02),
    ("report.csv", "https://data.example.org/", 18_400, 18_400, DONE, false, 0.3),
    ("photo.jpg", "https://commons.wikimedia.org/", 2_480_000, 2_480_000, DONE, false, 1.2),
    ("old-notes.txt", "https://example.net/", 0, 0, DownloadState::Cancelled, false, 9.),
];
const RUNNING: DownloadState = DownloadState::Running;
const PAUSED: DownloadState = DownloadState::Paused;
const DONE: DownloadState = DownloadState::Done;

/// Reports the Space shown with `today` as its Today tabs.
fn set_today(shell: &mut Shell, today: Vec<u64>, cx: &mut gpui::Context<Shell>) {
    set_lists(shell, shell.pinned().to_vec(), today, cx);
}

fn set_lists(shell: &mut Shell, pinned: Vec<u64>, today: Vec<u64>, cx: &mut gpui::Context<Shell>) {
    let (splits, folders) = (shell.splits(), shell.folders());
    shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
}

/// The folders holding `row`, innermost first.
fn holders(folders: &Folders, row: u64) -> impl Iterator<Item = u64> + '_ {
    std::iter::successors(folders.parents.get(&row).copied(), |folder| {
        folders.parents.get(folder).copied()
    })
}

/// Where `row`'s rows end in `pinned`: past what's in it, if it's a folder.
fn end_of(pinned: &[u64], folders: &Folders, row: u64) -> usize {
    let at = pinned.iter().position(|&id| id == row).map_or(0, |at| at + 1);
    let within = pinned[at..].iter().take_while(|&&id| holders(folders, id).any(|f| f == row));
    at + within.count()
}

/// Where `index` in `folder`, else in the pinned tabs, is in `pinned`, as the core counts.
fn pinned_at(pinned: &[u64], folders: &Folders, folder: u64, index: usize) -> usize {
    let start = pinned.iter().position(|&id| id == folder).map_or(0, |at| at + 1);
    let end = if folder == 0 { pinned.len() } else { end_of(pinned, folders, folder) };
    let parent = |at: &usize| folders.parents.get(&pinned[*at]).copied().unwrap_or(0) == folder;
    (start..end).filter(parent).nth(index).unwrap_or(end)
}

/// Moves `row`, with what's in it if it's a folder, to `index` in `folder`, else in the pinned
/// tabs, as the core does.
fn move_pinned(
    shell: &mut Shell,
    row: u64,
    folder: u64,
    index: usize,
    cx: &mut gpui::Context<Shell>,
) {
    let (mut pinned, mut folders) = (shell.pinned().to_vec(), shell.folders());
    let block = match pinned.iter().position(|&id| id == row) {
        Some(at) => pinned.drain(at..end_of(&pinned, &folders, row)).collect(),
        None => vec![row],
    };
    match folder {
        0 => folders.parents.remove(&row),
        folder => folders.parents.insert(row, folder),
    };
    let to = pinned_at(&pinned, &folders, folder, index);
    pinned.splice(to..to, block);
    let mut today = shell.today().to_vec();
    today.retain(|&id| id != row);
    let splits = shell.splits();
    shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0., |now| now.as_millis() as f64)
}

struct Fake {
    shell: WeakEntity<Shell>,
    next_id: Cell<u64>,
    spaces: RefCell<Vec<(u64, SharedString, Option<u32>, u64)>>,
    archive: RefCell<Vec<(u64, &'static str, &'static str, f64)>>,
    downloads: RefCell<Vec<Download>>,
}

impl Fake {
    fn report_archive(&self, cx: &mut App) {
        let archive = self
            .archive
            .borrow()
            .iter()
            .map(|&(id, title, url, archived)| Archived {
                id,
                url: url.into(),
                title: title.into(),
                archived,
            })
            .collect();
        self.later(cx, move |shell, cx| shell.set_archive(archive, cx));
    }

    fn report_downloads(&self, cx: &mut App) {
        let downloads = self.downloads.borrow().clone();
        self.later(cx, move |shell, cx| shell.set_downloads(downloads, cx));
    }

    /// Changes a folder's name or whether it's open.
    fn change_folder(
        &self,
        folder: u64,
        cx: &mut App,
        change: impl FnOnce(&mut (SharedString, bool)) + 'static,
    ) {
        self.later(cx, move |shell, cx| {
            let mut folders = shell.folders();
            if let Some(entry) = folders.folders.get_mut(&folder) {
                change(entry);
            }
            let (pinned, today) = (shell.pinned().to_vec(), shell.today().to_vec());
            let splits = shell.splits();
            shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
        });
    }

    fn report_spaces(&self, cx: &mut App) {
        let spaces = self.spaces.borrow().clone();
        self.later(cx, move |shell, cx| shell.set_spaces(spaces, cx));
    }

    /// Changes the shell once the current update is over, as the engine reports back.
    fn later(&self, cx: &mut App, change: impl FnOnce(&mut Shell, &mut gpui::Context<Shell>) + 'static) {
        let shell = self.shell.clone();
        cx.defer(move |cx| {
            shell.update(cx, change).ok();
        });
    }
}

impl Browser for Fake {
    fn select(&self, tab: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| shell.set_selected(tab, cx));
    }

    fn close(&self, tab: u64, _split: bool, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let Some(index) = today.iter().position(|&id| id == tab) else {
                return;
            };
            today.remove(index);
            let next = today.get(index).or(today.last()).copied().unwrap_or(0);
            set_today(shell, today, cx);
            if shell.selected() == tab {
                shell.set_selected(next, cx);
            }
        });
    }

    fn close_below(&self, tab: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let Some(index) = today.iter().position(|&id| id == tab) else {
                return;
            };
            today.truncate(index + 1);
            if !today.contains(&shell.selected()) && !shell.pinned().contains(&shell.selected()) {
                shell.set_selected(tab, cx);
            }
            set_today(shell, today, cx);
        });
    }

    // Under the tab, with its title and page, as the core does.
    fn duplicate(&self, tab: u64, cx: &mut App) {
        let copy = self.next_id.get();
        self.next_id.set(copy + 1);
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let at = today.iter().position(|&id| id == tab).map_or(0, |index| index + 1);
            today.insert(at, copy);
            shell.set_title(copy, shell.title(tab), cx);
            shell.set_page(copy, shell.page(tab), cx);
            set_today(shell, today, cx);
            shell.set_selected(copy, cx);
        });
    }

    // The made-up tabs stay in their Space.
    fn move_to_space(&self, _: u64, _: u64, _: &mut App) {}

    fn warn_moving_profiles(&self, _: &mut App) -> bool {
        true
    }

    fn set_warn_moving_profiles(&self, _: bool, _: &mut App) {}

    // As the core does: pinned at the bottom of the pinned tabs, unpinned at the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut tabs = [shell.pinned().to_vec(), shell.today().to_vec()];
            let (from, to) = if pinned { (1, 0) } else { (0, 1) };
            let Some(index) = tabs[from].iter().position(|&id| id == tab) else {
                return;
            };
            tabs[from].remove(index);
            let at = if pinned { tabs[to].len() } else { 0 };
            tabs[to].insert(at, tab);
            let [pinned, today] = tabs;
            set_lists(shell, pinned, today, cx);
        });
    }

    fn move_to_favorites(&self, tab: u64, index: usize, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut favorites = shell.favorites().to_vec();
            let favorite = favorites.contains(&tab);
            favorites.retain(|&id| id != tab);
            favorites.insert(index.min(favorites.len()), tab);
            // The core reports a tab in its new place before it leaves the old.
            shell.set_favorites(favorites.into_iter().map(|tab| (tab, 0)).collect(), cx);
            if !favorite {
                let (mut pinned, mut today) = (shell.pinned().to_vec(), shell.today().to_vec());
                let mut splits = shell.splits();
                pinned.retain(|&id| id != tab);
                today.retain(|&id| id != tab);
                splits.iter_mut().for_each(|split| split.tabs.retain(|&id| id != tab));
                splits.retain(|split| split.tabs.len() > 1);
                let folders = shell.folders();
                shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
            }
        });
    }

    fn move_tab(&self, tab: u64, pinned: bool, folder: u64, index: usize, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            if pinned {
                move_pinned(shell, tab, folder, index, cx);
            } else {
                let mut tabs = [shell.pinned().to_vec(), shell.today().to_vec()];
                for list in &mut tabs {
                    list.retain(|&id| id != tab);
                }
                tabs[1].insert(index.min(tabs[1].len()), tab);
                let [pinned, today] = tabs;
                set_lists(shell, pinned, today, cx);
            }
            if shell.favorites().contains(&tab) {
                let favorites = shell.favorites().iter().copied().filter(|&id| id != tab);
                shell.set_favorites(favorites.map(|tab| (tab, 0)).collect(), cx);
            }
        });
    }

    fn create_folder(&self, _: u64, name: &str, folder: u64, index: usize, cx: &mut App) -> u64 {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let name: SharedString = name.to_string().into();
        self.later(cx, move |shell, cx| {
            let (mut pinned, mut folders) = (shell.pinned().to_vec(), shell.folders());
            let at = pinned_at(&pinned, &folders, folder, index);
            pinned.insert(at, id);
            folders.folders.insert(id, (name, true));
            if folder != 0 {
                folders.parents.insert(id, folder);
            }
            let (today, splits) = (shell.today().to_vec(), shell.splits());
            shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
        });
        id
    }

    fn rename_folder(&self, folder: u64, name: &str, cx: &mut App) {
        let name: SharedString = name.to_string().into();
        self.change_folder(folder, cx, move |entry| entry.0 = name);
    }

    fn set_folder_open(&self, folder: u64, open: bool, cx: &mut App) {
        self.change_folder(folder, cx, move |entry| entry.1 = open);
    }

    fn move_folder(&self, folder: u64, to: u64, index: usize, cx: &mut App) {
        self.later(cx, move |shell, cx| move_pinned(shell, folder, to, index, cx));
    }

    fn delete_folder(&self, folder: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let (mut pinned, mut folders) = (shell.pinned().to_vec(), shell.folders());
            pinned.retain(|&id| id != folder);
            folders.folders.remove(&folder);
            let holder = folders.parents.remove(&folder).unwrap_or(0);
            for parent in folders.parents.values_mut().filter(|parent| **parent == folder) {
                *parent = holder;
            }
            folders.parents.retain(|_, parent| *parent != 0);
            let (today, splits) = (shell.today().to_vec(), shell.splits());
            shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
        });
    }

    // As the core does, without the undo.
    fn clear(&self, cx: &mut App) {
        self.later(cx, |shell, cx| {
            let selected = shell.selected();
            let today = shell.today().iter().copied().filter(|&tab| tab == selected).collect();
            set_today(shell, today, cx);
        });
    }

    fn undo_clear(&self, _: &mut App) -> usize {
        0
    }

    fn undo_delete_space(&self, _: &mut App) -> bool {
        false
    }

    fn reopen_closed_tab(&self, _: &mut App) {}

    // Takes it out of the archive, without opening it.
    fn restore_archived(&self, archived: u64, cx: &mut App) {
        self.archive.borrow_mut().retain(|&(id, ..)| id != archived);
        self.report_archive(cx);
    }

    fn remove_archived(&self, archived: u64, cx: &mut App) {
        self.archive.borrow_mut().retain(|&(id, ..)| id != archived);
        self.report_archive(cx);
    }

    fn clear_archive(&self, cx: &mut App) {
        self.archive.borrow_mut().clear();
        self.report_archive(cx);
    }

    // Opening and showing a file do nothing here.
    fn act_on_download(&self, download: u64, action: DownloadAction, cx: &mut App) {
        let mut downloads = self.downloads.borrow_mut();
        let Some(index) = downloads.iter().position(|each| each.id == download) else {
            return;
        };
        let download = &mut downloads[index];
        match action {
            DownloadAction::Open if download.state != DownloadState::Done => {
                download.open_when_done = !download.open_when_done
            }
            DownloadAction::Pause => download.state = DownloadState::Paused,
            DownloadAction::Resume => download.state = DownloadState::Running,
            DownloadAction::Cancel => download.state = DownloadState::Cancelled,
            DownloadAction::Keep => download.held = false,
            DownloadAction::Remove => _ = downloads.remove(index),
            DownloadAction::Open | DownloadAction::Show => {}
        }
        drop(downloads);
        self.report_downloads(cx);
    }

    fn clear_downloads(&self, cx: &mut App) {
        self.downloads.borrow_mut().retain(|each| {
            matches!(each.state, DownloadState::Running | DownloadState::Paused)
        });
        self.report_downloads(cx);
    }

    // The made-up files have the file glyph.
    fn download_icon(&self, _: u64, _: &mut App) {}

    // The made-up pages have the globe.
    fn page_favicon(&self, _: &str, _: &mut App) {}

    // In UTC, with weeks from Monday.
    fn day_starts(&self) -> [f64; 4] {
        let today = now() - now() % DAY;
        // 1 January 1970 was a Thursday.
        let week = today - ((today / DAY + 3.) % 7.) * DAY;
        [today, today - DAY, week, week - 7. * DAY]
    }

    // The made-up pages have no history to go through.
    fn back_to_pinned_url(&self, _: u64, _: &mut App) {}

    fn set_muted(&self, tab: u64, muted: bool, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let page = Page { muted, ..shell.page(tab) };
            shell.set_page(tab, page, cx);
        });
    }

    fn go_back(&self, _: u64, _: &mut App) {}

    fn go_forward(&self, _: u64, _: &mut App) {}

    fn reload(&self, _: u64, _: bool, _: &mut App) {}

    fn clear_site(&self, _: u64, _: bool, _: &mut App) {}

    fn stop(&self, _: u64, _: &mut App) {}

    fn zoom(&self, _: u64, _: i32, _: &mut App) {}

    // The made-up pages have no source and no DevTools.
    fn view_source(&self, _: u64, _: &mut App) {}
    fn save_page(&self, _: u64, _: &mut App) {}
    fn print(&self, _: u64, _: &mut App) {}

    fn devtools(&self, _: u64, _: i32, _: &mut App) {}

    fn task_manager(&self, _: &mut App) {}

    // The preview has no menu bar and no extensions.
    fn set_extensions_menu(&self, _: &[Extension], _: &mut App) {}

    fn run_extension(&self, _: &str, _: Bounds<Pixels>, _: &mut App) -> bool {
        false
    }

    fn set_extension_pinned(&self, _: u64, _: &str, _: bool, _: &mut App) {}

    fn manage_extensions(&self, _: Option<&str>, _: &mut App) {}

    fn close_other_window(&self, _: &mut App) -> bool {
        false
    }

    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        _: Option<Bounds<Pixels>>,
        cx: &mut App,
    ) {
        // No command bar here: a new tab opens at once.
        if editing.is_some() {
            return;
        }
        let tab = self.next_id.get();
        self.next_id.set(tab + 1);
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            today.insert(0, tab);
            set_today(shell, today, cx);
            shell.set_selected(tab, cx);
        });
    }

    // The made-up pages have nothing to find.
    fn find(&self, _: u64, _: &mut App) {}

    fn find_next(&self, _: u64, _: bool, _: &mut App) {}

    fn jump_to_selection(&self, _: u64, _: &mut App) {}

    // The preview has no Settings window.
    fn settings(&self, _: &mut App) {}

    // Each made-up Space shows its top tab.
    fn show_space(&self, space: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            shell.set_space(space, cx);
            let top = shell.today().first().or(shell.pinned().first()).copied().unwrap_or(0);
            shell.set_selected(top, cx);
        });
    }

    // The made-up Spaces' pages slide in without a selected row.
    fn space_tab(&self, _: u64) -> u64 {
        0
    }

    fn create_space(&self, name: &str, profile: u64, cx: &mut App) -> u64 {
        let space = self.next_id.get();
        self.next_id.set(space + 1);
        self.spaces.borrow_mut().push((space, name.to_string().into(), None, profile));
        self.report_spaces(cx);
        space
    }

    fn rename_space(&self, space: u64, name: &str, cx: &mut App) {
        if let Some(entry) = self.spaces.borrow_mut().iter_mut().find(|(id, ..)| *id == space) {
            entry.1 = name.to_string().into();
        }
        self.report_spaces(cx);
    }

    // The preview's tabs have no page title to go back to, so an empty name leaves the title.
    fn rename_tab(&self, tab: u64, name: &str, cx: &mut App) {
        let name: SharedString = name.to_string().into();
        if !name.is_empty() {
            self.later(cx, move |shell, cx| shell.set_title(tab, name, cx));
        }
    }

    fn delete_space(&self, space: u64, cx: &mut App) -> bool {
        self.spaces.borrow_mut().retain(|(id, ..)| *id != space);
        self.report_spaces(cx);
        true
    }

    fn set_space_color(&self, space: u64, color: Option<u32>, cx: &mut App) {
        if let Some(entry) = self.spaces.borrow_mut().iter_mut().find(|(id, ..)| *id == space) {
            entry.2 = color;
        }
        self.report_spaces(cx);
    }

    fn set_space_profile(&self, space: u64, profile: u64, cx: &mut App) {
        if let Some(entry) = self.spaces.borrow_mut().iter_mut().find(|(id, ..)| *id == space) {
            entry.3 = profile;
        }
        self.report_spaces(cx);
    }

    // The preview has the one profile.
    fn create_profile(&self, _: &str, _: &mut App) -> u64 {
        0
    }

    fn delete_profile(&self, _: u64, _: &mut App) {}

    // The preview's window is GPUI's own, which has the keyboard already.
    fn focus_ui(&self, _: &mut App) {}

    // There's no page, and the window's buttons stay where GPUI put them.
    fn lay_out_page(&self, _: Bounds<Pixels>, _: f32, _: &mut App) {}

    fn set_focus_color(&self, _: u32, _: &mut App) {}

    fn set_page_color(&self, _: u32, _: &mut App) {}

    fn set_pane_corner(&self, _: f32, _: f32, _: &mut App) {}

    // No command bar here, as for a new tab, and no split views but the made-up one.
    fn split_bar(&self, _: u64, _: Bounds<Pixels>, _: Bounds<Pixels>, _: &mut App) {}

    fn separate(&self, _: u64, _: &mut App) {}

    fn move_in_split(&self, _: u64, _: usize, _: &mut App) {}

    fn set_split_stacked(&self, tab: u64, stacked: bool, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut splits = shell.splits();
            for split in splits.iter_mut().filter(|split| split.tabs.contains(&tab)) {
                split.stacked = stacked;
            }
            let (pinned, today) = (shell.pinned().to_vec(), shell.today().to_vec());
            let folders = shell.folders();
            shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
        });
    }

    fn set_split_shares(&self, tab: u64, shares: Option<&[f64]>, cx: &mut App) {
        let shares = shares.map_or_else(Vec::new, <[f64]>::to_vec);
        self.later(cx, move |shell, cx| {
            let mut splits = shell.splits();
            for split in splits.iter_mut().filter(|split| split.tabs.contains(&tab)) {
                split.shares = shares.clone();
            }
            let (pinned, today) = (shell.pinned().to_vec(), shell.today().to_vec());
            let folders = shell.folders();
            shell.set_tabs(shell.space(), pinned, today, splits, folders, cx);
        });
    }

    fn split_with(&self, _: u64, _: u64, _: bool, _: &mut App) {}

    // The targets' window is the app's, over the page, which the preview has neither of.
    fn show_drop_targets(&self, _: bool, _: &mut App) {}

    fn show_window_buttons(&self, _: bool, _: &mut App) {}

    fn press_window_button(&self, _: WindowButton, _: &mut App) {}

    // The reveal's window is the app's, over the page, which the preview has neither of.
    fn show_reveal(&self, _: f32, _: &mut App) {}

    fn hide_reveal(&self, _: &mut App) {}

    fn saved_sidebar_width(&self) -> Option<f32> {
        None
    }

    fn set_sidebar_width(&self, _: f32, _: &mut App) {}

    fn detent(&self, _: &mut App) {}

    // The preview doesn't hear ⌃ go up, so each ⌃Tab there ends its walk.
    fn watch_control(&self, cx: &mut App) {
        self.later(cx, |shell, cx| shell.control_released(cx));
    }

    // The preview leaves the clipboard alone.
    fn copy(&self, _: &str, _: &mut App) {}

    // The toast's window is one the app's host lays over the browser window.
    fn toast(&self, _: &str, _: Option<(&str, &str)>, _: Duration, _: &mut App) {}

    // macOS's default blue.
    fn accent_color(&self) -> u32 {
        0x007aff
    }

    // Menus are AppKit's, and the dialog's window is one the app's host lays over the browser
    // window. The preview goes without both.
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

    fn quit(&self, cx: &mut App) {
        cx.quit();
    }
}

fn main() {
    gpui_platform::application().with_assets(shell::Assets).run(|cx| {
        shell::bind_keys(cx);
        browser::text_input::bind_keys(cx);
        let (width, height) = (1280., 800.);
        let (mut x, mut y) = (0., 0.);
        unsafe { host_centered(width, height, &mut x, &mut y) };
        let bounds = Bounds::new(
            point(px(x as f32), px(y as f32)),
            size(px(width as f32), px(height as f32)),
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                // Arc's: 16 pt buttons centred at (20, 24), 23 apart.
                traffic_light_position: Some(point(px(12.), px(16.))),
            }),
            ..Default::default()
        };
        let window = cx.open_window(options, |window, cx| {
            let shell = cx.new(|cx| {
                let first_space = TABS.len() as u64 + 1;
                // The preview's Spaces and favorites are in its one profile, 0.
                let spaces: Vec<(u64, SharedString, Option<u32>, u64)> = (first_space..)
                    .zip(SPACES)
                    .map(|(id, (name, color))| (id, name.into(), color, 0))
                    .collect();
                let first_archived = first_space + SPACES.len() as u64;
                let archive = (first_archived..)
                    .zip(ARCHIVE)
                    .map(|(id, (title, url, days))| (id, title, url, now() - days * DAY))
                    .collect();
                let first_download = first_archived + ARCHIVE.len() as u64;
                let downloads = (first_download..)
                    .zip(DOWNLOADS)
                    .map(|(id, (name, url, received, total, state, held, days))| Download {
                        id,
                        name: name.into(),
                        url: url.into(),
                        received,
                        total,
                        state,
                        open_when_done: false,
                        held,
                        started: now() - days * DAY,
                    })
                    .collect();
                let first_favorite = first_download + DOWNLOADS.len() as u64;
                let favorites: Vec<u64> = (first_favorite..).take(FAVORITES.len()).collect();
                let folder = first_favorite + FAVORITES.len() as u64;
                let browser = Rc::new(Fake {
                    shell: cx.weak_entity(),
                    next_id: Cell::new(folder + 1),
                    spaces: RefCell::new(spaces.clone()),
                    archive: RefCell::new(archive),
                    downloads: RefCell::new(downloads),
                });
                browser.report_archive(cx);
                browser.report_downloads(cx);
                let mut shell = Shell::new(browser, cx);
                shell.set_profiles(vec![(0, "Default".into())], cx);
                shell.set_spaces(spaces, cx);
                shell.set_space(first_space, cx);
                // The shell forgets what it knows of a tab in no list, so the lists come first.
                shell.set_favorites(favorites.iter().map(|&tab| (tab, 0)).collect(), cx);
                for (&tab, (title, url)) in favorites.iter().zip(FAVORITES) {
                    shell.set_title(tab, title.into(), cx);
                    shell.set_page(tab, Page { url: url.into(), ..Default::default() }, cx);
                }
                for (index, (title, url)) in TABS.iter().enumerate() {
                    let tab = index as u64 + 1;
                    shell.set_title(tab, (*title).into(), cx);
                    let page = Page {
                        url: (*url).into(),
                        // The first tab has been somewhere before, so back shows bright, and it's
                        // pinned somewhere else. The second is a pinned tab closed, and the third
                        // is still loading and plays sound.
                        can_go_back: index == 0,
                        away_from_pinned_url: index == 0,
                        unloaded: index == 1,
                        loading: index == 2,
                        audible: index == 2,
                        ..Default::default()
                    };
                    shell.set_page(tab, page, cx);
                }
                // The second in a folder, and the last two in a split view.
                let pinned = vec![1, folder, 2];
                let folders = Folders {
                    folders: [(folder, ("Docs".into(), true))].into(),
                    parents: [(2, folder)].into(),
                };
                let today = (3..=TABS.len() as u64).collect();
                let split = Split {
                    tabs: vec![TABS.len() as u64 - 1, TABS.len() as u64],
                    ..Split::default()
                };
                shell.set_tabs(first_space, pinned, today, vec![split], folders, cx);
                shell.set_selected(1, cx);
                shell
            });
            window.focus(&shell.focus_handle(cx), cx);
            shell
        });
        match window {
            // PREVIEW_LIBRARY=1 opens the Library, and PREVIEW_LIBRARY=downloads opens it there.
            Ok(window) if std::env::var_os("PREVIEW_LIBRARY").is_some() => {
                let command = match std::env::var("PREVIEW_LIBRARY").as_deref() {
                    Ok("downloads") => Command::ViewDownloads,
                    _ => Command::ViewLibrary,
                };
                window.update(cx, |shell, _, cx| shell.run(command, "preview", cx)).ok();
            }
            Ok(_) => {}
            Err(error) => {
                log!("couldn't open the window: {error}");
                cx.quit();
            }
        }
        cx.activate(true);
    });
}
