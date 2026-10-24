#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <string>

#include "host.h"
#include "include/base/cef_callback.h"
#include "include/cef_app.h"
#include "include/cef_application_mac.h"
#include "include/cef_browser.h"
#include "include/cef_client.h"
#include "include/cef_task.h"
#include "include/wrapper/cef_closure_task.h"
#include "include/wrapper/cef_library_loader.h"

// Chromium needs the application to tell it when -sendEvent: is on the stack. Chrome's own
// framework uses its BrowserCrApplication the same way.
@interface SpikeApplication : NSApplication <CefAppProtocol> {
  BOOL handlingSendEvent_;
}
@end

// drive.mm
void spike_drive_start(void);

namespace {

CefScopedLibraryLoader* g_library;
CefRefPtr<CefBrowser> g_browser;
spike_command_fn g_on_command;
bool g_context_ready = false;
// From asking Chromium for the page until it exists.
bool g_creating = false;
bool g_quitting = false;

// The command bar's window and view, and the browser window it floats over.
NSWindow* g_overlay;
NSView* g_overlay_view;
NSWindow* g_overlay_parent;
bool g_overlay_shown = false;

// The page view requested before Chromium finished starting.
struct PendingBrowser {
  NSView* parent;
  CefRect bounds;
  std::string url;
};
PendingBrowser* g_pending;

// The browser's shortcuts while the page has the keyboard. As in Chrome, the reserved ones never
// reach the page. The others reach it first and come back if it doesn't handle them, so a page
// like Google Docs can keep ⌘F.
struct Shortcut {
  char16_t key;
  spike_command command;
  bool reserved;
};
constexpr Shortcut kShortcuts[] = {
    {'t', SPIKE_NEW_TAB, true}, {'w', SPIKE_CLOSE_TAB, true}, {'q', SPIKE_QUIT, true},
    {'l', SPIKE_EDIT_URL, false}, {'f', SPIKE_FIND, false},
};

bool RunShortcut(const CefKeyEvent& event, bool reserved) {
  if (!g_on_command || event.type != KEYEVENT_RAWKEYDOWN ||
      !(event.modifiers & EVENTFLAG_COMMAND_DOWN) ||
      (event.modifiers & (EVENTFLAG_SHIFT_DOWN | EVENTFLAG_ALT_DOWN | EVENTFLAG_CONTROL_DOWN))) {
    return false;
  }
  for (const Shortcut& shortcut : kShortcuts) {
    if (shortcut.key == event.unmodified_character && shortcut.reserved == reserved) {
      g_on_command(shortcut.command);
      return true;
    }
  }
  return false;
}

class Client : public CefClient,
               public CefDisplayHandler,
               public CefLifeSpanHandler,
               public CefKeyboardHandler,
               public CefLoadHandler {
 public:
  CefRefPtr<CefDisplayHandler> GetDisplayHandler() override { return this; }
  CefRefPtr<CefLifeSpanHandler> GetLifeSpanHandler() override { return this; }
  CefRefPtr<CefKeyboardHandler> GetKeyboardHandler() override { return this; }
  CefRefPtr<CefLoadHandler> GetLoadHandler() override { return this; }

  // The spike has one tab, so a link that would open another opens in it.
  bool OnBeforePopup(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, int popup_id,
                     const CefString& target_url, const CefString& target_frame_name,
                     WindowOpenDisposition target_disposition, bool user_gesture,
                     const CefPopupFeatures& popup_features, CefWindowInfo& window_info,
                     CefRefPtr<CefClient>& client, CefBrowserSettings& settings,
                     CefRefPtr<CefDictionaryValue>& extra_info,
                     bool* no_javascript_access) override {
    browser->GetMainFrame()->LoadURL(target_url);
    return true;
  }

  void OnAfterCreated(CefRefPtr<CefBrowser> browser) override {
    g_creating = false;
    g_browser = browser;
    NSView* view = CAST_CEF_WINDOW_HANDLE_TO_NSVIEW(browser->GetHost()->GetWindowHandle());
    view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    // A quit that came while the page was being created waited for it.
    if (g_quitting) {
      browser->GetHost()->CloseBrowser(true);
    }
  }

  // check.sh reads these lines. Test pages report what reached them through their title.
  void OnTitleChange(CefRefPtr<CefBrowser> browser, const CefString& title) override {
    fprintf(stderr, "spike: title %s\n", title.ToString().c_str());
  }

  void OnLoadError(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame, ErrorCode error_code,
                   const CefString& error_text, const CefString& failed_url) override {
    if (frame->IsMain()) {
      fprintf(stderr, "spike: load error %d %s %s\n", error_code, error_text.ToString().c_str(),
              failed_url.ToString().c_str());
    }
  }

  void OnLoadEnd(CefRefPtr<CefBrowser> browser, CefRefPtr<CefFrame> frame,
                 int http_status_code) override {
    if (frame->IsMain()) {
      fprintf(stderr, "spike: loaded %d %s\n", http_status_code,
              frame->GetURL().ToString().c_str());
    }
  }

  // A page view is a tab, so closing it must not close the window, which is what CEF does when
  // this returns false. Taking the view out of the window finishes the close instead.
  bool DoClose(CefRefPtr<CefBrowser> browser) override {
    NSView* view = CAST_CEF_WINDOW_HANDLE_TO_NSVIEW(browser->GetHost()->GetWindowHandle());
    // After this callback returns: removing the view can destroy the browser.
    dispatch_async(dispatch_get_main_queue(), ^{
      [view removeFromSuperview];
    });
    return true;
  }

  void OnBeforeClose(CefRefPtr<CefBrowser> browser) override {
    g_browser = nullptr;
    if (g_quitting) {
      CefQuitMessageLoop();
    }
  }

  bool OnPreKeyEvent(CefRefPtr<CefBrowser> browser, const CefKeyEvent& event,
                     CefEventHandle os_event, bool* is_keyboard_shortcut) override {
    return RunShortcut(event, true);
  }

  // Called with the keys the page didn't handle.
  bool OnKeyEvent(CefRefPtr<CefBrowser> browser, const CefKeyEvent& event,
                  CefEventHandle os_event) override {
    return RunShortcut(event, false);
  }

 private:
  IMPLEMENT_REFCOUNTING(Client);
};

void CreateBrowser(NSView* parent, const CefRect& bounds, const std::string& url) {
  CefWindowInfo info;
  // On macOS, CEF only allows Alloy style in a client-provided view. The real browser won't
  // use CEF, so the spike doesn't care which style draws the page.
  info.SetAsChild(CAST_NSVIEW_TO_CEF_WINDOW_HANDLE(parent), bounds);
  info.runtime_style = CEF_RUNTIME_STYLE_ALLOY;
  g_creating = true;
  CefBrowserHost::CreateBrowser(info, new Client, url, CefBrowserSettings(), nullptr, nullptr);
}

void CreateBrowserWhenReady(NSView* parent, NSRect frame, const std::string& url) {
  CefRect bounds((int)NSMinX(frame), (int)NSMinY(frame), (int)NSWidth(frame), (int)NSHeight(frame));
  if (g_context_ready) {
    CreateBrowser(parent, bounds, url);
  } else {
    g_pending = new PendingBrowser{parent, bounds, url};
  }
}

class App : public CefApp, public CefBrowserProcessHandler {
 public:
  CefRefPtr<CefBrowserProcessHandler> GetBrowserProcessHandler() override { return this; }

