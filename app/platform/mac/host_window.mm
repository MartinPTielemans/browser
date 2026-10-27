#import <ImageIO/ImageIO.h>

#include <dlfcn.h>

#include <initializer_list>
#include <iterator>
#include <memory>
#include <string>

#include "host_internal.h"

namespace {
// The UI's, which hears the drags over its view.
bool (*g_dragged)(void*, int, double, double, const host_dragged*, size_t);
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

// In the order their views were made. Each is allocated alone, so the pointers stay put.
std::vector<std::unique_ptr<Browser>> g_browsers;

}  // namespace

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

namespace {

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

void host_show_launch_window(const char* profile_dir, double width, double height) {
  double x = 0, y = 0;
  host_centered(width, height, &x, &y);
  // host_centered's y runs down from the main display's top, AppKit's up from its bottom.
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
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return;
  }
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
  if (g_launch_window) {
    // It takes the launch window's place, and its picture stays over the UI until the UI has
    // drawn. The launch window itself can't stay over it: Chromium counts a covered window's
    // pages as hidden, and they don't paint.
    window.animationBehavior = NSWindowAnimationBehaviorNone;
    if (g_launch_picture) {
      [view addSubview:g_launch_picture];
    }
  }
  WatchMainWindow();
  [window makeKeyAndOrderFront:nil];
  [g_launch_window orderOut:nil];
  g_launch_window = nil;
  [NSApp activateIgnoringOtherApps:YES];

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

void host_take_drops(bool (*dragged)(void* ui_view, int phase, double x, double y,
                                     const host_dragged* items, size_t count)) {
  g_dragged = dragged;
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

void host_focus(void* view) {
  NSView* ns_view = (__bridge NSView*)view;
  [ns_view.window makeFirstResponder:ns_view];
}

// The Settings window and its view, whether it's to show, and whether it has, after which it keeps
// the place the user left it in.
NSWindow* g_settings;
NSView* g_settings_view;
bool g_settings_shown = false;
bool g_settings_placed = false;

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

void* host_browser_window(size_t index) {
  return index < g_browsers.size() ? (__bridge void*)g_browsers[index]->window : nullptr;
}
