// Test driver for check.sh. With BROWSER_DRIVE=1 the app reads one command per line on stdin
// and posts the matching AppKit events to its own window. They go through the same AppKit path
// as real input (hit testing, first responder changes, key equivalents) without touching the
// owner's mouse or keyboard. Input methods are system-wide, so instead of switching the owner's,
// the driver has a scripted one of its own.
//
//   click <x> <y> [cmd] [shift] [alt] [ctrl]
//                                         left click, in points from the window's top-left
//                                         corner, with those modifiers held
//   doubleclick <x> <y>                   two left clicks, the second with a click count of 2
//   rightclick <x> <y>                    a right click
//   clickkey <x> <y>                      a left click in the key window, such as the command bar
//   doubleclickkey <x> <y>                two left clicks in the key window, as doubleclick
//   drag <x> <y> <x2> <y2>                presses at (x, y), moves to (x2, y2) and lets go
//   press <x> <y>                         presses at (x, y), for a drag in steps
//   dragto <x> <y>                        moves the pointer pressed to (x, y)
//   release <x> <y>                       lets go of it at (x, y)
//   move <x> <y>                          moves the pointer to (x, y), without a button down
//   key <key> [cmd] [shift] [alt] [ctrl]  <key> is a lowercase letter, a digit, ".", "-", "+",
//                                         "[", "]", return, escape, tab, space, backspace,
//                                         delete (forward), pageup, pagedown, up, down, left,
//                                         right or f12. "+" is on "-"'s key, as on a Danish
//                                         keyboard
//   flags [cmd] [shift] [alt] [ctrl]      the modifiers held from now, as a modifier key going
//                                         down or up; key holds them only for its own key
//   type <text>                           lowercase letters, dots and spaces
//   scroll <x> <y> <dy> <count>           <count> trackpad scroll events at (x, y), each moving
//                                         the content up by <dy> points, at a trackpad's pace
//   swipe <x> <y> <dx> <dy> <count> [<rest>]
//                                         two fingers at (x, y): <count> trackpad scroll events,
//                                         each <dx> points right and <dy> down, from the touch's
//                                         start, then its end after <rest> ms more, else at once
//   wheel <x> <y> <dx>                    a mouse wheel's tick across at (x, y), <dx> lines right
//   resize <width> <height>               resizes the window, keeping its top-left corner
//   fullscreen                            enters or leaves full screen, like the green button
//   layout                                logs the frames of the window's views and child
//                                         windows, and the centres of its close, minimize and
//                                         zoom buttons
//   state                                 nothing, for the done line's report
//   title                                 logs the key window's title, as "browser: key window
//                                         titled <title>", such as a window of Chrome's
//   backdrop <grey>|off                   a window of the driver's in <grey> (0–255) behind the
//                                         browser window and 100 points past its edges, for the
//                                         window's material to blur something known
//   menu <title>                          chooses the main menu's item titled <title>, in a
//                                         submenu too, or in the submenu <title> names first
//                                         as "Submenu › Item"
//   choose <title>                        chooses the item titled <title> in the menu a right
//                                         click showed, the page's too, or the button in the
//                                         alert shown, Chrome's too
//   ime on|off                            the scripted input method, for keys to the UI or the
//                                         page: letters add to a composition, space converts it
//                                         to 日本語, return commits it and escape cancels it
//
// Clicks, drags and moves go to the app's window on top at that point of the browser window, such
// as the sidebar reveal over its edge, scrolls to the browser window, and keys and clickkey to the
// key window. It logs "browser: drive <n>: <command>" when command n arrives and "browser: drive
// <n> done, key <window class>, focus <view class>, active <0|1>, marked <location,length|none>"
// once AppKit has handled its events, "browser: menu tracked <titles>" as a menu opens, such as
// the page's, and "browser: modal shown <texts and buttons>" as an alert does. A page ignores
// clicks while the app isn't the active app, and macOS doesn't let a background process make it
// active.
#import <Cocoa/Cocoa.h>
#import <objc/runtime.h>

#include <cstdio>
#include <map>
#include <sstream>
#include <string>
#include <thread>

#include "host.h"

