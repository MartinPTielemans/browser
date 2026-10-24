// Test driver for check.sh. With BROWSER_DRIVE=1 the app reads one command per line on stdin
// and posts the matching AppKit events to its own window. They go through the same AppKit path
// as real input (hit testing, first responder changes, key equivalents) without touching the
// owner's mouse or keyboard. Input methods are system-wide, so instead of switching the owner's,
// the driver has a scripted one of its own.
//
//   click <x> <y>                         left click, in points from the window's top-left corner
//   doubleclick <x> <y>                   two left clicks, the second with a click count of 2
//   drag <x> <y> <x2> <y2>                presses at (x, y), moves to (x2, y2) and lets go
//   key <key> [cmd] [shift] [alt] [ctrl]  <key> is a lowercase letter, a digit, ".", "[", "]",
//                                         return, escape, tab, space, up, down, left or right
//   type <text>                           lowercase letters, dots and spaces
//   scroll <x> <y> <dy> <count>           <count> trackpad scroll events at (x, y), each moving
//                                         the content up by <dy> points, at a trackpad's pace
//   resize <width> <height>               resizes the window, keeping its top-left corner
//   fullscreen                            enters or leaves full screen, like the green button
//   layout                                logs the frames of the window's views, and the
//                                         centres of its close, minimize and zoom buttons
//   state                                 nothing, for the done line's report
//   menu <title>                          chooses the main menu's item titled <title>
//   ime on|off                            the scripted input method, for keys to the UI or the
//                                         page: letters add to a composition, space converts it
//                                         to 日本語, return commits it and escape cancels it
//
// Clicks, drags and scrolls go to the browser window and keys to the key window. It logs
// "browser: drive <n>: <command>" when command n arrives and "browser: drive <n> done, key <window
// class>, focus <view class>, active <0|1>, marked <location,length|none>" once AppKit has handled
// its events. A page ignores clicks while the app isn't the active app, and macOS doesn't let a
// background process make it active.
#import <Cocoa/Cocoa.h>
#import <objc/runtime.h>

#include <cstdio>
#include <map>
#include <sstream>
#include <string>
#include <thread>

#include "host.h"

