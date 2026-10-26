//! The browser. The core (core/, in Chrome's framework) makes the browser window and puts the page
//! in it, through its C API; GPUI draws the shell in a view inside that window, under the page.
//! Chromium owns the application object and the run loop.

use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString, c_char, c_int, c_void},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use browser::{command_bar, dialog, log, toast};
use browser::shell::{
    self, Archived, Browser, Command, Download, DownloadAction, DownloadState, Extension,
    ExtensionAction, ExtensionCommand, Folders, MenuEntry, Page, Shell, Split, WindowButton,
};
use browser::text_input::{Chosen, Copy, Cut, FieldMenu, Paste, Redo, SelectAll, Undo};
use gpui::{
    AnyWindowHandle, App, ApplicationHandle, AsyncApp, Bounds, Context, Focusable, Image,
    ImageFormat, KeyBinding, Menu, MenuItem, OsAction, Pixels, Point, SharedString, SystemMenuType,
    WeakEntity, Window, WindowBackgroundAppearance, WindowOptions, actions, point, prelude::*, px,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod drop_targets;
mod find_bar;
mod reveal;
mod settings;
mod zoom_hud;

/// The core's API, generated from core/public/browser_core.h.
#[allow(non_camel_case_types, dead_code)]
mod bc {
    include!(concat!(env!("OUT_DIR"), "/browser_core.rs"));
}

/// The AppKit side, in platform/mac/.
mod platform {
    use std::ffi::{c_char, c_int, c_void};

    unsafe extern "C" {
        pub fn drive_start();
        pub fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
        pub fn host_ui_view() -> *mut c_void;
        pub fn host_show_window(ui_view: *mut c_void);
        pub fn host_add_overlay(overlay_view: *mut c_void, dismissed: extern "C" fn());
        pub fn host_show_overlay(centered: bool, left: f64, top: f64, width: f64);
        pub fn host_set_overlay_height(height: f64);
        pub fn host_reshape_overlay();
        pub fn host_hide_overlay();
        pub fn host_dismiss_overlay();
        pub fn host_add_find_bar(find_bar_view: *mut c_void, right: f64, top: f64);
        pub fn host_show_find_bar(keyboard: bool);
        pub fn host_hide_find_bar();
        pub fn host_add_hud(hud_view: *mut c_void, right: f64, top: f64);
        pub fn host_set_pane_corner(right: f64, top: f64);
        pub fn host_show_hud();
        pub fn host_hide_hud(seconds: f64);
        pub fn host_add_drop_targets(view: *mut c_void);
        pub fn host_show_drop_targets();
        pub fn host_hide_drop_targets();
        pub fn host_focus(view: *mut c_void);
        pub fn host_show_window_buttons(shown: bool);
        pub fn host_press_window_button(button: i32);
        pub fn host_add_reveal(reveal_view: *mut c_void, pointer_moved: extern "C" fn(x: f64));
        pub fn host_show_reveal(width: f64);
        pub fn host_hide_reveal();
        pub fn host_detent();
        pub fn host_accent_color() -> u32;
        pub fn host_watch_accent_color(changed: extern "C" fn());
        pub fn host_watch_control(released: extern "C" fn());
        pub fn host_take_drops(
            dragged: extern "C" fn(
                phase: c_int,
                x: f64,
                y: f64,
                items: *const host_dragged,
                count: usize,
            ) -> bool,
        );
        pub fn host_copy(text: *const c_char);
        pub fn host_day_starts(starts: *mut f64);
        pub fn host_menu(
            items: *const host_menu_item,
            count: usize,
            x: f64,
            y: f64,
            place: i32,
            over: i32,
        ) -> i32;
        pub fn host_menu_in(
            items: *const host_menu_item,
            count: usize,
            x: f64,
            y: f64,
            view: *mut c_void,
        ) -> i32;
        pub fn host_add_settings(settings_view: *mut c_void);
        pub fn host_show_settings();
        pub fn host_hide_settings();
        pub fn host_close_other_window() -> bool;
        pub fn host_ask(
            message: *const c_char,
            detail: *const c_char,
            buttons: *const *const c_char,
            count: usize,
        ) -> i32;
        pub fn host_add_dialog(dialog_view: *mut c_void);
        pub fn host_show_dialog();
        pub fn host_hide_dialog();
        pub fn host_add_toast(toast_view: *mut c_void);
        pub fn host_show_toast();
        pub fn host_hide_toast();
    }

    /// Where host_menu shows a menu.
    pub const IN_WINDOW: i32 = 0;
    pub const IN_REVEAL: i32 = 1;
    pub const IN_SETTINGS: i32 = 2;

    /// What host_take_drops hears of a drag.
    pub const DRAG_OVER: c_int = 0;
    pub const DRAG_DROP: c_int = 2;

    #[repr(C)]
    pub struct host_dragged {
        pub url: *const c_char,
        pub file: bool,
    }

    #[repr(C)]
    pub struct host_menu_item {
        pub title: *const c_char,
        pub enabled: bool,
        pub key: *const c_char,
        pub command: bool,
        pub shift: bool,
        pub option: bool,
        pub control: bool,
        pub parent: i32,
        pub checked: bool,
        pub swatch: i32,
    }
}

const WINDOW_SIZE: (f64, f64) = (1280., 800.);

/// AppKit's key equivalent for `keystroke`, as SHORTCUTS writes it, and whether it takes ⌘, ⇧, ⌥
/// and ⌃.
fn key_equivalent(keystroke: &str) -> (String, [bool; 4]) {
    let mut parts: Vec<&str> = keystroke.split('-').collect();
    let key = match parts.pop().unwrap_or_default() {
        "enter" => "\r",
        "escape" => "\u{1b}",
        "up" => "\u{f700}",
        "down" => "\u{f701}",
        "left" => "\u{f702}",
        "right" => "\u{f703}",
        key => key,
    };
    let has = |modifier| parts.contains(&modifier);
    (key.to_string(), [has("cmd"), has("shift"), has("alt"), has("ctrl")])
}
const START_URL: &str = "https://en.wikipedia.org/wiki/Arc_(web_browser)";

actions!(browser, [Hide, HideOthers, ShowAll, Minimize]);

thread_local! {
    // The core calls back into the UI from Chromium's event handling, outside any GPUI update.
    static UI: RefCell<Option<(AsyncApp, WeakEntity<Shell>, AnyWindowHandle)>> =
        const { RefCell::new(None) };
    // GPUI starts inside Chromium's run loop, from a callback, and lives as long as the process.
    static APP: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    // The browser window. There's one so far.
    static WINDOW: Cell<bc::bc_id> = const { Cell::new(0) };
}

/// Changes the shell, from a core callback, and returns what the change does, unless there's no
/// shell.
fn update_shell<R>(change: impl FnOnce(&mut Shell, &mut Context<Shell>) -> R) -> Option<R> {
    let ui = UI.with_borrow(|ui| ui.as_ref().map(|(cx, shell, _)| (cx.clone(), shell.clone())));
    let (cx, shell) = ui?;
    cx.update(|cx| shell.update(cx, change).ok())
}

/// Changes the UI outside the shell, from a core callback.
fn update_app(change: impl FnOnce(&mut App)) {
    let cx = UI.with_borrow(|ui| ui.as_ref().map(|(cx, _, _)| cx.clone()));
    if let Some(cx) = cx {
        cx.update(change);
    }
}

/// The window's NSView, for the host.
fn ns_view(window: &Window) -> *mut c_void {
    match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::AppKit(handle)) => handle.ns_view.as_ptr(),
        _ => panic!("the window has no AppKit view"),
    }
}

/// Shows a menu of `items` at `position` in `place` (`platform::IN_*`), or with the item at `over`
/// there, and returns the index of the one chosen once it closes. AppKit runs the menu in an event
/// loop of its own, so this mustn't be called inside a GPUI update.
fn show_menu(
    items: &[MenuEntry],
    position: Point<Pixels>,
    place: i32,
    over: Option<usize>,
) -> Option<usize> {
    let (x, y) = (f64::from(position.x), f64::from(position.y));
    let over = over.map_or(-1, |over| over as i32);
    pop_up(items, |items, count| unsafe { platform::host_menu(items, count, x, y, place, over) })
}

