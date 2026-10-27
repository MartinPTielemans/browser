#import <objc/runtime.h>

#include <initializer_list>

#include "host_internal.h"

// Arc's Site Controls' shadow, fitted over a dark page: darker below, as it falls 4 pt. It's an
// overlay for its blur and shadow alone, outside g_overlays, so the tooltip shows over it.
Overlay g_site_controls_overlay("site controls", 0.24, 16, 4);

// The dialog's window and view, and the browser it shows over.
Browser* g_dialog_over;
NSWindow* g_dialog;
NSView* g_dialog_view;
bool g_dialog_shown = false;

// The Site Controls' window and view, the browser they show over, and where their top left goes
// there. Released, they're on screen only for the motion of their leaving, and take no clicks.
Browser* g_site_controls_over;
NSWindow* g_site_controls;
NSView* g_site_controls_view;
bool g_site_controls_shown = false;
bool g_site_controls_released = false;
double g_site_controls_left;
double g_site_controls_top;
void (*g_site_controls_dismissed)(void);

// Capture's window and view, the browser it shows over, where it goes there, the camera pointer,
// and while it shows, the monitor that keeps the pointer the camera.
Browser* g_capture_over;
NSWindow* g_capture;
NSView* g_capture_view;
bool g_capture_shown = false;
NSRect g_capture_frame;
void (*g_capture_dismissed)(void);
NSCursor* g_capture_cursor;
bool g_capture_camera = true;
id g_capture_monitor;

// The toast's window and view, and the browser it shows over.
Browser* g_toast_over;
NSWindow* g_toast;
NSView* g_toast_view;
bool g_toast_shown = false;

// The tooltip's window and view, the browser it shows over, where its top left goes there, and a
// count of its shows and hides, so a fade's end doesn't hide one shown since.
Browser* g_tooltip_over;
NSWindow* g_tooltip;
NSView* g_tooltip_view;
bool g_tooltip_shown = false;
double g_tooltip_left = 0;
double g_tooltip_top = 0;
unsigned g_tooltip_changes = 0;
void (*g_tooltip_away)(void);

// The link target's window and view, the browser it shows over, where its top left goes there, a
// count of its shows and hides as the tooltip's, the UI's handler for the pointer, and while it
// shows, the monitor that hands the handler the pointer.
Browser* g_link_over;
NSWindow* g_link;
NSView* g_link_view;
bool g_link_shown = false;
double g_link_left = 0;
double g_link_top = 0;
unsigned g_link_changes = 0;
void (*g_link_pointer)(double x, double y);
id g_link_monitor;

// The zoom HUD's window and view, the browser it shows over, its distance from the top right of
// that window's content, and a count of its shows and hides, so one posted before another doesn't
// undo it.
Browser* g_hud_over;
NSWindow* g_hud;
NSView* g_hud_view;
bool g_hud_shown = false;
double g_hud_right;
double g_hud_top;
double g_hud_margin;
unsigned g_hud_changes = 0;

// The drop targets' window and view, and the browser they show over.
Browser* g_drop_targets_over;
NSWindow* g_drop_targets;
NSView* g_drop_targets_view;
bool g_drop_targets_shown = false;

