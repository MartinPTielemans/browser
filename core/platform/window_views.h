// The core's native window and the views it puts in it, per platform.

#ifndef BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
#define BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_

#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/native_ui_types.h"

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

// Shows |page| at |frame|, from the top-left of the window's content, with corners of
// |corner_radius|, in place of the page shown, and its |devtools| if they're docked. A null |page|
// shows none.
void ShowPage(views::Widget* widget,
              gfx::NativeView page,
              const gfx::Rect& frame,
              double corner_radius,
              const DockedDevTools* devtools);

// Whether the keyboard in |widget|'s window is the page's: in the page shown, or in nothing, as
// when the window opens or the page view that had it left the window. Not in its DevTools.
bool KeyboardForPage(views::Widget* widget);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
