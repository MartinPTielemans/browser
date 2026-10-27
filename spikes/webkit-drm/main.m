// One window holding a WKWebView, to learn whether WebKit in an app that isn't Safari plays
// FairPlay-protected video, and what streaming sites show it. Every EME request a page makes, what
// WebKit answers, and each video's events are logged to stderr.
//
// SPIKE_URL        the page (default: the probe alone, on about:blank)
// SPIKE_UA=safari  append Safari's application name to the user agent; WKWebView's own ends at
//                  "(KHTML, like Gecko)"
// SPIKE_STORE      name of the persistent website data store (default "default"); one per profile
// SPIKE_DISPLAY    "builtin" (default) or "main"
// SPIKE_ACTIVATE=1 bring the app to the front, for a person to use it
// SPIKE_SOUND=1     let pages make sound; without it the page is muted
// SPIKE_AUTOPLAY=1  let media start without a click, as the stream test's play() needs
// SPIKE_IGNORE_OCCLUSION=1 keep painting the page while other windows cover it
// Arguments: "forget" deletes the store SPIKE_STORE names and exits.
// stdin: "js <code>" evaluates in the page and logs the result, "load <url>", "quit".
#import <Cocoa/Cocoa.h>
#import <CommonCrypto/CommonDigest.h>
#import <WebKit/WebKit.h>
#import <objc/message.h>

static NSDate *gStart;

static void Log(NSString *format, ...) {
  va_list args;
  va_start(args, format);
  NSString *line = [[NSString alloc] initWithFormat:format arguments:args];
  va_end(args);
  fprintf(stderr, "[%7.3f] %s\n", -gStart.timeIntervalSinceNow, line.UTF8String);
}

static NSString *Env(const char *name, NSString *fallback) {
  const char *value = getenv(name);
  return value && *value ? @(value) : fallback;
}

// dataStoreForIdentifier: takes a UUID; a name hashes to a stable one, so a profile keeps its store.
static NSUUID *StoreID(NSString *name) {
  unsigned char digest[CC_SHA256_DIGEST_LENGTH];
  NSData *data = [name dataUsingEncoding:NSUTF8StringEncoding];
  CC_SHA256(data.bytes, (CC_LONG)data.length, digest);
  digest[6] = (digest[6] & 0x0f) | 0x50;
  digest[8] = (digest[8] & 0x3f) | 0x80;
  return [[NSUUID alloc] initWithUUIDBytes:digest];
}

static NSString *SafariVersion(void) {
  NSDictionary *info =
      [NSDictionary dictionaryWithContentsOfFile:@"/Applications/Safari.app/Contents/Info.plist"];
  return info[@"CFBundleShortVersionString"] ?: @"26.0";
}


@interface Spike : NSObject <NSApplicationDelegate, WKScriptMessageHandler, WKNavigationDelegate,
                             WKUIDelegate>
@property(strong) NSWindow *window;
@property(strong) WKWebView *web;
@end

@implementation Spike

