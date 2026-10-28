//! Test driver for check.sh on Linux, as platform/mac/drive.mm is on the Mac. With BROWSER_DRIVE=1
//! the app reads one command per line on stdin, in drive.mm's language, and sends the matching X
//! events to its own windows only: Chromium's browser windows, and GPUI's UI and overlays in and
//! over them. Each window's own client handles them as it would the device's: Chromium takes core
//! events, and GPUI takes a sent core pointer event for the XInput 2 one it selects. So they go
//! through the same hit testing, focus changes and shortcuts as real input, without touching the
//! display's pointer or keyboard. Keys go to the window with the X server's input focus, as real
//! keys do, and the pointer's events to the window of the app's on top at that point.
//!
//! X11 only: under Wayland no client can hand another's surfaces input.
//! shortcut: a core call that hands Chromium's windows the events would drive Wayland too.
//!
//! The commands are drive.mm's, with what Linux has: click, doubleclick, rightclick, clickkey,
//! doubleclickkey, rightclickkey, drag, press, dragto, release, rightpress, rightdragto,
//! rightrelease, move, scroll, wheel, key, type, resize, place, order, layout, state, title, front,
//! menu, menubar, choose, dismiss, window, agentspace, agentmayact, agentrelease, agentdisconnect,
//! takeback and age. The Mac's modifiers are pressed as the app reads them off the Mac (keys.rs):
//! cmd with Ctrl, ctrl with Alt and alt with both. scroll turns the wheel a notch for each event, where the Mac's moves the content by points. The
//! others, such as ime, insert, flags, swipe, drags from other apps, full screen and displays, the
//! driver can't do yet, and says so as drive.mm does.
//!
//! It logs as drive.mm does: "browser: drive <n>: <command>" as command n arrives, "browser: drive
//! <n>: can't do that" for one it can't, and "browser: drive <n> done, key <window>, focus <view>,
//! active <0|1>, marked none" once Chromium and GPUI have handled its events. The window and view
//! are named as AppKit's classes are on the Mac, so the runs read one vocabulary: key is
//! BrowserCoreNSWindow for a browser window and GPUIPanel for an overlay, and focus
//! RenderWidgetHostViewCocoa for a page, AccessKitSubclassOfGPUIView for the UI and GPUIView for an
//! overlay's. Active is whether the input focus is in one of the app's windows.

use std::{
    cell::{Cell, RefCell},
    ffi::{CString, c_int, c_uint, c_void},
    io::BufRead,
    rc::Rc,
};

use browser::log;
use gpui::{Menu, MenuItem};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        self, ButtonPressEvent, ConfigureWindowAux, ConnectionExt as _, EnterNotifyEvent,
        EventMask, KeyButMask, KeyPressEvent, MotionNotifyEvent, StackMode,
    },
    rust_connection::RustConnection,
};

use crate::{BrowserWindow, CX, MENUS, WINDOWS, bc, overlays, platform};

unsafe extern "C" {
    // GLib's, from chrome, whose main loop is GLib's.
    fn g_idle_add(function: extern "C" fn(*mut c_void) -> c_int, data: *mut c_void) -> c_uint;
}

/// Starts reading commands from stdin.
pub fn start() {
    std::thread::spawn(|| {
        let mut number = 0;
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            number += 1;
            let n = number;
            idle(Box::new(move || {
                log!("browser: drive {n}: {line}");
                if !run(&line) {
                    log!("browser: drive {n}: can't do that");
                }
                settle(n);
            }));
        }
    });
}

/// Runs `task` on the main thread once nothing more urgent is ready, such as the X events Chromium
/// and GPUI have to read: GLib's idle priority is below their sources'.
fn idle(task: Box<dyn FnOnce() + Send>) {
    extern "C" fn run(data: *mut c_void) -> c_int {
        // SAFETY: the box `idle` leaked, which runs once.
        let task = unsafe { Box::from_raw(data as *mut Box<dyn FnOnce() + Send>) };
        task();
        0
    }
    unsafe { g_idle_add(run, Box::into_raw(Box::new(task)) as *mut c_void) };
}

/// Reports command `n` done once Chromium and GPUI have handled its events: once the X server has
/// handed them over, and the main loop has read them. Twice, as the server may write the windows'
/// clients' events after its answer to the driver's. GPUI's are handled as it syncs, since xcb
/// may have read them into its queue already, where they don't wake the loop.
fn settle(n: u64) {
    sync();
    gpui_linux::sync_display();
    idle(Box::new(move || {
        sync();
        gpui_linux::sync_display();
        idle(Box::new(move || {
            let (key, focus, active) = keyboard();
            log!(
                "browser: drive {n} done, key {key}, focus {focus}, active {}, marked none",
                active as u8
            );
        }));
    }));
}