/// Shows a text field's menu of `items` at `position` in `view`, a window's, as show_menu does.
fn show_field_menu(
    items: &[MenuEntry],
    position: Point<Pixels>,
    view: *mut c_void,
) -> Option<usize> {
    let (x, y) = (f64::from(position.x), f64::from(position.y));
    pop_up(items, |items, count| unsafe { platform::host_menu_in(items, count, x, y, view) })
}

/// Gives `show` the host's items for `items`, and returns the index it returns, if one was chosen.
fn pop_up(
    items: &[MenuEntry],
    show: impl FnOnce(*const platform::host_menu_item, usize) -> i32,
) -> Option<usize> {
    let strings: Vec<(CString, CString, [bool; 4])> = items
        .iter()
        .map(|item| {
            let (key, modifiers) = item.shortcut.map(key_equivalent).unwrap_or_default();
            let title = CString::new(item.title.as_ref()).unwrap_or_default();
            (title, CString::new(key).unwrap_or_default(), modifiers)
        })
        .collect();
    let host_items: Vec<_> = strings
        .iter()
        .zip(items)
        .map(|((title, key, [command, shift, option, control]), item)| {
            platform::host_menu_item {
                title: match item.title.is_empty() {
                    true => std::ptr::null(),
                    false => title.as_ptr(),
                },
                enabled: item.enabled,
                key: key.as_ptr(),
                command: *command,
                shift: *shift,
                option: *option,
                control: *control,
                parent: item.parent.map_or(-1, |parent| parent as i32),
                checked: item.checked,
                swatch: item.swatch.map_or(-1, |color| color as i32),
            }
        })
        .collect();
    usize::try_from(show(host_items.as_ptr(), host_items.len())).ok()
}

/// Shows a text field's menu in the field's window, outside the GPUI update that asks for it.
fn field_menu(
    items: Vec<(SharedString, bool)>,
    position: Point<Pixels>,
    window: &Window,
    chosen: Chosen,
    cx: &mut App,
) {
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        // The view is looked up once the update is over, in case its window closed meanwhile.
        let Ok(view) = cx.update_window(handle, |_, window, _| ns_view(window)) else {
            return;
        };
        let items: Vec<MenuEntry> =
            items.into_iter().map(|(title, enabled)| MenuEntry::new(title, enabled)).collect();
        if let Some(index) = show_field_menu(&items, position, view) {
            cx.update_window(handle, |_, window, cx| chosen(index, window, cx)).ok();
        }
    })
    .detach();
}

/// Tells the core where the page goes and the radius of its corners (`Shell::page_frame`).
fn set_page_frame((frame, radius): (Bounds<Pixels>, f32)) {
    unsafe { bc::bc_window_set_page_frame(WINDOW.get(), rect(frame), f64::from(radius)) };
}

fn rect(bounds: Bounds<Pixels>) -> bc::bc_rect {
    bc::bc_rect {
        x: f64::from(bounds.origin.x),
        y: f64::from(bounds.origin.y),
        width: f64::from(bounds.size.width),
        height: f64::from(bounds.size.height),
    }
}

/// Opens a tab loading `url` at the top of the Space's Today tabs, and shows it.
fn open_tab(url: &str) {
    let tab = open_tab_behind(url, false);
    if tab != 0 {
        unsafe { bc::bc_window_select(WINDOW.get(), tab) };
    }
}

/// Opens a tab loading `url` at the top of the Space's Today tabs, behind the one shown, or with
/// `when_shown` loading it only once it's first shown. Returns it, or 0.
fn open_tab_behind(url: &str, when_shown: bool) -> u64 {
    match CString::new(url) {
        Ok(url) => unsafe { bc::bc_tab_open(WINDOW.get(), url.as_ptr(), when_shown) },
        Err(_) => 0,
    }
}

/// Puts `tab` in a split view with `with`, on its right, and shows it there. The pane being added
/// for it, if there is one, gives the split its room first, so it shows at once at its place.
fn split_tab(tab: u64, with: u64, cx: &mut App) {
    shell::pane_added(cx);
    let window = WINDOW.get();
    unsafe {
        bc::bc_tab_split_with(tab, with, false);
        bc::bc_window_select(window, tab);
    }
}

/// Opens a tab loading `url` in a split view with `with`, on its right, and shows it there.
fn open_split(url: &str, with: u64, cx: &mut App) {
    let Ok(url) = CString::new(url) else {
        return;
    };
    let tab = unsafe { bc::bc_tab_open(WINDOW.get(), url.as_ptr(), false) };
    if tab != 0 {
        split_tab(tab, with, cx);
    }
}

/// Makes a Space after the others.
fn create_space(name: &str) -> bc::bc_id {
    let name = CString::new(name).unwrap_or_default();
    unsafe { bc::bc_space_create(name.as_ptr()) }
}

/// The URL the core's `get` copies for `text`.
fn url_for(text: &str, get: impl Fn(*const c_char, *mut c_char, usize) -> usize) -> Option<String> {
    let text = CString::new(text).ok()?;
    // Long enough for most, so the core reads what was typed once.
    let mut url = vec![0u8; 2048];
    let length = get(text.as_ptr(), url.as_mut_ptr().cast(), url.len());
    if length >= url.len() {
        url.resize(length + 1, 0);
        get(text.as_ptr(), url.as_mut_ptr().cast(), url.len());
    }
    url.truncate(length);
    (length > 0).then(|| String::from_utf8_lossy(&url).into_owned())
}

/// The dialog's requests, to the host's dialog window.
struct DialogHost;

impl dialog::Host for DialogHost {
    fn show(&self) {
        unsafe { platform::host_show_dialog() };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_dialog() };
    }
}

/// The toast's requests, to the host's toast window.
struct ToastHost;

impl toast::Host for ToastHost {
    fn show(&self) {
        unsafe { platform::host_show_toast() };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_toast() };
    }
}

/// The command bar's requests, as calls to the core and to the host's overlay window.
struct Bar;

impl command_bar::Host for Bar {
    fn suggest(&self, site: Option<&str>, text: &str, complete: bool) {
        let Ok(site) = site.map(CString::new).transpose() else {
            return;
        };
        let site = site.as_ref().map_or(std::ptr::null(), |site| site.as_ptr());
        if let Ok(text) = CString::new(text) {
            unsafe { bc::bc_window_suggest(WINDOW.get(), site, text.as_ptr(), complete) };
        }
    }

    fn remove_suggestion(&self, url: &str) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_window_remove_suggestion(WINDOW.get(), url.as_ptr()) };
        }
    }

    fn typed_url(&self, text: &str) -> Option<String> {
        url_for(text, |text, url, capacity| unsafe { bc::bc_typed_url(text, url, capacity) })
    }

    fn search_url(&self, site: Option<&str>, text: &str) -> Option<String> {
        let site = site.map(CString::new).transpose().ok()?;
        let site = site.as_ref().map_or(std::ptr::null(), |site| site.as_ptr());
        url_for(text, |text, url, capacity| unsafe { bc::bc_search_url(site, text, url, capacity) })
    }

    fn open_tab(&self, url: &str, behind: bool) {
        match behind {
            // ⌘↩'s tab loads once it's first shown, as a link opened behind does (Dia).
            true => _ = open_tab_behind(url, true),
            false => open_tab(url),
        }
    }

    fn navigate(&self, tab: u64, url: &str) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_tab_navigate(tab, url.as_ptr()) };
        }
    }

    fn select(&self, tab: u64) {
        unsafe { bc::bc_window_select(WINDOW.get(), tab) };
    }

    fn split(&self, tab: u64, with: u64, cx: &mut App) {
        split_tab(tab, with, cx);
    }

    fn open_split(&self, url: &str, with: u64, cx: &mut App) {
        open_split(url, with, cx);
    }

    fn copy(&self, text: &str, cx: &mut App) {
        shell::copy_from_bar(text, cx);
    }

    fn show(&self, centered: bool, left: f32, top: f32, width: f32) {
        let (left, top, width) = (f64::from(left), f64::from(top), f64::from(width));
        unsafe { platform::host_show_overlay(centered, left, top, width) };
    }

    fn set_height(&self, height: f32) {
        unsafe { platform::host_set_overlay_height(f64::from(height)) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_overlay() };
    }

    fn dismiss(&self) {
        unsafe { platform::host_dismiss_overlay() };
    }

    fn reshape(&self) {
        unsafe { platform::host_reshape_overlay() };
    }
}