namespace {

const std::map<std::string, unsigned short> kKeyCodes = {
    {"a", 0},  {"s", 1},  {"d", 2},  {"f", 3},  {"h", 4},  {"g", 5},       {"z", 6},
    {"x", 7},  {"c", 8},  {"v", 9},  {"b", 11}, {"q", 12}, {"w", 13},      {"e", 14},
    {"r", 15}, {"y", 16}, {"t", 17}, {"o", 31}, {"u", 32}, {"i", 34},      {"p", 35},
    {"l", 37}, {"j", 38}, {"k", 40}, {"n", 45}, {"m", 46}, {"return", 36}, {"tab", 48},
    {" ", 49}, {"space", 49}, {".", 47}, {"escape", 53}, {"1", 18},   {"2", 19},
    {"3", 20}, {"4", 21}, {"5", 23}, {"6", 22}, {"7", 26}, {"8", 28},      {"9", 25},
    {"[", 33}, {"]", 30}, {"up", 126}, {"down", 125}, {"left", 123}, {"right", 124},
};
// AppKit's characters for the arrows are NSUpArrowFunctionKey to NSRightArrowFunctionKey.
const std::map<std::string, std::string> kCharacters = {
    {"return", "\r"},
    {"tab", "\t"},
    {"space", " "},
    {"escape", "\x1b"},
    {"up", "\xEF\x9C\x80"},
    {"down", "\xEF\x9C\x81"},
    {"left", "\xEF\x9C\x82"},
    {"right", "\xEF\x9C\x83"},
};

// Marks the end of a command's events. AppKit handles events in the order they were posted.
constexpr short kDoneSubtype = 0x5350;

// The largest visible window: the browser window, not a child window such as a menu.
NSWindow* MainWindow() {
  NSWindow* main = nil;
  for (NSWindow* window in NSApp.windows) {
    if (window.visible && (!main || NSWidth(window.frame) * NSHeight(window.frame) >
                                        NSWidth(main.frame) * NSHeight(main.frame))) {
      main = window;
    }
  }
  return main;
}

void PostMouse(NSWindow* window, NSEventType type, double x, double y, int count) {
  [NSApp postEvent:[NSEvent mouseEventWithType:type
                                      location:NSMakePoint(x, NSHeight(window.frame) - y)
                                 modifierFlags:0
                                     timestamp:NSProcessInfo.processInfo.systemUptime
                                  windowNumber:window.windowNumber
                                       context:nil
                                   eventNumber:0
                                    clickCount:count
                                      pressure:1]
           atStart:NO];
}

void Click(NSWindow* window, double x, double y, int count) {
  PostMouse(window, NSEventTypeLeftMouseDown, x, y, count);
  PostMouse(window, NSEventTypeLeftMouseUp, x, y, count);
}

bool Key(NSWindow* window, const std::string& key, NSEventModifierFlags flags) {
  auto code = kKeyCodes.find(key);
  if (code == kKeyCodes.end()) {
    return false;
  }
  auto special = kCharacters.find(key);
  NSString* characters = @((special == kCharacters.end() ? key : special->second).c_str());
  // The keyboard marks arrow keys as function and keypad keys.
  if (key == "up" || key == "down" || key == "left" || key == "right") {
    flags |= NSEventModifierFlagFunction | NSEventModifierFlagNumericPad;
  }
  for (NSEventType type : {NSEventTypeKeyDown, NSEventTypeKeyUp}) {
    [NSApp postEvent:[NSEvent keyEventWithType:type
                                      location:NSZeroPoint
                                 modifierFlags:flags
                                     timestamp:NSProcessInfo.processInfo.systemUptime
                                  windowNumber:window.windowNumber
                                       context:nil
                                    characters:characters
                   charactersIgnoringModifiers:characters
                                     isARepeat:NO
                                       keyCode:code->second]
             atStart:NO];
  }
  return true;
}

// AppKit has no maker for scroll events, so they start as Quartz events, and AppKit reads their
// window and their position in it from data no public call sets. Without both, AppKit drops the
// event. Field 51 is where an NSEvent made with a window number keeps it; CGEventSetWindowLocation
// is a private CoreGraphics call. Fine for a test driver, never for the browser.
constexpr CGEventField kWindowNumberField = (CGEventField)51;
extern "C" void CGEventSetWindowLocation(CGEventRef event, CGPoint location);

// Trackpads report about every 8 ms while a finger moves. Each event carries pixel deltas, as a
// trackpad's do, so Chromium scrolls by exactly that much instead of animating a wheel tick.
void Scroll(NSWindow* window, double x, double y, double dy, int count, void (^done)(void)) {
  long window_number = window.windowNumber;
  __block int left = count;
  dispatch_source_t timer =
      dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0, dispatch_get_main_queue());
  dispatch_source_set_timer(timer, DISPATCH_TIME_NOW, 8 * NSEC_PER_MSEC, NSEC_PER_MSEC);
  dispatch_source_set_event_handler(timer, ^{
    CGEventRef event =
        CGEventCreateScrollWheelEvent2(nullptr, kCGScrollEventUnitPixel, 1, (int32_t)-dy, 0, 0);
    CGEventSetIntegerValueField(event, kWindowNumberField, window_number);
    CGEventSetWindowLocation(event, CGPointMake(x, y));
    [NSApp postEvent:[NSEvent eventWithCGEvent:event] atStart:NO];
    CFRelease(event);
    if (--left == 0) {
      dispatch_source_cancel(timer);
      done();
    }
  });
  dispatch_resume(timer);
}

// The scripted input method makes the calls a Japanese one makes, at the same point: while the
// view handles the key. GPUI hands keys to the input context and the page view interprets them
// itself, so each gets its own hook. The composition is nil when there isn't one.
bool g_ime_on = false;
NSString* g_composition;
IMP g_handle_event;
IMP g_interpret_key_events;

// Returns false for a key the input method leaves to the view.
bool ImeKey(id<NSTextInputClient> client, NSEvent* event) {
  constexpr NSEventModifierFlags kShortcut =
      NSEventModifierFlagCommand | NSEventModifierFlagControl | NSEventModifierFlagOption;
  if (!g_ime_on || event.type != NSEventTypeKeyDown || (event.modifierFlags & kShortcut)) {
    return false;
  }
  NSString* characters = event.characters;
  bool letter = characters.length == 1 && [NSCharacterSet.lowercaseLetterCharacterSet
                                              characterIsMember:[characters characterAtIndex:0]];
  if (!g_composition && !letter) {
    return false;
  }
  NSRange none = NSMakeRange(NSNotFound, 0);
  switch (event.keyCode) {
    case 36:  // return
      [client insertText:g_composition replacementRange:none];
      g_composition = nil;
      return true;
    case 53:  // escape
      [client setMarkedText:@"" selectedRange:NSMakeRange(0, 0) replacementRange:none];
      g_composition = nil;
      return true;
    case 49:  // space selects the whole conversion, as a real one does
      g_composition = @"日本語";
      [client setMarkedText:g_composition
              selectedRange:NSMakeRange(0, g_composition.length)
           replacementRange:none];
      return true;
  }
  if (!letter) {
    return true;
  }
  g_composition = [g_composition ?: @"" stringByAppendingString:characters];
  [client setMarkedText:g_composition
          selectedRange:NSMakeRange(g_composition.length, 0)
       replacementRange:none];
  return true;
}

BOOL HandleEvent(NSTextInputContext* context, SEL selector, NSEvent* event) {
  if ([(id)context.client isKindOfClass:NSClassFromString(@"GPUIView")] &&
      ImeKey(context.client, event)) {
    return YES;
  }
  return ((BOOL (*)(id, SEL, NSEvent*))g_handle_event)(context, selector, event);
}

void InterpretKeyEvents(NSView* view, SEL selector, NSArray<NSEvent*>* events) {
  if (events.count == 1 && ImeKey((id<NSTextInputClient>)view, events[0])) {
    return;
  }
  ((void (*)(id, SEL, NSArray*))g_interpret_key_events)(view, selector, events);
}

void HookIme() {
  if (g_handle_event) {
    return;
  }
  g_handle_event =
      method_setImplementation(class_getInstanceMethod(NSTextInputContext.class,
                                                       @selector(handleEvent:)),
                               (IMP)HandleEvent);
  g_interpret_key_events = method_setImplementation(
      class_getInstanceMethod(NSClassFromString(@"RenderWidgetHostViewCocoa"),
                              @selector(interpretKeyEvents:)),
      (IMP)InterpretKeyEvents);
}

// Frames are in points from the content view's bottom-left corner. A plain NSView and the core's
// window view only hold others, so the views in them are listed instead.
void AppendViews(NSMutableString* line, NSView* content, NSView* parent) {
  for (NSView* view in parent.subviews) {
    if (view.class == NSView.class || [NSStringFromClass(view.class) isEqual:@"BrowserCoreWindowView"]) {
      AppendViews(line, content, view);
      continue;
    }
    NSRect frame = [parent convertRect:view.frame toView:content];
    [line appendFormat:@", %@ %.0f,%.0f %.0fx%.0f", NSStringFromClass(view.class),
                       NSMinX(frame), NSMinY(frame), NSWidth(frame), NSHeight(frame)];
  }
}

void Layout(NSWindow* window) {
  NSView* content = window.contentView;
  NSMutableString* line = [NSMutableString
      stringWithFormat:@"browser: layout content %.0fx%.0f", NSWidth(content.bounds),
                       NSHeight(content.bounds)];
  AppendViews(line, content, content);
  // As the spec gives them: points from the window's top-left corner.
  [line appendString:@"\nbrowser: layout buttons"];
  for (NSWindowButton type :
       {NSWindowCloseButton, NSWindowMiniaturizeButton, NSWindowZoomButton}) {
    NSButton* button = [window standardWindowButton:type];
    NSRect frame = [button convertRect:button.bounds toView:nil];
    [line appendFormat:@" %.1f,%.1f", NSMidX(frame), NSHeight(window.frame) - NSMidY(frame)];
  }
  fprintf(stderr, "%s\n", line.UTF8String);
}

// Calls |done| once the command's events are posted. Returns false, without calling it, for a
// command it can't do.
bool Run(const std::string& line, void (^done)(void)) {
  NSWindow* window = MainWindow();
  std::istringstream words(line);
  std::string action;
  words >> action;
  if (!window) {
    return false;
  }
  if (action == "click") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    Click(window, x, y, 1);
    done();
    return true;
  }
  if (action == "doubleclick") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    Click(window, x, y, 1);
    Click(window, x, y, 2);
    done();
    return true;
  }
  if (action == "drag") {
    double x, y, x2, y2;
    if (!(words >> x >> y >> x2 >> y2)) {
      return false;
    }
    PostMouse(window, NSEventTypeLeftMouseDown, x, y, 1);
    PostMouse(window, NSEventTypeLeftMouseDragged, x2, y2, 1);
    PostMouse(window, NSEventTypeLeftMouseUp, x2, y2, 1);
    done();
    return true;
  }
  if (action == "scroll") {
    double x, y, dy;
    int count;
    if (!(words >> x >> y >> dy >> count) || count < 1) {
      return false;
    }
    Scroll(window, x, y, dy, count, done);
    return true;
  }
  if (action == "resize") {
    double width, height;
    if (!(words >> width >> height)) {
      return false;
    }
    NSRect frame = window.frame;
    [window setFrame:NSMakeRect(NSMinX(frame), NSMaxY(frame) - height, width, height) display:YES];
    done();
    return true;
  }
  if (action == "fullscreen") {
    [window toggleFullScreen:nil];
    done();
    return true;
  }
  if (action == "layout") {
    Layout(window);
    done();
    return true;
  }
  if (action == "state") {
    done();
    return true;
  }
  if (action == "menu") {
    NSString* title = @(line.substr(std::min(line.size(), action.size() + 1)).c_str());
    for (NSMenuItem* top in NSApp.mainMenu.itemArray) {
      NSInteger index = [top.submenu indexOfItemWithTitle:title];
      if (index >= 0) {
        [top.submenu performActionForItemAtIndex:index];
        done();
        return true;
      }
    }
    return false;
  }
  if (action == "ime") {
    std::string state;
    words >> state;
    if (state != "on" && state != "off") {
      return false;
    }
    HookIme();
    g_ime_on = state == "on";
    g_composition = nil;
    done();
    return true;
  }
  NSWindow* key_window = NSApp.keyWindow ?: window;
  if (action == "key") {
    std::string key, modifier;
    words >> key;
    NSEventModifierFlags flags = 0;
    while (words >> modifier) {
      flags |= modifier == "cmd"     ? NSEventModifierFlagCommand
               : modifier == "shift" ? NSEventModifierFlagShift
               : modifier == "alt"   ? NSEventModifierFlagOption
               : modifier == "ctrl"  ? NSEventModifierFlagControl
                                     : 0;
    }
    if (!Key(key_window, key, flags)) {
      return false;
    }
    done();
    return true;
  }
  if (action == "type") {
    std::string text = line.substr(std::min(line.size(), action.size() + 1));
    for (char character : text) {
      if (!Key(key_window, std::string(1, character), 0)) {
        return false;
      }
    }
    done();
    return true;
  }
  return false;
}

