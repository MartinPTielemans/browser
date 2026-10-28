#import <Cocoa/Cocoa.h>
#import <ImageIO/ImageIO.h>
#import <QuartzCore/QuartzCore.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#import <objc/runtime.h>

#include <dlfcn.h>
#include <sys/sysctl.h>
#include <sys/time.h>
#include <unistd.h>

#include <algorithm>
#include <initializer_list>
#include <iterator>
#include <memory>
#include <string>
#include <vector>

#include "../host.h"

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

// Gives the service the user picks in the share menu the page's title, as a mail's subject.
@interface HostShareDelegate : NSObject <NSSharingServicePickerDelegate>
@property(copy) NSString* title;
@end

@implementation HostShareDelegate
- (void)sharingServicePicker:(NSSharingServicePicker*)picker
     didChooseSharingService:(NSSharingService*)service {
  service.subject = self.title;
}
@end

namespace {

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

  // The offer to save a password: its window and view, its distance from the top right of the
  // window's content, and whether it has the keyboard, or gets it back, as the find bar does.
  NSWindow* offer;
  NSView* offer_view;
  bool offer_shown = false;
  double offer_right = 0;
  double offer_top = 0;
  bool offer_keyboard = false;

  // How far the focused page's top right corner is inside the card's, in a split view, which the
  // find bar, the offer and the HUD keep to.
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
// An overlay that grows in, as a system popover does, is done in 0.2 s, mostly in the first 0.1
// (Arc 1.170 rec at 2×, the Space's icon picker: 0.57 of its size at 55 ms, 0.83 at 105 ms).
constexpr double kOverlayGrow = 0.2;
constexpr double kOverlayGrowFade = 0.1;
// Arc's theme picker shrinks to its arrow and fades as it closes, gone in 0.11 s.
constexpr double kOverlayShrink = 0.11;
constexpr float kOverlayShadowOpacity = 0.23;

// A window of the UI's over a browser window's page, translucent over a blur, with a shadow: the
// command bar, the Space's icon or theme picker, or the Site Controls.
struct Overlay {
  explicit Overlay(const char* name) : name(name) {}
  Overlay(const char* name, float shadow_opacity, double shadow_radius, double shadow_drop)
      : name(name),
        shadow_opacity(shadow_opacity),
        shadow_radius(shadow_radius),
        shadow_drop(shadow_drop) {}
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
  // Where host_show_overlay last put it.
  bool centered = false;
  double left = 0;
  double top = 0;
  double width = 0;
  // From host_set_overlay_shape: x, y and radius for each corner, else it's a rounded rectangle.
  std::vector<double> corners;
  NSWindow* shadow = nil;
  CALayer* shadow_layer = nil;
  CAShapeLayer* shadow_mask = nil;
  // From host_set_overlay_growth: whether it grows in as it shows, and from where, in points from
  // its top left.
  bool grows = false;
  double grow_x = 0;
  double grow_y = 0;
  float shadow_opacity = kOverlayShadowOpacity;
  double shadow_radius = kOverlayShadowRadius;
  // How far down it falls.
  double shadow_drop = 0;
  // Past this, three times the blur and its drop, the shadow is nothing.
  double reach() const { return 3 * shadow_radius + shadow_drop; }
  // From host_set_overlay_shrink: whether it shrinks back there as it hides.
  bool shrinks = false;
};

Overlay g_overlays[] = {Overlay("command bar"), Overlay("icon picker"), Overlay("theme picker")};

// Arc's Site Controls' shadow, fitted over a dark page: darker below, as it falls 4 pt. It's an
// overlay for its blur and shadow alone, outside g_overlays, so the tooltip shows over it.
Overlay g_site_controls_overlay("site controls", 0.24, 16, 4);

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

// The menu or alert showing, for host_choose and host_dismiss_menu.
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

// As the tooltip's: where the UI asked, inside the browser window, and its window wider than the
// plate it holds.
void PlaceLinkTarget() {
  NSWindow* window = g_link_over->window;
  NSRect content = [window contentRectForFrameRect:window.frame];
  NSSize size = g_link.frame.size;
  [g_link setFrameOrigin:NSMakePoint(NSMinX(content) + g_link_left,
                                     NSMaxY(content) - g_link_top - size.height)];
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
  const double reach = overlay.reach();
  const NSRect frame = NSInsetRect(overlay.window.frame, -reach, -reach);
  if (NSEqualRects(frame, overlay.shadow.frame)) {
    return;
  }
  [overlay.shadow setFrame:frame display:NO];
  const CGRect bounds = CGRectMake(0, 0, NSWidth(frame), NSHeight(frame));
  CGPathRef shape = OverlayPath(overlay, CGRectInset(bounds, reach, reach));
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

// Puts the blur under the overlay's view and makes its shadow's window.
void AddBlurAndShadow(Overlay& overlay) {
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
  overlay.shadow_layer.shadowOpacity = overlay.shadow_opacity;
  overlay.shadow_layer.shadowRadius = overlay.shadow_radius;
  // The layer's y goes up.
  overlay.shadow_layer.shadowOffset = CGSizeMake(0, -overlay.shadow_drop);
  [holder addSublayer:overlay.shadow_layer];
  [shadow_view.layer addSublayer:holder];
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
                         ? browser->find_bar_top + browser->find_bar.frame.size.height + 8 -
                               g_hud_margin
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

// Keys typed right after a shortcut that opens the command bar or the find bar belong in its field
// (Ardia P§7), but the bar takes the keyboard only once it has drawn, a main-queue turn later, and
// a shortcut the page gets first, such as ⌘L, comes back after the keys behind it went to the page.
// So from the shortcut until the bar has the keyboard, keys are held and then given to it in order,
// a composition whole. A hold lasts kHoldKeys at most, which is the page keeping the shortcut.
NSMutableArray<NSEvent*>* g_held_keys;
bool g_holding_keys = false;
// The bar the keys wait for, or nil for whichever opens.
NSWindow* g_keys_for;
// Where the last hold's keys went, and when: keys typed before then that were still queued go there
// too.
NSWindow* g_keys_to;
NSTimeInterval g_keys_released;
// Counts holds, so a hold's time running out doesn't end a later one.
unsigned g_hold = 0;
constexpr double kHoldKeys = 0.4;
// The shortcut a hold is for, which is never held: Safari's engine sends a shortcut its page didn't
// keep through the app again, which runs it then, so holding it would hold it again each time.
NSTimeInterval g_holding_for_time = -1;
unsigned short g_holding_for_key = 0;

bool HoldsFor(NSEvent* event) {
  return event.type == NSEventTypeKeyDown && event.timestamp == g_holding_for_time &&
         event.keyCode == g_holding_for_key;
}

// |event| as typed in |window|, so it goes there whether AppKit hands a key to the key window or to
// the window the event names.
NSEvent* Retarget(NSEvent* event, NSWindow* window) {
  if (!window || event.windowNumber == window.windowNumber) {
    return event;
  }
  const bool flags = event.type == NSEventTypeFlagsChanged;
  return [NSEvent keyEventWithType:event.type
                          location:NSZeroPoint
                     modifierFlags:event.modifierFlags
                         timestamp:event.timestamp
                      windowNumber:window.windowNumber
                           context:nil
                        characters:flags ? @"" : event.characters
       charactersIgnoringModifiers:flags ? @"" : event.charactersIgnoringModifiers
                         isARepeat:flags ? NO : event.ARepeat
                           keyCode:event.keyCode];
}

// Gives the keys held, and those typed until now, to the key window, now that |from| has the
// keyboard or won't take it, unless they wait for another bar. The held ones are posted, since a
// GPUI window drops a key that comes while it's handling another event, such as the shortcut.
void ReleaseKeys(NSWindow* from) {
  if (from && g_keys_for && from != g_keys_for) {
    return;
  }
  NSWindow* to = NSApp.keyWindow;
  g_keys_to = to;
  g_keys_released = NSProcessInfo.processInfo.systemUptime;
  if (!g_holding_keys) {
    return;
  }
  g_holding_keys = false;
  g_keys_for = nil;
  ++g_hold;
  if (g_held_keys.count) {
    // check.sh reads this line.
    fprintf(stderr, "browser: held %lu key events for %s\n", (unsigned long)g_held_keys.count,
            to ? NSStringFromClass(to.class).UTF8String : "none");
  }
  // At the queue's start, last first, so they come in the order typed and before any typed since.
  for (NSEvent* event in g_held_keys.reverseObjectEnumerator) {
    [NSApp postEvent:Retarget(event, to) atStart:YES];
  }
  [g_held_keys removeAllObjects];
}

// Holds the keys typed from now until |window|, or if nil whichever bar opens, has the keyboard.
void HoldKeys(NSWindow* window) {
  if (!g_held_keys) {
    g_held_keys = [NSMutableArray array];
    constexpr NSEventMask kKeys =
        NSEventMaskKeyDown | NSEventMaskKeyUp | NSEventMaskFlagsChanged;
    [NSEvent addLocalMonitorForEventsMatchingMask:kKeys
                                          handler:^NSEvent*(NSEvent* event) {
                                            if (g_holding_keys && !HoldsFor(event)) {
                                              [g_held_keys addObject:event];
                                              return nil;
                                            }
                                            if (g_keys_to &&
                                                event.timestamp <= g_keys_released) {
                                              return Retarget(event, g_keys_to);
                                            }
                                            g_keys_to = nil;
                                            return event;
                                          }];
  }
  g_holding_keys = true;
  g_keys_for = window;
  // The key being handled is the shortcut that holds the keys after it.
  NSEvent* shortcut = NSApp.currentEvent;
  const bool key = shortcut.type == NSEventTypeKeyDown;
  g_holding_for_time = key ? shortcut.timestamp : -1;
  g_holding_for_key = key ? shortcut.keyCode : 0;
  const unsigned hold = ++g_hold;
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(kHoldKeys * NSEC_PER_SEC)),
                 dispatch_get_main_queue(), ^{
                   if (g_hold == hold) {
                     ReleaseKeys(nil);
                   }
                 });
}

}  // namespace

