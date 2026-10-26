//! What the shell asks of the browser, and what the browser tells it about a tab's page,
//! a split view and a Space's folders. The engine answers in main.rs, the preview with
//! made-up tabs in preview.rs.

use std::{collections::HashMap, time::Duration};

use gpui::{App, Bounds, Pixels, Point, SharedString};

use super::{Answer, DownloadAction, Extension, MenuEntry};

/// What the shell asks of the browser. The browser answers by updating the shell afterwards, never
/// during the call.
pub trait Browser {
    fn select(&self, tab: u64, cx: &mut App);
    /// Closes a Today tab. A pinned tab stays, and only its page goes. A tab in a split view
    /// leaves it, unless `split` closes the rest of the split with it.
    fn close(&self, tab: u64, split: bool, cx: &mut App);
    /// Closes the Today tabs below `tab`, showing `tab` if one of them showed.
    fn close_below(&self, tab: u64, cx: &mut App);
    /// Opens a copy of `tab` with its history, under it in Today, and shows it.
    fn duplicate(&self, tab: u64, cx: &mut App);
    /// Moves `tab` to `space`. The window goes with it if it shows it. Into another profile's Space
    /// it goes as a new tab, whose page loads there again.
    fn move_to_space(&self, tab: u64, space: u64, cx: &mut App);
    /// Whether to warn before moving a tab to another profile's Space, which the UI does once.
    fn warn_moving_profiles(&self, cx: &mut App) -> bool;
    fn set_warn_moving_profiles(&self, warn: bool, cx: &mut App);
    /// Pins `tab` at the bottom of its Space's pinned tabs, or unpins it to the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App);
    /// Makes `tab` the favorite at `index` of the favorites without it, else the last, out of its
    /// Space, or moves it there among them.
    fn move_to_favorites(&self, tab: u64, index: usize, cx: &mut App);
    /// Moves `tab` to `index` in its Space's Today tabs, or in its pinned tabs, inside `folder` if
    /// it isn't 0, pinning or unpinning it, with the rest of its split view. `index` counts tabs,
    /// not rows, without the split's, and folders directly in the pinned tabs or `folder` count
    /// one. A favorite goes to the Space shown.
    fn move_tab(&self, tab: u64, pinned: bool, folder: u64, index: usize, cx: &mut App);
    /// Opens tabs loading `urls`, in their order, at `index` in the Space shown's Today tabs, or
    /// in its pinned tabs, inside `folder` if it isn't 0, counted as `move_tab` counts. Shows the
    /// first and returns it, or 0 if none opened.
    fn open_at(&self, urls: &[String], pinned: bool, folder: u64, index: usize, cx: &mut App)
    -> u64;
    /// Makes an open folder named `name` at `index` in `folder`, else in the pinned tabs of
    /// `space`, counted as `move_tab` counts, else at the bottom, and returns it.
    fn create_folder(&self, space: u64, name: &str, folder: u64, index: usize, cx: &mut App)
    -> u64;
    fn rename_folder(&self, folder: u64, name: &str, cx: &mut App);
    fn set_folder_open(&self, folder: u64, open: bool, cx: &mut App);
    /// Moves `folder`, with what's in it, to `index` in `to`, else in the pinned tabs, counted as
    /// `move_tab` counts.
    fn move_folder(&self, folder: u64, to: u64, index: usize, cx: &mut App);
    /// Deletes `folder`, leaving what was in it where it was.
    fn delete_folder(&self, folder: u64, cx: &mut App);
    /// Takes `tab` out of its split view.
    fn separate(&self, tab: u64, cx: &mut App);
    /// Moves `tab` to `index` in its split view, counted from the left or the top.
    fn move_in_split(&self, tab: u64, index: usize, cx: &mut App);
    /// Stacks the pages of `tab`'s split view top to bottom, or puts them side by side.
    fn set_split_stacked(&self, tab: u64, stacked: bool, cx: &mut App);
    /// Gives the pages of `tab`'s split view `shares` of it, in their order, or even shares.
    fn set_split_shares(&self, tab: u64, shares: Option<&[f64]>, cx: &mut App);
    /// Puts `tab` in a split view with `with`, at its left end if `left`, else its right end, and
    /// shows it there.
    fn split_with(&self, tab: u64, with: u64, left: bool, cx: &mut App);
    /// Shows the window over the page that draws `Shell::drop_targets`, drawing it again, or
    /// hides it.
    fn show_drop_targets(&self, shown: bool, cx: &mut App);
    /// Loads a pinned tab's pinned URL, unless it's there already.
    fn back_to_pinned_url(&self, tab: u64, cx: &mut App);
    /// Mutes `tab`, or unmutes it.
    fn set_muted(&self, tab: u64, muted: bool, cx: &mut App);
    /// Closes the Today tabs of the Space shown, but the one shown.
    fn clear(&self, cx: &mut App);
    /// Brings back the tabs the last clear closed, and returns how many.
    fn undo_clear(&self, cx: &mut App) -> usize;
    /// Brings back the Space deleted last, and shows it. Returns whether it did.
    fn undo_delete_space(&self, cx: &mut App) -> bool;
    /// Reopens the Today tab closed last, and shows it.
    fn reopen_closed_tab(&self, cx: &mut App);
    /// Reopens the archived tab `archived` at the top of the Today tabs of the Space shown, and
    /// shows it.
    fn restore_archived(&self, archived: u64, cx: &mut App);
    /// Takes the archived tab `archived` out of the archive.
    fn remove_archived(&self, archived: u64, cx: &mut App);
    fn clear_archive(&self, cx: &mut App);
    /// Does `action` to the download.
    fn act_on_download(&self, download: u64, action: DownloadAction, cx: &mut App);
    /// Takes the downloads that aren't running or paused off the list, keeping their files.
    fn clear_downloads(&self, cx: &mut App);
    /// Looks up the icon the system shows for the download's file, and calls
    /// `Shell::set_download_icon`.
    fn download_icon(&self, download: u64, cx: &mut App);
    /// Looks up the favicon the browser has for `url`, and calls `Shell::set_page_favicon`.
    fn page_favicon(&self, url: &str, cx: &mut App);
    /// The starts of today, yesterday, this week and last week in the user's calendar, in
    /// milliseconds since the Unix epoch.
    fn day_starts(&self) -> [f64; 4];
    fn go_back(&self, tab: u64, cx: &mut App);
    fn go_forward(&self, tab: u64, cx: &mut App);
    fn reload(&self, tab: u64, bypass_cache: bool, cx: &mut App);
    /// Deletes the cookies, or else the cache, the profile keeps for the site of the tab's page,
    /// then reloads it.
    fn clear_site(&self, tab: u64, cookies: bool, cx: &mut App);
    /// Answers what the tab's page asks (Shell::set_question), if it still asks.
    fn answer_permission(&self, tab: u64, answer: Answer, cx: &mut App);
    fn stop(&self, tab: u64, cx: &mut App);
    /// Zooms the tab's page in (1), out (-1) or to 100 % (0), for its site.
    fn zoom(&self, tab: u64, step: i32, cx: &mut App);
    /// Opens the tab's source in a tab under it.
    fn view_source(&self, tab: u64, cx: &mut App);
    /// Asks where to save the tab's page, and saves it there.
    fn save_page(&self, tab: u64, cx: &mut App);
    /// Shows print preview for the tab's page.
    fn print(&self, tab: u64, cx: &mut App);
    /// Opens or closes the tab's DevTools (0), opens or closes them at the console (1), or opens
    /// them picking an element (2).
    fn devtools(&self, tab: u64, what: i32, cx: &mut App);
    /// Opens Chrome's Task Manager, with the page shown selected.
    fn task_manager(&self, cx: &mut App);
    /// Lists `extensions` in the menu bar's Extensions menu.
    fn set_extensions_menu(&self, extensions: &[Extension], cx: &mut App);
    /// Runs extension `id`'s action on the tab shown, its popup hanging from `anchor` in the
    /// window, and returns whether the extension is turned on in the tab's profile.
    fn run_extension(&self, id: &str, anchor: Bounds<Pixels>, cx: &mut App) -> bool;
    /// Runs extension `id`'s keyboard command `name` on the tab shown, and returns whether the
    /// extension is turned on in the tab's profile and listening.
    fn run_extension_command(&self, id: &str, name: &str, cx: &mut App) -> bool;
    /// Pins extension `id`'s action in `profile`, or unpins it.
    fn set_extension_pinned(&self, profile: u64, id: &str, pinned: bool, cx: &mut App);
    /// Removes extension `id` from `profile`, with the data it keeps, and returns whether it went.
    fn remove_extension(&self, profile: u64, id: &str, cx: &mut App) -> bool;
    /// Opens an extension's options page, `url`, in a new tab, and shows it.
    fn open_extension_options(&self, url: &str, cx: &mut App);
    /// Opens Chrome's extensions page in a new tab, at extension `id`'s details if there's one,
    /// and shows it.
    fn manage_extensions(&self, id: Option<&str>, cx: &mut App);
    /// Closes the window with the keyboard if it's another of the app's, such as Chrome's Task
    /// Manager, and says whether it did.
    fn close_other_window(&self, cx: &mut App) -> bool;
    fn show_space(&self, space: u64, cx: &mut App);
    /// The tab the window shows when it shows `space`, or the one it shows, else 0.
    fn space_tab(&self, space: u64) -> u64;
    /// Makes an empty Space named `name` after the others, in `profile` if it isn't 0, and returns
    /// it.
    fn create_space(&self, name: &str, profile: u64, cx: &mut App) -> u64;
    fn rename_space(&self, space: u64, name: &str, cx: &mut App);
    /// Names a tab, which shows `name` for its title from then on, or gives it back its page's
    /// title for an empty one.
    fn rename_tab(&self, tab: u64, name: &str, cx: &mut App);
    /// Gives a tab `icon`, an emoji, to show in place of its page's favicon, kept as its name is,
    /// or gives it back its favicon for an empty one.
    fn set_icon(&self, tab: u64, icon: &str, cx: &mut App);
    /// Deletes a Space, closing its tabs, and returns whether it will, as the last can't go.
    fn delete_space(&self, space: u64, cx: &mut App) -> bool;
    /// Gives a Space a colour, 0xRRGGBB, or none, for the system's accent colour.
    fn set_space_color(&self, space: u64, color: Option<u32>, cx: &mut App);
    /// Puts an empty Space in `profile`.
    fn set_space_profile(&self, space: u64, profile: u64, cx: &mut App);
    /// Makes a profile named `name`, and returns it. It's listed once it's made.
    fn create_profile(&self, name: &str, cx: &mut App) -> u64;
    /// Deletes a profile no Space or favorite is in, with everything the browser keeps in it.
    fn delete_profile(&self, profile: u64, cx: &mut App);
    /// Shows a menu of `items` at `position` in the window, or in the reveal's panel if
    /// `in_reveal`, as a right-click there does, or with the item at `over` on `position`, its
    /// title's top left there, as a pop-up button's menu does. Then calls `chosen` once it closes,
    /// with the index of the item chosen, if any.
    fn menu(
        &self,
        items: Vec<MenuEntry>,
        position: Point<Pixels>,
        over: Option<usize>,
        in_reveal: bool,
        chosen: Box<dyn FnOnce(Option<usize>, &mut App)>,
        cx: &mut App,
    );
    /// Asks whether to do what `message` asks, with `detail` under it, and calls `confirmed` if the
    /// user chooses `action` over Cancel.
    fn confirm(
        &self,
        message: String,
        detail: String,
        action: &'static str,
        confirmed: Box<dyn FnOnce(&mut App)>,
        cx: &mut App,
    );
    /// Gives the UI the keyboard, from the page if the page has it.
    fn focus_ui(&self, cx: &mut App);
    /// Lays the page out at `frame` in the window, with corners of `radius`
    /// (`Shell::page_frame`). A split view's pages share it.
    fn lay_out_page(&self, frame: Bounds<Pixels>, radius: f32, cx: &mut App);
    /// The colour of the ring inside a split view's focused page, 0xRRGGBB.
    fn set_focus_color(&self, color: u32, cx: &mut App);
    /// What the window's pages show until their first frame, 0xRRGGBB.
    fn set_page_color(&self, color: u32, cx: &mut App);
    /// Keeps the find bar and the zoom HUD at the top right corner of a split view's focused page,
    /// `right` points further in from the window's right edge and `top` further down.
    fn set_pane_corner(&self, right: f32, top: f32, cx: &mut App);
    /// Shows the window's close, minimize and zoom buttons, or hides them.
    fn show_window_buttons(&self, shown: bool, cx: &mut App);
    /// Does what a click on the window's close, minimize or zoom button does.
    fn press_window_button(&self, button: WindowButton, cx: &mut App);
    /// Shows the sidebar reveal's window over the page, `width` points wide from the browser
    /// window's left edge and as tall as it. It calls `Shell::pointer_at` as the pointer moves
    /// anywhere in the browser's windows until it's hidden.
    fn show_reveal(&self, width: f32, cx: &mut App);
    /// The docked sidebar's width the last launch kept, if it kept one.
    fn saved_sidebar_width(&self) -> Option<f32>;
    /// Keeps the docked sidebar's width for the next launch.
    fn set_sidebar_width(&self, width: f32, cx: &mut App);
    fn hide_reveal(&self, cx: &mut App);
    /// A detent on the trackpad, as a swipe's nearest Space changes.
    fn detent(&self, cx: &mut App);
    /// Calls `Shell::control_released` once the user lets go of ⌃.
    fn watch_control(&self, cx: &mut App);
    /// Puts `text` on the clipboard.
    fn copy(&self, text: &str, cx: &mut App);
    /// Shows `text` in the toast at the window's top right for `time`, then `key`'s chip and the
    /// words after it if there's a key.
    fn toast(&self, text: &str, key: Option<(&str, &str)>, time: Duration, cx: &mut App);
    /// The system's accent colour, as 0xRRGGBB. The app calls `Shell::accent_color_changed` when
    /// it changes.
    fn accent_color(&self) -> u32;
    /// The user asked the system for less motion, so the New Tab page has no intro.
    fn reduce_motion(&self) -> bool;
    /// Opens the command bar, for a new tab or on `editing`'s URL: out of the URL field at `field`
    /// in the window, or centred over it.
    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        field: Option<Bounds<Pixels>>,
        cx: &mut App,
    );
    /// Opens the command bar for a new tab as the New Tab page's box, in the card at `card`.
    fn new_tab_bar(&self, card: Bounds<Pixels>, cx: &mut App);
    /// Takes the page shown out of the card while the New Tab page is in its place, or puts it
    /// back.
    fn hide_page(&self, hidden: bool, cx: &mut App);
    /// Opens the command bar over `pane`, a new pane in the card at `card`, for a page to show
    /// there in a split view with `with`.
    fn split_bar(&self, with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App);
    /// Opens the tab's find bar with the keyboard.
    fn find(&self, tab: u64, cx: &mut App);
    /// Opens the Settings window, or brings it to the front.
    fn settings(&self, cx: &mut App);
    /// Selects the tab's next match, or the one before, opening its find bar if it's closed.
    fn find_next(&self, tab: u64, forward: bool, cx: &mut App);
    /// Scrolls the tab's page to put its selection in the middle.
    fn jump_to_selection(&self, tab: u64, cx: &mut App);
    fn quit(&self, cx: &mut App);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WindowButton {
    Close,
    Minimize,
    Zoom,
}

