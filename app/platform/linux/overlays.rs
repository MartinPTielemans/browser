//! The overlays of platform/host.h on Linux: GPUI's Linux platform shows each over the browser
//! window as a child of it, a subsurface under Wayland and a child window on X11, and the core
//! lends it the window's keyboard while it shows.
//!
//! shortcut: no blur, shadow or growth yet.

use std::{
    cell::RefCell,
    ffi::{c_int, c_void},
};

use gpui::{Bounds, Pixels, point, px, size};

use crate::bc;

struct Overlay {
    view: *mut c_void,
    radius: f64,
    dismissed: extern "C" fn(),
    /// The height its UI asked for last, which it shows at; else its window's.
    height: Option<Pixels>,
    shown: Option<Shown>,
}

struct Shown {
    window: bc::bc_id,
    bounds: Bounds<Pixels>,
}

thread_local! {
    static OVERLAYS: RefCell<[Option<Overlay>; 2]> = const { RefCell::new([None, None]) };
}

/// The overlays by index, as check.sh reads them in the log.
const NAMES: [&str; 2] = ["command bar", "icon picker"];

fn with_overlay<R>(overlay: c_int, f: impl FnOnce(&mut Overlay) -> R) -> Option<R> {
    OVERLAYS.with_borrow_mut(|overlays| overlays.get_mut(overlay as usize)?.as_mut().map(f))
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_overlay(
    overlay: c_int,
    view: *mut c_void,
    radius: f64,
    dismissed: extern "C" fn(),
) {
    OVERLAYS.with_borrow_mut(|overlays| {
        if let Some(slot) = overlays.get_mut(overlay as usize) {
            *slot = Some(Overlay {
                view,
                radius,
                dismissed,
                height: None,
                shown: None,
            });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_radius(overlay: c_int, radius: f64) {
    with_overlay(overlay, |o| o.radius = radius);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_shape(overlay: c_int, corners: *const f64, count: usize) {
    if corners.is_null() {
        return;
    }
    // SAFETY: the caller passes `count` corners of three numbers each.
    let numbers = unsafe { std::slice::from_raw_parts(corners, count * 3) };
    let corners: Vec<gpui_linux::Corner> = (numbers.chunks_exact(3))
        .map(|corner| (corner[0] as f32, corner[1] as f32, corner[2] as f32))
        .collect();
    if let Some(view) = with_overlay(overlay, |o| o.view) {
        gpui_linux::set_overlay_outline(view, &corners);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_growth(_overlay: c_int, _x: f64, _y: f64) {}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_shrink(_overlay: c_int) {}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_elevation(
    _overlay: c_int,
    _outline_opacity: f32,
    _outline_radius: f64,
    _shadow_opacity: f32,
    _shadow_radius: f64,
    _shadow_drop: f64,
) {
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_overlay(
    overlay: c_int,
    ui_view: *mut c_void,
    centered: bool,
    left: f64,
    top: f64,
    width: f64,
) {
    // The other overlay closes, as the user leaving it.
    if hide(1 - overlay) {
        with_overlay(1 - overlay, |other| later(other.dismissed));
    }
    let Some(content) = gpui_linux::embedder_size(ui_view) else {
        return;
    };
    let (content_width, content_height) = (f64::from(content.width), f64::from(content.height));
    let (left, top, width) = match centered {
        true => (
            (content_width - width) / 2.,
            content_height / 2. - top,
            width,
        ),
        false => (left, top, width.min(content_width - left - 8.)),
    };
    let window = crate::window_with_view(ui_view);
    let shown = with_overlay(overlay, |o| {
        let height = (o.height)
            .or_else(|| gpui_linux::overlay_size(o.view).map(|size| size.height))
            .unwrap_or_default();
        let bounds = Bounds::new(
            point(px(left as f32), px(top as f32)),
            size(px(width as f32), height),
        );
        let was_shown = o.shown.replace(Shown { window, bounds }).is_some();
        (o.view, px(o.radius as f32), bounds, was_shown)
    });
    let Some((view, radius, bounds, was_shown)) = shown else {
        return;
    };
    if !was_shown {
        unsafe { bc::bc_window_overlay_takes_keyboard(window, true) };
    }
    gpui_linux::show_overlay(view, ui_view, bounds, radius, true, left_overlay);
    let (origin, size) = (bounds.origin, bounds.size);
    let name = NAMES[overlay as usize];
    let at = [origin.x, origin.y, size.width, size.height].map(f32::from);
    eprintln!("browser: {name} at {} {} {} {}", at[0], at[1], at[2], at[3]);
    if !was_shown {
        eprintln!("browser: {name} shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_height(overlay: c_int, height: f64) {
    let moved = with_overlay(overlay, |o| {
        o.height = Some(px(height as f32));
        let shown = o.shown.as_mut()?;
        shown.bounds.size.height = px(height as f32);
        Some((o.view, shown.bounds, px(o.radius as f32)))
    });
    if let Some(Some((view, bounds, radius))) = moved {
        gpui_linux::move_overlay(view, bounds, radius);
        eprintln!("browser: {} height {height}", NAMES[overlay as usize]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_overlay(overlay: c_int) {
    hide(overlay);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_dismiss_overlay(overlay: c_int) {
    if hide(overlay) {
        with_overlay(overlay, |o| later(o.dismissed));
    }
}

/// Hides `overlay` if it shows, and says whether it did. The window has its keyboard back once
/// the overlay's gone, as the input focus on X11 says who had it.
fn hide(overlay: c_int) -> bool {
    let Some(Some((view, window))) = with_overlay(overlay, |o| {
        o.shown.take().map(|shown| (o.view, shown.window))
    }) else {
        return false;
    };
    gpui_linux::hide_overlay(view, move || unsafe {
        bc::bc_window_overlay_takes_keyboard(window, false)
    });
    eprintln!("browser: {} hidden", NAMES[overlay as usize]);
    true
}

/// The user left the overlay of `view`: a press outside it, or the keyboard going elsewhere. GPUI
/// says so once its events are dispatched, outside any update.
fn left_overlay(view: *mut c_void) {
    let overlay = OVERLAYS.with_borrow(|overlays| {
        overlays
            .iter()
            .position(|o| o.as_ref().is_some_and(|o| o.view == view))
    });
    if let Some(overlay) = overlay
        && hide(overlay as c_int)
        && let Some(dismissed) = with_overlay(overlay as c_int, |o| o.dismissed)
    {
        dismissed();
    }
}

/// Runs `dismissed` once the update under way, if any, is over.
fn later(dismissed: extern "C" fn()) {
    if let Some(cx) = crate::CX.with_borrow(Clone::clone) {
        cx.foreground_executor()
            .spawn(async move { dismissed() })
            .detach();
    }
}
