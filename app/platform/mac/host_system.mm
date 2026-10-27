#import <ImageIO/ImageIO.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

#include <sys/sysctl.h>
#include <sys/time.h>
#include <unistd.h>

#include <algorithm>

#include "host_internal.h"

// The source of a file host_drag_file drags, which offers a copy wherever it's let go.
@interface HostFileDragSource : NSObject <NSDraggingSource>
@property(nonatomic) void (*ended)(bool);
@end

@implementation HostFileDragSource
@synthesize ended = _ended;

- (NSDragOperation)draggingSession:(NSDraggingSession*)session
    sourceOperationMaskForDraggingContext:(NSDraggingContext)context {
  return NSDragOperationCopy;
}

- (void)draggingSession:(NSDraggingSession*)session
           endedAtPoint:(NSPoint)point
              operation:(NSDragOperation)operation {
  self.ended(operation != NSDragOperationNone);
}
@end

// Gives the service the user picks in the share menu the page's title, as a mail's subject.
@interface HostShareDelegate : NSObject <NSSharingServicePickerDelegate>
@property(copy) NSString* title;
@end

@implementation HostShareDelegate
- (void)sharingServicePicker:(NSSharingServicePicker*)picker
     didChooseSharingService:(NSSharingService*)service {
  service.subject = self.title;
}
@end

bool host_is_default_browser(void) {
  NSString* mine = NSBundle.mainBundle.bundleIdentifier;
  for (NSString* link in @[ @"http://", @"https://" ]) {
    NSURL* url = [NSURL URLWithString:link];
    NSURL* app = [NSWorkspace.sharedWorkspace URLForApplicationToOpenURL:url];
    if (!app || ![[NSBundle bundleWithURL:app].bundleIdentifier isEqualToString:mine]) {
      return false;
    }
  }
  return true;
}

void host_make_default_browser(void (*answered)(void)) {
  NSWorkspace* workspace = NSWorkspace.sharedWorkspace;
  NSURL* app = NSBundle.mainBundle.bundleURL;
  void (^done)(void) = ^{
    dispatch_async(dispatch_get_main_queue(), ^{
      answered();
    });
  };
  // https only once the user has said yes for http, as Chrome does, so a user who keeps their
  // browser is asked once.
  [workspace setDefaultApplicationAtURL:app
                   toOpenURLsWithScheme:@"http"
                      completionHandler:^(NSError* error) {
                        if (error) {
                          done();
                          return;
                        }
                        [NSWorkspace.sharedWorkspace setDefaultApplicationAtURL:app
                                                           toOpenURLsWithScheme:@"https"
                                                              completionHandler:^(NSError*) {
                                                                done();
                                                              }];
                      }];
  // From macOS 26.4, HTML files ask in a second dialog of Finder's, as Chrome found, so there the
  // user picks the app for them in Finder.
  if (@available(macOS 26.4, *)) {
  } else {
    [workspace setDefaultApplicationAtURL:app toOpenContentType:UTTypeHTML completionHandler:nil];
  }
  // The system's agent that asks can put its dialog behind the app's windows, where the user
  // never sees it.
  NSString* asker = @"com.apple.coreservices.uiagent";
  for (NSRunningApplication* agent in
       [NSRunningApplication runningApplicationsWithBundleIdentifier:asker]) {
    [agent activateWithOptions:NSApplicationActivateAllWindows];
  }
}

double host_since_launch(void) {
  struct kinfo_proc process;
  size_t size = sizeof(process);
  int name[] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, getpid()};
  if (sysctl(name, 4, &process, &size, nullptr, 0) != 0) {
    return 0;
  }
  struct timeval now;
  gettimeofday(&now, nullptr);
  struct timeval start = process.kp_proc.p_starttime;
  return (now.tv_sec - start.tv_sec) * 1e3 + (now.tv_usec - start.tv_usec) / 1e3;
}

namespace {

// host_copy's pasteboard: the clipboard, or in a driven run one of its own, so checks never touch
// the user's clipboard.
NSPasteboard* CopyPasteboard() {
  static NSPasteboard* driven =
      getenv("BROWSER_DRIVE") ? [NSPasteboard pasteboardWithUniqueName] : nil;
  return driven ?: NSPasteboard.generalPasteboard;
}

// A capture's name, for when it was taken, as macOS names a screenshot.
NSString* CaptureName() {
  NSDateFormatter* format = [[NSDateFormatter alloc] init];
  format.locale = [NSLocale localeWithLocaleIdentifier:@"en_US_POSIX"];
  format.dateFormat = @"'Capture' yyyy-MM-dd 'at' HH.mm.ss";
  return [[format stringFromDate:NSDate.date] stringByAppendingString:@".png"];
}

}  // namespace

