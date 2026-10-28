#include "browser_core/platform/window_views.h"

#include <windows.h>

#include <map>
#include <memory>
#include <utility>
#include <vector>

#include "base/functional/callback.h"
#include "base/memory/raw_ptr.h"
#include "base/no_destructor.h"
#include "base/numerics/safe_conversions.h"
#include "base/scoped_observation.h"
#include "ui/aura/client/focus_client.h"
#include "ui/aura/window.h"
#include "ui/aura/window_tracker.h"
#include "ui/display/win/screen_win.h"
#include "ui/gfx/geometry/insets.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/size_conversions.h"
#include "ui/views/widget/desktop_aura/desktop_native_widget_aura.h"
#include "ui/views/widget/widget.h"
#include "ui/views/widget/widget_observer.h"
#include "ui/views/win/hwnd_util.h"

// Aura draws every view of a widget into its one HWND, so a page can't go over a native view in it
// the way an NSView goes over the app's on the Mac. Here the app's UI is a child HWND that fills the
// window, and each page shown sits in a child widget of its own, a child HWND above the UI's.

namespace browser_core::platform {

namespace {

// A child widget, one HWND, holding one pane's page and its docked DevTools, or the side panel.
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

class WindowViews : public views::WidgetObserver {
 public:
  WindowViews(views::Widget* widget, HWND ui) : widget_(widget), ui_(ui) {
    observation_.Observe(widget);
  }

  HWND window() const { return views::HWNDForWidget(widget_); }
  HWND ui() const { return ui_; }
  std::vector<Slot>& panes() { return panes_; }
  Slot& side_panel() { return side_panel_; }
  double corner_radius() const { return corner_radius_; }
  void set_corner_radius(double radius) { corner_radius_ = radius; }

  // A slot's widget, made the first time it's needed.
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

  float Scale() const {
    return display::win::GetScreenWin()->GetScaleFactorForHWND(window());
  }

  // The window's content, in DIPs.
  gfx::Size ContentSize() const {
    RECT client;
    GetClientRect(window(), &client);
    return gfx::ScaleToFlooredSize(gfx::Size(client.right, client.bottom), 1 / Scale());
  }

  // Sizes the UI to the window's content, and each slot to its insets, over the UI.
  void LayOut() {
    RECT client;
    GetClientRect(window(), &client);
    SetWindowPos(ui_, HWND_BOTTOM, 0, 0, client.right, client.bottom,
                 SWP_NOACTIVATE | SWP_NOOWNERZORDER);
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
    HWND hwnd = views::HWNDForWidget(slot.widget.get());
    SetWindowPos(hwnd, HWND_TOP, pixels.x(), pixels.y(), pixels.width(), pixels.height(),
                 SWP_NOACTIVATE | SWP_NOOWNERZORDER);
    const gfx::Rect bounds(frame.size());
    if (slot.devtools) {
      slot.devtools->SetBounds(bounds);
      gfx::Rect page = slot.page_in_devtools;
      page.Intersect(bounds);
      slot.page->SetBounds(page);
    } else {
      slot.page->SetBounds(bounds);
    }
    // shortcut: a window region's corners are aliased, unlike Core Animation's on the Mac; upgrade
    // to a DirectComposition clip if the edge shows.
    const int diameter = base::ClampRound(2 * corner_radius_ * scale);
    HRGN region = CreateRoundRectRgn(0, 0, pixels.width() + 1, pixels.height() + 1, diameter,
                                     diameter);
    if (slot.bottom_corners_only) {
      HRGN top = CreateRectRgn(0, 0, pixels.width(), pixels.height() / 2);
      CombineRgn(region, region, top, RGN_OR);
      DeleteObject(top);
    }
    // The window owns the region from here.
    SetWindowRgn(hwnd, region, TRUE);
  }

