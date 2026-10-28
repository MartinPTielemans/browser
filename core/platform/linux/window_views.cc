#include "browser_core/platform/window_views.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <map>
#include <memory>
#include <utility>
#include <vector>

#include "base/functional/callback.h"
#include "base/memory/raw_ptr.h"
#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "browser_core/platform/linux/wayland.h"
#include "ui/aura/client/focus_client.h"
#include "ui/aura/window.h"
#include "ui/aura/window_tracker.h"
#include "ui/aura/window_tree_host.h"
#include "ui/gfx/geometry/insets.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/rect_conversions.h"
#include "ui/gfx/geometry/size_conversions.h"
#include "ui/gfx/x/connection.h"
#include "ui/gfx/x/event.h"
#include "ui/gfx/x/event_observer.h"
#include "ui/gfx/x/window_event_manager.h"
#include "ui/gfx/x/xproto.h"
#include "ui/views/widget/desktop_aura/desktop_native_widget_aura.h"
#include "ui/views/widget/widget.h"
#include "ui/views/widget/widget_observer.h"

// As on Windows, Aura draws every view of a widget into its one X window, so a page can't go over
// a native view in it the way an NSView goes over the app's on the Mac. Here the app's UI is a
// child X window that fills the window, and each page shown sits in a child widget of its own, a
// child X window above the UI's. Chromium makes every widget's X window on the root, so the core
// puts these inside the browser window itself. Chromium's GPU process maps a child X window of its
// own over the browser window's, for the window's compositor output, and the core sends it under
// the UI's: the UI draws everything that window would. On Wayland, each function here hands over
// to wayland.cc.

namespace browser_core::platform {

namespace {

x11::Window XWindowOf(views::Widget* widget) {
  return static_cast<x11::Window>(widget->GetNativeWindow()->GetHost()->GetAcceleratedWidget());
}

// Whether |window| is |ancestor| or inside it. A round trip to the X server a level.
bool IsWithin(x11::Window window, x11::Window ancestor) {
  x11::Connection* connection = x11::Connection::Get();
  while (window != x11::Window::None) {
    if (window == ancestor) {
      return true;
    }
    auto tree = connection->QueryTree({window}).Sync();
    if (!tree || tree->parent == tree->root) {
      return false;
    }
    window = tree->parent;
  }
  return false;
}

x11::Window InputFocus() {
  auto focus = x11::Connection::Get()->GetInputFocus().Sync();
  return focus ? focus->focus : x11::Window::None;
}

// A child widget, one X window, holding one pane's page and its docked DevTools, or the side panel.
struct Slot {
  std::unique_ptr<views::Widget> widget;
  raw_ptr<aura::Window> page = nullptr;
  raw_ptr<aura::Window> devtools = nullptr;
  // From each edge of the window's content to the slot's, kept as the window resizes.
  gfx::Insets insets;
  // With DevTools docked, where the page goes in the slot, from its top-left.
  gfx::Rect page_in_devtools;
  // Only the bottom corners are round, for the side panel under the app's header.
  bool bottom_corners_only = false;
};

// |size| with round corners of |radius|, as the rows X's shape extension takes, in DIPs.
std::unique_ptr<views::Widget::ShapeRects> RoundShape(const gfx::Size& size,
                                                      double radius,
                                                      bool bottom_only) {
  auto rects = std::make_unique<views::Widget::ShapeRects>();
  const int r = std::min<int>(std::lround(radius), std::min(size.width(), size.height()) / 2);
  auto inset_at = [r](int row) {
    const double dy = r - row - 0.5;
    return static_cast<int>(std::lround(r - std::sqrt(r * r - dy * dy)));
  };
  for (int row = 0; row < r; ++row) {
    const int inset = bottom_only ? 0 : inset_at(row);
    rects->emplace_back(inset, row, size.width() - 2 * inset, 1);
  }
  rects->emplace_back(0, r, size.width(), size.height() - 2 * r);
  for (int row = 0; row < r; ++row) {
    const int inset = inset_at(row);
    rects->emplace_back(inset, size.height() - 1 - row, size.width() - 2 * inset, 1);
  }
  return rects;
}

class WindowViews : public views::WidgetObserver, public x11::EventObserver {
 public:
  WindowViews(views::Widget* widget, x11::Window ui) : widget_(widget), ui_(ui) {
    observation_.Observe(widget);
    if (ui_ != x11::Window::None) {
      x11::Connection* connection = x11::Connection::Get();
      children_events_ =
          connection->ScopedSelectEvent(window(), x11::EventMask::SubstructureNotify);
      x11_observation_.Observe(connection);
    }
  }