- (void)applicationDidFinishLaunching:(NSNotification *)note {
  NSString *storeName = Env("SPIKE_STORE", @"default");
  WKWebViewConfiguration *config = [WKWebViewConfiguration new];
  config.websiteDataStore = [WKWebsiteDataStore dataStoreForIdentifier:StoreID(storeName)];
  BOOL sound = [Env("SPIKE_SOUND", @"") isEqualToString:@"1"];
  config.mediaTypesRequiringUserActionForPlayback =
      [Env("SPIKE_AUTOPLAY", @"") isEqualToString:@"1"] ? WKAudiovisualMediaTypeNone
                                                       : WKAudiovisualMediaTypeAll;
  config.preferences.elementFullscreenEnabled = YES;
  if ([Env("SPIKE_UA", @"") isEqualToString:@"safari"]) {
    config.applicationNameForUserAgent =
        [NSString stringWithFormat:@"Version/%@ Safari/605.1.15", SafariVersion()];
  }
  NSString *hook = [NSString stringWithContentsOfFile:[NSBundle.mainBundle pathForResource:@"hook"
                                                                                ofType:@"js"]
                                             encoding:NSUTF8StringEncoding
                                                error:nil];
  [config.userContentController
      addUserScript:[[WKUserScript alloc] initWithSource:hook
                                           injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                        forMainFrameOnly:NO]];
  [config.userContentController addScriptMessageHandler:self name:@"spike"];
  // The owner works next to the spike; a page that plays stays silent unless asked.
  if (!sound) {
    NSString *mute = @"new MutationObserver(() => document.querySelectorAll('video,audio')"
                      ".forEach(m => m.muted = true)).observe(document, {childList: true, "
                      "subtree: true}); document.addEventListener('play', e => e.target.muted = "
                      "true, true);";
    [config.userContentController
        addUserScript:[[WKUserScript alloc] initWithSource:mute
                                             injectionTime:WKUserScriptInjectionTimeAtDocumentStart
                                          forMainFrameOnly:NO]];
  }

  NSScreen *screen = NSScreen.mainScreen;
  if (![Env("SPIKE_DISPLAY", @"builtin") isEqualToString:@"main"]) {
    for (NSScreen *s in NSScreen.screens) {
      if (CGDisplayIsBuiltin([s.deviceDescription[@"NSScreenNumber"] unsignedIntValue])) screen = s;
    }
  }
  NSRect visible = screen.visibleFrame;
  NSSize size = NSMakeSize(MIN(1280, visible.size.width), MIN(800, visible.size.height));
  NSRect frame = NSMakeRect(NSMidX(visible) - size.width / 2, NSMidY(visible) - size.height / 2,
                            size.width, size.height);
  self.window = [[NSWindow alloc]
      initWithContentRect:frame
                styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                          NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable
                  backing:NSBackingStoreBuffered
                    defer:NO];
  [self.window setFrame:frame display:NO];
  self.window.title = [NSString stringWithFormat:@"WebKit DRM spike — store %@", storeName];
  self.web = [[WKWebView alloc] initWithFrame:NSZeroRect configuration:config];
  self.web.navigationDelegate = self;
  self.web.UIDelegate = self;
  self.web.inspectable = YES;
  // A covered window's page is hidden to WebKit, which stops painting it and its video. The
  // spike runs behind the owner's windows, so it turns that off (WebKit SPI, via KVC).
  if ([Env("SPIKE_IGNORE_OCCLUSION", @"") isEqualToString:@"1"]) {
    [self.web setValue:@NO forKey:@"windowOcclusionDetectionEnabled"];
  }
  self.window.contentView = self.web;
  SEL muteSel = NSSelectorFromString(@"_setPageMuted:");
  if (!sound && [self.web respondsToSelector:muteSel]) {
    // _WKMediaAudioMuted, WebKit SPI: mutes the whole page, Web Audio included.
    ((void (*)(id, SEL, NSUInteger))objc_msgSend)(self.web, muteSel, 1);
  }
  Log(@"sound %@", sound                                    ? @"on"
                   : [self.web respondsToSelector:muteSel] ? @"muted (page)"
                                                           : @"muted (script)");
  [self.window orderFrontRegardless];
  if ([Env("SPIKE_ACTIVATE", @"") isEqualToString:@"1"]) {
    [NSApp activate];
    [self.window makeKeyAndOrderFront:nil];
  }
  Log(@"store %@ (%@), ua %@", storeName, StoreID(storeName).UUIDString,
      config.applicationNameForUserAgent ?: @"WKWebView default");
  [self load:Env("SPIKE_URL", @"about:blank")];
  [NSThread detachNewThreadSelector:@selector(readCommands) toTarget:self withObject:nil];
}

- (void)load:(NSString *)url {
  [self.web loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:url]]];
}

