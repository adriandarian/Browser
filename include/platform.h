#ifndef TESSERA_PLATFORM_H
#define TESSERA_PLATFORM_H

#include <stdbool.h>
#include <stdint.h>

// NOTE: This header is the ABI contract between Rust and Zig.
// - Keep all structs POD and explicitly-sized.
// - Append new fields only at the end, and version-gate behavior.
// - Never reorder or remove existing fields.
// - Bump PLATFORM_ABI_VERSION on any breaking ABI change.
#define PLATFORM_ABI_VERSION 1u

enum platform_event_kind {
  PLATFORM_EVENT_NONE = 0,
  PLATFORM_EVENT_QUIT = 1,
  PLATFORM_EVENT_KEY_DOWN = 2,
  PLATFORM_EVENT_KEY_UP = 3,
  PLATFORM_EVENT_RESIZE = 4,
};

enum platform_key_code {
  PLATFORM_KEY_UNKNOWN = 0,
  PLATFORM_KEY_ESCAPE = 27,
};

typedef struct platform_config {
  uint32_t abi_version;
  uint32_t width;
  uint32_t height;
  const char *title_utf8;
} platform_config;

typedef struct platform_frame {
  uint32_t width;
  uint32_t height;
  uint32_t stride_bytes;
  const uint8_t *pixels_rgba8;
} platform_frame;

typedef struct platform_event {
  uint32_t kind;
  uint32_t key_code;
  uint32_t width;
  uint32_t height;
} platform_event;

#ifdef __cplusplus
extern "C" {
#endif

bool platform_init_window(const platform_config *config);
bool platform_poll_event(platform_event *out_event);
bool platform_present_frame(const platform_frame *frame);
void platform_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif
