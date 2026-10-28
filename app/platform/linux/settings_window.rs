//! The Settings window of platform/host.h on Linux: GPUI's window, which it opened unshown, shows as
//! a window of its own (gpui_linux::show_window), and hides keeping its place on X11.

use std::{cell::Cell, ffi::c_void};

thread_local! {
    static SETTINGS: Cell<*mut c_void> = const { Cell::new(std::ptr::null_mut()) };
    static SHOWN: Cell<bool> = const { Cell::new(false) };
    static PLACED: Cell<bool> = const { Cell::new(false) };
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_settings(settings_view: *mut c_void) {
    SETTINGS.set(settings_view);
}

/// Shows Settings in front with the keyboard. The first time it's centred over the browser window
/// last key, where the display lets the app place a window.
#[unsafe(no_mangle)]
pub extern "C" fn host_show_settings() {
    let view = SETTINGS.get();
    if view.is_null() {
        return;
    }
    let over = match PLACED.replace(true) {
        true => std::ptr::null_mut(),
        false => crate::front().1,
    };
    gpui_linux::show_window(view, over);
    SHOWN.set(true);
    // check.sh waits for this line.
    eprintln!("browser: settings shown");
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_settings() {
    if SHOWN.replace(false) {
        gpui_linux::hide_window(SETTINGS.get());
    }
}