namespace {

// Dia's window material (docs/ux-spec.md › Window material): what's behind the window, blurred,
// under the UI, which tints it. AppKit swaps it for an opaque fill while the window is inactive,
// and with Reduce Transparency or Increase Contrast.
NSVisualEffectView* WindowMaterial(NSRect frame) {
  NSVisualEffectView* material = [[NSVisualEffectView alloc] initWithFrame:frame];
  material.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  material.blendingMode = NSVisualEffectBlendingModeBehindWindow;
  // Dia's dark material, which AppKit doesn't name.
  material.material = (NSVisualEffectMaterial)29;
  material.state = NSVisualEffectStateFollowsWindowActiveState;
  material.emphasized = YES;
  material.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
  return material;
}

// What the first window looks like until its UI has drawn, put on screen before the browser
// starts, and the sidebar's picture in it, which the browser window takes over.
NSWindow* g_launch_window;
NSImageView* g_launch_picture;

// The picture of a |width| by |height| window's sidebar, named for the size, as a window opens at
// another size than the one it quit at.
NSString* LaunchPicturePath(const char* profile_dir, double width, double height) {
  return [NSString stringWithFormat:@"%s/launch-picture-%.0fx%.0f.png", profile_dir, width, height];
}

}  // namespace

void host_show_launch_window(const char* profile_dir, double x, double y, double width,
                             double height) {
  // The core's y runs down from the main display's top, AppKit's up from its bottom.
  const NSRect frame =
      NSMakeRect(x, NSHeight(NSScreen.screens.firstObject.frame) - y - height, width, height);
  NSWindow* window = [[NSWindow alloc]
      initWithContentRect:frame
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable |
                          NSWindowStyleMaskFullSizeContentView
                  backing:NSBackingStoreBuffered
                    defer:NO];
  window.releasedWhenClosed = NO;
  window.titlebarAppearsTransparent = YES;
  window.titleVisibility = NSWindowTitleHidden;
  window.ignoresMouseEvents = YES;
  // The picture has the window's buttons, where ilo puts them.
  for (NSWindowButton button : {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    [window standardWindowButton:button].hidden = YES;
  }
  // The material's colour rather than the material: its blur slows the browser's start by a
  // third, and the window is gone by the time the material could show through.
  window.backgroundColor = [NSColor colorWithWhite:0.15 alpha:1];
  NSView* content = window.contentView;
  NSImage* picture = [[NSImage alloc] initWithContentsOfFile:LaunchPicturePath(profile_dir, width, height)];
  NSImageRep* pixels = picture.representations.firstObject;
  const CGFloat scale = window.backingScaleFactor;
  // A picture taken on a screen of another scale is left out.
  if (pixels && pixels.pixelsHigh == lround(height * scale)) {
    if ([pixels isKindOfClass:NSBitmapImageRep.class]) {
      // The sidebar's right edge, halfway down, is the material with nothing on it.
      NSColor* edge = [(NSBitmapImageRep*)pixels colorAtX:pixels.pixelsWide - 1 y:pixels.pixelsHigh / 2];
      window.backgroundColor = [edge colorWithAlphaComponent:1];
    }
    picture.size = NSMakeSize(pixels.pixelsWide / scale, height);
    NSImageView* shown = [NSImageView imageViewWithImage:picture];
    shown.frame = NSMakeRect(0, 0, picture.size.width, height);
    shown.imageScaling = NSImageScaleNone;
    [content addSubview:shown];
    g_launch_picture = shown;
    // check.sh reads this line.
    fprintf(stderr, "browser: launch picture shown\n");
  }
  [window makeKeyAndOrderFront:nil];
  [NSApp activateIgnoringOtherApps:YES];
  // On screen now, as Chromium's startup holds the main thread for a while before its run loop.
  [CATransaction flush];
  g_launch_window = window;
}

void host_close_launch_window(void) {
  [g_launch_window orderOut:nil];
  g_launch_window = nil;
  [g_launch_picture removeFromSuperview];
  g_launch_picture = nil;
  // On screen now, before what the main thread does next holds it.
  [CATransaction flush];
}

void host_watch_finished_launching(void (*finished)(void)) {
  // AppKit posts it once.
  [NSNotificationCenter.defaultCenter
      addObserverForName:NSApplicationDidFinishLaunchingNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification*) {
                // A plain launch (Dock, Finder, `open -a`) carried no links to wait for.
                NSAppleEventDescriptor* launch =
                    NSAppleEventManager.sharedAppleEventManager.currentAppleEvent;
                if (launch.eventClass == kCoreEventClass && launch.eventID == kAEOpenApplication) {
                  finished();
                  return;
                }
                // After AppController's own handler, which hands the links to the core, as the
                // order observers hear a notification in isn't promised.
                dispatch_async(dispatch_get_main_queue(), ^{
                  finished();
                });
              }];
}

bool host_save_launch_picture(void* ui_view, const char* profile_dir, double width) {
  NSWindow* window = ((__bridge NSView*)ui_view).window;
  const NSSize size = window.frame.size;
  NSString* path = LaunchPicturePath(profile_dir, size.width, size.height);
  NSFileManager* files = NSFileManager.defaultManager;
  NSString* dir = [NSString stringWithUTF8String:profile_dir];
  for (NSString* name in [files contentsOfDirectoryAtPath:dir error:nil]) {
    if ([name hasPrefix:@"launch-picture-"]) {
      [files removeItemAtPath:[dir stringByAppendingPathComponent:name] error:nil];
    }
  }
  if (!window.visible || (window.styleMask & NSWindowStyleMaskFullScreen) || width < 1) {
    return false;
  }
  // Only the sidebar: the page could be anything, and shouldn't be kept on disk.
  // The SDK no longer declares it, though macOS keeps it, and it reads the app's own windows
  // without the Screen Recording permission that ScreenCaptureKit asks for.
  using CreateImage = CGImageRef (*)(CGRect, CGWindowListOption, CGWindowID, CGWindowImageOption);
  static const auto create_image =
      reinterpret_cast<CreateImage>(dlsym(RTLD_DEFAULT, "CGWindowListCreateImage"));
  CGImageRef whole =
      create_image ? create_image(CGRectNull, kCGWindowListOptionIncludingWindow,
                                  (CGWindowID)window.windowNumber, kCGWindowImageBoundsIgnoreFraming)
                   : nullptr;
  if (!whole) {
    return false;
  }
  const CGFloat scale = (CGFloat)CGImageGetWidth(whole) / size.width;
  CGImageRef sidebar = CGImageCreateWithImageInRect(
      whole, CGRectMake(0, 0, round(width * scale), CGImageGetHeight(whole)));
  CGImageRelease(whole);
  if (!sidebar) {
    return false;
  }
  // Encoded off the main thread, which the user is about to use elsewhere or quit.
  NSURL* url = [NSURL fileURLWithPath:path];
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
    CGImageDestinationRef png =
        CGImageDestinationCreateWithURL((__bridge CFURLRef)url, CFSTR("public.png"), 1, nullptr);
    if (png) {
      CGImageDestinationAddImage(png, sidebar, nullptr);
      CGImageDestinationFinalize(png);
      CFRelease(png);
    }
    CGImageRelease(sidebar);
  });
  return true;
}

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
  host_show_window_behind(ui_view, nullptr);
}

void host_show_window_behind(void* ui_view, void* above_ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
  Browser* above = above_ui_view ? BrowserWithView(above_ui_view) : nullptr;
  NSView* view = browser->ui_view;
  NSWindow* window = view.window;
  browser->window = window;
  // Never drawn, but Mission Control and the Window menu name the window by it.
  window.title = [NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleName"];
  // Under the UI's view, since the core keeps only the UI's and the page's views in its own.
  [view addSubview:WindowMaterial(view.bounds) positioned:NSWindowBelow relativeTo:nil];
  // Views keeps a new window clear until its compositor's first frame, which waits for the GPU
  // process to start, though the UI's and the pages' views cover all of that compositor.
  window.alphaValue = 1;
  if (g_launch_window && !above) {
    // It takes the launch window's place, and its picture stays over the UI until the UI has
    // drawn. The launch window itself can't stay over it: Chromium counts a covered window's
    // pages as hidden, and they don't paint.
    window.animationBehavior = NSWindowAnimationBehaviorNone;
    if (g_launch_picture) {
      [view addSubview:g_launch_picture];
    }
  }
  WatchMainWindow();
  if (above && above->window) {
    [window orderWindow:NSWindowBelow relativeTo:above->window.windowNumber];
  } else {
    [window makeKeyAndOrderFront:nil];
    [g_launch_window orderOut:nil];
    g_launch_window = nil;
    [NSApp activateIgnoringOtherApps:YES];
  }

  // A click outside the command bar closes it first, so the click lands the way it would with
  // the bar closed. Otherwise it only makes the browser window key again: the page doesn't take
  // the click that activates its window.
  constexpr NSEventMask kMouseDown =
      NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  // The find bar and the offer stay open, and a click on their browser window takes the keyboard
  // from them.
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
                                                each->offer_keyboard = false;
                                                if (each->find_bar.keyWindow ||
                                                    each->offer.keyWindow) {
                                                  [each->window makeKeyWindow];
                                                }
                                              } else if (event.window == each->find_bar) {
                                                each->find_bar_keyboard = true;
                                                each->offer_keyboard = false;
                                              } else if (event.window == each->offer) {
                                                each->offer_keyboard = true;
                                                each->find_bar_keyboard = false;
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
                                            if (event.window != g_capture) {
                                              DismissCapture();
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
                                                if (browser->offer_shown) {
                                                  PlaceOffer(browser);
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
                                                  PlaceOverlayShadow(g_site_controls_overlay);
                                                }
                                                if (g_capture_shown &&
                                                    g_capture_over == browser) {
                                                  DismissCapture();
                                                }
                                              }];
}

size_t host_screens(double* frames, size_t count) {
  NSScreen* main = NSScreen.screens.firstObject;
  NSScreen* first = WindowScreen();
  NSMutableArray<NSScreen*>* screens = [NSMutableArray arrayWithObject:first];
  // With BROWSER_DISPLAY=builtin, the Mac's own alone, so no window opens over the other.
  if (strcmp(getenv("BROWSER_DISPLAY") ?: "", "builtin") != 0) {
    for (NSScreen* screen in NSScreen.screens) {
      if (screen != first) {
        [screens addObject:screen];
      }
    }
  }
  for (size_t i = 0; i < MIN(count, (size_t)screens.count); i++) {
    const NSRect visible = screens[i].visibleFrame;
    frames[i * 4] = NSMinX(visible);
    // AppKit's y runs up from the bottom of the main display, the core's down from its top.
    frames[i * 4 + 1] = NSHeight(main.frame) - NSMaxY(visible);
    frames[i * 4 + 2] = NSWidth(visible);
    frames[i * 4 + 3] = NSHeight(visible);
  }
  return screens.count;
}

