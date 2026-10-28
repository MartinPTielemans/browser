#include "browser_core/platform/linux/wayland.h"

#include <wayland-client-core.h>

#include <map>
#include <memory>
#include <utility>
#include <vector>

#include "base/auto_reset.h"
#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/hooks.h"
#include "browser_core/host.h"
#include "browser_core/window.h"
#include "ui/aura/client/focus_client.h"
#include "ui/aura/window.h"
#include "ui/aura/window_tracker.h"
#include "ui/aura/window_tree_host_platform.h"
#include "ui/compositor/layer.h"
#include "ui/gfx/geometry/insets.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/rect_conversions.h"
#include "ui/gfx/geometry/rounded_corners_f.h"
#include "ui/gfx/geometry/size_conversions.h"
#include "ui/ozone/platform/wayland/host/proxy/wayland_proxy.h"
#include "ui/platform_window/platform_window.h"
#include "ui/views/widget/widget_delegate.h"
#include "ui/views/widget/widget_observer.h"
#include "ui/views/window/native_frame_view.h"

// On Wayland the app's UI shares Chromium's connection, so it can be a subsurface of the browser
// window's surface, under it: a surface on another connection can't be put in Chromium's. Chromium
// draws the pages into its window as Chrome does, each in an aura window of its own, and leaves the
// rest of its surface clear and outside its input region, so the UI under it shows and takes the
// clicks there. The engine's chromium-core-wayland.patch adds what this reaches into.

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

ui::PlatformWindow* PlatformWindowOf(views::Widget* widget) {
  return static_cast<aura::WindowTreeHostPlatform*>(widget->GetNativeWindow()->GetHost())
      ->platform_window();
}

float ScaleOf(views::Widget* widget) {
  return widget->GetNativeWindow()->GetHost()->device_scale_factor();
}

// The window's content, in DIPs.
gfx::Size ContentSizeOf(views::Widget* widget) {
  return gfx::ScaleToFlooredSize(widget->GetNativeWindow()->GetHost()->GetBoundsInPixels().size(),
                                 1 / ScaleOf(widget));
}

// An aura window in the browser window holding one pane's page and its docked DevTools, or the side
// panel, with their round corners.
struct Slot {
  std::unique_ptr<aura::Window> holder;
  raw_ptr<aura::Window> page = nullptr;
  raw_ptr<aura::Window> devtools = nullptr;
  // From each edge of the window's content to the slot's, kept as the window resizes.
  gfx::Insets insets;
  // With DevTools docked, where the page goes in the slot, from its top-left.
  gfx::Rect page_in_devtools;
  // Only the bottom corners are round, for the side panel under the app's header.
  bool bottom_corners_only = false;
};

class WindowViews : public views::WidgetObserver {
 public:
  explicit WindowViews(views::Widget* widget) : widget_(widget) { observation_.Observe(widget); }

  std::vector<Slot>& panes() { return panes_; }
  Slot& side_panel() { return side_panel_; }
  void set_corner_radius(double radius) { corner_radius_ = radius; }

  // A slot's holder, made the first time it's needed, over the window's own views.
  aura::Window* HolderFor(Slot& slot) {
    if (!slot.holder) {
      slot.holder = std::make_unique<aura::Window>(nullptr);
      slot.holder->set_owned_by_parent(false);
      slot.holder->Init(ui::LAYER_NOT_DRAWN);
      slot.holder->SetName("BrowserCorePane");
      slot.holder->layer()->SetMasksToBounds(true);
      slot.holder->layer()->SetIsFastRoundedCorner(true);
      widget_->GetNativeView()->AddChild(slot.holder.get());
    }
    return slot.holder.get();
  }

  // Takes |view| out of whichever slot holds it, so closing a slot never deletes a page.
  static void Detach(aura::Window* view) {
    if (view && view->parent()) {
      view->parent()->RemoveChild(view);
    }
  }

  void Empty(Slot& slot) {
    Detach(slot.page);
    Detach(slot.devtools);
    slot.page = nullptr;
    slot.devtools = nullptr;
    if (slot.holder) {
      slot.holder->Hide();
    }
  }

  // Sizes each slot to its insets.
  void LayOut() {
    const gfx::Size size = ContentSizeOf(widget_);
    for (Slot& slot : panes_) {
      LayOut(slot, size);
    }
    LayOut(side_panel_, size);
  }

