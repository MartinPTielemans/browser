#include "browser_core/platform/window_views.h"

#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

#include "base/apple/foundation_util.h"
#import "components/remote_cocoa/app_shim/bridged_content_view.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/views/widget/widget.h"

// A page the window shows, with its DevTools when they're docked.
@interface BrowserCorePane : NSObject
@property(weak, nonatomic) NSView* page;
@property(weak, nonatomic) NSView* devtools;
// The distance from each edge of the window to the page's frame, which it keeps as the window
// resizes.
@property(nonatomic) NSEdgeInsets insets;
// With DevTools docked, where the page goes in its frame, from the frame's top-left.
@property(nonatomic) NSRect pageInDevTools;
@end

@implementation BrowserCorePane
@synthesize page = _page;
@synthesize devtools = _devtools;
@synthesize insets = _insets;
@synthesize pageInDevTools = _pageInDevTools;
@end

// The ring inside the focused page's edge in a split view, over the page. Clicks go through it.
@interface BrowserCoreFocusRing : NSView
@end

@implementation BrowserCoreFocusRing
- (NSView*)hitTest:(NSPoint)point {
  return nil;
}
@end

// Holds the app's UI view and, over it, the pages shown, left to right, with the focused one's ring
// in a split view. Flipped, so frames run from the top-left as the core's do.
@interface BrowserCoreWindowView : NSView
@property(strong, nonatomic) NSView* uiView;
@property(copy, nonatomic) NSArray<BrowserCorePane*>* panes;
@property(nonatomic) NSUInteger focused;
@property(strong, nonatomic) BrowserCoreFocusRing* ring;
- (void)layOutPage;
@end

@implementation BrowserCoreWindowView
@synthesize uiView = _uiView;
@synthesize panes = _panes;
@synthesize focused = _focused;
@synthesize ring = _ring;

- (BOOL)isFlipped {
  return YES;
}

// After AppKit's pass rather than by autoresizing, which would resize the page twice: resizing the
// UI view tells the app, which shows the page again at its new frame before AppKit reaches it.
- (void)resizeSubviewsWithOldSize:(NSSize)oldSize {
  [super resizeSubviewsWithOldSize:oldSize];
  [self layOutPage];
}

- (NSRect)frameOf:(BrowserCorePane*)pane {
  NSRect bounds = self.bounds;
  NSEdgeInsets insets = pane.insets;
  return NSMakeRect(insets.left, insets.top, NSWidth(bounds) - insets.left - insets.right,
                    NSHeight(bounds) - insets.top - insets.bottom);
}

- (void)layOutPage {
  for (BrowserCorePane* pane in self.panes) {
    NSView* page = pane.page;
    if (page.superview != self) {
      continue;
    }
    NSRect frame = [self frameOf:pane];
    NSView* devtools = pane.devtools;
    if (devtools.superview != self) {
      page.frame = frame;
      continue;
    }
    devtools.frame = frame;
    // The core places the page again for each new frame, and DevTools once they've laid out at it.
    page.frame = NSIntersectionRect(
        NSOffsetRect(pane.pageInDevTools, NSMinX(frame), NSMinY(frame)), frame);
  }
  if (self.ring.superview == self && self.focused < self.panes.count) {
    self.ring.frame = [self frameOf:self.panes[self.focused]];
  }
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

void ShowPages(views::Widget* widget,
               base::span<const Pane> panes,
               double corner_radius,
               size_t focused,
               uint32_t ring_color) {
  BrowserCoreWindowView* view = WindowView(widget);
  NSMutableSet<NSView*>* staying = [NSMutableSet set];
  for (const Pane& pane : panes) {
    [staying addObject:pane.page.GetNativeNSView()];
    if (pane.devtools) {
      [staying addObject:pane.devtools->view.GetNativeNSView()];
    }
  }
  for (NSView* subview in [view.subviews copy]) {
    if (subview != view.uiView && subview != view.ring && ![staying containsObject:subview]) {
      [subview removeFromSuperview];
    }
  }
  NSRect bounds = view.bounds;
  NSMutableArray<BrowserCorePane*>* shown = [NSMutableArray array];
  for (const Pane& pane : panes) {
    NSView* page_view = pane.page.GetNativeNSView();
    NSView* devtools_view = pane.devtools ? pane.devtools->view.GetNativeNSView() : nil;
    BrowserCorePane* record = [[BrowserCorePane alloc] init];
    record.page = page_view;
    record.devtools = devtools_view;
    Unhide(page_view);
    if (devtools_view) {
      Unhide(devtools_view);
      RoundCorners(devtools_view, corner_radius, kAllCorners);
      record.pageInDevTools = pane.devtools->page_frame.ToCGRect();
      RoundCorners(page_view, corner_radius,
                   CornersReached(page_view, record.pageInDevTools, pane.frame.size().ToCGSize()));
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
      if (page_over == pane.devtools->page_under) {
        [view addSubview:page_view
              positioned:pane.devtools->page_under ? NSWindowBelow : NSWindowAbove
              relativeTo:devtools_view];
      }
    }
    NSRect rect = pane.frame.ToCGRect();
    record.insets = NSEdgeInsetsMake(NSMinY(rect), NSMinX(rect), NSHeight(bounds) - NSMaxY(rect),
                                     NSWidth(bounds) - NSMaxX(rect));
    [shown addObject:record];
  }
  view.panes = shown;
  view.focused = focused;
  if (shown.count > 1) {
    if (!view.ring) {
      view.ring = [[BrowserCoreFocusRing alloc] init];
      view.ring.wantsLayer = YES;
      // Arc's: 2 pt, inside the page's edge.
      view.ring.layer.borderWidth = 2;
    }
    view.ring.layer.cornerRadius = corner_radius;
    view.ring.layer.borderColor = [NSColor colorWithSRGBRed:((ring_color >> 16) & 0xff) / 255.0
                                                      green:((ring_color >> 8) & 0xff) / 255.0
                                                       blue:(ring_color & 0xff) / 255.0
                                                      alpha:1]
                                      .CGColor;
    // Moving the ring, which never has the keyboard, rather than a page.
    if (view.subviews.lastObject != view.ring) {
      [view addSubview:view.ring positioned:NSWindowAbove relativeTo:nil];
    }
  } else {
    [view.ring removeFromSuperview];
  }
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
  if (![focused isDescendantOf:view] || [focused isDescendantOf:view.uiView]) {
    return false;
  }
  for (BrowserCorePane* pane in view.panes) {
    if (pane.devtools && [focused isDescendantOf:pane.devtools]) {
      return false;
    }
  }
  return true;
}

}  // namespace browser_core::platform