/// Where a tab's page is, as the browser reports it.
#[derive(Clone, Default)]
pub struct Page {
    pub url: SharedString,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub loading: bool,
    /// The page isn't loaded, as in a tab put back and not yet shown, or a pinned tab closed. It
    /// loads once it's shown.
    pub unloaded: bool,
    /// The tab is pinned or a favorite, and its page isn't the one it was pinned or made a favorite
    /// at.
    pub away_from_pinned_url: bool,
    /// The page is playing sound, muted or not, or was in the last 2 s.
    pub audible: bool,
    pub muted: bool,
    /// The emoji the user gave the tab to show in place of its page's favicon, or empty.
    pub icon: SharedString,
}

/// A split view: its tabs in their order, which sit together in their list as one row, shown side
/// by side, else stacked top to bottom.
#[derive(Clone, Default)]
pub struct Split {
    pub tabs: Vec<u64>,
    pub stacked: bool,
    /// Each tab's share of the split's length with the gap after it, adding up to 1, or none
    /// while they're even.
    pub shares: Vec<f64>,
}

/// A Space's folders in its pinned tabs. What's in each follows it there.
#[derive(Clone, Default)]
pub struct Folders {
    /// Each folder's name, and whether it's open.
    pub folders: HashMap<u64, (SharedString, bool)>,
    /// The folder holding each pinned tab or folder in one.
    pub parents: HashMap<u64, u64>,
}