thread_local! {
    /// The driver's own connection to the X server, with its root window.
    static X: RefCell<Option<Rc<(RustConnection, xproto::Window)>>> = const { RefCell::new(None) };
    /// Each browser window the app has made, in order, by its UI's X window (`host_ui_view`'s
    /// order on the Mac): the window command's numbers.
    static MADE: RefCell<Vec<(bc::bc_id, xproto::Window)>> = const { RefCell::new(Vec::new()) };
    /// The window command's, from 0.
    static DRIVEN: Cell<usize> = const { Cell::new(0) };
    /// The window a press went to, which gets the drag's events until the release, as the X
    /// server's grab of the pointer does.
    static PRESSED: Cell<Option<Target>> = const { Cell::new(None) };
    /// The window the pointer is in, which hears it leave.
    static HOVERED: Cell<Option<Target>> = const { Cell::new(None) };
    /// The last click in each window: its time, place and button.
    static LAST_CLICKS: RefCell<std::collections::HashMap<xproto::Window, (std::time::Instant, Target, u8)>> =
        RefCell::default();
    /// The keysyms on each key, from the server's first keycode, and how many each key has.
    static KEYMAP: RefCell<Option<(u8, usize, Vec<u32>)>> = const { RefCell::new(None) };
}

fn server() -> Option<Rc<(RustConnection, xproto::Window)>> {
    // Under Wayland the views are pointers, which as X ids could name another client's windows.
    let window = WINDOWS.with_borrow(|windows| windows.first().map(|w| w.id))?;
    let (mut width, mut height) = (0., 0.);
    let wayland = unsafe { bc::bc_window_wayland_surface(window, &mut width, &mut height) };
    wayland.is_null().then_some(())?;
    X.with_borrow_mut(|x| {
        if x.is_none() {
            let (connection, screen) = x11rb::connect(None).ok()?;
            let root = connection.setup().roots[screen].root;
            *x = Some(Rc::new((connection, root)));
        }
        x.clone()
    })
}

/// Waits for the X server to have handled what the driver sent, and drops the errors it answered
/// with, such as for a window that has closed since.
fn sync() {
    let Some(x) = server() else { return };
    let (connection, _) = &*x;
    _ = connection.get_input_focus().map(|cookie| cookie.reply());
    while let Ok(Some(_)) = connection.poll_for_event() {}
}

/// A browser window, as the X server has it.
#[derive(Clone, Copy)]
struct Browser {
    id: bc::bc_id,
    /// Chromium's window.
    window: xproto::Window,
    /// The app's UI's window in it, the view the core holds (`host_ui_view`), and GPUI's window,
    /// which fills it.
    ui: xproto::Window,
    gpui: xproto::Window,
}

/// The browser window the app made `index`th, from 0, while it's open.
fn browser(index: usize) -> Option<Browser> {
    let open: Vec<(bc::bc_id, xproto::Window)> = WINDOWS.with_borrow(|windows| {
        windows
            .iter()
            .map(|w: &BrowserWindow| (w.id, w.ui_view as usize as u32))
            .collect()
    });
    let (id, ui) = MADE.with_borrow_mut(|made| {
        let mut new: Vec<_> = (open.iter())
            .filter(|each| !made.contains(each))
            .copied()
            .collect();
        new.sort();
        made.extend(new);
        made.get(index).copied()
    })?;
    open.contains(&(id, ui)).then_some(())?;
    let x = server()?;
    let (connection, _) = &*x;
    // Under Wayland the view is a surface, which the X server knows nothing of.
    let tree = connection.query_tree(ui).ok()?.reply().ok()?;
    Some(Browser {
        id,
        window: tree.parent,
        ui,
        gpui: *tree.children.first()?,
    })
}

fn driven() -> Option<Browser> {
    browser(DRIVEN.get())
}

/// Whether `window` is `ancestor` or inside it.
fn within(
    connection: &RustConnection,
    mut window: xproto::Window,
    ancestor: xproto::Window,
) -> bool {
    while window != x11rb::NONE {
        if window == ancestor {
            return true;
        }
        let Some(tree) = connection
            .query_tree(window)
            .ok()
            .and_then(|c| c.reply().ok())
        else {
            return false;
        };
        if tree.parent == tree.root {
            return false;
        }
        window = tree.parent;
    }
    false
}

/// The overlay or the browser window the X server's input focus is in, if it's the app's.
enum Keyboard {
    Overlay(xproto::Window),
    Browser(Browser, xproto::Window),
}

