#include "browser_core/platform/window_views.h"

#import <AppKit/AppKit.h>

#import "components/remote_cocoa/app_shim/native_widget_mac_nswindow.h"
#include "components/remote_cocoa/common/native_widget_ns_window.mojom.h"
#include "ui/base/cocoa/window_size_constants.h"
#include "ui/views/widget/native_widget_mac.h"

namespace {

// The window's buttons sit in the app's header row where Arc's do: centred in the top 48 points,
// the first 13 from the left edge, so their centres are 20, 43 and 66 across and 24 down
// (docs/ux-spec.md, Window).
constexpr CGFloat kButtonRowHeight = 48;
constexpr CGFloat kButtonInset = 13;

}  // namespace

// AppKit's private calls that place the window's buttons, which Chrome's browser window overrides
// too (components/remote_cocoa/app_shim/browser_native_widget_window_mac.mm).
@interface NSWindow (BrowserCorePrivate)
+ (Class)frameViewClassForStyleMask:(NSWindowStyleMask)windowStyle;
@end

@interface NSThemeFrame (BrowserCorePrivate)
- (CGFloat)_titlebarHeight;
- (void)setStyleMask:(NSUInteger)styleMask;
@end

// The window's frame view, which puts the buttons in the header row.
@interface BrowserCoreWindowFrame : NativeWidgetMacNSWindowTitledFrame
@end

@implementation BrowserCoreWindowFrame {
  BOOL _inFullScreen;
}

- (CGFloat)_titlebarHeight {
  // Full screen keeps AppKit's title bar, which comes down with the menu bar.
  return _inFullScreen ? [super _titlebarHeight] : kButtonRowHeight;
}

- (CGFloat)_minXTitlebarWidgetInset {
  return kButtonInset;
}

- (BOOL)_shouldCenterTrafficLights {
  return YES;
}

- (void)setStyleMask:(NSUInteger)styleMask {
  _inFullScreen = (styleMask & NSWindowStyleMaskFullScreen) != 0;
  [super setStyleMask:styleMask];
}

@end

@interface BrowserCoreNSWindow : NativeWidgetMacNSWindow
@end

@implementation BrowserCoreNSWindow

+ (Class)frameViewClassForStyleMask:(NSWindowStyleMask)windowStyle {
  // Nil if a future macOS has no NSThemeFrame, which leaves AppKit's own.
  if ((windowStyle & NSWindowStyleMaskTitled) && [BrowserCoreWindowFrame class]) {
    return [BrowserCoreWindowFrame class];
  }
  return [super frameViewClassForStyleMask:windowStyle];
}

@end

namespace browser_core::platform {

namespace {

class NativeWidget : public views::NativeWidgetMac {
 public:
  using views::NativeWidgetMac::NativeWidgetMac;

 protected:
  // As NativeWidgetNSWindowBridge::CreateNSWindow makes a default window, with the frame above.
  NativeWidgetMacNSWindow* CreateNSWindow(
      const remote_cocoa::mojom::CreateWindowParams* params) override {
    // The app's UI fills the window, title bar and all.
    NativeWidgetMacNSWindow* window = [[BrowserCoreNSWindow alloc]
        initWithContentRect:ui::kWindowSizeDeterminedLater
                  styleMask:params->style_mask | NSWindowStyleMaskFullSizeContentView
                    backing:NSBackingStoreBuffered
                      defer:NO];
    window.releasedWhenClosed = NO;
    window.titlebarAppearsTransparent = YES;
    window.titleVisibility = NSWindowTitleHidden;
    if (params->animation_enabled) {
      window.animationBehavior = NSWindowAnimationBehaviorDocumentWindow;
    }
    return window;
  }
};

}  // namespace

views::NativeWidget* NewNativeWidget(views::Widget* widget) {
  // It deletes itself as its window closes.
  return new NativeWidget(widget);
}

}  // namespace browser_core::platform
