#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <initializer_list>

#include "host.h"

// Takes the menu's choice, and says when it shows.
@interface HostMenuTarget : NSObject <NSMenuDelegate>
@property(nonatomic) NSInteger chosen;
@end

@implementation HostMenuTarget
@synthesize chosen = _chosen;

- (void)choose:(NSMenuItem*)item {
  self.chosen = item.tag;
}

- (void)menuWillOpen:(NSMenu*)menu {
  // check.sh waits for this line before it chooses.
  NSMutableArray<NSString*>* titles = [NSMutableArray array];
  for (NSMenuItem* item in menu.itemArray) {
    [titles addObject:item.enabled ? item.title
                                   : [item.title stringByAppendingString:@" (disabled)"]];
  }
  fprintf(stderr, "browser: menu shown %s\n",
          [titles componentsJoinedByString:@", "].UTF8String);
}
@end

namespace {

// The browser window, once shown.
NSWindow* g_window;

// The command bar's window and view.
NSWindow* g_overlay;
NSView* g_overlay_view;
bool g_overlay_shown = false;

// The find bar's window and view, and its distance from the top right of the browser window's
// content.
NSWindow* g_find_bar;
NSView* g_find_bar_view;
bool g_find_bar_shown = false;
double g_find_bar_right;
double g_find_bar_top;
// The find bar has the keyboard, or gets it back once the command bar closes: it took it opening
// or from a click, and the user hasn't clicked the browser window since. AppKit also makes it key
// when the command bar closes, which isn't the user choosing it.
bool g_find_bar_keyboard = false;

// The sidebar reveal's window and view, the shell's handler for the pointer, the window's width,
// and while it shows, the monitor that hands the handler the pointer.
NSWindow* g_reveal;
NSView* g_reveal_view;
void (*g_reveal_pointer)(double x);
double g_reveal_width;
id g_reveal_monitor;

// The menu or alert showing, for host_choose.
NSMenu* g_menu;
NSAlert* g_alert;

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

// GPUI gives a window a title bar. Keeping it (transparent) keeps the system's rounded corners;
// only its buttons go. A child window stays above its parent, so it doesn't need GPUI's floating
// level, which would also put it above other apps' windows.
void MakeOverlay(NSWindow* window) {
  for (NSWindowButton button :
       {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    [window standardWindowButton:button].hidden = YES;
  }
  window.level = NSNormalWindowLevel;
  // AppKit would otherwise hide the panel when the app deactivates and show it again on return.
  ((NSPanel*)window).hidesOnDeactivate = NO;
}

void PlaceFindBar() {
  NSRect content = [g_window contentRectForFrameRect:g_window.frame];
  NSSize size = g_find_bar.frame.size;
  [g_find_bar setFrameOrigin:NSMakePoint(NSMaxX(content) - g_find_bar_right - size.width,
                                         NSMaxY(content) - g_find_bar_top - size.height)];
}

// At the browser window's left edge and as tall as its content, so a point is at the same place in
// both windows.
void PlaceReveal() {
  NSRect content = [g_window contentRectForFrameRect:g_window.frame];
  [g_reveal setFrame:NSMakeRect(NSMinX(content), NSMinY(content), g_reveal_width,
                                NSHeight(content))
             display:NO];
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
  // The find bar stays open, and a click on the browser window takes the keyboard from it.
  [NSEvent addLocalMonitorForEventsMatchingMask:kMouseDown
                                        handler:^NSEvent*(NSEvent* event) {
                                          if (event.window == g_window) {
                                            g_find_bar_keyboard = false;
                                            if (g_find_bar.keyWindow) {
                                              [g_window makeKeyWindow];
                                            }
                                          } else if (event.window == g_find_bar) {
                                            g_find_bar_keyboard = true;
                                          }
                                          if (g_overlay_shown && event.window != g_overlay) {
                                            host_hide_overlay();
                                          }
                                          return event;
                                        }];
  // A window resizes from its top left corner as often as its bottom right, and AppKit moves a
  // child window only with its parent's bottom left.
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidResizeNotification
                                                  object:g_window
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                if (g_find_bar_shown) {
                                                  PlaceFindBar();
                                                }
                                              }];
}

