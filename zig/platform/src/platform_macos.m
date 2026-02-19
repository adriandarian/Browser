#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform.h"
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

@interface TesseraWindowDelegate : NSObject <NSWindowDelegate>
@end

@interface TesseraView : NSView
@property(nonatomic, strong) NSData *frameData;
@property(nonatomic) uint32_t frameWidth;
@property(nonatomic) uint32_t frameHeight;
@property(nonatomic) uint32_t frameStride;
@end

static NSWindow *g_window = nil;
static TesseraView *g_view = nil;
static TesseraWindowDelegate *g_delegate = nil;
static bool g_initialized = false;
static bool g_pending_quit = false;
static bool g_pending_resize = false;
static bool g_pending_escape_down = false;
static bool g_pending_escape_up = false;
static uint32_t g_resize_width = 0;
static uint32_t g_resize_height = 0;

@implementation TesseraWindowDelegate
- (void)windowWillClose:(NSNotification *)notification {
  (void)notification;
  g_pending_quit = true;
}

- (void)windowDidResize:(NSNotification *)notification {
  (void)notification;
  if (g_view == nil) {
    return;
  }

  NSRect bounds = [g_view bounds];
  uint32_t width = (uint32_t)bounds.size.width;
  uint32_t height = (uint32_t)bounds.size.height;
  if (width == 0 || height == 0) {
    return;
  }

  g_resize_width = width;
  g_resize_height = height;
  g_pending_resize = true;
}
@end

@implementation TesseraView
- (BOOL)isFlipped { return YES; }

- (BOOL)acceptsFirstResponder { return YES; }

- (void)drawRect:(NSRect)dirtyRect {
  (void)dirtyRect;
  if (self.frameData == nil || self.frameWidth == 0 || self.frameHeight == 0 || self.frameStride == 0) {
    [[NSColor blackColor] setFill];
    NSRectFill(self.bounds);
    return;
  }

  CGDataProviderRef provider =
      CGDataProviderCreateWithData(NULL, [self.frameData bytes], self.frameStride * self.frameHeight, NULL);
  if (provider == NULL) {
    return;
  }

  CGColorSpaceRef colorSpace = CGColorSpaceCreateDeviceRGB();
  CGImageRef image = CGImageCreate(self.frameWidth, self.frameHeight, 8, 32, self.frameStride, colorSpace,
                                   kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big, provider, NULL,
                                   false, kCGRenderingIntentDefault);

  if (image != NULL) {
    CGContextRef context = [[NSGraphicsContext currentContext] CGContext];
    CGContextDrawImage(context, CGRectMake(0, 0, self.frameWidth, self.frameHeight), image);
    CGImageRelease(image);
  }

  CGColorSpaceRelease(colorSpace);
  CGDataProviderRelease(provider);
}
@end

static void pump_system_events(void) {
  while (true) {
    NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                        untilDate:[NSDate distantPast]
                                           inMode:NSDefaultRunLoopMode
                                          dequeue:YES];
    if (event == nil) {
      break;
    }

    if ([event type] == NSEventTypeKeyDown && [event keyCode] == 53) {
      g_pending_escape_down = true;
      continue;
    }

    if ([event type] == NSEventTypeKeyUp && [event keyCode] == 53) {
      g_pending_escape_up = true;
      continue;
    }

    [NSApp sendEvent:event];
  }

  [NSApp updateWindows];
}

bool platform_init_window(const platform_config *config) {
  if (config == NULL || config->abi_version != PLATFORM_ABI_VERSION || config->width == 0 ||
      config->height == 0) {
    return false;
  }

  @autoreleasepool {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp finishLaunching];

    NSUInteger styleMask = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                           NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable;
    NSRect frame = NSMakeRect(0, 0, config->width, config->height);

    g_window = [[NSWindow alloc] initWithContentRect:frame
                                            styleMask:styleMask
                                              backing:NSBackingStoreBuffered
                                                defer:NO];
    if (g_window == nil) {
      return false;
    }

    NSString *title = @"Tessera";
    if (config->title_utf8 != NULL) {
      NSString *utf8Title = [NSString stringWithUTF8String:config->title_utf8];
      if (utf8Title != nil) {
        title = utf8Title;
      }
    }
    [g_window setTitle:title];

    g_view = [[TesseraView alloc] initWithFrame:frame];
    if (g_view == nil) {
      return false;
    }

    g_delegate = [[TesseraWindowDelegate alloc] init];
    [g_window setDelegate:g_delegate];
    [g_window setContentView:g_view];
    [g_window makeFirstResponder:g_view];
    [g_window center];
    [g_window makeKeyAndOrderFront:nil];

    [NSApp activateIgnoringOtherApps:YES];

    g_pending_quit = false;
    g_pending_resize = true;
    g_pending_escape_down = false;
    g_pending_escape_up = false;
    g_resize_width = config->width;
    g_resize_height = config->height;
    g_initialized = true;
    return true;
  }
}

bool platform_poll_event(platform_event *out_event) {
  if (!g_initialized || out_event == NULL) {
    return false;
  }

  memset(out_event, 0, sizeof(*out_event));

  @autoreleasepool {
    pump_system_events();

    if (g_pending_quit) {
      g_pending_quit = false;
      out_event->kind = PLATFORM_EVENT_QUIT;
      return true;
    }

    if (g_pending_resize) {
      g_pending_resize = false;
      out_event->kind = PLATFORM_EVENT_RESIZE;
      out_event->width = g_resize_width;
      out_event->height = g_resize_height;
      return true;
    }

    if (g_pending_escape_down) {
      g_pending_escape_down = false;
      out_event->kind = PLATFORM_EVENT_KEY_DOWN;
      out_event->key_code = PLATFORM_KEY_ESCAPE;
      return true;
    }

    if (g_pending_escape_up) {
      g_pending_escape_up = false;
      out_event->kind = PLATFORM_EVENT_KEY_UP;
      out_event->key_code = PLATFORM_KEY_ESCAPE;
      return true;
    }

    return false;
  }
}

bool platform_present_frame(const platform_frame *frame) {
  if (!g_initialized || g_view == nil || frame == NULL || frame->pixels_rgba8 == NULL || frame->width == 0 ||
      frame->height == 0 || frame->stride_bytes < (frame->width * 4)) {
    return false;
  }

  @autoreleasepool {
    size_t byteCount = (size_t)frame->stride_bytes * (size_t)frame->height;
    NSData *copiedFrame = [NSData dataWithBytes:frame->pixels_rgba8 length:byteCount];
    if (copiedFrame == nil) {
      return false;
    }

    g_view.frameData = copiedFrame;
    g_view.frameWidth = frame->width;
    g_view.frameHeight = frame->height;
    g_view.frameStride = frame->stride_bytes;

    [g_view setNeedsDisplay:YES];
    [g_view displayIfNeeded];
    return true;
  }
}

void platform_shutdown(void) {
  @autoreleasepool {
    if (g_window != nil) {
      [g_window orderOut:nil];
      [g_window close];
    }

    g_window = nil;
    g_view = nil;
    g_delegate = nil;
    g_pending_quit = false;
    g_pending_resize = false;
    g_pending_escape_down = false;
    g_pending_escape_up = false;
    g_resize_width = 0;
    g_resize_height = 0;
    g_initialized = false;
  }
}
