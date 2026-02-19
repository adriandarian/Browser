#import <AppKit/AppKit.h>
#import <CoreGraphics/CoreGraphics.h>

#include "platform.h"
#include <stdbool.h>
#include <stdint.h>
#include <string.h>

#define TESSERA_EVENT_QUEUE_CAPACITY 64

typedef struct TesseraEventQueue {
  platform_event events[TESSERA_EVENT_QUEUE_CAPACITY];
  uint32_t head;
  uint32_t tail;
} TesseraEventQueue;

static NSWindow *g_window = nil;
static NSView *g_view = nil;
static bool g_initialized = false;
static TesseraEventQueue g_event_queue = {0};

static bool event_queue_is_empty(void) { return g_event_queue.head == g_event_queue.tail; }

static bool event_queue_push(const platform_event *event) {
  uint32_t next_tail = (g_event_queue.tail + 1u) % TESSERA_EVENT_QUEUE_CAPACITY;
  if (next_tail == g_event_queue.head) {
    return false;
  }

  g_event_queue.events[g_event_queue.tail] = *event;
  g_event_queue.tail = next_tail;
  return true;
}

static bool event_queue_pop(platform_event *out_event) {
  if (event_queue_is_empty()) {
    return false;
  }

  *out_event = g_event_queue.events[g_event_queue.head];
  g_event_queue.head = (g_event_queue.head + 1u) % TESSERA_EVENT_QUEUE_CAPACITY;
  return true;
}

static void enqueue_quit_event(void) {
  platform_event event = {0};
  event.kind = PLATFORM_EVENT_QUIT;
  (void)event_queue_push(&event);
}

static void enqueue_resize_event(uint32_t width, uint32_t height) {
  if (width == 0 || height == 0) {
    return;
  }

  platform_event event = {0};
  event.kind = PLATFORM_EVENT_RESIZE;
  event.width = width;
  event.height = height;
  (void)event_queue_push(&event);
}

static void enqueue_escape_keydown_event(void) {
  platform_event event = {0};
  event.kind = PLATFORM_EVENT_KEY_DOWN;
  event.key_code = PLATFORM_KEY_ESCAPE;
  (void)event_queue_push(&event);
}

@interface TesseraWindowDelegate : NSObject <NSWindowDelegate>
@end

@interface TesseraView : NSView
@property(nonatomic, strong) NSData *frameData;
@property(nonatomic) uint32_t frameWidth;
@property(nonatomic) uint32_t frameHeight;
@property(nonatomic) uint32_t frameStride;
@end

@implementation TesseraWindowDelegate
- (void)windowWillClose:(NSNotification *)notification {
  (void)notification;
  enqueue_quit_event();
}

- (void)windowDidResize:(NSNotification *)notification {
  (void)notification;
  if (g_view == nil) {
    return;
  }

  NSRect bounds = [g_view bounds];
  enqueue_resize_event((uint32_t)bounds.size.width, (uint32_t)bounds.size.height);
}
@end

@implementation TesseraView
- (BOOL)isFlipped { return YES; }

- (BOOL)acceptsFirstResponder { return YES; }

- (void)keyDown:(NSEvent *)event {
  if (event != nil && [event keyCode] == 53) {
    enqueue_escape_keydown_event();
    return;
  }

  [super keyDown:event];
}

- (void)keyUp:(NSEvent *)event {
  [super keyUp:event];
}

