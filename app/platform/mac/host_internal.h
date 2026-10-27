// What the Mac host's files share: the browser windows, the overlays, the windows over a browser
// window, and the helpers more than one file calls. Rust calls host.h; nothing here is part of it.
#pragma once

#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

#include <vector>

#include "host.h"

// What's behind the window, blurred, without the material's tint, for the command bar's fill to
// tint (docs/ux-spec.md › Command bar). With Reduce Transparency, AppKit's opaque fill stays.
@interface HostBlurView : NSVisualEffectView
@end

// A browser window, once shown, the view the UI draws in, and the windows over it that are its
// own. The others show over one browser window at a time, the one they're asked to.
struct Browser {
  NSWindow* window;
  NSView* ui_view;

  // The find bar's window and view, and its distance from the top right of the window's content.
  NSWindow* find_bar;
  NSView* find_bar_view;
  bool find_bar_shown = false;
  double find_bar_right = 0;
  double find_bar_top = 0;
  // The find bar has the keyboard, or gets it back once the command bar closes: it took it opening
  // or from a click, and the user hasn't clicked the browser window since. AppKit also makes it key
  // when the command bar closes, which isn't the user choosing it.
  bool find_bar_keyboard = false;

  // The offer to save a password: its window and view, its distance from the top right of the
  // window's content, and whether it has the keyboard, or gets it back, as the find bar does.
  NSWindow* offer;
  NSView* offer_view;
  bool offer_shown = false;
  double offer_right = 0;
  double offer_top = 0;
  bool offer_keyboard = false;

  // How far the focused page's top right corner is inside the card's, in a split view, which the
  // find bar, the offer and the HUD keep to.
  double page_right = 0;
  double page_top = 0;

  // The sidebar reveal's window and view, the shell's handler for the pointer, the window's width,
  // and while it shows, the monitor that hands the handler the pointer.
  NSWindow* reveal;
  NSView* reveal_view;
  void (*reveal_pointer)(void* ui_view, double x);
  double reveal_width = 0;
  id reveal_monitor;
};

// The overlays' shadow, Arc's command bar's: soft and even on every side (Arc 1.170.0 cap over a
// light page, fitted to a Gaussian). It's a window of its own under the overlay's that the mouse
// goes through, so a click on it lands on the browser window and closes the overlay as any click
// outside it does, and the overlay's window keeps the overlay's frame. It goes on screen with the
// overlay's first show over a browser window and stays there, shown and hidden by its alpha, since
// putting a window on screen cost each ⌘T about 4 ms.
constexpr double kOverlayShadowRadius = 20;
// An overlay that grows in, as a system popover does, is done in 0.2 s, mostly in the first 0.1
// (Arc 1.170 rec at 2×, the Space's icon picker: 0.57 of its size at 55 ms, 0.83 at 105 ms).
constexpr double kOverlayGrow = 0.2;
constexpr double kOverlayGrowFade = 0.1;
constexpr float kOverlayShadowOpacity = 0.23;

// A window of the UI's over a browser window's page, translucent over a blur, with a shadow: the
// command bar, the Space's icon picker, or the Site Controls.
struct Overlay {
  explicit Overlay(const char* name) : name(name) {}
  Overlay(const char* name, float shadow_opacity, double shadow_radius, double shadow_drop)
      : name(name),
        shadow_opacity(shadow_opacity),
        shadow_radius(shadow_radius),
        shadow_drop(shadow_drop) {}
  // What its log lines call it, which check.sh reads.
  const char* name;
  NSWindow* window = nil;
  NSView* view = nil;
  HostBlurView* blur = nil;
  // The browser it shows over.
  Browser* over = nullptr;
  bool shown = false;
  void (*dismissed)(void) = nullptr;
  double radius = 0;
  // Where host_show_overlay last put it.
  bool centered = false;
  double left = 0;
  double top = 0;
  double width = 0;
  // From host_set_overlay_shape: x, y and radius for each corner, else it's a rounded rectangle.
  std::vector<double> corners;
  NSWindow* shadow = nil;
  CALayer* shadow_layer = nil;
  CAShapeLayer* shadow_mask = nil;
  // From host_set_overlay_growth: whether it grows in as it shows, and from where, in points from
  // its top left.
  bool grows = false;
  double grow_x = 0;
  double grow_y = 0;
  float shadow_opacity = kOverlayShadowOpacity;
  double shadow_radius = kOverlayShadowRadius;
  // How far down it falls.
  double shadow_drop = 0;
  // Past this, three times the blur and its drop, the shadow is nothing.
  double reach() const { return 3 * shadow_radius + shadow_drop; }
};

// host_window.mm
Browser* BrowserWithView(void* ui_view);
Browser* BrowserOf(NSWindow* window);
extern NSView* g_settings_view;

// host_overlay.mm
extern Overlay g_overlays[2];
void MaskOverlayBlur(Overlay& overlay);
void PlaceOverlayShadow(Overlay& overlay);
void AddBlurAndShadow(Overlay& overlay);
void RaiseOverlays(Browser* browser, NSWindow* window);

// host_children.mm
extern Overlay g_site_controls_overlay;
extern Browser* g_dialog_over;
extern NSWindow* g_dialog;
extern bool g_dialog_shown;
extern Browser* g_site_controls_over;
extern NSWindow* g_site_controls;
extern bool g_site_controls_shown;
extern Browser* g_capture_over;
extern NSWindow* g_capture;
extern bool g_capture_shown;
extern Browser* g_toast_over;
extern bool g_toast_shown;
extern Browser* g_tooltip_over;
extern Browser* g_link_over;
extern bool g_link_shown;
extern Browser* g_hud_over;
extern bool g_hud_shown;
extern Browser* g_drop_targets_over;
extern bool g_drop_targets_shown;
void AddOver(NSWindow* parent, NSWindow* child);
void WatchMainWindow();
void MakeOverlay(NSWindow* window);
void PlaceFindBar(Browser* browser);
void PlaceOffer(Browser* browser);
NSWindow* KeyWindowOver(Browser* browser);
void PlaceSiteControls();
void DismissSiteControls();
void DismissCapture();
void PlaceToast();
void PlaceHud();

// host_keys.mm
void ReleaseKeys(NSWindow* from);
void HoldKeys(NSWindow* window);