/// The user left the command bar without choosing anything in it.
extern "C" fn command_bar_dismissed() {
    update_shell(|shell, cx| shell.command_bar_dismissed(cx));
}

/// The shell's requests, as calls to the core. The core reports back once each call has returned.
struct Engine {
    /// GPUI's view, which has the keyboard when the UI does.
    view: *mut c_void,
}

impl Browser for Engine {
    fn select(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_window_select(WINDOW.get(), tab) };
    }

    fn close(&self, tab: u64, split: bool, _: &mut App) {
        unsafe { bc::bc_tab_close(tab, split) };
    }

    fn close_below(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_close_below(tab) };
    }

    fn duplicate(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_duplicate(tab) };
    }

    fn move_to_space(&self, tab: u64, space: u64, _: &mut App) {
        unsafe { bc::bc_tab_move_to_space(tab, space) };
    }

    fn warn_moving_profiles(&self, _: &mut App) -> bool {
        unsafe { bc::bc_warn_moving_profiles() }
    }

    fn set_warn_moving_profiles(&self, warn: bool, _: &mut App) {
        unsafe { bc::bc_set_warn_moving_profiles(warn) };
    }

    fn set_pinned(&self, tab: u64, pinned: bool, _: &mut App) {
        unsafe { bc::bc_tab_set_pinned(tab, pinned) };
    }

    fn move_to_favorites(&self, tab: u64, index: usize, _: &mut App) {
        unsafe { bc::bc_tab_move_to_favorites(tab, index) };
    }

    fn move_tab(&self, tab: u64, pinned: bool, folder: u64, index: usize, _: &mut App) {
        unsafe { bc::bc_tab_move(tab, pinned, folder, index) };
    }

    // The ones after the first load once shown, as links opened behind do.
    fn open_at(
        &self,
        urls: &[String],
        pinned: bool,
        folder: u64,
        index: usize,
        _: &mut App,
    ) -> u64 {
        let tabs = urls.iter().enumerate().map(|(at, url)| (at, open_tab_behind(url, at > 0)));
        let tabs: Vec<u64> = tabs.filter(|&(_, tab)| tab != 0).map(|(_, tab)| tab).collect();
        for (at, &tab) in tabs.iter().enumerate() {
            unsafe { bc::bc_tab_move(tab, pinned, folder, index + at) };
        }
        let first = tabs.first().copied().unwrap_or(0);
        if first != 0 {
            unsafe { bc::bc_window_select(WINDOW.get(), first) };
        }
        first
    }

    fn create_folder(&self, space: u64, name: &str, folder: u64, index: usize, _: &mut App) -> u64 {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_folder_create(space, name.as_ptr(), folder, index) }
    }

    fn rename_folder(&self, folder: u64, name: &str, _: &mut App) {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_folder_rename(folder, name.as_ptr()) };
    }

    fn set_folder_open(&self, folder: u64, open: bool, _: &mut App) {
        unsafe { bc::bc_folder_set_open(folder, open) };
    }

    fn move_folder(&self, folder: u64, to: u64, index: usize, _: &mut App) {
        unsafe { bc::bc_folder_move(folder, to, index) };
    }

    fn delete_folder(&self, folder: u64, _: &mut App) {
        unsafe { bc::bc_folder_delete(folder) };
    }

    fn separate(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_separate(tab) };
    }

    fn move_in_split(&self, tab: u64, index: usize, _: &mut App) {
        unsafe { bc::bc_tab_move_in_split(tab, index) };
    }

    fn set_split_stacked(&self, tab: u64, stacked: bool, _: &mut App) {
        unsafe { bc::bc_tab_set_split_stacked(tab, stacked) };
    }

    fn set_split_shares(&self, tab: u64, shares: Option<&[f64]>, _: &mut App) {
        let (shares, count) = shares.map_or((std::ptr::null(), 0), |shares| {
            (shares.as_ptr(), shares.len())
        });
        unsafe { bc::bc_tab_set_split_shares(tab, shares, count) };
    }

    fn split_with(&self, tab: u64, with: u64, left: bool, _: &mut App) {
        unsafe {
            bc::bc_tab_split_with(tab, with, left);
            bc::bc_window_select(WINDOW.get(), tab);
        }
    }

    fn show_drop_targets(&self, shown: bool, cx: &mut App) {
        // After the shell's draw, which asks for it.
        cx.defer(move |cx| drop_targets::show(shown, cx));
    }

    fn back_to_pinned_url(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_back_to_pinned_url(tab) };
    }

    fn set_muted(&self, tab: u64, muted: bool, _: &mut App) {
        unsafe { bc::bc_tab_set_muted(tab, muted) };
    }

    fn clear(&self, _: &mut App) {
        unsafe { bc::bc_window_clear(WINDOW.get()) };
    }

    fn undo_clear(&self, _: &mut App) -> usize {
        unsafe { bc::bc_window_undo_clear(WINDOW.get()) }
    }

    fn undo_delete_space(&self, _: &mut App) -> bool {
        unsafe { bc::bc_window_undo_delete_space(WINDOW.get()) != 0 }
    }

    fn reopen_closed_tab(&self, _: &mut App) {
        unsafe { bc::bc_window_reopen_closed_tab(WINDOW.get()) };
    }

    fn restore_archived(&self, archived: u64, _: &mut App) {
        unsafe { bc::bc_window_restore_archived(WINDOW.get(), archived) };
    }

    fn act_on_download(&self, download: u64, action: DownloadAction, _: &mut App) {
        let action = match action {
            DownloadAction::Open => bc::BC_DOWNLOAD_OPEN,
            DownloadAction::Show => bc::BC_DOWNLOAD_SHOW,
            DownloadAction::Pause => bc::BC_DOWNLOAD_PAUSE,
            DownloadAction::Resume => bc::BC_DOWNLOAD_RESUME,
            DownloadAction::Cancel => bc::BC_DOWNLOAD_CANCEL,
            DownloadAction::Remove => bc::BC_DOWNLOAD_REMOVE,
            DownloadAction::Keep => bc::BC_DOWNLOAD_KEEP,
        };
        unsafe { bc::bc_download_act(download, action as i32) };
    }

    fn clear_downloads(&self, _: &mut App) {
        unsafe { bc::bc_downloads_clear() };
    }

    fn download_icon(&self, download: u64, _: &mut App) {
        unsafe { bc::bc_download_icon(download) };
    }

    fn remove_archived(&self, archived: u64, _: &mut App) {
        unsafe { bc::bc_archive_remove(archived) };
    }

    fn clear_archive(&self, _: &mut App) {
        unsafe { bc::bc_archive_clear() };
    }

    fn page_favicon(&self, url: &str, _: &mut App) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_page_favicon(url.as_ptr()) };
        }
    }

    fn day_starts(&self) -> [f64; 4] {
        let mut starts = [0.; 4];
        unsafe { platform::host_day_starts(starts.as_mut_ptr()) };
        starts
    }

    fn go_back(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_go_back(tab) };
    }

    fn go_forward(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_go_forward(tab) };
    }

    fn reload(&self, tab: u64, bypass_cache: bool, _: &mut App) {
        unsafe { bc::bc_tab_reload(tab, bypass_cache) };
    }

    fn clear_site(&self, tab: u64, cookies: bool, _: &mut App) {
        unsafe { bc::bc_tab_clear_site(tab, cookies) };
    }

    fn answer_permission(&self, tab: u64, answer: shell::Answer, _: &mut App) {
        let answer = match answer {
            shell::Answer::Allow => bc::BC_PERMISSION_ALLOW,
            shell::Answer::Block => bc::BC_PERMISSION_BLOCK,
            shell::Answer::Dismiss => bc::BC_PERMISSION_DISMISS,
        };
        unsafe { bc::bc_tab_answer_permission(tab, answer as i32) };
    }

    fn stop(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_stop(tab) };
    }

    fn zoom(&self, tab: u64, step: i32, cx: &mut App) {
        if tab == 0 {
            return;
        }
        let percent = unsafe { bc::bc_tab_zoom(tab, step) };
        // check.sh reads this line.
        log!("browser: zoom {tab} {percent}%");
        zoom_hud::show(tab, percent, cx);
    }

    fn view_source(&self, tab: u64, _: &mut App) {
        if tab != 0 {
            unsafe { bc::bc_tab_view_source(tab) };
        }
    }

    fn save_page(&self, tab: u64, _: &mut App) {
        if tab != 0 {
            unsafe { bc::bc_tab_save_page(tab) };
        }
    }

    fn print(&self, tab: u64, _: &mut App) {
        if tab != 0 {
            unsafe { bc::bc_tab_print(tab) };
        }
    }

    fn devtools(&self, tab: u64, what: i32, _: &mut App) {
        if tab != 0 {
            unsafe { bc::bc_tab_devtools(tab, what) };
        }
    }

    fn task_manager(&self, _: &mut App) {
        unsafe { bc::bc_window_task_manager(WINDOW.get()) };
    }

    fn set_menus(&self, spaces: &[SharedString], extensions: &[Extension], cx: &mut App) {
        cx.set_menus(menus(spaces, extensions));
    }

    fn run_extension(&self, id: &str, anchor: Bounds<Pixels>, _: &mut App) -> bool {
        let Ok(id) = CString::new(id) else {
            return false;
        };
        unsafe { bc::bc_window_run_extension(WINDOW.get(), id.as_ptr(), rect(anchor)) }
    }

    fn run_extension_command(&self, id: &str, name: &str, _: &mut App) -> bool {
        let (Ok(id), Ok(name)) = (CString::new(id), CString::new(name)) else {
            return false;
        };
        unsafe { bc::bc_window_run_extension_command(WINDOW.get(), id.as_ptr(), name.as_ptr()) }
    }

    fn set_extension_pinned(&self, profile: u64, id: &str, pinned: bool, _: &mut App) {
        if let Ok(id) = CString::new(id) {
            unsafe { bc::bc_extension_set_pinned(profile, id.as_ptr(), pinned) };
        }
    }

    fn remove_extension(&self, profile: u64, id: &str, _: &mut App) -> bool {
        let Ok(id) = CString::new(id) else {
            return false;
        };
        unsafe { bc::bc_extension_remove(profile, id.as_ptr()) }
    }

    fn open_extension_options(&self, url: &str, _: &mut App) {
        open_tab(url);
    }

    fn manage_extensions(&self, id: Option<&str>, _: &mut App) {
        match id {
            Some(id) => open_tab(&format!("chrome://extensions/?id={id}")),
            None => open_tab("chrome://extensions"),
        }
    }

    fn view_history(&self, _: &mut App) {
        open_tab("chrome://history");
    }

    fn close_other_window(&self, _: &mut App) -> bool {
        unsafe { platform::host_close_other_window() }
    }

    fn show_space(&self, space: u64, _: &mut App) {
        unsafe { bc::bc_window_show_space(WINDOW.get(), space) };
    }

    fn space_tab(&self, space: u64) -> u64 {
        unsafe { bc::bc_window_space_tab(WINDOW.get(), space) }
    }

    fn create_space(&self, name: &str, profile: u64, _: &mut App) -> u64 {
        let space = create_space(name);
        if profile != 0 {
            unsafe { bc::bc_space_set_profile(space, profile) };
        }
        space
    }

    fn rename_space(&self, space: u64, name: &str, _: &mut App) {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_space_set_name(space, name.as_ptr()) };
    }

    fn rename_tab(&self, tab: u64, name: &str, _: &mut App) {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_tab_rename(tab, name.as_ptr()) };
    }

    fn set_icon(&self, tab: u64, icon: &str, _: &mut App) {
        let icon = CString::new(icon).unwrap_or_default();
        unsafe { bc::bc_tab_set_icon(tab, icon.as_ptr()) };
    }

    fn delete_space(&self, space: u64, _: &mut App) -> bool {
        unsafe { bc::bc_space_delete(space) }
    }

    fn set_space_color(&self, space: u64, color: Option<u32>, _: &mut App) {
        unsafe { bc::bc_space_set_color(space, color.is_some(), color.unwrap_or(0)) };
    }

    fn set_space_profile(&self, space: u64, profile: u64, _: &mut App) {
        unsafe { bc::bc_space_set_profile(space, profile) };
    }

    fn create_profile(&self, name: &str, _: &mut App) -> u64 {
        let name = CString::new(name).unwrap_or_default();
        unsafe { bc::bc_profile_create(name.as_ptr()) }
    }

    fn delete_profile(&self, profile: u64, _: &mut App) {
        unsafe { bc::bc_profile_delete(profile) };
    }

    fn menu(
        &self,
        items: Vec<MenuEntry>,
        position: Point<Pixels>,
        over: Option<usize>,
        in_reveal: bool,
        chosen: Box<dyn FnOnce(Option<usize>, &mut App)>,
        cx: &mut App,
    ) {
        // AppKit runs a menu in an event loop of its own until it closes, which GPUI's updates
        // mustn't be inside.
        cx.spawn(async move |cx| {
            let place = if in_reveal { platform::IN_REVEAL } else { platform::IN_WINDOW };
            let index = show_menu(&items, position, place, over);
            cx.update(|cx| chosen(index, cx));
        })
        .detach();
    }

    fn confirm(
        &self,
        message: String,
        detail: String,
        action: &'static str,
        confirmed: Box<dyn FnOnce(&mut App)>,
        cx: &mut App,
    ) {
        dialog::ask(message, detail, action, confirmed, cx);
    }

    fn focus_ui(&self, _: &mut App) {
        unsafe { platform::host_focus(self.view) };
    }

    fn lay_out_page(&self, frame: Bounds<Pixels>, radius: f32, _: &mut App) {
        set_page_frame((frame, radius));
    }

    fn set_focus_color(&self, color: u32, _: &mut App) {
        unsafe { bc::bc_window_set_focus_color(WINDOW.get(), color) };
    }

    fn set_page_color(&self, color: u32, _: &mut App) {
        unsafe { bc::bc_window_set_page_color(WINDOW.get(), color) };
    }

    fn set_pane_corner(&self, right: f32, top: f32, _: &mut App) {
        unsafe { platform::host_set_pane_corner(right as f64, top as f64) };
    }

    fn show_window_buttons(&self, shown: bool, _: &mut App) {
        unsafe { platform::host_show_window_buttons(shown) };
    }

    fn press_window_button(&self, button: WindowButton, _: &mut App) {
        let button = match button {
            WindowButton::Close => 0,
            WindowButton::Minimize => 1,
            WindowButton::Zoom => 2,
        };
        unsafe { platform::host_press_window_button(button) };
    }

    fn show_reveal(&self, width: f32, _: &mut App) {
        unsafe { platform::host_show_reveal(width.into()) };
    }

    fn hide_reveal(&self, _: &mut App) {
        unsafe { platform::host_hide_reveal() };
    }

    fn saved_sidebar_width(&self) -> Option<f32> {
        let width = unsafe { bc::bc_window_sidebar_width(WINDOW.get()) };
        (width > 0.).then_some(width as f32)
    }

    fn set_sidebar_width(&self, width: f32, _: &mut App) {
        unsafe { bc::bc_window_set_sidebar_width(WINDOW.get(), width.into()) };
    }

    fn detent(&self, _: &mut App) {
        if unsafe { bc::bc_haptics() } {
            unsafe { platform::host_detent() };
        }
    }

    fn accent_color(&self) -> u32 {
        unsafe { platform::host_accent_color() }
    }

    fn watch_control(&self, _: &mut App) {
        unsafe { platform::host_watch_control(control_released) };
    }

    fn copy(&self, text: &str, _: &mut App) {
        let text = CString::new(text.replace('\0', "")).unwrap_or_default();
        unsafe { platform::host_copy(text.as_ptr()) };
    }

    fn toast(&self, text: &str, key: Option<(&str, &str)>, time: Duration, cx: &mut App) {
        toast::show(text, key, time, cx);
    }

    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        field: Option<Bounds<Pixels>>,
        cx: &mut App,
    ) {
        command_bar::show(editing, field, cx);
    }

    fn split_bar(&self, with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App) {
        command_bar::show_split(with, pane, card, cx);
    }

    fn find(&self, tab: u64, cx: &mut App) {
        find_bar::show(tab, cx);
    }

    fn find_next(&self, tab: u64, forward: bool, cx: &mut App) {
        find_bar::find_next(tab, forward, cx);
    }

    fn jump_to_selection(&self, tab: u64, _: &mut App) {
        if tab != 0 {
            unsafe { bc::bc_tab_jump_to_selection(tab) };
        }
    }

    fn settings(&self, cx: &mut App) {
        settings::show(cx);
    }

    fn shortcuts(&self, cx: &mut App) {
        settings::show_shortcuts(cx);
    }

    /// Asks first, as Arc does, unless the user said not to. Only the user's Quit asks: a quit from
    /// the Dock or at logout doesn't come here.
    fn quit(&self, cx: &mut App) {
        if !unsafe { bc::bc_warn_before_quitting() } {
            unsafe { bc::bc_quit() };
            return;
        }
        cx.spawn(async move |_| {
            let buttons = ["Quit", "Quit, and don’t ask again", "Cancel"];
            match ask("Quit Browser?", "", &buttons) {
                0 => {}
                1 => {
                    // check.sh waits for this line.
                    log!("browser: warn before quitting off");
                    unsafe { bc::bc_set_warn_before_quitting(false) };
                }
                _ => return,
            }
            unsafe { bc::bc_quit() };
        })
        .detach();
    }
}

