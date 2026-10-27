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

use browser::{
    capture, command_bar, dialog, icon_picker, log, site_controls, toast, tooltip, zoom_hud,
};
use browser::shell::{
    self, Archived, BarTab, Browser, Command, Download, DownloadAction, DownloadState, Extension,
    ExtensionAction, ExtensionCommand, FilePreview, Folders, MenuEntry, Page, Shell, SidePanel,
    Split, WindowButton,
};
use browser::text_input::{Chosen, Copy, Cut, FieldMenu, Paste, Redo, SelectAll, Undo};
use gpui::{
    AnyWindowHandle, App, ApplicationHandle, AsyncApp, Bounds, Context, Focusable, Image,
    ImageFormat, KeyBinding, Menu, MenuItem, OsAction, Pixels, Point, SharedString, Size,
    SystemMenuType, WeakEntity, Window, WindowBackgroundAppearance, WindowHandle, WindowOptions,
    actions, point, prelude::*, px, size,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod drop_targets;
mod find_bar;
mod reveal;
mod settings;

/// The host's overlays (platform/mac/host.h).
const COMMAND_BAR: c_int = 0;
const ICON_PICKER: c_int = 1;

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
        pub fn host_show_launch_window(profile_dir: *const c_char, width: f64, height: f64);
        pub fn host_close_launch_window();
        pub fn host_save_launch_picture(
            ui_view: *mut c_void,
            profile_dir: *const c_char,
            width: f64,
        ) -> bool;
        pub fn host_cascaded(
            ui_view: *mut c_void,
            x: *mut f64,
            y: *mut f64,
            width: *mut f64,
            height: *mut f64,
        );
        pub fn host_set_window_title(ui_view: *mut c_void, title: *const c_char);
        pub fn host_activate(ui_view: *mut c_void);
        pub fn host_watch_key_window(changed: extern "C" fn(*mut c_void));
        pub fn host_watch_full_screen(changed: extern "C" fn(*mut c_void, bool));
        pub fn host_watch_resigning_active(resigning: extern "C" fn());
        pub fn host_forget_window(ui_view: *mut c_void);
        pub fn host_add_overlay(
            overlay: c_int,
            overlay_view: *mut c_void,
            radius: f64,
            dismissed: extern "C" fn(),
        );
        pub fn host_set_overlay_shape(overlay: c_int, corners: *const f64, count: usize);
        pub fn host_set_overlay_radius(overlay: c_int, radius: f64);
        pub fn host_set_overlay_growth(overlay: c_int, x: f64, y: f64);
        pub fn host_show_overlay(
            overlay: c_int,
            ui_view: *mut c_void,
            centered: bool,
            left: f64,
            top: f64,
            width: f64,
        );
        pub fn host_set_overlay_height(overlay: c_int, height: f64);
        pub fn host_hide_overlay(overlay: c_int);
        pub fn host_can_draw_emoji(emoji: *const c_char) -> bool;
        pub fn host_dismiss_overlay(overlay: c_int);
        pub fn host_hold_keys();
        pub fn host_add_find_bar(
            ui_view: *mut c_void,
            find_bar_view: *mut c_void,
            right: f64,
            top: f64,
        );
        pub fn host_show_find_bar(ui_view: *mut c_void, keyboard: bool);
        pub fn host_hide_find_bar(ui_view: *mut c_void);
        pub fn host_add_hud(hud_view: *mut c_void, right: f64, top: f64, margin: f64);
        pub fn host_set_pane_corner(ui_view: *mut c_void, right: f64, top: f64);
        pub fn host_show_hud(ui_view: *mut c_void);
        pub fn host_hide_hud();
        pub fn host_add_drop_targets(view: *mut c_void);
        pub fn host_show_drop_targets(ui_view: *mut c_void);
        pub fn host_hide_drop_targets();
        pub fn host_focus(view: *mut c_void);
        pub fn host_show_window_buttons(ui_view: *mut c_void, shown: bool);
        pub fn host_press_window_button(ui_view: *mut c_void, button: i32);
        pub fn host_add_reveal(
            ui_view: *mut c_void,
            reveal_view: *mut c_void,
            pointer_moved: extern "C" fn(ui_view: *mut c_void, x: f64),
        );
        pub fn host_show_reveal(ui_view: *mut c_void, width: f64);
        pub fn host_hide_reveal(ui_view: *mut c_void);
        pub fn host_detent();
        pub fn host_accent_color() -> u32;
        pub fn host_watch_accent_color(changed: extern "C" fn());
        pub fn host_watch_control(released: extern "C" fn());
        pub fn host_take_drops(
            dragged: extern "C" fn(
                ui_view: *mut c_void,
                phase: c_int,
                x: f64,
                y: f64,
                items: *const host_dragged,
                count: usize,
            ) -> bool,
        );
        pub fn host_trash(path: *const c_char) -> bool;
        pub fn host_drag_file(path: *const c_char, ended: extern "C" fn(taken: bool)) -> bool;
        pub fn host_file_preview(
            path: *const c_char,
            id: u64,
            done: extern "C" fn(
                id: u64,
                png: *const u8,
                size: usize,
                width: f64,
                height: f64,
                image: bool,
            ),
        );
        pub fn host_copy(text: *const c_char);
        pub fn host_open_in_safari(url: *const c_char);
        pub fn host_share(
            ui_view: *mut c_void,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            url: *const c_char,
            title: *const c_char,
        );
        pub fn host_day_starts(starts: *mut f64);
        pub fn host_menu(
            ui_view: *mut c_void,
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
        pub fn host_show_dialog(ui_view: *mut c_void);
        pub fn host_hide_dialog();
        pub fn host_add_site_controls(view: *mut c_void, dismissed: extern "C" fn());
        pub fn host_show_site_controls(
            ui_view: *mut c_void,
            left: f64,
            top: f64,
            width: f64,
            height: f64,
        );
        pub fn host_set_site_controls_shape(corners: *const f64, count: usize);
        pub fn host_set_site_controls_height(height: f64);
        pub fn host_site_controls_drew(scale: f64, opacity: f64, x: f64, y: f64);
        pub fn host_release_site_controls();
        pub fn host_hide_site_controls();
        pub fn host_add_capture(view: *mut c_void, dismissed: extern "C" fn());
        pub fn host_show_capture(
            ui_view: *mut c_void,
            left: f64,
            top: f64,
            width: f64,
            height: f64,
        );
        pub fn host_capture_camera(camera: bool);
        pub fn host_hide_capture();
        pub fn host_copy_png(png: *const u8, size: usize);
        pub fn host_save_png(ui_view: *mut c_void, png: *const u8, size: usize);
        pub fn host_share_png(
            png: *const u8,
            size: usize,
            view: *mut c_void,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            messages: bool,
        );
        pub fn host_add_toast(toast_view: *mut c_void);
        pub fn host_toast_takes_clicks(takes: bool);
        pub fn host_show_toast(ui_view: *mut c_void);
        pub fn host_hide_toast();
        pub fn host_add_tooltip(tooltip_view: *mut c_void, away: extern "C" fn());
        pub fn host_show_tooltip(ui_view: *mut c_void, left: f64, top: f64, fade: f64);
        pub fn host_hide_tooltip(fade: f64);
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
    let mut key = keystroke;
    let mut held = [false; 4];
    for (modifier, at) in [("ctrl-", 3), ("alt-", 2), ("shift-", 1), ("cmd-", 0)] {
        if let Some(rest) = key.strip_prefix(modifier) {
            held[at] = true;
            key = rest;
        }
    }
    // NSF1FunctionKey on.
    let function = key.strip_prefix('f').and_then(|number| number.parse::<u32>().ok());
    if let Some(number) = function.filter(|number| (1..=35).contains(number)) {
        return (char::from_u32(0xf703 + number).unwrap_or_default().to_string(), held);
    }
    let key = match key {
        "enter" => "\r".to_string(),
        "escape" => "\u{1b}".to_string(),
        "up" => "\u{f700}".to_string(),
        "down" => "\u{f701}".to_string(),
        "left" => "\u{f702}".to_string(),
        "right" => "\u{f703}".to_string(),
        "tab" => "\t".to_string(),
        "space" => " ".to_string(),
        "backspace" => "\u{8}".to_string(),
        "delete" => "\u{f728}".to_string(),
        "pageup" => "\u{f72c}".to_string(),
        "pagedown" => "\u{f72d}".to_string(),
        "home" => "\u{f729}".to_string(),
        "end" => "\u{f72b}".to_string(),
        key => key.to_string(),
    };
    (key, held)
}
const START_URL: &str = "https://en.wikipedia.org/wiki/Arc_(web_browser)";
/// Where Add Extension… goes, as Dia's does.
const WEB_STORE: &str = "https://chromewebstore.google.com/";

actions!(browser, [Hide, HideOthers, ShowAll, Minimize]);

/// A browser window: the core's, with the view the UI draws in and the shell drawn there.
#[derive(Clone)]
struct BrowserWindow {
    id: bc::bc_id,
    ui_view: *mut c_void,
    shell: WeakEntity<Shell>,
    handle: AnyWindowHandle,
    /// Its own overlays' windows, such as the find bar's, which close with it.
    children: Vec<AnyWindowHandle>,
    /// A private window's: it shows a Space of its own, in an off-the-record session.
    private: bool,
}

thread_local! {
    // The core calls back into the UI from Chromium's event handling, outside any GPUI update.
    static CX: RefCell<Option<AsyncApp>> = const { RefCell::new(None) };
    // GPUI starts inside Chromium's run loop, from a callback, and lives as long as the process.
    static APP: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
    // The most recently key first, as the core keeps its windows.
    static WINDOWS: RefCell<Vec<BrowserWindow>> = const { RefCell::new(Vec::new()) };
}

/// The core's id and the UI's view of the window that the overlays every window shares show over,
/// such as the command bar: the browser window last key. 0 and null before one opens.
fn front() -> (bc::bc_id, *mut c_void) {
    WINDOWS.with_borrow(|windows| {
        windows.first().map_or((0, std::ptr::null_mut()), |window| (window.id, window.ui_view))
    })
}

/// A browser window has gone full screen or left it, once macOS's transition is over.
extern "C" fn full_screen_changed(ui_view: *mut c_void, entered: bool) {
    update_window(window_with_view(ui_view), |shell, cx| shell.set_full_screen(entered, cx));
}

/// The window whose UI draws in `ui_view`, or 0.
fn window_with_view(ui_view: *mut c_void) -> bc::bc_id {
    WINDOWS.with_borrow(|windows| {
        windows.iter().find(|window| window.ui_view == ui_view).map_or(0, |window| window.id)
    })
}

/// The shell drawn in `ui_view`, for an overlay to read the look of the window it shows over.
pub(crate) fn shell_with_view(ui_view: *mut c_void) -> WeakEntity<Shell> {
    WINDOWS.with_borrow(|windows| {
        windows
            .iter()
            .find(|window| window.ui_view == ui_view)
            .map_or_else(WeakEntity::new_invalid, |window| window.shell.clone())
    })
}

/// A browser window became key: it goes first, so the shared overlays and the menu bar follow it.
extern "C" fn key_window_changed(ui_view: *mut c_void) {
    let moved = WINDOWS.with_borrow_mut(|windows| {
        let index = windows.iter().position(|window| window.ui_view == ui_view)?;
        (index > 0).then(|| {
            let window = windows.remove(index);
            windows.insert(0, window.clone());
            window.id
        })
    });
    if let Some(id) = moved {
        log!("browser: key window {id}");
        update_window(id, |shell, cx| shell.became_key(cx));
    }
}

/// Changes every window's shell, from a core callback about what they all show, such as the
/// Spaces and their tabs.
fn update_shells(change: impl Fn(&mut Shell, &mut Context<Shell>)) {
    let Some(cx) = CX.with_borrow(Clone::clone) else {
        return;
    };
    let shells: Vec<_> =
        WINDOWS.with_borrow(|windows| windows.iter().map(|window| window.shell.clone()).collect());
    cx.update(|cx| {
        for shell in shells {
            shell.update(cx, &change).ok();
        }
    });
}

/// Changes the shell of the window `id`, from a core callback about that window, and returns what
/// the change does, unless there's no such window.
fn update_window<R>(
    id: bc::bc_id,
    change: impl FnOnce(&mut Shell, &mut Context<Shell>) -> R,
) -> Option<R> {
    let shell = WINDOWS.with_borrow(|windows| {
        windows.iter().find(|window| window.id == id).map(|window| window.shell.clone())
    })?;
    let cx = CX.with_borrow(Clone::clone)?;
    cx.update(|cx| shell.update(cx, change).ok())
}

/// The window showing `tab`'s page, as the core says, else the one whose shell thinks it does,
/// else the one last key, such as for a key from a page no shell has heard is shown yet.
fn tab_window(tab: u64, cx: &App) -> Option<BrowserWindow> {
    let core = unsafe { bc::bc_tab_window(tab) };
    WINDOWS.with_borrow(|windows| {
        let shows = |window: &&BrowserWindow| {
            window.shell.upgrade().is_some_and(|shell| shell.read(cx).shows(tab))
        };
        windows
            .iter()
            .find(|window| window.id == core)
            .or_else(|| windows.iter().find(shows))
            .or(windows.first())
            .cloned()
    })
}

/// Changes the shell of the window showing `tab`'s page (`tab_window`), from a core callback about
/// the page, and returns what the change does.
fn update_tab_window<R>(
    tab: u64,
    change: impl FnOnce(&mut Shell, &mut Context<Shell>) -> R,
) -> Option<R> {
    let cx = CX.with_borrow(Clone::clone)?;
    cx.update(|cx| tab_window(tab, cx)?.shell.update(cx, change).ok())
}

/// Changes the UI outside the shells, from a core callback.
fn update_app(change: impl FnOnce(&mut App)) {
    if let Some(cx) = CX.with_borrow(Clone::clone) {
        cx.update(change);
    }
}

/// Changes the app once the update under way, if any, is over, for a core callback that can come
/// from inside the call that asked for it.
fn update_app_later(change: impl FnOnce(&mut App) + 'static) {
    if let Some(cx) = CX.with_borrow(Clone::clone) {
        let executor = cx.foreground_executor().clone();
        executor.spawn(async move { cx.update(change) }).detach();
    }
}

/// The window's NSView, for the host.
fn ns_view(window: &Window) -> *mut c_void {
    match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::AppKit(handle)) => handle.ns_view.as_ptr(),
        _ => panic!("the window has no AppKit view"),
    }
}

/// Shows a menu of `items` at `position` in `place` (`platform::IN_*`) of `ui_view`'s window, or
/// with the item at `over` there, and returns the index of the one chosen once it closes. AppKit
/// runs the menu in an event loop of its own, so this mustn't be called inside a GPUI update.
fn show_menu(
    ui_view: *mut c_void,
    items: &[MenuEntry],
    position: Point<Pixels>,
    place: i32,
    over: Option<usize>,
) -> Option<usize> {
    let (x, y) = (f64::from(position.x), f64::from(position.y));
    let over = over.map_or(-1, |over| over as i32);
    pop_up(items, |items, count| unsafe {
        platform::host_menu(ui_view, items, count, x, y, place, over)
    })
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
            let (key, modifiers) = item.shortcut.as_deref().map(key_equivalent).unwrap_or_default();
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

/// Tells the core where the window's page goes and the radius of its corners
/// (`Shell::page_frame`).
fn set_page_frame(window: bc::bc_id, (frame, radius): (Bounds<Pixels>, f32)) {
    unsafe { bc::bc_window_set_page_frame(window, rect(frame), f64::from(radius)) };
}

/// Tells the core where the window's page, at `page`, and its side panel's page go, in a window
/// whose content is `viewport`, as it resizes or a page goes full screen between the shell's draws.
fn set_frames(
    window: bc::bc_id,
    shell: &Shell,
    page: (Bounds<Pixels>, f32),
    viewport: Size<Pixels>,
) {
    set_page_frame(window, page);
    let panel = shell.side_panel_page(viewport).unwrap_or_default();
    unsafe { bc::bc_window_set_side_panel_frame(window, rect(panel)) };
}

fn rect(bounds: Bounds<Pixels>) -> bc::bc_rect {
    bc::bc_rect {
        x: f64::from(bounds.origin.x),
        y: f64::from(bounds.origin.y),
        width: f64::from(bounds.size.width),
        height: f64::from(bounds.size.height),
    }
}

/// Opens a tab loading `url` at the top of the Today tabs of the Space `window` shows, and shows
/// it there.
fn open_tab(window: bc::bc_id, url: &str) {
    let tab = open_tab_behind(window, url, false);
    if tab != 0 {
        unsafe { bc::bc_window_select(window, tab) };
    }
}

/// Opens a tab loading `url` at the top of the Today tabs of the Space `window` shows, behind the
/// one shown, or with `when_shown` loading it only once it's first shown. Returns it, or 0.
fn open_tab_behind(window: bc::bc_id, url: &str, when_shown: bool) -> u64 {
    match CString::new(url) {
        Ok(url) => unsafe { bc::bc_tab_open(window, url.as_ptr(), when_shown) },
        Err(_) => 0,
    }
}

/// Puts `tab` in a split view with `with`, on its right, and shows it in `window`. The pane being
/// added for it, if there is one, gives the split its room first, so it shows at once at its place.
fn split_tab(window: bc::bc_id, tab: u64, with: u64, cx: &mut App) {
    shell::pane_added(cx);
    unsafe {
        bc::bc_tab_split_with(tab, with, false);
        bc::bc_window_select(window, tab);
    }
}

/// Opens a tab loading `url` in a split view with `with`, on its right, and shows it in `window`.
fn open_split(window: bc::bc_id, url: &str, with: u64, cx: &mut App) {
    let Ok(url) = CString::new(url) else {
        return;
    };
    let tab = unsafe { bc::bc_tab_open(window, url.as_ptr(), false) };
    if tab != 0 {
        split_tab(window, tab, with, cx);
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
        unsafe { platform::host_show_dialog(front().1) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_dialog() };
    }
}

/// The Site Controls' requests, as calls to the core and to the host's window for them, over the
/// window in front (`front`).
struct SiteControlsHost {
    /// The panel's view, for its menus, once its window is open.
    view: Cell<*mut c_void>,
}

/// The panel's shape, for the host's blur and shadow, as x, y and radius each.
fn set_site_controls_shape(corners: &[(f32, f32, f32)]) {
    let flat: Vec<f64> = (corners.iter())
        .flat_map(|&(x, y, radius)| [f64::from(x), f64::from(y), f64::from(radius)])
        .collect();
    unsafe { platform::host_set_site_controls_shape(flat.as_ptr(), corners.len()) };
}

impl site_controls::Host for SiteControlsHost {
    fn show(&self, at: Point<Pixels>, size: gpui::Size<Pixels>, corners: &[(f32, f32, f32)]) {
        set_site_controls_shape(corners);
        let (left, top) = (f64::from(at.x), f64::from(at.y));
        let (width, height) = (f64::from(size.width), f64::from(size.height));
        unsafe { platform::host_show_site_controls(front().1, left, top, width, height) };
    }

    fn set_height(&self, height: f32, corners: &[(f32, f32, f32)]) {
        set_site_controls_shape(corners);
        unsafe { platform::host_set_site_controls_height(f64::from(height)) };
    }

    fn drew(&self, scale: f32, opacity: f32, origin: Point<Pixels>) {
        let (x, y) = (f64::from(origin.x), f64::from(origin.y));
        unsafe { platform::host_site_controls_drew(scale.into(), opacity.into(), x, y) };
    }

    fn release(&self) {
        unsafe { platform::host_release_site_controls() };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_site_controls() };
    }

    fn read(&self, tab: u64) {
        unsafe { bc::bc_tab_site_controls(tab) };
    }

    fn stop_reading(&self) {
        unsafe { bc::bc_site_controls_close() };
    }

    fn set_permission(&self, tab: u64, kind: i32, value: site_controls::Value) {
        let value = match value {
            site_controls::Value::Allow => bc::BC_SITE_ALLOW,
            site_controls::Value::Block => bc::BC_SITE_BLOCK,
            site_controls::Value::Ask => bc::BC_SITE_ASK,
        };
        unsafe { bc::bc_tab_set_site_permission(tab, kind, value as i32) };
    }

    fn reset_permissions(&self, tab: u64) {
        unsafe { bc::bc_tab_reset_site_permissions(tab) };
    }

    fn run(&self, run: site_controls::Run, tab: u64, cx: &mut App) {
        let Some(window) = tab_window(tab, cx) else {
            return;
        };
        match run {
            // The shell's commands, which the keys and the command bar run too.
            site_controls::Run::Command(command) => {
                window.shell.update(cx, |shell, cx| shell.run(command, "site-controls", cx)).ok();
            }
            site_controls::Run::Share(frame) => {
                let Some((url, title)) = window.shell.upgrade().map(|shell| {
                    let shell = shell.read(cx);
                    (shell.page(tab).url, shell.title(tab))
                }) else {
                    return;
                };
                let (url, title) = (CString::new(url.as_ref()), CString::new(title.as_ref()));
                let (Ok(url), Ok(title)) = (url, title) else {
                    return;
                };
                let frame = rect(frame);
                let (x, y, width, height) = (frame.x, frame.y, frame.width, frame.height);
                let (url, title) = (url.as_ptr(), title.as_ptr());
                unsafe { platform::host_share(window.ui_view, x, y, width, height, url, title) };
            }
            site_controls::Run::SiteSettings => {
                let url = window.shell.upgrade().map(|shell| shell.read(cx).page(tab).url);
                let settings = url.and_then(|url| site_controls::site_settings_url(&url));
                open_tab(window.id, settings.as_deref().unwrap_or("chrome://settings/content"));
            }
            site_controls::Run::Extension(id) => {
                window.shell.update(cx, |shell, cx| shell.run_extension(&id, cx)).ok();
            }
            site_controls::Run::Certificate => unsafe { bc::bc_tab_show_certificate(tab) },
        }
    }

    fn menu(&self, items: &[MenuEntry], position: Point<Pixels>) -> Option<usize> {
        show_field_menu(items, position, self.view.get())
    }

    fn tooltip(&self, tip: Option<(SharedString, tooltip::Anchor)>, cx: &mut App) {
        match tip {
            Some((text, anchor)) => tooltip::hover(tooltip::Tip::named(text), anchor, cx),
            None => tooltip::dismiss(cx),
        }
    }
}

/// The user left the Site Controls: a click outside them, another window or another app.
extern "C" fn site_controls_dismissed() {
    update_app(site_controls::dismissed);
}

/// The toast's requests, to the host's toast window.
struct ToastHost {
    /// The toast's view, for the share menu its Share button shows, once its window is open.
    view: Cell<*mut c_void>,
}

impl toast::Host for ToastHost {
    fn show(&self) {
        unsafe { platform::host_show_toast(front().1) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_toast() };
    }

    fn take_clicks(&self, takes: bool) {
        unsafe { platform::host_toast_takes_clicks(takes) };
    }
}

/// Capture's requests, as calls to the core, to the host's window for capture mode and to the
/// system, over the window in front (`front`).
struct CaptureHost {
    /// The toast's view, which its Share button's menu shows from.
    toast: Rc<ToastHost>,
}

impl capture::Host for CaptureHost {
    fn show(&self, frame: Bounds<Pixels>) {
        let (left, top) = (f64::from(frame.origin.x), f64::from(frame.origin.y));
        let (width, height) = (f64::from(frame.size.width), f64::from(frame.size.height));
        unsafe { platform::host_show_capture(front().1, left, top, width, height) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_capture() };
    }

    fn camera(&self, camera: bool) {
        unsafe { platform::host_capture_camera(camera) };
    }

    fn element_at(&self, tab: u64, at: Point<Pixels>) {
        unsafe { bc::bc_tab_element_at(tab, f64::from(at.x), f64::from(at.y)) };
    }

    fn capture(&self, tab: u64, request: u64, full_page: bool, rect: Option<Bounds<Pixels>>) {
        let rect = rect.map_or(bc::bc_rect { x: 0., y: 0., width: 0., height: 0. }, |rect| {
            bc::bc_rect {
                x: f64::from(rect.origin.x),
                y: f64::from(rect.origin.y),
                width: f64::from(rect.size.width),
                height: f64::from(rect.size.height),
            }
        });
        unsafe { bc::bc_tab_capture(tab, request, full_page, rect) };
    }

    fn end(&self, tab: u64) {
        unsafe { bc::bc_tab_capture_end(tab) };
    }

    fn copy(&self, png: &[u8]) {
        unsafe { platform::host_copy_png(png.as_ptr(), png.len()) };
    }

    fn save(&self, png: &[u8]) {
        unsafe { platform::host_save_png(front().1, png.as_ptr(), png.len()) };
    }

    fn share(&self, png: &[u8], share: capture::Share) {
        let (view, at, messages) = match share {
            capture::Share::Menu(at) => (front().1, at, false),
            capture::Share::Toast(at) => {
                (self.toast.view.get(), Bounds::new(at, size(px(1.), px(1.))), false)
            }
            capture::Share::Messages => (std::ptr::null_mut(), Bounds::default(), true),
        };
        let (x, y) = (f64::from(at.origin.x), f64::from(at.origin.y));
        let (width, height) = (f64::from(at.size.width), f64::from(at.size.height));
        unsafe {
            platform::host_share_png(png.as_ptr(), png.len(), view, x, y, width, height, messages)
        };
    }
}

/// The user left capture mode: a click outside it, another window or another app.
extern "C" fn capture_dismissed() {
    update_app(capture::dismissed);
}

unsafe extern "C" fn tab_element_box(
    _: *mut c_void,
    tab: bc::bc_id,
    x: f64,
    y: f64,
    box_: bc::bc_rect,
) {
    let at = point(px(x as f32), px(y as f32));
    let found = (box_.width > 0. && box_.height > 0.).then(|| {
        Bounds::new(
            point(px(box_.x as f32), px(box_.y as f32)),
            size(px(box_.width as f32), px(box_.height as f32)),
        )
    });
    update_app_later(move |cx| capture::element_box(tab, at, found, cx));
}

unsafe extern "C" fn tab_captured(
    _: *mut c_void,
    tab: bc::bc_id,
    request: u64,
    png: *const u8,
    size: usize,
    width: i32,
    height: i32,
) {
    // check.sh reads this line.
    log!("browser: tab {tab} captured {request}: {width}x{height}, {size} bytes");
    let png = (!png.is_null()).then(|| unsafe { std::slice::from_raw_parts(png, size) }.to_vec());
    // A window's picture of a page another window shows, not capture mode's.
    if shell::is_picture(request) {
        update_shells(|shell, cx| shell.page_pictured(request, png.clone(), cx));
        return;
    }
    update_app_later(move |cx| capture::captured(tab, request, png, cx));
}

/// The zoom HUD's requests, to the host's HUD window, over the browser window it names.
struct HudHost;

impl zoom_hud::Host for HudHost {
    fn show(&self, shell: &WeakEntity<Shell>) {
        let ui_view = WINDOWS.with_borrow(|windows| {
            let mut windows = windows.iter();
            let window = windows.find(|window| window.shell.entity_id() == shell.entity_id());
            window.map(|window| window.ui_view)
        });
        if let Some(ui_view) = ui_view {
            unsafe { platform::host_show_hud(ui_view) };
        }
    }

    fn hide(&self) {
        unsafe { platform::host_hide_hud() };
    }
}

/// The command bar's requests, as calls to the core and to the host's overlay window, for the
/// window it shows over (`front`).
struct Bar;

impl command_bar::Host for Bar {
    fn suggest(&self, site: Option<&str>, text: &str, complete: bool) {
        let Ok(site) = site.map(CString::new).transpose() else {
            return;
        };
        let site = site.as_ref().map_or(std::ptr::null(), |site| site.as_ptr());
        if let Ok(text) = CString::new(text) {
            unsafe { bc::bc_window_suggest(front().0, site, text.as_ptr(), complete) };
        }
    }

    fn remove_suggestion(&self, url: &str) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_window_remove_suggestion(front().0, url.as_ptr()) };
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

    fn page_favicon(&self, url: &str) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_page_favicon(url.as_ptr()) };
        }
    }

    fn open_tab(&self, url: &str, behind: bool) {
        match behind {
            // ⌘↩'s tab loads once it's first shown, as a link opened behind does (Dia).
            true => _ = open_tab_behind(front().0, url, true),
            false => open_tab(front().0, url),
        }
    }

    fn open_window(&self, url: &str, cx: &mut App) {
        let (id, url) = (front().0, url.to_string());
        // Outside the bar's update, as ⌘N's window opens outside the shell's.
        cx.defer(move |cx| {
            if let Some(window) = open_window_after(id, cx) {
                open_tab(window, &url);
            }
        });
    }

    fn navigate(&self, tab: u64, url: &str) {
        if let Ok(url) = CString::new(url) {
            unsafe { bc::bc_tab_navigate(tab, url.as_ptr()) };
        }
    }

    fn select(&self, tab: u64) {
        unsafe { bc::bc_window_select(front().0, tab) };
    }

    fn tab_window(&self, tab: u64) -> Option<u64> {
        let window = unsafe { bc::bc_tab_window(tab) };
        (window != 0 && window != front().0).then_some(window)
    }

    fn activate(&self, window: u64, tab: u64, cx: &mut App) {
        let Some(ui_view) = WINDOWS.with_borrow(|windows| {
            windows.iter().find(|open| open.id == window).map(|open| open.ui_view)
        }) else {
            return;
        };
        // Outside any update: the window reports that it's key at once, to its shell.
        cx.spawn(async move |_| unsafe {
            bc::bc_window_select(window, tab);
            platform::host_activate(ui_view);
        })
        .detach();
    }

    fn split(&self, tab: u64, with: u64, cx: &mut App) {
        split_tab(front().0, tab, with, cx);
    }

    fn open_split(&self, url: &str, with: u64, cx: &mut App) {
        open_split(front().0, url, with, cx);
    }

    fn copy(&self, text: &str, cx: &mut App) {
        shell::copy_from_bar(text, cx);
    }

    fn show(&self, centered: bool, left: f32, top: f32, width: f32, radius: f32) {
        let (left, top, width) = (f64::from(left), f64::from(top), f64::from(width));
        unsafe {
            platform::host_set_overlay_radius(COMMAND_BAR, radius.into());
            platform::host_show_overlay(COMMAND_BAR, front().1, centered, left, top, width)
        };
    }

    fn set_height(&self, height: f32) {
        unsafe { platform::host_set_overlay_height(COMMAND_BAR, f64::from(height)) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_overlay(COMMAND_BAR) };
    }

    fn dismiss(&self) {
        unsafe { platform::host_dismiss_overlay(COMMAND_BAR) };
    }

    fn commands(&self, text: &str, cx: &App) -> Vec<shell::BarCommand> {
        shell::bar_commands(text, cx)
    }
}

/// The user left the command bar without choosing anything in it.
extern "C" fn command_bar_dismissed() {
    update_app(command_bar::dismissed);
    update_window(front().0, |shell, cx| shell.command_bar_dismissed(cx));
}

/// The icon picker's requests, to the host's overlay window, over the window in front.
struct PickerHost;

impl icon_picker::Host for PickerHost {
    fn show(&self, left: f32, top: f32) {
        let (left, top, width) = (left.into(), top.into(), icon_picker::WIDTH.into());
        unsafe { platform::host_show_overlay(ICON_PICKER, front().1, false, left, top, width) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_overlay(ICON_PICKER) };
    }

    fn keep_recents(&self, recents: &str) {
        if let Err(error) = std::fs::write(recent_emoji_path(), recents) {
            log!("couldn't save the recent emoji: {error}");
        }
    }

    fn draws(&self, emoji: &str) -> bool {
        // An emoji has no NUL, so the string always converts.
        std::ffi::CString::new(emoji)
            .is_ok_and(|emoji| unsafe { platform::host_can_draw_emoji(emoji.as_ptr()) })
    }
}

/// Where the emoji the icon picker offers first are kept: in the profile directory beside the
/// shortcuts, as the core keeps no settings of the app's own.
fn recent_emoji_path() -> String {
    format!("{}/Recent Emoji", profile_dir())
}

fn recent_emoji() -> icon_picker::Recents {
    icon_picker::Recents::parse(&std::fs::read_to_string(recent_emoji_path()).unwrap_or_default())
}

/// The user left the icon picker without picking.
extern "C" fn icon_picker_dismissed() {
    update_app(icon_picker::dismissed);
}

/// The tooltip's requests, to the host's tooltip window.
struct TooltipHost;

impl tooltip::Host for TooltipHost {
    fn show(&self, left: f32, top: f32) {
        let fade = tooltip::FADE_IN.as_secs_f64();
        unsafe { platform::host_show_tooltip(front().1, left.into(), top.into(), fade) };
    }

    fn hide(&self) {
        unsafe { platform::host_hide_tooltip(tooltip::FADE_OUT.as_secs_f64()) };
    }
}

/// The window lost the keyboard, or the app stopped being the active one.
extern "C" fn tooltip_away() {
    update_app(tooltip::dismiss);
}

/// A window's shell's requests, as calls to the core. The core reports back once each call has
/// returned.
struct Engine {
    /// The window's, in the core.
    window: bc::bc_id,
    /// The view the UI draws in, for the host.
    ui_view: *mut c_void,
    /// GPUI's view, which has the keyboard when the UI does.
    view: *mut c_void,
}

impl Browser for Engine {
    fn select(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_window_select(self.window, tab) };
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
        let tabs = urls
            .iter()
            .enumerate()
            .map(|(at, url)| (at, open_tab_behind(self.window, url, at > 0)));
        let tabs: Vec<u64> = tabs.filter(|&(_, tab)| tab != 0).map(|(_, tab)| tab).collect();
        for (at, &tab) in tabs.iter().enumerate() {
            unsafe { bc::bc_tab_move(tab, pinned, folder, index + at) };
        }
        let first = tabs.first().copied().unwrap_or(0);
        if first != 0 {
            unsafe { bc::bc_window_select(self.window, first) };
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
            bc::bc_window_select(self.window, tab);
        }
    }

    fn show_drop_targets(&self, shown: bool, cx: &mut App) {
        let ui_view = self.ui_view;
        // After the shell's draw, which asks for it.
        cx.defer(move |cx| drop_targets::show(shown, ui_view, cx));
    }

    fn back_to_pinned_url(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_back_to_pinned_url(tab) };
    }

    fn set_muted(&self, tab: u64, muted: bool, _: &mut App) {
        unsafe { bc::bc_tab_set_muted(tab, muted) };
    }

    fn clear(&self, _: &mut App) {
        unsafe { bc::bc_window_clear(self.window) };
    }

    fn undo_clear(&self, _: &mut App) -> usize {
        unsafe { bc::bc_window_undo_clear(self.window) }
    }

    fn undo_delete_space(&self, _: &mut App) -> bool {
        unsafe { bc::bc_window_undo_delete_space(self.window) != 0 }
    }

    fn reopen_closed_tab(&self, _: &mut App) {
        unsafe { bc::bc_window_reopen_closed_tab(self.window) };
    }

    fn restore_archived(&self, archived: u64, _: &mut App) {
        unsafe { bc::bc_window_restore_archived(self.window, archived) };
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

    fn file_preview(&self, download: u64, path: &str, _: &mut App) {
        if let Ok(path) = CString::new(path) {
            unsafe { platform::host_file_preview(path.as_ptr(), download, file_preview) };
        }
    }

    fn trash(&self, path: &str, _: &mut App) -> bool {
        CString::new(path).is_ok_and(|path| unsafe { platform::host_trash(path.as_ptr()) })
    }

    fn drag_file(&self, path: &str, _: &mut App) -> bool {
        CString::new(path)
            .is_ok_and(|path| unsafe { platform::host_drag_file(path.as_ptr(), file_drag_ended) })
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
        zoom_hud::show(tab, percent, shell_with_view(self.ui_view), cx);
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
        unsafe { bc::bc_window_task_manager(self.window) };
    }

    fn set_menus(&self, spaces: &[SharedString], extensions: &[Extension], cx: &mut App) {
        // The menu bar is the key window's, which lists its own Space's extensions.
        if front().0 == self.window {
            cx.set_menus(menus(spaces, extensions));
        }
    }

    fn run_extension(&self, id: &str, anchor: Bounds<Pixels>, _: &mut App) -> bool {
        let Ok(id) = CString::new(id) else {
            return false;
        };
        unsafe { bc::bc_window_run_extension(self.window, id.as_ptr(), rect(anchor)) }
    }

    fn run_extension_command(&self, id: &str, name: &str, _: &mut App) -> bool {
        let (Ok(id), Ok(name)) = (CString::new(id), CString::new(name)) else {
            return false;
        };
        unsafe { bc::bc_window_run_extension_command(self.window, id.as_ptr(), name.as_ptr()) }
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
        open_tab(self.window, url);
    }

    fn manage_extensions(&self, id: Option<&str>, _: &mut App) {
        match id {
            Some(id) => open_tab(self.window, &format!("chrome://extensions/?id={id}")),
            None => open_tab(self.window, "chrome://extensions"),
        }
    }

    fn add_extension(&self, _: &mut App) {
        open_tab(self.window, WEB_STORE);
    }

    fn lay_out_side_panel(&self, frame: Bounds<Pixels>, _: &mut App) {
        unsafe { bc::bc_window_set_side_panel_frame(self.window, rect(frame)) };
    }

    fn close_side_panel(&self, _: &mut App) {
        unsafe { bc::bc_window_close_side_panel(self.window) };
    }

    fn has_side_panel(&self, id: &str) -> bool {
        let Ok(id) = CString::new(id) else {
            return false;
        };
        unsafe { bc::bc_window_has_side_panel(self.window, id.as_ptr()) }
    }

    fn toggle_side_panel(&self, id: &str, _: &mut App) {
        if let Ok(id) = CString::new(id) {
            unsafe { bc::bc_window_toggle_side_panel(self.window, id.as_ptr()) };
        }
    }

    fn set_side_panel_width(&self, width: f32, _: &mut App) {
        unsafe { bc::bc_window_set_side_panel_width(self.window, width.into()) };
    }

    fn view_history(&self, _: &mut App) {
        open_tab(self.window, "chrome://history");
    }

    fn close_other_window(&self, _: &mut App) -> bool {
        unsafe { platform::host_close_other_window() }
    }

    fn show_space(&self, space: u64, _: &mut App) {
        unsafe { bc::bc_window_show_space(self.window, space) };
    }

    fn space_tab(&self, space: u64) -> u64 {
        unsafe { bc::bc_window_space_tab(self.window, space) }
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

    fn set_space_icon(&self, space: u64, icon: &str, _: &mut App) {
        let icon = CString::new(icon).unwrap_or_default();
        unsafe { bc::bc_space_set_icon(space, icon.as_ptr()) };
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
        let ui_view = self.ui_view;
        cx.spawn(async move |cx| {
            let place = if in_reveal { platform::IN_REVEAL } else { platform::IN_WINDOW };
            let index = show_menu(ui_view, &items, position, place, over);
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
        set_page_frame(self.window, (frame, radius));
    }

    fn set_focus_color(&self, color: u32, cx: &mut App) {
        command_bar::set_accent(color, cx);
        unsafe { bc::bc_window_set_focus_color(self.window, color) };
    }

    fn set_page_color(&self, color: u32, _: &mut App) {
        unsafe { bc::bc_window_set_page_color(self.window, color) };
    }

    fn page_moving(&self, moving: bool, _: &mut App) {
        unsafe { bc::bc_window_page_moving(self.window, moving) };
    }

    fn set_pane_corner(&self, right: f32, top: f32, _: &mut App) {
        unsafe { platform::host_set_pane_corner(self.ui_view, right as f64, top as f64) };
    }

    fn show_window_buttons(&self, shown: bool, _: &mut App) {
        unsafe { platform::host_show_window_buttons(self.ui_view, shown) };
    }

    fn press_window_button(&self, button: WindowButton, _: &mut App) {
        let button = match button {
            WindowButton::Close => 0,
            WindowButton::Minimize => 1,
            WindowButton::Zoom => 2,
        };
        unsafe { platform::host_press_window_button(self.ui_view, button) };
    }

    fn show_reveal(&self, width: f32, _: &mut App) {
        unsafe { platform::host_show_reveal(self.ui_view, width.into()) };
    }

    fn hide_reveal(&self, _: &mut App) {
        unsafe { platform::host_hide_reveal(self.ui_view) };
    }

    fn saved_sidebar_width(&self) -> Option<f32> {
        let width = unsafe { bc::bc_window_sidebar_width(self.window) };
        (width > 0.).then_some(width as f32)
    }

    fn set_sidebar_width(&self, width: f32, _: &mut App) {
        unsafe { bc::bc_window_set_sidebar_width(self.window, width.into()) };
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

    fn can_open_in_safari(&self) -> bool {
        true
    }

    fn open_in_safari(&self, url: &str, _: &mut App) {
        if let Ok(url) = CString::new(url) {
            unsafe { platform::host_open_in_safari(url.as_ptr()) };
        }
    }

    fn toast(&self, text: &str, key: Option<(&str, &str)>, time: Duration, cx: &mut App) {
        toast::show(text, key, time, cx);
    }

    fn full_screen_note(&self, shown: bool, cx: &mut App) {
        match shown {
            true => toast::show_full_screen(cx),
            false => toast::hide_full_screen(cx),
        }
    }

    fn tooltip(&self, tip: Option<(tooltip::Tip, tooltip::Anchor)>, cx: &mut App) {
        match tip {
            Some((tip, anchor)) => tooltip::hover(tip, anchor, cx),
            None => tooltip::dismiss(cx),
        }
    }

    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        tabs: Vec<BarTab>,
        field: Option<Bounds<Pixels>>,
        cx: &mut App,
    ) {
        command_bar::show(editing, tabs, field, cx);
    }

    fn command_bar_open(&self, cx: &App) -> bool {
        command_bar::is_open(cx)
    }

    fn split_bar(&self, with: u64, pane: Bounds<Pixels>, card: Bounds<Pixels>, cx: &mut App) {
        command_bar::show_split(with, pane, card, cx);
    }

    fn find(&self, tab: u64, cx: &mut App) {
        find_bar::show(self.window, tab, cx);
    }

    fn site_controls(
        &self,
        tab: u64,
        place: site_controls::Place,
        window: gpui::Size<Pixels>,
        extensions: Vec<site_controls::Extension>,
        cx: &mut App,
    ) {
        site_controls::show(tab, place, window, extensions, cx);
    }

    fn capture(
        &self,
        tab: u64,
        frame: Bounds<Pixels>,
        radius: f32,
        full_page: bool,
        cx: &mut App,
    ) {
        match full_page {
            true => capture::full_page(tab, cx),
            false => capture::start(tab, frame, radius, cx),
        }
    }

    fn page_picture(&self, tab: u64, request: u64, _: &mut App) {
        // The page's whole view, which is the shape it has where it's live; the card fits it.
        let whole = bc::bc_rect { x: 0., y: 0., width: 0., height: 0. };
        unsafe { bc::bc_tab_capture(tab, request, false, whole) };
    }

    fn page_picture_end(&self, tab: u64, _: &mut App) {
        unsafe { bc::bc_tab_capture_end(tab) };
    }

    fn window(&self) -> u64 {
        self.window
    }

    fn find_next(&self, tab: u64, forward: bool, cx: &mut App) {
        find_bar::find_next(self.window, tab, forward, cx);
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

    fn new_window(&self, cx: &mut App) {
        let id = self.window;
        // Outside the shell's update, as the new window's shell reads this one's.
        cx.defer(move |cx| {
            // From a private window, the Spaces are the regular window's in front.
            let from = WINDOWS.with_borrow(|windows| {
                let this = windows.iter().find(|w| w.id == id && !w.private);
                this.or_else(|| windows.iter().find(|w| !w.private)).cloned()
            });
            if from.is_some() {
                open_window(from, None, cx);
            }
        });
    }

    fn new_private_window(&self, cx: &mut App) {
        let id = self.window;
        cx.defer(move |cx| {
            let from = WINDOWS.with_borrow(|windows| windows.iter().find(|w| w.id == id).cloned());
            // The profile of the Space this window shows, a private one's too.
            let profile = unsafe { bc::bc_space_profile(bc::bc_window_space(id)) };
            if from.is_some() && profile != 0 {
                open_window(from, Some(profile), cx);
            }
        });
    }

    fn close_window(&self, cx: &mut App) {
        let id = self.window;
        // Outside the shell's update: the core closes the NSWindow at once, and GPUI's handler for
        // that needs the App.
        cx.spawn(async move |_| {
            if WINDOWS.with_borrow(|windows| windows.iter().any(|w| w.id == id)) {
                unsafe { bc::bc_window_close(id) };
            }
        })
        .detach();
    }

    /// Asks first, as Arc does, unless the user said not to. Only the user's Quit asks: a quit from
    /// the Dock or at logout doesn't come here.
    fn quit(&self, cx: &mut App) {
        // Once the shell that asked is done, as it reads the shell. Chromium's quit is posted, so
        // comes after.
        cx.defer(save_launch_picture);
        // With no window to ask over, there's nothing open to lose.
        if !unsafe { bc::bc_warn_before_quitting() } || front().1.is_null() {
            unsafe { bc::bc_quit() };
            return;
        }
        let question = dialog::Question {
            icon: None,
            message: concat!("Quit ", env!("APP_NAME"), "?").into(),
            detail: "".into(),
            list: Vec::new(),
            action: "Quit".into(),
            cancel: "Cancel".into(),
            tone: dialog::Tone::Safe,
            also: Some((
                "Quit, and don’t ask again".into(),
                Box::new(|_| {
                    // check.sh waits for this line.
                    log!("browser: warn before quitting off");
                    unsafe {
                        bc::bc_set_warn_before_quitting(false);
                        bc::bc_quit();
                    }
                }),
            )),
        };
        dialog::pose(question, Box::new(|_| unsafe { bc::bc_quit() }), cx);
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

/// Runs the shortcut `key` is, if it's one of the page shortcuts that are `reserved` or aren't,
/// on the keys the user has now, in the window showing `tab`'s page, which had the key.
fn run_page_shortcut(tab: bc::bc_id, key: *const bc::bc_key, reserved: bool) -> bool {
    let key = unsafe { &*key };
    let keystroke = keystroke(key);
    // Read without an update, as every key the page gets comes here.
    let cx = CX.with_borrow(Clone::clone);
    let command = cx.and_then(|cx| {
        cx.try_read_global(|keys: &shell::UserKeys, _| keys.command_for(&keystroke, reserved))
    });
    let Some(command) = command.flatten() else {
        return false;
    };
    // ⌘Z in a text field is the page's, which the Edit menu gives it.
    if command == Command::Undo && key.editing {
        return false;
    }
    update_tab_window(tab, |shell, cx| shell.run_for_page(command, cx)).unwrap_or(false)
}

unsafe extern "C" fn tab_key_before_page(
    _: *mut c_void,
    tab: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    // As in Chrome, the reserved shortcuts come first, then the extensions' keys.
    if run_page_shortcut(tab, key, true) {
        return true;
    }
    let keystroke = keystroke(unsafe { &*key });
    if update_tab_window(tab, |shell, cx| shell.run_extension_key(&keystroke, "page", cx))
        .unwrap_or(false)
    {
        return true;
    }
    // The keys typed after ⌘L would reach the page before it hands ⌘L back to open the bar.
    let cx = CX.with_borrow(Clone::clone);
    let holds = cx.and_then(|cx| {
        cx.try_read_global(|keys: &shell::UserKeys, _| keys.holds_keys(&keystroke))
    });
    if holds == Some(true) {
        unsafe { platform::host_hold_keys() };
    }
    false
}

unsafe extern "C" fn tab_key_unhandled(
    _: *mut c_void,
    tab: bc::bc_id,
    key: *const bc::bc_key,
) -> bool {
    run_page_shortcut(tab, key, false)
}

/// Opens a browser window centered on the main display, or cascaded from the one of `after` and as
/// big, with `ui_view` filling it under the page, and returns its id. With `private`, a profile,
/// it's a private window in that profile's off-the-record session.
fn open_browser_window(
    ui_view: *mut c_void,
    after: Option<*mut c_void>,
    private: Option<bc::bc_id>,
) -> Option<bc::bc_id> {
    let (mut width, mut height) = WINDOW_SIZE;
    let (mut x, mut y) = (0., 0.);
    match after {
        Some(after) => unsafe {
            platform::host_cascaded(after, &mut x, &mut y, &mut width, &mut height)
        },
        None => unsafe { platform::host_centered(width, height, &mut x, &mut y) },
    }
    let bounds = bc::bc_rect {
        x,
        y,
        width,
        height,
    };
    let window = match private {
        Some(profile) => unsafe { bc::bc_window_open_private(bounds, ui_view, profile) },
        None => unsafe { bc::bc_window_open(bounds, ui_view) },
    };
    (window != 0).then_some(window)
}

/// Shows the Spaces as the user left them in `window`, else a first Space with the first tab. A
/// page given in BROWSER_URL opens on top either way.
fn restore_spaces(window: bc::bc_id) {
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
        Ok(url) => open_tab(window, &url),
        Err(_) if !restored => open_tab(window, START_URL),
        Err(_) => {}
    }
}

extern "C" fn accent_color_changed() {
    update_shells(|shell, cx| shell.accent_color_changed(cx));
}

// The ⌃Tab walk and the file's drag were the user's in the window in front.
extern "C" fn control_released() {
    update_window(front().0, |shell, cx| shell.control_released(cx));
}

extern "C" fn file_preview(
    download: u64,
    png: *const u8,
    size: usize,
    width: f64,
    height: f64,
    image: bool,
) {
    let preview = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        let png = Arc::new(Image::from_bytes(ImageFormat::Png, png));
        FilePreview { image: png, width: width as f32, height: height as f32, photo: image }
    });
    update_shells(|shell, cx| shell.set_download_preview(download, preview.clone(), cx));
}

extern "C" fn file_drag_ended(taken: bool) {
    update_window(front().0, |shell, cx| shell.file_drag_ended(taken, cx));
}

extern "C" fn dragged(
    ui_view: *mut c_void,
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
    let answer = update_window(window_with_view(ui_view), |shell, cx| match phase {
        platform::DRAG_OVER => !urls.is_empty() && shell.drag_in(position, cx),
        platform::DRAG_DROP => shell.drop_in(position, &urls, cx),
        _ => {
            shell.drag_away(cx);
            false
        }
    });
    answer.unwrap_or(false)
}

/// Opens a browser window after `id`'s, for ⇧↩ in the command bar: as ⌘N does, or from a private
/// window as ⇧⌘N does, so what was typed there stays off the record. Returns it.
fn open_window_after(id: bc::bc_id, cx: &mut App) -> Option<bc::bc_id> {
    let from = WINDOWS.with_borrow(|windows| windows.iter().find(|w| w.id == id).cloned())?;
    // The profile of the Space a private window shows, as ⇧⌘N takes it.
    let private = from.private.then(|| unsafe { bc::bc_space_profile(bc::bc_window_space(id)) });
    if private == Some(0) {
        return None;
    }
    open_window(Some(from), private, cx)
}

/// Opens a browser window and the shell inside it under the page, and returns it. The first opens
/// the overlays every window shares and the Spaces as the user left them. Another, from `from`,
/// starts with its shell's state and shows its Space with no page, as ⌘N's window does. With
/// `private`, a profile, it's ⇧⌘N's private window, which shows a Space of its own instead, with
/// no page.
fn open_window(
    from: Option<BrowserWindow>,
    private: Option<bc::bc_id>,
    cx: &mut App,
) -> Option<bc::bc_id> {
    let ui_view = unsafe { platform::host_ui_view() };
    let after = from.as_ref().map(|from| from.ui_view);
    let id = open_browser_window(ui_view, after, private)?;
    let from_shell = from.as_ref().and_then(|from| from.shell.upgrade());
    // Its Space, which the core made with the window.
    let private_space = private.map(|profile| (unsafe { bc::bc_window_space(id) }, profile));
    if let Some(from) = &from {
        // The shell reads its sidebar's width from the core as it's made.
        let width = unsafe { bc::bc_window_sidebar_width(from.id) };
        unsafe { bc::bc_window_set_sidebar_width(id, width) };
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
            cx.observe_window_bounds(window, move |shell: &mut Shell, window, _| {
                let viewport = window.viewport_size();
                if let Some(frame) = shell.window_page_frame(viewport) {
                    set_frames(id, shell, frame, viewport);
                    shell.full_screen_step("laid", viewport);
                }
            })
            .detach();
            let engine = Rc::new(Engine { window: id, ui_view, view: ns_view(window) });
            match (&from_shell, private_space) {
                (_, Some((space, profile))) => {
                    Shell::private_window(engine, space, private_name(profile), profile, cx)
                }
                (Some(from), None) => Shell::seeded_from(from, engine, cx),
                (None, None) => Shell::new(engine, cx),
            }
        });
        window.focus(&shell.focus_handle(cx), cx);
        CX.set(Some(cx.to_async()));
        // In front, as it's about to be key.
        WINDOWS.with_borrow_mut(|windows| {
            windows.insert(
                0,
                BrowserWindow {
                    id,
                    ui_view,
                    shell: shell.downgrade(),
                    handle: Window::window_handle(window),
                    children: Vec::new(),
                    private: private.is_some(),
                },
            )
        });
        if from_shell.is_none() {
            // The core read the extensions before there was a shell to take them.
            unsafe { bc::bc_extensions_report() };
        }
        shell
    });
    let window = match window {
        Ok(window) => window,
        Err(error) => {
            log!("couldn't open the window: {error}");
            if from.is_none() {
                unsafe { bc::bc_quit() };
            } else {
                unsafe { bc::bc_window_close(id) };
            }
            return None;
        }
    };
    window
        .update(cx, |shell, window, _| {
            let viewport = window.viewport_size();
            set_frames(id, shell, shell.page_frame(viewport), viewport)
        })
        .ok();
    if let Some(from) = from.as_ref().filter(|_| private.is_none()) {
        let space = unsafe { bc::bc_window_space(from.id) };
        unsafe { bc::bc_window_show_space_empty(id, space) };
        // Its Space drawn before it's on screen, rather than once the core's report comes.
        window.update(cx, |shell, _, cx| shell.set_space(space, cx)).ok();
    }
    let first = from.is_none();
    if first {
        // Once the page frame is set, so the tab shown loads at its size, and before the window is
        // on screen, so the core's reports of its tabs can come before its first frame.
        restore_spaces(id);
    }
    unsafe { platform::host_show_window(ui_view) };
    if !first {
        finish_opening(id, ui_view, window, true, private, cx);
        return Some(id);
    }
    // Making the overlays' windows takes the main thread for a while, so at launch they wait for
    // the frame after the first: the window goes on screen with its sidebar drawn, then they're
    // made. Later windows don't wait, as a page may open in them at once. A window that isn't
    // visible (another desktop, launched hidden) draws no frames, so a timer makes them if the
    // frames haven't.
    let pending = Rc::new(Cell::new(true));
    let finish = {
        let pending = pending.clone();
        move |cx: &mut App| {
            if pending.replace(false) {
                finish_opening(id, ui_view, window, false, private, cx);
            }
        }
    };
    let on_frame = finish.clone();
    window
        .update(cx, |_, window, _| {
            // Deferred, as the shell's window is borrowed while it draws.
            window.on_next_frame(move |window, _| {
                window.on_next_frame(move |_, cx| cx.defer(on_frame))
            })
        })
        .ok();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(Duration::from_millis(500)).await;
        cx.update(finish)
    })
    .detach();
    Some(id)
}

