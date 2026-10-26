#import <Cocoa/Cocoa.h>
#import <ImageIO/ImageIO.h>
#import <QuartzCore/QuartzCore.h>
#import <objc/runtime.h>

#include <algorithm>
#include <initializer_list>
#include <iterator>
#include <memory>
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
bool (*g_dragged)(void*, int, double, double, const host_dragged*, size_t);
}  // namespace

// The source of a file host_drag_file drags, which offers a copy wherever it's let go.
@interface HostFileDragSource : NSObject <NSDraggingSource>
@property(nonatomic) void (*ended)(bool);
@end

@implementation HostFileDragSource
@synthesize ended = _ended;

- (NSDragOperation)draggingSession:(NSDraggingSession*)session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)context {
  return NSDragOperationCopy;
}

- (void)draggingSession:(NSDraggingSession*)session
           endedAtPoint:(NSPoint)point
              operation:(NSDragOperation)operation {
  self.ended(operation != NSDragOperationNone);
}
@end

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
  return g_dragged((__bridge void*)self, phase, point.x, NSHeight(self.bounds) - point.y,
                   items.data(), items.size());
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

// Takes the tint and the extra saturation off |layer| and the layers in it, leaving the blur, as
// GPUI's blurred window background does (crates/gpui_macos/src/window.rs).
void StripTint(CALayer* layer) {
  layer.backgroundColor = nil;
  if ([layer.className isEqualToString:@"CAChameleonLayer"]) {
    layer.hidden = YES;
    return;
  }
  NSPredicate* saturating = [NSPredicate predicateWithBlock:^BOOL(id filter, NSDictionary*) {
    return ![[filter description] containsString:@"Saturat"];
  }];
  if (layer.filters) {
    layer.filters = [layer.filters filteredArrayUsingPredicate:saturating];
  }
  for (CALayer* sublayer in layer.sublayers) {
    StripTint(sublayer);
  }
}

}  // namespace

// What's behind the window, blurred, without the material's tint, for the command bar's fill to
// tint (docs/ux-spec.md › Command bar). With Reduce Transparency, AppKit's opaque fill stays.
@interface HostBlurView : NSVisualEffectView
@end

