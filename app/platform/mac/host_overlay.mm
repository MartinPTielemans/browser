#include <algorithm>
#include <iterator>

#include "host_internal.h"

namespace {

// Takes the tint and the extra saturation off |layer| and the layers in it, leaving the blur, as
// GPUI's blurred window background does (crates/gpui_macos/src/window.rs).
void StripTint(CALayer* layer) {
  layer.backgroundColor = nil;
  if ([layer.className isEqualToString:@"CAChameleonLayer"]) {
    layer.hidden = YES;
    return;
  }
  NSPredicate* saturating = [NSPredicate predicateWithBlock:^BOOL(id filter, NSDictionary*) {
    return ![[filter description] containsString:@"Saturat"];
  }];
  if (layer.filters) {
    layer.filters = [layer.filters filteredArrayUsingPredicate:saturating];
  }
  for (CALayer* sublayer in layer.sublayers) {
    StripTint(sublayer);
  }
}

}  // namespace

@implementation HostBlurView
- (void)updateLayer {
  [super updateLayer];
  if (!NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceTransparency) {
    StripTint(self.layer);
  }
}
@end

Overlay g_overlays[] = {Overlay("command bar"), Overlay("icon picker")};

namespace {

Overlay* OverlayAt(int index) {
  return index >= 0 && index < int(std::size(g_overlays)) && g_overlays[index].window
             ? &g_overlays[index]
             : nullptr;
}

// The overlay's shape filling |bounds|, in coordinates whose y goes up, as Core Graphics draws.
CGPathRef OverlayPath(const Overlay& overlay, CGRect bounds) {
  const size_t count = overlay.corners.size() / 3;
  if (count < 3) {
    return CGPathCreateWithRoundedRect(bounds, overlay.radius, overlay.radius, nullptr);
  }
  auto corner = [&](size_t index) {
    const double* at = &overlay.corners[3 * (index % count)];
    return CGPointMake(CGRectGetMinX(bounds) + at[0], CGRectGetMaxY(bounds) - at[1]);
  };
  CGMutablePathRef path = CGPathCreateMutable();
  // From the middle of the last side, so each corner rounds between its two sides.
  const CGPoint first = corner(0);
  const CGPoint last = corner(count - 1);
  CGPathMoveToPoint(path, nullptr, (first.x + last.x) / 2, (first.y + last.y) / 2);
  for (size_t index = 0; index < count; index++) {
    const CGPoint at = corner(index);
    const CGPoint next = corner(index + 1);
    CGPathAddArcToPoint(path, nullptr, at.x, at.y, next.x, next.y,
                        overlay.corners[3 * index + 2]);
  }
  CGPathCloseSubpath(path);
  return path;
}

}  // namespace

// Under the UI's view, in the overlay's shape at the window's size, so nothing shows outside it.
void MaskOverlayBlur(Overlay& overlay) {
  const NSSize size = overlay.window.frame.size;
  if (overlay.corners.empty()) {
    const double radius = overlay.radius;
    NSImage* mask = [NSImage imageWithSize:NSMakeSize(2 * radius + 1, 2 * radius + 1)
                                   flipped:NO
                            drawingHandler:^BOOL(NSRect rect) {
                              [NSColor.blackColor setFill];
                              [[NSBezierPath bezierPathWithRoundedRect:rect
                                                               xRadius:radius
                                                               yRadius:radius] fill];
                              return YES;
                            }];
    mask.capInsets = NSEdgeInsetsMake(radius, radius, radius, radius);
    mask.resizingMode = NSImageResizingModeStretch;
    overlay.blur.maskImage = mask;
    return;
  }
  Overlay* shaped = &overlay;
  NSImage* mask = [NSImage imageWithSize:size
                                 flipped:NO
                          drawingHandler:^BOOL(NSRect rect) {
                            CGContextRef context = NSGraphicsContext.currentContext.CGContext;
                            CGPathRef path = OverlayPath(*shaped, rect);
                            CGContextAddPath(context, path);
                            CGContextSetFillColorWithColor(context, NSColor.blackColor.CGColor);
                            CGContextFillPath(context);
                            CGPathRelease(path);
                            return YES;
                          }];
  // Drawn at the window's size, but stretched rather than tiled if the view's differs.
  mask.resizingMode = NSImageResizingModeStretch;
  overlay.blur.maskImage = mask;
}

