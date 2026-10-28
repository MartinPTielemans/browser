//! The window buttons of platform/host.h on Linux. The browser window is Chromium's, without a frame
//! of its own, so the UI draws its buttons where AppKit's show (shell.rs), showing them asks nothing
//! of the host, and a press goes to the window through the core.

use std::ffi::c_void;

use crate::bc;

#[unsafe(no_mangle)]
pub extern "C" fn host_show_window_buttons(_: *mut c_void, _: bool) {}

#[unsafe(no_mangle)]
pub extern "C" fn host_press_window_button(ui_view: *mut c_void, button: i32) {
    let window = crate::window_with_view(ui_view);
    if window == 0 {
        return;
    }
    // After the UI's update that asked, since closing and full screen call back into it.
    crate::update_app_later(move |_| unsafe {
        match button {
            0 => bc::bc_window_close(window),
            1 => bc::bc_window_minimize(window),
            _ => bc::bc_window_toggle_full_screen(window),
        }
    });
}
