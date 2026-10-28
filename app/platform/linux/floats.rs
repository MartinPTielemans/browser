//! The floats of platform/host.h on Linux: the zoom HUD, the split view's drop targets, the toast,
//! the tooltip and the link target, which float over a browser window without the keyboard, as
//! their windows do on the Mac. GPUI shows each as an overlay of that window (overlays.rs) that
//! lets the pointer through to what's under it, but for the toast's plate while it has buttons.
//!
//! shortcut: the tooltip and the link target show and hide at once, without fading, and the link
//! target doesn't hear the pointer over the page, which is Chromium's on Linux, so it doesn't move
//! out of the pointer's way.

use std::{cell::RefCell, ffi::c_void};

use gpui::{Bounds, Pixels, point, px, size};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Hud,
    DropTargets,
    Toast,
    Tooltip,
    LinkTarget,
}

struct Float {
    kind: Kind,
    view: *mut c_void,
    /// The UI's view of the browser window it shows over; null while hidden.
    over: *mut c_void,
    /// Where the UI asked, from the window's content's top left: the tooltip's and the link
    /// target's own corner, which their window holds `margin` in from its own.
    left: f64,
    top: f64,
}

#[derive(Default)]
struct Hud {
    right: f64,
    top: f64,
    margin: f64,
}

#[derive(Default)]
struct Toast {
    takes: bool,
    plate: Option<Bounds<Pixels>>,
}

thread_local! {
    static FLOATS: RefCell<Vec<Float>> = const { RefCell::new(Vec::new()) };
    static HUD: RefCell<Hud> = RefCell::default();
    static TOAST: RefCell<Toast> = RefCell::default();
    /// The tooltip's and the link target's margins, and what runs as the user leaves the tooltip's
    /// window.
    static TOOLTIP: RefCell<(f64, Option<extern "C" fn()>)> = const { RefCell::new((0., None)) };
    static LINK_MARGIN: RefCell<f64> = const { RefCell::new(0.) };
}

fn add(kind: Kind, view: *mut c_void) {
    // None of them takes the pointer until the toast has buttons.
    gpui_linux::set_overlay_pointer(view, Some(&[]));
    FLOATS.with_borrow_mut(|floats| {
        floats.retain(|float| float.kind != kind);
        floats.push(Float { kind, view, over: std::ptr::null_mut(), left: 0., top: 0. });
    });
}

fn find<R>(kind: Kind, f: impl FnOnce(&mut Float) -> R) -> Option<R> {
    FLOATS.with_borrow_mut(|floats| floats.iter_mut().find(|float| float.kind == kind).map(f))
}

/// Where the float `kind` goes over `ui_view`'s window, at its own size but for the drop targets,
/// which cover the window's content.
fn bounds(kind: Kind, ui_view: *mut c_void) -> Option<Bounds<Pixels>> {
    let (view, left, top) = find(kind, |float| (float.view, float.left, float.top))?;
    let content = gpui_linux::embedder_size(ui_view)?;
    let own = gpui_linux::overlay_size(view)?;
    let width = f64::from(content.width);
    let (x, y) = match kind {
        Kind::DropTargets => return Some(Bounds::new(point(px(0.), px(0.)), content)),
        Kind::Toast => (width - f64::from(own.width), 0.),
        Kind::Hud => {
            let (page_right, page_top) = crate::panels::page_corner(ui_view);
            let (right, top, margin) = HUD.with_borrow(|hud| (hud.right, hud.top, hud.margin));
            // 8 points below the find bar while it shows, where the HUD would cover it.
            let top = crate::panels::below_find_bar(ui_view).map_or(top, |below| below - margin);
            (width - page_right - right - f64::from(own.width), page_top + top)
        }
        Kind::Tooltip => {
            let margin = TOOLTIP.with_borrow(|(margin, _)| *margin);
            (left - margin, top - margin)
        }
        Kind::LinkTarget => {
            let margin = LINK_MARGIN.with_borrow(|margin| *margin);
            (left - margin, top - margin)
        }
    };
    Some(Bounds::new(point(px(x as f32), px(y as f32)), size(own.width, own.height)))
}

/// Shows the float `kind` over `ui_view`'s window, or moves it there, and says where for check.sh.
fn show(kind: Kind, ui_view: *mut c_void) -> Option<Bounds<Pixels>> {
    let over = find(kind, |float| float.over)?;
    if !over.is_null() && over != ui_view {
        hide(kind);
    }
    let bounds = bounds(kind, ui_view)?;
    let view = find(kind, |float| {
        float.over = ui_view;
        float.view
    })?;
    gpui_linux::show_overlay(view, ui_view, bounds, px(0.), false, left);
    Some(bounds)
}

fn hide(kind: Kind) -> bool {
    let Some((view, over)) =
        find(kind, |float| (float.view, std::mem::replace(&mut float.over, std::ptr::null_mut())))
    else {
        return false;
    };
    if over.is_null() {
        return false;
    }
    gpui_linux::hide_overlay(view, || {});
    true
}

