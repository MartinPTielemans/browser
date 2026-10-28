//! The overlays of platform/host.h on Linux: GPUI's Linux platform shows each over the browser
//! window: a subsurface of it under Wayland, and on X11 a window of its own over it, as the X
//! server blends no child window's edges and a compositor blends a window's. The core lends it the
//! window's keyboard while it shows.
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
    /// The app's overlays that have each browser window's keyboard, by their views.
    static LENT: RefCell<Vec<(bc::bc_id, usize)>> = const { RefCell::new(Vec::new()) };
}

/// The overlay of `view` takes the keyboard of `window`, or gives it back. The core lends it while
/// any has it, as the find bar can keep it under the command bar.
pub(crate) fn lend_keyboard(window: bc::bc_id, view: *mut c_void, takes: bool) {
    let entry = (window, view as usize);
    let (before, after) = LENT.with_borrow_mut(|lent| {
        let before = lent.iter().any(|&(each, _)| each == window);
        lent.retain(|&each| each != entry);
        if takes {
            lent.push(entry);
        }
        (before, lent.iter().any(|&(each, _)| each == window))
    });
    if before != after {
        unsafe { bc::bc_window_overlay_takes_keyboard(window, after) };
    }
}

/// Whether an overlay but that of `view` has the keyboard of `window`.
pub(crate) fn keyboard_lent_to_another(window: bc::bc_id, view: *mut c_void) -> bool {
    LENT.with_borrow(|lent| {
        lent.iter()
            .any(|&(each, other)| each == window && other != view as usize)
    })
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
        lend_keyboard(window, view, true);
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

/// The views of the overlays that show, for the test driver (drive.rs).
pub(crate) fn shown() -> Vec<*mut c_void> {
    OVERLAYS.with_borrow(|overlays| {
        (overlays.iter().flatten())
            .filter(|o| o.shown.is_some())
            .map(|o| o.view)
            .collect()
    })
}

/// Hides `overlay` if it shows, and says whether it did. The window has its keyboard back once
/// the overlay's gone, as the input focus on X11 says who had it.
fn hide(overlay: c_int) -> bool {
    let Some(Some((view, window))) = with_overlay(overlay, |o| {
        o.shown.take().map(|shown| (o.view, shown.window))
    }) else {
        return false;
    };
    gpui_linux::hide_overlay(view, move || lend_keyboard(window, view, false));
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
pub(crate) fn later(dismissed: extern "C" fn()) {
    if let Some(cx) = crate::CX.with_borrow(Clone::clone) {
        cx.foreground_executor()
            .spawn(async move { dismissed() })
            .detach();
    }
}

/// The dialog's view, and while it shows, the browser window it's over (its UI's view and the
/// core's id) and how far it's inset from that one's content's edges, left, top, right and bottom:
/// a page's over its tab's card, the rest over all of it.
struct Dialog {
    view: *mut c_void,
    over: Option<(*mut c_void, bc::bc_id)>,
    insets: [f64; 4],
}

thread_local! {
    static DIALOG: RefCell<Dialog> = const {
        RefCell::new(Dialog { view: std::ptr::null_mut(), over: None, insets: [0.; 4] })
    };
}

/// Where the dialog goes in the content of the browser window of `ui_view`, and its corners: a
/// tab's card's, or the window's.
fn dialog_bounds(ui_view: *mut c_void, insets: [f64; 4]) -> Option<(Bounds<Pixels>, Pixels)> {
    let content = gpui_linux::embedder_size(ui_view)?;
    let [left, top, right, bottom] = insets.map(|inset| px(inset as f32));
    let bounds = Bounds::new(
        point(left, top),
        size(
            content.width - left - right,
            content.height - top - bottom,
        ),
    );
    let card = insets != [0.; 4];
    Some((bounds, px(if card { browser::shell::CARD_RADIUS } else { 0. })))
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_dialog(dialog_view: *mut c_void) {
    DIALOG.with_borrow_mut(|dialog| dialog.view = dialog_view);
}

/// Shows the dialog over all of the browser window of `ui_view`, or over the tab's card at `x`,
/// `y`, `width` and `height` in its content, with the keyboard, as it's answered before anything
/// else. Asked again, it goes where the card went, such as when the sidebar hid.
#[unsafe(no_mangle)]
pub extern "C" fn host_show_dialog(ui_view: *mut c_void, x: f64, y: f64, width: f64, height: f64) {
    let Some(content) = gpui_linux::embedder_size(ui_view) else {
        return;
    };
    let insets = match width > 0. {
        true => [
            x,
            y,
            f64::from(content.width) - x - width,
            f64::from(content.height) - y - height,
        ],
        false => [0.; 4],
    };
    let (view, over) = DIALOG.with_borrow_mut(|dialog| {
        if dialog.over.is_none_or(|(over, _)| over == ui_view) {
            dialog.insets = insets;
        }
        (dialog.view, dialog.over)
    });
    if view.is_null() {
        return;
    }
    if over.is_some() {
        place_dialog(ui_view);
        return;
    }
    let Some((bounds, radius)) = dialog_bounds(ui_view, insets) else {
        return;
    };
    let window = crate::window_with_view(ui_view);
    DIALOG.with_borrow_mut(|dialog| dialog.over = Some((ui_view, window)));
    lend_keyboard(window, view, true);
    gpui_linux::show_overlay(view, ui_view, bounds, radius, true, left_dialog);
    // check.sh reads this line.
    eprintln!("browser: dialog window shown");
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_dialog() {
    let (view, over) = DIALOG.with_borrow_mut(|dialog| (dialog.view, dialog.over.take()));
    let Some((_, window)) = over else {
        return;
    };
    gpui_linux::hide_overlay(view, move || lend_keyboard(window, view, false));
    eprintln!("browser: dialog window hidden");
}

/// Places the dialog again if it shows over the browser window of `ui_view`, as its content
/// changed size.
pub(crate) fn place_dialog(ui_view: *mut c_void) {
    let (view, over, insets) =
        DIALOG.with_borrow(|dialog| (dialog.view, dialog.over, dialog.insets));
    if over.is_some_and(|(over, _)| over == ui_view)
        && let Some((bounds, radius)) = dialog_bounds(ui_view, insets)
    {
        gpui_linux::move_overlay(view, bounds, radius);
    }
}

/// Hides the dialog if it shows over the browser window of `ui_view`, which is closing.
pub(crate) fn forget_window(ui_view: *mut c_void) {
    if DIALOG.with_borrow(|dialog| dialog.over.is_some_and(|(over, _)| over == ui_view)) {
        host_hide_dialog();
    }
}

/// The user left the dialog, such as for the sidebar beside a page's, or Chromium took its
/// window's keyboard as a page committed: it takes the keyboard back, unless another app has it.
fn left_dialog(view: *mut c_void) {
    if DIALOG.with_borrow(|dialog| dialog.over.is_some()) {
        gpui_linux::take_overlay_keyboard(view, left_dialog);
    }
}
