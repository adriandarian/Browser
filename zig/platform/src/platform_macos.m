#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform.h"
#include <stdint.h>
#include <string.h>

@interface TesseraView : NSView
@property(nonatomic) const uint8_t *pixels;
@property(nonatomic) uint32_t width;
@property(nonatomic) uint32_t height;
@property(nonatomic) uint32_t stride;
@end

@implementation TesseraView
- (BOOL)isFlipped { return YES; }
- (void)drawRect:(NSRect)dirtyRect {
  (void)dirtyRect;
  if (!self.pixels || self.width == 0 || self.height == 0) {
    [[NSColor blackColor] setFill];
    NSRectFill(self.bounds);
    return;
  }

  CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
  CGDataProviderRef provider =
      CGDataProviderCreateWithData(NULL, self.pixels, self.stride * self.height, NULL);
  CGImageRef image = CGImageCreate(self.width, self.height, 8, 32, self.stride, color_space,
                                   kCGImageAlphaPremultipliedLast | kCGBitmapByteOrderDefault,
                                   provider, NULL, false, kCGRenderingIntentDefault);

  CGContextRef ctx = [[NSGraphicsContext currentContext] CGContext];
  CGContextDrawImage(ctx, CGRectMake(0, 0, self.width, self.height), image);

  CGImageRelease(image);
  CGDataProviderRelease(provider);
  CGColorSpaceRelease(color_space);
}
@end

static NSWindow *g_window = nil;
static TesseraView *g_view = nil;
static bool g_initialized = false;
static bool g_seen_close = false;

uint32_t platform_get_abi_version(void) { return PLATFORM_ABI_VERSION; }

uint8_t platform_init_window(const platform_config *config) {
  if (config == NULL || config->struct_size < sizeof(platform_config) ||
      config->abi_version != PLATFORM_ABI_VERSION) {
    return PLATFORM_FALSE;
  }

  @autoreleasepool {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];

    NSUInteger style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                       NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable;
    NSRect frame = NSMakeRect(0, 0, config->width, config->height);

    g_window = [[NSWindow alloc] initWithContentRect:frame
                                            styleMask:style
                                              backing:NSBackingStoreBuffered
                                                defer:NO];

    NSString *title = @"Tessera";
    if (config->title_utf8 != NULL) {
      title = [NSString stringWithUTF8String:config->title_utf8];
    }
    [g_window setTitle:title];

    g_view = [[TesseraView alloc] initWithFrame:frame];
    [g_window setContentView:g_view];
    [g_window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];

    g_initialized = true;
    return PLATFORM_TRUE;
  }
}

uint8_t platform_poll_event(platform_event *out_event) {
  if (!g_initialized || out_event == NULL ||
      out_event->struct_size < sizeof(platform_event)) {
    return PLATFORM_FALSE;
  }

  memset(out_event, 0, sizeof(*out_event));
  out_event->struct_size = sizeof(platform_event);

  @autoreleasepool {
    NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                        untilDate:[NSDate distantPast]
                                           inMode:NSDefaultRunLoopMode
                                          dequeue:YES];

    if (event == nil) {
      if (g_seen_close) {
        out_event->kind = PLATFORM_EVENT_QUIT;
        g_seen_close = false;
        return PLATFORM_TRUE;
      }
      return PLATFORM_FALSE;
    }

    if ([event type] == NSEventTypeKeyDown) {
      if ([event keyCode] == 53) {
        out_event->kind = PLATFORM_EVENT_KEY_DOWN;
        out_event->key_code = PLATFORM_KEY_ESCAPE;
      } else {
        out_event->kind = PLATFORM_EVENT_KEY_DOWN;
      }
      return PLATFORM_TRUE;
    }

    if ([event type] == NSEventTypeKeyUp) {
      if ([event keyCode] == 53) {
        out_event->kind = PLATFORM_EVENT_KEY_UP;
        out_event->key_code = PLATFORM_KEY_ESCAPE;
      } else {
        out_event->kind = PLATFORM_EVENT_KEY_UP;
      }
      return PLATFORM_TRUE;
    }

    if ([event type] == NSEventTypeApplicationDefined && [event subtype] == 0) {
      out_event->kind = PLATFORM_EVENT_QUIT;
      return PLATFORM_TRUE;
    }

    [NSApp sendEvent:event];
    [NSApp updateWindows];

    if (![g_window isVisible]) {
      out_event->kind = PLATFORM_EVENT_QUIT;
      return PLATFORM_TRUE;
    }

    NSRect bounds = [g_view bounds];
    out_event->kind = PLATFORM_EVENT_RESIZE;
    out_event->width = (uint32_t)bounds.size.width;
    out_event->height = (uint32_t)bounds.size.height;
    return PLATFORM_TRUE;
  }
}

uint8_t platform_present_frame(const platform_frame *frame) {
  if (!g_initialized || frame == NULL || frame->struct_size < sizeof(platform_frame) ||
      frame->pixels_rgba8 == NULL) {
    return PLATFORM_FALSE;
  }

  @autoreleasepool {
    g_view.pixels = frame->pixels_rgba8;
    g_view.width = frame->width;
    g_view.height = frame->height;
    g_view.stride = frame->stride_bytes;
    [g_view setNeedsDisplay:YES];
    [g_view displayIfNeeded];
    return PLATFORM_TRUE;
  }
}

void platform_shutdown(void) {
  @autoreleasepool {
    if (g_window != nil) {
      [g_window close];
      g_window = nil;
    }
    g_view = nil;
    g_initialized = false;
  }
}