@implementation HostBlurView
- (void)updateLayer {
  [super updateLayer];
  if (!NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceTransparency) {
    StripTint(self.layer);
  }
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

// A browser window, once shown, the view the UI draws in, and the windows over it that are its
// own. The others show over one browser window at a time, the one they're asked to.
struct Browser {
  NSWindow* window;
  NSView* ui_view;

  // The find bar's window and view, and its distance from the top right of the window's content.
  NSWindow* find_bar;
  NSView* find_bar_view;
  bool find_bar_shown = false;
  double find_bar_right = 0;
  double find_bar_top = 0;
  // The find bar has the keyboard, or gets it back once the command bar closes: it took it opening
  // or from a click, and the user hasn't clicked the browser window since. AppKit also makes it key
  // when the command bar closes, which isn't the user choosing it.
  bool find_bar_keyboard = false;

  // How far the focused page's top right corner is inside the card's, in a split view, which the
  // find bar and the HUD keep to.
  double page_right = 0;
  double page_top = 0;

  // The sidebar reveal's window and view, the shell's handler for the pointer, the window's width,
  // and while it shows, the monitor that hands the handler the pointer.
  NSWindow* reveal;
  NSView* reveal_view;
  void (*reveal_pointer)(void* ui_view, double x);
  double reveal_width = 0;
  id reveal_monitor;
};

// In the order their views were made. Each is allocated alone, so the pointers stay put.
std::vector<std::unique_ptr<Browser>> g_browsers;

// The browser whose UI draws in |ui_view|, or null.
Browser* BrowserWithView(void* ui_view) {
  for (auto& browser : g_browsers) {
    if ((__bridge void*)browser->ui_view == ui_view) {
      return browser.get();
    }
  }
  return nullptr;
}

// The browser whose window is |window|, or null.
Browser* BrowserOf(NSWindow* window) {
  for (auto& browser : g_browsers) {
    if (window && browser->window == window) {
      return browser.get();
    }
  }
  return nullptr;
}

// The browser window in front, else the first still open.
Browser* FrontBrowser() {
  for (NSWindow* window in NSApp.orderedWindows) {
    if (Browser* browser = BrowserOf(window)) {
      return browser;
    }
  }
  for (auto& browser : g_browsers) {
    if (browser->window) {
      return browser.get();
    }
  }
  return nullptr;
}

// The overlays' shadow, Arc's command bar's: soft and even on every side (Arc 1.170.0 cap over a
// light page, fitted to a Gaussian). It's a window of its own under the overlay's that the mouse
// goes through, so a click on it lands on the browser window and closes the overlay as any click
// outside it does, and the overlay's window keeps the overlay's frame. It goes on screen with the
// overlay's first show over a browser window and stays there, shown and hidden by its alpha, since
// putting a window on screen cost each ⌘T about 4 ms.
constexpr double kOverlayShadowRadius = 20;
constexpr float kOverlayShadowOpacity = 0.23;
// Past this, three times the blur, the shadow is nothing.
constexpr double kOverlayShadowReach = 3 * kOverlayShadowRadius;

// A window of the UI's over a browser window's page, translucent over a blur, with a shadow: the
// command bar, or the Space's icon picker.
struct Overlay {
  explicit Overlay(const char* name) : name(name) {}
  // What its log lines call it, which check.sh reads.
  const char* name;
  NSWindow* window = nil;
  NSView* view = nil;
  HostBlurView* blur = nil;
  // The browser it shows over.
  Browser* over = nullptr;
  bool shown = false;
  void (*dismissed)(void) = nullptr;
  double radius = 0;
  // From host_set_overlay_shape: x, y and radius for each corner, else it's a rounded rectangle.
  std::vector<double> corners;
  NSWindow* shadow = nil;
  CALayer* shadow_layer = nil;
  CAShapeLayer* shadow_mask = nil;
};

Overlay g_overlays[] = {Overlay("command bar"), Overlay("icon picker")};

Overlay* OverlayAt(int index) {
  return index >= 0 && index < int(std::size(g_overlays)) && g_overlays[index].window
             ? &g_overlays[index]
             : nullptr;
}

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

// The zoom HUD's window and view, the browser it shows over, its distance from the top right of
// that window's content, and a count of its shows and hides, so one posted before another doesn't
// undo it.
Browser* g_hud_over;
NSWindow* g_hud;
NSView* g_hud_view;
bool g_hud_shown = false;
double g_hud_right;
double g_hud_top;
unsigned g_hud_changes = 0;

// The drop targets' window and view, and the browser they show over.
Browser* g_drop_targets_over;
NSWindow* g_drop_targets;
NSView* g_drop_targets_view;
bool g_drop_targets_shown = false;

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

// At the browser window's top right corner, where the toast draws itself.
void PlaceToast() {
  NSRect frame = g_toast_over->window.frame;
  NSSize size = g_toast.frame.size;
  [g_toast setFrameOrigin:NSMakePoint(NSMaxX(frame) - size.width, NSMaxY(frame) - size.height)];
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

// The overlay's shape filling |bounds|, in coordinates whose y goes up, as Core Graphics draws.
CGPathRef OverlayPath(const Overlay& overlay, CGRect bounds) {
  const size_t count = overlay.corners.size() / 3;
  if (count < 3) {
    return CGPathCreateWithRoundedRect(bounds, overlay.radius, overlay.radius, nullptr);
  }
  auto corner = [&](size_t index) {
    const double* at = &overlay.corners[3 * (index % count)];
    return CGPointMake(CGRectGetMinX(bounds) + at[0], CGRectGetMaxY(bounds) - at[1]);
  };
  CGMutablePathRef path = CGPathCreateMutable();
  // From the middle of the last side, so each corner rounds between its two sides.
  const CGPoint first = corner(0);
  const CGPoint last = corner(count - 1);
  CGPathMoveToPoint(path, nullptr, (first.x + last.x) / 2, (first.y + last.y) / 2);
  for (size_t index = 0; index < count; index++) {
    const CGPoint at = corner(index);
    const CGPoint next = corner(index + 1);
    CGPathAddArcToPoint(path, nullptr, at.x, at.y, next.x, next.y,
                        overlay.corners[3 * index + 2]);
  }
  CGPathCloseSubpath(path);
  return path;
}

// Under the UI's view, in the overlay's shape at the window's size, so nothing shows outside it.
void MaskOverlayBlur(Overlay& overlay) {
  const NSSize size = overlay.window.frame.size;
  if (overlay.corners.empty()) {
    const double radius = overlay.radius;
    NSImage* mask = [NSImage imageWithSize:NSMakeSize(2 * radius + 1, 2 * radius + 1)
                                   flipped:NO
                            drawingHandler:^BOOL(NSRect rect) {
                              [NSColor.blackColor setFill];
                              [[NSBezierPath bezierPathWithRoundedRect:rect
                                                               xRadius:radius
                                                               yRadius:radius] fill];
                              return YES;
                            }];
    mask.capInsets = NSEdgeInsetsMake(radius, radius, radius, radius);
    mask.resizingMode = NSImageResizingModeStretch;
    overlay.blur.maskImage = mask;
    return;
  }
  Overlay* shaped = &overlay;
  NSImage* mask = [NSImage imageWithSize:size
                                 flipped:NO
                          drawingHandler:^BOOL(NSRect rect) {
                            CGContextRef context = NSGraphicsContext.currentContext.CGContext;
                            CGPathRef path = OverlayPath(*shaped, rect);
                            CGContextAddPath(context, path);
                            CGContextSetFillColorWithColor(context, NSColor.blackColor.CGColor);
                            CGContextFillPath(context);
                            CGPathRelease(path);
                            return YES;
                          }];
  // Drawn at the window's size, but stretched rather than tiled if the view's differs.
  mask.resizingMode = NSImageResizingModeStretch;
  overlay.blur.maskImage = mask;
}

// Around the overlay by the shadow's reach, with the overlay's shape cut out of it, since what's
// under the translucent overlay shows through it.
void PlaceOverlayShadow(Overlay& overlay) {
  const NSRect frame =
      NSInsetRect(overlay.window.frame, -kOverlayShadowReach, -kOverlayShadowReach);
  if (NSEqualRects(frame, overlay.shadow.frame)) {
    return;
  }
  [overlay.shadow setFrame:frame display:NO];
  const CGRect bounds = CGRectMake(0, 0, NSWidth(frame), NSHeight(frame));
  CGPathRef shape =
      OverlayPath(overlay, CGRectInset(bounds, kOverlayShadowReach, kOverlayShadowReach));
  CGMutablePathRef outside = CGPathCreateMutable();
  CGPathAddRect(outside, nullptr, bounds);
  CGPathAddPath(outside, nullptr, shape);
  // At once, not animated to the new size.
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  overlay.shadow_layer.superlayer.frame = bounds;
  overlay.shadow_layer.frame = bounds;
  overlay.shadow_layer.shadowPath = shape;
  overlay.shadow_mask.frame = bounds;
  overlay.shadow_mask.path = outside;
  [CATransaction commit];
  CGPathRelease(outside);
  CGPathRelease(shape);
}

// Puts the overlays showing over |browser| above |window|, one of its child windows.
void RaiseOverlays(Browser* browser, NSWindow* window) {
  for (Overlay& overlay : g_overlays) {
    if (overlay.shown && overlay.over == browser) {
      [overlay.window orderWindow:NSWindowAbove relativeTo:window.windowNumber];
    }
  }
}

// 8 points below the find bar while it shows, where the HUD would cover it.
void PlaceHud() {
  Browser* browser = g_hud_over;
  NSRect content = [browser->window contentRectForFrameRect:browser->window.frame];
  NSSize size = g_hud.frame.size;
  const double top = browser->find_bar_shown
                         ? browser->find_bar_top + browser->find_bar.frame.size.height + 8
                         : g_hud_top;
  [g_hud setFrameOrigin:NSMakePoint(
                            NSMaxX(content) - browser->page_right - g_hud_right - size.width,
                            NSMaxY(content) - browser->page_top - top - size.height)];
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

void host_centered(double width, double height, double* x, double* y) {
  NSScreen* main = NSScreen.screens.firstObject;
  NSRect visible = WindowScreen().visibleFrame;
  *x = NSMidX(visible) - width / 2;
  // AppKit's y runs up from the bottom of the main display, the core's down from its top.
  *y = NSHeight(main.frame) - NSMidY(visible) - height / 2;
}

void* host_ui_view(void) {
  auto browser = std::make_unique<Browser>();
  browser->ui_view = [[HostUIView alloc] initWithFrame:NSZeroRect];
  g_browsers.push_back(std::move(browser));
  // Kept by its browser, and by the window the core puts it in.
  return (__bridge void*)g_browsers.back()->ui_view;
}

void host_show_window(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  NSView* view = browser->ui_view;
  NSWindow* window = view.window;
  browser->window = window;
  // Dia's window material (docs/ux-spec.md › Window material): what's behind the window, blurred,
  // under the UI, which tints it. AppKit swaps it for an opaque fill while the window is inactive,
  // and with Reduce Transparency or Increase Contrast.
  NSVisualEffectView* material = [[NSVisualEffectView alloc] initWithFrame:view.bounds];
  material.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  material.blendingMode = NSVisualEffectBlendingModeBehindWindow;
  // Dia's dark material, which AppKit doesn't name.
  material.material = (NSVisualEffectMaterial)29;
  material.state = NSVisualEffectStateFollowsWindowActiveState;
  material.emphasized = YES;
  material.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
  // Under the UI's view, since the core keeps only the UI's and the page's views in its own.
  [view addSubview:material positioned:NSWindowBelow relativeTo:nil];
  [window makeKeyAndOrderFront:nil];
  [NSApp activateIgnoringOtherApps:YES];

  // A click outside the command bar closes it first, so the click lands the way it would with
  // the bar closed. Otherwise it only makes the browser window key again: the page doesn't take
  // the click that activates its window.
  constexpr NSEventMask kMouseDown =
      NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  // The find bar stays open, and a click on its browser window takes the keyboard from it.
  static bool watching_clicks = false;
  if (!watching_clicks) {
    watching_clicks = true;
    [NSEvent addLocalMonitorForEventsMatchingMask:kMouseDown
                                          handler:^NSEvent*(NSEvent* event) {
                                            for (auto& each : g_browsers) {
                                              if (!event.window) {
                                                break;
                                              }
                                              if (event.window == each->window) {
                                                each->find_bar_keyboard = false;
                                                if (each->find_bar.keyWindow) {
                                                  [each->window makeKeyWindow];
                                                }
                                              } else if (event.window == each->find_bar) {
                                                each->find_bar_keyboard = true;
                                              }
                                            }
                                            for (int index = 0;
                                                 index < int(std::size(g_overlays)); index++) {
                                              if (event.window != g_overlays[index].window) {
                                                host_dismiss_overlay(index);
                                              }
                                            }
                                            if (event.window != g_site_controls) {
                                              DismissSiteControls();
                                            }
                                            return event;
                                          }];
  }
  // A window resizes from its top left corner as often as its bottom right, and AppKit moves a
  // child window only with its parent's bottom left.
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidResizeNotification
                                                  object:window
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                if (browser->find_bar_shown) {
                                                  PlaceFindBar(browser);
                                                }
                                                if (g_hud_shown && g_hud_over == browser) {
                                                  PlaceHud();
                                                }
                                                if (g_dialog_shown && g_dialog_over == browser) {
                                                  [g_dialog setFrame:window.frame display:YES];
                                                }
                                                if (g_toast_shown && g_toast_over == browser) {
                                                  PlaceToast();
                                                }
                                                if (g_site_controls_shown &&
                                                    g_site_controls_over == browser) {
                                                  PlaceSiteControls();
                                                }
                                              }];
}

void host_cascaded(void* ui_view, double* x, double* y, double* width, double* height) {
  Browser* browser = BrowserWithView(ui_view);
  NSWindow* window = browser ? browser->window : nil;
  if (!window) {
    return;
  }
  NSScreen* main = NSScreen.screens.firstObject;
  NSRect frame = window.frame;
  NSRect visible = (window.screen ?: WindowScreen()).visibleFrame;
  // Where AppKit puts the next window, so both title bars show. The zero point leaves this one be.
  NSPoint top_left = [window cascadeTopLeftFromPoint:NSZeroPoint];
  if (top_left.x + NSWidth(frame) > NSMaxX(visible)) {
    top_left.x = NSMinX(visible);
  }
  if (top_left.y - NSHeight(frame) < NSMinY(visible)) {
    top_left.y = NSMaxY(visible);
  }
  *x = top_left.x;
  *y = NSHeight(main.frame) - top_left.y;
  *width = NSWidth(frame);
  *height = NSHeight(frame);
}

void host_set_window_title(void* ui_view, const char* title) {
  Browser* browser = BrowserWithView(ui_view);
  if (browser && browser->window) {
    browser->window.title = @(title);
  }
}

void host_watch_key_window(void (*changed)(void* ui_view)) {
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidBecomeKeyNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification* notification) {
                                                if (Browser* browser =
                                                        BrowserOf(notification.object)) {
                                                  changed((__bridge void*)browser->ui_view);
                                                }
                                              }];
}

