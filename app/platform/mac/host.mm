#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <objc/runtime.h>

#include <algorithm>
#include <initializer_list>
#include <string>
#include <vector>

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
  auto add = ^(NSMenuItem* item, NSString* title) {
    if (item.state == NSControlStateValueOn) {
      title = [title stringByAppendingString:@" (checked)"];
    }
    [titles addObject:item.enabled ? title : [title stringByAppendingString:@" (disabled)"]];
  };
  for (NSMenuItem* item in menu.itemArray) {
    if (item.separatorItem) {
      continue;
    }
    if (!item.submenu) {
      add(item, item.title);
    }
    // A submenu's items as "Move to Space › Work".
    for (NSMenuItem* inner in item.submenu.itemArray) {
      if (!inner.separatorItem) {
        add(inner, [NSString stringWithFormat:@"%@ › %@", item.title, inner.title]);
      }
    }
  }
  fprintf(stderr, "browser: menu shown %s\n",
          [titles componentsJoinedByString:@", "].UTF8String);
}
@end

namespace {
// The UI's, which hears the drags over its view.
bool (*g_dragged)(int, double, double, const host_dragged*, size_t);
}  // namespace

// The view the UI draws in. It takes the links and files dragged onto it, from the page or another
// app, for the UI.
@interface HostUIView : NSView
@end

@implementation HostUIView {
  // What the drag over the view carries, read as it comes.
  std::vector<std::string> _urls;
  std::vector<bool> _files;
}

- (instancetype)initWithFrame:(NSRect)frame {
  if ((self = [super initWithFrame:frame])) {
    [self registerForDraggedTypes:@[
      NSPasteboardTypeFileURL, NSPasteboardTypeURL, NSPasteboardTypeString
    ]];
  }
  return self;
}

// Tells the UI, with where the pointer is from the view's top-left corner.
- (BOOL)tell:(int)phase info:(id<NSDraggingInfo>)info {
  if (!g_dragged) {
    return NO;
  }
  NSPoint point = [self convertPoint:info.draggingLocation fromView:nil];
  std::vector<host_dragged> items;
  for (size_t i = 0; i < _urls.size(); i++) {
    items.push_back({_urls[i].c_str(), _files[i]});
  }
  return g_dragged(phase, point.x, NSHeight(self.bounds) - point.y, items.data(), items.size());
}

- (NSDragOperation)draggingEntered:(id<NSDraggingInfo>)info {
  _urls.clear();
  _files.clear();
  NSPasteboard* pasteboard = info.draggingPasteboard;
  NSArray<NSURL*>* files =
      [pasteboard readObjectsForClasses:@[ NSURL.class ]
                                options:@{NSPasteboardURLReadingFileURLsOnlyKey : @YES}];
  for (NSURL* url in files) {
    _urls.push_back(url.absoluteString.UTF8String);
    _files.push_back(true);
  }
  if (_urls.empty()) {
    for (NSURL* url in [pasteboard readObjectsForClasses:@[ NSURL.class ] options:@{}]) {
      _urls.push_back(url.absoluteString.UTF8String);
      _files.push_back(false);
    }
  }
  // Text, which the UI opens only if it's a web link.
  NSString* text = [pasteboard stringForType:NSPasteboardTypeString];
  if (_urls.empty() && text) {
    NSCharacterSet* space = NSCharacterSet.whitespaceAndNewlineCharacterSet;
    _urls.push_back([text stringByTrimmingCharactersInSet:space].UTF8String);
    _files.push_back(false);
  }
  return [self draggingUpdated:info];
}

- (NSDragOperation)draggingUpdated:(id<NSDraggingInfo>)info {
  return [self tell:HOST_DRAG_OVER info:info] ? NSDragOperationCopy : NSDragOperationNone;
}

- (void)draggingExited:(id<NSDraggingInfo>)info {
  [self tell:HOST_DRAG_LEFT info:info];
}