fn keyboard_window() -> Option<Keyboard> {
    let x = server()?;
    let (connection, _) = &*x;
    let focus = connection.get_input_focus().ok()?.reply().ok()?.focus;
    for view in overlays::shown() {
        if within(connection, focus, view as usize as u32) {
            return Some(Keyboard::Overlay(focus));
        }
    }
    let count = MADE.with_borrow(Vec::len).max(1);
    (0..count + WINDOWS.with_borrow(Vec::len))
        .filter_map(browser)
        .find(|browser| within(connection, focus, browser.window))
        .map(|browser| Keyboard::Browser(browser, focus))
}

/// The done line's key window, focus and whether the app is active.
fn keyboard() -> (&'static str, &'static str, bool) {
    match keyboard_window() {
        Some(Keyboard::Overlay(_)) => ("GPUIPanel", "GPUIView", true),
        Some(Keyboard::Browser(browser, _)) => {
            match unsafe { bc::bc_window_keyboard_is_ui(browser.id) } {
                true => ("BrowserCoreNSWindow", "AccessKitSubclassOfGPUIView", true),
                false => ("BrowserCoreNSWindow", "RenderWidgetHostViewCocoa", true),
            }
        }
        None => ("none", "none", false),
    }
}

/// Where a pointer event goes: a window of the app's, and the point in its pixels and the root's.
#[derive(Clone, Copy, PartialEq)]
struct Target {
    window: xproto::Window,
    x: i16,
    y: i16,
    root_x: i16,
    root_y: i16,
}

fn scale() -> f64 {
    unsafe { bc::bc_display_scale() }
}

/// The point (`x`, `y`), in points from `window`'s top-left corner, in `window`.
fn target_in(window: xproto::Window, x: f64, y: f64) -> Option<Target> {
    let connection = server()?;
    let (connection, root) = &*connection;
    let (x, y) = ((x * scale()).round() as i16, (y * scale()).round() as i16);
    let at = connection
        .translate_coordinates(window, *root, x, y)
        .ok()?
        .reply()
        .ok()?;
    Some(Target {
        window,
        x,
        y,
        root_x: at.dst_x,
        root_y: at.dst_y,
    })
}

/// The window of the app's that the pointer at (`x`, `y`) in the driven browser window is over, as
/// the X server picks it: an overlay over the window, then GPUI's window, but where its shape
/// leaves holes for the pages, which are Chromium's window's.
fn target_at(x: f64, y: f64) -> Option<Target> {
    let browser = driven()?;
    let at = target_in(browser.window, x, y)?;
    let connection = server()?;
    let (connection, root) = &*connection;
    let top = connection
        .translate_coordinates(*root, *root, at.root_x, at.root_y)
        .ok()?
        .reply()
        .ok()?;
    if let Some(view) = overlays::shown()
        .into_iter()
        .find(|&view| view as usize as u32 == top.child)
    {
        let view = view as usize as u32;
        let inside = connection
            .translate_coordinates(*root, view, at.root_x, at.root_y)
            .ok()?
            .reply()
            .ok()?;
        return Some(Target {
            window: view,
            x: inside.dst_x,
            y: inside.dst_y,
            ..at
        });
    }
    let child = (connection.translate_coordinates(browser.window, browser.window, at.x, at.y))
        .ok()?
        .reply()
        .ok()?
        .child;
    Some(match child == browser.ui {
        true => Target {
            window: browser.gpui,
            ..at
        },
        false => at,
    })
}

/// The point (`x`, `y`) in points from the key window's top-left corner, such as the command
/// bar's, or the driven browser window's.
fn target_in_key(x: f64, y: f64) -> Option<Target> {
    match keyboard_window() {
        Some(Keyboard::Overlay(window)) => target_in(window, x, y),
        _ => target_at(x, y),
    }
}

const BUTTON_MASKS: [u16; 3] = [1 << 8, 1 << 9, 1 << 10];

/// Moves the pointer into `to`, telling the window it was in that it left, and `to` that it came.
fn hover(to: Target, state: u16) -> Option<()> {
    let x = server()?;
    let (connection, root) = &*x;
    let from = HOVERED.replace(Some(to));
    if from.map(|from| from.window) == Some(to.window) {
        return Some(());
    }
    let crossing = |kind: u8, at: Target| EnterNotifyEvent {
        response_type: kind,
        detail: xproto::NotifyDetail::NONLINEAR,
        sequence: 0,
        time: x11rb::CURRENT_TIME,
        root: *root,
        event: at.window,
        child: x11rb::NONE,
        root_x: at.root_x,
        root_y: at.root_y,
        event_x: at.x,
        event_y: at.y,
        state: KeyButMask::from(state),
        mode: xproto::NotifyMode::NORMAL,
        same_screen_focus: 1,
    };
    if let Some(from) = from {
        let from = Target {
            root_x: to.root_x,
            root_y: to.root_y,
            ..from
        };
        let event = crossing(xproto::LEAVE_NOTIFY_EVENT, from);
        connection
            .send_event(false, from.window, EventMask::NO_EVENT, event)
            .ok()?;
    }
    let event = crossing(xproto::ENTER_NOTIFY_EVENT, to);
    connection
        .send_event(false, to.window, EventMask::NO_EVENT, event)
        .ok()?;
    Some(())
}