namespace {

// A character typed with ⇧, such as + or ?, has its key's code on a US keyboard.
const std::map<std::string, unsigned short> kKeyCodes = {
    {"a", 0},  {"s", 1},  {"d", 2},  {"f", 3},  {"h", 4},  {"g", 5},       {"z", 6},
    {"x", 7},  {"c", 8},  {"v", 9},  {"b", 11}, {"q", 12}, {"w", 13},      {"e", 14},
    {"r", 15}, {"y", 16}, {"t", 17}, {"o", 31}, {"u", 32}, {"i", 34},      {"p", 35},
    {"l", 37}, {"j", 38}, {"k", 40}, {"n", 45}, {"m", 46}, {"return", 36}, {"tab", 48},
    {" ", 49}, {"space", 49}, {".", 47}, {"escape", 53}, {"1", 18},   {"2", 19},
    {"3", 20}, {"4", 21}, {"5", 23}, {"6", 22}, {"7", 26}, {"8", 28},      {"9", 25},
    {"0", 29}, {"-", 27}, {"+", 27}, {"[", 33}, {"]", 30}, {"up", 126}, {"down", 125},
    {"left", 123}, {"right", 124}, {"backspace", 51}, {",", 43}, {"delete", 117},
    {"pageup", 116}, {"pagedown", 121}, {"/", 44}, {"?", 44}, {"=", 24}, {"&", 26}, {"_", 27},
    {":", 41}, {"f12", 111},
};
// AppKit's characters for the arrows are NSUpArrowFunctionKey to NSRightArrowFunctionKey, and for
// the others NSDeleteFunctionKey, NSPageUpFunctionKey, NSPageDownFunctionKey and NSF12FunctionKey.
const std::map<std::string, std::string> kCharacters = {
    {"return", "\r"},
    {"tab", "\t"},
    {"space", " "},
    {"escape", "\x1b"},
    {"backspace", "\x7f"},
    {"up", "\xEF\x9C\x80"},
    {"down", "\xEF\x9C\x81"},
    {"left", "\xEF\x9C\x82"},
    {"right", "\xEF\x9C\x83"},
    {"delete", "\xEF\x9C\xA8"},
    {"pageup", "\xEF\x9C\xAC"},
    {"pagedown", "\xEF\x9C\xAD"},
    {"f12", "\xEF\x9C\x8F"},
};

// Marks the end of a command's events. AppKit handles events in the order they were posted.
constexpr short kDoneSubtype = 0x5350;

// The window the backdrop command puts behind the browser window.
NSWindow* g_backdrop;

// The menu AppKit tracks, such as a page's context menu, which Chrome shows. choose picks from it
// when none of the app's own menus is showing.
__weak NSMenu* g_tracked;

// The largest visible window: the browser window, not a child window such as a menu, nor the
// backdrop.
NSWindow* MainWindow() {
  NSWindow* main = nil;
  for (NSWindow* window in NSApp.windows) {
    if (window.visible && window != g_backdrop &&
        (!main || NSWidth(window.frame) * NSHeight(window.frame) >
                      NSWidth(main.frame) * NSHeight(main.frame))) {
      main = window;
    }
  }
  return main;
}

void PostMouse(NSWindow* window,
               NSEventType type,
               double x,
               double y,
               int count,
               NSEventModifierFlags flags = 0) {
  NSEvent* event = [NSEvent mouseEventWithType:type
                                      location:NSMakePoint(x, NSHeight(window.frame) - y)
                                 modifierFlags:flags
                                     timestamp:NSProcessInfo.processInfo.systemUptime
                                  windowNumber:window.windowNumber
                                       context:nil
                                   eventNumber:0
                                    clickCount:count
                                      pressure:1];
  // AppKit gives every event it makes button 0, and GPUI takes the button from the number.
  if (type == NSEventTypeRightMouseDown || type == NSEventTypeRightMouseUp) {
    CGEventRef right = CGEventCreateCopy(event.CGEvent);
    CGEventSetIntegerValueField(right, kCGMouseEventButtonNumber, kCGMouseButtonRight);
    event = [NSEvent eventWithCGEvent:right];
    CFRelease(right);
  }
  [NSApp postEvent:event atStart:NO];
}

// Hands |move| to the owner of every tracking area in |view| and under it that asks for moves and
// holds the pointer, as AppKit does with the window server's moves and not with posted ones, but
// to |hit|, which has it.
void TrackMove(NSView* view, NSEvent* move, NSView* hit) {
  if (view.hiddenOrHasHiddenAncestor) {
    return;
  }
  NSPoint at = [view convertPoint:move.locationInWindow fromView:nil];
  for (NSTrackingArea* area in view.trackingAreas) {
    NSTrackingAreaOptions options = area.options;
    NSRect rect = options & NSTrackingInVisibleRect ? view.visibleRect : area.rect;
    bool active = options & NSTrackingActiveAlways ||
                  (options & NSTrackingActiveInActiveApp && NSApp.active) ||
                  (options & NSTrackingActiveInKeyWindow && view.window.keyWindow) ||
                  (options & NSTrackingActiveWhenFirstResponder &&
                   view.window.firstResponder == view);
    if (options & NSTrackingMouseMoved && active && NSPointInRect(at, rect) &&
        area.owner != hit && [area.owner respondsToSelector:@selector(mouseMoved:)]) {
      [area.owner mouseMoved:move];
    }
  }
  for (NSView* subview in view.subviews) {
    TrackMove(subview, move, hit);
  }
}

// (x, y) in |from| as a point in |to|, each from its window's top-left corner.
void Convert(NSWindow* from, NSWindow* to, double* x, double* y) {
  NSPoint screen = [from convertPointToScreen:NSMakePoint(*x, NSHeight(from.frame) - *y)];
  NSPoint at = [to convertPointFromScreen:screen];
  *x = at.x;
  *y = NSHeight(to.frame) - at.y;
}

// The window of the app's that the pointer at (x, y) in |browser| is over, as the window server
// picks it: the top one there, such as the sidebar reveal over the browser window's edge.
NSWindow* WindowAt(NSWindow* browser, double x, double y) {
  NSPoint screen = [browser convertPointToScreen:NSMakePoint(x, NSHeight(browser.frame) - y)];
  NSInteger number = [NSWindow windowNumberAtPoint:screen belowWindowWithWindowNumber:0];
  for (NSWindow* window in NSApp.windows) {
    if (window.visible && window.windowNumber == number) {
      return window;
    }
  }
  return browser;
}

void Click(NSWindow* window, double x, double y, int count, NSEventModifierFlags flags = 0) {
  PostMouse(window, NSEventTypeLeftMouseDown, x, y, count, flags);
  PostMouse(window, NSEventTypeLeftMouseUp, x, y, count, flags);
}

// The modifiers named in the rest of |words|: cmd, shift, alt and ctrl.
NSEventModifierFlags Modifiers(std::istringstream& words) {
  NSEventModifierFlags flags = 0;
  std::string modifier;
  while (words >> modifier) {
    flags |= modifier == "cmd"     ? NSEventModifierFlagCommand
             : modifier == "shift" ? NSEventModifierFlagShift
             : modifier == "alt"   ? NSEventModifierFlagOption
             : modifier == "ctrl"  ? NSEventModifierFlagControl
                                   : 0;
  }
  return flags;
}

bool Key(NSWindow* window, const std::string& key, NSEventModifierFlags flags) {
  auto code = kKeyCodes.find(key);
  if (code == kKeyCodes.end()) {
    return false;
  }
  auto special = kCharacters.find(key);
  NSString* characters = @((special == kCharacters.end() ? key : special->second).c_str());
  // The keyboard marks arrow keys as function and keypad keys, and the keys above them as function
  // keys.
  if (key == "up" || key == "down" || key == "left" || key == "right") {
    flags |= NSEventModifierFlagFunction | NSEventModifierFlagNumericPad;
  } else if (key == "delete" || key == "pageup" || key == "pagedown" || key == "f12") {
    flags |= NSEventModifierFlagFunction;
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

// Two fingers, as a trackpad reports them: a touch that begins, moves, rests for |rest_ms| and
// ends, with the phases AppKit reads from Quartz's field.
void Swipe(NSWindow* window, double x, double y, double dx, double dy, int count, int rest_ms,
           void (^done)(void)) {
  long window_number = window.windowNumber;
  void (^post)(double, double, int64_t) = ^(double across, double along, int64_t phase) {
    CGEventRef event = CGEventCreateScrollWheelEvent2(nullptr, kCGScrollEventUnitPixel, 2,
                                                      (int32_t)along, (int32_t)across, 0);
    CGEventSetIntegerValueField(event, kCGScrollWheelEventScrollPhase, phase);
    CGEventSetIntegerValueField(event, kWindowNumberField, window_number);
    CGEventSetWindowLocation(event, CGPointMake(x, y));
    [NSApp postEvent:[NSEvent eventWithCGEvent:event] atStart:NO];
    CFRelease(event);
  };
  __block int sent = 0;
  dispatch_source_t timer =
      dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0, dispatch_get_main_queue());
  dispatch_source_set_timer(timer, DISPATCH_TIME_NOW, 8 * NSEC_PER_MSEC, NSEC_PER_MSEC);
  dispatch_source_set_event_handler(timer, ^{
    post(dx, dy, sent == 0 ? kCGScrollPhaseBegan : kCGScrollPhaseChanged);
    if (++sent < count) {
      return;
    }
    dispatch_source_cancel(timer);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)rest_ms * NSEC_PER_MSEC),
                   dispatch_get_main_queue(), ^{
                     post(0, 0, kCGScrollPhaseEnded);
                     done();
                   });
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
    if (view.layer.masksToBounds && view.layer.cornerRadius > 0) {
      [line appendFormat:@" radius %.0f", view.layer.cornerRadius];
    }
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
    if (button.hidden) {
      [line appendString:@" hidden"];
      continue;
    }
    NSRect frame = [button convertRect:button.bounds toView:nil];
    [line appendFormat:@" %.1f,%.1f", NSMidX(frame), NSHeight(window.frame) - NSMidY(frame)];
  }
  // Child windows, such as the find bar: from the content's top-left corner, and their size.
  NSRect bounds = [window contentRectForFrameRect:window.frame];
  for (NSWindow* child in window.childWindows) {
    NSRect frame = child.frame;
    [line appendFormat:@"\nbrowser: layout child %.0f,%.0f %.0fx%.0f",
                       NSMinX(frame) - NSMinX(bounds), NSMaxY(bounds) - NSMaxY(frame),
                       NSWidth(frame), NSHeight(frame)];
  }
  fprintf(stderr, "%s\n", line.UTF8String);
}