/// Asks `message` in an alert with `buttons`, the first the default, and returns the index of the
/// one chosen, or none if AppKit ended it. It blocks in AppKit's loop until the alert closes, so
/// it's called in a task.
fn ask(message: &str, detail: &str, buttons: &[&str]) -> usize {
    let text = |text: &str| CString::new(text).unwrap_or_default();
    let (message, detail) = (text(message), text(detail));
    let buttons: Vec<CString> = buttons.iter().map(|button| text(button)).collect();
    let pointers: Vec<*const c_char> = buttons.iter().map(|button| button.as_ptr()).collect();
    let chosen = unsafe {
        platform::host_ask(
            message.as_ptr(),
            detail.as_ptr(),
            pointers.as_ptr(),
            pointers.len(),
        )
    };
    usize::try_from(chosen).unwrap_or(usize::MAX)
}

/// A key the page didn't keep, or one it never sees, as a keystroke in GPUI's terms, with the
/// modifiers in the order shell::SHORTCUTS writes them.
fn keystroke(key: &bc::bc_key) -> String {
    let name = unsafe { CStr::from_ptr(key.key) }.to_string_lossy();
    let mut keystroke = String::new();
    for (flag, modifier) in [
        (bc::BC_MODIFIER_CONTROL, "ctrl-"),
        (bc::BC_MODIFIER_ALT, "alt-"),
        (bc::BC_MODIFIER_SHIFT, "shift-"),
        (bc::BC_MODIFIER_META, "cmd-"),
    ] {
        if key.modifiers & flag != 0 {
            keystroke.push_str(modifier);
        }
    }
    let key = match name.as_ref() {
        "ArrowDown" => "down".to_string(),
        "ArrowUp" => "up".to_string(),
        "ArrowLeft" => "left".to_string(),
        "ArrowRight" => "right".to_string(),
        " " => "space".to_string(),
        name => name.to_lowercase(),
    };
    keystroke + &key
}