void host_copy_png(const uint8_t* png, size_t size) {
  NSPasteboard* pasteboard = CopyPasteboard();
  [pasteboard clearContents];
  [pasteboard setData:[NSData dataWithBytes:png length:size] forType:NSPasteboardTypePNG];
  // check.sh reads this line: what the pasteboard holds now, and which one it is.
  fprintf(stderr, "browser: pasteboard png %lu bytes, %s\n",
          (unsigned long)[pasteboard dataForType:NSPasteboardTypePNG].length,
          pasteboard == NSPasteboard.generalPasteboard ? "general" : "private");
}

void host_save_png(void* ui_view, const uint8_t* png, size_t size) {
  Browser* browser = BrowserWithView(ui_view);
  if (!browser || !browser->window) {
    return;
  }
  NSData* data = [NSData dataWithBytes:png length:size];
  NSSavePanel* panel = [NSSavePanel savePanel];
  panel.allowedContentTypes = @[ UTTypePNG ];
  panel.nameFieldStringValue = CaptureName();
  [panel beginSheetModalForWindow:browser->window
                completionHandler:^(NSModalResponse response) {
                  if (response == NSModalResponseOK) {
                    [data writeToURL:panel.URL atomically:YES];
                  }
                }];
  fprintf(stderr, "browser: capture save panel shown\n");
}

void host_share_png(const uint8_t* png,
                    size_t size,
                    void* view,
                    double x,
                    double y,
                    double width,
                    double height,
                    bool messages) {
  // Shared as a file, so what takes it gets the capture's name, in a folder of its own.
  NSString* folder = [NSTemporaryDirectory()
      stringByAppendingPathComponent:NSUUID.UUID.UUIDString];
  [NSFileManager.defaultManager createDirectoryAtPath:folder
                          withIntermediateDirectories:YES
                                           attributes:nil
                                                error:nil];
  NSURL* file = [NSURL fileURLWithPath:[folder stringByAppendingPathComponent:CaptureName()]];
  if (![[NSData dataWithBytes:png length:size] writeToURL:file atomically:YES]) {
    return;
  }
  if (messages) {
    [[NSSharingService sharingServiceNamed:NSSharingServiceNameComposeMessage]
        performWithItems:@[ file ]];
    fprintf(stderr, "browser: capture sent to messages\n");
    return;
  }
  NSView* anchor = (__bridge NSView*)view;
  if (!anchor.window) {
    return;
  }
  NSRect rect = NSMakeRect(x, anchor.flipped ? y : NSHeight(anchor.bounds) - y - height, width,
                           height);
  NSSharingServicePicker* picker = [[NSSharingServicePicker alloc] initWithItems:@[ file ]];
  [picker showRelativeToRect:rect ofView:anchor preferredEdge:NSRectEdgeMinY];
  fprintf(stderr, "browser: capture share menu shown\n");
}

void host_detent(void) {
  [NSHapticFeedbackManager.defaultPerformer
      performFeedbackPattern:NSHapticFeedbackPatternAlignment
             performanceTime:NSHapticFeedbackPerformanceTimeNow];
}