/// Sends a press of `button`, 1 the left, 3 the right and 4 to 7 the wheel's, or its release, at
/// `at`, with `state` the modifiers and the buttons held before it.
fn button(at: Target, button: u8, press: bool, state: u16) -> Option<()> {
    hover(at, state)?;
    let x = server()?;
    let (connection, root) = &*x;
    let event = ButtonPressEvent {
        response_type: match press {
            true => xproto::BUTTON_PRESS_EVENT,
            false => xproto::BUTTON_RELEASE_EVENT,
        },
        detail: button,
        sequence: 0,
        time: x11rb::CURRENT_TIME,
        root: *root,
        event: at.window,
        child: x11rb::NONE,
        root_x: at.root_x,
        root_y: at.root_y,
        event_x: at.x,
        event_y: at.y,
        state: KeyButMask::from(state),
        same_screen: true,
    };
    connection
        .send_event(false, at.window, EventMask::NO_EVENT, event)
        .ok()?;
    connection.flush().ok()
}

fn motion(at: Target, state: u16) -> Option<()> {
    hover(at, state)?;
    let x = server()?;
    let (connection, root) = &*x;
    let event = MotionNotifyEvent {
        response_type: xproto::MOTION_NOTIFY_EVENT,
        detail: xproto::Motion::NORMAL,
        sequence: 0,
        time: x11rb::CURRENT_TIME,
        root: *root,
        event: at.window,
        child: x11rb::NONE,
        root_x: at.root_x,
        root_y: at.root_y,
        event_x: at.x,
        event_y: at.y,
        state: KeyButMask::from(state),
        same_screen: true,
    };
    connection
        .send_event(false, at.window, EventMask::NO_EVENT, event)
        .ok()?;
    connection.flush().ok()
}

/// A click of `which` (1 the left, 3 the right, 4 to 7 the wheel's) at `at`, the second of a
/// double click with `again`. A first click waits until GPUI wouldn't count it with the last
/// one, as the driver clicks faster than a hand does.
fn click(at: Target, which: u8, state: u16, again: bool) -> Option<()> {
    if which <= 3 && !again {
        let last = LAST_CLICKS.with_borrow(|clicks| clicks.get(&at.window).copied());
        if let Some((then, last, last_which)) = last
            && last_which == which
            && (last.x - at.x).abs() <= 5 * scale().ceil() as i16
            && (last.y - at.y).abs() <= 5 * scale().ceil() as i16
        {
            // GPUI's DOUBLE_CLICK_INTERVAL, with a margin.
            std::thread::sleep(
                std::time::Duration::from_millis(450).saturating_sub(then.elapsed()),
            );
        }
    }
    if which <= 3 {
        let now = std::time::Instant::now();
        LAST_CLICKS.with_borrow_mut(|clicks| clicks.insert(at.window, (now, at, which)));
    }
    button(at, which, true, state)?;
    button(
        at,
        which,
        false,
        state | BUTTON_MASKS[usize::from(which - 1)],
    )
}

/// The Mac's modifiers named in `words`, cmd, shift, alt and ctrl, as an X state holds them on
/// the keys that press them off the Mac.
fn modifiers<'a>(words: impl Iterator<Item = &'a str>) -> u16 {
    let (ctrl, alt) = (u16::from(KeyButMask::CONTROL), u16::from(KeyButMask::MOD1));
    words.fold(0, |state, word| {
        state
            | match word {
                "shift" => u16::from(KeyButMask::SHIFT),
                "cmd" => ctrl,
                "ctrl" => alt,
                "alt" => ctrl | alt,
                _ => 0,
            }
    })
}

/// The keysym of drive.mm's key `key`: a printable ASCII character, or a named key.
fn keysym(key: &str) -> Option<u32> {
    Some(match key {
        "return" => 0xff0d,
        "escape" => 0xff1b,
        "tab" => 0xff09,
        "space" => 0x20,
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "pageup" => 0xff55,
        "pagedown" => 0xff56,
        "up" => 0xff52,
        "down" => 0xff54,
        "left" => 0xff51,
        "right" => 0xff53,
        "f10" => 0xffc7,
        "f12" => 0xffc9,
        _ => {
            let mut characters = key.chars();
            match (characters.next(), characters.next()) {
                (Some(c), None) if c.is_ascii_graphic() || c == ' ' => c as u32,
                _ => return None,
            }
        }
    })
}