// Chooses the item titled |title| in |menu| or its submenus, such as View › Developer, or with
// "Submenu › Item", the item in the submenu so titled, where the same title is elsewhere too.
bool ChooseMenuItem(NSMenu* menu, NSString* title) {
  NSRange split = [title rangeOfString:@" › "];
  if (split.location != NSNotFound) {
    NSString* parent = [title substringToIndex:split.location];
    NSString* item_title = [title substringFromIndex:NSMaxRange(split)];
    for (NSMenuItem* item in menu.itemArray) {
      if (!item.submenu) {
        continue;
      }
      if ([item.title isEqualToString:parent] ? ChooseMenuItem(item.submenu, item_title)
                                               : ChooseMenuItem(item.submenu, title)) {
        return true;
      }
    }
    return false;
  }
  NSInteger index = [menu indexOfItemWithTitle:title];
  if (index >= 0) {
    [menu performActionForItemAtIndex:index];
    return true;
  }
  for (NSMenuItem* item in menu.itemArray) {
    if (item.submenu && ChooseMenuItem(item.submenu, title)) {
      return true;
    }
  }
  return false;
}

// Chooses the enabled item titled |title| in the menu AppKit tracks, and closes the menu.
bool ChooseTracked(NSString* title) {
  NSMenu* menu = g_tracked;
  NSInteger index = [menu indexOfItemWithTitle:title];
  if (index < 0 || ![menu itemAtIndex:index].enabled) {
    return false;
  }
  [menu performActionForItemAtIndex:index];
  [menu cancelTracking];
  return true;
}