  x11::Window window() const { return XWindowOf(widget_); }
  x11::Window ui() const { return ui_; }
  std::vector<Slot>& panes() { return panes_; }
  Slot& side_panel() { return side_panel_; }
  double corner_radius() const { return corner_radius_; }
  void set_corner_radius(double radius) { corner_radius_ = radius; }

  // A slot's widget, made the first time it's needed, inside the browser window.
  views::Widget* WidgetFor(Slot& slot) {
    if (!slot.widget) {
      slot.widget = std::make_unique<views::Widget>();
      views::Widget::InitParams params(views::Widget::InitParams::CLIENT_OWNS_WIDGET,
                                       views::Widget::InitParams::TYPE_CONTROL);
      params.child = true;
      params.parent = widget_->GetNativeView();
      params.native_widget = new views::DesktopNativeWidgetAura(slot.widget.get());
      params.name = "BrowserCorePane";
      slot.widget->Init(std::move(params));
      // Before it's first shown, so the window manager never takes it as a window of its own.
      x11::Connection::Get()->ReparentWindow({XWindowOf(slot.widget.get()), window(), 0, 0});
    }
    return slot.widget.get();
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
    if (slot.widget) {
      slot.widget->Hide();
    }
  }

  float Scale() const { return widget_->GetNativeWindow()->GetHost()->device_scale_factor(); }

  gfx::Size ContentPixels() const {
    return widget_->GetNativeWindow()->GetHost()->GetBoundsInPixels().size();
  }

  // The window's content, in DIPs.
  gfx::Size ContentSize() const {
    return gfx::ScaleToFlooredSize(ContentPixels(), 1 / Scale());
  }

  // Sizes the UI to the window's content, and each slot to its insets, over the UI.
  void LayOut() {
    const gfx::Size pixels = ContentPixels();
    if (ui_ != x11::Window::None) {
      x11::Connection::Get()->ConfigureWindow({
          .window = ui_,
          .x = 0,
          .y = 0,
          .width = pixels.width(),
          .height = pixels.height(),
      });
      // GPUI's window, which it opened inside |ui_| on its own connection, follows on its
      // ConfigureNotify.
      if (auto tree = x11::Connection::Get()->QueryTree({ui_}).Sync()) {
        for (x11::Window child : tree->children) {
          x11::Connection::Get()->ConfigureWindow({
              .window = child,
              .x = 0,
              .y = 0,
              .width = pixels.width(),
              .height = pixels.height(),
          });
        }
      }
    }
    const float scale = Scale();
    const gfx::Size size = ContentSize();
    for (Slot& slot : panes_) {
      LayOut(slot, size, scale);
    }
    LayOut(side_panel_, size, scale);
  }

  void LayOut(Slot& slot, const gfx::Size& window_size, float scale) {
    if (!slot.widget || !slot.page) {
      return;
    }
    gfx::Rect frame(window_size);
    frame.Inset(slot.insets);
    const gfx::Rect pixels = gfx::ScaleToEnclosingRect(frame, scale);
    // Moved in the X server rather than through the widget, whose bounds are on the screen: the
    // X window's are from the browser window's corner now. Chromium reads them back as they change.
    x11::Connection::Get()->ConfigureWindow({
        .window = XWindowOf(slot.widget.get()),
        .x = pixels.x(),
        .y = pixels.y(),
        .width = pixels.width(),
        .height = pixels.height(),
        .stack_mode = x11::StackMode::Above,
    });
    const gfx::Rect bounds(frame.size());
    if (slot.devtools) {
      slot.devtools->SetBounds(bounds);
      gfx::Rect page = slot.page_in_devtools;
      page.Intersect(bounds);
      slot.page->SetBounds(page);
    } else {
      slot.page->SetBounds(bounds);
    }
    // shortcut: a shape's corners are aliased, as Windows' regions are and Core Animation's on the
    // Mac aren't; draw the corners in the page's compositor if the edge shows.
    slot.widget->SetShape(RoundShape(frame.size(), corner_radius_, slot.bottom_corners_only));
  }