/// The key that types `keysym` on the server's keyboard, and whether with shift.
fn keycode(keysym: u32) -> Option<(u8, bool)> {
    KEYMAP.with_borrow_mut(|keymap| {
        if keymap.is_none() {
            let x = server()?;
            let (connection, _) = &*x;
            let (first, last) = (
                connection.setup().min_keycode,
                connection.setup().max_keycode,
            );
            let map = connection
                .get_keyboard_mapping(first, last - first + 1)
                .ok()?
                .reply()
                .ok()?;
            *keymap = Some((first, usize::from(map.keysyms_per_keycode), map.keysyms));
        }
        let (first, per, keysyms) = keymap.as_ref()?;
        (0..keysyms.len() / per).find_map(|key| {
            let level = (0..(*per).min(2)).find(|&level| keysyms[key * per + level] == keysym)?;
            Some((*first + key as u8, level == 1))
        })
    })
}

/// Presses and releases `key` with the modifiers `state` in the window with the keyboard, which is
/// the driven browser window's unless the input focus is in one of the app's.
fn key(key: &str, mut state: u16) -> Option<()> {
    let (code, shift) = keycode(keysym(key)?)?;
    if shift {
        state |= u16::from(KeyButMask::SHIFT);
    }
    let x = server()?;
    let (connection, root) = &*x;
    // As the X server picks it: the window under the pointer when that's in the focus window, else
    // the focus window.
    let window = match keyboard_window() {
        Some(Keyboard::Overlay(focus) | Keyboard::Browser(_, focus)) => (HOVERED.get())
            .map(|hovered| hovered.window)
            .filter(|&hovered| within(connection, hovered, focus))
            .unwrap_or(focus),
        None => driven()?.window,
    };
    for kind in [xproto::KEY_PRESS_EVENT, xproto::KEY_RELEASE_EVENT] {
        let event = KeyPressEvent {
            response_type: kind,
            detail: code,
            sequence: 0,
            time: x11rb::CURRENT_TIME,
            root: *root,
            event: window,
            child: x11rb::NONE,
            root_x: 0,
            root_y: 0,
            event_x: 0,
            event_y: 0,
            state: KeyButMask::from(state),
            same_screen: true,
        };
        connection
            .send_event(false, window, EventMask::NO_EVENT, event)
            .ok()?;
    }
    connection.flush().ok()
}

/// Two numbers from `words`.
fn point<'a>(words: &mut impl Iterator<Item = &'a str>) -> Option<(f64, f64)> {
    Some((words.next()?.parse().ok()?, words.next()?.parse().ok()?))
}

/// The browser window `window` is, as order and front name it: "window <n>", numbered as the
/// window command numbers them.
fn name_of(browser: &Browser) -> String {
    let index = MADE.with_borrow(|made| made.iter().position(|&(id, _)| id == browser.id));
    index.map_or_else(
        || "none".to_string(),
        |index| format!("window {}", index + 1),
    )
}

/// The browser windows that are open, front to back, as the X server stacks them.
fn stacked() -> Option<Vec<Browser>> {
    let x = server()?;
    let (connection, root) = &*x;
    let browsers: Vec<Browser> = (0..MADE.with_borrow(Vec::len) + WINDOWS.with_borrow(Vec::len))
        .filter_map(browser)
        .collect();
    let tops = connection.query_tree(*root).ok()?.reply().ok()?.children;
    Some(
        (tops.iter().rev())
            .filter_map(|&top| {
                browsers
                    .iter()
                    .find(|b| within(connection, b.window, top))
                    .copied()
            })
            .collect(),
    )
}

/// `window`'s frame in points, from the root's top-left corner: x, y, width and height.
fn frame(window: xproto::Window) -> Option<[f64; 4]> {
    let x = server()?;
    let (connection, root) = &*x;
    let geometry = connection.get_geometry(window).ok()?.reply().ok()?;
    let at = connection
        .translate_coordinates(window, *root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    let scale = scale();
    Some(
        [
            at.dst_x.into(),
            at.dst_y.into(),
            geometry.width.into(),
            geometry.height.into(),
        ]
        .map(|n: f64| n / scale),
    )
}

/// The text property `name` of `window`, as UTF-8.
fn text_property(window: xproto::Window, name: &str) -> Option<String> {
    let x = server()?;
    let (connection, _) = &*x;
    let atom = connection
        .intern_atom(false, name.as_bytes())
        .ok()?
        .reply()
        .ok()?
        .atom;
    let property = (connection.get_property(false, window, atom, xproto::AtomEnum::ANY, 0, 1024))
        .ok()?
        .reply()
        .ok()?;
    String::from_utf8(property.value).ok()
}

/// Whether the window manager has `window` full screen.
fn full_screen(window: xproto::Window) -> Option<bool> {
    let x = server()?;
    let (connection, _) = &*x;
    let [state, full] = ["_NET_WM_STATE", "_NET_WM_STATE_FULLSCREEN"].map(|name| {
        connection
            .intern_atom(false, name.as_bytes())
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    });
    let property = (connection.get_property(false, window, state?, xproto::AtomEnum::ATOM, 0, 64))
        .ok()?
        .reply()
        .ok()?;
    let full = full?;
    Some(property.value32()?.any(|atom| atom == full))
}

/// The item of `menus` titled `title`, in a submenu too, or with "Submenu › Item", the item in the
/// submenu so titled.
fn menu_item<'a>(menus: &'a [Menu], title: &str) -> Option<&'a MenuItem> {
    let (parent, item_title) = title
        .split_once(" › ")
        .map_or((None, title), |(p, i)| (Some(p), i));
    menus
        .iter()
        .find_map(|menu| find_item(&menu.items, parent, item_title, parent.is_none()))
}

fn find_item<'a>(
    items: &'a [MenuItem],
    parent: Option<&str>,
    title: &str,
    inside: bool,
) -> Option<&'a MenuItem> {
    items.iter().find_map(|item| match item {
        MenuItem::Action { name, .. } if inside && name.as_ref() == title => Some(item),
        MenuItem::Submenu(menu) => {
            let inside = inside || parent == Some(menu.name.as_ref());
            find_item(&menu.items, parent, title, inside)
        }
        _ => None,
    })
}

/// Runs a command. Returns false for one it can't do.
fn run(line: &str) -> bool {
    run_command(line).is_some()
}