void host_forget_window(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  for (int index = 0; index < int(std::size(g_overlays)); index++) {
    if (g_overlays[index].shown && g_overlays[index].over == browser) {
      host_dismiss_overlay(index);
    }
  }
  if (g_dialog_shown && g_dialog_over == browser) {
    host_hide_dialog();
  }
  if (g_toast_shown && g_toast_over == browser) {
    host_hide_toast();
  }
  if (g_tooltip_over == browser) {
    host_hide_tooltip(0);
  }
  if (g_hud_shown && g_hud_over == browser) {
    host_hide_hud(0);
  }
  if (g_drop_targets_shown && g_drop_targets_over == browser) {
    host_hide_drop_targets();
  }
  if (browser->reveal_monitor) {
    [NSEvent removeMonitor:browser->reveal_monitor];
    browser->reveal_monitor = nil;
  }
  // The entry stays, as the observers made for it hold it and the driver counts windows by it,
  // but nothing finds it by its window or view again, and they go. The app closes the find bar's
  // and the reveal's windows.
  browser->find_bar_shown = false;
  browser->window = nil;
  browser->ui_view = nil;
  browser->find_bar = nil;
  browser->find_bar_view = nil;
  browser->reveal = nil;
  browser->reveal_view = nil;
}

void host_add_overlay(int index, void* overlay_view, double radius, void (*dismissed)(void)) {
  if (index < 0 || index >= int(std::size(g_overlays))) {
    return;
  }
  Overlay& overlay = g_overlays[index];
  overlay.view = (__bridge NSView*)overlay_view;
  overlay.dismissed = dismissed;
  overlay.radius = radius;
  overlay.window = overlay.view.window;
  MakeOverlay(overlay.window);
  // The UI draws the overlay's shape, translucent, over the blur, and the shadow is its own
  // window. A titled window would draw its edge at the window's full size.
  overlay.window.styleMask = NSWindowStyleMaskBorderless;
  overlay.window.hasShadow = NO;
  overlay.blur = [[HostBlurView alloc] initWithFrame:overlay.view.frame];
  overlay.blur.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  overlay.blur.blendingMode = NSVisualEffectBlendingModeBehindWindow;
  overlay.blur.material = NSVisualEffectMaterialUnderWindowBackground;
  overlay.blur.state = NSVisualEffectStateActive;
  MaskOverlayBlur(overlay);
  [overlay.view.superview addSubview:overlay.blur
                          positioned:NSWindowBelow
                          relativeTo:overlay.view];
  overlay.shadow = [[NSWindow alloc] initWithContentRect:NSZeroRect
                                               styleMask:NSWindowStyleMaskBorderless
                                                 backing:NSBackingStoreBuffered
                                                   defer:YES];
  overlay.shadow.releasedWhenClosed = NO;
  overlay.shadow.opaque = NO;
  overlay.shadow.backgroundColor = NSColor.clearColor;
  overlay.shadow.hasShadow = NO;
  overlay.shadow.ignoresMouseEvents = YES;
  NSView* shadow_view = overlay.shadow.contentView;
  shadow_view.wantsLayer = YES;
  // The mask is on a layer holding the shadow's, since a layer's own mask doesn't cut its shadow.
  CALayer* holder = [CALayer layer];
  overlay.shadow_mask = [CAShapeLayer layer];
  overlay.shadow_mask.fillRule = kCAFillRuleEvenOdd;
  holder.mask = overlay.shadow_mask;
  overlay.shadow_layer = [CALayer layer];
  overlay.shadow_layer.shadowColor = NSColor.blackColor.CGColor;
  overlay.shadow_layer.shadowOpacity = kOverlayShadowOpacity;
  overlay.shadow_layer.shadowRadius = kOverlayShadowRadius;
  overlay.shadow_layer.shadowOffset = CGSizeZero;
  [holder addSublayer:overlay.shadow_layer];
  [shadow_view.layer addSublayer:holder];
  // The command bar grows and shrinks from its top with the rows. Until the frame at the new size
  // lands, the last one stays as drawn at the top; scaled to the window, it stretched or squashed
  // them.
  overlay.view.layer.contentsGravity = kCAGravityTopLeft;
  // An overlay closes when the user leaves it: a click outside it, another window taking the
  // keyboard, or switching apps.
  Overlay* each = &overlay;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    host_dismiss_overlay(index);
                  }];
  // Chromium makes its window key by itself, such as when a page commits and its view replaces
  // the one that had focus. That isn't the user leaving, so the overlay takes the keyboard back.
  [center addObserverForName:NSWindowDidBecomeKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    NSWindow* window = notification.object;
                    Browser* browser = BrowserOf(window);
                    if (each->shown && window != each->window) {
                      if (browser == each->over) {
                        [each->window makeKeyWindow];
                      } else {
                        host_dismiss_overlay(index);
                      }
                    } else if (index == 0 && !g_dialog_shown && browser &&
                               browser->find_bar_shown && browser->find_bar_keyboard &&
                               std::none_of(std::begin(g_overlays), std::end(g_overlays),
                                            [](const Overlay& other) { return other.shown; })) {
                      // The same for the find bar, which the user left only by clicking.
                      [browser->find_bar makeKeyWindow];
                    }
                  }];
}