namespace {

// AppKit keeps a titled window's top below the menu bar, which full screen on a screen without a
// notch puts over the browser window's top. An overlay goes where the host puts it, over that.
NSRect Unconstrained(id, SEL, NSRect frame, NSScreen*) {
  return frame;
}

// Another app that asks which of ours is in front takes the first window at the normal level, in
// the window server's order: T3 Code's window capture does, and so do others. Our windows over a
// browser window are child windows, ahead of it in that order, so a tooltip or an overlay's shadow
// on screen at alpha 0 was taken for it. Over the main window of the active app, they're a level
// above, as AppKit's own help tags and menus are. Over any other window, and while another app is
// active, they're at its level, so they don't cover another window or app over it.
constexpr NSWindowLevel kOverLevel = NSNormalWindowLevel + 1;

// Ours of a window's child windows: GPUI's panels and the overlays' shadows, not the bubbles
// Chromium levels itself.
bool IsOurs(NSWindow* child) {
  static Class panel = NSClassFromString(@"GPUIPanel");
  return [child isKindOfClass:panel] || child.class == NSWindow.class;
}

// A window above the normal level defaults to transient, which Spaces and Mission Control treat
// apart from a normal window, so it's managed, as Chromium makes its own.
void SetOverLevel(NSWindow* child, bool raised) {
  const NSWindowLevel level = raised ? kOverLevel : NSNormalWindowLevel;
  if (child.level != level) {
    child.collectionBehavior |= NSWindowCollectionBehaviorManaged;
    child.level = level;
  }
}

// Puts ours of |parent|'s child windows at the level the rule above gives them, back to front, so
// they keep their order.
void LevelOver(NSWindow* parent, bool raised) {
  NSArray<NSWindow*>* children = [parent.childWindows
      sortedArrayUsingComparator:^NSComparisonResult(NSWindow* a, NSWindow* b) {
        return a.orderedIndex > b.orderedIndex   ? NSOrderedAscending
               : a.orderedIndex < b.orderedIndex ? NSOrderedDescending
                                                 : NSOrderedSame;
      }];
  for (NSWindow* child in children) {
    if (IsOurs(child)) {
      SetOverLevel(child, raised);
    }
  }
}

// Lucide's camera (https://github.com/lucide-icons/lucide, commit
// 7faf87628a1ad6175975bc595344337ce14a7db4), ISC License; see NOTICE. Drawn as Arc's capture
// pointer is (docs/ux-spec.md › Capture): a light line a point wide, over a dark fill, with a dark
// edge either side so it shows on any page.
constexpr const char* kCameraBody =
    "M13.997 4a2 2 0 0 1 1.76 1.05l.486.9A2 2 0 0 0 18.003 7H20a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H4"
    "a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h1.997a2 2 0 0 0 1.759-1.048l.489-.904A2 2 0 0 1 10.004 4z";

NSCursor* CameraCursor() {
  NSString* svg = [NSString
      stringWithFormat:
          @"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" "
          @"viewBox=\"0 0 24 24\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
          @"<path d=\"%s\" fill=\"rgb(30,41,48)\" stroke=\"rgba(0,0,0,0.6)\" stroke-width=\"3\"/>"
          @"<circle cx=\"12\" cy=\"13\" r=\"3\" fill=\"none\" stroke=\"rgba(0,0,0,0.6)\" "
          @"stroke-width=\"3.5\"/>"
          @"<path d=\"%s\" fill=\"none\" stroke=\"rgb(215,215,215)\" stroke-width=\"1\"/>"
          @"<circle cx=\"12\" cy=\"13\" r=\"3\" fill=\"none\" stroke=\"white\" "
          @"stroke-width=\"1.5\"/></svg>",
          kCameraBody, kCameraBody];
  NSImage* image = [[NSImage alloc] initWithData:[svg dataUsingEncoding:NSUTF8StringEncoding]];
  return image ? [[NSCursor alloc] initWithImage:image hotSpot:NSMakePoint(12, 12)]
               : NSCursor.crosshairCursor;
}

void SetCaptureCursor() {
  [(g_capture_camera ? g_capture_cursor : NSCursor.arrowCursor) set];
}

// Where the UI asked, which it has kept inside the browser window: its window is wider than the
// tooltip it holds, so clamping the window here would move the tooltip off the tile it names.
void PlaceTooltip() {
  NSWindow* window = g_tooltip_over->window;
  NSRect content = [window contentRectForFrameRect:window.frame];
  NSSize size = g_tooltip.frame.size;
  [g_tooltip setFrameOrigin:NSMakePoint(NSMinX(content) + g_tooltip_left,
                                        NSMaxY(content) - g_tooltip_top - size.height)];
}

// As the tooltip's: where the UI asked, inside the browser window, and its window wider than the
// plate it holds.
void PlaceLinkTarget() {
  NSWindow* window = g_link_over->window;
  NSRect content = [window contentRectForFrameRect:window.frame];
  NSSize size = g_link.frame.size;
  [g_link setFrameOrigin:NSMakePoint(NSMinX(content) + g_link_left,
                                     NSMaxY(content) - g_link_top - size.height)];
}

// At the browser window's left edge and as tall as its content, so a point is at the same place in
// both windows.
void PlaceReveal(Browser* browser) {
  NSRect content = [browser->window contentRectForFrameRect:browser->window.frame];
  [browser->reveal setFrame:NSMakeRect(NSMinX(content), NSMinY(content), browser->reveal_width,
                                       NSHeight(content))
                    display:NO];
}

}  // namespace

// Puts |child| over |parent|, at the level of |parent|'s others.
void AddOver(NSWindow* parent, NSWindow* child) {
  SetOverLevel(child, NSApp.active && parent.mainWindow);
  [parent addChildWindow:child ordered:NSWindowAbove];
}

// From the first browser window, the levels follow the main window and the active app.
void WatchMainWindow() {
  static bool watching = false;
  if (watching) {
    return;
  }
  watching = true;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSWindowDidBecomeMainNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    LevelOver(notification.object, NSApp.active);
                  }];
  [center addObserverForName:NSWindowDidResignMainNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    LevelOver(notification.object, false);
                  }];
  [center addObserverForName:NSApplicationDidBecomeActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    LevelOver(NSApp.mainWindow, true);
                  }];
  // Before the tooltip and Site Controls start fading out as the app goes.
  [center addObserverForName:NSApplicationWillResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    for (NSWindow* window in NSApp.windows) {
                      LevelOver(window, false);
                    }
                  }];
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
  // The Window menu lists the browser windows only.
  window.excludedFromWindowsMenu = YES;
  // AppKit would otherwise hide the panel when the app deactivates and show it again on return.
  ((NSPanel*)window).hidesOnDeactivate = NO;
  // GPUI's class for its panels, which are all overlays here.
  SEL constrain = @selector(constrainFrameRect:toScreen:);
  class_addMethod(object_getClass(window), constrain, (IMP)Unconstrained,
                  method_getTypeEncoding(class_getInstanceMethod(NSWindow.class, constrain)));
}

void PlaceFindBar(Browser* browser) {
  NSRect content = [browser->window contentRectForFrameRect:browser->window.frame];
  NSSize size = browser->find_bar.frame.size;
  [browser->find_bar
      setFrameOrigin:NSMakePoint(NSMaxX(content) - browser->page_right -
                                     browser->find_bar_right - size.width,
                                 NSMaxY(content) - browser->page_top - browser->find_bar_top -
                                     size.height)];
}

// 8 points below the find bar while it shows, which would cover it.
void PlaceOffer(Browser* browser) {
  NSRect content = [browser->window contentRectForFrameRect:browser->window.frame];
  NSSize size = browser->offer.frame.size;
  const double top = browser->find_bar_shown
                         ? browser->find_bar_top + browser->find_bar.frame.size.height + 8
                         : browser->offer_top;
  [browser->offer
      setFrameOrigin:NSMakePoint(NSMaxX(content) - browser->page_right - browser->offer_right -
                                     size.width,
                                 NSMaxY(content) - browser->page_top - top - size.height)];
}

