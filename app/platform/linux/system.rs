//! What the Linux host does through GPUI's Linux platform rather than in host.cc, as GPUI already
//! has it: the clipboard, the file chooser portal, the browser window's activation, the touchpad's
//! swipes over the page, Control's release and a file's drag, and on X11 a window's frame and the
//! screens'.

use std::{
    ffi::{CStr, c_char, c_void},
    path::PathBuf,
};

use browser::log;
use gpui::{ClipboardItem, Context, Image, ImageFormat, Window};

use crate::platform;

/// Puts the capture on the clipboard, as host_copy_png does on the Mac.
pub fn copy_png(png: &[u8]) {
    let image = Image::from_bytes(ImageFormat::Png, png.to_vec());
    // After the update under way, which the capture's menu or toast is in.
    crate::update_app_later(move |cx| {
        cx.write_to_clipboard(ClipboardItem::new_image(&image));
        // check.sh reads this line, as the Mac's pasteboard one.
        log!("browser: clipboard png {} bytes", image.bytes.len());
    });
}

/// Asks where to save the capture, through the desktop's file chooser portal, and saves it there.
pub fn save_png(png: &[u8]) {
    let mut name = [0 as c_char; 64];
    unsafe { platform::host_capture_name(name.as_mut_ptr(), name.len()) };
    let name = unsafe { CStr::from_ptr(name.as_ptr()) }.to_string_lossy().into_owned();
    let png = png.to_vec();
    crate::update_app_later(move |cx| {
        // The portal opens in the folder given, and GPUI asks for one; the home folder is always
        // there.
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let chosen = cx.prompt_for_new_path(&home, Some(&name));
        let background = cx.background_executor().clone();
        cx.foreground_executor()
            .spawn(async move {
                if let Ok(Ok(Some(path))) = chosen.await {
                    _ = background.spawn(async move { std::fs::write(path, png) }).await;
                }
            })
            .detach();
        // check.sh reads this line, as the Mac's.
        log!("browser: capture save panel shown");
    });
}

/// Tells the host each time the browser window of `ui_view`, whose UI `window` is, becomes active
/// or stops being, for its key window and active app watches. Under Wayland, `whole_window`, GPUI
/// has the keyboard the compositor gives the browser window.
pub fn watch_activation<T: 'static>(
    window: &mut Window,
    ui_view: *mut c_void,
    whole_window: bool,
    cx: &mut Context<T>,
) {
    cx.observe_window_activation(window, move |_, window, _| unsafe {
        platform::host_window_active(ui_view, window.is_window_active(), whole_window)
    })
    .detach();
}

/// The frame of the browser window of `ui_view`, as host.h says: Chromium's window, which the UI's
/// is in, in points from the screen's top-left corner, for the next launch to open at. None under
/// Wayland, which keeps windows' places from their apps, nor while the window is full screen.
#[unsafe(no_mangle)]
pub extern "C" fn host_window_frame(
    ui_view: *mut c_void,
    x: *mut f64,
    y: *mut f64,
    width: *mut f64,
    height: *mut f64,
) -> bool {
    let window = crate::WINDOWS.with_borrow(|windows| {
        windows.iter().find(|window| window.ui_view == ui_view).map(|window| window.id)
    });
    if window.is_some_and(|window| unsafe { crate::bc::bc_window_is_full_screen(window) }) {
        return false;
    }
    let Some(frame) = crate::drive::browser_frame(ui_view as usize as u32) else {
        return false;
    };
    unsafe { [*x, *y, *width, *height] = frame };
    true
}

/// The screens' visible frames, as host.h says, the primary first: none under Wayland, whose
/// compositor places windows itself (host_centered).
#[unsafe(no_mangle)]
pub extern "C" fn host_screens(frames: *mut f64, count: usize) -> usize {
    let screens = crate::drive::screens().unwrap_or_default();
    if !frames.is_null() {
        let frames = unsafe { std::slice::from_raw_parts_mut(frames, count * 4) };
        for (frame, screen) in frames.chunks_exact_mut(4).zip(&screens) {
            frame.copy_from_slice(screen);
        }
    }
    screens.len()
}

/// Where the next browser window goes after the one of `ui_view`, as host.h says: as big, a step
/// down and to the right, back to its screen's left or top edge where it would go past the other.
/// Left as it is under Wayland, whose compositor places windows itself (host_centered).
#[unsafe(no_mangle)]
pub extern "C" fn host_cascaded(
    ui_view: *mut c_void,
    x: *mut f64,
    y: *mut f64,
    width: *mut f64,
    height: *mut f64,
) {
    // About a header row, so both windows' tabs and buttons show.
    const STEP: f64 = 28.;
    let Some([left, top, w, h]) = crate::drive::browser_frame(ui_view as usize as u32) else {
        return;
    };
    let screens = crate::drive::screens().unwrap_or_default();
    let inside = |&&[sx, sy, sw, sh]: &&[f64; 4]| {
        left >= sx && left < sx + sw && top >= sy && top < sy + sh
    };
    let (mut next_x, mut next_y) = (left + STEP, top + STEP);
    if let Some(&[sx, sy, sw, sh]) = screens.iter().find(inside).or(screens.first()) {
        if next_x + w > sx + sw {
            next_x = sx;
        }
        if next_y + h > sy + sh {
            next_y = sy;
        }
    }
    unsafe { [*x, *y, *width, *height] = [next_x, next_y, w, h] };
}

/// Starts the system's drag of the file at `path`, as host.h says: XDND on X11, the data device's
/// drag under Wayland, from the press the left button holds.
#[unsafe(no_mangle)]
pub extern "C" fn host_drag_file(path: *const c_char, ended: extern "C" fn(taken: bool)) -> bool {
    let path = PathBuf::from(unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned());
    if !path.exists() {
        return false;
    }
    gpui_linux::start_file_drag(&path, move |taken| {
        // After the event that let it go, out of the display's handling.
        if let Some(cx) = crate::CX.with_borrow(Clone::clone) {
            cx.foreground_executor().spawn(async move { ended(taken) }).detach();
        }
    })
}

/// Calls `released` once Control is no longer held, after the event that shows it, as a ⌃Tab walk
/// ends: GPUI hears the keyboard's state whichever view has the keys.
#[unsafe(no_mangle)]
pub extern "C" fn host_watch_control(released: extern "C" fn()) {
    // After the update under way, which the walk's ⌃Tab asked from.
    gpui_linux::watch_control(move || crate::overlays::later(released));
}

/// A touchpad swipe over the page in the browser window whose Wayland surface is `surface`
/// (gpui_linux::watch_embedder_swipes).
pub fn swiped(surface: *mut c_void, x: f32, y: f32, back: bool) {
    crate::swiped(surface, x.into(), y.into(), back);
}
