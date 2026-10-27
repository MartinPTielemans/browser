#include "browser_core/platform/system_page.h"

#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#include <dlfcn.h>
#include <objc/message.h>
#include <stdio.h>

#include <utility>

#include "base/apple/owned_objc.h"
#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/logging.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/strings/sys_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "components/input/native_web_keyboard_event.h"
#include "media/base/media_switches.h"
#include "net/base/apple/url_conversions.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"
#include "url/gurl.h"

// The core never links WebKit, and starts none of it, no view, store or WebKit process, before the
// first system page, or before it clears or deletes a store a profile made in an earlier run. Its
// code is mapped at launch all the same: system frameworks Chromium's framework links, such as
// AuthenticationServices, SafariServices and Quartz, link it. Its classes are found by name and its
// constants by symbol, so the core adds no link of its own.

namespace browser_core::platform {
class SystemPageMac;
}

namespace {

constexpr char kWebKitPath[] = "/System/Library/Frameworks/WebKit.framework/WebKit";
void* g_webkit = nullptr;

// Loads WebKit, once. Only the three calls below that need it do, each saying why.
bool LoadWebKit(const char* why) {
  if (!g_webkit) {
    // Whether something else mapped it first, such as a system framework of Chromium's.
    const bool mapped = dlopen(kWebKitPath, RTLD_LAZY | RTLD_LOCAL | RTLD_NOLOAD) != nullptr;
    g_webkit = dlopen(kWebKitPath, RTLD_LAZY | RTLD_LOCAL);
    // check.sh reads this line.
    fprintf(stderr, "browser: system engine loaded for %s%s\n", why,
            mapped ? ", already mapped" : "");
  }
  return g_webkit != nullptr;
}

// WebKit, once a call above has loaded it.
void* WebKit() {
  return g_webkit;
}

Class WebKitClass(NSString* name) {
  return WebKit() ? NSClassFromString(name) : nil;
}

NSString* WebKitConstant(const char* name) {
  void* symbol = WebKit() ? dlsym(WebKit(), name) : nullptr;
  return symbol ? *static_cast<NSString* __unsafe_unretained*>(symbol) : nil;
}

// The stores the pages use, by id, which one profile's pages share. A private session's keeps
// nothing on disk, and goes once it's released.
NSMutableDictionary<NSString*, WKWebsiteDataStore*>* Stores() {
  static NSMutableDictionary<NSString*, WKWebsiteDataStore*>* const stores =
      [NSMutableDictionary dictionary];
  return stores;
}

WKWebsiteDataStore* StoreFor(const browser_core::platform::SystemStore& store) {
  NSString* key = base::SysUTF8ToNSString(store.id);
  if (WKWebsiteDataStore* made = Stores()[key]) {
    return made;
  }
  WKWebsiteDataStore* made = nil;
  id stores = WebKitClass(@"WKWebsiteDataStore");
  if (!store.persistent) {
    made = [stores nonPersistentDataStore];
  } else if (@available(macOS 14, *)) {
    NSUUID* uuid = [[NSUUID alloc] initWithUUIDString:key];
    made = uuid ? [stores dataStoreForIdentifier:uuid] : nil;
  }
  if (made) {
    Stores()[key] = made;
  }
  return made;
}

// Safari's application name, which the page's user agent ends with. Spotify serves WKWebView's
// own user agent, which has none, its phone page, and Safari's its web player.
NSString* SafariName() {
  static NSString* const name = [] {
    NSDictionary* info =
        [NSDictionary dictionaryWithContentsOfFile:@"/Applications/Safari.app/Contents/Info.plist"];
    NSString* version = info[@"CFBundleShortVersionString"] ?: @"26.0";
    return [NSString stringWithFormat:@"Version/%@ Safari/605.1.15", version];
  }();
  return name;
}

// The WebKit page's own keys, which KVO reports.
NSArray<NSString*>* ObservedKeys(WKWebView* web) {
  NSMutableArray<NSString*>* keys = [@[
    @"URL", @"title", @"loading", @"canGoBack", @"canGoForward", @"hasOnlySecureContent"
  ] mutableCopy];
  // WebKit's own, not public: whether the page plays sound, as Safari's speaker shows.
  if ([web respondsToSelector:NSSelectorFromString(@"_isPlayingAudio")]) {
    [keys addObject:@"_isPlayingAudio"];
  }
  return keys;
}

}  // namespace