uint32_t host_accent_color(void) {
  NSColor* color = [NSColor.controlAccentColor colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
  auto channel = [](CGFloat value) {
    return (uint32_t)lround(std::clamp(value, (CGFloat)0, (CGFloat)1) * 255);
  };
  return channel(color.redComponent) << 16 | channel(color.greenComponent) << 8 |
         channel(color.blueComponent);
}

void host_watch_accent_color(void (*changed)(void)) {
  [NSNotificationCenter.defaultCenter addObserverForName:NSSystemColorsDidChangeNotification
                                                  object:nil
                                                   queue:nil
                                              usingBlock:^(NSNotification*) {
                                                changed();
                                              }];
}

bool host_trash(const char* path) {
  NSURL* file = [NSURL fileURLWithPath:@(path)];
  return [NSFileManager.defaultManager trashItemAtURL:file resultingItemURL:nil error:nil] ||
         ![file checkResourceIsReachableAndReturnError:nil];
}

bool host_drag_file(const char* path, void (*ended)(bool taken)) {
  // Kept until the next drag, as the session doesn't keep its source.
  static HostFileDragSource* source;
  NSString* file = @(path);
  NSEvent* event = NSApp.currentEvent;
  Browser* browser = BrowserOf(event.window);
  if (!browser || event.type != NSEventTypeLeftMouseDragged ||
      ![NSFileManager.defaultManager fileExistsAtPath:file]) {
    return false;
  }
  NSView* view = browser->ui_view;
  NSDraggingItem* item =
      [[NSDraggingItem alloc] initWithPasteboardWriter:[NSURL fileURLWithPath:file]];
  // The file's icon under the pointer, 32 points square, as Finder drags one.
  NSPoint point = [view convertPoint:event.locationInWindow fromView:nil];
  [item setDraggingFrame:NSMakeRect(point.x - 16, point.y - 16, 32, 32)
                contents:[NSWorkspace.sharedWorkspace iconForFile:file]];
  source = [[HostFileDragSource alloc] init];
  source.ended = ended;
  return [view beginDraggingSessionWithItems:@[ item ] event:event source:source] != nil;
}

// |image| as a PNG, or nil.
static NSData* PNGOf(CGImageRef image) {
  NSMutableData* png = [NSMutableData data];
  CGImageDestinationRef destination = CGImageDestinationCreateWithData(
      (__bridge CFMutableDataRef)png, CFSTR("public.png"), 1, nullptr);
  if (!destination) {
    return nil;
  }
  CGImageDestinationAddImage(destination, image, nullptr);
  bool written = CGImageDestinationFinalize(destination);
  CFRelease(destination);
  return written ? png : nil;
}

void host_file_preview(const char* path, uint64_t id,
                       void (*done)(uint64_t id, const uint8_t* png, size_t size, double width,
                                    double height, bool image)) {
  NSString* file = @(path);
  // A photo's thumbnail reads and decodes its file, which can take a while for a large one.
  dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
    NSData* thumbnail = nil;
    size_t width = 0, height = 0;
    CGImageSourceRef source =
        CGImageSourceCreateWithURL((__bridge CFURLRef)[NSURL fileURLWithPath:file], nullptr);
    if (source) {
      NSDictionary* options = @{
        (__bridge NSString*)kCGImageSourceCreateThumbnailFromImageAlways : @YES,
        (__bridge NSString*)kCGImageSourceCreateThumbnailWithTransform : @YES,
        (__bridge NSString*)kCGImageSourceThumbnailMaxPixelSize : @96,
      };
      CGImageRef image = CGImageSourceGetCount(source) > 0
                             ? CGImageSourceCreateThumbnailAtIndex(
                                   source, 0, (__bridge CFDictionaryRef)options)
                             : nullptr;
      if (image) {
        thumbnail = PNGOf(image);
        width = CGImageGetWidth(image);
        height = CGImageGetHeight(image);
        CGImageRelease(image);
      }
      CFRelease(source);
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      if (thumbnail) {
        done(id, static_cast<const uint8_t*>(thumbnail.bytes), thumbnail.length, width, height,
             true);
        return;
      }
      if (![NSFileManager.defaultManager fileExistsAtPath:file]) {
        done(id, nullptr, 0, 0, 0, false);
        return;
      }
      // The icon at 26 points on a 2× display, drawn as Finder draws it.
      const NSInteger side = 52;
      NSBitmapImageRep* bitmap =
          [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:nullptr
                                                  pixelsWide:side
                                                  pixelsHigh:side
                                               bitsPerSample:8
                                             samplesPerPixel:4
                                                    hasAlpha:YES
                                                    isPlanar:NO
                                              colorSpaceName:NSDeviceRGBColorSpace
                                                 bytesPerRow:0
                                                bitsPerPixel:0];
      [NSGraphicsContext saveGraphicsState];
      NSGraphicsContext.currentContext =
          [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
      [[NSWorkspace.sharedWorkspace iconForFile:file] drawInRect:NSMakeRect(0, 0, side, side)];
      [NSGraphicsContext restoreGraphicsState];
      NSData* png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
      done(id, static_cast<const uint8_t*>(png.bytes), png.length, side, side, false);
    });
  });
}

