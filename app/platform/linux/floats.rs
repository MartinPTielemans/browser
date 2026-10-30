//! The floats of platform/host.h on Linux: the zoom HUD, the split view's drop targets, the toast,
//! the tooltip and the link target, which float over a browser window without the keyboard, as
//! their windows do on the Mac. GPUI shows each as an overlay of that window (overlays.rs) that
//! lets the pointer through to what's under it, but for the toast's plate while it has buttons.
//!
//! The tooltip and the link target fade in and out as the UI asks, as the Mac's do, drawn by GPUI
//! (motion.rs).
//!
//! The link target hears the pointer over the page while it shows, which Chromium's windows have
//! on Linux, through GPUI (`gpui_linux::watch_pointer_over`), so it moves out of the pointer's way.

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
};

use gpui::{Bounds, Pixels, Point, point, px, size};

use crate::motion::{self, EASE_OUT, Motion, Track};

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
    /// The one it's still fading out over, once hidden.
    fading: *mut c_void,
    /// How many times it was shown or hidden, so a fade's end that came after another knows.
    changes: u32,
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
    /// Where the link target tells the UI the pointer is, over the window it shows over.
    static LINK_POINTER: Cell<Option<extern "C" fn(f64, f64)>> = const { Cell::new(None) };
}

fn add(kind: Kind, view: *mut c_void) {
    // None of them takes the pointer until the toast has buttons.
    gpui_linux::set_overlay_pointer(view, Some(&[]));
    if matches!(kind, Kind::Tooltip | Kind::LinkTarget) {
        motion::attach(view);
    }
    FLOATS.with_borrow_mut(|floats| {
        floats.retain(|float| float.kind != kind);
        floats.push(Float {
            kind,
            view,
            over: std::ptr::null_mut(),
            fading: std::ptr::null_mut(),
            changes: 0,
            left: 0.,
            top: 0.,
        });
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
    let (over, fading) = find(kind, |float| (float.over, float.fading))?;
    if [over, fading].iter().any(|each| !each.is_null() && *each != ui_view) {
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

/// Hides the float `kind` at once, and says whether it showed: it may have been fading out.
fn hide(kind: Kind) -> bool {
    let Some((view, over, fading)) = find(kind, |float| {
        float.changes += 1;
        let over = std::mem::replace(&mut float.over, std::ptr::null_mut());
        (float.view, over, std::mem::replace(&mut float.fading, std::ptr::null_mut()))
    }) else {
        return false;
    };
    if over.is_null() && fading.is_null() {
        return false;
    }
    if kind == Kind::LinkTarget {
        gpui_linux::watch_pointer_over(std::ptr::null_mut(), None);
    }
    gpui_linux::hide_overlay(view, || {});
    !over.is_null()
}

/// Shows the tooltip or the link target over `ui_view`'s window, fading in over `fade` seconds from
/// where it is, as one fading out over that window goes back to full, and then says it's `shown`.
fn fade_in(kind: Kind, ui_view: *mut c_void, fade: f64, shown: &'static str) {
    let Some((view, over, fading)) = find(kind, |float| (float.view, float.over, float.fading))
    else {
        return;
    };
    let from = match ui_view == over || ui_view == fading {
        true => motion::opacity(view),
        false => 0.,
    };
    let fade = seconds(fade);
    let opacity = Track { from, to: 1., seconds: fade, curve: EASE_OUT };
    motion::start(view, Motion::fade(opacity));
    if show(kind, ui_view).is_none() {
        return;
    }
    let Some(change) = find(kind, |float| {
        float.fading = std::ptr::null_mut();
        float.changes += 1;
        float.changes
    }) else {
        return;
    };
    motion::after(fade, move || {
        if find(kind, |float| float.changes) == Some(change) {
            // check.sh waits for this line, so a capture after it shows the float whole.
            eprintln!("{shown}");
        }
    });
}

/// Hides the tooltip or the link target, fading out over `fade` seconds; with none, at once, which
/// also ends a fade out.
fn fade_out(kind: Kind, fade: f64) {
    let fade = seconds(fade);
    if fade == 0. {
        hide(kind);
        return;
    }
    let Some(Some((view, change))) = find(kind, |float| {
        if float.over.is_null() {
            return None;
        }
        float.fading = std::mem::replace(&mut float.over, std::ptr::null_mut());
        float.changes += 1;
        Some((float.view, float.changes))
    }) else {
        return;
    };
    let from = motion::opacity(view);
    motion::start(view, Motion::fade(Track { from, to: 0., seconds: fade, curve: EASE_OUT }));
    motion::after(fade, move || {
        if find(kind, |float| float.changes) == Some(change) {
            hide(kind);
        }
    });
}

/// A fade's seconds, none where the float can't blend with what's under it.
fn seconds(fade: f64) -> f32 {
    match gpui_linux::overlays_blend() {
        true => fade as f32,
        false => 0.,
    }
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
        if find(kind, |float| float.over == ui_view || float.fading == ui_view) == Some(true) {
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

/// The toast's view and its plate, the part of its window that takes the pointer, while it shows
/// with buttons: the driver clicks there only where a pointer would land on it (drive.rs).
pub(crate) fn toast_plate() -> Option<(*mut c_void, Bounds<Pixels>)> {
    let (view, over) = find(Kind::Toast, |float| (float.view, float.over))?;
    let plate = TOAST.with_borrow(|toast| toast.plate.filter(|_| toast.takes))?;
    (!over.is_null()).then_some((view, plate))
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
pub extern "C" fn host_show_tooltip(ui_view: *mut c_void, left: f64, top: f64, fade: f64) {
    find(Kind::Tooltip, |float| (float.left, float.top) = (left, top));
    // check.sh reads this line: where it is, to compare with where the UI asked.
    eprintln!("browser: tooltip window at {left} {top}");
    fade_in(Kind::Tooltip, ui_view, fade, "browser: tooltip window shown");
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_tooltip(fade: f64) {
    fade_out(Kind::Tooltip, fade);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_add_link_target(
    link_view: *mut c_void,
    margin: f64,
    pointer: extern "C" fn(x: f64, y: f64),
) {
    LINK_MARGIN.set(margin);
    LINK_POINTER.set(Some(pointer));
    add(Kind::LinkTarget, link_view);
}

#[unsafe(no_mangle)]
pub extern "C" fn host_show_link_target(ui_view: *mut c_void, left: f64, top: f64, fade: f64) {
    find(Kind::LinkTarget, |float| (float.left, float.top) = (left, top));
    // check.sh reads this line: which side it's on, out of the pointer's way.
    eprintln!("browser: link target window at {left} {top}");
    fade_in(Kind::LinkTarget, ui_view, fade, "browser: link target window shown");
    // Only while it shows, so an idle browser is never woken by the pointer.
    gpui_linux::watch_pointer_over(ui_view, Some(link_pointer));
}

/// The pointer moved over the window the link target shows over: the UI hears it after the event.
fn link_pointer(at: Point<Pixels>) {
    let Some(pointer) = LINK_POINTER.get() else {
        return;
    };
    let (x, y) = (f64::from(f32::from(at.x)), f64::from(f32::from(at.y)));
    if let Some(cx) = crate::CX.with_borrow(Clone::clone) {
        cx.foreground_executor().spawn(async move { pointer(x, y) }).detach();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn host_hide_link_target(fade: f64) {
    gpui_linux::watch_pointer_over(std::ptr::null_mut(), None);
    fade_out(Kind::LinkTarget, fade);
}