fn run_command(line: &str) -> Option<()> {
    let mut words = line.split_whitespace();
    let action = words.next()?;
    // What follows the command's name, for those that take text.
    let rest = line.get(action.len() + 1..).unwrap_or("");
    let none = 0;
    match action {
        "window" => {
            let number: usize = words.next()?.parse().ok()?;
            let browser = browser(number.checked_sub(1)?)?;
            DRIVEN.set(number - 1);
            log!("browser: drive window {number}");
            // To the front with the keyboard, as a click on it does.
            let x = server()?;
            let (connection, _) = &*x;
            let above = ConfigureWindowAux::new().stack_mode(StackMode::ABOVE);
            connection.configure_window(browser.window, &above).ok()?;
            let focus = xproto::InputFocus::PARENT;
            connection
                .set_input_focus(focus, browser.window, x11rb::CURRENT_TIME)
                .ok()?;
            connection.flush().ok()
        }
        "click" | "rightclick" | "doubleclick" => {
            let (x, y) = point(&mut words)?;
            let at = target_at(x, y)?;
            let which = if action == "rightclick" { 3 } else { 1 };
            click(at, which, modifiers(words), false)?;
            if action == "doubleclick" {
                click(at, 1, none, true)?;
            }
            Some(())
        }
        "clickkey" | "rightclickkey" | "doubleclickkey" => {
            let (x, y) = point(&mut words)?;
            let at = target_in_key(x, y)?;
            click(
                at,
                if action == "rightclickkey" { 3 } else { 1 },
                none,
                false,
            )?;
            if action == "doubleclickkey" {
                click(at, 1, none, true)?;
            }
            Some(())
        }
        "drag" => {
            let (x, y) = point(&mut words)?;
            let (x2, y2) = point(&mut words)?;
            let at = target_at(x, y)?;
            let to = Target {
                window: at.window,
                ..target_in(driven()?.window, x2, y2)?
            };
            let to = relative_to(to, at.window)?;
            button(at, 1, true, none)?;
            motion(to, BUTTON_MASKS[0])?;
            button(to, 1, false, BUTTON_MASKS[0])
        }
        "press" | "dragto" | "release" | "rightpress" | "rightdragto" | "rightrelease" => {
            let (x, y) = point(&mut words)?;
            let (which, step) = match action.strip_prefix("right") {
                Some(step) => (3, step),
                None => (1, action),
            };
            let held = BUTTON_MASKS[usize::from(which - 1)];
            if step == "press" {
                let at = target_at(x, y)?;
                PRESSED.set(Some(at));
                return button(at, which, true, none);
            }
            let to = target_in(driven()?.window, x, y)?;
            let to = match PRESSED.get() {
                Some(pressed) => relative_to(to, pressed.window)?,
                None => target_at(x, y)?,
            };
            if step == "dragto" {
                return motion(to, held);
            }
            PRESSED.set(None);
            button(to, which, false, held)
        }
        "move" => {
            let (x, y) = point(&mut words)?;
            motion(target_at(x, y)?, none)
        }
        // A notch of the wheel for each event, down for content moving up.
        "scroll" => {
            let (x, y) = point(&mut words)?;
            let dy: f64 = words.next()?.parse().ok()?;
            let count: usize = words.next()?.parse().ok()?;
            let at = target_at(x, y)?;
            for _ in 0..count.max(1) {
                click(at, if dy > 0. { 5 } else { 4 }, none, false)?;
            }
            Some(())
        }
        "wheel" => {
            let (x, y) = point(&mut words)?;
            let dx: f64 = words.next()?.parse().ok()?;
            let at = target_at(x, y)?;
            for _ in 0..(dx.abs().round() as usize).max(1) {
                click(at, if dx > 0. { 7 } else { 6 }, none, false)?;
            }
            Some(())
        }
        "key" => {
            let name = words.next()?;
            key(name, modifiers(words))
        }
        "type" => rest.chars().try_for_each(|c| key(&c.to_string(), none)),
        "resize" | "place" => {
            let browser = driven()?;
            let scale = scale();
            let numbers: Vec<f64> = words.map(|w| w.parse().ok()).collect::<Option<_>>()?;
            let aux = match (action, numbers.as_slice()) {
                ("resize", &[_, _]) => ConfigureWindowAux::new(),
                ("place", &[x, y, _, _]) => ConfigureWindowAux::new()
                    .x((x * scale) as i32)
                    .y((y * scale) as i32),
                _ => return None,
            };
            let [width, height] = [numbers[numbers.len() - 2], numbers[numbers.len() - 1]];
            let aux = aux
                .width((width * scale) as u32)
                .height((height * scale) as u32);
            let x = server()?;
            x.0.configure_window(browser.window, &aux).ok()?;
            x.0.flush().ok()
        }
        "order" => {
            let stacked = stacked()?;
            let mut order = String::new();
            for browser in &stacked {
                let name = name_of(browser);
                order += &format!(" {name}");
                let [x, y, width, height] = frame(browser.window)?;
                log!("browser: frame {name} {x:.0} {y:.0} {width:.0} {height:.0}");
            }
            let key = match keyboard_window() {
                Some(Keyboard::Browser(browser, _)) => name_of(&browser),
                _ => "none".to_string(),
            };
            log!("browser: order{order}, key {key}");
            Some(())
        }
        "front" => {
            let driven = driven().map(|b| b.id);
            let describe = |browser: Option<Browser>| match browser {
                Some(b) if Some(b.id) == driven => "driven".to_string(),
                Some(b) => name_of(&b),
                None => "none".to_string(),
            };
            let first = describe(stacked()?.first().copied());
            let key = match keyboard_window() {
                Some(Keyboard::Browser(browser, _)) => describe(Some(browser)),
                _ => "none".to_string(),
            };
            log!("browser: front {first}, main {key}, key {key}, ax focused {key}, ax main {key}");
            Some(())
        }
        // The window's content and the views the X server has in it: GPUI's, and the overlays
        // over it. shortcut: the pages are aura windows in Chromium's window, which the X server
        // doesn't see, so they aren't listed.
        "layout" => {
            let browser = driven()?;
            let [_, _, width, height] = frame(browser.window)?;
            log!(
                "browser: layout content {width:.0}x{height:.0}, GPUIView 0,0 {width:.0}x{height:.0}"
            );
            log!("browser: layout buttons hidden hidden hidden");
            let [left, top, ..] = frame(browser.window)?;
            for view in overlays::shown() {
                let [x, y, width, height] = frame(view as usize as u32)?;
                log!(
                    "browser: layout child {:.0},{:.0} {width:.0}x{height:.0}",
                    x - left,
                    y - top
                );
            }
            log!(
                "browser: layout full screen {}",
                full_screen(browser.window)? as u8
            );
            Some(())
        }
        "state" => Some(()),
        "title" => {
            let window = match keyboard_window()? {
                Keyboard::Overlay(window) => window,
                Keyboard::Browser(browser, _) => browser.window,
            };
            let title = text_property(window, "_NET_WM_NAME").unwrap_or_default();
            log!("browser: key window titled {title}");
            Some(())
        }
        // The app's menus, which Linux shows no bar for, run in the driven window as the Mac's
        // menu bar runs them in the key one.
        "menu" | "menubar" => {
            let menus = MENUS.with_borrow(|menus| menus.build());
            if action == "menubar" {
                let menu = menus.iter().find(|menu| menu.name.as_ref() == rest)?;
                let mut titles = Vec::new();
                let mut add = |item: &MenuItem, prefix: &str| {
                    if let MenuItem::Action {
                        name,
                        checked,
                        disabled,
                        ..
                    } = item
                    {
                        let mut title = format!("{prefix}{name}");
                        title += if *checked { " (checked)" } else { "" };
                        title += if *disabled { " (disabled)" } else { "" };
                        titles.push(title);
                    }
                };
                for item in &menu.items {
                    match item {
                        MenuItem::Submenu(sub) => {
                            let prefix = format!("{} › ", sub.name);
                            sub.items.iter().for_each(|inner| add(inner, &prefix));
                        }
                        _ => add(item, ""),
                    }
                }
                log!("browser: menu bar {rest}: {}", titles.join(", "));
                return Some(());
            }
            let MenuItem::Action { action, .. } = menu_item(&menus, rest)? else {
                return None;
            };
            let handle = WINDOWS.with_borrow(|windows| {
                let id = driven()?.id;
                windows.iter().find(|w| w.id == id).map(|w| w.handle)
            })?;
            let cx = CX.with_borrow(Clone::clone)?;
            cx.update(|cx| {
                handle.update(cx, |_, window, cx| {
                    window.dispatch_action(action.boxed_clone(), cx)
                })
            })
            .ok()
        }
        // An alert's button; the app's menus don't show on Linux yet, so there are none to choose
        // from or dismiss.
        "choose" => {
            let title = CString::new(rest).ok()?;
            unsafe { platform::host_choose(title.as_ptr()) }.then_some(())
        }
        "agentspace" => {
            let agent = words.next()?;
            let name = rest.get(agent.len() + 1..)?.trim();
            (!name.is_empty()).then_some(())?;
            let (agent_c, name_c) = (CString::new(agent).ok()?, CString::new(name).ok()?);
            let space = unsafe {
                bc::bc_space_create_for_agent(name_c.as_ptr(), agent_c.as_ptr(), name_c.as_ptr())
            };
            log!("browser: made space {space} for agent {agent}");
            Some(())
        }
        "agentmayact" => {
            let agent = words.next()?;
            let tab: bc::bc_id = words.next()?.parse().ok()?;
            let what = words.next().unwrap_or("act");
            let (agent_c, what_c) = (CString::new(agent).ok()?, CString::new(what).ok()?);
            let mut reason = [0 as std::ffi::c_char; 256];
            let may = unsafe {
                bc::bc_agent_may_act(
                    agent_c.as_ptr(),
                    tab,
                    what_c.as_ptr(),
                    reason.as_mut_ptr(),
                    reason.len(),
                )
            };
            log!("browser: agent {agent} may act on {tab}: {}", may as u8);
            if !may {
                let reason = unsafe { std::ffi::CStr::from_ptr(reason.as_ptr()) }.to_string_lossy();
                log!("browser: agent {agent} refused {tab}: {reason}");
            }
            Some(())
        }
        "agentrelease" => {
            let agent = words.next()?;
            let tab: bc::bc_id = words.next()?.parse().ok()?;
            let agent_c = CString::new(agent).ok()?;
            let released = unsafe { bc::bc_agent_release_tab(agent_c.as_ptr(), tab) };
            log!("browser: agent {agent} released {tab}: {}", released as u8);
            Some(())
        }
        "agentdisconnect" => {
            let agent = words.next()?;
            let agent_c = CString::new(agent).ok()?;
            let ended = unsafe { bc::bc_agent_disconnected(agent_c.as_ptr()) };
            log!("browser: agent {agent} disconnected, {ended} sessions ended");
            Some(())
        }
        "takeback" => {
            let tab: bc::bc_id = words.next()?.parse().ok()?;
            let took = unsafe { bc::bc_tab_take_back(tab) };
            log!("browser: took back {tab}: {}", took as u8);
            Some(())
        }
        "age" => {
            let seconds: f64 = words.next()?.parse().ok()?;
            unsafe { bc::bc_age_tabs_for_testing(seconds) };
            Some(())
        }
        _ => None,
    }
}

/// `to`, a point in the root's terms, in `window`, which a press grabbed the pointer for.
fn relative_to(to: Target, window: xproto::Window) -> Option<Target> {
    let x = server()?;
    let (connection, root) = &*x;
    let at = connection
        .translate_coordinates(*root, window, to.root_x, to.root_y)
        .ok()?
        .reply()
        .ok()?;
    Some(Target {
        window,
        x: at.dst_x,
        y: at.dst_y,
        ..to
    })
}