// Around the overlay by the shadow's reach, with the overlay's shape cut out of it, since what's
// under the translucent overlay shows through it.
void PlaceOverlayShadow(Overlay& overlay) {
  const double reach = overlay.reach();
  const NSRect frame = NSInsetRect(overlay.window.frame, -reach, -reach);
  if (NSEqualRects(frame, overlay.shadow.frame)) {
    return;
  }
  [overlay.shadow setFrame:frame display:NO];
  const CGRect bounds = CGRectMake(0, 0, NSWidth(frame), NSHeight(frame));
  CGPathRef shape = OverlayPath(overlay, CGRectInset(bounds, reach, reach));
  CGMutablePathRef outside = CGPathCreateMutable();
  CGPathAddRect(outside, nullptr, bounds);
  CGPathAddPath(outside, nullptr, shape);
  // At once, not animated to the new size.
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  overlay.shadow_layer.superlayer.frame = bounds;
  overlay.shadow_layer.frame = bounds;
  overlay.shadow_layer.shadowPath = shape;
  overlay.shadow_mask.frame = bounds;
  overlay.shadow_mask.path = outside;
  [CATransaction commit];
  CGPathRelease(outside);
  CGPathRelease(shape);
}

// Puts the blur under the overlay's view and makes its shadow's window.
void AddBlurAndShadow(Overlay& overlay) {
  // The UI draws the overlay's shape, translucent, over the blur, and the shadow is its own
  // window. A titled window would draw its edge at the window's full size.
  overlay.window.styleMask = NSWindowStyleMaskBorderless;
  overlay.window.hasShadow = NO;
  overlay.blur = [[HostBlurView alloc] initWithFrame:overlay.view.frame];
  overlay.blur.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  overlay.blur.blendingMode = NSVisualEffectBlendingModeBehindWindow;
  overlay.blur.material = NSVisualEffectMaterialUnderWindowBackground;
  overlay.blur.state = NSVisualEffectStateActive;
  MaskOverlayBlur(overlay);
  [overlay.view.superview addSubview:overlay.blur
                          positioned:NSWindowBelow
                          relativeTo:overlay.view];
  overlay.shadow = [[NSWindow alloc] initWithContentRect:NSZeroRect
                                               styleMask:NSWindowStyleMaskBorderless
                                                 backing:NSBackingStoreBuffered
                                                   defer:YES];
  overlay.shadow.releasedWhenClosed = NO;
  overlay.shadow.opaque = NO;
  overlay.shadow.backgroundColor = NSColor.clearColor;
  overlay.shadow.hasShadow = NO;
  overlay.shadow.ignoresMouseEvents = YES;
  NSView* shadow_view = overlay.shadow.contentView;
  shadow_view.wantsLayer = YES;
  // The mask is on a layer holding the shadow's, since a layer's own mask doesn't cut its shadow.
  CALayer* holder = [CALayer layer];
  overlay.shadow_mask = [CAShapeLayer layer];
  overlay.shadow_mask.fillRule = kCAFillRuleEvenOdd;
  holder.mask = overlay.shadow_mask;
  overlay.shadow_layer = [CALayer layer];
  overlay.shadow_layer.shadowColor = NSColor.blackColor.CGColor;
  overlay.shadow_layer.shadowOpacity = overlay.shadow_opacity;
  overlay.shadow_layer.shadowRadius = overlay.shadow_radius;
  // The layer's y goes up.
  overlay.shadow_layer.shadowOffset = CGSizeMake(0, -overlay.shadow_drop);
  [holder addSublayer:overlay.shadow_layer];
  [shadow_view.layer addSublayer:holder];
}

// Puts the overlays showing over |browser| above |window|, one of its child windows.
void RaiseOverlays(Browser* browser, NSWindow* window) {
  for (Overlay& overlay : g_overlays) {
    if (overlay.shown && overlay.over == browser) {
      [overlay.window orderWindow:NSWindowAbove relativeTo:window.windowNumber];
    }
  }
}