// The window of |browser| that gets the keyboard back when what had it over the page goes: the find
// bar or the offer if the user gave it the keyboard, else the browser window.
NSWindow* KeyWindowOver(Browser* browser) {
  if (browser->find_bar_shown && browser->find_bar_keyboard) {
    return browser->find_bar;
  }
  if (browser->offer_shown && browser->offer_keyboard) {
    return browser->offer;
  }
  return browser->window;
}

// Where the UI asked, from the top left of the browser window's content, which a window resized
// from its top moves.
void PlaceSiteControls() {
  NSWindow* window = g_site_controls_over->window;
  NSRect content = [window contentRectForFrameRect:window.frame];
  NSSize size = g_site_controls.frame.size;
  [g_site_controls setFrameOrigin:NSMakePoint(NSMinX(content) + g_site_controls_left,
                                              NSMaxY(content) - g_site_controls_top - size.height)];
}

// The user left the Site Controls. Posted: the UI may be handling the event that left them.
void DismissSiteControls() {
  if (!g_site_controls_shown || g_site_controls_released) {
    return;
  }
  host_release_site_controls();
  if (g_site_controls_dismissed) {
    dispatch_async(dispatch_get_main_queue(), ^{
      g_site_controls_dismissed();
    });
  }
}

// The user left Capture. Posted: the UI may be handling the event that left it.
void DismissCapture() {
  if (!g_capture_shown) {
    return;
  }
  host_hide_capture();
  if (g_capture_dismissed) {
    dispatch_async(dispatch_get_main_queue(), ^{
      g_capture_dismissed();
    });
  }
}

// At the browser window's top right corner, where the toast draws itself.
void PlaceToast() {
  NSRect frame = g_toast_over->window.frame;
  NSSize size = g_toast.frame.size;
  [g_toast setFrameOrigin:NSMakePoint(NSMaxX(frame) - size.width, NSMaxY(frame) - size.height)];
}

// 8 points below the find bar while it shows, where the HUD would cover it.
void PlaceHud() {
  Browser* browser = g_hud_over;
  NSRect content = [browser->window contentRectForFrameRect:browser->window.frame];
  NSSize size = g_hud.frame.size;
  const double top = browser->find_bar_shown
                         ? browser->find_bar_top + browser->find_bar.frame.size.height + 8 -
                               g_hud_margin
                         : g_hud_top;
  [g_hud setFrameOrigin:NSMakePoint(
                            NSMaxX(content) - browser->page_right - g_hud_right - size.width,
                            NSMaxY(content) - browser->page_top - top - size.height)];
}

void host_add_find_bar(void* ui_view, void* find_bar_view, double right, double top) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  browser->find_bar_view = (__bridge NSView*)find_bar_view;
  browser->find_bar = browser->find_bar_view.window;
  browser->find_bar_right = right;
  browser->find_bar_top = top;
  MakeOverlay(browser->find_bar);
  // The UI draws the bar's rounded shape, which a borderless window's shadow takes.
  browser->find_bar.styleMask = NSWindowStyleMaskBorderless;
  browser->find_bar.hasShadow = YES;
}

void host_show_find_bar(void* ui_view, bool keyboard) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->find_bar || !browser->window) {
    return;
  }
  NSWindow* find_bar = browser->find_bar;
  browser->find_bar_keyboard = browser->find_bar_keyboard || keyboard;
  if (keyboard) {
    browser->offer_keyboard = false;
  }
  if (browser->find_bar_shown) {
    // Once it's on screen. Until then, it takes the keyboard as it goes on.
    if (keyboard && find_bar.visible) {
      [find_bar makeKeyAndOrderFront:nil];
      ReleaseKeys(find_bar);
    } else if (keyboard) {
      HoldKeys(find_bar);
    }
    return;
  }
  browser->find_bar_shown = true;
  if (keyboard) {
    HoldKeys(find_bar);
  }
  // As the command bar does: it goes on screen once it has drawn what it shows now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!browser->find_bar_shown) {
      ReleaseKeys(find_bar);
      return;
    }
    [browser->find_bar_view.layer setNeedsDisplay];
    [browser->find_bar_view.layer displayIfNeeded];
    [find_bar invalidateShadow];
    PlaceFindBar(browser);
    AddOver(browser->window, find_bar);
    if (browser->find_bar_keyboard) {
      // From the command bar too, which closes.
      [find_bar makeKeyAndOrderFront:nil];
      ReleaseKeys(find_bar);
    } else {
      RaiseOverlays(browser, find_bar);
    }
    if (browser->offer_shown) {
      PlaceOffer(browser);
    }
    if (g_hud_shown && g_hud_over == browser) {
      PlaceHud();
    }
    fprintf(stderr, "browser: find bar shown\n");
  });
}

void host_hide_find_bar(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->find_bar_shown) {
    return;
  }
  browser->find_bar_shown = false;
  browser->find_bar_keyboard = false;
  const bool had_keyboard = browser->find_bar.keyWindow;
  [browser->window removeChildWindow:browser->find_bar];
  [browser->find_bar orderOut:nil];
  if (had_keyboard) {
    [browser->window makeKeyWindow];
  }
  if (browser->offer_shown) {
    PlaceOffer(browser);
  }
  if (g_hud_shown && g_hud_over == browser) {
    PlaceHud();
  }
  fprintf(stderr, "browser: find bar hidden\n");
}