// After a drop too, which the UI keeps.
- (void)draggingEnded:(id<NSDraggingInfo>)info {
  [self tell:HOST_DRAG_LEFT info:info];
  _urls.clear();
  _files.clear();
}

- (BOOL)performDragOperation:(id<NSDraggingInfo>)info {
  return [self tell:HOST_DRAG_DROP info:info];
}

// Only as the pointer moves, so a drag held still doesn't wake the UI.
- (BOOL)wantsPeriodicDraggingUpdates {
  return NO;
}
@end

namespace {

// A round swatch of |color|, 0xRRGGBB in sRGB, for a menu item.
NSImage* Swatch(int32_t color) {
  NSColor* fill = [NSColor colorWithSRGBRed:((color >> 16) & 0xff) / 255.0
                                      green:((color >> 8) & 0xff) / 255.0
                                       blue:(color & 0xff) / 255.0
                                      alpha:1];
  return [NSImage imageWithSize:NSMakeSize(12, 12)
                        flipped:NO
                 drawingHandler:^BOOL(NSRect rect) {
                   [fill setFill];
                   [[NSBezierPath bezierPathWithOvalInRect:rect] fill];
                   return YES;
                 }];
}

// The browser window, once shown, and the view the UI draws in.
NSWindow* g_window;
NSView* g_ui_view;

// The command bar's window and view.
NSWindow* g_overlay;
NSView* g_overlay_view;
bool g_overlay_shown = false;
void (*g_overlay_dismissed)(void);

// The dialog's window and view.
NSWindow* g_dialog;
NSView* g_dialog_view;
bool g_dialog_shown = false;

// The toast's window and view.
NSWindow* g_toast;
NSView* g_toast_view;
bool g_toast_shown = false;

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

// The zoom HUD's window and view, its distance from the top right of the browser window's content,
// and a count of its shows and hides, so one posted before another doesn't undo it.
NSWindow* g_hud;
NSView* g_hud_view;
bool g_hud_shown = false;
double g_hud_right;
double g_hud_top;
unsigned g_hud_changes = 0;

// The drop targets' window and view.
NSWindow* g_drop_targets;
NSView* g_drop_targets_view;
bool g_drop_targets_shown = false;

// How far the focused page's top right corner is inside the card's, in a split view, which the
// find bar and the HUD keep to.
double g_page_right = 0;
double g_page_top = 0;

// The sidebar reveal's window and view, the shell's handler for the pointer, the window's width,
// and while it shows, the monitor that hands the handler the pointer.
NSWindow* g_reveal;
NSView* g_reveal_view;
void (*g_reveal_pointer)(double x);
double g_reveal_width;
id g_reveal_monitor;

// The Settings window and its view, whether it's to show, and whether it has, after which it keeps
// the place the user left it in.
NSWindow* g_settings;
NSView* g_settings_view;
bool g_settings_shown = false;
bool g_settings_placed = false;

// While a host_watch_control is on, its monitor, its observer and the handler to call.
id g_control_monitor;
id g_control_observer;
void (*g_control_released)(void);

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

// AppKit keeps a titled window's top below the menu bar, which full screen on a screen without a
// notch puts over the browser window's top. An overlay goes where the host puts it, over that.
NSRect Unconstrained(id, SEL, NSRect frame, NSScreen*) {
  return frame;
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
  // GPUI's class for its panels, which are all overlays here.
  SEL constrain = @selector(constrainFrameRect:toScreen:);
  class_addMethod(object_getClass(window), constrain, (IMP)Unconstrained,
                  method_getTypeEncoding(class_getInstanceMethod(NSWindow.class, constrain)));
}

void PlaceFindBar() {
  NSRect content = [g_window contentRectForFrameRect:g_window.frame];
  NSSize size = g_find_bar.frame.size;
  [g_find_bar setFrameOrigin:NSMakePoint(NSMaxX(content) - g_page_right - g_find_bar_right -
                                             size.width,
                                         NSMaxY(content) - g_page_top - g_find_bar_top -
                                             size.height)];
}

// At the browser window's top right corner, where the toast draws itself.
void PlaceToast() {
  NSRect frame = g_window.frame;
  NSSize size = g_toast.frame.size;
  [g_toast setFrameOrigin:NSMakePoint(NSMaxX(frame) - size.width, NSMaxY(frame) - size.height)];
}

// 8 points below the find bar while it shows, where the HUD would cover it.
void PlaceHud() {
  NSRect content = [g_window contentRectForFrameRect:g_window.frame];
  NSSize size = g_hud.frame.size;
  const double top =
      g_find_bar_shown ? g_find_bar_top + g_find_bar.frame.size.height + 8 : g_hud_top;
  [g_hud setFrameOrigin:NSMakePoint(NSMaxX(content) - g_page_right - g_hud_right - size.width,
                                    NSMaxY(content) - g_page_top - top - size.height)];
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
  return (__bridge_retained void*)[[HostUIView alloc] initWithFrame:NSZeroRect];
}

void host_show_window(void* ui_view) {
  g_ui_view = (__bridge NSView*)ui_view;
  g_window = g_ui_view.window;
  // Dia's window material (docs/ux-spec.md › Window material): what's behind the window, blurred,
  // under the UI, which tints it. AppKit swaps it for an opaque fill while the window is inactive,
  // and with Reduce Transparency or Increase Contrast.
  NSVisualEffectView* material = [[NSVisualEffectView alloc] initWithFrame:g_ui_view.bounds];
  material.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  material.blendingMode = NSVisualEffectBlendingModeBehindWindow;
  // Dia's dark material, which AppKit doesn't name.
  material.material = (NSVisualEffectMaterial)29;
  material.state = NSVisualEffectStateFollowsWindowActiveState;
  material.emphasized = YES;
  material.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
  // Under the UI's view, since the core keeps only the UI's and the page's views in its own.
  [g_ui_view addSubview:material positioned:NSWindowBelow relativeTo:nil];
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
                                          if (event.window != g_overlay) {
                                            host_dismiss_overlay();
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
                                                if (g_hud_shown) {
                                                  PlaceHud();
                                                }
                                                if (g_dialog_shown) {
                                                  [g_dialog setFrame:g_window.frame display:YES];
                                                }
                                                if (g_toast_shown) {
                                                  PlaceToast();
                                                }
                                              }];
}

void host_add_overlay(void* overlay_view, void (*dismissed)(void)) {
  g_overlay_view = (__bridge NSView*)overlay_view;
  g_overlay_dismissed = dismissed;
  g_overlay = g_overlay_view.window;
  MakeOverlay(g_overlay);
  // The UI draws the bar's shape, which grows out of the URL field. A titled window would draw its
  // edge and shadow at the window's full size meanwhile; a borderless one's shadow takes the shape
  // drawn.
  g_overlay.styleMask = NSWindowStyleMaskBorderless;
  g_overlay.hasShadow = YES;
  // The overlay closes when the user leaves it: a click outside it, another window taking the
  // keyboard, or switching apps.
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    host_dismiss_overlay();
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
                        host_dismiss_overlay();
                      }
                    } else if (!g_overlay_shown && !g_dialog_shown && window == g_window &&
                               g_find_bar_shown && g_find_bar_keyboard) {
                      // The same for the find bar, which the user left only by clicking.
                      [g_find_bar makeKeyWindow];
                    }
                  }];
}

void host_show_overlay(bool centered, double left, double top, double width) {
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
    NSRect content = [g_window contentRectForFrameRect:g_window.frame];
    const double height = NSHeight(g_overlay.frame);
    NSRect frame;
    if (centered) {
      // A fifth of the way down the browser window. Arc's placement isn't measured yet.
      frame = NSMakeRect(NSMidX(content) - width / 2,
                         NSMaxY(content) - NSHeight(content) / 5 - height, width, height);
    } else {
      frame = NSMakeRect(NSMinX(content) + left, NSMaxY(content) - top - height,
                         fmin(width, NSWidth(content) - left - 8), height);
    }
    // At its new size before it draws, so it draws once.
    [g_overlay setFrame:frame display:NO];
    [g_overlay_view.layer setNeedsDisplay];
    [g_overlay_view.layer displayIfNeeded];
    [g_window addChildWindow:g_overlay ordered:NSWindowAbove];
    [g_overlay makeKeyAndOrderFront:nil];
    // check.sh reads these lines.
    fprintf(stderr, "browser: command bar at %.0f %.0f %.0f %.0f\n",
            NSMinX(frame) - NSMinX(content), NSMaxY(content) - NSMaxY(frame), NSWidth(frame),
            NSHeight(frame));
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
    [g_overlay invalidateShadow];
    fprintf(stderr, "browser: command bar height %.0f\n", height);
  });
}