  // views::WidgetObserver:
  void OnWidgetBoundsChanged(views::Widget*, const gfx::Rect&) override { LayOut(); }

  // x11::EventObserver: a child X window the core didn't put in the browser window, mapped there,
  // is Chromium's output for the window itself, opaque and over the UI until it's sent under it.
  void OnEvent(const x11::Event& event) override {
    auto* map = event.As<x11::MapNotifyEvent>();
    if (!map || map->event != window() || map->window == ui_ || IsSlotWindow(map->window)) {
      return;
    }
    x11::Connection::Get()->ConfigureWindow(
        {.window = map->window, .stack_mode = x11::StackMode::Below});
  }

  void OnWidgetDestroying(views::Widget*) override {
    x11_observation_.Reset();
    children_events_ = x11::ScopedEventSelector();
    for (Slot& slot : panes_) {
      Empty(slot);
    }
    Empty(side_panel_);
    panes_.clear();
    side_panel_ = Slot();
    observation_.Reset();
  }

 private:
  bool IsSlotWindow(x11::Window window) {
    auto holds = [window](const Slot& slot) {
      return slot.widget && XWindowOf(slot.widget.get()) == window;
    };
    return std::ranges::any_of(panes_, holds) || holds(side_panel_);
  }

  raw_ptr<views::Widget> widget_;
  x11::Window ui_;
  std::vector<Slot> panes_;
  Slot side_panel_;
  double corner_radius_ = 0;
  base::ScopedObservation<views::Widget, views::WidgetObserver> observation_{this};
  x11::ScopedEventSelector children_events_;
  base::ScopedObservation<x11::Connection, x11::EventObserver> x11_observation_{this};
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

// The slot whose X window has the keyboard, or null.
Slot* SlotWithFocus(WindowViews* views, x11::Window focus) {
  for (Slot& slot : views->panes()) {
    if (slot.widget && IsWithin(focus, XWindowOf(slot.widget.get()))) {
      return &slot;
    }
  }
  return nullptr;
}

}  // namespace

views::NativeWidget* NewNativeWidget(views::Widget* widget) {
  // shortcut: keeps the window manager's title bar, where the Mac's UI fills it; draw the app's
  // own once the header row is ported.
  return new views::DesktopNativeWidgetAura(widget);
}

void AddWindowViews(views::Widget* widget, void* ui_view) {
  if (OnWayland()) {
    wayland::AddWindowViews(widget);
    return;
  }
  // The app's UI is its X window's id. The app makes it unmapped, so no window manager has taken
  // it as a window of its own.
  const auto ui = static_cast<x11::Window>(reinterpret_cast<uintptr_t>(ui_view));
  auto views = std::make_unique<WindowViews>(widget, ui);
  WindowViews* raw = views.get();
  AllWindowViews()[widget] = std::move(views);
  if (ui != x11::Window::None) {
    x11::Connection* connection = x11::Connection::Get();
    connection->ReparentWindow({ui, raw->window(), 0, 0});
    connection->MapWindow({ui});
    raw->LayOut();
  }
}

void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               size_t focused,
               uint32_t ring_color,
               std::optional<SidePanelPage> side_panel) {
  if (OnWayland()) {
    wayland::ShowPages(widget, panes, corner_radius, std::move(side_panel));
    return;
  }
  WindowViews* views = ViewsOf(widget);
  if (!views) {
    return;
  }
  views->set_corner_radius(corner_radius);
  const bool hidden = !widget->IsVisible();
  const gfx::Size size = views->ContentSize();

  std::vector<Slot>& slots = views->panes();
  // Pages that stay in the same pane keep their slot, and with it the keyboard.
  for (size_t i = panes.size(); i < slots.size(); ++i) {
    views->Empty(slots[i]);
  }
  if (slots.size() < panes.size()) {
    slots.resize(panes.size());
  }
  for (size_t i = 0; i < panes.size(); ++i) {
    const Pane& pane = panes[i];
    Slot& slot = slots[i];
    views::Widget* holder = views->WidgetFor(slot);
    aura::Window* parent = holder->GetNativeView();
    aura::Window* devtools = pane.devtools ? pane.devtools->view : nullptr;
    // A page or DevTools that left this pane leaves its slot.
    // A copy, as Detach changes the list; its raw_ptrs convert one at a time.
    const aura::Window::Windows& children = parent->children();
    for (aura::Window* child : std::vector<aura::Window*>(children.begin(), children.end())) {
      if (child != pane.page && child != devtools) {
        WindowViews::Detach(child);
      }
    }
    if (devtools) {
      Hold(parent, devtools, hidden);
      slot.page_in_devtools = pane.devtools->page_frame;
    }
    Hold(parent, pane.page, hidden);
    if (devtools) {
      if (pane.devtools->page_under) {
        parent->StackChildBelow(pane.page, devtools);
      } else {
        parent->StackChildAbove(pane.page, devtools);
      }
    }
    slot.page = pane.page;
    slot.devtools = devtools;
    slot.insets = InsetsOf(pane.frame, size);
    hidden ? holder->Hide() : holder->ShowInactive();
  }
  // shortcut: a split view's focused pane has no ring on Linux yet; draw it in the app's UI around
  // the pane when split view is ported, as on Windows.
  (void)focused;
  (void)ring_color;

  Slot& panel = views->side_panel();
  if (side_panel) {
    views::Widget* holder = views->WidgetFor(panel);
    aura::Window* parent = holder->GetNativeView();
    // A copy, as Detach changes the list; its raw_ptrs convert one at a time.
    const aura::Window::Windows& children = parent->children();
    for (aura::Window* child : std::vector<aura::Window*>(children.begin(), children.end())) {
      if (child != side_panel->page) {
        WindowViews::Detach(child);
      }
    }
    Hold(parent, side_panel->page, hidden);
    panel.page = side_panel->page;
    panel.insets = InsetsOf(side_panel->frame, size);
    panel.bottom_corners_only = true;
    hidden ? holder->Hide() : holder->ShowInactive();
  } else {
    views->Empty(panel);
  }
  views->LayOut();
}

void HidePages(views::Widget* widget) {
  if (OnWayland()) {
    wayland::HidePages(widget);
    return;
  }
  WindowViews* views = ViewsOf(widget);
  if (!views) {
    return;
  }
  for (Slot& slot : views->panes()) {
    if (slot.widget) {
      slot.widget->Hide();
    }
  }
  if (views->side_panel().widget) {
    views->side_panel().widget->Hide();
  }
}

bool KeyboardForPage(views::Widget* widget) {
  if (OnWayland()) {
    return wayland::KeyboardForPage(widget);
  }
  WindowViews* views = ViewsOf(widget);
  const x11::Window focus = InputFocus();
  // The window itself has it as it opens, or once the view that had it went.
  if (!views || focus == x11::Window::None || focus == views->window()) {
    return true;
  }
  if (views->ui() != x11::Window::None && IsWithin(focus, views->ui())) {
    return false;
  }
  if (Slot* slot = SlotWithFocus(views, focus)) {
    aura::client::FocusClient* client =
        aura::client::GetFocusClient(slot->widget->GetNativeView());
    aura::Window* focused = client ? client->GetFocusedWindow() : nullptr;
    return !slot->devtools || !focused || !slot->devtools->Contains(focused);
  }
  // The side panel, or another window's.
  return false;
}

void KeepKeyboard(views::Widget* widget, base::OnceClosure change) {
  if (OnWayland()) {
    wayland::KeepKeyboard(widget, std::move(change));
    return;
  }
  WindowViews* views = ViewsOf(widget);
  const x11::Window had = InputFocus();
  aura::WindowTracker focused;
  if (views) {
    if (Slot* slot = SlotWithFocus(views, had)) {
      if (auto* client = aura::client::GetFocusClient(slot->widget->GetNativeView())) {
        if (aura::Window* window = client->GetFocusedWindow()) {
          focused.Add(window);
        }
      }
    }
  }
  std::move(change).Run();
  if (!views || had == x11::Window::None || InputFocus() == had ||
      !IsWithin(had, views->window())) {
    return;
  }
  x11::Connection::Get()
      ->SetInputFocus({x11::InputFocus::Parent, had, x11::Time::CurrentTime})
      .IgnoreError();
  if (!focused.windows().empty()) {
    aura::Window* window = focused.windows().front();
    if (auto* client = aura::client::GetFocusClient(window)) {
      client->FocusWindow(window);
    }
  }
}

void StretchOnResize(content::RenderWidgetHostView*, bool) {
  // shortcut: no stretch on Linux; the page shows its last frame unscaled until it redraws.
}

}  // namespace browser_core::platform
