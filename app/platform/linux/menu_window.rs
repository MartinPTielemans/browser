//! ilo's own context menu's window (src/menu.rs) on Linux, as mac/menu_window.mm is on the Mac:
//! GPUI's Linux platform shows it over the window it opened for as one of its overlays
//! (overlays.rs), with the keyboard, and the menu's own window takes its pointer events. A press
//! outside or the keyboard going elsewhere is the user leaving it.
//!
//! shortcut: the pointer isn't the menu's first, so a press outside closes it and lands where it
//! was pressed too, and a right-click elsewhere doesn't open that menu in its place. It stays
//! inside its window, which it can't go past under Wayland.

use std::{
    cell::Cell,
    ffi::{c_char, c_void},
};

use gpui::{Bounds, point, px, size};

use crate::{bc, overlays};

struct Menu {
    view: *mut c_void,
    dismissed: extern "C" fn(),
    choose: extern "C" fn(*const c_char) -> bool,
}

thread_local! {
    static MENU: Cell<Option<&'static Menu>> = const { Cell::new(None) };
    /// The browser window it shows over and whether it lent that window's keyboard; None while
    /// hidden.
    static SHOWN: Cell<Option<(bc::bc_id, bool)>> = const { Cell::new(None) };
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_menu_window(
    view: *mut c_void,
    _pointer: extern "C" fn(kind: i32, x: f64, y: f64) -> bool,
    dismissed: extern "C" fn(),
    choose: extern "C" fn(*const c_char) -> bool,
) {
    MENU.set(Some(Box::leak(Box::new(Menu {
        view,
        dismissed,
        choose,
    }))));
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_menu_window(
    ui_view: *mut c_void,
    _in_reveal: bool,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
) {
    let Some(menu) = MENU.get() else { return };
    if SHOWN.get().is_none() {
        // Over the command bar, which has the window's keyboard lent already, the menu keeps it.
        let window = crate::window_with_view(ui_view);
        let lends = window != 0 && overlays::shown().is_empty();
        if lends {
            unsafe { bc::bc_window_overlay_takes_keyboard(window, true) };
        }
        SHOWN.set(Some((window, lends)));
    }
    let bounds = Bounds::new(
        point(px(left as f32), px(top as f32)),
        size(px(width as f32), px(height as f32)),
    );
    // The menu draws its panels' corners and shadows inside its window.
    gpui_linux::show_overlay(menu.view, ui_view, bounds, px(0.), true, left_menu);
    // check.sh reads it.
    eprintln!("browser: menu window at {left:.0},{top:.0} {width:.0}x{height:.0}");
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_menu_window() {
    let (Some(menu), Some((window, lent))) = (MENU.get(), SHOWN.take()) else {
        return;
    };
    gpui_linux::hide_overlay(menu.view, move || {
        if lent {
            unsafe { bc::bc_window_overlay_takes_keyboard(window, false) };
        }
    });
    eprintln!("browser: menu window hidden");
}

/// The window's content, in its own points: the menu stays inside it.
#[unsafe(no_mangle)]
pub extern "C" fn host_menu_area(ui_view: *mut c_void, _in_reveal: bool, area: *mut f64) {
    let content = gpui_linux::embedder_size(ui_view).unwrap_or_default();
    let edges = [0., 0., f64::from(content.width), f64::from(content.height)];
    // SAFETY: the caller passes four.
    unsafe { std::slice::from_raw_parts_mut(area, 4) }.copy_from_slice(&edges);
}

/// The menu's view while it shows, for the test driver (drive.rs).
pub(crate) fn shown() -> Option<*mut c_void> {
    SHOWN.get().and(MENU.get()).map(|menu| menu.view)
}

/// For the driver: chooses `title`, or "Submenu › Item", in the menu that shows. Returns whether
/// a menu was showing with that item.
pub(crate) fn choose(title: *const c_char) -> bool {
    shown().is_some() && MENU.get().is_some_and(|menu| (menu.choose)(title))
}

/// For the driver: closes the menu that shows, as the user leaving it. Returns whether one did.
pub(crate) fn dismiss() -> bool {
    let Some(view) = shown() else { return false };
    gpui_linux::leave_overlay(view);
    true
}

/// The user left the menu. GPUI says so once its events are dispatched, outside any update.
fn left_menu(_: *mut c_void) {
    if SHOWN.get().is_some()
        && let Some(menu) = MENU.get()
    {
        (menu.dismissed)();
    }
}
