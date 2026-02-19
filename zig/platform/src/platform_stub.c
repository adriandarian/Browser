#include "platform.h"

uint32_t platform_get_abi_version(void) { return PLATFORM_ABI_VERSION; }

uint8_t platform_init_window(const platform_config *config) {
  (void)config;
  return PLATFORM_FALSE;
}

uint8_t platform_poll_event(platform_event *out_event) {
  (void)out_event;
  return PLATFORM_FALSE;
}

uint8_t platform_present_frame(const platform_frame *frame) {
  (void)frame;
  return PLATFORM_FALSE;
}

void platform_shutdown(void) {}