void host_set_overlay_shape(int index, const double* corners, size_t count) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  overlay->corners.assign(corners, corners + 3 * count);
  MaskOverlayBlur(*overlay);
  // Drawn again at its next show.
  [overlay->shadow setFrame:NSZeroRect display:NO];
}

void host_show_overlay(int index,
                       void* ui_view,
                       bool centered,
                       double left,
                       double top,
                       double width) {
  Overlay* overlay = OverlayAt(index);
  Browser* browser = BrowserWithView(ui_view);
  if (!overlay || !browser || !browser->window) {
    return;
  }
  if (overlay->shown && overlay->over == browser) {
    [overlay->window makeKeyAndOrderFront:nil];
    return;
  }
  // One overlay at a time, as one has the keyboard.
  for (int other = 0; other < int(std::size(g_overlays)); other++) {
    if (other != index) {
      host_dismiss_overlay(other);
    }
  }
  host_hide_overlay(index);
  overlay->shown = true;
  overlay->over = browser;
  // The window still holds what it drew when it last closed, such as the previous query. It goes
  // on screen once the UI has drawn it as it is now, which can't happen while the UI is still
  // handling the shortcut that opened it.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!overlay->shown || overlay->over != browser) {
      return;
    }
    NSWindow* window = browser->window;
    NSRect content = [window contentRectForFrameRect:window.frame];
    const double height = NSHeight(overlay->window.frame);
    NSRect frame;
    if (centered) {
      // Centred, with its top 0.322 of the way down, where Arc's is: y 305.5 in a window 949 tall.
      frame = NSMakeRect(NSMidX(content) - width / 2,
                         NSMaxY(content) - NSHeight(content) * 0.322 - height, width, height);
    } else {
      frame = NSMakeRect(NSMinX(content) + left, NSMaxY(content) - top - height,
                         fmin(width, NSWidth(content) - left - 8), height);
    }
    // At its new size before it draws, so it draws once.
    [overlay->window setFrame:frame display:NO];
    [overlay->view.layer setNeedsDisplay];
    [overlay->view.layer displayIfNeeded];
    PlaceOverlayShadow(*overlay);
    overlay->shadow.alphaValue = 1;
    if (overlay->shadow.parentWindow != window) {
      [overlay->shadow.parentWindow removeChildWindow:overlay->shadow];
      [window addChildWindow:overlay->shadow ordered:NSWindowAbove];
    }
    [window addChildWindow:overlay->window ordered:NSWindowAbove];
    [overlay->window makeKeyAndOrderFront:nil];
    // check.sh reads these lines.
    fprintf(stderr, "browser: %s at %g %g %g %g\n", overlay->name,
            NSMinX(frame) - NSMinX(content), NSMaxY(content) - NSMaxY(frame), NSWidth(frame),
            NSHeight(frame));
    fprintf(stderr, "browser: %s shown\n", overlay->name);
  });
}

