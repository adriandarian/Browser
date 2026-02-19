#include "platform.h"

bool platform_init_window(const platform_config *config) {
  (void)config;
  return false;
}

bool platform_poll_event(platform_event *out_event) {
  (void)out_event;
  return false;
}

bool platform_present_frame(const platform_frame *frame) {
  (void)frame;
  return false;
}

void platform_shutdown(void) {}
