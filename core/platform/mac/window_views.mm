#include "browser_core/platform/window_views.h"

#import <AppKit/AppKit.h>

#include "base/apple/foundation_util.h"
#import "components/remote_cocoa/app_shim/bridged_content_view.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/views/widget/widget.h"

// Holds the app's UI view and, over it, the page shown. Flipped, so frames run from the top-left as
// the core's do.
@interface BrowserCoreWindowView : NSView
@property(strong, nonatomic) NSView* uiView;
@property(weak, nonatomic) NSView* pageView;
// The page's distance from each edge, which it keeps as the window resizes.
@property(nonatomic) NSEdgeInsets pageInsets;
- (void)layOutPage;
@end

@implementation BrowserCoreWindowView
@synthesize uiView = _uiView;
@synthesize pageView = _pageView;
@synthesize pageInsets = _pageInsets;

- (BOOL)isFlipped {
  return YES;
}

// After AppKit's pass rather than by autoresizing, which would resize the page twice: resizing the
// UI view tells the app, which shows the page again at its new frame before AppKit reaches it.
- (void)resizeSubviewsWithOldSize:(NSSize)oldSize {
  [super resizeSubviewsWithOldSize:oldSize];
  [self layOutPage];
}

- (void)layOutPage {
  NSView* page = self.pageView;
  if (page.superview != self) {
    return;
  }
  NSRect bounds = self.bounds;
  NSEdgeInsets insets = self.pageInsets;
  page.frame = NSMakeRect(insets.left, insets.top, NSWidth(bounds) - insets.left - insets.right,
                          NSHeight(bounds) - insets.top - insets.bottom);
}
@end

namespace browser_core::platform {

namespace {

BridgedContentView* ContentView(views::Widget* widget) {
  return base::apple::ObjCCastStrict<BridgedContentView>(
      widget->GetNativeWindow().GetNativeNSWindow().contentView);
}

BrowserCoreWindowView* WindowView(views::Widget* widget) {
  return base::apple::ObjCCastStrict<BrowserCoreWindowView>(ContentView(widget).embeddedView);
}

}  // namespace

void AddWindowViews(views::Widget* widget, void* ui_view) {
  BridgedContentView* content = ContentView(widget);
  BrowserCoreWindowView* view = [[BrowserCoreWindowView alloc] initWithFrame:content.bounds];
  view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  if (NSView* ui = (__bridge NSView*)ui_view) {
    ui.frame = view.bounds;
    ui.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    [view addSubview:ui];
    view.uiView = ui;
  }
  [content addSubview:view];
  // Without this, the content view's hit test and accessibility would skip the page and the UI
  // (engine/patches/ours/chromium-core-embedded-view.patch).
  content.embeddedView = view;
}

void ShowPage(views::Widget* widget, gfx::NativeView page, const gfx::Rect& frame) {
  BrowserCoreWindowView* view = WindowView(widget);
  NSView* page_view = page.GetNativeNSView();
  for (NSView* subview in [view.subviews copy]) {
    if (subview != view.uiView && subview != page_view) {
      [subview removeFromSuperview];
    }
  }
  page_view.autoresizingMask = NSViewNotSizable;
  // A background tab's page view starts hidden, and Chrome's views host shows it. Hidden, the page
  // never draws, and Chromium drops the keys it doesn't use instead of passing them to the window.
  page_view.hidden = NO;
  if (page_view.superview != view) {
    [view addSubview:page_view];
  }
  NSRect bounds = view.bounds;
  NSRect rect = frame.ToCGRect();
  view.pageView = page_view;
  view.pageInsets = NSEdgeInsetsMake(NSMinY(rect), NSMinX(rect), NSHeight(bounds) - NSMaxY(rect),
                                     NSWidth(bounds) - NSMaxX(rect));
  [view layOutPage];
}

bool KeyboardForPage(views::Widget* widget) {
  NSWindow* window = widget->GetNativeWindow().GetNativeNSWindow();
  // AppKit gives the keyboard to the window when the view that had it leaves, and views gives it to
  // the content view as the window opens. Neither holds anything of the core's that takes it.
  if (window.firstResponder == window || window.firstResponder == window.contentView) {
    return true;
  }
  BrowserCoreWindowView* view = WindowView(widget);
  NSView* focused = base::apple::ObjCCast<NSView>(window.firstResponder);
  return [focused isDescendantOf:view] && ![focused isDescendantOf:view.uiView];
}

}  // namespace browser_core::platform