- (void)readCommands {
  char line[65536];
  while (fgets(line, sizeof line, stdin)) {
    NSString *command = [@(line) stringByTrimmingCharactersInSet:NSCharacterSet.newlineCharacterSet];
    dispatch_async(dispatch_get_main_queue(), ^{
      if ([command hasPrefix:@"js "]) {
        [self.web evaluateJavaScript:[command substringFromIndex:3]
                   completionHandler:^(id result, NSError *error) {
                     Log(@"js -> %@", error ? error.localizedDescription : result);
                   }];
      } else if ([command hasPrefix:@"load "]) {
        [self load:[command substringFromIndex:5]];
      } else if ([command isEqualToString:@"quit"]) {
        [NSApp terminate:nil];
      }
    });
  }
}

- (void)userContentController:(WKUserContentController *)controller
      didReceiveScriptMessage:(WKScriptMessage *)message {
  NSData *json = [NSJSONSerialization dataWithJSONObject:message.body
                                                 options:NSJSONWritingSortedKeys
                                                   error:nil];
  Log(@"page %@", json ? [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding]
                       : message.body);
}

- (void)webView:(WKWebView *)web didFinishNavigation:(WKNavigation *)navigation {
  Log(@"loaded %@ \"%@\"", web.URL, web.title);
}

- (void)webView:(WKWebView *)web
    didFailProvisionalNavigation:(WKNavigation *)navigation
                       withError:(NSError *)error {
  Log(@"failed %@: %@", web.URL, error.localizedDescription);
}

- (void)webViewWebContentProcessDidTerminate:(WKWebView *)web {
  Log(@"web content process terminated");
}

// Links that open a new window load here instead; the spike has one view.
- (WKWebView *)webView:(WKWebView *)web
    createWebViewWithConfiguration:(WKWebViewConfiguration *)config
               forNavigationAction:(WKNavigationAction *)action
                    windowFeatures:(WKWindowFeatures *)features {
  [web loadRequest:action.request];
  return nil;
}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)app {
  return YES;
}

@end

// Edit's items give text fields ⌘C and ⌘V, which a person signing in needs.
static void BuildMenu(void) {
  NSMenu *bar = [NSMenu new];
  NSMenu *app = [NSMenu new];
  [app addItemWithTitle:@"Quit" action:@selector(terminate:) keyEquivalent:@"q"];
  NSMenu *edit = [[NSMenu alloc] initWithTitle:@"Edit"];
  [edit addItemWithTitle:@"Undo" action:@selector(undo:) keyEquivalent:@"z"];
  [edit addItemWithTitle:@"Cut" action:@selector(cut:) keyEquivalent:@"x"];
  [edit addItemWithTitle:@"Copy" action:@selector(copy:) keyEquivalent:@"c"];
  [edit addItemWithTitle:@"Paste" action:@selector(paste:) keyEquivalent:@"v"];
  [edit addItemWithTitle:@"Select All" action:@selector(selectAll:) keyEquivalent:@"a"];
  for (NSMenu *menu in @[ app, edit ]) {
    NSMenuItem *item = [NSMenuItem new];
    item.submenu = menu;
    [bar addItem:item];
  }
  NSApp.mainMenu = bar;
}

int main(int argc, const char *argv[]) {
  @autoreleasepool {
    gStart = [NSDate date];
    if (argc > 1 && strcmp(argv[1], "forget") == 0) {
      NSString *name = Env("SPIKE_STORE", @"default");
      [WKWebsiteDataStore removeDataStoreForIdentifier:StoreID(name)
                                     completionHandler:^(NSError *error) {
                                       Log(@"forget %@: %@", name,
                                           error ? error.localizedDescription : @"done");
                                       exit(error ? 1 : 0);
                                     }];
      [[NSRunLoop mainRunLoop] run];
    }
    [NSApplication sharedApplication];
    NSApp.activationPolicy = NSApplicationActivationPolicyRegular;
    BuildMenu();
    Spike *spike = [Spike new];
    NSApp.delegate = spike;
    [NSApp run];
  }
  return 0;
}