void host_reshape_overlay(void) {
  // After the frame the UI is drawing, whose shape the shadow takes.
  dispatch_async(dispatch_get_main_queue(), ^{
    [g_overlay invalidateShadow];
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

void host_dismiss_overlay(void) {
  if (!g_overlay_shown) {
    return;
  }
  host_hide_overlay();
  // Posted: the UI may be handling the event that closed the overlay.
  if (g_overlay_dismissed) {
    dispatch_async(dispatch_get_main_queue(), ^{
      g_overlay_dismissed();
    });
  }
}

void host_add_find_bar(void* find_bar_view, double right, double top) {
  g_find_bar_view = (__bridge NSView*)find_bar_view;
  g_find_bar = g_find_bar_view.window;
  g_find_bar_right = right;
  g_find_bar_top = top;
  MakeOverlay(g_find_bar);
  // The UI draws the bar's rounded shape, which a borderless window's shadow takes.
  g_find_bar.styleMask = NSWindowStyleMaskBorderless;
  g_find_bar.hasShadow = YES;
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
    [g_find_bar invalidateShadow];
    PlaceFindBar();
    [g_window addChildWindow:g_find_bar ordered:NSWindowAbove];
    if (g_find_bar_keyboard) {
      // From the command bar too, which closes.
      [g_find_bar makeKeyAndOrderFront:nil];
    } else if (g_overlay_shown) {
      [g_overlay orderWindow:NSWindowAbove relativeTo:g_find_bar.windowNumber];
    }
    if (g_hud_shown) {
      PlaceHud();
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
  if (g_hud_shown) {
    PlaceHud();
  }
  fprintf(stderr, "browser: find bar hidden\n");
}

void host_set_pane_corner(double right, double top) {
  g_page_right = right;
  g_page_top = top;
  if (g_find_bar_shown) {
    PlaceFindBar();
  }
  if (g_hud_shown) {
    PlaceHud();
  }
}

void host_add_hud(void* hud_view, double right, double top) {
  g_hud_view = (__bridge NSView*)hud_view;
  g_hud = g_hud_view.window;
  g_hud_right = right;
  g_hud_top = top;
  MakeOverlay(g_hud);
  g_hud.styleMask = NSWindowStyleMaskBorderless;
  g_hud.hasShadow = YES;
  g_hud.ignoresMouseEvents = YES;
}

void host_show_hud(void) {
  if (!g_hud || !g_window) {
    return;
  }
  g_hud_shown = true;
  const unsigned change = ++g_hud_changes;
  // As the find bar does, once it has drawn what it shows now, which may be narrower than before.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (change != g_hud_changes) {
      return;
    }
    [g_hud_view.layer setNeedsDisplay];
    [g_hud_view.layer displayIfNeeded];
    [g_hud invalidateShadow];
    PlaceHud();
    // Stops a fade.
    [NSAnimationContext
        runAnimationGroup:^(NSAnimationContext* context) {
          context.duration = 0;
          g_hud.animator.alphaValue = 1;
        }
        completionHandler:nil];
    if (!g_hud.parentWindow) {
      [g_window addChildWindow:g_hud ordered:NSWindowAbove];
    }
    if (g_find_bar_shown) {
      [g_hud orderWindow:NSWindowAbove relativeTo:g_find_bar.windowNumber];
    }
    if (g_overlay_shown) {
      [g_overlay orderWindow:NSWindowAbove relativeTo:g_hud.windowNumber];
    }
    fprintf(stderr, "browser: zoom hud shown\n");
  });
}

void host_hide_hud(double seconds) {
  if (!g_hud_shown) {
    return;
  }
  g_hud_shown = false;
  const unsigned change = ++g_hud_changes;
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = seconds;
        g_hud.animator.alphaValue = 0;
      }
      completionHandler:^{
        if (change != g_hud_changes) {
          return;
        }
        [g_window removeChildWindow:g_hud];
        [g_hud orderOut:nil];
        fprintf(stderr, "browser: zoom hud hidden\n");
      }];
}