void host_add_offer(void* ui_view, void* offer_view, double right, double top) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  browser->offer_view = (__bridge NSView*)offer_view;
  browser->offer = browser->offer_view.window;
  browser->offer_right = right;
  browser->offer_top = top;
  MakeOverlay(browser->offer);
  // The UI draws the card's rounded shape, which a borderless window's shadow takes.
  browser->offer.styleMask = NSWindowStyleMaskBorderless;
  browser->offer.hasShadow = YES;
}

void host_show_offer(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->offer || !browser->window || browser->offer_shown) {
    return;
  }
  browser->offer_shown = true;
  NSWindow* offer = browser->offer;
  // As the find bar does: it goes on screen once it has drawn what it shows now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!browser->offer_shown) {
      return;
    }
    [browser->offer_view.layer setNeedsDisplay];
    [browser->offer_view.layer displayIfNeeded];
    [offer invalidateShadow];
    PlaceOffer(browser);
    AddOver(browser->window, offer);
    RaiseOverlays(browser, offer);
    // The core reports an offer when it comes, which can be while a dialog covers the window.
    if (g_dialog_shown && g_dialog_over == browser && g_dialog.visible) {
      [g_dialog orderWindow:NSWindowAbove relativeTo:offer.windowNumber];
    }
    fprintf(stderr, "browser: offer shown\n");
  });
}

void host_hide_offer(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->offer_shown) {
    return;
  }
  browser->offer_shown = false;
  browser->offer_keyboard = false;
  const bool had_keyboard = browser->offer.keyWindow;
  [browser->window removeChildWindow:browser->offer];
  [browser->offer orderOut:nil];
  if (had_keyboard) {
    [browser->window makeKeyWindow];
  }
  fprintf(stderr, "browser: offer hidden\n");
}

void host_set_pane_corner(void* ui_view, double right, double top) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  browser->page_right = right;
  browser->page_top = top;
  if (browser->find_bar_shown) {
    PlaceFindBar(browser);
  }
  if (browser->offer_shown) {
    PlaceOffer(browser);
  }
  if (g_hud_shown && g_hud_over == browser) {
    PlaceHud();
  }
}

void host_add_hud(void* hud_view, double right, double top, double margin) {
  g_hud_view = (__bridge NSView*)hud_view;
  g_hud = g_hud_view.window;
  g_hud_right = right;
  g_hud_top = top;
  g_hud_margin = margin;
  MakeOverlay(g_hud);
  g_hud.styleMask = NSWindowStyleMaskBorderless;
  // The HUD draws its pill's shadow, which grows and shrinks with it.
  g_hud.hasShadow = NO;
  g_hud.ignoresMouseEvents = YES;
}

void host_show_hud(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_hud || !browser || !browser->window) {
    return;
  }
  g_hud_shown = true;
  g_hud_over = browser;
  const unsigned change = ++g_hud_changes;
  // As the find bar does, once it has drawn what it shows now, which may be narrower than before.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (change != g_hud_changes) {
      return;
    }
    [g_hud_view.layer setNeedsDisplay];
    [g_hud_view.layer displayIfNeeded];
    PlaceHud();
    if (g_hud.parentWindow != browser->window) {
      [g_hud.parentWindow removeChildWindow:g_hud];
      AddOver(browser->window, g_hud);
    }
    if (browser->find_bar_shown) {
      [g_hud orderWindow:NSWindowAbove relativeTo:browser->find_bar.windowNumber];
    }
    RaiseOverlays(browser, g_hud);
    fprintf(stderr, "browser: zoom hud shown\n");
  });
}

void host_hide_hud() {
  if (!g_hud_shown) {
    return;
  }
  g_hud_shown = false;
  // Cancels a show still waiting for the HUD to draw.
  ++g_hud_changes;
  [g_hud.parentWindow removeChildWindow:g_hud];
  [g_hud orderOut:nil];
  fprintf(stderr, "browser: zoom hud hidden\n");
}

void host_add_drop_targets(void* view) {
  g_drop_targets_view = (__bridge NSView*)view;
  g_drop_targets = g_drop_targets_view.window;
  MakeOverlay(g_drop_targets);
  g_drop_targets.styleMask = NSWindowStyleMaskBorderless;
  g_drop_targets.hasShadow = NO;
  g_drop_targets.ignoresMouseEvents = YES;
}

void host_show_drop_targets(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_drop_targets || !browser || !browser->window) {
    return;
  }
  if (g_drop_targets_shown && g_drop_targets_over != browser) {
    host_hide_drop_targets();
  }
  if (g_drop_targets_shown) {
    return;
  }
  g_drop_targets_shown = true;
  g_drop_targets_over = browser;
  // After the UI's update that asked, since the window draws here: at the content's frame, before
  // it shows, since it keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_drop_targets_shown || g_drop_targets_over != browser ||
        g_drop_targets.parentWindow) {
      return;
    }
    NSWindow* window = browser->window;
    [g_drop_targets setFrame:[window contentRectForFrameRect:window.frame] display:NO];
    [g_drop_targets_view.layer setNeedsDisplay];
    [g_drop_targets_view.layer displayIfNeeded];
    AddOver(window, g_drop_targets);
    fprintf(stderr, "browser: drop targets shown\n");
  });
}

