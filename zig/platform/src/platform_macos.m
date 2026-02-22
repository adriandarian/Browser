#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform.h"
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

@interface BrowserView : NSView
@property(nonatomic) const uint8_t *pixels;
@property(nonatomic) uint32_t width;
@property(nonatomic) uint32_t height;
@property(nonatomic) uint32_t stride;
@end

@implementation BrowserView
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

#define EVENT_CAPACITY 256
static platform_event g_events[EVENT_CAPACITY];
static unsigned int g_event_head = 0;
static unsigned int g_event_tail = 0;

static NSWindow *g_window = nil;
static BrowserView *g_view = nil;
static bool g_initialized = false;
static bool g_should_quit = false;
static bool g_quit_enqueued = false;
static uint32_t g_last_width = 0;
static uint32_t g_last_height = 0;

static void push_event(const platform_event *event) {
  unsigned int next = (g_event_tail + 1u) % EVENT_CAPACITY;
  if (next == g_event_head) {
    return;
  }
  g_events[g_event_tail] = *event;
  g_event_tail = next;
}

static bool pop_event(platform_event *event) {
  if (g_event_head == g_event_tail) {
    return false;
  }
  *event = g_events[g_event_head];
  g_event_head = (g_event_head + 1u) % EVENT_CAPACITY;
  return true;
}

static void enqueue_quit_if_needed(void) {
  if (g_quit_enqueued) {
    return;
  }
  platform_event event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(platform_event);
  event.kind = PLATFORM_EVENT_QUIT;
  push_event(&event);
  g_quit_enqueued = true;
}

@interface BrowserWindowDelegate : NSObject <NSWindowDelegate>
@end

@implementation BrowserWindowDelegate
- (void)windowWillClose:(NSNotification *)notification {
  (void)notification;
  g_should_quit = true;
}
@end

static BrowserWindowDelegate *g_window_delegate = nil;

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

    NSString *title = @"Browser";
    if (config->title_utf8 != NULL) {
      title = [NSString stringWithUTF8String:config->title_utf8];
    }
    [g_window setTitle:title];

    g_window_delegate = [[BrowserWindowDelegate alloc] init];
    [g_window setDelegate:g_window_delegate];

    g_view = [[BrowserView alloc] initWithFrame:frame];
    [g_window setContentView:g_view];
    [g_window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];

    g_event_head = 0;
    g_event_tail = 0;
    g_should_quit = false;
    g_quit_enqueued = false;
    g_last_width = config->width;
    g_last_height = config->height;

    g_initialized = true;
    return PLATFORM_TRUE;
  }
}

uint8_t platform_poll_event(platform_event *out_event) {
  if (!g_initialized || out_event == NULL || out_event->struct_size < sizeof(platform_event)) {
    return PLATFORM_FALSE;
  }

  memset(out_event, 0, sizeof(*out_event));
  out_event->struct_size = sizeof(platform_event);

  if (pop_event(out_event)) {
    return PLATFORM_TRUE;
  }

  @autoreleasepool {
    NSEvent *event = nil;
    while ((event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                       untilDate:[NSDate distantPast]
                                          inMode:NSDefaultRunLoopMode
                                         dequeue:YES])) {
      platform_event next;
      memset(&next, 0, sizeof(next));
      next.struct_size = sizeof(platform_event);

      if ([event type] == NSEventTypeKeyDown) {
        next.kind = PLATFORM_EVENT_KEY_DOWN;
        next.key_code = ([event keyCode] == 53) ? PLATFORM_KEY_ESCAPE : PLATFORM_KEY_UNKNOWN;
        push_event(&next);
      } else if ([event type] == NSEventTypeKeyUp) {
        next.kind = PLATFORM_EVENT_KEY_UP;
        next.key_code = ([event keyCode] == 53) ? PLATFORM_KEY_ESCAPE : PLATFORM_KEY_UNKNOWN;
        push_event(&next);
      } else {
        [NSApp sendEvent:event];
      }
    }

    [NSApp updateWindows];

    if (g_should_quit || ![g_window isVisible]) {
      enqueue_quit_if_needed();
    }

    NSRect bounds = [g_view bounds];
    uint32_t width = (uint32_t)bounds.size.width;
    uint32_t height = (uint32_t)bounds.size.height;
    if (width > 0 && height > 0 && (width != g_last_width || height != g_last_height)) {
      g_last_width = width;
      g_last_height = height;

      platform_event resize;
      memset(&resize, 0, sizeof(resize));
      resize.struct_size = sizeof(platform_event);
      resize.kind = PLATFORM_EVENT_RESIZE;
      resize.width = width;
      resize.height = height;
      push_event(&resize);
    }

    if (pop_event(out_event)) {
      return PLATFORM_TRUE;
    }
  }

  return PLATFORM_FALSE;
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
      [g_window setDelegate:nil];
      [g_window close];
      g_window = nil;
    }
    g_window_delegate = nil;
    g_view = nil;
    g_initialized = false;
    g_should_quit = false;
    g_quit_enqueued = false;
    g_event_head = 0;
    g_event_tail = 0;
  }
}