// The view the window shows in the tab's pane, holding the WebKit page. A page full screen leaves
// it for WebKit's own window and comes back, while this stays in the core's window. AppKit offers
// it every key equivalent in the window before the page has it, as Chromium's page hands the
// browser's shortcuts over (AGENTS.md › Keyboard with the page focused).
@interface BrowserCoreSystemPageView : NSView {
 @public
  browser_core::platform::SystemPageMac* _owner;
}
@end

// WebKit's delegates, which tell the page's C++ side.
@interface BrowserCoreWebKitDelegate : NSObject <WKNavigationDelegate, WKUIDelegate> {
 @public
  browser_core::platform::SystemPageMac* _owner;
}
@end

namespace browser_core::platform {

class SystemPageMac : public SystemPage {
 public:
  SystemPageMac(WKWebsiteDataStore* store, const GURL& url, Delegate* delegate)
      : delegate_(delegate) {
    WKWebViewConfiguration* config = [[WebKitClass(@"WKWebViewConfiguration") alloc] init];
    config.websiteDataStore = store;
    config.applicationNameForUserAgent = SafariName();
    // A video's full screen button takes it full screen in WebKit's own window.
    config.preferences.elementFullscreenEnabled = YES;
    web_ = [[WebKitClass(@"WKWebView") alloc] initWithFrame:NSZeroRect configuration:config];
    web_.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    web_.allowsMagnification = YES;
    if (@available(macOS 13.3, *)) {
      // The user's: Safari's Develop menu inspects it, as DevTools can't.
      web_.inspectable = YES;
    }
    // As Chromium's pages are, for the checks.
    if (base::CommandLine::ForCurrentProcess()->HasSwitch(switches::kMuteAudio)) {
      SetMuted(true);
    }
    delegates_ = [[BrowserCoreWebKitDelegate alloc] init];
    delegates_->_owner = this;
    web_.navigationDelegate = delegates_;
    web_.UIDelegate = delegates_;
    view_ = [[BrowserCoreSystemPageView alloc] initWithFrame:NSZeroRect];
    view_->_owner = this;
    web_.frame = view_.bounds;
    [view_ addSubview:web_];
    // Before KVO: WebKit reports the load starting while the tab is still making the page, and the
    // tab reads the page's state once it has it.
    Load(url);
    for (NSString* key in ObservedKeys(web_)) {
      [web_ addObserver:delegates_ forKeyPath:key options:0 context:nullptr];
    }
  }

  ~SystemPageMac() override {
    for (NSString* key in ObservedKeys(web_)) {
      [web_ removeObserver:delegates_ forKeyPath:key];
    }
    web_.navigationDelegate = nil;
    web_.UIDelegate = nil;
    delegates_->_owner = nullptr;
    view_->_owner = nullptr;
    // Out of full screen and Picture in Picture, whose windows would keep the page.
    [web_ closeAllMediaPresentationsWithCompletionHandler:^{
    }];
    [web_ stopLoading];
    [web_ removeFromSuperview];
    [view_ removeFromSuperview];
    // Let go on the next turn: the tab can go inside a call the page made, such as ⌘W passed on
    // from a key equivalent, with these still on the stack.
    WKWebView* web = web_;
    BrowserCoreSystemPageView* view = view_;
    BrowserCoreWebKitDelegate* delegates = delegates_;
    dispatch_async(dispatch_get_main_queue(), ^{
      [web self];
      [view self];
      [delegates self];
    });
  }

  // SystemPage:
  gfx::NativeView view() override { return gfx::NativeView(view_); }

  void Focus() override {
    NSWindow* window = view_.window;
    if (window && web_.window == window) {
      [window makeFirstResponder:web_];
    }
  }

  GURL url() override { return net::GURLWithNSURL(web_.URL); }
  std::u16string title() override { return base::SysNSStringToUTF16(web_.title ?: @""); }
  bool loading() override { return web_.loading; }
  bool can_go_back() override { return web_.canGoBack; }
  bool can_go_forward() override { return web_.canGoForward; }
  bool secure() override {
    return [web_.URL.scheme isEqualToString:@"https"] && web_.hasOnlySecureContent;
  }
  bool audible() override {
    SEL playing = NSSelectorFromString(@"_isPlayingAudio");
    return [web_ respondsToSelector:playing] &&
           reinterpret_cast<BOOL (*)(id, SEL)>(objc_msgSend)(web_, playing);
  }
  bool muted() override { return muted_; }