fn finish_opening(
    id: bc::bc_id,
    ui_view: *mut c_void,
    window: WindowHandle<Shell>,
    seeded: bool,
    private: Option<bc::bc_id>,
    cx: &mut App,
) {
    unsafe { platform::host_close_launch_window() };
    let Ok(shell) = window.entity(cx) else {
        return;
    };
    let children: Vec<AnyWindowHandle> =
        [find_bar::open(id, ui_view, cx), reveal::open(shell, ui_view, cx)]
            .into_iter()
            .flatten()
            .collect();
    WINDOWS.with_borrow_mut(|windows| {
        if let Some(window) = windows.iter_mut().find(|window| window.id == id) {
            window.children = children;
        }
    });
    // check.sh reads these lines.
    log!("browser: window opened {id}");
    if let Some(profile) = private {
        log!("browser: private window opened {id} profile {profile}");
        // The core showed its Space before there was a window to title.
        title_windows();
    }
    if seeded {
        // With no page, the keyboard is the UI's, as in a Space without tabs.
        if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
            unsafe { platform::host_focus(view) };
        }
        return;
    }
    unsafe {
        platform::host_watch_accent_color(accent_color_changed);
        platform::host_take_drops(dragged);
        platform::host_watch_key_window(key_window_changed);
        platform::host_watch_full_screen(full_screen_changed);
        platform::host_watch_resigning_active(launch_picture_on_resigning);
    }
    if let Some(bar) = command_bar::open(Rc::new(Bar), cx)
        && let Ok(view) = bar.update(cx, |_, window, _| ns_view(window))
    {
        let radius = command_bar::RADIUS.into();
        unsafe { platform::host_add_overlay(COMMAND_BAR, view, radius, command_bar_dismissed) };
    }
    if let Some(picker) = icon_picker::open(Rc::new(PickerHost), recent_emoji(), cx)
        && let Ok(view) = picker.update(cx, |_, window, _| ns_view(window))
    {
        let corners: Vec<f64> = icon_picker::corners()
            .iter()
            .flat_map(|&(x, y, radius)| [x, y, radius].map(f64::from))
            .collect();
        unsafe {
            platform::host_add_overlay(ICON_PICKER, view, 0., icon_picker_dismissed);
            platform::host_set_overlay_shape(ICON_PICKER, corners.as_ptr(), corners.len() / 3);
            let (x, y) = icon_picker::ARROW_TIP;
            platform::host_set_overlay_growth(ICON_PICKER, x.into(), y.into());
        }
    }
    if let Some(dialog) = dialog::open(Rc::new(DialogHost), cx)
        && let Ok(view) = dialog.update(cx, |_, window, _| ns_view(window))
    {
        unsafe { platform::host_add_dialog(view) };
    }
    // The window the toast shows over gives its colour.
    let fill = |cx: &App| {
        let shell = WINDOWS.with_borrow(|windows| windows.first().map(|w| w.shell.clone()));
        shell.and_then(|shell| shell.upgrade()).map(|shell| shell.read(cx).window_fill())
    };
    let toast_host = Rc::new(ToastHost { view: Cell::new(std::ptr::null_mut()) });
    if let Some(toast) =
        toast::open(toast_host.clone(), move |cx| fill(cx).unwrap_or_default(), cx)
        && let Ok(view) = toast.update(cx, |_, window, _| ns_view(window))
    {
        toast_host.view.set(view);
        unsafe { platform::host_add_toast(view) };
    }
    if let Some(capture) = capture::open(Rc::new(CaptureHost { toast: toast_host }), cx)
        && let Ok(view) = capture.update(cx, |_, window, _| ns_view(window))
    {
        unsafe { platform::host_add_capture(view, capture_dismissed) };
    }
    let host = Rc::new(SiteControlsHost { view: Cell::new(std::ptr::null_mut()) });
    if let Some(panel) = site_controls::open(host.clone(), cx)
        && let Ok(view) = panel.update(cx, |_, window, _| ns_view(window))
    {
        host.view.set(view);
        unsafe { platform::host_add_site_controls(view, site_controls_dismissed) };
    }
    if let Some(tip) = tooltip::open(Rc::new(TooltipHost), cx)
        && let Ok(view) = tip.update(cx, |_, window, _| ns_view(window))
    {
        unsafe { platform::host_add_tooltip(view, tooltip_away) };
    }
    if let Some(hud) = zoom_hud::open(Rc::new(HudHost), cx)
        && let Ok(view) = hud.update(cx, |_, window, _| ns_view(window))
    {
        let (right, top) = (zoom_hud::WINDOW_RIGHT, zoom_hud::WINDOW_TOP);
        let margin = f64::from(zoom_hud::MARGIN);
        unsafe { platform::host_add_hud(view, f64::from(right), f64::from(top), margin) };
    }
    drop_targets::open(cx);
}