void host_set_overlay_height(int index, double height) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  // After the UI's update that asked, since the overlay draws at its new size here.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (NSHeight(overlay->window.frame) == height) {
      return;
    }
    NSRect frame = overlay->window.frame;
    // Keeping its top edge, so the rows grow down from the input.
    [overlay->window
        setFrame:NSMakeRect(NSMinX(frame), NSMaxY(frame) - height, NSWidth(frame), height)
         display:NO];
    [overlay->view.layer setNeedsDisplay];
    [overlay->view.layer displayIfNeeded];
    PlaceOverlayShadow(*overlay);
    fprintf(stderr, "browser: %s height %g\n", overlay->name, height);
  });
}

void host_hide_overlay(int index) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || !overlay->shown) {
    return;
  }
  overlay->shown = false;
  Browser* browser = overlay->over;
  [browser->window removeChildWindow:overlay->window];
  [overlay->window orderOut:nil];
  overlay->shadow.alphaValue = 0;
  [browser->find_bar_shown && browser->find_bar_keyboard ? browser->find_bar : browser->window
      makeKeyWindow];
  fprintf(stderr, "browser: %s hidden\n", overlay->name);
}

void host_dismiss_overlay(int index) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || !overlay->shown) {
    return;
  }
  host_hide_overlay(index);
  // Posted: the UI may be handling the event that closed the overlay.
  if (void (*dismissed)(void) = overlay->dismissed) {
    dispatch_async(dispatch_get_main_queue(), ^{
      dismissed();
    });
  }
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
  if (browser->find_bar_shown) {
    // Once it's on screen. Until then, it takes the keyboard as it goes on.
    if (keyboard && find_bar.visible) {
      [find_bar makeKeyAndOrderFront:nil];
    }
    return;
  }
  browser->find_bar_shown = true;
  // As the command bar does: it goes on screen once it has drawn what it shows now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!browser->find_bar_shown) {
      return;
    }
    [browser->find_bar_view.layer setNeedsDisplay];
    [browser->find_bar_view.layer displayIfNeeded];
    [find_bar invalidateShadow];
    PlaceFindBar(browser);
    [browser->window addChildWindow:find_bar ordered:NSWindowAbove];
    if (browser->find_bar_keyboard) {
      // From the command bar too, which closes.
      [find_bar makeKeyAndOrderFront:nil];
    } else {
      RaiseOverlays(browser, find_bar);
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
  if (g_hud_shown && g_hud_over == browser) {
    PlaceHud();
  }
  fprintf(stderr, "browser: find bar hidden\n");
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
  if (g_hud_shown && g_hud_over == browser) {
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
    [g_hud invalidateShadow];
    PlaceHud();
    // Stops a fade.
    [NSAnimationContext
        runAnimationGroup:^(NSAnimationContext* context) {
          context.duration = 0;
          g_hud.animator.alphaValue = 1;
        }
        completionHandler:nil];
    if (g_hud.parentWindow != browser->window) {
      [g_hud.parentWindow removeChildWindow:g_hud];
      [browser->window addChildWindow:g_hud ordered:NSWindowAbove];
    }
    if (browser->find_bar_shown) {
      [g_hud orderWindow:NSWindowAbove relativeTo:browser->find_bar.windowNumber];
    }
    RaiseOverlays(browser, g_hud);
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
        [g_hud.parentWindow removeChildWindow:g_hud];
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
    [window addChildWindow:g_drop_targets ordered:NSWindowAbove];
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
    [browser->window addChildWindow:g_dialog ordered:NSWindowAbove];
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
  [browser->find_bar_shown && browser->find_bar_keyboard ? browser->find_bar : browser->window
      makeKeyWindow];
}

void host_add_site_controls(void* view, void (*dismissed)(void)) {
  g_site_controls_view = (__bridge NSView*)view;
  g_site_controls = g_site_controls_view.window;
  g_site_controls_dismissed = dismissed;
  MakeOverlay(g_site_controls);
  // The UI draws the panel's rounded shape, which a borderless window's shadow takes, as the find
  // bar's does.
  g_site_controls.styleMask = NSWindowStyleMaskBorderless;
  g_site_controls.hasShadow = YES;
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
    [g_site_controls setFrame:NSMakeRect(0, 0, width, height) display:NO];
    PlaceSiteControls();
    [g_site_controls_view.layer setNeedsDisplay];
    [g_site_controls_view.layer displayIfNeeded];
    [g_site_controls invalidateShadow];
    [window addChildWindow:g_site_controls ordered:NSWindowAbove];
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
    [g_site_controls invalidateShadow];
    fprintf(stderr, "browser: site controls height %g\n", height);
  });
}