/// The user left the float of `view`, by a press elsewhere or the keyboard going to another
/// window: the tooltip goes, as it does on the Mac when the browser window stops being key.
fn left(view: *mut c_void) {
    let tooltip = find(Kind::Tooltip, |float| float.view == view && !float.over.is_null());
    if tooltip == Some(true)
        && let Some(away) = TOOLTIP.with_borrow(|(_, away)| *away)
    {
        away();
    }
}

/// Places the floats over `ui_view`'s window again, as its content changed size, its page moved or
/// its find bar showed or hid.
pub(crate) fn place(ui_view: *mut c_void) {
    for kind in [Kind::Hud, Kind::DropTargets, Kind::Toast] {
        let shown = find(kind, |float| (float.view, float.over == ui_view));
        if let Some((view, true)) = shown
            && let Some(bounds) = bounds(kind, ui_view)
        {
            gpui_linux::move_overlay(view, bounds, px(0.));
        }
    }
}

/// `ui_view`'s window is closing: what floats over it goes.
pub(crate) fn forget_window(ui_view: *mut c_void) {
    for kind in [Kind::Hud, Kind::DropTargets, Kind::Toast, Kind::Tooltip, Kind::LinkTarget] {
        if find(kind, |float| float.over == ui_view) == Some(true) {
            hide(kind);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_hud(hud_view: *mut c_void, right: f64, top: f64, margin: f64) {
    HUD.set(Hud { right, top, margin });
    add(Kind::Hud, hud_view);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_hud(ui_view: *mut c_void) {
    if show(Kind::Hud, ui_view).is_some() {
        // check.sh reads this line.
        eprintln!("browser: zoom hud shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_hud() {
    if hide(Kind::Hud) {
        eprintln!("browser: zoom hud hidden");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_drop_targets(view: *mut c_void) {
    add(Kind::DropTargets, view);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_drop_targets(ui_view: *mut c_void) {
    if find(Kind::DropTargets, |float| float.over == ui_view) == Some(true) {
        return;
    }
    if show(Kind::DropTargets, ui_view).is_some() {
        eprintln!("browser: drop targets shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_drop_targets() {
    if hide(Kind::DropTargets) {
        eprintln!("browser: drop targets hidden");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_toast(toast_view: *mut c_void) {
    add(Kind::Toast, toast_view);
}

/// The toast takes the pointer on its plate while it has buttons, and lets it through elsewhere.
fn toast_pointer() {
    let Some(view) = find(Kind::Toast, |float| float.view) else { return };
    let plate = TOAST.with_borrow(|toast| toast.plate.filter(|_| toast.takes));
    gpui_linux::set_overlay_pointer(view, Some(plate.as_slice()));
}

#[unsafe(no_mangle)]
pub extern "C" fn host_toast_takes_clicks(takes: bool) {
    TOAST.with_borrow_mut(|toast| toast.takes = takes);
    toast_pointer();
}

#[unsafe(no_mangle)]
pub extern "C" fn host_set_toast_plate(x: f64, y: f64, width: f64, height: f64) {
    let plate = Bounds::new(
        point(px(x as f32), px(y as f32)),
        size(px(width as f32), px(height as f32)),
    );
    TOAST.with_borrow_mut(|toast| toast.plate = Some(plate));
    toast_pointer();
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_toast(ui_view: *mut c_void) {
    if find(Kind::Toast, |float| float.over == ui_view) == Some(true) {
        return;
    }
    if show(Kind::Toast, ui_view).is_some() {
        // check.sh reads this line.
        eprintln!("browser: toast window shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_toast() {
    if hide(Kind::Toast) {
        TOAST.with_borrow_mut(|toast| toast.plate = None);
        toast_pointer();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_tooltip(tooltip_view: *mut c_void, margin: f64, away: extern "C" fn()) {
    TOOLTIP.set((margin, Some(away)));
    add(Kind::Tooltip, tooltip_view);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_tooltip(ui_view: *mut c_void, left: f64, top: f64, _fade: f64) {
    find(Kind::Tooltip, |float| (float.left, float.top) = (left, top));
    if show(Kind::Tooltip, ui_view).is_some() {
        // check.sh reads these lines: where it is, to compare with where the UI asked.
        eprintln!("browser: tooltip window at {left} {top}");
        eprintln!("browser: tooltip window shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_tooltip(_fade: f64) {
    hide(Kind::Tooltip);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_link_target(
    link_view: *mut c_void,
    margin: f64,
    _pointer: extern "C" fn(x: f64, y: f64),
) {
    LINK_MARGIN.set(margin);
    add(Kind::LinkTarget, link_view);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_link_target(ui_view: *mut c_void, left: f64, top: f64, _fade: f64) {
    find(Kind::LinkTarget, |float| (float.left, float.top) = (left, top));
    if show(Kind::LinkTarget, ui_view).is_some() {
        // check.sh reads this line.
        eprintln!("browser: link target window shown");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_link_target(_fade: f64) {
    hide(Kind::LinkTarget);
}