void host_add_overlay(void* overlay_view) {
  g_overlay_view = (__bridge NSView*)overlay_view;
  g_overlay = g_overlay_view.window;
  MakeOverlay(g_overlay);
  // The overlay closes when the user leaves it: a click outside it, another window taking the
  // keyboard, or switching apps.
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
                    NSWindow* window = notification.object;
                    if (g_overlay_shown && window != g_overlay) {
                      if (window == g_window) {
                        [g_overlay makeKeyWindow];
                      } else {
                        host_hide_overlay();
                      }
                    } else if (!g_overlay_shown && window == g_window && g_find_bar_shown &&
                               g_find_bar_keyboard) {
                      // The same for the find bar, which the user left only by clicking.
                      [g_find_bar makeKeyWindow];
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

void host_set_overlay_height(double height) {
  // After the UI's update that asked, since the overlay draws at its new size here.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_overlay || NSHeight(g_overlay.frame) == height) {
      return;
    }
    NSRect frame = g_overlay.frame;
    // Keeping its top edge, so the rows grow down from the input.
    [g_overlay setFrame:NSMakeRect(NSMinX(frame), NSMaxY(frame) - height, NSWidth(frame), height)
                display:NO];
    [g_overlay_view.layer setNeedsDisplay];
    [g_overlay_view.layer displayIfNeeded];
    fprintf(stderr, "browser: command bar height %.0f\n", height);
  });
}

void host_hide_overlay(void) {
  if (!g_overlay_shown) {
    return;
  }
  g_overlay_shown = false;
  [g_window removeChildWindow:g_overlay];
  [g_overlay orderOut:nil];
  [g_find_bar_shown && g_find_bar_keyboard ? g_find_bar : g_window makeKeyWindow];
  fprintf(stderr, "browser: command bar hidden\n");
}

void host_add_find_bar(void* find_bar_view, double right, double top) {
  g_find_bar_view = (__bridge NSView*)find_bar_view;
  g_find_bar = g_find_bar_view.window;
  g_find_bar_right = right;
  g_find_bar_top = top;
  MakeOverlay(g_find_bar);
}

void host_show_find_bar(bool keyboard) {
  if (!g_find_bar || !g_window) {
    return;
  }
  g_find_bar_keyboard = g_find_bar_keyboard || keyboard;
  if (g_find_bar_shown) {
    // Once it's on screen. Until then, it takes the keyboard as it goes on.
    if (keyboard && g_find_bar.visible) {
      [g_find_bar makeKeyAndOrderFront:nil];
    }
    return;
  }
  g_find_bar_shown = true;
  // As the command bar does: it goes on screen once it has drawn what it shows now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_find_bar_shown) {
      return;
    }
    [g_find_bar_view.layer setNeedsDisplay];
    [g_find_bar_view.layer displayIfNeeded];
    PlaceFindBar();
    [g_window addChildWindow:g_find_bar ordered:NSWindowAbove];
    if (g_find_bar_keyboard) {
      // From the command bar too, which closes.
      [g_find_bar makeKeyAndOrderFront:nil];
    } else if (g_overlay_shown) {
      [g_overlay orderWindow:NSWindowAbove relativeTo:g_find_bar.windowNumber];
    }
    fprintf(stderr, "browser: find bar shown\n");
  });
}

void host_hide_find_bar(void) {
  if (!g_find_bar_shown) {
    return;
  }
  g_find_bar_shown = false;
  g_find_bar_keyboard = false;
  const bool had_keyboard = g_find_bar.keyWindow;
  [g_window removeChildWindow:g_find_bar];
  [g_find_bar orderOut:nil];
  if (had_keyboard) {
    [g_window makeKeyWindow];
  }
  fprintf(stderr, "browser: find bar hidden\n");
}

void host_show_window_buttons(bool shown) {
  for (NSWindowButton button :
       {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    [g_window standardWindowButton:button].hidden = !shown;
  }
}

void host_detent(void) {
  [NSHapticFeedbackManager.defaultPerformer
      performFeedbackPattern:NSHapticFeedbackPatternAlignment
             performanceTime:NSHapticFeedbackPerformanceTimeNow];
}

void host_press_window_button(int button) {
  // After the UI's update that asked, since closing or going full screen calls back into it.
  dispatch_async(dispatch_get_main_queue(), ^{
    switch (button) {
      case 0:
        [g_window performClose:nil];
        break;
      case 1:
        [g_window miniaturize:nil];
        break;
      default:
        // What a click on the green button does.
        [g_window toggleFullScreen:nil];
    }
  });
}

void host_add_reveal(void* reveal_view, void (*pointer_moved)(double x)) {
  g_reveal_view = (__bridge NSView*)reveal_view;
  g_reveal = g_reveal_view.window;
  g_reveal_pointer = pointer_moved;
  MakeOverlay(g_reveal);
  // A click in the panel leaves the keyboard where it was, in the page or the browser window.
  ((NSPanel*)g_reveal).becomesKeyOnlyIfNeeded = YES;
  g_reveal.hasShadow = NO;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSWindowDidResizeNotification
                      object:g_window
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (g_reveal_monitor) {
                      PlaceReveal();
                    }
                  }];
  // The pointer can't be in the panel's hold zone in an app that isn't the active one.
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (g_reveal_monitor) {
                      g_reveal_pointer(INFINITY);
                    }
                  }];
}