void host_hide_drop_targets(void) {
  g_drop_targets_shown = false;
  if (!g_drop_targets.parentWindow) {
    return;
  }
  [g_drop_targets.parentWindow removeChildWindow:g_drop_targets];
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

void host_show_dialog(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_dialog || !browser || !browser->window || g_dialog_shown) {
    return;
  }
  g_dialog_shown = true;
  g_dialog_over = browser;
  // After the UI's update that asked, since the window draws here: at the browser window's frame,
  // before it shows, since it keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_dialog_shown) {
      return;
    }
    [g_dialog setFrame:browser->window.frame display:NO];
    [g_dialog_view.layer setNeedsDisplay];
    [g_dialog_view.layer displayIfNeeded];
    AddOver(browser->window, g_dialog);
    [g_dialog makeKeyAndOrderFront:nil];
    fprintf(stderr, "browser: dialog window shown\n");
  });
}

void host_hide_dialog(void) {
  if (!g_dialog_shown) {
    return;
  }
  g_dialog_shown = false;
  Browser* browser = g_dialog_over;
  [browser->window removeChildWindow:g_dialog];
  [g_dialog orderOut:nil];
  [KeyWindowOver(browser) makeKeyWindow];
}

void host_add_site_controls(void* view, void (*dismissed)(void)) {
  g_site_controls_view = (__bridge NSView*)view;
  g_site_controls = g_site_controls_view.window;
  g_site_controls_dismissed = dismissed;
  MakeOverlay(g_site_controls);
  Overlay& overlay = g_site_controls_overlay;
  overlay.view = g_site_controls_view;
  overlay.window = g_site_controls;
  AddBlurAndShadow(overlay);
  // Until the frame at a new height lands, the last one stays as drawn at the top.
  g_site_controls_view.layer.contentsGravity = kCAGravityTopLeft;
  // They close when the user leaves them, as the command bar does: a click outside them (the
  // browser window's click monitor), another window taking the keyboard, or switching apps.
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    DismissSiteControls();
                  }];
  // Chromium makes its window key by itself, such as when a page commits. That isn't the user
  // leaving, so they take the keyboard back.
  [center addObserverForName:NSWindowDidBecomeKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    NSWindow* window = notification.object;
                    if (!g_site_controls_shown || g_site_controls_released ||
                        window == g_site_controls || !g_site_controls.visible) {
                      return;
                    }
                    if (BrowserOf(window) == g_site_controls_over) {
                      [g_site_controls makeKeyWindow];
                    } else {
                      DismissSiteControls();
                    }
                  }];
}

void host_set_site_controls_shape(const double* corners, size_t count) {
  g_site_controls_overlay.corners.assign(corners, corners + 3 * count);
}

namespace {

// The blur and the shadow in the Site Controls' shape at their window's size now.
void ShapeSiteControls() {
  Overlay& overlay = g_site_controls_overlay;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  overlay.blur.frame = overlay.view.frame;
  // At its full size while it's placed, as a transform would scale the new frame.
  overlay.shadow_layer.superlayer.affineTransform = CGAffineTransformIdentity;
  [CATransaction commit];
  MaskOverlayBlur(overlay);
  // Drawn again at the new shape.
  [overlay.shadow setFrame:NSZeroRect display:NO];
  PlaceOverlayShadow(overlay);
}

}  // namespace

void host_show_site_controls(void* ui_view, double left, double top, double width, double height) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_site_controls || !browser || !browser->window) {
    return;
  }
  if (g_site_controls_shown && g_site_controls_over != browser) {
    host_hide_site_controls();
  }
  g_site_controls_shown = true;
  g_site_controls_released = false;
  g_site_controls_over = browser;
  g_site_controls_left = left;
  g_site_controls_top = top;
  g_site_controls.ignoresMouseEvents = NO;
  // As the command bar does: on screen once the UI has drawn it as it is now, which it can't while
  // it handles the press that opened it.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_site_controls_shown || g_site_controls_released || g_site_controls_over != browser) {
      return;
    }
    NSWindow* window = browser->window;
    Overlay& overlay = g_site_controls_overlay;
    [g_site_controls setFrame:NSMakeRect(0, 0, width, height) display:NO];
    PlaceSiteControls();
    [g_site_controls_view.layer setNeedsDisplay];
    [g_site_controls_view.layer displayIfNeeded];
    ShapeSiteControls();
    // They come in from nothing, as the UI's frames say (host_site_controls_drew).
    overlay.blur.alphaValue = 0;
    overlay.shadow.alphaValue = 0;
    if (overlay.shadow.parentWindow != window) {
      [overlay.shadow.parentWindow removeChildWindow:overlay.shadow];
      AddOver(window, overlay.shadow);
    }
    AddOver(window, g_site_controls);
    [g_site_controls makeKeyAndOrderFront:nil];
    // check.sh reads these lines.
    fprintf(stderr, "browser: site controls at %g %g %g %g\n", left, top, width, height);
    fprintf(stderr, "browser: site controls shown\n");
  });
}

void host_set_site_controls_height(double height) {
  // After the UI's update that asked, since they draw at their new size here.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_site_controls_shown || NSHeight(g_site_controls.frame) == height) {
      return;
    }
    NSRect frame = g_site_controls.frame;
    [g_site_controls
        setFrame:NSMakeRect(NSMinX(frame), NSMaxY(frame) - height, NSWidth(frame), height)
         display:NO];
    [g_site_controls_view.layer setNeedsDisplay];
    [g_site_controls_view.layer displayIfNeeded];
    ShapeSiteControls();
    fprintf(stderr, "browser: site controls height %g\n", height);
  });
}

