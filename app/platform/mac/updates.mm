// Updates on the Mac (docs/ux-spec.md › Updates): Sparkle 2, from a release build that carries it
// with its feed and key, loaded once the first window shows so launch never waits for it. Around it,
// what keeps an update quick and whole: macOS's check of a new version runs while the user waits for
// the restart, not on its first launch (#389), and Chromium never starts without its cookie key
// after a change of signature.

#import <Cocoa/Cocoa.h>
#import <Security/Security.h>
#import <Sparkle/Sparkle.h>

#include <signal.h>

#include "../host.h"

namespace {

// Ardia's: after 90 s a check that hasn't finished never will, and Install and Relaunch waits for
// one at most 15 s.
constexpr int64_t kPrewarmLimit = 90;
constexpr int64_t kInstallWait = 15;
// The release notes of the update being installed: its version and their URL.
NSString* const kNotes = @"UpdateNotes";
// Settings › General's Install updates automatically: a ready update installs while the Mac is
// idle. Off unless the user turns it on; Sparkle's own setting only downloads it.
NSString* const kIdleInstall = @"InstallUpdatesWhileIdle";
// The designated requirement Chromium last read its cookie key under.
NSString* const kKeySignature = @"KeychainKeySignature";

void Log(NSString* line) {
  fprintf(stderr, "browser: %s\n", line.UTF8String);
}

// The running app's designated requirement, which macOS grants Keychain items to: alike across
// versions signed by the same Developer ID, new for another identity, and for every ad hoc build.
SecRequirementRef CopyOwnRequirement() {
  SecCodeRef running = nullptr;
  SecStaticCodeRef code = nullptr;
  SecRequirementRef requirement = nullptr;
  if (SecCodeCopySelf(kSecCSDefaultFlags, &running) == errSecSuccess &&
      SecCodeCopyStaticCode(running, kSecCSDefaultFlags, &code) == errSecSuccess) {
    SecCodeCopyDesignatedRequirement(code, kSecCSDefaultFlags, &requirement);
  }
  if (code) CFRelease(code);
  if (running) CFRelease(running);
  return requirement;
}

NSString* OwnSignature() {
  SecRequirementRef requirement = CopyOwnRequirement();
  if (!requirement) return nil;
  CFStringRef text = nullptr;
  SecRequirementCopyString(requirement, kSecCSDefaultFlags, &text);
  CFRelease(requirement);
  return CFBridgingRelease(text);
}

// Reads the cookie key as Chromium will, so macOS asks for it now if it's going to. The key itself
// is dropped at once.
OSStatus ReadKey(NSString* name) {
  NSDictionary* query = @{
    (__bridge id)kSecClass : (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrService : [name stringByAppendingString:@" Safe Storage"],
    (__bridge id)kSecAttrAccount : name,
    (__bridge id)kSecReturnData : @YES,
  };
  CFTypeRef key = nullptr;
  OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &key);
  if (key) CFRelease(key);
  return status;
}

bool AnyFullScreen() {
  for (NSWindow* window in NSApp.windows) {
    if (window.styleMask & NSWindowStyleMaskFullScreen) return true;
  }
  return false;
}

// Where Sparkle unpacks an update before installing it, as Sparkle 2.10's installer does: the app
// is a few folders down in ~/Library/Caches/<bundle id>/org.sparkle-project.Sparkle/Installation.
NSBundle* StagedApp(NSString* version) {
  NSString* bundle = NSBundle.mainBundle.bundleIdentifier;
  NSURL* caches = [NSFileManager.defaultManager URLsForDirectory:NSCachesDirectory
                                                       inDomains:NSUserDomainMask]
                      .firstObject;
  NSURL* root = [[caches URLByAppendingPathComponent:bundle]
      URLByAppendingPathComponent:@"org.sparkle-project.Sparkle/Installation"];
  NSDirectoryEnumerator* walk = [NSFileManager.defaultManager
                 enumeratorAtURL:root
      includingPropertiesForKeys:nil
                         options:NSDirectoryEnumerationSkipsHiddenFiles |
                                 NSDirectoryEnumerationSkipsPackageDescendants
                    errorHandler:nil];
  for (NSURL* url in walk) {
    if (walk.level > 4) {
      [walk skipDescendants];
      continue;
    }
    if (![url.pathExtension isEqual:@"app"]) continue;
    NSBundle* app = [NSBundle bundleWithURL:url];
    if ([app.bundleIdentifier isEqual:bundle] &&
        [[app objectForInfoDictionaryKey:@"CFBundleVersion"] isEqual:version]) {
      return app;
    }
  }
  return nil;
}

// Whether |app| is signed as this app is, whole: what's run from it is only ever ours. Reading every
// file's hash is also most of what macOS does before a new version's first launch.
bool SignedLikeThisApp(NSURL* app) {
  SecRequirementRef requirement = CopyOwnRequirement();
  SecStaticCodeRef code = nullptr;
  bool alike = requirement &&
               SecStaticCodeCreateWithPath((__bridge CFURLRef)app, kSecCSDefaultFlags, &code) ==
                   errSecSuccess &&
               SecStaticCodeCheckValidity(code,
                                          kSecCSStrictValidate | kSecCSCheckNestedCode |
                                              kSecCSCheckAllArchitectures,
                                          requirement) == errSecSuccess;
  if (code) CFRelease(code);
  if (requirement) CFRelease(requirement);
  return alike;
}

// What host_watch_updates gave.
void (*g_ready)(const char* version);
bool (*g_quiet)(void);

}  // namespace

