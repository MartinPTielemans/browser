#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <string>
#include <vector>

#import "ArdiaCore.h"
#include "host.h"

// drive.mm
void spike_drive_start(void);

@interface SpikeHost : NSObject <ArdiaCoreEngineDelegate, ArdiaCoreWindowDelegate, ArdiaCoreTabDelegate>
@end

namespace {

SpikeHost* g_host;
spike_started_fn g_on_started;
spike_command_fn g_on_command;
ArdiaCoreWindow* g_window;
ArdiaCoreTab* g_tab;
// Where the page goes in the window: inset by |left| on the left and |gap| on the other sides.
CGFloat g_left, g_gap;

// Whether the page is full screen, as when a video's full-screen button is pressed, and whether
// the window went full screen for it, so leaving puts the window back as it was.
bool g_page_fullscreen = false;
bool g_window_fullscreen_for_page = false;

// The command bar's window and view.
NSWindow* g_overlay;
NSView* g_overlay_view;
bool g_overlay_shown = false;

// The browser's shortcuts while the page has the keyboard. As in Chrome, the reserved ones never
// reach the page. The others reach it first and come back if it doesn't handle them, so a page
// like Google Docs can keep ⌘F.
struct Shortcut {
  unichar key;
  spike_command command;
  bool reserved;
};
constexpr Shortcut kShortcuts[] = {
    {'t', SPIKE_NEW_TAB, true}, {'w', SPIKE_CLOSE_TAB, true}, {'q', SPIKE_QUIT, true},
    {'l', SPIKE_EDIT_URL, false}, {'f', SPIKE_FIND, false},
};

bool RunShortcut(NSEvent* event, bool reserved) {
  constexpr NSEventModifierFlags kOthers =
      NSEventModifierFlagShift | NSEventModifierFlagOption | NSEventModifierFlagControl;
  NSString* key = event.charactersIgnoringModifiers;
  if (!g_on_command || event.type != NSEventTypeKeyDown || key.length != 1 ||
      !(event.modifierFlags & NSEventModifierFlagCommand) || (event.modifierFlags & kOthers)) {
    return false;
  }
  for (const Shortcut& shortcut : kShortcuts) {
    if (shortcut.key == [key characterAtIndex:0] && shortcut.reserved == reserved) {
      g_on_command(shortcut.command);
      return true;
    }
  }
  return false;
}

// A full-screen page covers the whole window, sidebar included.
NSRect PageFrame() {
  NSRect bounds = g_window.hostView.bounds;
  if (g_page_fullscreen) {
    return bounds;
  }
  return NSMakeRect(g_left, g_gap, NSWidth(bounds) - g_left - g_gap, NSHeight(bounds) - 2 * g_gap);
}

bool IsWindowFullscreen() {
  return g_window.window.styleMask & NSWindowStyleMaskFullScreen;
}

// Puts |tab|'s page in the window, in place of the one there.
void ShowTab(ArdiaCoreTab* tab) {
  ArdiaCoreTab* old = g_tab;
  g_tab = tab;
  tab.delegate = g_host;
  NSView* host = g_window.hostView;
  NSView* page = tab.view;
  page.frame = PageFrame();
  page.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  [host addSubview:page];
  if (old && old != tab) {
    [old.view removeFromSuperview];
    [old close];
  }
}

}  // namespace

@implementation SpikeHost

- (void)engineDidStart {
  g_on_started();
}

// check.sh reads these lines. Test pages report what reached them through their title.
- (void)tabDidChangeTitle:(ArdiaCoreTab*)tab {
  fprintf(stderr, "spike: title %s\n", tab.title.UTF8String);
}

- (void)tabDidChangeLoading:(ArdiaCoreTab*)tab {
  if (!tab.loading) {
    fprintf(stderr, "spike: loaded %s\n", tab.url.UTF8String);
  }
}

- (void)tab:(ArdiaCoreTab*)tab didFailLoad:(NSString*)url code:(int)code description:(NSString*)text {
  fprintf(stderr, "spike: load error %d %s %s\n", code, text.UTF8String, url.UTF8String);
}

// The spike has one tab, so a link that would open another opens in its place. Chromium has
// already made the new page; it replaces the old one.
- (void)window:(ArdiaCoreWindow*)window
    didInsertTab:(ArdiaCoreTab*)tab
          opener:(ArdiaCoreTab*)opener
     disposition:(NSString*)disposition {
  if (g_tab && tab != g_tab) {
    ShowTab(tab);
  }
}

- (BOOL)window:(ArdiaCoreWindow*)window preHandleKeyEvent:(NSEvent*)event {
  // As in Chrome, escape takes a full-screen page back, and the page doesn't see it.
  constexpr unsigned short kEscape = 53;
  constexpr NSEventModifierFlags kModifiers = NSEventModifierFlagCommand | NSEventModifierFlagShift |
                                              NSEventModifierFlagOption | NSEventModifierFlagControl;
  if (g_page_fullscreen && event.type == NSEventTypeKeyDown && event.keyCode == kEscape &&
      !(event.modifierFlags & kModifiers)) {
    [g_tab exitExclusiveAccess];
    return YES;
  }
  return RunShortcut(event, true);
}

// The page went full screen or came back. As in Chrome, the window goes full screen with it unless
// it already was, and comes back only if it went for the page.
- (void)window:(ArdiaCoreWindow*)window tab:(ArdiaCoreTab*)tab didChangeFullscreen:(BOOL)fullscreen {
  if (tab != g_tab || fullscreen == g_page_fullscreen) {
    return;
  }
  g_page_fullscreen = fullscreen;
  tab.view.frame = PageFrame();
  if (fullscreen && !IsWindowFullscreen()) {
    g_window_fullscreen_for_page = true;
    [window.window toggleFullScreen:nil];
  } else if (!fullscreen && g_window_fullscreen_for_page) {
    g_window_fullscreen_for_page = false;
    if (IsWindowFullscreen()) {
      [window.window toggleFullScreen:nil];
    }
  }
  fprintf(stderr, "spike: page %s full screen\n", fullscreen ? "entered" : "left");
}