/// Runs the shortcut `key` is, if it's one of the page shortcuts that are `reserved` or aren't.
fn run_page_shortcut(key: *const bc::bc_key, reserved: bool) -> bool {
    let key = unsafe { &*key };
    let Some(command) = Command::for_keystroke(&keystroke(key), reserved) else {
        return false;
    };
    // ⌘Z in a text field is the page's, which the Edit menu gives it.
    if command == Command::Undo && key.editing {
        return false;
    }
    let mut ran = false;
    update_shell(|shell, cx| ran = shell.run_for_page(command, cx));
    ran
}

unsafe extern "C" fn tab_key_before_page(
    _: *mut c_void,
    _: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    // As in Chrome, the reserved shortcuts come first, then the extensions' keys.
    run_page_shortcut(key, true) || {
        let keystroke = keystroke(unsafe { &*key });
        let mut ran = false;
        update_shell(|shell, cx| ran = shell.run_extension_key(&keystroke, "page", cx));
        ran
    }
}

unsafe extern "C" fn tab_key_unhandled(
    _: *mut c_void,
    _: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    run_page_shortcut(key, false)
}

/// Opens the browser window centered on the main display, with `ui_view` filling it under the
/// page.
fn open_browser_window(ui_view: *mut c_void) -> bool {
    let (width, height) = WINDOW_SIZE;
    let (mut x, mut y) = (0., 0.);
    unsafe { platform::host_centered(width, height, &mut x, &mut y) };
    let bounds = bc::bc_rect {
        x,
        y,
        width,
        height,
    };
    let window = unsafe { bc::bc_window_open(bounds, ui_view) };
    if window == 0 {
        return false;
    }
    WINDOW.set(window);
    true
}

/// Shows the Spaces as the user left them, else a first Space with the first tab. A page given in
/// BROWSER_URL opens on top either way.
fn restore_spaces() {
    let window = WINDOW.get();
    let start = Instant::now();
    let restored = unsafe { bc::bc_window_restore(window) };
    if restored {
        // check.sh reads this line.
        log!(
            "browser: restored in {:.1} ms",
            start.elapsed().as_secs_f64() * 1000.
        );
    } else {
        unsafe { bc::bc_window_show_space(window, create_space("Space 1")) };
    }
    match std::env::var("BROWSER_URL") {
        Ok(url) => open_tab(&url),
        Err(_) if !restored => open_tab(START_URL),
        Err(_) => {}
    }
}

extern "C" fn accent_color_changed() {
    update_shell(|shell, cx| shell.accent_color_changed(cx));
}

extern "C" fn control_released() {
    update_shell(|shell, cx| shell.control_released(cx));
}

extern "C" fn dragged(
    phase: c_int,
    x: f64,
    y: f64,
    items: *const platform::host_dragged,
    count: usize,
) -> bool {
    let items = match items.is_null() {
        true => &[][..],
        false => unsafe { std::slice::from_raw_parts(items, count) },
    };
    let items: Vec<_> = items
        .iter()
        .map(|item| (unsafe { CStr::from_ptr(item.url) }.to_string_lossy(), item.file))
        .collect();
    let urls = shell::dropped_urls(items.iter().map(|(url, file)| (url.as_ref(), *file)));
    let position = point(px(x as f32), px(y as f32));
    let answer = update_shell(|shell, cx| match phase {
        platform::DRAG_OVER => !urls.is_empty() && shell.drag_in(position, cx),
        platform::DRAG_DROP => shell.drop_in(position, &urls, cx),
        _ => {
            shell.drag_away(cx);
            false
        }
    });
    answer.unwrap_or(false)
}

/// Opens the browser window, the shell inside it under the page, and its Spaces.
fn open_window(cx: &mut App) {
    let ui_view = unsafe { platform::host_ui_view() };
    if !open_browser_window(ui_view) {
        return;
    }
    gpui_macos::open_next_window_in(ui_view);
    // Transparent, over the window's material.
    let options = WindowOptions {
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        let shell = cx.new(|cx| {
            // The page keeps its place in the card as the window resizes.
            cx.observe_window_bounds(window, |shell: &mut Shell, window, _| {
                set_page_frame(shell.page_frame(window.viewport_size()))
            })
            .detach();
            Shell::new(Rc::new(Engine { view: ns_view(window) }), cx)
        });
        window.focus(&shell.focus_handle(cx), cx);
        UI.set(Some((
            cx.to_async(),
            shell.downgrade(),
            Window::window_handle(window),
        )));
        // The core read the extensions before there was a shell to take them.
        unsafe { bc::bc_extensions_report() };
        shell
    });
    match window {
        Ok(window) => {
            window
                .update(cx, |shell, window, _| {
                    set_page_frame(shell.page_frame(window.viewport_size()))
                })
                .ok();
            unsafe {
                platform::host_show_window(ui_view);
                platform::host_watch_accent_color(accent_color_changed);
                platform::host_take_drops(dragged);
            }
            if let Some(bar) = command_bar::open(Rc::new(Bar), cx)
                && let Ok(view) = bar.update(cx, |_, window, _| ns_view(window))
            {
                unsafe { platform::host_add_overlay(view, command_bar_dismissed) };
            }
            if let Some(dialog) = dialog::open(Rc::new(DialogHost), cx)
                && let Ok(view) = dialog.update(cx, |_, window, _| ns_view(window))
            {
                unsafe { platform::host_add_dialog(view) };
            }
            if let Ok(shell) = window.entity(cx) {
                let fill = shell.clone();
                if let Some(toast) = toast::open(
                    Rc::new(ToastHost),
                    move |cx| fill.read(cx).window_fill(),
                    cx,
                ) && let Ok(view) = toast.update(cx, |_, window, _| ns_view(window))
                {
                    unsafe { platform::host_add_toast(view) };
                }
                find_bar::open(shell.clone(), cx);
                zoom_hud::open(shell.clone(), cx);
                drop_targets::open(shell.clone(), cx);
                reveal::open(shell, cx);
            }
            // Once the page frame is set, so the tab shown loads at its size.
            restore_spaces();
        }
        Err(error) => {
            log!("couldn't open the window: {error}");
            unsafe { bc::bc_quit() };
        }
    }
}