void host_add_drop_targets(void* view) {
  g_drop_targets_view = (__bridge NSView*)view;
  g_drop_targets = g_drop_targets_view.window;
  MakeOverlay(g_drop_targets);
  g_drop_targets.styleMask = NSWindowStyleMaskBorderless;
  g_drop_targets.hasShadow = NO;
  g_drop_targets.ignoresMouseEvents = YES;
}

void host_show_drop_targets(void) {
  if (!g_drop_targets || !g_window || g_drop_targets_shown) {
    return;
  }
  g_drop_targets_shown = true;
  // After the UI's update that asked, since the window draws here: at the content's frame, before
  // it shows, since it keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_drop_targets_shown || g_drop_targets.parentWindow) {
      return;
    }
    [g_drop_targets setFrame:[g_window contentRectForFrameRect:g_window.frame] display:NO];
    [g_drop_targets_view.layer setNeedsDisplay];
    [g_drop_targets_view.layer displayIfNeeded];
    [g_window addChildWindow:g_drop_targets ordered:NSWindowAbove];
    fprintf(stderr, "browser: drop targets shown\n");
  });
}

void host_hide_drop_targets(void) {
  g_drop_targets_shown = false;
  if (!g_drop_targets.parentWindow) {
    return;
  }
  [g_window removeChildWindow:g_drop_targets];
  [g_drop_targets orderOut:nil];
  fprintf(stderr, "browser: drop targets hidden\n");
}

void host_add_dialog(void* dialog_view) {
  g_dialog_view = (__bridge NSView*)dialog_view;
  g_dialog = g_dialog_view.window;
  MakeOverlay(g_dialog);
  // Titled, as GPUI made it, so its corners are the browser window's. It draws the dim over all
  // of the browser window, which has its own shadow.
  g_dialog.hasShadow = NO;
  // The dialog is answered before anything else, as an alert is. Chromium makes its window key by
  // itself, such as when a page commits, and the dialog takes the keyboard back.
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidBecomeKeyNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification* notification) {
                                                if (g_dialog_shown &&
                                                    notification.object != g_dialog) {
                                                  [g_dialog makeKeyWindow];
                                                }
                                              }];
}

void host_show_dialog(void) {
  if (!g_dialog || !g_window || g_dialog_shown) {
    return;
  }
  g_dialog_shown = true;
  // After the UI's update that asked, since the window draws here: at the browser window's frame,
  // before it shows, since it keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_dialog_shown) {
      return;
    }
    [g_dialog setFrame:g_window.frame display:NO];
    [g_dialog_view.layer setNeedsDisplay];
    [g_dialog_view.layer displayIfNeeded];
    [g_window addChildWindow:g_dialog ordered:NSWindowAbove];
    [g_dialog makeKeyAndOrderFront:nil];
    fprintf(stderr, "browser: dialog window shown\n");
  });
}

void host_hide_dialog(void) {
  if (!g_dialog_shown) {
    return;
  }
  g_dialog_shown = false;
  [g_window removeChildWindow:g_dialog];
  [g_dialog orderOut:nil];
  [g_find_bar_shown && g_find_bar_keyboard ? g_find_bar : g_window makeKeyWindow];
}

