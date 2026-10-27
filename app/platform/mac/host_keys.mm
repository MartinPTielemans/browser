#include "host_internal.h"

namespace {

// While a host_watch_control is on, its monitor, its observer and the handler to call.
id g_control_monitor;
id g_control_observer;
void (*g_control_released)(void);

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

}  // namespace

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

void host_hold_keys(void) {
  HoldKeys(nil);
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