  void Load(const GURL& url) override {
    if (NSURL* page = net::NSURLWithGURL(url)) {
      [web_ loadRequest:[NSURLRequest requestWithURL:page]];
    }
  }
  void GoBack() override { [web_ goBack]; }
  void GoForward() override { [web_ goForward]; }
  void Reload(bool bypass_cache) override {
    if (bypass_cache) {
      [web_ reloadFromOrigin];
    } else {
      [web_ reload];
    }
  }
  void Stop() override { [web_ stopLoading]; }

  void SetMuted(bool muted) override {
    // WebKit's own, not public: _WKMediaAudioMuted mutes the whole page, Web Audio included, as
    // Chrome's speaker button does.
    SEL mute = NSSelectorFromString(@"_setPageMuted:");
    if (![web_ respondsToSelector:mute]) {
      return;
    }
    reinterpret_cast<void (*)(id, SEL, NSUInteger)>(objc_msgSend)(web_, mute, muted ? 1 : 0);
    muted_ = muted;
    delegate_->SystemPageChanged();
  }

  // From the views and delegates above.
  void Changed() { delegate_->SystemPageChanged(); }
  void Committed() { Soon(base::BindOnce(&Delegate::SystemPageCommitted)); }
  void Loaded() { Soon(base::BindOnce(&Delegate::SystemPageLoaded)); }
  void Open(const GURL& url) {
    Soon(base::BindOnce([](const GURL& url, Delegate* delegate) { delegate->SystemPageOpen(url); },
                        url));
  }
  // As Chromium's page's click is: the window is part way through the event.
  void ClickedSoon() { Soon(base::BindOnce(&Delegate::SystemPageClicked)); }

  // Whether the page loads |url| in its main frame for a link the user followed: a page of its own
  // site does, and the tab takes another site's.
  bool Follows(const GURL& url) {
    if (net::registry_controlled_domains::SameDomainOrHost(
            url, net::GURLWithNSURL(web_.URL),
            net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES)) {
      return true;
    }
    Soon(base::BindOnce([](const GURL& url, Delegate* delegate) { delegate->SystemPageLeft(url); },
                        url));
    return false;
  }

  bool Key(NSEvent* event, bool before_page) {
    const input::NativeWebKeyboardEvent key{base::apple::OwnedNSEvent(event)};
    // WebKit gives the window's input context only to a page whose focused element takes text,
    // so input methods leave the rest alone.
    return delegate_->SystemPageKey(key, before_page, web_.inputContext != nil);
  }

  bool Focused() {
    NSResponder* responder = view_.window.firstResponder;
    return [responder isKindOfClass:[NSView class]] &&
           [static_cast<NSView*>(responder) isDescendantOf:web_];
  }

  // The content process went: crashed, or killed for memory. WebKit shows nothing until a load.
  void Gone() {
    // check.sh waits for this line, an error as Chromium's tab's is.
    LOG(ERROR) << "browser: system page gone at " << url().spec();
    [web_ reload];
  }

 private:
  // Tells the tab from a task of its own: WebKit is part way through its call, and the tab may let
  // the page go.
  void Soon(base::OnceCallback<void(Delegate*)> call) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<SystemPageMac> page,
                          base::OnceCallback<void(Delegate*)> call) {
                         if (page) {
                           std::move(call).Run(page->delegate_.get());
                         }
                       },
                       weak_factory_.GetWeakPtr(), std::move(call)));
  }

  const raw_ptr<Delegate> delegate_;
  WKWebView* __strong web_;
  BrowserCoreSystemPageView* __strong view_;
  BrowserCoreWebKitDelegate* __strong delegates_;
  bool muted_ = false;
  base::WeakPtrFactory<SystemPageMac> weak_factory_{this};
};

}  // namespace browser_core::platform

@implementation BrowserCoreSystemPageView {
  // The key equivalent last passed to the page, which WebKit sends round again if the page
  // doesn't keep it. Known by when it was typed and what it is: the app may hold it while a bar
  // opens and post it again, and AppKit hands a posted event back as a new one.
  BOOL _sent;
  NSTimeInterval _sentTime;
  unsigned short _sentKey;
  NSEventModifierFlags _sentFlags;
  NSTimeInterval _lastClick;
}