void host_add_toast(void* toast_view) {
  g_toast_view = (__bridge NSView*)toast_view;
  g_toast = g_toast_view.window;
  MakeOverlay(g_toast);
  g_toast.styleMask = NSWindowStyleMaskBorderless;
  // The toast draws its own, which scales with it.
  g_toast.hasShadow = NO;
  g_toast.ignoresMouseEvents = YES;
}

void host_show_toast(void) {
  if (!g_toast || !g_window || g_toast_shown) {
    return;
  }
  g_toast_shown = true;
  // After the UI's update that asked, since the window draws here, before it shows, since it
  // keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_toast_shown) {
      return;
    }
    PlaceToast();
    [g_toast_view.layer setNeedsDisplay];
    [g_toast_view.layer displayIfNeeded];
    [g_window addChildWindow:g_toast ordered:NSWindowAbove];
    if (g_overlay_shown) {
      [g_overlay orderWindow:NSWindowAbove relativeTo:g_toast.windowNumber];
    }
    fprintf(stderr, "browser: toast window shown\n");
  });
}

void host_hide_toast(void) {
  if (!g_toast_shown) {
    return;
  }
  g_toast_shown = false;
  [g_window removeChildWindow:g_toast];
  [g_toast orderOut:nil];
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

uint32_t host_accent_color(void) {
  NSColor* color = [NSColor.controlAccentColor colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
  auto channel = [](CGFloat value) {
    return (uint32_t)lround(std::clamp(value, (CGFloat)0, (CGFloat)1) * 255);
  };
  return channel(color.redComponent) << 16 | channel(color.greenComponent) << 8 |
         channel(color.blueComponent);
}

bool host_reduce_motion(void) {
  return NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceMotion;
}

void host_watch_accent_color(void (*changed)(void)) {
  [NSNotificationCenter.defaultCenter addObserverForName:NSSystemColorsDidChangeNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                changed();
                                              }];
}

void ControlReleased() {
  if (!g_control_monitor) {
    return;
  }
  [NSEvent removeMonitor:g_control_monitor];
  g_control_monitor = nil;
  [NSNotificationCenter.defaultCenter removeObserver:g_control_observer];
  g_control_observer = nil;
  void (*released)(void) = g_control_released;
  // After the event that showed it, so that a click or key goes where it was going first.
  dispatch_async(dispatch_get_main_queue(), ^{
    released();
  });
}

void host_take_drops(bool (*dragged)(int phase, double x, double y, const host_dragged* items,
                                     size_t count)) {
  g_dragged = dragged;
}

void host_watch_control(void (*released)(void)) {
  g_control_released = released;
  if (g_control_monitor) {
    return;
  }
  NSEventMask mask = NSEventMaskFlagsChanged | NSEventMaskKeyDown | NSEventMaskLeftMouseDown |
                     NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  g_control_monitor = [NSEvent
      addLocalMonitorForEventsMatchingMask:mask
                                   handler:^NSEvent*(NSEvent* event) {
                                     bool key = event.type == NSEventTypeFlagsChanged ||
                                                event.type == NSEventTypeKeyDown;
                                     if (!key ||
                                         !(event.modifierFlags & NSEventModifierFlagControl)) {
                                       ControlReleased();
                                     }
                                     return event;
                                   }];
  // ⌃ can go up in another app, where the monitor doesn't see it.
  g_control_observer = [NSNotificationCenter.defaultCenter
      addObserverForName:NSApplicationDidResignActiveNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification*) {
                ControlReleased();
              }];
}

void host_copy(const char* text) {
  static NSPasteboard* driven =
      getenv("BROWSER_DRIVE") ? [NSPasteboard pasteboardWithUniqueName] : nil;
  NSPasteboard* pasteboard = driven ?: NSPasteboard.generalPasteboard;
  [pasteboard clearContents];
  [pasteboard setString:@(text) forType:NSPasteboardTypeString];
}