void host_add_overlay(int index, void* overlay_view, double radius, void (*dismissed)(void)) {
  if (index < 0 || index >= int(std::size(g_overlays))) {
    return;
  }
  Overlay& overlay = g_overlays[index];
  overlay.view = (__bridge NSView*)overlay_view;
  overlay.dismissed = dismissed;
  overlay.radius = radius;
  overlay.window = overlay.view.window;
  MakeOverlay(overlay.window);
  AddBlurAndShadow(overlay);
  // The command bar grows and shrinks from its top with the rows. Until the frame at the new size
  // lands, the last one stays as drawn at the top; scaled to the window, it stretched or squashed
  // them.
  overlay.view.layer.contentsGravity = kCAGravityTopLeft;
  // An overlay closes when the user leaves it: a click outside it, another window taking the
  // keyboard, or switching apps.
  Overlay* each = &overlay;
  NSNotificationCenter* center = NSNotificationCenter.defaultCenter;
  [center addObserverForName:NSApplicationDidResignActiveNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification*) {
                    host_dismiss_overlay(index);
                  }];
  // Chromium makes its window key by itself, such as when a page commits and its view replaces
  // the one that had focus. That isn't the user leaving, so the overlay takes the keyboard back.
  [center addObserverForName:NSWindowDidBecomeKeyNotification
                      object:nil
                       queue:nil
                  usingBlock:^(NSNotification* notification) {
                    NSWindow* window = notification.object;
                    Browser* browser = BrowserOf(window);
                    if (each->shown && window != each->window) {
                      if (browser == each->over) {
                        [each->window makeKeyWindow];
                      } else {
                        host_dismiss_overlay(index);
                      }
                    } else if (index == 0 && !g_dialog_shown && browser &&
                               std::none_of(std::begin(g_overlays), std::end(g_overlays),
                                            [](const Overlay& other) { return other.shown; })) {
                      // The same for the find bar and the offer, which the user left only by
                      // clicking.
                      if (NSWindow* over = KeyWindowOver(browser); over != window) {
                        [over makeKeyWindow];
                      }
                    }
                  }];
}

void host_set_overlay_shape(int index, const double* corners, size_t count) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  overlay->corners.assign(corners, corners + 3 * count);
  MaskOverlayBlur(*overlay);
  // Drawn again at its next show.
  [overlay->shadow setFrame:NSZeroRect display:NO];
}

void host_set_overlay_radius(int index, double radius) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || overlay->radius == radius) {
    return;
  }
  overlay->radius = radius;
  MaskOverlayBlur(*overlay);
  // Drawn again at its next show.
  [overlay->shadow setFrame:NSZeroRect display:NO];
}

// Grows |overlay| in from near nothing about its growth point, its blur and drawing together, as
// NSPopover does, with its shadow fading in alongside.
static void GrowOverlay(Overlay& overlay) {
  NSView* container = overlay.view.superview;
  container.wantsLayer = YES;
  CALayer* layer = container.layer;
  const NSSize size = container.bounds.size;
  // A sublayer transform applies about the layer's anchor point, which AppKit puts at a view's
  // bottom left, and y goes up unless the view is flipped.
  const double x = overlay.grow_x - layer.anchorPoint.x * size.width;
  const double y = (container.isFlipped ? overlay.grow_y : size.height - overlay.grow_y) -
                   layer.anchorPoint.y * size.height;
  CATransform3D from = CATransform3DMakeTranslation(x, y, 0);
  from = CATransform3DScale(from, 0.05, 0.05, 1);
  from = CATransform3DTranslate(from, -x, -y, 0);
  CABasicAnimation* grow = [CABasicAnimation animationWithKeyPath:@"sublayerTransform"];
  grow.fromValue = [NSValue valueWithCATransform3D:from];
  grow.toValue = [NSValue valueWithCATransform3D:CATransform3DIdentity];
  grow.duration = kOverlayGrow;
  grow.timingFunction = [CAMediaTimingFunction functionWithControlPoints:0.15:0.75:0.35:1];
  [layer addAnimation:grow forKey:@"grow"];
  CABasicAnimation* fade = [CABasicAnimation animationWithKeyPath:@"opacity"];
  fade.fromValue = @0;
  fade.toValue = @1;
  fade.duration = kOverlayGrowFade;
  [layer addAnimation:fade forKey:@"fade"];
  overlay.shadow.alphaValue = 0;
  [NSAnimationContext
      runAnimationGroup:^(NSAnimationContext* context) {
        context.duration = kOverlayGrow;
        overlay.shadow.animator.alphaValue = 1;
      }
      completionHandler:nil];
}

void host_set_overlay_growth(int index, double x, double y) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  overlay->grows = true;
  overlay->grow_x = x;
  overlay->grow_y = y;
}