  // views::WidgetObserver:
  void OnWidgetBoundsChanged(views::Widget*, const gfx::Rect&) override { LayOut(); }

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
  HWND ui_;
  std::vector<Slot> panes_;
  Slot side_panel_;
  double corner_radius_ = 0;
  base::ScopedObservation<views::Widget, views::WidgetObserver> observation_{this};
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

}  // namespace

views::NativeWidget* NewNativeWidget(views::Widget* widget, base::RepeatingClosure) {
  // shortcut: keeps Windows' title bar, where the Mac's UI fills it; give the window a custom frame
  // with the app's caption buttons when the header row is ported. Windows has no full screen of
  // its own, so the window leaves it only when Chrome asks.
  return new views::DesktopNativeWidgetAura(widget);
}

void AddWindowViews(views::Widget* widget, void* ui_view) {
  HWND ui = static_cast<HWND>(ui_view);
  HWND window = views::HWNDForWidget(widget);
  if (ui) {
    // The app's UI was a window of its own until now, made before the core's.
    SetWindowLongPtr(ui, GWL_STYLE,
                     (GetWindowLongPtr(ui, GWL_STYLE) & ~WS_POPUP) | WS_CHILD | WS_VISIBLE);
    SetParent(ui, window);
  }
  auto views = std::make_unique<WindowViews>(widget, ui);
  WindowViews* raw = views.get();
  AllWindowViews()[widget] = std::move(views);
  if (ui) {
    raw->LayOut();
  }
}

void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               size_t focused,
               uint32_t ring_color,
               std::optional<SidePanelPage> side_panel) {
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
    for (aura::Window* child : aura::Window::Windows(parent->children())) {
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
  // shortcut: a split view's focused pane has no ring on Windows yet; draw it in the app's UI around
  // the pane, or as a layered child HWND, when split view is ported.
  (void)focused;
  (void)ring_color;

  Slot& panel = views->side_panel();
  if (side_panel) {
    views::Widget* holder = views->WidgetFor(panel);
    aura::Window* parent = holder->GetNativeView();
    for (aura::Window* child : aura::Window::Windows(parent->children())) {
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
  WindowViews* views = ViewsOf(widget);
  HWND focus = GetFocus();
  // The window itself has it as it opens, or once the view that had it went.
  if (!views || !focus || focus == views->window()) {
    return true;
  }
  if (focus == views->ui() || IsChild(views->ui(), focus)) {
    return false;
  }
  for (Slot& slot : views->panes()) {
    if (!slot.widget || focus != views::HWNDForWidget(slot.widget.get())) {
      continue;
    }
    aura::client::FocusClient* client =
        aura::client::GetFocusClient(slot.widget->GetNativeView());
    aura::Window* focused = client ? client->GetFocusedWindow() : nullptr;
    return !slot.devtools || !focused || !slot.devtools->Contains(focused);
  }
  // The side panel, or another window's.
  return false;
}

void KeepKeyboard(views::Widget* widget, base::OnceClosure change) {
  WindowViews* views = ViewsOf(widget);
  HWND had = GetFocus();
  aura::WindowTracker focused;
  if (views) {
    for (Slot& slot : views->panes()) {
      if (slot.widget && had == views::HWNDForWidget(slot.widget.get())) {
        if (auto* client = aura::client::GetFocusClient(slot.widget->GetNativeView())) {
          if (aura::Window* window = client->GetFocusedWindow()) {
            focused.Add(window);
          }
        }
      }
    }
  }
  std::move(change).Run();
  if (!views || GetFocus() == had || !IsWindow(had) ||
      (had != views->window() && !IsChild(views->window(), had))) {
    return;
  }
  SetFocus(had);
  if (!focused.windows().empty()) {
    aura::Window* window = focused.windows().front();
    if (auto* client = aura::client::GetFocusClient(window)) {
      client->FocusWindow(window);
    }
  }
}

void StretchOnResize(content::RenderWidgetHostView*, bool) {
  // shortcut: no stretch on Windows; the page shows its last frame unscaled until it redraws.
}

}  // namespace browser_core::platform
