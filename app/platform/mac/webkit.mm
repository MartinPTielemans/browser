// WebKit tabs: a WKWebView in place of a tab's Chromium page, for the sites whose video needs
// FairPlay, which WebKit has and our Chromium can't (docs/decisions.md, decision 9). The core shows
// the view where the page would be and keeps the tab; this keeps the view and tells the core what
// it shows.

#import <AppKit/AppKit.h>
#import <CommonCrypto/CommonDigest.h>
#import <WebKit/WebKit.h>

#include <map>
#include <string>

#include "browser_core.h"
#include "host.h"

@class IloWebKitTab;

namespace {

bool (*g_link)(uint64_t, const char*, int);
bool (*g_key)(uint64_t, const char*, uint32_t, bool);
void (*g_focused)(uint64_t);

std::map<uint64_t, IloWebKitTab*>& Tabs() {
  static std::map<uint64_t, IloWebKitTab*> tabs;
  return tabs;
}

// Kept per profile directory, so every tab of a profile shares one session, as its Chromium tabs
// do.
NSMutableDictionary<NSString*, WKWebsiteDataStore*>* Stores() {
  static NSMutableDictionary* stores = [NSMutableDictionary dictionary];
  return stores;
}

NSMutableDictionary<NSString*, NSData*>* Favicons() {
  static NSMutableDictionary* favicons = [NSMutableDictionary dictionary];
  return favicons;
}

// WebKit names a persistent store by a UUID, so a profile's is made from its directory's path.
NSUUID* StoreId(NSString* path) API_AVAILABLE(macos(14.0)) {
  NSData* bytes = [path dataUsingEncoding:NSUTF8StringEncoding];
  unsigned char digest[CC_SHA256_DIGEST_LENGTH];
  CC_SHA256(bytes.bytes, (CC_LONG)bytes.length, digest);
  digest[6] = (digest[6] & 0x0f) | 0x50;
  digest[8] = (digest[8] & 0x3f) | 0x80;
  return [[NSUUID alloc] initWithUUIDBytes:digest];
}

WKWebsiteDataStore* Store(NSString* path, bool private_session) {
  NSString* key = private_session ? [@"private:" stringByAppendingString:path] : path;
  WKWebsiteDataStore* store = Stores()[key];
  if (store) {
    return store;
  }
  if (private_session) {
    store = [WKWebsiteDataStore nonPersistentDataStore];
  } else if (@available(macOS 14.0, *)) {
    store = [WKWebsiteDataStore dataStoreForIdentifier:StoreId(path)];
  } else {
    // Before macOS 14 WebKit keeps one store for the app, which the profiles then share.
    store = [WKWebsiteDataStore defaultDataStore];
  }
  Stores()[key] = store;
  return store;
}

// The user agent's end as Safari's, which sites check before they offer FairPlay.
NSString* SafariName() {
  NSBundle* safari = [NSBundle bundleWithPath:@"/Applications/Safari.app"];
  NSString* version = safari.infoDictionary[@"CFBundleShortVersionString"] ?: @"26.0";
  return [NSString stringWithFormat:@"Version/%@ Safari/605.1.15", version];
}

// A key as the web names it, for bc_key: "t", "Enter", "ArrowLeft".
NSString* KeyName(NSEvent* event) {
  NSString* characters = event.charactersIgnoringModifiers;
  if (characters.length == 0) {
    return nil;
  }
  unichar c = [characters characterAtIndex:0];
  switch (c) {
    case NSUpArrowFunctionKey:
      return @"ArrowUp";
    case NSDownArrowFunctionKey:
      return @"ArrowDown";
    case NSLeftArrowFunctionKey:
      return @"ArrowLeft";
    case NSRightArrowFunctionKey:
      return @"ArrowRight";
    case NSHomeFunctionKey:
      return @"Home";
    case NSEndFunctionKey:
      return @"End";
    case NSPageUpFunctionKey:
      return @"PageUp";
    case NSPageDownFunctionKey:
      return @"PageDown";
    case NSDeleteFunctionKey:
      return @"Delete";
    case '\r':
    case 3:
      return @"Enter";
    case '\t':
    case 0x19:
      return @"Tab";
    case 0x1b:
      return @"Escape";
    case 0x7f:
      return @"Backspace";
  }
  if (c >= NSF1FunctionKey && c <= NSF12FunctionKey) {
    return [NSString stringWithFormat:@"F%d", c - NSF1FunctionKey + 1];
  }
  return characters;
}

// What a tab reports to the core, the last private: WebKit says whether the page plays sound only
// through its private API.
NSArray<NSString*>* Observed(WKWebView* web) {
  NSArray* keys = @[ @"URL", @"title", @"canGoBack", @"canGoForward", @"loading" ];
  if ([web respondsToSelector:NSSelectorFromString(@"_isPlayingAudio")]) {
    keys = [keys arrayByAddingObject:@"_isPlayingAudio"];
  }
  return keys;
}

// |image| drawn 32 pixels square, as a PNG, for the core's favicon.
NSData* Png(NSImage* image) {
  NSBitmapImageRep* bitmap =
      [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:nullptr
                                              pixelsWide:32
                                              pixelsHigh:32
                                           bitsPerSample:8
                                         samplesPerPixel:4
                                                hasAlpha:YES
                                                isPlanar:NO
                                          colorSpaceName:NSDeviceRGBColorSpace
                                             bytesPerRow:0
                                            bitsPerPixel:0];
  [NSGraphicsContext saveGraphicsState];
  NSGraphicsContext.currentContext = [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
  [image drawInRect:NSMakeRect(0, 0, 32, 32)];
  [NSGraphicsContext restoreGraphicsState];
  return [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
}

uint32_t Modifiers(NSEvent* event) {
  NSEventModifierFlags flags = event.modifierFlags;
  return ((flags & NSEventModifierFlagShift) ? BC_MODIFIER_SHIFT : 0) |
         ((flags & NSEventModifierFlagControl) ? BC_MODIFIER_CONTROL : 0) |
         ((flags & NSEventModifierFlagOption) ? BC_MODIFIER_ALT : 0) |
         ((flags & NSEventModifierFlagCommand) ? BC_MODIFIER_META : 0);
}

}  // namespace

// What the core shows in the page's place. WebKit takes the web view out of its superview for an
// element's full screen and puts it back after, so the core gets this, which stays (as Ardia's
// WebKitTab.swift, GPL-3.0, at v0.2.41, 073bc1a0, keeps its web view in a container).
@interface IloWebKitPage : NSView
@end

@implementation IloWebKitPage
- (BOOL)isFlipped {
  return YES;
}
@end

@interface IloWebView : WKWebView
@property(nonatomic) uint64_t tab;
@end

@implementation IloWebView {
  // The key WebKit was given and may send again, through NSApp, if the page doesn't keep it.
  __weak NSEvent* _offered;
}

- (BOOL)hasKeyboard {
  NSResponder* responder = self.window.firstResponder;
  return [responder isKindOfClass:[NSView class]] && [(NSView*)responder isDescendantOf:self];
}

- (BOOL)offer:(NSEvent*)event reserved:(BOOL)reserved {
  NSString* key = KeyName(event);
  return key && g_key && g_key(_tab, key.UTF8String, Modifiers(event), reserved);
}

// AppKit offers ⌘ and ⌃ keys to every view before the focused one gets them, which Chromium's
// page answers as Chrome does: the browser's reserved shortcuts first, the rest only if the page
// doesn't keep them. WebKit answers the page's later, sending a key it didn't keep through NSApp
// again.
- (BOOL)performKeyEquivalent:(NSEvent*)event {
  IloWebView* keep = self;
  if (![keep hasKeyboard]) {
    return [super performKeyEquivalent:event];
  }
  if (event == _offered) {
    _offered = nil;
    return [keep offer:event reserved:NO] || [super performKeyEquivalent:event];
  }
  bool shortcut = event.modifierFlags & (NSEventModifierFlagCommand | NSEventModifierFlagControl);
  if (shortcut && [keep offer:event reserved:YES]) {
    return YES;
  }
  _offered = event;
  return [super performKeyEquivalent:event];
}

- (BOOL)becomeFirstResponder {
  BOOL became = [super becomeFirstResponder];
  if (became && g_focused) {
    uint64_t tab = _tab;
    dispatch_async(dispatch_get_main_queue(), ^{
      g_focused(tab);
    });
  }
  return became;
}

// WebKit's menu opens links in new windows, which here are tabs (as Ardia's WebKitWebView.swift
// renames them).
- (void)willOpenMenu:(NSMenu*)menu withEvent:(NSEvent*)event {
  [super willOpenMenu:menu withEvent:event];
  NSDictionary* renames = @{
    @"WKMenuItemIdentifierOpenLinkInNewWindow" : @"Open Link in New Tab",
    @"WKMenuItemIdentifierOpenImageInNewWindow" : @"Open Image in New Tab",
    @"WKMenuItemIdentifierOpenMediaInNewWindow" : @"Open Video in New Tab",
    @"WKMenuItemIdentifierOpenFrameInNewWindow" : @"Open Frame in New Tab",
  };
  for (NSMenuItem* item in menu.itemArray) {
    if (NSString* title = renames[item.identifier]) {
      item.title = title;
    }
  }
}
@end

@interface IloWebKitTab : NSObject <WKNavigationDelegate, WKUIDelegate>
@property(nonatomic, readonly) IloWebKitPage* page;
@property(nonatomic, readonly) IloWebView* web;
@end

@implementation IloWebKitTab {
  uint64_t _tab;
  NSData* _favicon;
  NSString* _faviconHost;
  BOOL _reportPosted;
  BOOL _closed;
}

- (instancetype)initWithTab:(uint64_t)tab store:(WKWebsiteDataStore*)store {
  if (!(self = [super init])) {
    return nil;
  }
  _tab = tab;
  WKWebViewConfiguration* configuration = [[WKWebViewConfiguration alloc] init];
  configuration.websiteDataStore = store;
  configuration.applicationNameForUserAgent = SafariName();
  configuration.preferences.elementFullscreenEnabled = YES;
  _page = [[IloWebKitPage alloc] initWithFrame:NSMakeRect(0, 0, 800, 600)];
  _web = [[IloWebView alloc] initWithFrame:_page.bounds configuration:configuration];
  _web.tab = tab;
  _web.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  _web.allowsBackForwardNavigationGestures = YES;
  _web.allowsMagnification = YES;
  _web.navigationDelegate = self;
  _web.UIDelegate = self;
  [_page addSubview:_web];
  for (NSString* key in Observed(_web)) {
    [_web addObserver:self forKeyPath:key options:0 context:nil];
  }
  return self;
}

- (void)close {
  if (_closed) {
    return;
  }
  _closed = YES;
  for (NSString* key in Observed(_web)) {
    [_web removeObserver:self forKeyPath:key];
  }
  _web.navigationDelegate = nil;
  _web.UIDelegate = nil;
  [_web stopLoading];
  // The page can be closing from inside its own key event, as ⌘W, so the view goes after it.
  IloWebKitPage* page = _page;
  IloWebView* web = _web;
  dispatch_async(dispatch_get_main_queue(), ^{
    [web removeFromSuperview];
    [page removeFromSuperview];
  });
}

- (void)setMuted:(bool)muted {
  // WebKit mutes a page's audio only through its private API, as Ardia's WebKitWebView.swift does.
  SEL selector = NSSelectorFromString(@"_setPageMuted:");
  if (![_web respondsToSelector:selector]) {
    fprintf(stderr, "browser: webkit %llu can't be muted\n", _tab);
    return;
  }
  auto set = (void (*)(id, SEL, NSUInteger))[_web methodForSelector:selector];
  set(_web, selector, muted ? 1 : 0);
}

- (void)observeValueForKeyPath:(NSString*)keyPath
                      ofObject:(id)object
                        change:(NSDictionary*)change
                       context:(void*)context {
  if ([keyPath isEqualToString:@"URL"]) {
    [self fetchFavicon];
  }
  [self reportSoon];
}

// Once for what changed together, as a load changes its URL, title and the rest at once.
- (void)reportSoon {
  if (_reportPosted || _closed) {
    return;
  }
  _reportPosted = YES;
  dispatch_async(dispatch_get_main_queue(), ^{
    self->_reportPosted = NO;
    [self report];
  });
}

- (void)report {
  if (_closed) {
    return;
  }
  // Before its first load starts the view has no URL, and the tab keeps the one it had.
  NSString* url = _web.URL.absoluteString;
  if (!url) {
    return;
  }
  bool audible = false;
  if ([_web respondsToSelector:NSSelectorFromString(@"_isPlayingAudio")]) {
    audible = [[_web valueForKey:@"_isPlayingAudio"] boolValue];
  }
  bc_native_page_state state = {
      .url = url.UTF8String,
      .title = (_web.title ?: @"").UTF8String,
      .can_go_back = _web.canGoBack,
      .can_go_forward = _web.canGoForward,
      .loading = _web.loading,
      .audible = audible,
      .favicon = static_cast<const uint8_t*>(_favicon.bytes),
      .favicon_size = _favicon.length,
  };
  bc_tab_set_native_page_state(_tab, &state);
}

// The site's /favicon.ico, as WebKit gives the app no page's icon. Kept per host for the session.
- (void)fetchFavicon {
  NSURL* url = _web.URL;
  NSString* host = url.host;
  if (!host || [host isEqualToString:_faviconHost]) {
    return;
  }
  _faviconHost = host;
  _favicon = Favicons()[host];
  if (_favicon) {
    return;
  }
  NSURLComponents* icon = [[NSURLComponents alloc] init];
  icon.scheme = url.scheme;
  icon.host = host;
  icon.port = url.port;
  icon.path = @"/favicon.ico";
  NSURLSession* session = [NSURLSession
      sessionWithConfiguration:[NSURLSessionConfiguration ephemeralSessionConfiguration]];
  __weak IloWebKitTab* weak = self;
  [[session dataTaskWithURL:icon.URL
          completionHandler:^(NSData* data, NSURLResponse*, NSError*) {
            dispatch_async(dispatch_get_main_queue(), ^{
              NSImage* image = data ? [[NSImage alloc] initWithData:data] : nil;
              NSData* png = image ? Png(image) : nil;
              IloWebKitTab* strong = weak;
              if (!png || !strong) {
                return;
              }
              Favicons()[host] = png;
              if ([strong->_faviconHost isEqualToString:host]) {
                strong->_favicon = png;
                [strong reportSoon];
              }
            });
          }] resume];
  [session finishTasksAndInvalidate];
}

- (void)webView:(WKWebView*)webView
    decidePolicyForNavigationAction:(WKNavigationAction*)action
                    decisionHandler:(void (^)(WKNavigationActionPolicy))decisionHandler {
  NSString* scheme = action.request.URL.scheme.lowercaseString;
  bool web = [scheme isEqualToString:@"http"] || [scheme isEqualToString:@"https"];
  if (!web || !action.targetFrame.isMainFrame ||
      action.navigationType != WKNavigationTypeLinkActivated) {
    decisionHandler(WKNavigationActionPolicyAllow);
    return;
  }
  NSEventModifierFlags flags = action.modifierFlags;
  int where = HOST_LINK_HERE;
  if (action.buttonNumber == 2 || (flags & NSEventModifierFlagCommand)) {
    where = (flags & NSEventModifierFlagShift) ? HOST_LINK_NEW_TAB : HOST_LINK_BEHIND;
  }
  bool taken = g_link && g_link(_tab, action.request.URL.absoluteString.UTF8String, where);
  decisionHandler(taken ? WKNavigationActionPolicyCancel : WKNavigationActionPolicyAllow);
}

- (void)webView:(WKWebView*)webView
    decidePolicyForNavigationResponse:(WKNavigationResponse*)response
                      decisionHandler:(void (^)(WKNavigationResponsePolicy))decisionHandler {
  // A file the page can't show would be a download, which WebKit tabs leave to Chromium's tabs.
  decisionHandler(response.canShowMIMEType ? WKNavigationResponsePolicyAllow
                                           : WKNavigationResponsePolicyCancel);
}

- (void)webViewWebContentProcessDidTerminate:(WKWebView*)webView {
  fprintf(stderr, "browser: webkit %llu page process ended\n", _tab);
  [webView reload];
}

// A page's window.open, or WebKit's Open Link in New Window, opens a tab, which has no opener.
- (WKWebView*)webView:(WKWebView*)webView
    createWebViewWithConfiguration:(WKWebViewConfiguration*)configuration
               forNavigationAction:(WKNavigationAction*)action
                    windowFeatures:(WKWindowFeatures*)features {
  NSString* url = action.request.URL.absoluteString;
  if (url.length && g_link) {
    g_link(_tab, url.UTF8String, HOST_LINK_NEW_TAB);
  }
  return nil;
}

- (void)webViewDidClose:(WKWebView*)webView {
}
@end

namespace {

IloWebKitTab* TabOf(uint64_t tab) {
  auto found = Tabs().find(tab);
  return found == Tabs().end() ? nil : found->second;
}

}  // namespace

void host_webkit_start(bool (*link)(uint64_t, const char*, int),
                       bool (*key)(uint64_t, const char*, uint32_t, bool),
                       void (*focused)(uint64_t)) {
  g_link = link;
  g_key = key;
  g_focused = focused;
}

void host_webkit_show(uint64_t tab,
                      const char* profile_path,
                      bool private_session,
                      const char* url) {
  IloWebKitTab* page = TabOf(tab);
  if (!page) {
    WKWebsiteDataStore* store = Store(@(profile_path), private_session);
    page = [[IloWebKitTab alloc] initWithTab:tab store:store];
    Tabs()[tab] = page;
    bc_tab_set_native_page(tab, (__bridge void*)page.page);
  }
  NSURL* address = [NSURL URLWithString:@(url)];
  if (address) {
    [page.web loadRequest:[NSURLRequest requestWithURL:address]];
  }
}

bool host_webkit_has(uint64_t tab) {
  return TabOf(tab) != nil;
}

bool host_webkit_go(uint64_t tab, int step) {
  WKWebView* web = TabOf(tab).web;
  if (step < 0 && web.canGoBack) {
    [web goBack];
    return true;
  }
  if (step > 0 && web.canGoForward) {
    [web goForward];
    return true;
  }
  return false;
}

void host_webkit_reload(uint64_t tab, bool bypass_cache) {
  WKWebView* web = TabOf(tab).web;
  bypass_cache ? [web reloadFromOrigin] : [web reload];
}

void host_webkit_stop(uint64_t tab) {
  [TabOf(tab).web stopLoading];
}

void host_webkit_set_muted(uint64_t tab, bool muted) {
  [TabOf(tab) setMuted:muted];
}

void host_webkit_close(uint64_t tab) {
  IloWebKitTab* page = TabOf(tab);
  Tabs().erase(tab);
  [page close];
}

void host_webkit_clear_site(uint64_t tab, bool cookies) {
  WKWebView* web = TabOf(tab).web;
  NSString* host = web.URL.host.lowercaseString;
  if (!host) {
    return;
  }
  WKWebsiteDataStore* store = web.configuration.websiteDataStore;
  NSMutableSet* types = [[WKWebsiteDataStore allWebsiteDataTypes] mutableCopy];
  if (!cookies) {
    [types removeObject:WKWebsiteDataTypeCookies];
  }
  [store fetchDataRecordsOfTypes:types
               completionHandler:^(NSArray<WKWebsiteDataRecord*>* records) {
                 // A record is named for the site, the registrable domain, such as netflix.com.
                 NSPredicate* site = [NSPredicate
                     predicateWithBlock:^BOOL(WKWebsiteDataRecord* record, NSDictionary*) {
                       NSString* name = record.displayName.lowercaseString;
                       return [host isEqualToString:name] ||
                              [host hasSuffix:[@"." stringByAppendingString:name]];
                     }];
                 [store removeDataOfTypes:types
                           forDataRecords:[records filteredArrayUsingPredicate:site]
                        completionHandler:^{
                          [web reload];
                        }];
               }];
}

void host_webkit_end_private(const char* profile_path) {
  [Stores() removeObjectForKey:[@"private:" stringByAppendingString:@(profile_path)]];
}

void host_webkit_delete_profile(const char* profile_path) {
  NSString* path = @(profile_path);
  [Stores() removeObjectForKey:path];
  if (@available(macOS 14.0, *)) {
    [WKWebsiteDataStore
        removeDataStoreForIdentifier:StoreId(path)
                   completionHandler:^(NSError* error) {
                     if (error) {
                       fprintf(stderr, "browser: webkit store of %s stays: %s\n", path.UTF8String,
                               error.localizedDescription.UTF8String);
                     }
                   }];
  }
}