  void LayOut(Slot& slot, const gfx::Size& window_size) {
    if (!slot.holder || !slot.page) {
      return;
    }
    gfx::Rect frame(window_size);
    frame.Inset(slot.insets);
    slot.holder->SetBounds(frame);
    const gfx::Rect bounds(frame.size());
    if (slot.devtools) {
      slot.devtools->SetBounds(bounds);
      gfx::Rect page = slot.page_in_devtools;
      page.Intersect(bounds);
      slot.page->SetBounds(page);
    } else {
      slot.page->SetBounds(bounds);
    }
    const float r = corner_radius_;
    slot.holder->layer()->SetRoundedCornerRadius(
        slot.bottom_corners_only ? gfx::RoundedCornersF(0, 0, r, r) : gfx::RoundedCornersF(r));
  }

  // Lets clicks through the window's surface to the app's UI under it, but on the pages shown.
  void SetInputRegion() {
    std::vector<gfx::Rect> region;
    const float scale = ScaleOf(widget_);
    auto add = [&](const Slot& slot) {
      if (slot.holder && slot.page && slot.holder->IsVisible()) {
        region.push_back(gfx::ScaleToEnclosingRect(slot.holder->bounds(), scale));
      }
    };
    for (const Slot& slot : panes_) {
      add(slot);
    }
    add(side_panel_);
    // No rects is the whole surface; one empty rect is none of it.
    if (region.empty()) {
      region.emplace_back();
    }
    PlatformWindowOf(widget_)->SetInputRegion(region);
  }

  // views::WidgetObserver:
  void OnWidgetBoundsChanged(views::Widget*, const gfx::Rect&) override {
    LayOut();
    // Chromium sets the input region back to the whole window as the bounds change, after telling
    // observers (DesktopWindowTreeHostLinux::UpdateFrameHints).
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE,
        base::BindOnce(&WindowViews::SetInputRegion, weak_factory_.GetWeakPtr()));
    // No compositor sizes a subsurface, so the app sizes its UI to the window's content.
    if (!Host().window_resized) {
      return;
    }
    const gfx::Size size = ContentSizeOf(widget_);
    for (Window* window : Window::All()) {
      if (window->widget() == widget_) {
        Host().window_resized(Host().context, window->id(), size.width(), size.height());
      }
    }
  }

  void OnWidgetDestroying(views::Widget*) override {
    for (Slot& slot : panes_) {
      Empty(slot);
    }
    Empty(side_panel_);
    panes_.clear();
    side_panel_ = Slot();
    observation_.Reset();
  }

 private:
  raw_ptr<views::Widget> widget_;
  std::vector<Slot> panes_;
  Slot side_panel_;
  double corner_radius_ = 0;
  base::ScopedObservation<views::Widget, views::WidgetObserver> observation_{this};
  base::WeakPtrFactory<WindowViews> weak_factory_{this};
};

std::map<views::Widget*, std::unique_ptr<WindowViews>>& AllWindowViews() {
  static base::NoDestructor<std::map<views::Widget*, std::unique_ptr<WindowViews>>> all;
  return *all;
}

WindowViews* ViewsOf(views::Widget* widget) {
  auto it = AllWindowViews().find(widget);
  return it == AllWindowViews().end() ? nullptr : it->second.get();
}

gfx::Insets InsetsOf(const gfx::Rect& frame, const gfx::Size& window) {
  return gfx::Insets::TLBR(frame.y(), frame.x(), window.height() - frame.bottom(),
                           window.width() - frame.right());
}

// Puts |view| in |parent| if it isn't there, and shows it unless |hidden|.
void Hold(aura::Window* parent, aura::Window* view, bool hidden) {
  if (view->parent() != parent) {
    WindowViews::Detach(view);
    parent->AddChild(view);
  }
  if (hidden) {
    view->Hide();
  } else {
    view->Show();
  }
}

