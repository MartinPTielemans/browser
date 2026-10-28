#include "browser_core/platform/linux/wayland.h"

#include <wayland-client-core.h>

#include <memory>

#include "base/auto_reset.h"
#include "base/functional/bind.h"
#include "browser_core/hooks.h"
#include "ui/aura/window.h"
#include "ui/aura/window_tree_host.h"
#include "ui/gfx/geometry/size_conversions.h"
#include "ui/ozone/platform/wayland/host/proxy/wayland_proxy.h"
#include "ui/views/widget/widget_delegate.h"
#include "ui/views/window/native_frame_view.h"

// On Wayland the app's UI shares Chromium's connection, so it can be a subsurface of the browser
// window's surface, under it: a surface on another connection can't be put in Chromium's. Chromium
// draws the pages into its window as Chrome does, each in an aura window of its own, and leaves the
// rest of its surface clear and outside its input region, so the UI under it shows and takes the
// clicks there (window_views.cc). The engine's chromium-core-wayland.patch adds what this reaches
// into.

namespace browser_core::platform {

namespace {

void (*g_app_dispatched)() = nullptr;

// What Chromium calls after each dispatch. The app's dispatch can call the core, and the core
// Chromium, whose round trips land here again: the app's dispatch isn't reentrant, and the outer one
// takes what the inner would have.
void DispatchApp() {
  static bool dispatching = false;
  if (dispatching) {
    return;
  }
  base::AutoReset<bool> reset(&dispatching, true);
  g_app_dispatched();
}

// The window's content, in DIPs.
gfx::Size ContentSizeOf(views::Widget* widget) {
  return gfx::ScaleToFlooredSize(
      widget->GetNativeWindow()->GetHost()->GetBoundsInPixels().size(),
      1 / widget->GetNativeWindow()->GetHost()->device_scale_factor());
}

}  // namespace

bool OnWayland() {
  return wl::WaylandProxy::GetInstance() != nullptr;
}

void* JoinWaylandDisplay(void (*dispatched)()) {
  wl::WaylandProxy* proxy = wl::WaylandProxy::GetInstance();
  if (!proxy) {
    return nullptr;
  }
  // Chromium calls the core, never the app: Clang's indirect-call check in Chromium's code would
  // trap on a function of the app's (cfi_ignores.txt).
  g_app_dispatched = dispatched;
  g_browser_core_wayland_dispatched = dispatched ? &DispatchApp : nullptr;
  return wl_proxy_get_display(reinterpret_cast<wl_proxy*>(proxy->GetRegistry()));
}

void* WaylandSurfaceOf(views::Widget* widget, gfx::Size* size) {
  wl::WaylandProxy* proxy = wl::WaylandProxy::GetInstance();
  if (!proxy) {
    return nullptr;
  }
  *size = ContentSizeOf(widget);
  return proxy->GetWlSurfaceForAcceleratedWidget(
      widget->GetNativeWindow()->GetHost()->GetAcceleratedWidget());
}

void SetUpWaylandWindow(views::Widget::InitParams& params, views::WidgetDelegate& delegate) {
  // Translucent, or the surface's opaque region lets the compositor skip drawing the UI under it.
  params.opacity = views::Widget::InitParams::WindowOpacity::kTranslucent;
  // A frame of the compositor's own makes it ignore the input region.
  params.remove_standard_frame = true;
  // shortcut: no edges to resize by or title bar to move by; the app's header is to take them, as
  // its buttons are on the Mac. A frame view that draws nothing, in place of GTK's title bar.
  delegate.SetFrameViewFactory(
      base::BindRepeating([](views::Widget* widget) -> std::unique_ptr<views::FrameView> {
        return std::make_unique<views::NativeFrameView>(widget);
      }));
}

}  // namespace browser_core::platform
