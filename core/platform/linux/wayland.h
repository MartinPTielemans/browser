// The app's UI on Chromium's Wayland connection: a subsurface of the browser window's surface,
// under the pages, on the display Chromium reads.

#ifndef BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_
#define BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_

#include "ui/gfx/geometry/size.h"
#include "ui/views/widget/widget.h"

namespace views {
class WidgetDelegate;
}  // namespace views

namespace browser_core::platform {

// Whether Chromium runs on Wayland, rather than X11.
bool OnWayland();

// Chromium's wl_display, from now on calling |dispatched| after each time Chromium dispatches it
// (bc_wayland_display). Null off Wayland.
void* JoinWaylandDisplay(void (*dispatched)());

// |widget|'s wl_surface, and its content's size in DIPs. Null off Wayland.
void* WaylandSurfaceOf(views::Widget* widget, gfx::Size* size);

// Makes the browser window frameless, under Wayland and X11, as the app draws the window's buttons,
// and under Wayland see-through, for the app's UI under it to show and take the clicks the pages
// don't.
void SetUpWindow(views::Widget::InitParams& params, views::WidgetDelegate& delegate);

// Gives the keyboard in |widget|'s window to the app's UI (bc_window_ui_takes_keyboard), and whether
// the UI has it (bc_window_keyboard_is_ui). In window_views.cc, as on X11 the core follows the X
// server's focus.
void UITakesKeyboard(views::Widget* widget);
bool KeyboardIsUI(views::Widget* widget);

// Lends the keyboard in |widget|'s window to an overlay of the app's, or takes it back
// (bc_window_overlay_takes_keyboard).
void OverlayTakesKeyboard(views::Widget* widget, bool takes);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_