// Empties |holder| of all but |keep| and |keep_too|.
void DetachOthers(aura::Window* holder, aura::Window* keep, aura::Window* keep_too) {
  // A copy, as Detach changes the list; its raw_ptrs convert one at a time.
  const aura::Window::Windows& children = holder->children();
  for (aura::Window* child : std::vector<aura::Window*>(children.begin(), children.end())) {
    if (child != keep && child != keep_too) {
      WindowViews::Detach(child);
    }
  }
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

namespace wayland {

void AddWindowViews(views::Widget* widget) {
  AllWindowViews()[widget] = std::make_unique<WindowViews>(widget);
  ViewsOf(widget)->SetInputRegion();
}

void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               std::optional<SidePanelPage> side_panel) {
  WindowViews* views = ViewsOf(widget);
  if (!views) {
    return;
  }
  views->set_corner_radius(corner_radius);
  const bool hidden = !widget->IsVisible();
  const gfx::Size size = ContentSizeOf(widget);

  std::vector<Slot>& slots = views->panes();
  // Pages that stay in the same pane keep their slot.
  for (size_t i = panes.size(); i < slots.size(); ++i) {
    views->Empty(slots[i]);
  }
  if (slots.size() < panes.size()) {
    slots.resize(panes.size());
  }
  for (size_t i = 0; i < panes.size(); ++i) {
    const Pane& pane = panes[i];
    Slot& slot = slots[i];
    aura::Window* holder = views->HolderFor(slot);
    aura::Window* devtools = pane.devtools ? pane.devtools->view : nullptr;
    DetachOthers(holder, pane.page, devtools);
    if (devtools) {
      Hold(holder, devtools, hidden);
      slot.page_in_devtools = pane.devtools->page_frame;
    }
    Hold(holder, pane.page, hidden);
    if (devtools) {
      if (pane.devtools->page_under) {
        holder->StackChildBelow(pane.page, devtools);
      } else {
        holder->StackChildAbove(pane.page, devtools);
      }
    }
    slot.page = pane.page;
    slot.devtools = devtools;
    slot.insets = InsetsOf(pane.frame, size);
    hidden ? holder->Hide() : holder->Show();
  }

  Slot& panel = views->side_panel();
  if (side_panel) {
    aura::Window* holder = views->HolderFor(panel);
    DetachOthers(holder, side_panel->page, nullptr);
    Hold(holder, side_panel->page, hidden);
    panel.page = side_panel->page;
    panel.insets = InsetsOf(side_panel->frame, size);
    panel.bottom_corners_only = true;
    hidden ? holder->Hide() : holder->Show();
  } else {
    views->Empty(panel);
  }
  views->LayOut();
  views->SetInputRegion();
}

void HidePages(views::Widget* widget) {
  WindowViews* views = ViewsOf(widget);
  if (!views) {
    return;
  }
  for (Slot& slot : views->panes()) {
    if (slot.holder) {
      slot.holder->Hide();
    }
  }
  if (views->side_panel().holder) {
    views->side_panel().holder->Hide();
  }
  views->SetInputRegion();
}

bool KeyboardForPage(views::Widget* widget) {
  // shortcut: the keyboard is always Chromium's, as the compositor gives it to the window's surface
  // and the app's UI takes none yet; forward keys to the UI when it has the keyboard.
  WindowViews* views = ViewsOf(widget);
  aura::client::FocusClient* client = aura::client::GetFocusClient(widget->GetNativeView());
  aura::Window* focused = client ? client->GetFocusedWindow() : nullptr;
  // The window itself has it as it opens, or once the view that had it went.
  if (!views || !focused || focused == widget->GetNativeView()) {
    return true;
  }
  for (Slot& slot : views->panes()) {
    if (slot.holder && slot.holder->Contains(focused)) {
      return !slot.devtools || !slot.devtools->Contains(focused);
    }
  }
  // The side panel.
  return false;
}

void KeepKeyboard(views::Widget* widget, base::OnceClosure change) {
  aura::WindowTracker focused;
  if (auto* client = aura::client::GetFocusClient(widget->GetNativeView())) {
    if (aura::Window* window = client->GetFocusedWindow()) {
      focused.Add(window);
    }
  }
  std::move(change).Run();
  if (focused.windows().empty()) {
    return;
  }
  aura::Window* window = focused.windows().front();
  auto* client = aura::client::GetFocusClient(window);
  if (client && client->GetFocusedWindow() != window &&
      widget->GetNativeView()->Contains(window)) {
    client->FocusWindow(window);
  }
}

}  // namespace wayland

}  // namespace browser_core::platform