void host_copy(const char* text) {
  NSPasteboard* pasteboard = CopyPasteboard();
  [pasteboard clearContents];
  [pasteboard setString:@(text) forType:NSPasteboardTypeString];
}

void host_open_in_safari(const char* url) {
  NSURL* page = [NSURL URLWithString:@(url)];
  // Only web pages go to another app.
  if (![page.scheme isEqualToString:@"http"] && ![page.scheme isEqualToString:@"https"]) {
    return;
  }
  // check.sh reads this line. A driven run opens nothing, so checks never launch Safari.
  fprintf(stderr, "browser: open in safari %s\n", url);
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  NSURL* safari =
      [NSWorkspace.sharedWorkspace URLForApplicationWithBundleIdentifier:@"com.apple.Safari"];
  if (!safari) {
    fprintf(stderr, "browser: no Safari to open %s\n", url);
    return;
  }
  [NSWorkspace.sharedWorkspace openURLs:@[ page ]
                   withApplicationAtURL:safari
                          configuration:[NSWorkspaceOpenConfiguration configuration]
                      completionHandler:nil];
}

void host_share(void* ui_view,
                double x,
                double y,
                double width,
                double height,
                const char* url,
                const char* title) {
  Browser* browser = BrowserWithView(ui_view);
  NSURL* link = [NSURL URLWithString:@(url)];
  if (!browser || !browser->window || !link) {
    return;
  }
  // check.sh reads this line. A driven run doesn't show the menu, which would wait on a choice.
  fprintf(stderr, "browser: share %s\n", url);
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  // Kept while the menu shows, which doesn't keep its delegate.
  static NSSharingServicePicker* picker;
  static HostShareDelegate* delegate;
  delegate = [[HostShareDelegate alloc] init];
  delegate.title = @(title);
  picker = [[NSSharingServicePicker alloc] initWithItems:@[ link ]];
  picker.delegate = delegate;
  NSView* content = browser->window.contentView;
  // From the content's top left, under the tile it hangs from.
  const NSRect rect = NSMakeRect(x, content.flipped ? y : NSHeight(content.bounds) - y - height,
                                 width, height);
  [picker showRelativeToRect:rect
                      ofView:content
               preferredEdge:content.flipped ? NSRectEdgeMaxY : NSRectEdgeMinY];
}

namespace {

// The system's share services for |link|, as its share menu lists them. AppKit's own Share item
// can't be a submenu of the UI's menus, and this is the list its picker shows.
NSArray<NSSharingService*>* ShareServices(NSURL* link) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
  return [NSSharingService sharingServicesForItems:@[ link ]];
#pragma clang diagnostic pop
}

}  // namespace

void host_share_services(const char* url,
                         void (*each)(void* context, const char* name),
                         void* context) {
  NSURL* link = [NSURL URLWithString:@(url)];
  if (!link) {
    return;
  }
  for (NSSharingService* service in ShareServices(link)) {
    each(context, service.menuItemTitle.UTF8String ?: "");
  }
}

void host_share_with(const char* url, const char* title, size_t index) {
  NSURL* link = [NSURL URLWithString:@(url)];
  NSArray<NSSharingService*>* services = link ? ShareServices(link) : @[];
  if (index >= services.count) {
    return;
  }
  NSSharingService* service = services[index];
  // check.sh reads this line. A driven run shares nothing, as the service takes the page to
  // another app.
  fprintf(stderr, "browser: share %s with %s\n", url, service.menuItemTitle.UTF8String ?: "");
  if (getenv("BROWSER_DRIVE")) {
    return;
  }
  service.subject = @(title);
  [service performWithItems:@[ link ]];
}

void host_day_starts(double starts[4]) {
  NSCalendar* calendar = NSCalendar.currentCalendar;
  NSDate* now = NSDate.date;
  NSDate* today = [calendar startOfDayForDate:now];
  NSDate* week = today;
  [calendar rangeOfUnit:NSCalendarUnitWeekOfYear startDate:&week interval:nil forDate:now];
  NSDate* dates[] = {
      today,
      [calendar dateByAddingUnit:NSCalendarUnitDay value:-1 toDate:today options:0],
      week,
      [calendar dateByAddingUnit:NSCalendarUnitWeekOfYear value:-1 toDate:week options:0],
  };
  for (int i = 0; i < 4; i++) {
    starts[i] = dates[i].timeIntervalSince1970 * 1000;
  }
}