// Sparkle's delegate, for both its updater and its alerts.
@interface Updates : NSObject <SPUUpdaterDelegate, SPUStandardUserDriverDelegate>
@property(nonatomic) SPUStandardUpdaterController* controller;
// A scheduled update's alert held back while a window was full screen.
@property(nonatomic) BOOL waiting;
// The update downloaded to install on quit, and what installs it now.
@property(nonatomic) NSString* ready;
@property(nonatomic, copy) void (^installReady)(void);
@end

@implementation Updates {
  // Versions whose check macOS has done, the one it's doing and what waits for it.
  NSMutableSet<NSString*>* _warmed;
  NSString* _warming;
  NSTask* _task;
  NSMutableArray<dispatch_block_t>* _afterWarming;
}

- (instancetype)init {
  if ((self = [super init])) {
    _warmed = [NSMutableSet set];
    _afterWarming = [NSMutableArray array];
    NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
    [center addObserver:self
               selector:@selector(leftFullScreen:)
                   name:NSWindowDidExitFullScreenNotification
                 object:nil];
    [center addObserver:self
               selector:@selector(terminating:)
                   name:NSApplicationWillTerminateNotification
                 object:nil];
    // The Mac going idle, when a ready update can install without the user waiting for it.
    NSDistributedNotificationCenter* system = NSDistributedNotificationCenter.defaultCenter;
    for (NSString* name in @[ @"com.apple.screenIsLocked", @"com.apple.screensaver.didstart" ]) {
      [system addObserver:self selector:@selector(idle:) name:name object:nil];
    }
  }
  return self;
}

// The release notes the first launch of |item| opens, kept until then.
- (void)keepNotes:(SUAppcastItem*)item {
  NSURL* url = item.fullReleaseNotesURL ?: item.releaseNotesURL;
  if (![url.scheme isEqual:@"https"]) return;
  [NSUserDefaults.standardUserDefaults
      setObject:@{@"version" : item.versionString, @"url" : url.absoluteString}
         forKey:kNotes];
}