unsafe extern "C" fn started(_: *mut c_void) {
    let app = gpui_platform::application().with_assets(shell::Assets).run_embedded(|cx| {
        shell::bind_keys(cx);
        browser::text_input::bind_keys(cx);
        cx.set_global(FieldMenu(Rc::new(field_menu)));
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &Minimize, _| unsafe { platform::host_press_window_button(1) });
        cx.bind_keys([
            KeyBinding::new("cmd-h", Hide, Some(shell::KEY_CONTEXT)),
            KeyBinding::new("alt-cmd-h", HideOthers, Some(shell::KEY_CONTEXT)),
            KeyBinding::new("cmd-m", Minimize, Some(shell::KEY_CONTEXT)),
        ]);
        cx.set_menus(menus(&[], &[]));
        open_window(cx);
    });
    APP.set(Some(app));
}

/// The menu bar: Arc's menus, with the commands the browser has, the Spaces named `spaces` and the
/// shown Space's profile's `extensions` (docs/ux-spec.md › Menus). The Edit menu's standard items
/// go to the first responder, so a focused page cuts, copies, pastes, selects and undoes for
/// itself.
fn menus(spaces: &[SharedString], extensions: &[Extension]) -> Vec<Menu> {
    let menu = |name: &str, items| Menu { name: name.to_string().into(), items, disabled: false };
    let item = shell::menu_item;
    vec![
        menu(
            "Browser",
            vec![
                item("Settings…", Command::Settings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide Browser", Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                item("Quit Browser", Command::Quit),
            ],
        ),
        menu(
            "File",
            vec![
                item("New Tab…", Command::NewTab),
                item("Restore Last Closed Tab", Command::ReopenClosedTab),
                MenuItem::separator(),
                item("Open Command Bar", Command::EditUrl),
                MenuItem::separator(),
                item("Archive Tab", Command::CloseTab),
                MenuItem::separator(),
                item("Save Page As", Command::SavePage),
                item("Print", Command::Print),
            ],
        ),
        menu(
            "Edit",
            vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                item("Copy URL", Command::CopyUrl),
                item("Copy URL as Markdown", Command::CopyMarkdownLink),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::submenu(menu(
                    "Find",
                    vec![
                        item("Find…", Command::Find),
                        item("Find Next", Command::FindNext),
                        item("Find Previous", Command::FindPrevious),
                        item("Jump to Selection", Command::JumpToSelection),
                    ],
                )),
            ],
        ),
        menu(
            "View",
            vec![
                item("Show/Hide Sidebar", Command::ToggleSidebar),
                MenuItem::separator(),
                item("Stop Loading", Command::Stop),
                item("Refresh the Page", Command::Reload),
                item("Force Refresh the Page", Command::ForceReload),
                item("Clear Cookies and Refresh", Command::ClearCookiesAndRefresh),
                item("Clear Cache and Refresh", Command::ClearCacheAndRefresh),
                MenuItem::separator(),
                item("Add Split View", Command::AddSplit),
                item("Close this Split Pane", Command::CloseSplitPane),
                item("Separate Page from Split View", Command::SeparatePane),
                MenuItem::separator(),
                item("Zoom to Actual Size", Command::ActualSize),
                item("Zoom In", Command::ZoomIn),
                item("Zoom Out", Command::ZoomOut),
                MenuItem::separator(),
                MenuItem::submenu(menu(
                    "Developer",
                    vec![
                        item("View Source", Command::ViewSource),
                        item("Developer Tools", Command::DevTools),
                        item("Inspect Elements", Command::InspectElements),
                        item("JavaScript Console", Command::JavaScriptConsole),
                    ],
                )),
            ],
        ),
        menu(
            "Spaces",
            [
                item("New Space…", Command::NewSpace),
                item("Rename Space", Command::RenameSpace),
                MenuItem::separator(),
                item("Next Space", Command::NextSpace),
                item("Previous Space", Command::PreviousSpace),
            ]
            .into_iter()
            .chain((!spaces.is_empty()).then(MenuItem::separator))
            .chain(spaces.iter().enumerate().map(|(index, name)| {
                shell::menu_item(name.clone(), Command::SelectSpace(index))
            }))
            .collect(),
        ),
        menu(
            "Tabs",
            vec![
                item("New Tab…", Command::NewTab),
                item("Pin/Unpin Tab", Command::TogglePin),
                item("New Folder…", Command::NewFolder),
                MenuItem::separator(),
                item("Next Tab", Command::NextTab),
                item("Previous Tab", Command::PreviousTab),
                MenuItem::separator(),
                item("Clear Unpinned Tabs", Command::Clear),
            ],
        ),
        menu(
            "Archive",
            vec![
                item("Go Back", Command::Back),
                item("Go Forward", Command::Forward),
                MenuItem::separator(),
                item("View History", Command::ViewHistory),
                item("View Archive", Command::ViewArchive),
                item("Clear Archive", Command::ClearArchive),
            ],
        ),
        menu(
            "Extensions",
            (extensions.iter().map(shell::extension_menu_item))
                .chain((!extensions.is_empty()).then(MenuItem::separator))
                .chain((!extensions.is_empty()).then(|| {
                    let pins = extensions.iter().map(shell::extension_pin_menu_item).collect();
                    MenuItem::submenu(menu("Pin to URL Field", pins))
                }))
                .chain([item("Manage Extensions…", Command::ManageExtensions)])
                .collect(),
        ),
        menu(
            "Window",
            vec![
                MenuItem::action("Minimize Window", Minimize),
                MenuItem::separator(),
                item("View Library", Command::ViewLibrary),
                item("View Downloads", Command::ViewDownloads),
            ],
        ),
        menu(
            "Help",
            vec![
                item("Essential Keyboard Shortcuts", Command::KeyboardShortcuts),
                MenuItem::separator(),
                MenuItem::submenu(menu(
                    "Troubleshooting",
                    vec![item("Open Task Manager", Command::TaskManager)],
                )),
            ],
        ),
    ]
}

// There's one window, so closing it quits, as its close button does in Chrome's last window.
unsafe extern "C" fn window_closed(_: *mut c_void, _: bc::bc_id) {
    log!("browser: window closed");
    unsafe { bc::bc_quit() };
}

unsafe extern "C" fn stopping(_: *mut c_void) {
    log!("browser: stopping");
}

/// A Space's or a folder's name, which `copy` copies as bc_space_name does.
fn name(
    copy: unsafe extern "C" fn(bc::bc_id, *mut c_char, usize) -> usize,
    id: u64,
) -> SharedString {
    let length = unsafe { copy(id, std::ptr::null_mut(), 0) };
    let mut name = vec![0u8; length + 1];
    unsafe { copy(id, name.as_mut_ptr().cast(), name.len()) };
    name.truncate(length);
    String::from_utf8_lossy(&name).into_owned().into()
}

/// The ids the core's `copy` copies.
fn ids(copy: unsafe extern "C" fn(*mut bc::bc_id, usize) -> usize) -> Vec<bc::bc_id> {
    let count = unsafe { copy(std::ptr::null_mut(), 0) };
    let mut ids = vec![0; count];
    let count = unsafe { copy(ids.as_mut_ptr(), ids.len()) };
    ids.truncate(count);
    ids
}