  // Chromium keeps its cookie encryption key in the Keychain. An ad-hoc-signed build asking for
  // it gets a password prompt, and every request that uses cookies waits until someone answers.
  void OnBeforeCommandLineProcessing(const CefString& process_type,
                                     CefRefPtr<CefCommandLine> command_line) override {
    if (process_type.empty()) {
      command_line->AppendSwitch("use-mock-keychain");
    }
  }

  void OnContextInitialized() override {
    g_context_ready = true;
    if (g_pending) {
      CreateBrowser(g_pending->parent, g_pending->bounds, g_pending->url);
      delete g_pending;
      g_pending = nullptr;
    }
  }

 private:
  IMPLEMENT_REFCOUNTING(App);
};

}  // namespace

@implementation SpikeApplication

- (BOOL)isHandlingSendEvent {
  return handlingSendEvent_;
}

- (void)setHandlingSendEvent:(BOOL)handlingSendEvent {
  handlingSendEvent_ = handlingSendEvent;
}

- (void)sendEvent:(NSEvent*)event {
  CefScopedSendingEvent sendingEventScoper;
  // A click outside the command bar closes it first, so the click lands the way it would with
  // the bar closed. Otherwise it only makes the browser window key again: the page doesn't take
  // the click that activates its window.
  constexpr NSEventMask kMouseDown =
      NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown;
  if (g_overlay_shown && (NSEventMaskFromType(event.type) & kMouseDown) &&
      event.window != g_overlay) {
    spike_host_hide_overlay();
  }
  [super sendEvent:event];
}