// The text fields' and buttons' views in |view|, in the order AppKit lays them out.
void ModalTexts(NSView* view, NSMutableArray<NSView*>* texts) {
  if ([view isKindOfClass:NSTextField.class] || [view isKindOfClass:NSButton.class]) {
    [texts addObject:view];
  }
  for (NSView* subview in view.subviews) {
    ModalTexts(subview, texts);
  }
}

// Presses the button titled |title| in the window AppKit runs modally, such as Chrome's own alerts.
bool ChooseModal(NSString* title) {
  NSMutableArray<NSView*>* texts = [NSMutableArray array];
  ModalTexts(NSApp.modalWindow.contentView, texts);
  for (NSView* view in texts) {
    if ([view isKindOfClass:NSButton.class] && [((NSButton*)view).title isEqual:title]) {
      [(NSButton*)view performClick:nil];
      return true;
    }
  }
  return false;
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
    NSWindow* target = WindowAt(window, x, y);
    Convert(window, target, &x, &y);
    Click(target, x, y, 1, Modifiers(words));
    done();
    return true;
  }
  if (action == "clickkey") {
    double x, y;
    if (!NSApp.keyWindow || !(words >> x >> y)) {
      return false;
    }
    Click(NSApp.keyWindow, x, y, 1);
    done();
    return true;
  }
  if (action == "doubleclickkey") {
    double x, y;
    if (!NSApp.keyWindow || !(words >> x >> y)) {
      return false;
    }
    Click(NSApp.keyWindow, x, y, 1);
    Click(NSApp.keyWindow, x, y, 2);
    done();
    return true;
  }
  if (action == "rightclick") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    NSWindow* target = WindowAt(window, x, y);
    Convert(window, target, &x, &y);
    PostMouse(target, NSEventTypeRightMouseDown, x, y, 1);
    PostMouse(target, NSEventTypeRightMouseUp, x, y, 1);
    done();
    return true;
  }
  if (action == "doubleclick") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    NSWindow* target = WindowAt(window, x, y);
    Convert(window, target, &x, &y);
    Click(target, x, y, 1);
    Click(target, x, y, 2);
    done();
    return true;
  }
  if (action == "drag") {
    double x, y, x2, y2;
    if (!(words >> x >> y >> x2 >> y2)) {
      return false;
    }
    NSWindow* target = WindowAt(window, x, y);
    Convert(window, target, &x, &y);
    Convert(window, target, &x2, &y2);
    PostMouse(target, NSEventTypeLeftMouseDown, x, y, 1);
    PostMouse(target, NSEventTypeLeftMouseDragged, x2, y2, 1);
    PostMouse(target, NSEventTypeLeftMouseUp, x2, y2, 1);
    done();
    return true;
  }
  // A drag in steps goes to the window the press went to, as AppKit sends it.
  static NSWindow* pressed;
  if (action == "press" || action == "dragto" || action == "release") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    if (action == "press") {
      pressed = WindowAt(window, x, y);
    }
    NSWindow* target = pressed ?: window;
    Convert(window, target, &x, &y);
    NSEventType type = action == "press"    ? NSEventTypeLeftMouseDown
                       : action == "dragto" ? NSEventTypeLeftMouseDragged
                                            : NSEventTypeLeftMouseUp;
    PostMouse(target, type, x, y, 1);
    if (action == "release") {
      pressed = nil;
    }
    done();
    return true;
  }
  if (action == "move") {
    double x, y;
    if (!(words >> x >> y)) {
      return false;
    }
    NSWindow* target = WindowAt(window, x, y);
    Convert(window, target, &x, &y);
    NSEvent* move = [NSEvent mouseEventWithType:NSEventTypeMouseMoved
                                       location:NSMakePoint(x, NSHeight(target.frame) - y)
                                  modifierFlags:0
                                      timestamp:NSProcessInfo.processInfo.systemUptime
                                   windowNumber:target.windowNumber
                                        context:nil
                                    eventNumber:0
                                     clickCount:0
                                       pressure:0];
    // It goes to the view under the pointer and to the tracking areas holding it, as a real move
    // does, which is how GPUI's view, under the page's, hears the pointer over the page. It's also
    // posted, for the app's event monitors.
    NSView* content = target.contentView;
    NSView* hit = [content hitTest:[content.superview convertPoint:move.locationInWindow
                                                          fromView:nil]];
    [hit mouseMoved:move];
    TrackMove(content, move, hit);
    [NSApp postEvent:move atStart:NO];
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
  if (action == "swipe") {
    double x, y, dx, dy;
    int count, rest = 0;
    if (!(words >> x >> y >> dx >> dy >> count) || count < 1) {
      return false;
    }
    words >> rest;
    Swipe(window, x, y, dx, dy, count, rest, done);
    return true;
  }
  if (action == "wheel") {
    double x, y, dx;
    if (!(words >> x >> y >> dx)) {
      return false;
    }
    CGEventRef event =
        CGEventCreateScrollWheelEvent2(nullptr, kCGScrollEventUnitLine, 2, 0, (int32_t)dx, 0);
    CGEventSetIntegerValueField(event, kWindowNumberField, window.windowNumber);
    CGEventSetWindowLocation(event, CGPointMake(x, y));
    [NSApp postEvent:[NSEvent eventWithCGEvent:event] atStart:NO];
    CFRelease(event);
    done();
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
  if (action == "title") {
    fprintf(stderr, "browser: key window titled %s\n", NSApp.keyWindow.title.UTF8String ?: "");
    done();
    return true;
  }
  if (action == "backdrop") {
    std::string grey;
    if (!(words >> grey)) {
      return false;
    }
    if (grey == "off") {
      [g_backdrop orderOut:nil];
    } else {
      if (!g_backdrop) {
        g_backdrop = [[NSWindow alloc] initWithContentRect:window.frame
                                                 styleMask:NSWindowStyleMaskBorderless
                                                   backing:NSBackingStoreBuffered
                                                     defer:NO];
        g_backdrop.releasedWhenClosed = NO;
        g_backdrop.hasShadow = NO;
      }
      const double level = std::stod(grey) / 255;
      g_backdrop.backgroundColor = [NSColor colorWithSRGBRed:level green:level blue:level alpha:1];
      // Past the edges by more than the material's blur, so it blurs nothing else.
      [g_backdrop setFrame:NSInsetRect(window.frame, -100, -100) display:YES];
      [g_backdrop orderWindow:NSWindowBelow relativeTo:window.windowNumber];
    }
    done();
    return true;
  }
  if (action == "menu") {
    NSString* title = @(line.substr(std::min(line.size(), action.size() + 1)).c_str());
    for (NSMenuItem* top in NSApp.mainMenu.itemArray) {
      if (top.submenu && ChooseMenuItem(top.submenu, title)) {
        done();
        return true;
      }
    }
    return false;
  }
  if (action == "choose") {
    std::string title = line.substr(std::min(line.size(), action.size() + 1));
    if (!host_choose(title.c_str()) && !ChooseTracked(@(title.c_str())) &&
        !ChooseModal(@(title.c_str()))) {
      return false;
    }
    // Once the menu or alert has closed, which GCD's main queue waits for.
    dispatch_async(dispatch_get_main_queue(), done);
    return true;
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
  if (action == "key" || action == "flags") {
    std::string key;
    if (action == "key") {
      words >> key;
    }
    NSEventModifierFlags flags = Modifiers(words);
    if (action == "flags") {
      // As the left ⌃ key would report it.
      [NSApp postEvent:[NSEvent keyEventWithType:NSEventTypeFlagsChanged
                                        location:NSZeroPoint
                                   modifierFlags:flags
                                       timestamp:NSProcessInfo.processInfo.systemUptime
                                    windowNumber:key_window.windowNumber
                                         context:nil
                                      characters:@""
                     charactersIgnoringModifiers:@""
                                       isARepeat:NO
                                         keyCode:59]
               atStart:NO];
    } else if (!Key(key_window, key, flags)) {
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
  // A menu AppKit starts tracking, such as a page's context menu, which Chrome shows: its items,
  // for check.sh to wait for and read, as the app's own menus log theirs.
  [NSNotificationCenter.defaultCenter
      addObserverForName:NSMenuDidBeginTrackingNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification* notification) {
                NSMenu* menu = notification.object;
                if (menu == NSApp.mainMenu) {
                  return;
                }
                g_tracked = menu;
                NSMutableArray<NSString*>* titles = [NSMutableArray array];
                for (NSMenuItem* item in menu.itemArray) {
                  if (!item.separatorItem && !item.hidden) {
                    [titles addObject:item.title];
                  }
                }
                fprintf(stderr, "browser: menu tracked %s\n",
                        [titles componentsJoinedByString:@", "].UTF8String);
              }];
  // An alert AppKit runs modally, such as Chrome's own: its texts and buttons, for check.sh to
  // wait for and read.
  [NSNotificationCenter.defaultCenter
      addObserverForName:NSWindowDidBecomeKeyNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification* notification) {
                NSWindow* window = notification.object;
                if (window != NSApp.modalWindow) {
                  return;
                }
                NSMutableArray<NSView*>* views = [NSMutableArray array];
                ModalTexts(window.contentView, views);
                NSMutableArray<NSString*>* texts = [NSMutableArray array];
                for (NSView* view in views) {
                  NSString* text = [view isKindOfClass:NSButton.class]
                                       ? ((NSButton*)view).title
                                       : ((NSTextField*)view).stringValue;
                  if (text.length && !view.hidden) {
                    [texts addObject:text];
                  }
                }
                fprintf(stderr, "browser: modal shown %s\n",
                        [texts componentsJoinedByString:@", "].UTF8String);
              }];
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
      // In the run loop's common modes, so a command also runs while a menu is open. GCD's main
      // queue waits until it closes.
      CFRunLoopPerformBlock(CFRunLoopGetMain(), kCFRunLoopCommonModes, ^{
        fprintf(stderr, "browser: drive %ld: %s\n", n, line.c_str());
        if (!Run(line, ^{
              PostDone(n);
            })) {
          fprintf(stderr, "browser: drive %ld: can't do that\n", n);
          PostDone(n);
        }
      });
      CFRunLoopWakeUp(CFRunLoopGetMain());
    }
  }).detach();
}