// Runs the staged copy of |version| for a moment, with BROWSER_PREWARM, which ends it before it
// does anything, so macOS checks it now, as it would before its first launch. Then |then|.
- (void)warm:(NSString*)version then:(dispatch_block_t)then {
  if ([_warmed containsObject:version]) {
    if (then) then();
    return;
  }
  if (then) [_afterWarming addObject:then];
  if ([version isEqual:_warming]) return;
  _warming = version;
  NSDate* began = NSDate.date;
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
    NSBundle* app = StagedApp(version);
    NSString* outcome = nil;
    if (!app) {
      outcome = @"not found";
    } else if (!SignedLikeThisApp(app.bundleURL)) {
      outcome = @"not signed as this app is";
    } else {
      NSTask* task = [[NSTask alloc] init];
      task.executableURL = app.executableURL;
      NSMutableDictionary* environment = [NSProcessInfo.processInfo.environment mutableCopy];
      environment[@"BROWSER_PREWARM"] = @"1";
      task.environment = environment;
      task.standardInput = NSFileHandle.fileHandleWithNullDevice;
      task.standardOutput = NSFileHandle.fileHandleWithNullDevice;
      task.standardError = NSFileHandle.fileHandleWithNullDevice;
      NSError* error = nil;
      if ([task launchAndReturnError:&error]) {
        dispatch_sync(dispatch_get_main_queue(), ^{
          self->_task = task;
        });
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, kPrewarmLimit * NSEC_PER_SEC),
                       dispatch_get_main_queue(), ^{
                         if (task.running) kill(task.processIdentifier, SIGKILL);
                       });
        [task waitUntilExit];
        outcome = [NSString stringWithFormat:@"status %d", task.terminationStatus];
      } else {
        outcome = error.localizedDescription;
      }
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      Log([NSString stringWithFormat:@"update %@ prewarmed in %.0f ms, %@", version,
                                     -began.timeIntervalSinceNow * 1000, outcome]);
      self->_task = nil;
      self->_warming = nil;
      [self->_warmed addObject:version];
      NSArray<dispatch_block_t>* waiting = self->_afterWarming;
      self->_afterWarming = [NSMutableArray array];
      for (dispatch_block_t next in waiting) next();
    });
  });
}

- (void)terminating:(NSNotification*)notification {
  if (_task.running) kill(_task.processIdentifier, SIGKILL);
}

// An update downloaded in the background, to install when the app quits: macOS checks it now,
// while the user goes on browsing, and the sidebar offers to restart into it. Sparkle installs it
// on quit all the same.
- (BOOL)updater:(SPUUpdater*)updater
    willInstallUpdateOnQuit:(SUAppcastItem*)item
    immediateInstallationBlock:(void (^)(void))immediateInstallHandler {
  [self keepNotes:item];
  [self warm:item.versionString then:nil];
  self.ready = item.versionString;
  self.installReady = immediateInstallHandler;
  Log([NSString stringWithFormat:@"update %@ ready", item.versionString]);
  if (g_ready) g_ready(item.versionString.UTF8String);
  return YES;
}

// Installs the ready update and relaunches, once macOS has checked it, waiting at most 15 s, as
// Install and Relaunch does.
- (void)installNow {
  void (^install)(void) = self.installReady;
  if (!install) return;
  self.installReady = nil;
  __block BOOL installed = NO;
  dispatch_block_t once = ^{
    if (installed) return;
    installed = YES;
    install();
  };
  [self warm:self.ready then:once];
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, kInstallWait * NSEC_PER_SEC),
                 dispatch_get_main_queue(), once);
}

// The screen locked or its screensaver started: the user is away, so a ready update installs, as
// they set it to, unless it would cut off what they left playing or showing full screen.
- (void)idle:(NSNotification*)notification {
  if (!self.installReady) {
    Log(@"update idle: none ready");
    return;
  }
  NSString* outcome = @"installs";
  if (![NSUserDefaults.standardUserDefaults boolForKey:kIdleInstall]) {
    outcome = @"waits: setting off";
  } else if (AnyFullScreen()) {
    outcome = @"waits: full screen";
  } else if (g_quiet && !g_quiet()) {
    outcome = @"waits: a page plays sound";
  }
  Log([NSString stringWithFormat:@"update %@ idle %@", self.ready, outcome]);
  if ([outcome isEqual:@"installs"]) [self installNow];
}

// A driven run's app is the check's own, which a relaunch would open outside the run: with no
// driver, in the dev build's profile and on the main display. It installs all the same.
- (BOOL)updaterShouldRelaunchApplication:(SPUUpdater*)updater {
  BOOL driven = getenv("BROWSER_DRIVE") != nullptr;
  NSString* outcome = driven ? @"relaunch held: driven" : @"relaunches";
  Log([NSString stringWithFormat:@"update %@ %@", self.ready, outcome]);
  return !driven;
}

- (void)updater:(SPUUpdater*)updater didFindValidUpdate:(SUAppcastItem*)item {
  Log([NSString stringWithFormat:@"update %@ found", item.versionString]);
}

// A check that finds no update says why, with Sparkle's reason (SPUNoUpdateFoundReason) and the
// newest build the feed has, as Sparkle is only loaded, not linked, its keys are spelled out here.
- (void)updaterDidNotFindUpdate:(SPUUpdater*)updater error:(NSError*)error {
  SUAppcastItem* latest = error.userInfo[@"SULatestAppcastItemFound"];
  Log([NSString stringWithFormat:@"update none: reason %@, feed's newest %@, running %@",
                                 error.userInfo[@"SUNoUpdateFoundReason"],
                                 latest.versionString ?: @"none",
                                 [NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleVersion"]]);
}

// Any other end to a check, such as a feed that doesn't parse, with what Sparkle says went wrong.
- (void)updater:(SPUUpdater*)updater didAbortWithError:(NSError*)error {
  // SUNoUpdateError, which updaterDidNotFindUpdate:error: has told.
  if (error.code == 1001) return;
  NSError* cause = error.userInfo[NSUnderlyingErrorKey];
  Log([NSString stringWithFormat:@"update failed: %@ %ld, %@%@", error.domain, (long)error.code,
                                 error.localizedDescription,
                                 cause ? [@" " stringByAppendingString:cause.description] : @""]);
}

- (void)updater:(SPUUpdater*)updater willScheduleUpdateCheckAfterDelay:(NSTimeInterval)delay {
  Log([NSString stringWithFormat:@"update check in %.0f s", delay]);
}

- (void)updater:(SPUUpdater*)updater willInstallUpdate:(SUAppcastItem*)item {
  [self keepNotes:item];
}

// Install and Relaunch: the restart waits for macOS's check of the new version, up to 15 s.
- (BOOL)updater:(SPUUpdater*)updater
    shouldPostponeRelaunchForUpdate:(SUAppcastItem*)item
                 untilInvokingBlock:(void (^)(void))installHandler {
  [self keepNotes:item];
  if ([_warmed containsObject:item.versionString]) return NO;
  __block BOOL installed = NO;
  dispatch_block_t install = ^{
    if (installed) return;
    installed = YES;
    installHandler();
  };
  [self warm:item.versionString then:install];
  dispatch_after(dispatch_time(DISPATCH_TIME_NOW, kInstallWait * NSEC_PER_SEC),
                 dispatch_get_main_queue(), install);
  return YES;
}

// A scheduled update's alert waits while a window is full screen (ux-spec), and shows when none is.
- (BOOL)supportsGentleScheduledUpdateReminders {
  return YES;
}

- (BOOL)standardUserDriverShouldHandleShowingScheduledUpdate:(SUAppcastItem*)update
                                         andInImmediateFocus:(BOOL)immediateFocus {
  return !AnyFullScreen();
}

- (void)standardUserDriverWillHandleShowingUpdate:(BOOL)handleShowingUpdate
                                        forUpdate:(SUAppcastItem*)update
                                            state:(SPUUserUpdateState*)state {
  self.waiting = !handleShowingUpdate;
  NSString* outcome = handleShowingUpdate ? @"shown" : @"held in full screen";
  Log([NSString stringWithFormat:@"update %@ %@", update.versionString, outcome]);
}

- (void)standardUserDriverDidReceiveUserAttentionForUpdate:(SUAppcastItem*)update {
  Log([NSString stringWithFormat:@"update %@ alert has the keyboard", update.versionString]);
}

// The notes the alert shows, as they are: the log has their first line.
- (NSAttributedString*)standardUserDriverWillShowReleaseNotesText:(NSAttributedString*)notes
                                                        forUpdate:(SUAppcastItem*)update
                                         withBundleDisplayVersion:(NSString*)displayVersion
                                                    bundleVersion:(NSString*)version {
  NSString* first = [notes.string componentsSeparatedByString:@"\n"].firstObject;
  Log([NSString stringWithFormat:@"update %@ notes %@", update.versionString, first]);
  return nil;
}

- (void)standardUserDriverWillFinishUpdateSession {
  self.waiting = NO;
}

- (void)leftFullScreen:(NSNotification*)notification {
  // AppKit takes the style away just after this, so the window that left is looked at next turn.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!self.waiting || AnyFullScreen()) return;
    self.waiting = NO;
    Log(@"update shown after full screen");
    [self.controller checkForUpdates:nil];
  });
}

@end

namespace {
Updates* g_updates;
}

bool host_keep_keychain_key(void) {
  NSString* name = [NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleName"];
  NSString* signature = OwnSignature();
  NSUserDefaults* defaults = NSUserDefaults.standardUserDefaults;
  // Under a signature the key was read under, macOS doesn't ask, so launch doesn't wait for it.
  if (name.length == 0 || !signature || [[defaults stringForKey:kKeySignature] isEqual:signature]) {
    return true;
  }
  for (;;) {
    OSStatus status = ReadKey(name);
    // Not there: a first launch, and Chromium makes one.
    if (status == errSecSuccess || status == errSecItemNotFound) {
      [defaults setObject:signature forKey:kKeySignature];
      return true;
    }
    Log([NSString stringWithFormat:@"the cookie key couldn't be read: %d", (int)status]);
    // AppKit isn't up before Chromium starts, so the alert is the system's.
    NSString* title = [NSString stringWithFormat:@"%@ needs its key in your Keychain", name];
    NSString* message = [NSString
        stringWithFormat:@"%@ keeps the key to your cookies and saved passwords in your Keychain, as "
                         @"\"%@ Safe Storage\". Without it, websites would sign you out. When macOS "
                         @"asks, enter your password and choose Always Allow.",
                         name, name];
    CFOptionFlags response = kCFUserNotificationOtherResponse;
    CFUserNotificationDisplayAlert(0, kCFUserNotificationCautionAlertLevel, nullptr, nullptr,
                                   nullptr, (__bridge CFStringRef)title,
                                   (__bridge CFStringRef)message, CFSTR("Try Again"),
                                   CFSTR("Quit"), nullptr, &response);
    if (response != kCFUserNotificationDefaultResponse) return false;
  }
}

bool host_updates_available(void) {
  NSBundle* main = NSBundle.mainBundle;
  NSString* sparkle = [main.privateFrameworksPath stringByAppendingPathComponent:@"Sparkle.framework"];
  return [main objectForInfoDictionaryKey:@"SUFeedURL"] &&
         [main objectForInfoDictionaryKey:@"SUPublicEDKey"] &&
         [NSFileManager.defaultManager fileExistsAtPath:sparkle];
}

// Sparkle quits the app itself, for Install and Relaunch.
void host_start_updater(void (*)(void)) {
  if (g_updates || !host_updates_available()) return;
  NSString* path =
      [NSBundle.mainBundle.privateFrameworksPath stringByAppendingPathComponent:@"Sparkle.framework"];
  NSError* error = nil;
  // Loaded here, not linked, so builds without it run and launch never loads it.
  if (![[NSBundle bundleWithPath:path] loadAndReturnError:&error]) {
    Log([NSString stringWithFormat:@"no updates: %@", error.localizedDescription]);
    return;
  }
  g_updates = [[Updates alloc] init];
  g_updates.controller = [[NSClassFromString(@"SPUStandardUpdaterController") alloc]
      initWithStartingUpdater:YES
              updaterDelegate:g_updates
           userDriverDelegate:g_updates];
  Log(@"updater started");
}

void host_check_for_updates(void) {
  [g_updates.controller checkForUpdates:nil];
}

void host_take_update_notes(void (*notes)(const char* url)) {
  NSUserDefaults* defaults = NSUserDefaults.standardUserDefaults;
  NSDictionary* kept = [defaults dictionaryForKey:kNotes];
  // Kept until the update they're for is the one running.
  if (![kept[@"version"] isEqual:[NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleVersion"]]) {
    return;
  }
  [defaults removeObjectForKey:kNotes];
  NSString* url = kept[@"url"];
  if ([url isKindOfClass:NSString.class]) notes(url.UTF8String);
}

void host_watch_updates(void (*ready)(const char* version), bool (*quiet)(void)) {
  g_ready = ready;
  g_quiet = quiet;
}

void host_install_update(void) {
  [g_updates installNow];
}

bool host_automatic_updates(void) {
  return [NSUserDefaults.standardUserDefaults boolForKey:kIdleInstall];
}

void host_set_automatic_updates(bool on) {
  [NSUserDefaults.standardUserDefaults setBool:on forKey:kIdleInstall];
  Log([NSString stringWithFormat:@"updates install while idle %@", on ? @"on" : @"off"]);
}

// The driver's idle, as the screen locking: it posts no notification, which every app would hear.
void DriveUpdatesIdle() {
  if (g_updates) {
    [g_updates idle:nil];
  } else {
    Log(@"update idle: no updater");
  }
}
