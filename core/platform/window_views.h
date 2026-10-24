// The core's native window and the views it puts in it, per platform.

#ifndef BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
#define BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_

#include "ui/gfx/native_ui_types.h"

namespace gfx {
class Rect;
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

// Shows |page| at |frame|, from the top-left of the window's content, in place of the page shown.
void ShowPage(views::Widget* widget, gfx::NativeView page, const gfx::Rect& frame);

// Whether the keyboard in |widget|'s window is the page's: in the page shown, or in nothing, as
// when the window opens or the page view that had it left the window.
bool KeyboardForPage(views::Widget* widget);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_WINDOW_VIEWS_H_
