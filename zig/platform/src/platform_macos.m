#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform.h"
#include <stdlib.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

@interface BrowserView : NSView
@property(nonatomic) uint8_t *pixels;
@property(nonatomic) size_t pixels_capacity;
@property(nonatomic) uint32_t frame_width;
@property(nonatomic) uint32_t frame_height;
@property(nonatomic) uint32_t frame_stride;
- (BOOL)updateFrame:(const platform_frame *)frame;
@end

@implementation BrowserView
- (BOOL)isFlipped { return YES; }
- (instancetype)initWithFrame:(NSRect)frameRect {
  self = [super initWithFrame:frameRect];
  if (self) {
    _pixels = NULL;
    _pixels_capacity = 0;
    _frame_width = 0;
    _frame_height = 0;
    _frame_stride = 0;
  }
  return self;
}

- (void)dealloc {
  free(_pixels);
  _pixels = NULL;
  _pixels_capacity = 0;
}

- (BOOL)updateFrame:(const platform_frame *)frame {
  if (frame == NULL || frame->pixels_rgba8 == NULL || frame->width == 0 || frame->height == 0) {
    return NO;
  }

  uint32_t min_stride = frame->width * 4u;
  if (frame->stride_bytes < min_stride) {
    return NO;
  }

  size_t row_bytes = (size_t)min_stride;
  size_t required = row_bytes * (size_t)frame->height;
  if (required == 0) {
    return NO;
  }

  if (required > self.pixels_capacity) {
    uint8_t *next = (uint8_t *)realloc(self.pixels, required);
    if (next == NULL) {
      return NO;
    }
    self.pixels = next;
    self.pixels_capacity = required;
  }

  if (frame->stride_bytes == min_stride) {
    memcpy(self.pixels, frame->pixels_rgba8, required);
  } else {
    for (uint32_t y = 0; y < frame->height; ++y) {
      const uint8_t *src = frame->pixels_rgba8 + ((size_t)y * (size_t)frame->stride_bytes);
      uint8_t *dst = self.pixels + ((size_t)y * row_bytes);
      memcpy(dst, src, row_bytes);
    }
  }

  self.frame_width = frame->width;
  self.frame_height = frame->height;
  self.frame_stride = min_stride;
  return YES;
}

- (void)drawRect:(NSRect)dirtyRect {
  (void)dirtyRect;
  if (self.pixels == NULL || self.frame_width == 0 || self.frame_height == 0 ||
      self.frame_stride == 0) {
    [[NSColor blackColor] setFill];
    NSRectFill(self.bounds);
    return;
  }

  CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
  CGDataProviderRef provider =
      CGDataProviderCreateWithData(NULL, self.pixels,
                                   (size_t)self.frame_stride * (size_t)self.frame_height, NULL);
  CGImageRef image = CGImageCreate(
      self.frame_width, self.frame_height, 8, 32, self.frame_stride, color_space,
      kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big,
                                   provider, NULL, false, kCGRenderingIntentDefault);

  CGContextRef ctx = [[NSGraphicsContext currentContext] CGContext];
  CGContextDrawImage(ctx, NSRectToCGRect(self.bounds), image);

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
  enqueue_quit_if_needed();
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

  if (width != g_last_width || height != g_last_height) {
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
    [NSApp finishLaunching];

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
      if ([event type] == NSEventTypeKeyDown) {
        if ([event keyCode] == 53) {
          platform_event next;
          memset(&next, 0, sizeof(next));
          next.struct_size = sizeof(platform_event);
          next.kind = PLATFORM_EVENT_KEY_DOWN;
          next.key_code = PLATFORM_KEY_ESCAPE;
          push_event(&next);
        }
      } else {
        [NSApp sendEvent:event];
      }
    }

    [NSApp updateWindows];

    if (g_should_quit || ![g_window isVisible]) {
      enqueue_quit_if_needed();
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
    if (![g_view updateFrame:frame]) {
      return PLATFORM_FALSE;
    }
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
    g_last_width = 0;
    g_last_height = 0;
    g_event_head = 0;
    g_event_tail = 0;
  }
}