// Called with the keys the page didn't handle.
- (BOOL)window:(ArdiaCoreWindow*)window handleKeyEvent:(NSEvent*)event {
  return RunShortcut(event, false);
}

// The window's close button closes the browser, which has one window.
- (BOOL)windowShouldClose:(ArdiaCoreWindow*)window {
  spike_host_quit();
  return NO;
}

@end

int spike_host_run(int argc, char** argv, spike_started_fn on_started, spike_command_fn on_command) {
  bool helper = false;
  for (int i = 1; i < argc; i++) {
    helper |= strncmp(argv[i], "--type=", 7) == 0;
  }
  std::vector<const char*> args(argv, argv + argc);
  std::string data_dir;
  if (!helper) {
    g_on_started = on_started;
    g_on_command = on_command;
    NSString* dir =
        [NSHomeDirectory() stringByAppendingPathComponent:@"Library/Caches/browser-spike-ardia"];
    data_dir = std::string("--user-data-dir=") + dir.UTF8String;
    args.push_back(data_dir.c_str());
    // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed build asking for
    // it gets a password prompt, and every request that uses cookies waits until someone answers.
    args.push_back("--use-mock-keychain");
    // Chrome would copy the app bundle to check its signature against, which an ad-hoc-signed
    // build doesn't need. Ardia turns it off the same way.
    args.push_back("--disable-features=MacAppCodeSignClone");
    if (getenv("SPIKE_DRIVE")) {
      spike_drive_start();
    }
  }
  g_host = [SpikeHost new];
  return [ArdiaCoreEngine runWithArgc:(int)args.size() argv:args.data() delegate:g_host];
}

void* spike_host_open_window(const char* url, double left, double gap) {
  g_left = left;
  g_gap = gap;
  g_window = [[ArdiaCoreWindow alloc] initWithContentRect:NSMakeRect(0, 0, 1280, 800)];
  g_window.delegate = g_host;
  NSWindow* window = g_window.window;
  window.styleMask |= NSWindowStyleMaskFullSizeContentView;
  window.titlebarAppearsTransparent = YES;
  window.titleVisibility = NSWindowTitleHidden;
  window.releasedWhenClosed = NO;
  [window center];
  ShowTab([g_window openTab:@(url) profile:ArdiaCoreEngine.sharedEngine.defaultProfile
                 foreground:YES]);
  [window makeKeyAndOrderFront:nil];
  [g_tab focus];
  [NSApp activateIgnoringOtherApps:YES];

  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  // Leaving full screen with the green button or ⌃⌘F takes a full-screen page back too.
  [center addObserverForName:NSWindowWillExitFullScreenNotification
                      object:window
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (g_page_fullscreen) {
                      g_window_fullscreen_for_page = false;
                      [g_tab exitExclusiveAccess];
                    }
                  }];
  // AppKit ignores a toggle during a transition, so a page that left full screen while the window
  // was still entering it takes the window back once it's there.
  [center addObserverForName:NSWindowDidEnterFullScreenNotification
                      object:window
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    if (!g_page_fullscreen && g_window_fullscreen_for_page) {
                      g_window_fullscreen_for_page = false;
                      [g_window.window toggleFullScreen:nil];
                    }
                  }];

  // A click outside the command bar closes it first, so the click lands the way it would with
  // the bar closed. Otherwise it only makes the browser window key again: the page doesn't take
  // the click that activates its window.
  constexpr NSEventMask kMouseDown =
      NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  [NSEvent addLocalMonitorForEventsMatchingMask:kMouseDown
                                        handler:^NSEvent*(NSEvent* event) {
                                          if (g_overlay_shown && event.window != g_overlay) {
                                            spike_host_hide_overlay();
                                          }
                                          return event;
                                        }];
  return (__bridge void*)g_window.hostView;
}

void spike_host_load_url(const char* url) {
  [g_tab loadURL:@(url)];
}

void spike_host_add_overlay(void* overlay_view) {
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
                    spike_host_hide_overlay();
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
                    if (notification.object == g_window.window) {
                      [g_overlay makeKeyWindow];
                    } else {
                      spike_host_hide_overlay();
                    }
                  }];
}

void spike_host_show_overlay(void) {
  if (!g_overlay) {
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
    NSWindow* parent = g_window.window;
    NSRect frame = parent.frame;
    NSSize size = g_overlay.frame.size;
    [g_overlay setFrameOrigin:NSMakePoint(NSMidX(frame) - size.width / 2,
                                          NSMaxY(frame) - NSHeight(frame) / 5 - size.height)];
    [parent addChildWindow:g_overlay ordered:NSWindowAbove];
    [g_overlay makeKeyAndOrderFront:nil];
    fprintf(stderr, "spike: command bar shown\n");
  });
}

void spike_host_hide_overlay(void) {
  if (!g_overlay_shown) {
    return;
  }
  g_overlay_shown = false;
  NSWindow* parent = g_window.window;
  [parent removeChildWindow:g_overlay];
  [g_overlay orderOut:nil];
  [parent makeKeyWindow];
  fprintf(stderr, "spike: command bar hidden\n");
}

void spike_host_quit(void) {
  // Chromium closes its pages and windows, then ends the run loop.
  [ArdiaCoreEngine.sharedEngine quit];
}