bool host_window_frame(void* ui_view, double* x, double* y, double* width, double* height) {
  Browser* browser = BrowserWithView(ui_view);
  NSWindow* window = browser ? browser->window : nil;
  if (!window || (window.styleMask & NSWindowStyleMaskFullScreen)) {
    return false;
  }
  const NSRect frame = window.frame;
  *x = NSMinX(frame);
  *y = NSHeight(NSScreen.screens.firstObject.frame) - NSMaxY(frame);
  *width = NSWidth(frame);
  *height = NSHeight(frame);
  return true;
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

void host_activate(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  NSWindow* window = browser ? browser->window : nil;
  if (!window) {
    return;
  }
  if (window.miniaturized) {
    [window deminiaturize:nil];
  }
  [window makeKeyAndOrderFront:nil];
  // Its find bar or offer takes the keyboard back if it had it, as when an overlay over it hides.
  if (NSWindow* over = KeyWindowOver(browser); over != window) {
    [over makeKeyWindow];
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

void host_watch_full_screen(void (*changed)(void* ui_view, bool entered)) {
  for (NSNotificationName name in
       @[ NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification ]) {
    const bool entered = name == NSWindowDidEnterFullScreenNotification;
    [NSNotificationCenter.defaultCenter addObserverForName:name
                                                    object:nil
                                                     queue:nil
                                                usingBlock:^(NSNotification* notification) {
                                                  if (Browser* browser =
                                                          BrowserOf(notification.object)) {
                                                    changed((__bridge void*)browser->ui_view,
                                                            entered);
                                                  }
                                                }];
  }
}

void host_watch_resigning_active(void (*resigning)(void)) {
  [NSNotificationCenter.defaultCenter addObserverForName:NSApplicationWillResignActiveNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                resigning();
                                              }];
}

void host_watch_active(void (*became_active)(void)) {
  [NSNotificationCenter.defaultCenter addObserverForName:NSApplicationDidBecomeActiveNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                became_active();
                                              }];
}

bool host_is_default_browser(void) {
  NSString* mine = NSBundle.mainBundle.bundleIdentifier;
  for (NSString* link in @[ @"http://", @"https://" ]) {
    NSURL* url = [NSURL URLWithString:link];
    NSURL* app = [NSWorkspace.sharedWorkspace URLForApplicationToOpenURL:url];
    if (!app || ![[NSBundle bundleWithURL:app].bundleIdentifier isEqualToString:mine]) {
      return false;
    }
  }
  return true;
}

void host_make_default_browser(void (*answered)(void)) {
  NSWorkspace* workspace = NSWorkspace.sharedWorkspace;
  NSURL* app = NSBundle.mainBundle.bundleURL;
  void (^done)(void) = ^{
    dispatch_async(dispatch_get_main_queue(), ^{
      answered();
    });
  };
  // https only once the user has said yes for http, as Chrome does, so a user who keeps their
  // browser is asked once.
  [workspace setDefaultApplicationAtURL:app
                   toOpenURLsWithScheme:@"http"
                      completionHandler:^(NSError* error) {
                        if (error) {
                          done();
                          return;
                        }
                        [NSWorkspace.sharedWorkspace setDefaultApplicationAtURL:app
                                                           toOpenURLsWithScheme:@"https"
                                                              completionHandler:^(NSError*) {
                                                                done();
                                                              }];
                      }];
  // From macOS 26.4, HTML files ask in a second dialog of Finder's, as Chrome found, so there the
  // user picks the app for them in Finder.
  if (@available(macOS 26.4, *)) {
  } else {
    [workspace setDefaultApplicationAtURL:app toOpenContentType:UTTypeHTML completionHandler:nil];
  }
  // The system's agent that asks can put its dialog behind the app's windows, where the user
  // never sees it.
  NSString* asker = @"com.apple.coreservices.uiagent";
  for (NSRunningApplication* agent in
       [NSRunningApplication runningApplicationsWithBundleIdentifier:asker]) {
    [agent activateWithOptions:NSApplicationActivateAllWindows];
  }
}

double host_since_launch(void) {
  struct kinfo_proc process;
  size_t size = sizeof(process);
  int name[] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()};
  if (sysctl(name, 4, &process, &size, nullptr, 0) != 0) {
    return 0;
  }
  struct timeval now;
  gettimeofday(&now, nullptr);
  struct timeval start = process.kp_proc.p_starttime;
  return (now.tv_sec - start.tv_sec) * 1e3 + (now.tv_usec - start.tv_usec) / 1e3;
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
  if (g_link_shown && g_link_over == browser) {
    host_hide_link_target(0);
  }
  if (g_hud_shown && g_hud_over == browser) {
    host_hide_hud();
  }
  if (g_drop_targets_shown && g_drop_targets_over == browser) {
    host_hide_drop_targets();
  }
  if (browser->reveal_monitor) {
    [NSEvent removeMonitor:browser->reveal_monitor];
    browser->reveal_monitor = nil;
  }
  // The entry stays, as the observers made for it hold it and the driver counts windows by it,
  // but nothing finds it by its window or view again, and they go. The app closes the find bar's,
  // the offer's and the reveal's windows.
  browser->find_bar_shown = false;
  browser->offer_shown = false;
  browser->window = nil;
  browser->ui_view = nil;
  browser->find_bar = nil;
  browser->find_bar_view = nil;
  browser->offer = nil;
  browser->offer_view = nil;
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
  AddBlurAndShadow(overlay);
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
                               std::none_of(std::begin(g_overlays), std::end(g_overlays),
                                            [](const Overlay& other) { return other.shown; })) {
                      // The same for the find bar and the offer, which the user left only by
                      // clicking.
                      if (NSWindow* over = KeyWindowOver(browser); over != window) {
                        [over makeKeyWindow];
                      }
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

void host_set_overlay_radius(int index, double radius) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || overlay->radius == radius) {
    return;
  }
  overlay->radius = radius;
  MaskOverlayBlur(*overlay);
  // Drawn again at its next show.
  [overlay->shadow setFrame:NSZeroRect display:NO];
}

