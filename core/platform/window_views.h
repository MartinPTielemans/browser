// The core's native window and the views it puts in it, per platform.

#ifndef BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
#define BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_

#include <cstddef>
#include <cstdint>
#include <optional>

#include "base/containers/span.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/native_ui_types.h"

namespace content {
class RenderWidgetHostView;
}

namespace views {
class NativeWidget;
class Widget;
}  // namespace views

namespace browser_core::platform {

// The native window for |widget|, whose title bar is the app's: on the Mac, the UI fills it and
// the window's buttons sit in the app's header row.
views::NativeWidget* NewNativeWidget(views::Widget* widget);

// Fills |widget|'s window with the view pages are shown in, holding the app's |ui_view| under
// them: an NSView on the Mac, or null.
void AddWindowViews(views::Widget* widget, void* ui_view);

// A page's DevTools, docked: they fill the page's frame, and the page goes at |page_frame| in it,
// over them, or under them if |page_under|.
struct DockedDevTools {
  gfx::NativeView view;
  gfx::Rect page_frame;
  bool page_under;
};

// A page to show, at |frame| from the top-left of the window's content, with its DevTools if
// they're docked.
struct Pane {
  gfx::NativeView page;
  gfx::Rect frame;
  std::optional<DockedDevTools> devtools;
};

// Shows |panes| in place of the pages shown, with corners of |corner_radius|. With more than one,
// as in a split view, the one at |focused| has a ring of |ring_color|, 0xRRGGBB, inside its edge.
// No panes shows no page.
void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               size_t focused,
               uint32_t ring_color);

// Whether the keyboard in |widget|'s window is the page's: in a page shown, or in nothing, as when
// the window opens or the page view that had it left the window. Not in DevTools.
bool KeyboardForPage(views::Widget* widget);

// Whether |view|, a page's, stretches the frame it last drew to fill it as it resizes, until the
// page draws at its new size, rather than show that frame at its old size in the top-left corner.
void StretchOnResize(content::RenderWidgetHostView* view, bool stretch);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
