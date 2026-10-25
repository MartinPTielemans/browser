//! The sidebar reveal's window. GPUI can't draw over the page's native view, so the panel the
//! hidden sidebar slides out in is a window of its own, the browser window's child, at its left
//! edge. The shell draws the panel; this window only holds it.

use browser::shell::{SIDEBAR_WIDTH, Shell};
use gpui::{
    App, Bounds, Context, Entity, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, point, prelude::*, px, size,
};

use crate::{ns_view, platform, update_shell};

struct Reveal {
    shell: Entity<Shell>,
}

impl Render for Reveal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| shell.render_reveal(window, cx))
    }
}

/// Opens the reveal's window, hidden. Opening it ahead of time keeps window setup off the path
/// from the pointer reaching the edge to the panel moving.
pub fn open(shell: Entity<Shell>, cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(SIDEBAR_WIDTH), px(SIDEBAR_WIDTH)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        // A panel that doesn't take the keyboard when clicked.
        kind: WindowKind::PopUp,
        // GPUI holds a window that isn't the key one to 30 fps, and this one never is.
        inactive_frame_interval: None,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| {
            // It draws only while the panel is out.
            cx.observe(&shell, |_, shell, cx| {
                if shell.read(cx).reveal_shown() {
                    cx.notify();
                }
            })
            .detach();
            Reveal { shell }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
                unsafe { platform::host_add_reveal(view, pointer_moved) };
            }
        }
        Err(error) => eprintln!("couldn't open the sidebar reveal: {error}"),
    }
}

extern "C" fn pointer_moved(x: f64) {
    update_shell(|shell, cx| shell.pointer_at(x as f32, cx));
}