unsafe extern "C" fn started(_: *mut c_void) {
    let app = gpui_platform::application().with_assets(shell::Assets).run_embedded(|cx| {
        // The user's keys, before the shortcuts are bound to them.
        settings::read_shortcuts(cx);
        shell::bind_keys(cx);
        browser::text_input::bind_keys(cx);
        cx.set_global(FieldMenu(Rc::new(field_menu)));
        cx.on_action(|_: &Hide, cx| cx.hide());
        cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
        cx.on_action(|_: &Minimize, _| unsafe {
            platform::host_press_window_button(front().1, 1)
        });
        cx.bind_keys([
            KeyBinding::new("cmd-h", Hide, Some(shell::KEY_CONTEXT)),
            KeyBinding::new("alt-cmd-h", HideOthers, Some(shell::KEY_CONTEXT)),
            KeyBinding::new("cmd-m", Minimize, Some(shell::KEY_CONTEXT)),
        ]);
        cx.set_menus(menus(&[], &[]));
        open_window(None, None, cx);
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
            env!("APP_NAME"),
            vec![
                item("Settings…", Command::Settings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action(concat!("Hide ", env!("APP_NAME")), Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                item(concat!("Quit ", env!("APP_NAME")), Command::Quit),
            ],
        ),
        menu(
            "File",
            vec![
                item("New Tab…", Command::NewTab),
                item("New Window", Command::NewWindow),
                item("New Incognito Window", Command::NewPrivateWindow),
                item("Restore Last Closed Tab", Command::ReopenClosedTab),
                MenuItem::separator(),
                item("Open Command Bar", Command::EditUrl),
                MenuItem::separator(),
                item("Close Window", Command::CloseWindow),
                item("Archive Tab", Command::CloseTab),
                MenuItem::separator(),
                item("Capture…", Command::CapturePortion),
                item("Capture Full Page", Command::CaptureFullPage),
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
                .chain([
                    item("Add Extension…", Command::AddExtension),
                    item("Manage Extensions…", Command::ManageExtensions),
                ])
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

// Closing the last window quits, as its close button does in Chrome's last window.
unsafe extern "C" fn window_closed(_: *mut c_void, id: bc::bc_id) {
    let (window, left) = WINDOWS.with_borrow_mut(|windows| {
        let index = windows.iter().position(|window| window.id == id);
        (index.map(|index| windows.remove(index)), windows.len())
    });
    let private = window.as_ref().is_some_and(|window| window.private);
    if let Some(window) = window {
        unsafe { platform::host_forget_window(window.ui_view) };
        update_app(|cx| {
            find_bar::window_closed(id, cx);
            for child in window.children {
                child.update(cx, |_, window, _| window.remove_window()).ok();
            }
            // The shared overlays and the menu bar follow the window now in front.
            if let Some(front) = WINDOWS.with_borrow(|windows| windows.first().cloned()) {
                front.shell.update(cx, |shell, cx| shell.became_key(cx)).ok();
            }
        });
    }
    log!("browser: window closed {id}");
    // A private window alone doesn't keep the browser: the Spaces' pages closed with the last
    // regular window, which the next launch brings back, so the private ones close with them.
    let regular = WINDOWS.with_borrow(|windows| windows.iter().any(|window| !window.private));
    if left == 0 || (!private && !regular) {
        unsafe { bc::bc_quit() };
    }
}

unsafe extern "C" fn private_session_ended(_: *mut c_void, profile: bc::bc_id) {
    // check.sh reads this line.
    log!("browser: private session ended {profile}");
}

/// The page menu's Open Link in New Window and Open Link in Incognito Window: `url` opens in a
/// window from `window` as ⌘N's, or with `private` in a private one in `profile`'s session as
/// ⇧⌘N's, and shows there.
unsafe extern "C" fn window_open_link(
    _: *mut c_void,
    window: bc::bc_id,
    url: *const c_char,
    profile: bc::bc_id,
    private: bool,
) {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    update_app(|cx| {
        // A regular window takes its Spaces from a regular one, as ⌘N's does from a private one.
        let from = WINDOWS.with_borrow(|windows| {
            let this = windows.iter().find(|w| w.id == window && (private || !w.private));
            this.or_else(|| windows.iter().find(|w| !w.private)).cloned()
        });
        let private = private.then_some(profile);
        if let Some(opened) = from.and_then(|from| open_window(Some(from), private, cx)) {
            open_tab(opened, &url);
        }
    });
}

unsafe extern "C" fn launching(_: *mut c_void) {
    let Ok(profile) = CString::new(profile_dir()) else {
        return;
    };
    let (width, height) = WINDOW_SIZE;
    unsafe { platform::host_show_launch_window(profile.as_ptr(), width, height) };
}

unsafe extern "C" fn stopping(_: *mut c_void) {
    log!("browser: stopping");
}

// The next launch shows the front window's sidebar until its UI has drawn. Taken as the app stops
// being the active one and on the user's Quit, as a window is already hidden when it closes. A
// private window's isn't kept.
extern "C" fn launch_picture_on_resigning() {
    if let Some(cx) = CX.with_borrow(Clone::clone) {
        cx.update(save_launch_picture);
    }
}

fn save_launch_picture(cx: &mut App) {
    let Some(window) = WINDOWS.with_borrow(|windows| windows.first().cloned()) else {
        return;
    };
    let shell = window.shell.upgrade().filter(|_| !window.private);
    let width = shell.map_or(0., |shell| shell.read(cx).card_left());
    let Ok(profile) = CString::new(profile_dir()) else {
        return;
    };
    let profile = profile.as_ptr();
    let taken = unsafe { platform::host_save_launch_picture(window.ui_view, profile, width.into()) };
    // check.sh reads this line.
    log!("browser: launch picture {}", if taken { "taken" } else { "not taken" });
}

/// A Space's or a folder's name, or a Space's icon, which `copy` copies as bc_space_name does.
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
    let profiles: Vec<_> = ids(bc::bc_profiles)
        .into_iter()
        .map(|profile| (profile, name(bc::bc_profile_name, profile)))
        .collect();
    let spaces: Vec<_> = ids(bc::bc_spaces)
        .into_iter()
        .map(|space| {
            let mut color = 0;
            let has_color = unsafe { bc::bc_space_color(space, &mut color) };
            let profile = unsafe { bc::bc_space_profile(space) };
            let name_of = name(bc::bc_space_name, space);
            let icon = name(bc::bc_space_icon, space);
            let agent = name(bc::bc_space_agent_name, space);
            (space, name_of, has_color.then_some(color), profile, icon, agent)
        })
        .collect();
    update_shells(|shell, cx| {
        shell.set_profiles(profiles.clone(), cx);
        shell.set_spaces(spaces.clone(), cx);
    });
    title_windows();
    update_app(settings::refresh);
}

unsafe extern "C" fn favorites_changed(_: *mut c_void) {
    // Each with its holder: the Space it belongs to under Each Space, else its profile.
    let holder = |tab| unsafe {
        match bc::bc_favorite_space(tab) {
            0 => bc::bc_tab_profile(tab),
            space => space,
        }
    };
    let tabs: Vec<_> = ids(bc::bc_favorites).into_iter().map(|tab| (tab, holder(tab))).collect();
    update_shells(|shell, cx| shell.set_favorites(tabs.clone(), cx));
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
    update_shells(|shell, cx| {
        let (pinned, today) = (pinned.clone(), today.clone());
        shell.set_tabs(space, pinned, today, splits.clone(), folders.clone(), cx)
    });
}

/// Titles each window with its Space's name, which AppKit lists in the Window menu, and a private
/// window as private. The window doesn't draw its title.
fn title_windows() {
    let windows: Vec<_> = WINDOWS
        .with_borrow(|windows| windows.iter().map(|w| (w.id, w.ui_view, w.private)).collect());
    for (id, ui_view, private) in windows {
        let space = unsafe { bc::bc_window_space(id) };
        let title = match private {
            true => private_name(unsafe { bc::bc_space_profile(space) }),
            false => name(bc::bc_space_name, space),
        };
        let title = CString::new(title.as_ref()).unwrap_or_default();
        unsafe { platform::host_set_window_title(ui_view, title.as_ptr()) };
    }
}

/// A private window's name, in its sidebar and the Window menu: Chrome's, and its profile's while
/// there's more than one, as each has its own private session.
fn private_name(profile: bc::bc_id) -> SharedString {
    match ids(bc::bc_profiles).len() {
        0 | 1 => "Incognito".into(),
        _ => format!("Incognito · {}", name(bc::bc_profile_name, profile)).into(),
    }
}

unsafe extern "C" fn window_space_shown(_: *mut c_void, window: bc::bc_id, space: bc::bc_id) {
    update_window(window, |shell, cx| shell.set_space(space, cx));
    title_windows();
}

unsafe extern "C" fn window_selected(_: *mut c_void, window: bc::bc_id, tab: bc::bc_id) {
    // check.sh reads this line, as a tab's page moves between windows.
    log!("browser: window {window} shows {tab}");
    update_window(window, |shell, cx| {
        shell.set_selected(tab, cx);
        find_bar::tab_shown(window, tab, cx);
        zoom_hud::tab_shown(tab, cx);
    });
}

unsafe extern "C" fn window_page_elsewhere(_: *mut c_void, window: bc::bc_id, tab: bc::bc_id) {
    // check.sh reads this line.
    log!("browser: window {window} page elsewhere {tab}");
    update_window(window, |shell, cx| shell.page_elsewhere(tab, cx));
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
    update_shells(|shell, cx| shell.set_page(tab, page.clone(), cx));
}

unsafe extern "C" fn tab_title_changed(_: *mut c_void, tab: bc::bc_id, title: *const c_char) {
    let title = unsafe { CStr::from_ptr(title) }.to_string_lossy().into_owned();
    log!("browser: title {tab} {title}");
    update_shells(|shell, cx| shell.set_title(tab, title.clone().into(), cx));
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
    update_shells(|shell, cx| shell.set_favicon(tab, favicon.clone(), cx));
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
    let archive: Vec<_> = tabs
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
    update_shells(|shell, cx| shell.set_archive(archive.clone(), cx));
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
    let downloads: Vec<_> = downloads
        .iter()
        .map(|download| Download {
            id: download.id,
            name: text(download.name),
            url: text(download.url),
            path: text(download.path),
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
    update_shells(|shell, cx| shell.set_downloads(downloads.clone(), cx));
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
    update_shells(|shell, cx| shell.set_download_icon(download, icon.clone(), cx));
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
    let extensions: Vec<_> = extensions
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
    update_shells(|shell, cx| shell.set_extensions(profile, extensions.clone(), cx));
}

unsafe extern "C" fn window_extension_actions(
    _: *mut c_void,
    window: bc::bc_id,
    actions: *const bc::bc_extension_action,
    count: usize,
) {
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
    update_window(window, |shell, cx| shell.set_extension_actions(actions, cx));
}

unsafe extern "C" fn window_extension_question(
    _: *mut c_void,
    window: bc::bc_id,
    question: *const bc::bc_extension_question,
) {
    let Some(question) = (unsafe { question.as_ref() }) else {
        if window == front().0 {
            // check.sh waits for this line.
            log!("browser: extension question none");
            update_app(dialog::withdraw_chrome);
        }
        return;
    };
    let id = question.id;
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned();
    let lines = |texts: *const *const c_char| match texts.is_null() {
        true => Vec::new(),
        false => unsafe { std::slice::from_raw_parts(texts, question.warning_count) }
            .iter()
            .map(|&line| text(line))
            .collect(),
    };
    let warnings = lines(question.warnings);
    // check.sh waits for this line.
    log!("browser: extension question {} asks {}", text(question.extension), warnings.join("; "));
    let list: Vec<_> = warnings
        .into_iter()
        .zip(lines(question.details))
        .map(|(warning, details)| {
            (warning.into(), details.lines().map(|line| line.to_owned().into()).collect())
        })
        .collect();
    let question = dialog::Question {
        icon: (!question.icon.is_null()).then(|| {
            let png = unsafe { std::slice::from_raw_parts(question.icon, question.icon_size) };
            Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec()))
        }),
        message: text(question.title).into(),
        // Chrome's heading, such as "It can:", leads its warnings, and there's none without them.
        detail: if list.is_empty() { "".into() } else { text(question.heading).into() },
        list,
        action: text(question.accept).into(),
        cancel: text(question.cancel).into(),
        tone: dialog::Tone::Accent,
        also: None,
    };
    let mut asked = false;
    if window == front().0 {
        update_app(|cx| {
            asked = true;
            let answered = Box::new(move |accept, _: &mut App| unsafe {
                bc::bc_extension_answer(id, accept)
            });
            dialog::ask_chrome(question, answered, cx);
        });
    }
    // With no UI to ask in, Chrome isn't left waiting.
    if !asked {
        unsafe { bc::bc_extension_answer(id, false) };
    }
}

unsafe extern "C" fn window_side_panel(
    _: *mut c_void,
    window: bc::bc_id,
    panel: *const bc::bc_side_panel,
) {
    let text = |text| unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into();
    let panel = unsafe { panel.as_ref() }.map(|panel| SidePanel {
        extension: text(panel.extension),
        name: text(panel.name),
        icon: (!panel.icon.is_null()).then(|| {
            let png = unsafe { std::slice::from_raw_parts(panel.icon, panel.icon_size) };
            Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec()))
        }),
        title: text(panel.title),
        width: panel.width as f32,
    });
    update_window(window, |shell, cx| shell.set_side_panel(panel, cx));
}

unsafe extern "C" fn window_extension_popup(
    _: *mut c_void,
    window: bc::bc_id,
    extension: *const c_char,
) {
    let extension = unsafe { CStr::from_ptr(extension) }.to_string_lossy();
    if WINDOWS.with_borrow(|windows| windows.iter().any(|open| open.id == window)) {
        // check.sh waits for these lines.
        match extension.is_empty() {
            true => log!("browser: extension popup none"),
            false => log!("browser: extension popup {extension}"),
        }
    }
}

unsafe extern "C" fn window_show_downloads(_: *mut c_void, window: bc::bc_id) {
    update_window(window, |shell, cx| shell.show_downloads(cx));
}

unsafe extern "C" fn page_favicon(_: *mut c_void, url: *const c_char, png: *const u8, size: usize) {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    let favicon = (!png.is_null()).then(|| {
        let png = unsafe { std::slice::from_raw_parts(png, size) }.to_vec();
        Arc::new(Image::from_bytes(ImageFormat::Png, png))
    });
    update_app(|cx| command_bar::page_favicon(&url, favicon.clone(), cx));
    update_shells(|shell, cx| shell.set_page_favicon(url.clone().into(), favicon.clone(), cx));
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
// the shell hears of it from space_changed instead, and Site Controls once that update is over.
unsafe extern "C" fn tab_closed(_: *mut c_void, tab: bc::bc_id) {
    log!("browser: tab {tab} closed");
    find_bar::forget(tab);
    if let Some(cx) = CX.with_borrow(Clone::clone) {
        let executor = cx.foreground_executor().clone();
        executor
            .spawn(async move {
                cx.update(|cx| {
                    site_controls::tab_closed(tab, cx);
                    capture::tab_closed(tab, cx);
                })
            })
            .detach();
    }
}

unsafe extern "C" fn tab_site_controls(
    _: *mut c_void,
    tab: bc::bc_id,
    controls: *const bc::bc_site_controls,
) {
    let controls = unsafe { &*controls };
    let string = |text: *const c_char| -> SharedString {
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned().into()
    };
    let value = |value: u32| match value {
        bc::BC_SITE_ALLOW => site_controls::Value::Allow,
        bc::BC_SITE_BLOCK => site_controls::Value::Block,
        _ => site_controls::Value::Ask,
    };
    let bit = |each: site_controls::Value| match each {
        site_controls::Value::Allow => bc::BC_SITE_ALLOW,
        site_controls::Value::Block => bc::BC_SITE_BLOCK,
        site_controls::Value::Ask => bc::BC_SITE_ASK,
    };
    let permissions = match controls.count {
        0 => &[][..],
        count => unsafe { std::slice::from_raw_parts(controls.permissions, count) },
    };
    let site = site_controls::Site {
        site: string(controls.site),
        connection: match controls.connection as u32 {
            bc::BC_CONNECTION_SECURE => site_controls::Connection::Secure,
            bc::BC_CONNECTION_INTERNAL => site_controls::Connection::Internal,
            _ => site_controls::Connection::NotSecure,
        },
        connection_summary: string(controls.connection_summary),
        certificate: controls.certificate,
        permissions: permissions
            .iter()
            .map(|permission| site_controls::Permission {
                kind: permission.type_,
                key: string(permission.key),
                name: string(permission.name),
                value: value(permission.value as u32),
                state: string(permission.state),
                choices: (site_controls::Value::ALL.into_iter())
                    .filter(|&each| permission.choices & (1 << bit(each)) != 0)
                    .collect(),
            })
            .collect(),
        reset: Some(string(controls.reset)).filter(|reset| !reset.is_empty()),
    };
    update_app(|cx| site_controls::changed(tab, site, cx));
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
    update_shells(|shell, cx| shell.set_question(tab, question.clone(), cx));
}

unsafe extern "C" fn tab_media_unsupported(
    _: *mut c_void,
    tab: bc::bc_id,
    key_system: *const c_char,
) {
    let key_system = unsafe { CStr::from_ptr(key_system) }.to_string_lossy().into_owned();
    // check.sh reads these lines.
    match key_system.is_empty() {
        true => log!("browser: page {tab} needs nothing"),
        false => log!("browser: page {tab} needs {key_system}"),
    }
    update_shells(|shell, cx| shell.set_media_unsupported(tab, &key_system, cx));
}

unsafe extern "C" fn window_suggestions(
    _: *mut c_void,
    window: bc::bc_id,
    text: *const c_char,
    suggestions: *const bc::bc_suggestion,
    count: usize,
) {
    // The command bar shows over the window last key.
    if window != front().0 {
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
    // check.sh times the steps from the core's "asked" line to the window's and the UI's.
    log!(
        "browser: full screen {} told at {:.1} ms",
        if fullscreen { "enter" } else { "exit" },
        browser::uptime_ms()
    );
    let Some(cx) = CX.with_borrow(Clone::clone) else {
        return;
    };
    cx.update(|cx| {
        let Some(BrowserWindow { id, shell, handle, .. }) = tab_window(tab, cx) else {
            return;
        };
        // A page leaves full screen as its tab is switched away from or closes, and only one page
        // is full screen at a time.
        let shown = shell.read_with(cx, |shell, _| shell.selected() == tab).unwrap_or(false);
        if fullscreen && !shown {
            return;
        }
        // Told before AppKit resizes a window going full screen with the page, or back, so the
        // resize lays the page out once, at its new size. Otherwise it's laid out now.
        cx.update_window(handle, |_, window, cx| {
            shell.update(cx, |shell, cx| {
                shell.set_page_fullscreen(fullscreen, window, cx);
                let viewport = window.viewport_size();
                if let Some(frame) = shell.window_page_frame(viewport) {
                    set_frames(id, shell, frame, viewport);
                }
            })
        })
        .ok();
    });
    log!(
        "browser: page {} full screen",
        if fullscreen { "entered" } else { "left" }
    );
}

/// Where Chromium keeps the profiles, and the app the user's keys. A dev build keeps a profile of
/// its own, never the owner's, since the checks delete what the last run saved. The prod build
/// keeps the user's in Application Support, under the product's name.
fn profile_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let name = include_str!("../branding/name").trim();
    std::env::var("BROWSER_PROFILE_DIR").unwrap_or_else(|_| {
        if cfg!(feature = "prod") {
            format!("{home}/Library/Application Support/{name}")
        } else {
            format!("{home}/Library/Caches/browser-app")
        }
    })
}

fn main() {
    let mut args: Vec<String> = std::env::args().take(1).collect();
    let profile = profile_dir();
    args.extend([
        format!("--user-data-dir={profile}"),
        // Chrome would copy the app bundle to check its signature against, which an
        // ad-hoc-signed build doesn't need.
        "--disable-features=MacAppCodeSignClone".into(),
        // An idle browser shouldn't wake: Chrome's stack sampling feeds metrics nobody receives,
        // and the allocator's memory reclaim backs off from every 4 s while there's little to give
        // back.
        "--disable-stack-profiler".into(),
        "--enable-features=PartitionAllocAdaptiveMemoryReclaimInterval".into(),
    ]);
    // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed dev build asking
    // for it gets a password prompt after every build, and every request that uses cookies waits.
    // The prod build is signed alike every time (tools/install.sh), so macOS asks once.
    if !cfg!(feature = "prod") {
        args.push("--use-mock-keychain".into());
    }
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
        launching: Some(launching),
        started: Some(started),
        stopping: Some(stopping),
        window_closed: Some(window_closed),
        private_session_ended: Some(private_session_ended),
        window_open_link: Some(window_open_link),
        space_changed: Some(space_changed),
        spaces_changed: Some(spaces_changed),
        favorites_changed: Some(favorites_changed),
        window_space_shown: Some(window_space_shown),
        window_selected: Some(window_selected),
        window_page_elsewhere: Some(window_page_elsewhere),
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
        window_extension_question: Some(window_extension_question),
        window_side_panel: Some(window_side_panel),
        tab_fullscreen_changed: Some(tab_fullscreen_changed),
        tab_find_changed: Some(tab_find_changed),
        tab_permission_asked: Some(tab_permission_asked),
        tab_site_controls: Some(tab_site_controls),
        tab_element_box: Some(tab_element_box),
        tab_captured: Some(tab_captured),
        window_suggestions: Some(window_suggestions),
        tab_key_before_page: Some(tab_key_before_page),
        tab_key_unhandled: Some(tab_key_unhandled),
        tab_media_unsupported: Some(tab_media_unsupported),
        ..Default::default()
    };
    let code = unsafe { bc::bc_main(argv.len() as c_int, argv.as_mut_ptr(), &host) };
    std::process::exit(code);
}