// |overlay| at near nothing about its growth point, as a sublayer transform of the layer holding
// its blur and drawing.
static CATransform3D ShrunkOverlay(const Overlay& overlay) {
  NSView* container = overlay.view.superview;
  CALayer* layer = container.layer;
  const NSSize size = container.bounds.size;
  // A sublayer transform applies about the layer's anchor point, which AppKit puts at a view's
  // bottom left, and y goes up unless the view is flipped.
  const double x = overlay.grow_x - layer.anchorPoint.x * size.width;
  const double y = (container.isFlipped ? overlay.grow_y : size.height - overlay.grow_y) -
                   layer.anchorPoint.y * size.height;
  CATransform3D shrunk = CATransform3DMakeTranslation(x, y, 0);
  shrunk = CATransform3DScale(shrunk, 0.05, 0.05, 1);
  return CATransform3DTranslate(shrunk, -x, -y, 0);
}

// Grows |overlay| in from near nothing about its growth point, its blur and drawing together, as
// NSPopover does, with its shadow fading in alongside.
static void GrowOverlay(Overlay& overlay) {
  NSView* container = overlay.view.superview;
  container.wantsLayer = YES;
  CALayer* layer = container.layer;
  // Shown again while it shrank away.
  [layer removeAnimationForKey:@"shrink"];
  [layer removeAnimationForKey:@"shrink fade"];
  CABasicAnimation* grow = [CABasicAnimation animationWithKeyPath:@"sublayerTransform"];
  grow.fromValue = [NSValue valueWithCATransform3D:ShrunkOverlay(overlay)];
  grow.toValue = [NSValue valueWithCATransform3D:CATransform3DIdentity];
  grow.duration = kOverlayGrow;
  grow.timingFunction = [CAMediaTimingFunction functionWithControlPoints:0.15:0.75:0.35:1];
  [layer addAnimation:grow forKey:@"grow"];
  CABasicAnimation* fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
  fade.fromValue = @0;
  fade.toValue = @1;
  fade.duration = kOverlayGrowFade;
  [layer addAnimation:fade forKey:@"fade"];
  overlay.shadow.alphaValue = 0;
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = kOverlayGrow;
        overlay.shadow.animator.alphaValue = 1;
      }
      completionHandler:nil];
}

// Shrinks |overlay| back to its growth point and fades it, then takes it off |parent|.
static void ShrinkOverlay(Overlay& overlay, NSWindow* parent) {
  CALayer* layer = overlay.view.superview.layer;
  NSWindow* window = overlay.window;
  // What it shows stays, and the clicks go through it to what's under it.
  window.ignoresMouseEvents = YES;
  Overlay* shrinking = &overlay;
  [CATransaction begin];
  [CATransaction setCompletionBlock:^{
    window.ignoresMouseEvents = NO;
    if (shrinking->shown) {
      return;
    }
    [layer removeAnimationForKey:@"shrink"];
    [layer removeAnimationForKey:@"shrink fade"];
    [parent removeChildWindow:window];
    [window orderOut:nil];
  }];
  CAMediaTimingFunction* timing =
      [CAMediaTimingFunction functionWithName:kCAMediaTimingFunctionEaseIn];
  CABasicAnimation* shrink = [CABasicAnimation animationWithKeyPath:@"sublayerTransform"];
  shrink.toValue = [NSValue valueWithCATransform3D:ShrunkOverlay(overlay)];
  CABasicAnimation* fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
  fade.toValue = @0;
  for (CABasicAnimation* animation in @[ shrink, fade ]) {
    animation.duration = kOverlayShrink;
    animation.timingFunction = timing;
    // Held at its end until the window goes, so it doesn't come back for a frame.
    animation.fillMode = kCAFillModeForwards;
    animation.removedOnCompletion = NO;
  }
  [layer addAnimation:shrink forKey:@"shrink"];
  [layer addAnimation:fade forKey:@"shrink fade"];
  [CATransaction commit];
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = kOverlayShrink;
        overlay.shadow.animator.alphaValue = 0;
      }
      completionHandler:nil];
}

