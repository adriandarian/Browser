#ifndef TESSERA_PLATFORM_H
#define TESSERA_PLATFORM_H

#include <stdint.h>

// NOTE: This header is the ABI contract between Rust and Zig.
//
// ABI stability rules:
// - All structs must remain plain-old-data with fixed-width integer types.
// - Do not use C `bool` in function signatures or struct fields; use uint8_t.
// - Structs use native C ABI alignment (`repr(C)` in Rust mirrors this exactly).
// - Never change packing/alignment rules (no `#pragma pack`, no packed attributes).
// - To extend a struct safely, append trailing fields and include a size field.
// - Never reorder or remove existing fields.
// - Bump PLATFORM_ABI_VERSION on any breaking ABI change.
#define PLATFORM_ABI_VERSION ((uint32_t)2u)

#define PLATFORM_FALSE ((uint8_t)0u)
#define PLATFORM_TRUE ((uint8_t)1u)

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
  PLATFORM_KEY_ENTER = 13,
  PLATFORM_KEY_SPACE = 32,
  PLATFORM_KEY_F = 70,
  PLATFORM_KEY_H = 72,
  PLATFORM_KEY_J = 74,
  PLATFORM_KEY_K = 75,
  PLATFORM_KEY_S = 83,
};

typedef struct platform_config {
  // Size in bytes of this struct provided by the caller.
  // Allows forward/backward-compatible trailing field extensions.
  uint32_t struct_size;
  uint32_t abi_version;
  uint32_t width;
  uint32_t height;
  const char *title_utf8;
} platform_config;

typedef struct platform_frame {
  // Size in bytes of this struct provided by the caller.
  uint32_t struct_size;
  uint32_t width;
  uint32_t height;
  uint32_t stride_bytes;
  const uint8_t *pixels_rgba8;
} platform_frame;

typedef struct platform_event {
  // Size in bytes of this struct provided by the caller.
  uint32_t struct_size;
  uint32_t kind;
  uint32_t key_code;
  uint32_t width;
  uint32_t height;
} platform_event;

// ABI sanity checks. Pointer-sized structs are checked in Rust tests for both
// 32-bit and 64-bit expectations.
#if defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
_Static_assert(sizeof(platform_event) == 20u, "platform_event ABI size changed");
#endif

#ifdef __cplusplus
extern "C" {
#endif

uint32_t platform_get_abi_version(void);
uint8_t platform_init_window(const platform_config *config);
uint8_t platform_poll_event(platform_event *out_event);
uint8_t platform_present_frame(const platform_frame *frame);
void platform_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif
