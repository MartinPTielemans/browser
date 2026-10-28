//! The overlays of platform/host.h on Linux: GPUI's Linux platform shows each over the browser
//! window as a child of it, a subsurface under Wayland and a child window on X11, and the core
//! lends it the window's keyboard while it shows.
//!
//! shortcut: no blur, shadow, shape or growth yet; the overlay is its UI's rounded rectangle.

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
    shown: Option<Shown>,
}

struct Shown {
    window: bc::bc_id,
    bounds: Bounds<Pixels>,
}

thread_local! {
    static OVERLAYS: RefCell<[Option<Overlay>; 2]> = const { RefCell::new([None, None]) };
}

/// The height an overlay shows at before its UI says how tall it is.
const FIRST_HEIGHT: f32 = 60.;

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
            *slot = Some(Overlay { view, radius, dismissed, shown: None });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_radius(overlay: c_int, radius: f64) {
    with_overlay(overlay, |o| o.radius = radius);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_shape(_overlay: c_int, _corners: *const f64, _count: usize) {}

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
    let (left, top) = match centered {
        true => ((content_width - width) / 2., content_height / 2. - top),
        false => (left.min(content_width - 8. - width), top),
    };
    let window = crate::window_with_view(ui_view);
    let shown = with_overlay(overlay, |o| {
        let height = o.shown.as_ref().map_or(px(FIRST_HEIGHT), |shown| shown.bounds.size.height);
        let bounds = Bounds::new(point(px(left as f32), px(top as f32)), size(px(width as f32), height));
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
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_overlay_height(overlay: c_int, height: f64) {
    let moved = with_overlay(overlay, |o| {
        let shown = o.shown.as_mut()?;
        shown.bounds.size.height = px(height as f32);
        Some((o.view, shown.bounds, px(o.radius as f32)))
    });
    if let Some(Some((view, bounds, radius))) = moved {
        gpui_linux::move_overlay(view, bounds, radius);
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
    let Some(Some((view, window))) =
        with_overlay(overlay, |o| o.shown.take().map(|shown| (o.view, shown.window)))
    else {
        return false;
    };
    gpui_linux::hide_overlay(view, move || unsafe {
        bc::bc_window_overlay_takes_keyboard(window, false)
    });
    true
}

/// The user left the overlay of `view`: a press outside it, or the keyboard going elsewhere. GPUI
/// says so once its events are dispatched, outside any update.
fn left_overlay(view: *mut c_void) {
    let overlay = OVERLAYS.with_borrow(|overlays| {
        overlays.iter().position(|o| o.as_ref().is_some_and(|o| o.view == view))
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
        cx.foreground_executor().spawn(async move { dismissed() }).detach();
    }
}