void host_day_starts(double starts[4]) {
  NSCalendar* calendar = NSCalendar.currentCalendar;
  NSDate* now = NSDate.date;
  NSDate* today = [calendar startOfDayForDate:now];
  NSDate* week = today;
  [calendar rangeOfUnit:NSCalendarUnitWeekOfYear startDate:&week interval:nil forDate:now];
  NSDate* dates[] = {
      today,
      [calendar dateByAddingUnit:NSCalendarUnitDay value:-1 toDate:today options:0],
      week,
      [calendar dateByAddingUnit:NSCalendarUnitWeekOfYear value:-1 toDate:week options:0],
  };
  for (int i = 0; i < 4; i++) {
    starts[i] = dates[i].timeIntervalSince1970 * 1000;
  }
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

namespace {

// Shows the menu at |x|, |y| from the top-left of |view|, after the UI in |drawn| draws what the
// menu changed. See host_menu.
int ShowMenu(const host_menu_item* items,
             size_t count,
             double x,
             double y,
             NSView* view,
             NSArray<NSView*>* drawn,
             int over) {
  HostMenuTarget* target = [[HostMenuTarget alloc] init];
  target.chosen = -1;
  NSMenu* menu = [[NSMenu alloc] init];
  menu.autoenablesItems = NO;
  menu.delegate = target;
  // By index, for the items in their submenus.
  NSMutableArray<NSMenuItem*>* made = [NSMutableArray array];
  for (size_t i = 0; i < count; ++i) {
    NSMenu* in = menu;
    if (items[i].parent >= 0 && static_cast<NSUInteger>(items[i].parent) < made.count) {
      NSMenuItem* parent = made[items[i].parent];
      if (!parent.submenu) {
        parent.submenu = [[NSMenu alloc] init];
        parent.submenu.autoenablesItems = NO;
      }
      in = parent.submenu;
    }
    if (!items[i].title) {
      NSMenuItem* line = [NSMenuItem separatorItem];
      [in addItem:line];
      [made addObject:line];
      continue;
    }
    NSMenuItem* item = [in addItemWithTitle:@(items[i].title)
                                     action:@selector(choose:)
                              keyEquivalent:@(items[i].key)];
    [made addObject:item];
    item.keyEquivalentModifierMask = (items[i].command ? NSEventModifierFlagCommand : 0) |
                                     (items[i].shift ? NSEventModifierFlagShift : 0) |
                                     (items[i].option ? NSEventModifierFlagOption : 0) |
                                     (items[i].control ? NSEventModifierFlagControl : 0);
    item.target = target;
    item.tag = static_cast<NSInteger>(i);
    item.enabled = items[i].enabled;
    item.state = items[i].checked ? NSControlStateValueOn : NSControlStateValueOff;
    if (items[i].swatch >= 0) {
      item.image = Swatch(items[i].swatch);
    }
  }
  // The UI draws what the menu changed, such as its row's fill, now: it can't once the menu's
  // event loop runs.
  for (NSView* ui in drawn) {
    [ui.layer setNeedsDisplay];
    [ui.layer displayIfNeeded];
  }
  NSPoint at = NSMakePoint(x, view.flipped ? y : NSHeight(view.bounds) - y);
  NSMenuItem* positioned =
      over >= 0 && static_cast<NSUInteger>(over) < made.count ? made[over] : nil;
  if (positioned) {
    // AppKit puts the item's edge at the point, and its title this far in, past the checkmarks'
    // column (measured, as AppKit doesn't say).
    constexpr double kMenuTitleInset = 31.5;
    at.x -= kMenuTitleInset;
  }
  g_menu = menu;
  [menu popUpMenuPositioningItem:positioned atLocation:at inView:view];
  g_menu = nil;
  return static_cast<int>(target.chosen);
}

}  // namespace

int host_menu(const host_menu_item* items, size_t count, double x, double y, int in, int over) {
  NSView* panel = in == HOST_IN_REVEAL     ? g_reveal_view
                  : in == HOST_IN_SETTINGS ? g_settings_view
                                           : nil;
  return ShowMenu(items, count, x, y, panel ?: g_window.contentView,
                  panel ? @[ panel ] : g_ui_view.subviews, over);
}

int host_menu_in(const host_menu_item* items, size_t count, double x, double y, void* view) {
  NSView* in = (__bridge NSView*)view;
  return ShowMenu(items, count, x, y, in, @[ in ], -1);
}

void host_add_settings(void* settings_view) {
  g_settings_view = (__bridge NSView*)settings_view;
  g_settings = g_settings_view.window;
  // Its controls and menus are drawn dark, as the browser's are.
  g_settings.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
  g_settings.tabbingMode = NSWindowTabbingModeDisallowed;
  g_settings.collectionBehavior |=
      NSWindowCollectionBehaviorFullScreenAuxiliary | NSWindowCollectionBehaviorMoveToActiveSpace;
}

void host_show_settings(void) {
  if (!g_settings) {
    return;
  }
  if (g_settings_shown) {
    [g_settings makeKeyAndOrderFront:nil];
    fprintf(stderr, "browser: settings shown %s\n", g_settings.title.UTF8String);
    return;
  }
  g_settings_shown = true;
  // On screen once the UI has drawn it as it is now, which it can't while it handles the shortcut
  // that opened it, as the command bar does.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_settings_shown) {
      return;
    }
    if (!g_settings_placed) {
      g_settings_placed = true;
      NSRect visible = (g_window.screen ?: WindowScreen()).visibleFrame;
      NSSize size = g_settings.frame.size;
      double x = NSMidX(g_window.frame) - size.width / 2;
      double y = NSMidY(g_window.frame) - size.height / 2;
      x = fmax(NSMinX(visible), fmin(x, NSMaxX(visible) - size.width));
      y = fmax(NSMinY(visible), fmin(y, NSMaxY(visible) - size.height));
      [g_settings setFrameOrigin:NSMakePoint(x, y)];
    }
    [g_settings_view.layer setNeedsDisplay];
    [g_settings_view.layer displayIfNeeded];
    [g_settings makeKeyAndOrderFront:nil];
    // check.sh waits for this line. The window's title is its tab's name.
    fprintf(stderr, "browser: settings shown %s\n", g_settings.title.UTF8String);
  });
}