void host_site_controls_drew(void) {
  // Posted, so the shadow is taken from the frame the UI is drawing now.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (g_site_controls_shown) {
      [g_site_controls invalidateShadow];
    }
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
    [browser->find_bar_shown && browser->find_bar_keyboard ? browser->find_bar : browser->window
        makeKeyWindow];
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
  fprintf(stderr, "browser: site controls hidden\n");
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
    [browser->window addChildWindow:g_toast ordered:NSWindowAbove];
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

// The window's alpha, which AppKit animates, so the fade wakes nothing of ours but its end.
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
    [g_tooltip_view.layer setNeedsDisplay];
    [g_tooltip_view.layer displayIfNeeded];
    // Still fading out over this window, it fades back in from where it is.
    if (g_tooltip.parentWindow != browser->window) {
      [g_tooltip.parentWindow removeChildWindow:g_tooltip];
      g_tooltip.alphaValue = 0;
      [browser->window addChildWindow:g_tooltip ordered:NSWindowAbove];
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

void host_show_window_buttons(void* ui_view, bool shown) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  for (NSWindowButton button :
       {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    [browser->window standardWindowButton:button].hidden = !shown;
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

void host_take_drops(bool (*dragged)(void* ui_view, int phase, double x, double y,
                                     const host_dragged* items, size_t count)) {
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

bool host_trash(const char* path) {
  NSURL* file = [NSURL fileURLWithPath:@(path)];
  return [NSFileManager.defaultManager trashItemAtURL:file resultingItemURL:nil error:nil] ||
         ![file checkResourceIsReachableAndReturnError:nil];
}

bool host_drag_file(const char* path, void (*ended)(bool taken)) {
  // Kept until the next drag, as the session doesn't keep its source.
  static HostFileDragSource* source;
  NSString* file = @(path);
  NSEvent* event = NSApp.currentEvent;
  Browser* browser = BrowserOf(event.window);
  if (!browser || event.type != NSEventTypeLeftMouseDragged ||
      ![NSFileManager.defaultManager fileExistsAtPath:file]) {
    return false;
  }
  NSView* view = browser->ui_view;
  NSDraggingItem* item =
      [[NSDraggingItem alloc] initWithPasteboardWriter:[NSURL fileURLWithPath:file]];
  // The file's icon under the pointer, 32 points square, as Finder drags one.
  NSPoint point = [view convertPoint:event.locationInWindow fromView:nil];
  [item setDraggingFrame:NSMakeRect(point.x - 16, point.y - 16, 32, 32)
                contents:[NSWorkspace.sharedWorkspace iconForFile:file]];
  source = [[HostFileDragSource alloc] init];
  source.ended = ended;
  return [view beginDraggingSessionWithItems:@[ item ] event:event source:source] != nil;
}

// |image| as a PNG, or nil.
static NSData* PNGOf(CGImageRef image) {
  NSMutableData* png = [NSMutableData data];
  CGImageDestinationRef destination = CGImageDestinationCreateWithData(
      (__bridge CFMutableDataRef)png, CFSTR("public.png"), 1, nullptr);
  if (!destination) {
    return nil;
  }
  CGImageDestinationAddImage(destination, image, nullptr);
  bool written = CGImageDestinationFinalize(destination);
  CFRelease(destination);
  return written ? png : nil;
}

void host_file_preview(const char* path, uint64_t id,
                       void (*done)(uint64_t id, const uint8_t* png, size_t size, double width,
                                    double height, bool image)) {
  NSString* file = @(path);
  // A photo's thumbnail reads and decodes its file, which can take a while for a large one.
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
    NSData* thumbnail = nil;
    size_t width = 0, height = 0;
    CGImageSourceRef source =
        CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:file], nullptr);
    if (source) {
      NSDictionary* options = @{
        (__bridge NSString*)kCGImageSourceCreateThumbnailFromImageAlways : @YES,
        (__bridge NSString*)kCGImageSourceCreateThumbnailWithTransform : @YES,
        (__bridge NSString*)kCGImageSourceThumbnailMaxPixelSize : @96,
      };
      CGImageRef image = CGImageSourceGetCount(source) > 0
                             ? CGImageSourceCreateThumbnailAtIndex(
                                   source, 0, (__bridge CFDictionaryRef)options)
                             : nullptr;
      if (image) {
        thumbnail = PNGOf(image);
        width = CGImageGetWidth(image);
        height = CGImageGetHeight(image);
        CGImageRelease(image);
      }
      CFRelease(source);
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      if (thumbnail) {
        done(id, static_cast<const uint8_t*>(thumbnail.bytes), thumbnail.length, width, height,
             true);
        return;
      }
      if (![NSFileManager.defaultManager fileExistsAtPath:file]) {
        done(id, nullptr, 0, 0, 0, false);
        return;
      }
      // The icon at 26 points on a 2× display, drawn as Finder draws it.
      const NSInteger side = 52;
      NSBitmapImageRep* bitmap =
          [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:nullptr
                                                  pixelsWide:side
                                                  pixelsHigh:side
                                               bitsPerSample:8
                                             samplesPerPixel:4
                                                    hasAlpha:YES
                                                    isPlanar:NO
                                              colorSpaceName:NSDeviceRGBColorSpace
                                                 bytesPerRow:0
                                                bitsPerPixel:0];
      [NSGraphicsContext saveGraphicsState];
      NSGraphicsContext.currentContext =
          [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
      [[NSWorkspace.sharedWorkspace iconForFile:file] drawInRect:NSMakeRect(0, 0, side, side)];
      [NSGraphicsContext restoreGraphicsState];
      NSData* png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
      done(id, static_cast<const uint8_t*>(png.bytes), png.length, side, side, false);
    });
  });
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

void host_press_window_button(void* ui_view, int button) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  NSWindow* window = browser->window;
  // After the UI's update that asked, since closing or going full screen calls back into it.
  dispatch_async(dispatch_get_main_queue(), ^{
    switch (button) {
      case 0:
        [window performClose:nil];
        break;
      case 1:
        [window miniaturize:nil];
        break;
      default:
        // What a click on the green button does.
        [window toggleFullScreen:nil];
    }
  });
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
    [browser->window addChildWindow:browser->reveal ordered:NSWindowAbove];
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

int host_menu(void* ui_view,
              const host_menu_item* items,
              size_t count,
              double x,
              double y,
              int in,
              int over) {
  if (in == HOST_IN_SETTINGS) {
    return ShowMenu(items, count, x, y, g_settings_view, @[ g_settings_view ], over);
  }
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return -1;
  }
  NSView* panel = in == HOST_IN_REVEAL ? browser->reveal_view : nil;
  return ShowMenu(items, count, x, y, panel ?: browser->window.contentView,
                  panel ? @[ panel ] : browser->ui_view.subviews, over);
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
      Browser* front = FrontBrowser();
      NSWindow* window = front ? front->window : nil;
      NSRect visible = (window.screen ?: WindowScreen()).visibleFrame;
      NSSize size = g_settings.frame.size;
      double x = NSMidX(window.frame) - size.width / 2;
      double y = NSMidY(window.frame) - size.height / 2;
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
  if (!key || BrowserOf(key) || key == g_settings || key.parentWindow ||
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

void* host_browser_window(size_t index) {
  return index < g_browsers.size() ? (__bridge void*)g_browsers[index]->window : nullptr;
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