unsafe extern "C" fn spaces_changed(_: *mut c_void) {
    let profiles = ids(bc::bc_profiles)
        .into_iter()
        .map(|profile| (profile, name(bc::bc_profile_name, profile)))
        .collect();
    let spaces = ids(bc::bc_spaces)
        .into_iter()
        .map(|space| {
            let mut color = 0;
            let has_color = unsafe { bc::bc_space_color(space, &mut color) };
            let profile = unsafe { bc::bc_space_profile(space) };
            (space, name(bc::bc_space_name, space), has_color.then_some(color), profile)
        })
        .collect();
    update_shell(|shell, cx| {
        shell.set_profiles(profiles, cx);
        shell.set_spaces(spaces, cx);
    });
    update_app(settings::refresh);
}

unsafe extern "C" fn favorites_changed(_: *mut c_void) {
    let tabs = ids(bc::bc_favorites)
        .into_iter()
        .map(|tab| (tab, unsafe { bc::bc_tab_profile(tab) }))
        .collect();
    update_shell(|shell, cx| shell.set_favorites(tabs, cx));
    update_app(settings::refresh);
}

unsafe extern "C" fn space_changed(_: *mut c_void, space: bc::bc_id) {
    let list = |copy: unsafe extern "C" fn(bc::bc_id, *mut bc::bc_id, usize) -> usize| {
        let count = unsafe { copy(space, std::ptr::null_mut(), 0) };
        let mut tabs = vec![0; count];
        let count = unsafe { copy(space, tabs.as_mut_ptr(), tabs.len()) };
        tabs.truncate(count);
        tabs
    };
    let (pinned, today) = (list(bc::bc_space_pinned), list(bc::bc_space_today));
    let mut splits = Vec::new();
    for &tab in pinned.iter().chain(&today) {
        // The core's splits have up to three tabs.
        let mut split = [0; 3];
        let count = unsafe { bc::bc_tab_split(tab, split.as_mut_ptr(), split.len()) };
        if count > 1 && split[0] == tab {
            let tabs = split[..count.min(split.len())].to_vec();
            let shares: Vec<f64> =
                tabs.iter().map(|&tab| unsafe { bc::bc_tab_split_share(tab) }).collect();
            // Even shares are the core's integer split, which the shell draws the same way.
            let even = shares.iter().all(|&share| share == shares[0]);
            splits.push(Split {
                tabs,
                stacked: unsafe { bc::bc_tab_split_stacked(tab) },
                shares: if even { Vec::new() } else { shares },
            });
        }
    }
    let mut folders = Folders::default();
    for &row in &pinned {
        let mut open = false;
        if unsafe { bc::bc_folder(row, &mut open) } {
            folders.folders.insert(row, (name(bc::bc_folder_name, row), open));
        }
        match unsafe { bc::bc_folder_of(row) } {
            0 => {}
            folder => _ = folders.parents.insert(row, folder),
        }
    }
    update_shell(|shell, cx| shell.set_tabs(space, pinned, today, splits, folders, cx));
}

unsafe extern "C" fn window_space_shown(_: *mut c_void, window: bc::bc_id, space: bc::bc_id) {
    if window == WINDOW.get() {
        update_shell(|shell, cx| shell.set_space(space, cx));
    }
}

unsafe extern "C" fn window_selected(_: *mut c_void, window: bc::bc_id, tab: bc::bc_id) {
    if window == WINDOW.get() {
        update_shell(|shell, cx| {
            shell.set_selected(tab, cx);
            find_bar::tab_shown(tab, cx);
            zoom_hud::tab_shown(tab, cx);
        });
    }
}

// check.sh reads these lines. Test pages report what reached them through their title.
unsafe extern "C" fn tab_loaded(_: *mut c_void, tab: bc::bc_id, url: *const c_char) {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy();
    log!("browser: loaded {tab} {url}");
}

unsafe extern "C" fn tab_state_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    state: *const bc::bc_tab_state,
) {
    let state = unsafe { &*state };
    let page = Page {
        url: unsafe { CStr::from_ptr(state.url) }
            .to_string_lossy()
            .into_owned()
            .into(),
        can_go_back: state.can_go_back,
        can_go_forward: state.can_go_forward,
        loading: state.loading,
        unloaded: state.unloaded,
        away_from_pinned_url: state.away_from_pinned_url,
        audible: state.audible,
        muted: state.muted,
        icon: unsafe { CStr::from_ptr(state.icon) }.to_string_lossy().into_owned().into(),
    };
    update_shell(|shell, cx| shell.set_page(tab, page, cx));
}

unsafe extern "C" fn tab_title_changed(_: *mut c_void, tab: bc::bc_id, title: *const c_char) {
    let title = unsafe { CStr::from_ptr(title) }.to_string_lossy().into_owned();
    log!("browser: title {tab} {title}");
    update_shell(|shell, cx| shell.set_title(tab, title.into(), cx));
}

unsafe extern "C" fn tab_favicon_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    png: *const u8,
    size: usize,
) {
    let favicon = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        Arc::new(Image::from_bytes(ImageFormat::Png, png))
    });
    log!("browser: favicon {tab} {size} bytes");
    update_shell(|shell, cx| shell.set_favicon(tab, favicon, cx));
}

unsafe extern "C" fn archive_changed(
    _: *mut c_void,
    tabs: *const bc::bc_archived_tab,
    count: usize,
) {
    let tabs = match tabs.is_null() {
        true => &[][..],
        false => unsafe { std::slice::from_raw_parts(tabs, count) },
    };
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into();
    let archive = tabs
        .iter()
        .map(|tab| Archived {
            id: tab.id,
            url: text(tab.url),
            title: text(tab.title),
            archived: tab.archived,
        })
        .collect();
    // check.sh waits for this line.
    log!("browser: archive {count} tabs");
    update_shell(|shell, cx| shell.set_archive(archive, cx));
}

unsafe extern "C" fn downloads_changed(
    _: *mut c_void,
    downloads: *const bc::bc_download,
    count: usize,
) {
    let downloads = match downloads.is_null() {
        true => &[][..],
        false => unsafe { std::slice::from_raw_parts(downloads, count) },
    };
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into();
    let downloads = downloads
        .iter()
        .map(|download| Download {
            id: download.id,
            name: text(download.name),
            url: text(download.url),
            received: download.received,
            total: download.total,
            state: match download.state as u32 {
                bc::BC_DOWNLOAD_RUNNING => DownloadState::Running,
                bc::BC_DOWNLOAD_PAUSED => DownloadState::Paused,
                bc::BC_DOWNLOAD_DONE => DownloadState::Done,
                bc::BC_DOWNLOAD_CANCELLED => DownloadState::Cancelled,
                _ => DownloadState::Failed,
            },
            open_when_done: download.open_when_done,
            held: download.held,
            started: download.started,
        })
        .collect();
    update_shell(|shell, cx| shell.set_downloads(downloads, cx));
}

unsafe extern "C" fn download_icon(
    _: *mut c_void,
    download: bc::bc_id,
    png: *const u8,
    size: usize,
) {
    let icon = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        Arc::new(Image::from_bytes(ImageFormat::Png, png))
    });
    update_shell(|shell, cx| shell.set_download_icon(download, icon, cx));
}

unsafe extern "C" fn extensions_changed(
    _: *mut c_void,
    profile: bc::bc_id,
    extensions: *const bc::bc_extension,
    count: usize,
) {
    let extensions = match extensions.is_null() {
        true => &[][..],
        false => unsafe { std::slice::from_raw_parts(extensions, count) },
    };
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into();
    let extensions = extensions
        .iter()
        .map(|extension| Extension {
            id: text(extension.id),
            name: text(extension.name),
            enabled: extension.enabled,
            pinned: extension.pinned,
            options: text(extension.options),
            commands: match extension.commands.is_null() {
                true => &[][..],
                false => unsafe {
                    std::slice::from_raw_parts(extension.commands, extension.command_count)
                },
            }
            .iter()
            .map(|command| ExtensionCommand {
                name: text(command.name),
                description: text(command.description),
                keystroke: keystroke(&bc::bc_key {
                    key: command.key,
                    modifiers: command.modifiers,
                    editing: false,
                })
                .into(),
                suggested: command.suggested,
            })
            .collect(),
        })
        .collect();
    update_shell(|shell, cx| shell.set_extensions(profile, extensions, cx));
}