void host_show_reveal(double width) {
  // After the UI's update that asked: the window draws as it goes on screen and as it resizes.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_reveal || !g_window) {
      return;
    }
    g_reveal_width = width;
    if (g_reveal_monitor) {
      PlaceReveal();
      return;
    }
    // As the command bar does, it goes on screen once it has drawn what it shows now.
    PlaceReveal();
    [g_reveal_view.layer setNeedsDisplay];
    [g_reveal_view.layer displayIfNeeded];
    [g_window addChildWindow:g_reveal ordered:NSWindowAbove];
    // The page's view takes the pointer's moves while it's over the page, so the panel's view
    // doesn't hear them leave.
    g_reveal_monitor = [NSEvent
        addLocalMonitorForEventsMatchingMask:NSEventMaskMouseMoved | NSEventMaskLeftMouseDragged
                                     handler:^NSEvent*(NSEvent* event) {
                                       NSPoint at = event.locationInWindow;
                                       if (event.window) {
                                         at = [event.window convertPointToScreen:at];
                                       }
                                       g_reveal_pointer([g_window convertPointFromScreen:at].x);
                                       return event;
                                     }];
    fprintf(stderr, "browser: reveal window shown\n");
  });
}

void host_hide_reveal(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_reveal_monitor) {
      return;
    }
    [NSEvent removeMonitor:g_reveal_monitor];
    g_reveal_monitor = nil;
    [g_window removeChildWindow:g_reveal];
    [g_reveal orderOut:nil];
  });
}

void host_focus(void* view) {
  NSView* ns_view = (__bridge NSView*)view;
  [ns_view.window makeFirstResponder:ns_view];
}

int host_menu(const host_menu_item* items, size_t count, double x, double y) {
  HostMenuTarget* target = [[HostMenuTarget alloc] init];
  target.chosen = -1;
  NSMenu* menu = [[NSMenu alloc] init];
  menu.autoenablesItems = NO;
  menu.delegate = target;
  for (size_t i = 0; i < count; ++i) {
    NSMenuItem* item = [menu addItemWithTitle:@(items[i].title)
                                       action:@selector(choose:)
                                keyEquivalent:@""];
    item.target = target;
    item.tag = static_cast<NSInteger>(i);
    item.enabled = items[i].enabled;
  }
  NSView* view = g_window.contentView;
  NSPoint at = NSMakePoint(x, view.flipped ? y : NSHeight(view.bounds) - y);
  g_menu = menu;
  [menu popUpMenuPositioningItem:nil atLocation:at inView:view];
  g_menu = nil;
  return static_cast<int>(target.chosen);
}

bool host_confirm(const char* message, const char* detail, const char* action) {
  NSAlert* alert = [[NSAlert alloc] init];
  alert.messageText = @(message);
  alert.informativeText = @(detail);
  [alert addButtonWithTitle:@(action)].hasDestructiveAction = YES;
  [alert addButtonWithTitle:@"Cancel"];
  g_alert = alert;
  fprintf(stderr, "browser: alert shown %s\n", message);
  NSModalResponse response = [alert runModal];
  g_alert = nil;
  return response == NSAlertFirstButtonReturn;
}

bool host_choose(const char* title) {
  NSString* chosen = @(title);
  for (NSMenuItem* item in g_menu.itemArray) {
    if ([item.title isEqual:chosen] && item.enabled) {
      [g_menu performActionForItemAtIndex:[g_menu indexOfItem:item]];
      [g_menu cancelTracking];
      return true;
    }
  }
  for (NSButton* button in g_alert.buttons) {
    if ([button.title isEqual:chosen]) {
      [button performClick:nil];
      return true;
    }
  }
  return false;
}
