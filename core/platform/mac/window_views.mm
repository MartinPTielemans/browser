#include "browser_core/platform/window_views.h"

#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

#include "base/apple/foundation_util.h"
#import "components/remote_cocoa/app_shim/bridged_content_view.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/views/widget/widget.h"

// Holds the app's UI view and, over it, the page shown, with its DevTools when they're docked.
// Flipped, so frames run from the top-left as the core's do.
@interface BrowserCoreWindowView : NSView
@property(strong, nonatomic) NSView* uiView;
@property(weak, nonatomic) NSView* pageView;
@property(weak, nonatomic) NSView* devtoolsView;
// The distance from each edge of the page's frame, which it keeps as the window resizes.
@property(nonatomic) NSEdgeInsets pageInsets;
// With DevTools docked, where the page goes in its frame, from the frame's top-left.
@property(nonatomic) NSRect pageInDevTools;
- (void)layOutPage;
@end

@implementation BrowserCoreWindowView
@synthesize uiView = _uiView;
@synthesize pageView = _pageView;
@synthesize devtoolsView = _devtoolsView;
@synthesize pageInsets = _pageInsets;
@synthesize pageInDevTools = _pageInDevTools;

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
  NSRect frame = NSMakeRect(insets.left, insets.top, NSWidth(bounds) - insets.left - insets.right,
                            NSHeight(bounds) - insets.top - insets.bottom);
  NSView* devtools = self.devtoolsView;
  if (devtools.superview != self) {
    page.frame = frame;
    return;
  }
  devtools.frame = frame;
  // The core places the page again for each new frame, and DevTools once they've laid out at it.
  page.frame = NSIntersectionRect(
      NSOffsetRect(self.pageInDevTools, NSMinX(frame), NSMinY(frame)), frame);
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

// Shows a page's view, or its DevTools', which Chrome's views host leaves hidden while their tab is
// in the background. Hidden, a page never draws, and Chromium drops the keys it doesn't use instead
// of passing them to the window.
void Unhide(NSView* view) {
  view.autoresizingMask = NSViewNotSizable;
  view.hidden = NO;
  // Clipped by Core Animation, which rounds the compositor's layers under it without redrawing
  // them.
  view.wantsLayer = YES;
}

void RoundCorners(NSView* view, double radius, CACornerMask corners) {
  view.layer.cornerRadius = radius;
  view.layer.maskedCorners = corners;
  view.layer.masksToBounds = radius > 0 && corners;
}

// The corners of |frame| that |rect|, in it from its top-left, reaches.
CACornerMask CornersReached(NSView* view, NSRect rect, NSSize frame) {
  const bool left = NSMinX(rect) <= 0;
  const bool right = NSMaxX(rect) >= frame.width;
  const bool top = NSMinY(rect) <= 0;
  const bool bottom = NSMaxY(rect) >= frame.height;
  // A layer's minimum Y is its view's top if the view is flipped.
  const bool min_y = view.isFlipped ? top : bottom;
  const bool max_y = view.isFlipped ? bottom : top;
  return (left && min_y ? kCALayerMinXMinYCorner : 0) |
         (right && min_y ? kCALayerMaxXMinYCorner : 0) |
         (left && max_y ? kCALayerMinXMaxYCorner : 0) |
         (right && max_y ? kCALayerMaxXMaxYCorner : 0);
}

constexpr CACornerMask kAllCorners = kCALayerMinXMinYCorner | kCALayerMaxXMinYCorner |
                                     kCALayerMinXMaxYCorner | kCALayerMaxXMaxYCorner;

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

void ShowPage(views::Widget* widget,
              gfx::NativeView page,
              const gfx::Rect& frame,
              double corner_radius,
              const DockedDevTools* devtools) {
  BrowserCoreWindowView* view = WindowView(widget);
  NSView* page_view = page.GetNativeNSView();
  NSView* devtools_view = devtools ? devtools->view.GetNativeNSView() : nil;
  for (NSView* subview in [view.subviews copy]) {
    if (subview != view.uiView && subview != page_view && subview != devtools_view) {
      [subview removeFromSuperview];
    }
  }
  view.devtoolsView = devtools_view;
  if (!page_view) {
    view.pageView = nil;
    return;
  }
  Unhide(page_view);
  if (devtools_view) {
    Unhide(devtools_view);
    RoundCorners(devtools_view, corner_radius, kAllCorners);
    view.pageInDevTools = devtools->page_frame.ToCGRect();
    RoundCorners(page_view, corner_radius,
                 CornersReached(page_view, view.pageInDevTools, frame.size().ToCGSize()));
    if (devtools_view.superview != view) {
      [view addSubview:devtools_view positioned:NSWindowAbove relativeTo:view.uiView];
    }
  } else {
    RoundCorners(page_view, corner_radius, kAllCorners);
  }
  if (page_view.superview != view) {
    [view addSubview:page_view];
  }
  if (devtools_view) {
    // Only when it's out of order: a view moved loses the keyboard.
    NSArray<NSView*>* subviews = view.subviews;
    const bool page_over =
        [subviews indexOfObject:page_view] > [subviews indexOfObject:devtools_view];
    if (page_over == devtools->page_under) {
      [view addSubview:page_view
            positioned:devtools->page_under ? NSWindowBelow : NSWindowAbove
            relativeTo:devtools_view];
    }
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
  return [focused isDescendantOf:view] && ![focused isDescendantOf:view.uiView] &&
         !(view.devtoolsView && [focused isDescendantOf:view.devtoolsView]);
}

}  // namespace browser_core::platform
