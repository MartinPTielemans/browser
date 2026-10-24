#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <initializer_list>

#include "host.h"

namespace {

// The browser window, once shown.
NSWindow* g_window;

// The command bar's window and view.
NSWindow* g_overlay;
NSView* g_overlay_view;
bool g_overlay_shown = false;

// The main display, or with BROWSER_DISPLAY=builtin the Mac's own, so a check doesn't open over
// what the owner is watching on another.
NSScreen* WindowScreen() {
  NSScreen* main = NSScreen.screens.firstObject;
  if (strcmp(getenv("BROWSER_DISPLAY") ?: "", "builtin") != 0) {
    return main;
  }
  for (NSScreen* screen in NSScreen.screens) {
    if (CGDisplayIsBuiltin([screen.deviceDescription[@"NSScreenNumber"] unsignedIntValue])) {
      return screen;
    }
  }
  return main;
}

}  // namespace

void host_centered(double width, double height, double* x, double* y) {
  NSScreen* main = NSScreen.screens.firstObject;
  NSRect visible = WindowScreen().visibleFrame;
  *x = NSMidX(visible) - width / 2;
  // AppKit's y runs up from the bottom of the main display, the core's down from its top.
  *y = NSHeight(main.frame) - NSMidY(visible) - height / 2;
}

void* host_ui_view(void) {
  // Retained by the window the core puts it in.
  return (__bridge_retained void*)[[NSView alloc] initWithFrame:NSZeroRect];
}

void host_show_window(void* ui_view) {
  g_window = ((__bridge NSView*)ui_view).window;
  [g_window makeKeyAndOrderFront:nil];
  [NSApp activateIgnoringOtherApps:YES];

  // A click outside the command bar closes it first, so the click lands the way it would with
  // the bar closed. Otherwise it only makes the browser window key again: the page doesn't take
  // the click that activates its window.
  constexpr NSEventMask kMouseDown =
      NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  [NSEvent addLocalMonitorForEventsMatchingMask:kMouseDown
                                        handler:^NSEvent*(NSEvent* event) {
                                          if (g_overlay_shown && event.window != g_overlay) {
                                            host_hide_overlay();
                                          }
                                          return event;
                                        }];
}

void host_add_overlay(void* overlay_view) {
  g_overlay_view = (__bridge NSView*)overlay_view;
  g_overlay = g_overlay_view.window;
  // GPUI gives the window a title bar. Keeping it (transparent) keeps the system's rounded
  // corners; only its buttons go.
  for (NSWindowButton button :
       {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    [g_overlay standardWindowButton:button].hidden = YES;
  }
  // A child window stays above its parent, so it doesn't need GPUI's floating level, which
  // would also put it above other apps' windows.
  g_overlay.level = NSNormalWindowLevel;
  // The overlay closes when the user leaves it: a click outside it, another window taking the
  // keyboard, or switching apps. AppKit would otherwise hide the panel when the app deactivates
  // and show it again on return.
  ((NSPanel*)g_overlay).hidesOnDeactivate = NO;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    host_hide_overlay();
                  }];
  // Chromium makes its window key by itself, such as when a page commits and its view replaces
  // the one that had focus. That isn't the user leaving, so the overlay takes the keyboard back.
  [center addObserverForName:NSWindowDidBecomeKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    if (!g_overlay_shown || notification.object == g_overlay) {
                      return;
                    }
                    if (notification.object == g_window) {
                      [g_overlay makeKeyWindow];
                    } else {
                      host_hide_overlay();
                    }
                  }];
}

void host_show_overlay(void) {
  if (!g_overlay || !g_window) {
    return;
  }
  if (g_overlay_shown) {
    [g_overlay makeKeyAndOrderFront:nil];
    return;
  }
  g_overlay_shown = true;
  // The window still holds what it drew when it last closed, such as the previous query. It goes
  // on screen once the UI has drawn it as it is now, which can't happen while the UI is still
  // handling the shortcut that opened it.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_overlay_shown) {
      return;
    }
    [g_overlay_view.layer setNeedsDisplay];
    [g_overlay_view.layer displayIfNeeded];
    // Centered, a fifth of the way down the browser window. Arc's placement isn't measured yet.
    NSRect frame = g_window.frame;
    NSSize size = g_overlay.frame.size;
    [g_overlay setFrameOrigin:NSMakePoint(NSMidX(frame) - size.width / 2,
                                          NSMaxY(frame) - NSHeight(frame) / 5 - size.height)];
    [g_window addChildWindow:g_overlay ordered:NSWindowAbove];
    [g_overlay makeKeyAndOrderFront:nil];
    fprintf(stderr, "browser: command bar shown\n");
  });
}

void host_hide_overlay(void) {
  if (!g_overlay_shown) {
    return;
  }
  g_overlay_shown = false;
  [g_window removeChildWindow:g_overlay];
  [g_overlay orderOut:nil];
  [g_window makeKeyWindow];
  fprintf(stderr, "browser: command bar hidden\n");
}