void host_hide_settings(void) {
  g_settings_shown = false;
  [g_settings orderOut:nil];
}

bool host_close_other_window(void) {
  NSWindow* key = NSApp.keyWindow;
  // The browser window, the windows over it and Settings take ⌘W themselves.
  if (!key || key == g_window || key == g_settings || key.parentWindow ||
      !(key.styleMask & NSWindowStyleMaskClosable)) {
    return false;
  }
  [key performClose:nil];
  return true;
}

int host_ask(const char* message, const char* detail, const char* const* buttons, size_t count) {
  NSAlert* alert = [[NSAlert alloc] init];
  alert.messageText = @(message);
  alert.informativeText = @(detail);
  for (size_t i = 0; i < count; ++i) {
    [alert addButtonWithTitle:@(buttons[i])];
  }
  g_alert = alert;
  fprintf(stderr, "browser: alert shown %s\n", message);
  NSModalResponse response = [alert runModal];
  g_alert = nil;
  return static_cast<int>(response - NSAlertFirstButtonReturn);
}

bool host_choose(const char* title) {
  NSString* chosen = @(title);
  for (NSMenuItem* item in g_menu.itemArray) {
    // An item in a submenu, else one in the menu.
    for (NSMenuItem* each in item.submenu ? item.submenu.itemArray : @[ item ]) {
      if ([each.title isEqual:chosen] && each.enabled && !each.submenu) {
        [each.menu performActionForItemAtIndex:[each.menu indexOfItem:each]];
        [g_menu cancelTracking];
        return true;
      }
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
