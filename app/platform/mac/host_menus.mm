#include "host_internal.h"

// Takes the menu's choice, and says when it shows.
@interface HostMenuTarget : NSObject <NSMenuDelegate>
@property(nonatomic) NSInteger chosen;
@end

@implementation HostMenuTarget
@synthesize chosen = _chosen;

- (void)choose:(NSMenuItem*)item {
  self.chosen = item.tag;
}

- (void)menuWillOpen:(NSMenu*)menu {
  // check.sh waits for this line before it chooses.
  NSMutableArray<NSString*>* titles = [NSMutableArray array];
  auto add = ^(NSMenuItem* item, NSString* title) {
    if (item.state == NSControlStateValueOn) {
      title = [title stringByAppendingString:@" (checked)"];
    }
    [titles addObject:item.enabled ? title : [title stringByAppendingString:@" (disabled)"]];
  };
  for (NSMenuItem* item in menu.itemArray) {
    if (item.separatorItem) {
      continue;
    }
    if (!item.submenu) {
      add(item, item.title);
    }
    // A submenu's items as "Move to Space › Work".
    for (NSMenuItem* inner in item.submenu.itemArray) {
      if (!inner.separatorItem) {
        add(inner, [NSString stringWithFormat:@"%@ › %@", item.title, inner.title]);
      }
    }
  }
  fprintf(stderr, "browser: menu shown %s\n",
          [titles componentsJoinedByString:@", "].UTF8String);
}
@end

namespace {

// A round swatch of |color|, 0xRRGGBB in sRGB, for a menu item.
NSImage* Swatch(int32_t color) {
  NSColor* fill = [NSColor colorWithSRGBRed:((color >> 16) & 0xff) / 255.0
                                      green:((color >> 8) & 0xff) / 255.0
                                       blue:(color & 0xff) / 255.0
                                      alpha:1];
  return [NSImage imageWithSize:NSMakeSize(12, 12)
                        flipped:NO
                 drawingHandler:^BOOL(NSRect rect) {
                   [fill setFill];
                   [[NSBezierPath bezierPathWithOvalInRect:rect] fill];
                   return YES;
                 }];
}

// The menu or alert showing, for host_choose and host_dismiss_menu.
NSMenu* g_menu;
NSAlert* g_alert;

// Shows the menu at |x|, |y| from the top-left of |view|, after the UI in |drawn| draws what the
// menu changed. See host_menu.
int ShowMenu(const host_menu_item* items,
             size_t count,
             double x,
             double y,
             NSView* view,
             NSArray<NSView*>* drawn,
             int over) {
  HostMenuTarget* target = [[HostMenuTarget alloc] init];
  target.chosen = -1;
  NSMenu* menu = [[NSMenu alloc] init];
  menu.autoenablesItems = NO;
  menu.delegate = target;
  // By index, for the items in their submenus.
  NSMutableArray<NSMenuItem*>* made = [NSMutableArray array];
  for (size_t i = 0; i < count; ++i) {
    NSMenu* in = menu;
    if (items[i].parent >= 0 && static_cast<NSUInteger>(items[i].parent) < made.count) {
      NSMenuItem* parent = made[items[i].parent];
      if (!parent.submenu) {
        parent.submenu = [[NSMenu alloc] init];
        parent.submenu.autoenablesItems = NO;
      }
      in = parent.submenu;
    }
    if (!items[i].title) {
      NSMenuItem* line = [NSMenuItem separatorItem];
      [in addItem:line];
      [made addObject:line];
      continue;
    }
    NSMenuItem* item = [in addItemWithTitle:@(items[i].title)
                                     action:@selector(choose:)
                              keyEquivalent:@(items[i].key)];
    [made addObject:item];
    item.keyEquivalentModifierMask = (items[i].command ? NSEventModifierFlagCommand : 0) |
                                     (items[i].shift ? NSEventModifierFlagShift : 0) |
                                     (items[i].option ? NSEventModifierFlagOption : 0) |
                                     (items[i].control ? NSEventModifierFlagControl : 0);
    item.target = target;
    item.tag = static_cast<NSInteger>(i);
    item.enabled = items[i].enabled;
    item.state = items[i].checked ? NSControlStateValueOn : NSControlStateValueOff;
    if (items[i].swatch >= 0) {
      item.image = Swatch(items[i].swatch);
    }
  }
  // The UI draws what the menu changed, such as its row's fill, now: it can't once the menu's
  // event loop runs.
  for (NSView* ui in drawn) {
    [ui.layer setNeedsDisplay];
    [ui.layer displayIfNeeded];
  }
  NSPoint at = NSMakePoint(x, view.flipped ? y : NSHeight(view.bounds) - y);
  NSMenuItem* positioned =
      over >= 0 && static_cast<NSUInteger>(over) < made.count ? made[over] : nil;
  if (positioned) {
    // AppKit puts the item's edge at the point, and its title this far in, past the checkmarks'
    // column (measured, as AppKit doesn't say).
    constexpr double kMenuTitleInset = 31.5;
    at.x -= kMenuTitleInset;
  }
  g_menu = menu;
  [menu popUpMenuPositioningItem:positioned atLocation:at inView:view];
  g_menu = nil;
  return static_cast<int>(target.chosen);
}

}  // namespace

int host_menu(void* ui_view,
              const host_menu_item* items,
              size_t count,
              double x,
              double y,
              int in,
              int over) {
  if (in == HOST_IN_SETTINGS) {
    return ShowMenu(items, count, x, y, g_settings_view, @[ g_settings_view ], over);
  }
  Browser* browser = BrowserWithView(ui_view);
  if (!browser) {
    return -1;
  }
  NSView* panel = in == HOST_IN_REVEAL ? browser->reveal_view : nil;
  return ShowMenu(items, count, x, y, panel ?: browser->window.contentView,
                  panel ? @[ panel ] : browser->ui_view.subviews, over);
}

int host_menu_in(const host_menu_item* items, size_t count, double x, double y, void* view) {
  NSView* in = (__bridge NSView*)view;
  return ShowMenu(items, count, x, y, in, @[ in ], -1);
}

int host_ask(const char* message, const char* detail, const char* const* buttons, size_t count) {
  NSAlert* alert = [[NSAlert alloc] init];
  alert.messageText = @(message);
  alert.informativeText = @(detail);
  for (size_t i = 0; i < count; ++i) {
    [alert addButtonWithTitle:@(buttons[i])];
  }
  g_alert = alert;
  fprintf(stderr, "browser: alert shown %s\n", message);
  NSModalResponse response = [alert runModal];
  g_alert = nil;
  return static_cast<int>(response - NSAlertFirstButtonReturn);
}

bool host_dismiss_menu(void) {
  if (!g_menu) {
    return false;
  }
  [g_menu cancelTracking];
  return true;
}

bool host_choose(const char* title) {
  NSString* chosen = @(title);
  for (NSMenuItem* item in g_menu.itemArray) {
    // An item in a submenu, else one in the menu.
    for (NSMenuItem* each in item.submenu ? item.submenu.itemArray : @[ item ]) {
      if ([each.title isEqual:chosen] && each.enabled && !each.submenu) {
        [each.menu performActionForItemAtIndex:[each.menu indexOfItem:each]];
        [g_menu cancelTracking];
        return true;
      }
    }
  }
  for (NSButton* button in g_alert.buttons) {
    if ([button.title isEqual:chosen]) {
      [button performClick:nil];
      return true;
    }
  }
  return false;
}