- (void)drawRect:(NSRect)dirtyRect {
  (void)dirtyRect;

  if (self.frameData == nil || self.frameWidth == 0 || self.frameHeight == 0 || self.frameStride == 0) {
    [[NSColor blackColor] setFill];
    NSRectFill(self.bounds);
    return;
  }

  const size_t data_size = (size_t)self.frameStride * (size_t)self.frameHeight;
  CGDataProviderRef provider = CGDataProviderCreateWithData(NULL, [self.frameData bytes], data_size, NULL);
  if (provider == NULL) {
    return;
  }

  CGColorSpaceRef color_space = CGColorSpaceCreateDeviceRGB();
  CGImageRef image = CGImageCreate(self.frameWidth, self.frameHeight, 8, 32, self.frameStride, color_space,
                                   kCGImageAlphaLast | kCGBitmapByteOrder32Big, provider, NULL,
                                   false, kCGRenderingIntentDefault);

  if (image != NULL) {
    CGContextRef context = [[NSGraphicsContext currentContext] CGContext];
    CGContextDrawImage(context, CGRectMake(0, 0, self.frameWidth, self.frameHeight), image);
    CGImageRelease(image);
  }

  CGColorSpaceRelease(color_space);
  CGDataProviderRelease(provider);
}
@end

static TesseraWindowDelegate *g_delegate = nil;

static void pump_system_events(void) {
  while (true) {
    NSEvent *event = [NSApp nextEventMatchingMask:NSEventMaskAny
                                        untilDate:[NSDate distantPast]
                                           inMode:NSDefaultRunLoopMode
                                          dequeue:YES];
    if (event == nil) {
      break;
    }

    [NSApp sendEvent:event];
  }

  [NSApp updateWindows];
}

bool platform_init_window(const platform_config *config) {
  if (g_initialized) {
    return false;
  }

  if (config == NULL || config->abi_version != PLATFORM_ABI_VERSION || config->width == 0 ||
      config->height == 0) {
    return false;
  }

  @autoreleasepool {
    [NSApplication sharedApplication];
    [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
    [NSApp finishLaunching];

    NSRect frame = NSMakeRect(0, 0, config->width, config->height);
    const NSUInteger style_mask = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                                  NSWindowStyleMaskResizable | NSWindowStyleMaskMiniaturizable;

    g_window = [[NSWindow alloc] initWithContentRect:frame
                                            styleMask:style_mask
                                              backing:NSBackingStoreBuffered
                                                defer:NO];
    if (g_window == nil) {
      return false;
    }

    NSString *title = @"Tessera";
    if (config->title_utf8 != NULL) {
      NSString *utf8_title = [NSString stringWithUTF8String:config->title_utf8];
      if (utf8_title != nil) {
        title = utf8_title;
      }
    }
    [g_window setTitle:title];

    TesseraView *view = [[TesseraView alloc] initWithFrame:frame];
    if (view == nil) {
      return false;
    }

    g_delegate = [[TesseraWindowDelegate alloc] init];
    g_view = view;
    [g_window setDelegate:g_delegate];
    [g_window setContentView:g_view];
    [g_window makeFirstResponder:g_view];
    [g_window center];
    [g_window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];

    memset(&g_event_queue, 0, sizeof(g_event_queue));
    enqueue_resize_event(config->width, config->height);
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
    return event_queue_pop(out_event);
  }
}

bool platform_present_frame(const platform_frame *frame) {
  if (!g_initialized || g_view == nil || frame == NULL || frame->pixels_rgba8 == NULL || frame->width == 0 ||
      frame->height == 0 || frame->stride_bytes < (frame->width * 4)) {
    return false;
  }

  @autoreleasepool {
    TesseraView *view = (TesseraView *)g_view;
    const size_t bytes = (size_t)frame->stride_bytes * (size_t)frame->height;
    NSData *copy = [NSData dataWithBytes:frame->pixels_rgba8 length:bytes];
    if (copy == nil) {
      return false;
    }

    view.frameData = copy;
    view.frameWidth = frame->width;
    view.frameHeight = frame->height;
    view.frameStride = frame->stride_bytes;
    [view setNeedsDisplay:YES];
    [view displayIfNeeded];
    return true;
  }
}

void platform_shutdown(void) {
  @autoreleasepool {
    if (g_window != nil) {
      [g_window setDelegate:nil];
      [g_window orderOut:nil];
      [g_window close];
    }

    g_window = nil;
    g_view = nil;
    g_delegate = nil;
    memset(&g_event_queue, 0, sizeof(g_event_queue));
    g_initialized = false;
  }
}