void host_site_controls_drew(double scale, double opacity, double x, double y) {
  // Posted, so the blur and the shadow take the frame the UI is drawing now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_site_controls_shown) {
      return;
    }
    Overlay& overlay = g_site_controls_overlay;
    const NSRect full = overlay.view.frame;
    // From the window's top left to AppKit's bottom left.
    const double up = NSHeight(full) - y;
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    overlay.blur.frame =
        NSMakeRect(NSMinX(full) + x * (1 - scale), NSMinY(full) + up * (1 - scale),
                   NSWidth(full) * scale, NSHeight(full) * scale);
    overlay.blur.alphaValue = opacity;
    // Scaled about the same point of the shadow's window, which reaches past the panel's.
    CALayer* holder = overlay.shadow_layer.superlayer;
    const double reach = overlay.reach();
    const CGPoint about = CGPointMake(reach + x, reach + up);
    // Its middle, which a layer's transform scales about.
    const CGPoint middle = holder.position;
    holder.affineTransform =
        CGAffineTransformMake(scale, 0, 0, scale, (1 - scale) * (about.x - middle.x),
                              (1 - scale) * (about.y - middle.y));
    [CATransaction commit];
    overlay.shadow.alphaValue = opacity;
  });
}

void host_release_site_controls(void) {
  if (!g_site_controls_shown || g_site_controls_released) {
    return;
  }
  g_site_controls_released = true;
  g_site_controls.ignoresMouseEvents = YES;
  if (g_site_controls.keyWindow) {
    Browser* browser = g_site_controls_over;
    [KeyWindowOver(browser) makeKeyWindow];
  }
  fprintf(stderr, "browser: site controls released\n");
}

void host_hide_site_controls(void) {
  if (!g_site_controls_shown) {
    return;
  }
  host_release_site_controls();
  g_site_controls_shown = false;
  [g_site_controls_over->window removeChildWindow:g_site_controls];
  [g_site_controls orderOut:nil];
  g_site_controls_overlay.shadow.alphaValue = 0;
  fprintf(stderr, "browser: site controls hidden\n");
}

void host_add_capture(void* view, void (*dismissed)(void)) {
  g_capture_view = (__bridge NSView*)view;
  g_capture = g_capture_view.window;
  g_capture_dismissed = dismissed;
  g_capture_cursor = CameraCursor();
  MakeOverlay(g_capture);
  // The UI draws the page's rounded card, which the browser window's shadow is already under.
  g_capture.styleMask = NSWindowStyleMaskBorderless;
  g_capture.hasShadow = NO;
  // GPUI keeps its pointer in cursor rects, which would put the arrow back over the camera.
  [g_capture disableCursorRects];
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    DismissCapture();
                  }];
  // Chromium makes its window key by itself, such as when a page commits. That isn't the user
  // leaving, so Capture takes the keyboard back.
  [center addObserverForName:NSWindowDidBecomeKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    NSWindow* window = notification.object;
                    if (!g_capture_shown || window == g_capture || !g_capture.visible) {
                      return;
                    }
                    if (BrowserOf(window) == g_capture_over) {
                      [g_capture makeKeyWindow];
                    } else {
                      DismissCapture();
                    }
                  }];
}

void host_show_capture(void* ui_view, double left, double top, double width, double height) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_capture || !browser || !browser->window) {
    return;
  }
  if (g_capture_shown && g_capture_over != browser) {
    host_hide_capture();
  }
  g_capture_shown = true;
  g_capture_over = browser;
  g_capture_camera = true;
  // As the dialog does: on screen once the UI has drawn the picture of the page it covers, so the
  // page doesn't flash between.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_capture_shown || g_capture_over != browser || g_capture.parentWindow) {
      return;
    }
    NSWindow* window = browser->window;
    NSRect content = [window contentRectForFrameRect:window.frame];
    g_capture_frame = NSMakeRect(NSMinX(content) + left, NSMaxY(content) - top - height, width,
                                 height);
    [g_capture setFrame:g_capture_frame display:NO];
    [g_capture_view.layer setNeedsDisplay];
    [g_capture_view.layer displayIfNeeded];
    AddOver(window, g_capture);
    [g_capture makeKeyAndOrderFront:nil];
    SetCaptureCursor();
    g_capture_monitor = [NSEvent
        addLocalMonitorForEventsMatchingMask:NSEventMaskMouseMoved | NSEventMaskLeftMouseDragged |
                                             NSEventMaskLeftMouseUp | NSEventMaskMouseEntered
                                     handler:^NSEvent*(NSEvent* event) {
                                       if (event.window == g_capture) {
                                         SetCaptureCursor();
                                       }
                                       return event;
                                     }];
    // check.sh reads this line.
    fprintf(stderr, "browser: capture shown at %g %g %g %g\n", left, top, width, height);
  });
}

void host_capture_camera(bool camera) {
  g_capture_camera = camera;
  if (g_capture_shown && g_capture.parentWindow) {
    SetCaptureCursor();
  }
}

void host_hide_capture(void) {
  if (!g_capture_shown) {
    return;
  }
  g_capture_shown = false;
  if (g_capture_monitor) {
    [NSEvent removeMonitor:g_capture_monitor];
    g_capture_monitor = nil;
  }
  Browser* browser = g_capture_over;
  const bool key = g_capture.keyWindow;
  if (g_capture.parentWindow) {
    [browser->window removeChildWindow:g_capture];
  }
  [g_capture orderOut:nil];
  if (key) {
    [KeyWindowOver(browser) makeKeyWindow];
  }
  [NSCursor.arrowCursor set];
  fprintf(stderr, "browser: capture hidden\n");
}

