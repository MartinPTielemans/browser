//! The zoom HUD: Arc's percent over the top right of the page for a moment after a zoom, its only
//! sign of one. As the find bar is, it's a window of its own that the host shows over the browser
//! window, as its child, and the mouse goes through it to the page.

use std::{ffi::c_void, time::Duration};

use browser::log;
use browser::shell::{self, Shell};
use gpui::{
    App, Bounds, Context, FontWeight, Global, Task, TitlebarOptions, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, point,
    prelude::*, px, rgb, size,
};

use crate::platform;

// Arc's percent is 14 pt, its right edge 20 pt in from the card's and its middle 29.5 below the
// card's top (docs/ux-spec.md › Zoom). The pill around it is ours, as tall as the find bar and
// with its fill, so their right edges line up. It's right-aligned in a window wide enough for
// "500%".
const WIDTH: f32 = 80.;
const HEIGHT: f32 = 32.;
const RADIUS: f32 = 6.;
const PADDING: f32 = 10.;
const RIGHT: f32 = shell::CARD_INSET * 2.;
const TOP: f32 = shell::CARD_INSET + 29.5 - HEIGHT / 2.;
const TEXT: u32 = 0xffffff;
// Ours: how long Arc's stays isn't measured.
const SHOWN: Duration = Duration::from_millis(1500);
const FADE: f64 = 0.15;

pub struct ZoomHud {
    /// The shell of the browser window it shows over, for the window's colour, which the HUD takes.
    shell: WeakEntity<Shell>,
    /// The tab zoomed, which the HUD goes with.
    tab: u64,
    percent: i32,
    _hide: Option<Task<()>>,
}

struct ZoomHudWindow(WindowHandle<ZoomHud>);

impl Global for ZoomHudWindow {}

/// Opens the HUD's window, hidden, for every browser window, ahead of time as the find bar does.
pub fn open(cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(HEIGHT)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Floating,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The pill is drawn, and the shadow takes its shape.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|_| ZoomHud {
            shell: WeakEntity::new_invalid(),
            tab: 0,
            percent: 100,
            _hide: None,
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| crate::ns_view(window)) {
                unsafe { platform::host_add_hud(view, f64::from(RIGHT), f64::from(TOP)) };
                cx.set_global(ZoomHudWindow(window));
            }
        }
        Err(error) => log!("couldn't open the zoom HUD: {error}"),
    }
}

/// Shows `tab`'s zoom, `percent`, for a moment from now, over the browser window whose UI draws
/// in `ui_view`.
pub fn show(tab: u64, percent: i32, ui_view: *mut c_void, cx: &mut App) {
    let Some(window) = cx.try_global::<ZoomHudWindow>().map(|hud| hud.0) else {
        return;
    };
    window
        .update(cx, |hud, _, cx| {
            hud.shell = crate::shell_with_view(ui_view);
            hud.tab = tab;
            hud.percent = percent;
            hud._hide = Some(cx.spawn(async |_, cx| {
                cx.background_executor().timer(SHOWN).await;
                unsafe { platform::host_hide_hud(FADE) };
            }));
            cx.notify();
        })
        .ok();
    unsafe { platform::host_show_hud(ui_view) };
}

/// The window shows `tab`, or no page (0): the HUD goes at once if it showed another tab's zoom.
pub fn tab_shown(tab: u64, cx: &mut App) {
    let Some(window) = cx.try_global::<ZoomHudWindow>().map(|hud| hud.0) else {
        return;
    };
    // Its timer stays: hiding it again does nothing.
    let other = window.update(cx, |hud, _, _| hud.tab != tab).unwrap_or(false);
    if other {
        unsafe { platform::host_hide_hud(0.) };
    }
}

impl Render for ZoomHud {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let fill = self.shell.upgrade().map(|shell| shell.read(cx).window_fill());
        div().size_full().flex().justify_end().child(
            div()
                .h_full()
                .px(px(PADDING))
                .flex()
                .items_center()
                .rounded(px(RADIUS))
                .when_some(fill, |pill, fill| pill.bg(fill))
                .font_family(".SystemUIFont")
                .font_weight(FontWeight::MEDIUM)
                .text_size(px(14.))
                .text_color(rgb(TEXT))
                .child(format!("{}%", self.percent)),
        )
    }
}