unsafe extern "C" fn window_extension_actions(
    _: *mut c_void,
    window: bc::bc_id,
    actions: *const bc::bc_extension_action,
    count: usize,
) {
    if window != WINDOW.get() {
        return;
    }
    let actions = match actions.is_null() {
        true => &[][..],
        false => unsafe { std::slice::from_raw_parts(actions, count) },
    };
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into();
    let actions = actions
        .iter()
        .map(|action| ExtensionAction {
            id: text(action.id),
            title: text(action.title),
            icon: (!action.icon.is_null()).then(|| {
                let png = unsafe { std::slice::from_raw_parts(action.icon, action.icon_size) };
                Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec()))
            }),
            badge: text(action.badge),
            badge_color: action.badge_color,
            enabled: action.enabled,
        })
        .collect();
    update_shell(|shell, cx| shell.set_extension_actions(actions, cx));
}

unsafe extern "C" fn window_extension_popup(
    _: *mut c_void,
    window: bc::bc_id,
    extension: *const c_char,
) {
    let extension = unsafe { CStr::from_ptr(extension) }.to_string_lossy();
    if window == WINDOW.get() {
        // check.sh waits for these lines.
        match extension.is_empty() {
            true => log!("browser: extension popup none"),
            false => log!("browser: extension popup {extension}"),
        }
    }
}

unsafe extern "C" fn window_show_downloads(_: *mut c_void, window: bc::bc_id) {
    if window == WINDOW.get() {
        update_shell(|shell, cx| shell.show_downloads(cx));
    }
}

unsafe extern "C" fn page_favicon(_: *mut c_void, url: *const c_char, png: *const u8, size: usize) {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    let favicon = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        Arc::new(Image::from_bytes(ImageFormat::Png, png))
    });
    update_shell(|shell, cx| shell.set_page_favicon(url.into(), favicon, cx));
}

unsafe extern "C" fn tab_opened(
    _: *mut c_void,
    _: bc::bc_id,
    tab: bc::bc_id,
    opener: bc::bc_id,
    foreground: bool,
) {
    let side = if foreground { "foreground" } else { "background" };
    log!("browser: tab {tab} opened by {opener} in the {side}");
}

// Called while Chromium closes the tab, which can be inside a GPUI update that asked for it, so
// the shell hears of it from space_changed instead.
unsafe extern "C" fn tab_closed(_: *mut c_void, tab: bc::bc_id) {
    log!("browser: tab {tab} closed");
    find_bar::forget(tab);
}

unsafe extern "C" fn tab_find_changed(
    _: *mut c_void,
    tab: bc::bc_id,
    state: *const bc::bc_find_state,
) {
    let state = unsafe { &*state };
    let find = find_bar::Find {
        open: state.open,
        text: unsafe { CStr::from_ptr(state.text) }
            .to_string_lossy()
            .into_owned(),
        matches: state.matches,
        selected: state.selected,
    };
    update_app(|cx| find_bar::changed(tab, find, cx));
}

unsafe extern "C" fn tab_permission_asked(
    _: *mut c_void,
    tab: bc::bc_id,
    site: *const c_char,
    permissions: *const *const c_char,
    count: usize,
) {
    let string = |text: *const c_char| {
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    };
    let permissions: Vec<String> = match count {
        0 => Vec::new(),
        _ => unsafe { std::slice::from_raw_parts(permissions, count) }
            .iter()
            .map(|permission| string(*permission))
            .collect(),
    };
    let question = match count {
        0 => {
            log!("browser: page {tab} asks nothing");
            None
        }
        _ => {
            let site = string(site);
            log!("browser: page {tab} asks {site} to {}", permissions.join("; "));
            Some(shell::Question {
                site: site.into(),
                permissions: permissions.into_iter().map(Into::into).collect(),
            })
        }
    };
    update_shell(|shell, cx| shell.set_question(tab, question, cx));
}

unsafe extern "C" fn window_suggestions(
    _: *mut c_void,
    window: bc::bc_id,
    text: *const c_char,
    suggestions: *const bc::bc_suggestion,
    count: usize,
) {
    if window != WINDOW.get() {
        return;
    }
    let string = |text: *const c_char| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned();
    let suggestions = match count {
        0 => &[][..],
        _ => unsafe { std::slice::from_raw_parts(suggestions, count) },
    };
    let rows = suggestions
        .iter()
        .map(|suggestion| command_bar::Suggestion {
            url: string(suggestion.url),
            text: string(suggestion.text).into(),
            detail: string(suggestion.detail).into(),
            tab: suggestion.tab,
            completion: string(suggestion.completion),
            removable: suggestion.removable,
            keyword: string(suggestion.keyword),
            site: string(suggestion.site).into(),
            ..command_bar::Suggestion::default()
        })
        .collect();
    let text = string(text);
    update_app(|cx| command_bar::suggested(text, rows, cx));
}

unsafe extern "C" fn tab_fullscreen_changed(_: *mut c_void, tab: bc::bc_id, fullscreen: bool) {
    let ui = UI.with_borrow(|ui| ui.as_ref().map(|(cx, shell, window)| (cx.clone(), shell.clone(), *window)));
    let Some((cx, shell, window)) = ui else {
        return;
    };
    cx.update(|cx| {
        // A page leaves full screen as its tab is switched away from or closes, and only one page
        // is full screen at a time.
        let shown = shell.read_with(cx, |shell, _| shell.selected() == tab).unwrap_or(false);
        if fullscreen && !shown {
            return;
        }
        shell.update(cx, |shell, cx| shell.set_page_fullscreen(fullscreen, cx)).ok();
        cx.update_window(window, |_, window, cx| {
            let viewport = window.viewport_size();
            if let Ok(frame) = shell.read_with(cx, |shell, _| shell.page_frame(viewport)) {
                set_page_frame(frame);
            }
        })
        .ok();
    });
    log!(
        "browser: page {} full screen",
        if fullscreen { "entered" } else { "left" }
    );
}

fn main() {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut args: Vec<String> = std::env::args().take(1).collect();
    // A profile of the dev build's own, never the owner's. A copy the owner uses is given its own
    // in BROWSER_PROFILE_DIR, so the checks, which delete what the last run saved, never reach it.
    let profile = std::env::var("BROWSER_PROFILE_DIR")
        .unwrap_or_else(|_| format!("{home}/Library/Caches/browser-app"));
    args.extend([
        format!("--user-data-dir={profile}"),
        // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed build
        // asking for it gets a password prompt, and every request that uses cookies waits.
        "--use-mock-keychain".into(),
        // Chrome would copy the app bundle to check its signature against, which an
        // ad-hoc-signed build doesn't need.
        "--disable-features=MacAppCodeSignClone".into(),
    ]);
    // Switches given to the app come after, so they override these: Chromium keeps a repeated
    // switch's last value.
    args.extend(std::env::args().skip(1));
    let args: Vec<CString> = args
        .into_iter()
        .filter_map(|arg| CString::new(arg).ok())
        .collect();
    if std::env::var_os("BROWSER_DRIVE").is_some() {
        unsafe { platform::drive_start() };
    }
    let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    let host = bc::bc_host {
        started: Some(started),
        stopping: Some(stopping),
        window_closed: Some(window_closed),
        space_changed: Some(space_changed),
        spaces_changed: Some(spaces_changed),
        favorites_changed: Some(favorites_changed),
        window_space_shown: Some(window_space_shown),
        window_selected: Some(window_selected),
        tab_opened: Some(tab_opened),
        tab_closed: Some(tab_closed),
        tab_loaded: Some(tab_loaded),
        tab_state_changed: Some(tab_state_changed),
        tab_title_changed: Some(tab_title_changed),
        tab_favicon_changed: Some(tab_favicon_changed),
        archive_changed: Some(archive_changed),
        page_favicon: Some(page_favicon),
        downloads_changed: Some(downloads_changed),
        download_icon: Some(download_icon),
        window_show_downloads: Some(window_show_downloads),
        extensions_changed: Some(extensions_changed),
        window_extension_popup: Some(window_extension_popup),
        window_extension_actions: Some(window_extension_actions),
        tab_fullscreen_changed: Some(tab_fullscreen_changed),
        tab_find_changed: Some(tab_find_changed),
        tab_permission_asked: Some(tab_permission_asked),
        window_suggestions: Some(window_suggestions),
        tab_key_before_page: Some(tab_key_before_page),
        tab_key_unhandled: Some(tab_key_unhandled),
        ..Default::default()
    };
    let code = unsafe { bc::bc_main(argv.len() as c_int, argv.as_mut_ptr(), &host) };
    std::process::exit(code);
}
