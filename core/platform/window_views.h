// The core's native window and the views it puts in it, per platform.

#ifndef BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
#define BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_

#include <cstddef>
#include <cstdint>
#include <optional>

#include "base/containers/span.h"
#include "base/functional/callback_forward.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/native_ui_types.h"

namespace content {
class RenderWidgetHostView;
}  // namespace content

namespace views {
class NativeWidget;
class Widget;
}  // namespace views

namespace browser_core::platform {

// The native window for |widget|, whose title bar is the app's: on the Mac, the UI fills it and
// the window's buttons sit in the app's header row. |leaving_full_screen| runs as the window starts
// to leave full screen, on a platform whose window can be taken out without finishing (the Mac's).
views::NativeWidget* NewNativeWidget(views::Widget* widget,
                                     base::RepeatingClosure leaving_full_screen);

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

// An extension's side panel page, at |frame| like a pane's. The app draws its header above it, so
// only its bottom corners are round.
struct SidePanelPage {
  gfx::NativeView page;
  gfx::Rect frame;
};

// Shows |panes| in place of the pages shown, with corners of |corner_radius|, and |side_panel| in
// place of the side panel shown. With more than one pane, as in a split view, the one at |focused|
// has a ring of |ring_color|, 0xRRGGBB, inside its edge. No panes shows no page. While |widget|
// isn't visible, the pages stay hidden.
void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               size_t focused,
               uint32_t ring_color,
               std::optional<SidePanelPage> side_panel);

// Hides the pages shown, their DevTools and the side panel, in place, for a window out of sight, so
// Chromium counts them hidden. ShowPages shows them again.
void HidePages(views::Widget* widget);

// Whether the keyboard in |widget|'s window is the page's: in a page shown, or in nothing, as when
// the window opens or the page view that had it left the window. Not in DevTools or the side
// panel.
bool KeyboardForPage(views::Widget* widget);

// Runs |change|, which may move the keyboard in |widget|'s window, then gives the keyboard back to
// what had it, if that's still in the window.
void KeepKeyboard(views::Widget* widget, base::OnceClosure change);

// Whether |view|, a page's, stretches the frame it last drew to fill itself as it resizes, until
// the page draws at the new size, rather than shows that frame at its old size in the top-left
// corner. Null for a page with no view yet.
void StretchOnResize(content::RenderWidgetHostView* view, bool stretch);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