void host_set_overlay_shrink(int index) {
  if (Overlay* overlay = OverlayAt(index)) {
    overlay->shrinks = true;
  }
}

void host_set_overlay_growth(int index, double x, double y) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  overlay->grows = true;
  overlay->grow_x = x;
  overlay->grow_y = y;
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
  const bool moved = overlay->centered != centered || overlay->left != left ||
                     overlay->top != top || overlay->width != width;
  overlay->centered = centered;
  overlay->left = left;
  overlay->top = top;
  overlay->width = width;
  if (overlay->shown && overlay->over == browser) {
    // Such as ⌘L while the command bar is open over the URL field: it moves without going.
    if (!moved) {
      [overlay->window makeKeyAndOrderFront:nil];
      ReleaseKeys(overlay->window);
      return;
    }
  } else {
    // One overlay at a time, as one has the keyboard.
    for (int other = 0; other < int(std::size(g_overlays)); other++) {
      if (other != index) {
        host_dismiss_overlay(other);
      }
    }
    host_hide_overlay(index);
    overlay->shown = true;
    overlay->over = browser;
    HoldKeys(overlay->window);
  }
  // The window still holds what it drew when it last closed, such as the previous query. It goes
  // on screen once the UI has drawn it as it is now, which can't happen while the UI is still
  // handling the shortcut that opened it.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!overlay->shown) {
      ReleaseKeys(overlay->window);
    }
    if (!overlay->shown || overlay->over != browser) {
      return;
    }
    NSWindow* window = browser->window;
    NSRect content = [window contentRectForFrameRect:window.frame];
    const double height = NSHeight(overlay->window.frame);
    NSRect frame;
    if (centered) {
      frame = NSMakeRect(NSMidX(content) - width / 2, NSMidY(content) + top - height, width,
                         height);
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
      AddOver(window, overlay->shadow);
    }
    // Still on another browser window as it shrinks away from there.
    if (overlay->window.parentWindow && overlay->window.parentWindow != window) {
      [overlay->window.parentWindow removeChildWindow:overlay->window];
    }
    overlay->window.ignoresMouseEvents = NO;
    AddOver(window, overlay->window);
    if (overlay->grows) {
      GrowOverlay(*overlay);
    }
    [overlay->window makeKeyAndOrderFront:nil];
    ReleaseKeys(overlay->window);
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
  if (overlay->shrinks && overlay->grows && overlay->window.visible) {
    ShrinkOverlay(*overlay, browser->window);
  } else {
    [browser->window removeChildWindow:overlay->window];
    [overlay->window orderOut:nil];
    overlay->shadow.alphaValue = 0;
  }
  [KeyWindowOver(browser) makeKeyWindow];
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

void host_hold_keys(void) {
  HoldKeys(nil);
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

namespace {

// host_copy's pasteboard: the clipboard, or in a driven run one of its own, so checks never touch
// the user's clipboard.
NSPasteboard* CopyPasteboard() {
  static NSPasteboard* driven =
      getenv("BROWSER_DRIVE") ? [NSPasteboard pasteboardWithUniqueName] : nil;
  return driven ?: NSPasteboard.generalPasteboard;
}

// A capture's name, for when it was taken, as macOS names a screenshot.
NSString* CaptureName() {
  NSDateFormatter* format = [[NSDateFormatter alloc] init];
  format.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
  format.dateFormat = @"'Capture' yyyy-MM-dd 'at' HH.mm.ss";
  return [[format stringFromDate:NSDate.date] stringByAppendingString:@".png"];
}

}  // namespace

void host_copy_png(const uint8_t* png, size_t size) {
  NSPasteboard* pasteboard = CopyPasteboard();
  [pasteboard clearContents];
  [pasteboard setData:[NSData dataWithBytes:png length:size] forType:NSPasteboardTypePNG];
  // check.sh reads this line: what the pasteboard holds now, and which one it is.
  fprintf(stderr, "browser: pasteboard png %lu bytes, %s\n",
          (unsigned long)[pasteboard dataForType:NSPasteboardTypePNG].length,
          pasteboard == NSPasteboard.generalPasteboard ? "general" : "private");
}

void host_save_png(void* ui_view, const uint8_t* png, size_t size) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->window) {
    return;
  }
  NSData* data = [NSData dataWithBytes:png length:size];
  NSSavePanel* panel = [NSSavePanel savePanel];
  panel.allowedContentTypes = @[ UTTypePNG ];
  panel.nameFieldStringValue = CaptureName();
  [panel beginSheetModalForWindow:browser->window
                completionHandler:^(NSModalResponse response) {
                  if (response == NSModalResponseOK) {
                    [data writeToURL:panel.URL atomically:YES];
                  }
                }];
  fprintf(stderr, "browser: capture save panel shown\n");
}