- (BOOL)isFlipped {
  return YES;
}

- (BOOL)performKeyEquivalent:(NSEvent*)event {
  if (!_owner || event.type != NSEventTypeKeyDown || !_owner->Focused()) {
    return [super performKeyEquivalent:event];
  }
  const NSEventModifierFlags flags =
      event.modifierFlags & NSEventModifierFlagDeviceIndependentFlagsMask;
  const bool unkept = _sent && event.timestamp == _sentTime && event.keyCode == _sentKey &&
                      flags == _sentFlags;
  _sent = NO;
  if (_owner->Key(event, /*before_page=*/!unkept)) {
    return YES;
  }
  // The page didn't keep it, and the app didn't use it: the menus get it next.
  if (unkept) {
    return NO;
  }
  _sent = YES;
  _sentTime = event.timestamp;
  _sentKey = event.keyCode;
  _sentFlags = flags;
  return [super performKeyEquivalent:event];
}

// A click in the page, for the split view's focus. The page's view takes the event itself, so it's
// seen as the window finds the view it goes to.
- (NSView*)hitTest:(NSPoint)point {
  NSView* hit = [super hitTest:point];
  NSEvent* event = NSApp.currentEvent;
  if (_owner && hit && event.type == NSEventTypeLeftMouseDown && event.window == self.window &&
      event.timestamp != _lastClick) {
    _lastClick = event.timestamp;
    _owner->ClickedSoon();
  }
  return hit;
}

@end

@implementation BrowserCoreWebKitDelegate

- (void)observeValueForKeyPath:(NSString*)keyPath
                      ofObject:(id)object
                        change:(NSDictionary*)change
                       context:(void*)context {
  if (_owner) {
    _owner->Changed();
  }
}

- (void)webView:(WKWebView*)web
    decidePolicyForNavigationAction:(WKNavigationAction*)action
                    decisionHandler:(void (^)(WKNavigationActionPolicy))decide {
  const GURL url = net::GURLWithNSURL(action.request.URL);
  // Frames load as they do. A new window's link comes to createWebView below.
  if (!_owner || !action.targetFrame.mainFrame) {
    decide(WKNavigationActionPolicyAllow);
    return;
  }
  // No downloads in this engine, nor other apps' schemes, such as mailto: the page goes nowhere.
  if (action.shouldPerformDownload ||
      !(url.SchemeIsHTTPOrHTTPS() || url.SchemeIs("about") || url.SchemeIs("blob"))) {
    LOG(WARNING) << "browser: system page refused " << url.possibly_invalid_spec();
    decide(WKNavigationActionPolicyCancel);
    return;
  }
  // A link the user followed to another site can leave the engine. A redirect, a form and the
  // page's own script stay in it: sign-ins go through other sites.
  if (action.navigationType == WKNavigationTypeLinkActivated && !_owner->Follows(url)) {
    decide(WKNavigationActionPolicyCancel);
    return;
  }
  decide(WKNavigationActionPolicyAllow);
}

- (void)webView:(WKWebView*)web
    decidePolicyForNavigationResponse:(WKNavigationResponse*)response
                      decisionHandler:(void (^)(WKNavigationResponsePolicy))decide {
  // What the page can't show would be a download, which this engine doesn't do.
  if (response.forMainFrame && !response.canShowMIMEType) {
    LOG(WARNING) << "browser: system page refused a download of "
                 << net::GURLWithNSURL(response.response.URL).possibly_invalid_spec();
    decide(WKNavigationResponsePolicyCancel);
    return;
  }
  decide(WKNavigationResponsePolicyAllow);
}

- (void)webView:(WKWebView*)web didCommitNavigation:(WKNavigation*)navigation {
  if (_owner) {
    _owner->Committed();
  }
}

// The main frame's document and what it loads have finished loading, as Chromium's DidFinishLoad
// reports a page's: not a same-document navigation, nor a load that failed.
- (void)webView:(WKWebView*)web didFinishNavigation:(WKNavigation*)navigation {
  if (_owner) {
    _owner->Loaded();
  }
}

- (void)webViewWebContentProcessDidTerminate:(WKWebView*)web {
  if (_owner) {
    _owner->Gone();
  }
}