void host_toast_takes_clicks(bool takes) {
  g_toast.ignoresMouseEvents = !takes;
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

void host_show_toast(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_toast || !browser || !browser->window) {
    return;
  }
  if (g_toast_shown && g_toast_over != browser) {
    host_hide_toast();
  }
  if (g_toast_shown) {
    return;
  }
  g_toast_shown = true;
  g_toast_over = browser;
  // After the UI's update that asked, since the window draws here, before it shows, since it
  // keeps the last frame it drew.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!g_toast_shown || g_toast_over != browser) {
      return;
    }
    PlaceToast();
    [g_toast_view.layer setNeedsDisplay];
    [g_toast_view.layer displayIfNeeded];
    AddOver(browser->window, g_toast);
    RaiseOverlays(browser, g_toast);
    fprintf(stderr, "browser: toast window shown\n");
  });
}

void host_hide_toast(void) {
  if (!g_toast_shown) {
    return;
  }
  g_toast_shown = false;
  [g_toast_over->window removeChildWindow:g_toast];
  [g_toast orderOut:nil];
}

void host_add_tooltip(void* tooltip_view, void (*away)(void)) {
  g_tooltip_view = (__bridge NSView*)tooltip_view;
  g_tooltip = g_tooltip_view.window;
  g_tooltip_away = away;
  MakeOverlay(g_tooltip);
  g_tooltip.styleMask = NSWindowStyleMaskBorderless;
  g_tooltip.hasShadow = NO;
  // It takes neither a click nor the keyboard.
  g_tooltip.ignoresMouseEvents = YES;
  // To accessibility it's a help tag, as AppKit's tooltips are, not one of the app's windows.
  g_tooltip.accessibilityRole = NSAccessibilityHelpTagRole;
  // The user leaves the browser window for another or another app.
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  void (^left_window)(NSNotification*) = ^(NSNotification*) {
    if (g_tooltip_shown && g_tooltip_away) {
      g_tooltip_away();
    }
  };
  [center addObserverForName:NSWindowDidResignKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    if (g_tooltip_shown && notification.object == g_tooltip_over->window) {
                      left_window(notification);
                    }
                  }];
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:left_window];
}

// The window's alpha, which AppKit animates, so the fade wakes nothing of ours but its end. With
// no seconds it's set in the group, which also ends a fade still running.
void FadeTooltip(double alpha, double seconds, void (^done)(void)) {
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = seconds;
        context.timingFunction =
            [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseOut];
        g_tooltip.animator.alphaValue = alpha;
      }
      completionHandler:done];
}

void host_show_tooltip(void* ui_view, double left, double top, double fade) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_tooltip || !browser || !browser->window) {
    return;
  }
  if (g_tooltip_shown && g_tooltip_over != browser) {
    host_hide_tooltip(0);
  }
  g_tooltip_left = left;
  g_tooltip_top = top;
  g_tooltip_shown = true;
  g_tooltip_over = browser;
  const unsigned change = ++g_tooltip_changes;
  // After the UI's update that asked, since the window draws here, before it shows or moves, since
  // it keeps the last frame it drew, the last name.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (change != g_tooltip_changes) {
      return;
    }
    PlaceTooltip();
    {
      // Where the window really is, from the browser window's content top left, for check.sh to
      // compare with where the UI asked.
      NSWindow* over = browser->window;
      NSRect content = [over contentRectForFrameRect:over.frame];
      fprintf(stderr, "browser: tooltip window at %g %g\n",
              NSMinX(g_tooltip.frame) - NSMinX(content),
              NSMaxY(content) - NSMaxY(g_tooltip.frame));
    }
    [g_tooltip_view.layer setNeedsDisplay];
    [g_tooltip_view.layer displayIfNeeded];
    // Still fading out over this window, it goes back to full, as Arc's shows in one frame.
    if (g_tooltip.parentWindow != browser->window) {
      [g_tooltip.parentWindow removeChildWindow:g_tooltip];
      g_tooltip.alphaValue = 0;
      AddOver(browser->window, g_tooltip);
      RaiseOverlays(browser, g_tooltip);
    }
    FadeTooltip(1, fade, ^{
      if (change == g_tooltip_changes) {
        // check.sh waits for this line, so a capture after it shows the tooltip whole.
        fprintf(stderr, "browser: tooltip window shown\n");
      }
    });
  });
}

void host_hide_tooltip(double fade) {
  // At once, it also ends a fade out.
  if (!g_tooltip_shown && fade > 0) {
    return;
  }
  g_tooltip_shown = false;
  const unsigned change = ++g_tooltip_changes;
  void (^gone)(void) = ^{
    if (change != g_tooltip_changes) {
      return;
    }
    [g_tooltip.parentWindow removeChildWindow:g_tooltip];
    [g_tooltip orderOut:nil];
  };
  // At once as its window closes, which a fade's end would come after.
  if (fade == 0) {
    gone();
    return;
  }
  FadeTooltip(0, fade, gone);
}

void host_add_link_target(void* link_view, void (*pointer)(double x, double y)) {
  g_link_view = (__bridge NSView*)link_view;
  g_link = g_link_view.window;
  g_link_pointer = pointer;
  MakeOverlay(g_link);
  g_link.styleMask = NSWindowStyleMaskBorderless;
  g_link.hasShadow = NO;
  // It takes neither a click nor the keyboard.
  g_link.ignoresMouseEvents = YES;
}

void host_show_link_target(void* ui_view, double left, double top, double fade) {
  Browser* browser = BrowserWithView(ui_view);
  if (!g_link || !browser || !browser->window) {
    return;
  }
  if (g_link_shown && g_link_over != browser) {
    host_hide_link_target(0);
  }
  g_link_left = left;
  g_link_top = top;
  g_link_shown = true;
  g_link_over = browser;
  const unsigned change = ++g_link_changes;
  // The page's view takes the pointer's moves while it's over the page, so the UI's view doesn't
  // hear them. Only while the plate shows, so an idle browser is never woken.
  if (!g_link_monitor) {
    g_link_monitor = [NSEvent
        addLocalMonitorForEventsMatchingMask:NSEventMaskMouseMoved
                                     handler:^NSEvent*(NSEvent* event) {
                                       if (!g_link_shown) {
                                         return event;
                                       }
                                       NSPoint at = event.locationInWindow;
                                       if (event.window) {
                                         at = [event.window convertPointToScreen:at];
                                       }
                                       NSWindow* window = g_link_over->window;
                                       NSRect content =
                                           [window contentRectForFrameRect:window.frame];
                                       g_link_pointer(at.x - NSMinX(content),
                                                      NSMaxY(content) - at.y);
                                       return event;
                                     }];
  }
  // After the UI's update that asked, since the window draws here, before it shows or moves, since
  // it keeps the last frame it drew, the last address.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (change != g_link_changes) {
      return;
    }
    PlaceLinkTarget();
    [g_link_view.layer setNeedsDisplay];
    [g_link_view.layer displayIfNeeded];
    // Still fading out over this window, it goes back to full.
    if (g_link.parentWindow != browser->window) {
      [g_link.parentWindow removeChildWindow:g_link];
      g_link.alphaValue = 0;
      [browser->window addChildWindow:g_link ordered:NSWindowAbove];
      RaiseOverlays(browser, g_link);
    }
    [NSAnimationContext
        runAnimationGroup:^(NSAnimationContext* context) {
          context.duration = fade;
          context.timingFunction =
              [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseOut];
          g_link.animator.alphaValue = 1;
        }
        completionHandler:^{
          if (change == g_link_changes) {
            // check.sh waits for this line, so a capture after it shows the plate whole.
            fprintf(stderr, "browser: link target window shown\n");
          }
        }];
  });
}

void host_hide_link_target(double fade) {
  // At once, it also ends a fade out.
  if (!g_link_shown && fade > 0) {
    return;
  }
  g_link_shown = false;
  if (g_link_monitor) {
    [NSEvent removeMonitor:g_link_monitor];
    g_link_monitor = nil;
  }
  const unsigned change = ++g_link_changes;
  void (^gone)(void) = ^{
    if (change != g_link_changes) {
      return;
    }
    [g_link.parentWindow removeChildWindow:g_link];
    [g_link orderOut:nil];
  };
  // At once as its window closes, which a fade's end would come after.
  if (fade == 0) {
    gone();
    return;
  }
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = fade;
        context.timingFunction =
            [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseOut];
        g_link.animator.alphaValue = 0;
      }
      completionHandler:gone];
}

void host_add_reveal(void* ui_view,
                     void* reveal_view,
                     void (*pointer_moved)(void* ui_view, double x)) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  browser->reveal_view = (__bridge NSView*)reveal_view;
  NSWindow* reveal = browser->reveal_view.window;
  browser->reveal = reveal;
  browser->reveal_pointer = pointer_moved;
  MakeOverlay(reveal);
  // A click in the panel leaves the keyboard where it was, in the page or the browser window.
  ((NSPanel*)reveal).becomesKeyOnlyIfNeeded = YES;
  reveal.hasShadow = NO;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSWindowDidResizeNotification
                      object:browser->window
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (browser->reveal_monitor) {
                      PlaceReveal(browser);
                    }
                  }];
  // The pointer can't be in the panel's hold zone in an app that isn't the active one.
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (browser->reveal_monitor) {
                      browser->reveal_pointer(ui_view, INFINITY);
                    }
                  }];
}

void host_show_reveal(void* ui_view, double width) {
  Browser* browser = BrowserWithView(ui_view);
  // After the UI's update that asked: the window draws as it goes on screen and as it resizes.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!browser || !browser->reveal || !browser->window) {
      return;
    }
    browser->reveal_width = width;
    if (browser->reveal_monitor) {
      PlaceReveal(browser);
      return;
    }
    // As the command bar does, it goes on screen once it has drawn what it shows now.
    PlaceReveal(browser);
    [browser->reveal_view.layer setNeedsDisplay];
    [browser->reveal_view.layer displayIfNeeded];
    AddOver(browser->window, browser->reveal);
    // The page's view takes the pointer's moves while it's over the page, so the panel's view
    // doesn't hear them leave.
    browser->reveal_monitor = [NSEvent
        addLocalMonitorForEventsMatchingMask:NSEventMaskMouseMoved | NSEventMaskLeftMouseDragged
                                     handler:^NSEvent*(NSEvent* event) {
                                       NSPoint at = event.locationInWindow;
                                       if (event.window) {
                                         at = [event.window convertPointToScreen:at];
                                       }
                                       at = [browser->window convertPointFromScreen:at];
                                       browser->reveal_pointer(ui_view, at.x);
                                       return event;
                                     }];
    fprintf(stderr, "browser: reveal window shown\n");
  });
}

void host_hide_reveal(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!browser || !browser->reveal_monitor) {
      return;
    }
    [NSEvent removeMonitor:browser->reveal_monitor];
    browser->reveal_monitor = nil;
    // Asked before it goes: a window that hides while key leaves AppKit to pick the next key one
    // later, and the panel can still be it when a key arrives.
    const bool had_keyboard = browser->reveal.keyWindow;
    [browser->window removeChildWindow:browser->reveal];
    [browser->reveal orderOut:nil];
    // Its first responder, the page or the UI, has the keyboard again.
    if (had_keyboard) {
      [browser->window makeKeyWindow];
    }
  });
}
