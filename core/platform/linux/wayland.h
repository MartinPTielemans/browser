// The app's UI on Chromium's Wayland connection: a subsurface of the browser window's surface,
// under the pages, on the display Chromium reads.

#ifndef BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_
#define BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_

#include <optional>

#include "base/containers/span.h"
#include "base/functional/callback_forward.h"
#include "browser_core/platform/window_views.h"
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

// Makes the browser window's surface see-through and frameless, for the app's UI under it to show
// and take the clicks the pages don't.
void SetUpWaylandWindow(views::Widget::InitParams& params, views::WidgetDelegate& delegate);

// window_views.h on Wayland, where the X11 versions hand over.
namespace wayland {
void AddWindowViews(views::Widget* widget);
void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               std::optional<SidePanelPage> side_panel);
void HidePages(views::Widget* widget);
bool KeyboardForPage(views::Widget* widget);
void KeepKeyboard(views::Widget* widget, base::OnceClosure change);
}  // namespace wayland

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_LINUX_WAYLAND_H_