void host_share_png(const uint8_t* png,
                    size_t size,
                    void* view,
                    double x,
                    double y,
                    double width,
                    double height,
                    bool messages) {
  // Shared as a file, so what takes it gets the capture's name, in a folder of its own.
  NSString* folder = [NSTemporaryDirectory()
      stringByAppendingPathComponent:NSUUID.UUID.UUIDString];
  [NSFileManager.defaultManager createDirectoryAtPath:folder
                          withIntermediateDirectories:YES
                                           attributes:nil
                                                error:nil];
  NSURL* file = [NSURL fileURLWithPath:[folder stringByAppendingPathComponent:CaptureName()]];
  if (![[NSData dataWithBytes:png length:size] writeToURL:file atomically:YES]) {
    return;
  }
  if (messages) {
    [[NSSharingService sharingServiceNamed:NSSharingServiceNameComposeMessage]
        performWithItems:@[ file ]];
    fprintf(stderr, "browser: capture sent to messages\n");
    return;
  }
  NSView* anchor = (__bridge NSView*)view;
  if (!anchor.window) {
    return;
  }
  NSRect rect = NSMakeRect(x, anchor.flipped ? y : NSHeight(anchor.bounds) - y - height, width,
                           height);
  NSSharingServicePicker* picker = [[NSSharingServicePicker alloc] initWithItems:@[ file ]];
  [picker showRelativeToRect:rect ofView:anchor preferredEdge:NSRectEdgeMinY];
  fprintf(stderr, "browser: capture share menu shown\n");
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
      AddOver(browser->window, g_link);
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
  NSPasteboard* pasteboard = CopyPasteboard();
  [pasteboard clearContents];
  [pasteboard setString:@(text) forType:NSPasteboardTypeString];
}

void host_open_in_safari(const char* url) {
  NSURL* page = [NSURL URLWithString:@(url)];
  // Only web pages go to another app.
  if (![page.scheme isEqualToString:@"http"] && ![page.scheme isEqualToString:@"https"]) {
    return;
  }
  // check.sh reads this line. A driven run opens nothing, so checks never launch Safari.
  fprintf(stderr, "browser: open in safari %s\n", url);
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  NSURL* safari =
      [NSWorkspace.sharedWorkspace URLForApplicationWithBundleIdentifier:@"com.apple.Safari"];
  if (!safari) {
    fprintf(stderr, "browser: no Safari to open %s\n", url);
    return;
  }
  [NSWorkspace.sharedWorkspace openURLs:@[ page ]
                   withApplicationAtURL:safari
                          configuration:[NSWorkspaceOpenConfiguration configuration]
                      completionHandler:nil];
}

void host_share(void* ui_view,
                double x,
                double y,
                double width,
                double height,
                const char* url,
                const char* title) {
  Browser* browser = BrowserWithView(ui_view);
  NSURL* link = [NSURL URLWithString:@(url)];
  if (!browser || !browser->window || !link) {
    return;
  }
  // check.sh reads this line. A driven run doesn't show the menu, which would wait on a choice.
  fprintf(stderr, "browser: share %s\n", url);
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  // Kept while the menu shows, which doesn't keep its delegate.
  static NSSharingServicePicker* picker;
  static HostShareDelegate* delegate;
  delegate = [[HostShareDelegate alloc] init];
  delegate.title = @(title);
  picker = [[NSSharingServicePicker alloc] initWithItems:@[ link ]];
  picker.delegate = delegate;
  NSView* content = browser->window.contentView;
  // From the content's top left, under the tile it hangs from.
  const NSRect rect = NSMakeRect(x, content.flipped ? y : NSHeight(content.bounds) - y - height,
                                 width, height);
  [picker showRelativeToRect:rect
                      ofView:content
               preferredEdge:content.flipped ? NSRectEdgeMaxY : NSRectEdgeMinY];
}

namespace {

// The system's share services for |link|, as its share menu lists them. AppKit's own Share item
// can't be a submenu of the UI's menus, and this is the list its picker shows.
NSArray<NSSharingService*>* ShareServices(NSURL* link) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  return [NSSharingService sharingServicesForItems:@[ link ]];
#pragma clang diagnostic pop
}

}  // namespace

void host_share_services(const char* url,
                         void (*each)(void* context, const char* name),
                         void* context) {
  NSURL* link = [NSURL URLWithString:@(url)];
  if (!link) {
    return;
  }
  for (NSSharingService* service in ShareServices(link)) {
    each(context, service.menuItemTitle.UTF8String ?: "");
  }
}

void host_share_with(const char* url, const char* title, size_t index) {
  NSURL* link = [NSURL URLWithString:@(url)];
  NSArray<NSSharingService*>* services = link ? ShareServices(link) : @[];
  if (index >= services.count) {
    return;
  }
  NSSharingService* service = services[index];
  // check.sh reads this line. A driven run shares nothing, as the service takes the page to
  // another app.
  fprintf(stderr, "browser: share %s with %s\n", url, service.menuItemTitle.UTF8String ?: "");
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  service.subject = @(title);
  [service performWithItems:@[ link ]];
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

bool host_dismiss_menu(void) {
  if (!g_menu) {
    return false;
  }
  [g_menu cancelTracking];
  return true;
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

// For menu_window.mm, whose window goes over a browser window as the overlays here do.
void HostAddOver(NSWindow* parent, NSWindow* child) {
  AddOver(parent, child);
}

void HostMakeOverlay(NSWindow* window) {
  MakeOverlay(window);
}

NSView* HostRevealView(void* ui_view) {
  Browser* browser = BrowserWithView(ui_view);
  return browser ? browser->reveal_view : nil;
}