// AppKit's terminate: exits the process right away. Chromium has to close its pages first.
- (void)terminate:(id)sender {
  spike_host_quit();
}

@end

bool spike_host_init(int argc, char** argv, spike_command_fn on_command) {
  g_library = new CefScopedLibraryLoader;
  if (!g_library->LoadInMain()) {
    return false;
  }
  g_on_command = on_command;
  [SpikeApplication sharedApplication];
  if (getenv("SPIKE_DRIVE")) {
    spike_drive_start();
  }

  CefSettings settings;
  settings.no_sandbox = true;
  NSString* cache = [NSHomeDirectory() stringByAppendingPathComponent:@"Library/Caches/browser-spike"];
  CefString(&settings.root_cache_path) = cache.UTF8String;
  return CefInitialize(CefMainArgs(argc, argv), settings, new App, nullptr);
}

void spike_host_create_browser(void* parent_view, double x, double y, double width, double height,
                               const char* url) {
  // The page view goes beside the UI's view, the way the real window host parents tab views
  // (decision 3), with GPUI's view underneath.
  NSView* ui = (__bridge NSView*)parent_view;
  NSView* parent = ui.superview;
  CreateBrowserWhenReady(parent, [ui convertRect:NSMakeRect(x, y, width, height) toView:parent],
                         url);
}

void spike_host_open_plain_window(const char* url) {
  NSWindow* window =
      [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 1280, 800)
                                  styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskResizable |
                                            NSWindowStyleMaskFullSizeContentView
                                    backing:NSBackingStoreBuffered
                                      defer:NO];
  window.releasedWhenClosed = NO;
  window.titlebarAppearsTransparent = YES;
  [window center];
  [window makeKeyAndOrderFront:nil];
  // The same frame as in the spike's window, where the sidebar takes the left 240 points.
  NSView* content = window.contentView;
  NSRect frame = NSInsetRect(content.bounds, 0, 8);
  frame.origin.x = 240;
  frame.size.width = NSWidth(content.bounds) - 248;
  CreateBrowserWhenReady(content, frame, url);
  [NSApp activateIgnoringOtherApps:YES];
}

void spike_host_load_url(const char* url) {
  if (g_browser) {
    g_browser->GetMainFrame()->LoadURL(url);
  }
}

void spike_host_add_overlay(void* parent_view, void* overlay_view) {
  g_overlay_parent = ((__bridge NSView*)parent_view).window;
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
  // The overlay closes when it loses the keyboard, to another window or another app. AppKit
  // would otherwise hide the panel when the app deactivates and show it again on return.
  ((NSPanel*)g_overlay).hidesOnDeactivate = NO;
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidResignKeyNotification
                                                  object:g_overlay
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                spike_host_hide_overlay();
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
    NSRect parent = g_overlay_parent.frame;
    NSSize size = g_overlay.frame.size;
    [g_overlay setFrameOrigin:NSMakePoint(NSMidX(parent) - size.width / 2,
                                          NSMaxY(parent) - NSHeight(parent) / 5 - size.height)];
    [g_overlay_parent addChildWindow:g_overlay ordered:NSWindowAbove];
    [g_overlay makeKeyAndOrderFront:nil];
    fprintf(stderr, "spike: command bar shown\n");
  });
}

void spike_host_hide_overlay(void) {
  // Ordering the window out takes the keyboard from it, which calls back here.
  if (!g_overlay_shown) {
    return;
  }
  g_overlay_shown = false;
  [g_overlay_parent removeChildWindow:g_overlay];
  [g_overlay orderOut:nil];
  [g_overlay_parent makeKeyWindow];
  fprintf(stderr, "spike: command bar hidden\n");
}

void spike_host_run(void) {
  CefRunMessageLoop();
  CefShutdown();
  delete g_library;
}

void spike_host_quit(void) {
  if (g_quitting) {
    return;
  }
  g_quitting = true;
  // Chromium has to close the page before it shuts down. A page being created closes once it
  // exists.
  if (g_browser) {
    g_browser->GetHost()->CloseBrowser(true);
  } else if (!g_creating) {
    delete g_pending;
    g_pending = nullptr;
    // Posted, so it also ends a run loop that hasn't started yet.
    CefPostTask(TID_UI, base::BindOnce(&CefQuitMessageLoop));
  }
}