void host_show_overlay(int index,
                       void* ui_view,
                       bool centered,
                       double left,
                       double top,
                       double width) {
  Overlay* overlay = OverlayAt(index);
  Browser* browser = BrowserWithView(ui_view);
  if (!overlay || !browser || !browser->window) {
    return;
  }
  const bool moved = overlay->centered != centered || overlay->left != left ||
                     overlay->top != top || overlay->width != width;
  overlay->centered = centered;
  overlay->left = left;
  overlay->top = top;
  overlay->width = width;
  if (overlay->shown && overlay->over == browser) {
    // Such as ⌘L while the command bar is open over the URL field: it moves without going.
    if (!moved) {
      [overlay->window makeKeyAndOrderFront:nil];
      ReleaseKeys(overlay->window);
      return;
    }
  } else {
    // One overlay at a time, as one has the keyboard.
    for (int other = 0; other < int(std::size(g_overlays)); other++) {
      if (other != index) {
        host_dismiss_overlay(other);
      }
    }
    host_hide_overlay(index);
    overlay->shown = true;
    overlay->over = browser;
    HoldKeys(overlay->window);
  }
  // The window still holds what it drew when it last closed, such as the previous query. It goes
  // on screen once the UI has drawn it as it is now, which can't happen while the UI is still
  // handling the shortcut that opened it.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!overlay->shown) {
      ReleaseKeys(overlay->window);
    }
    if (!overlay->shown || overlay->over != browser) {
      return;
    }
    NSWindow* window = browser->window;
    NSRect content = [window contentRectForFrameRect:window.frame];
    const double height = NSHeight(overlay->window.frame);
    NSRect frame;
    if (centered) {
      frame = NSMakeRect(NSMidX(content) - width / 2, NSMidY(content) + top - height, width,
                         height);
    } else {
      frame = NSMakeRect(NSMinX(content) + left, NSMaxY(content) - top - height,
                         fmin(width, NSWidth(content) - left - 8), height);
    }
    // At its new size before it draws, so it draws once.
    [overlay->window setFrame:frame display:NO];
    [overlay->view.layer setNeedsDisplay];
    [overlay->view.layer displayIfNeeded];
    PlaceOverlayShadow(*overlay);
    overlay->shadow.alphaValue = 1;
    if (overlay->shadow.parentWindow != window) {
      [overlay->shadow.parentWindow removeChildWindow:overlay->shadow];
      AddOver(window, overlay->shadow);
    }
    AddOver(window, overlay->window);
    if (overlay->grows) {
      GrowOverlay(*overlay);
    }
    [overlay->window makeKeyAndOrderFront:nil];
    ReleaseKeys(overlay->window);
    // check.sh reads these lines.
    fprintf(stderr, "browser: %s at %g %g %g %g\n", overlay->name,
            NSMinX(frame) - NSMinX(content), NSMaxY(content) - NSMaxY(frame), NSWidth(frame),
            NSHeight(frame));
    fprintf(stderr, "browser: %s shown\n", overlay->name);
  });
}

void host_set_overlay_height(int index, double height) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay) {
    return;
  }
  // After the UI's update that asked, since the overlay draws at its new size here.
  dispatch_async(dispatch_get_main_queue(), ^{
    if (NSHeight(overlay->window.frame) == height) {
      return;
    }
    NSRect frame = overlay->window.frame;
    // Keeping its top edge, so the rows grow down from the input.
    [overlay->window
        setFrame:NSMakeRect(NSMinX(frame), NSMaxY(frame) - height, NSWidth(frame), height)
         display:NO];
    [overlay->view.layer setNeedsDisplay];
    [overlay->view.layer displayIfNeeded];
    PlaceOverlayShadow(*overlay);
    fprintf(stderr, "browser: %s height %g\n", overlay->name, height);
  });
}

void host_hide_overlay(int index) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || !overlay->shown) {
    return;
  }
  overlay->shown = false;
  Browser* browser = overlay->over;
  [browser->window removeChildWindow:overlay->window];
  [overlay->window orderOut:nil];
  overlay->shadow.alphaValue = 0;
  [KeyWindowOver(browser) makeKeyWindow];
  fprintf(stderr, "browser: %s hidden\n", overlay->name);
}

void host_dismiss_overlay(int index) {
  Overlay* overlay = OverlayAt(index);
  if (!overlay || !overlay->shown) {
    return;
  }
  host_hide_overlay(index);
  // Posted: the UI may be handling the event that closed the overlay.
  if (void (*dismissed)(void) = overlay->dismissed) {
    dispatch_async(dispatch_get_main_queue(), ^{
      dismissed();
    });
  }
}