void PostDone(long number) {
  [NSApp postEvent:[NSEvent otherEventWithType:NSEventTypeApplicationDefined
                                      location:NSZeroPoint
                                 modifierFlags:0
                                     timestamp:0
                                  windowNumber:0
                                       context:nil
                                       subtype:kDoneSubtype
                                         data1:number
                                         data2:0]
           atStart:NO];
}

}  // namespace

void drive_start(void) {
  NSEvent* (^report_done)(NSEvent*) = ^NSEvent*(NSEvent* event) {
    if (event.subtype != kDoneSubtype) {
      return event;
    }
    NSWindow* key = NSApp.keyWindow;
    NSResponder* focus = (key ?: MainWindow()).firstResponder;
    NSString* marked = @"none";
    if ([focus conformsToProtocol:@protocol(NSTextInputClient)]) {
      NSRange range = [(id<NSTextInputClient>)focus markedRange];
      if (range.location != NSNotFound) {
        marked = [NSString stringWithFormat:@"%lu,%lu", range.location, range.length];
      }
    }
    fprintf(stderr, "browser: drive %ld done, key %s, focus %s, active %d, marked %s\n",
            (long)event.data1, key ? NSStringFromClass(key.class).UTF8String : "none",
            NSStringFromClass(focus.class).UTF8String, NSApp.active, marked.UTF8String);
    return nil;
  };
  [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskApplicationDefined handler:report_done];
  // Full-screen transitions animate, and end well after the command's done.
  for (NSNotificationName name :
       {NSWindowDidEnterFullScreenNotification, NSWindowDidExitFullScreenNotification}) {
    bool entered = [name isEqualToString:NSWindowDidEnterFullScreenNotification];
    [NSNotificationCenter.defaultCenter addObserverForName:name
                                                    object:nil
                                                     queue:nil
                                                usingBlock:^(NSNotification*) {
                                                  fprintf(stderr, "browser: window %s full screen\n",
                                                          entered ? "entered" : "left");
                                                }];
  }
  // So does a zoom, and the window is zoomed only at its last frame.
  __block BOOL zoomed = NO;
  [NSNotificationCenter.defaultCenter addObserverForName:NSWindowDidResizeNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification* notification) {
                                                NSWindow* window = notification.object;
                                                if (window != MainWindow() ||
                                                    window.zoomed == zoomed) {
                                                  return;
                                                }
                                                zoomed = window.zoomed;
                                                fprintf(stderr, "browser: window zoomed %d\n",
                                                        zoomed);
                                              }];
  std::thread([] {
    char buffer[1024];
    long number = 0;
    while (fgets(buffer, sizeof buffer, stdin)) {
      std::string line(buffer);
      line.erase(line.find_last_not_of("\r\n") + 1);
      long n = ++number;
      dispatch_async(dispatch_get_main_queue(), ^{
        fprintf(stderr, "browser: drive %ld: %s\n", n, line.c_str());
        if (!Run(line, ^{
              PostDone(n);
            })) {
          fprintf(stderr, "browser: drive %ld: can't do that\n", n);
          PostDone(n);
        }
      });
    }
  }).detach();
}
