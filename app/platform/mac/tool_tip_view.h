// A view that shows the system's tooltip for what's under the pointer, as AppKit shows a view's
// toolTip, but changed while the pointer stays where it is. The UI's view is in one.
#pragma once

#import <AppKit/AppKit.h>

@interface ToolTipView : NSView <NSViewToolTipOwner>

// The tooltip shows after AppKit's usual delay. nil takes it away.
- (void)setToolTipAtMousePoint:(NSString*)string;

@end