// A new window opens as a tab of the browser's, at its URL: the page's opener is lost, and a
// site the engine is kept for comes back to it there.
- (WKWebView*)webView:(WKWebView*)web
    createWebViewWithConfiguration:(WKWebViewConfiguration*)config
               forNavigationAction:(WKNavigationAction*)action
                    windowFeatures:(WKWindowFeatures*)features {
  const GURL url = net::GURLWithNSURL(action.request.URL);
  if (_owner && url.SchemeIsHTTPOrHTTPS()) {
    _owner->Open(url);
  }
  return nil;
}

// The camera and the microphone are refused: this engine has no question to ask the user.
- (void)webView:(WKWebView*)web
    requestMediaCapturePermissionForOrigin:(WKSecurityOrigin*)origin
                          initiatedByFrame:(WKFrameInfo*)frame
                                      type:(WKMediaCaptureType)type
                           decisionHandler:(void (^)(WKPermissionDecision))decide {
  decide(WKPermissionDecisionDeny);
}

// Nor does it pick files for the page.
- (void)webView:(WKWebView*)web
    runOpenPanelWithParameters:(WKOpenPanelParameters*)parameters
              initiatedByFrame:(WKFrameInfo*)frame
             completionHandler:(void (^)(NSArray<NSURL*>*))done {
  done(nil);
}

@end

namespace browser_core::platform {

// static
bool SystemPage::Available() {
  if (@available(macOS 14, *)) {
    return true;
  }
  return false;
}

// static
std::unique_ptr<SystemPage> SystemPage::Create(const SystemStore& store,
                                               const GURL& url,
                                               Delegate* delegate) {
  if (!Available() || !LoadWebKit("a page") || !WebKitClass(@"WKWebView")) {
    return nullptr;
  }
  WKWebsiteDataStore* data = StoreFor(store);
  return data ? std::make_unique<SystemPageMac>(data, url, delegate) : nullptr;
}

// static
void SystemPage::DeleteStore(const std::string& id) {
  NSString* key = base::SysUTF8ToNSString(id);
  NSUUID* uuid = [[NSUUID alloc] initWithUUIDString:key];
  if (!uuid || !Available() || !LoadWebKit("deleting a profile's store")) {
    return;
  }
  // Ours lets go first: WebKit won't delete a store in use.
  [Stores() removeObjectForKey:key];
  if (@available(macOS 14, *)) {
    [WebKitClass(@"WKWebsiteDataStore")
        removeDataStoreForIdentifier:uuid
                   completionHandler:^(NSError* error) {
                     if (error) {
                       VLOG(1) << "browser: system store " << base::SysNSStringToUTF8(key)
                               << " not deleted: "
                               << base::SysNSStringToUTF8(error.localizedDescription);
                     }
                   }];
  }
}

// static
void SystemPage::ReleaseStore(const std::string& id) {
  [Stores() removeObjectForKey:base::SysUTF8ToNSString(id)];
}

// static
void SystemPage::ClearSite(const SystemStore& store,
                           const std::string& domain,
                           bool cookies,
                           base::OnceClosure done) {
  WKWebsiteDataStore* data =
      Available() && LoadWebKit("clearing a site") ? StoreFor(store) : nil;
  NSSet<NSString*>* types =
      cookies ? [NSSet setWithObjects:WebKitConstant("WKWebsiteDataTypeCookies"), nil]
              : [NSSet setWithObjects:WebKitConstant("WKWebsiteDataTypeDiskCache"),
                                      WebKitConstant("WKWebsiteDataTypeMemoryCache"), nil];
  if (!data || types.count == 0) {
    std::move(done).Run();
    return;
  }
  __block base::OnceClosure then = std::move(done);
  NSString* site = base::SysUTF8ToNSString(domain);
  [data fetchDataRecordsOfTypes:types
              completionHandler:^(NSArray<WKWebsiteDataRecord*>* records) {
                // WebKit names a record by its site's registrable domain, as the core's filter
                // takes it.
                NSMutableArray<WKWebsiteDataRecord*>* of_site = [NSMutableArray array];
                for (WKWebsiteDataRecord* record in records) {
                  if ([record.displayName isEqualToString:site]) {
                    [of_site addObject:record];
                  }
                }
                [data removeDataOfTypes:types
                         forDataRecords:of_site
                      completionHandler:^{
                        std::move(then).Run();
                      }];
              }];
}

}  // namespace browser_core::platform
