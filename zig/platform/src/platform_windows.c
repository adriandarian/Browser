#include "platform.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

static HWND g_hwnd = NULL;
static HDC g_dc = NULL;
static HINSTANCE g_instance = NULL;
static bool g_quit_enqueued = false;
static uint32_t g_last_width = 0;
static uint32_t g_last_height = 0;
static uint8_t *g_present_bgra = NULL;
static size_t g_present_bgra_capacity = 0;

#define EVENT_CAPACITY 256
static platform_event g_events[EVENT_CAPACITY];
static unsigned int g_event_head = 0;
static unsigned int g_event_tail = 0;

static void push_event(const platform_event *event) {
  unsigned int next = (g_event_tail + 1u) % EVENT_CAPACITY;
  if (next == g_event_head) {
    return;
  }
  g_events[g_event_tail] = *event;
  g_event_tail = next;
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

uint32_t platform_get_abi_version(void) { return PLATFORM_ABI_VERSION; }

static bool ensure_present_buffer_capacity(size_t needed) {
  if (needed <= g_present_bgra_capacity) {
    return true;
  }

  uint8_t *next = (uint8_t *)realloc(g_present_bgra, needed);
  if (next == NULL) {
    return false;
  }

  g_present_bgra = next;
  g_present_bgra_capacity = needed;
  return true;
}

static bool utf8_to_utf16_alloc(const char *utf8, wchar_t **out_wide) {
  if (utf8 == NULL || out_wide == NULL) {
    return false;
  }

  int wide_len = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, NULL, 0);
  if (wide_len <= 0) {
    return false;
  }

  wchar_t *wide = (wchar_t *)malloc((size_t)wide_len * sizeof(wchar_t));
  if (wide == NULL) {
    return false;
  }

  int converted = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, wide, wide_len);
  if (converted <= 0) {
    free(wide);
    return false;
  }

  *out_wide = wide;
  return true;
}

static LRESULT CALLBACK window_proc(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam) {
  (void)hwnd;
  platform_event event;
  memset(&event, 0, sizeof(event));
  event.struct_size = sizeof(platform_event);

  switch (msg) {
    case WM_CLOSE:
      enqueue_quit_if_needed();
      DestroyWindow(hwnd);
      return 0;
    case WM_DESTROY:
      enqueue_quit_if_needed();
      PostQuitMessage(0);
      return 0;
    case WM_KEYDOWN:
      event.kind = PLATFORM_EVENT_KEY_DOWN;
      event.key_code = (wparam == VK_ESCAPE) ? PLATFORM_KEY_ESCAPE : PLATFORM_KEY_UNKNOWN;
      push_event(&event);
      return 0;
    case WM_KEYUP:
      event.kind = PLATFORM_EVENT_KEY_UP;
      event.key_code = (wparam == VK_ESCAPE) ? PLATFORM_KEY_ESCAPE : PLATFORM_KEY_UNKNOWN;
      push_event(&event);
      return 0;
    case WM_SIZE: {
      uint32_t width = (uint32_t)LOWORD(lparam);
      uint32_t height = (uint32_t)HIWORD(lparam);
      if (width == 0 || height == 0) {
        return 0;
      }

      if (width != g_last_width || height != g_last_height) {
        g_last_width = width;
        g_last_height = height;
        event.kind = PLATFORM_EVENT_RESIZE;
        event.width = width;
        event.height = height;
        push_event(&event);
      }
      return 0;
    }
    default:
      return DefWindowProcW(hwnd, msg, wparam, lparam);
  }
}

uint8_t platform_init_window(const platform_config *config) {
  if (config == NULL || config->struct_size < sizeof(platform_config) ||
      config->abi_version != PLATFORM_ABI_VERSION) {
    return PLATFORM_FALSE;
  }

  HINSTANCE instance = GetModuleHandleW(NULL);
  if (instance == NULL) {
    return PLATFORM_FALSE;
  }
  g_instance = instance;
  const wchar_t *class_name = L"BrowserWindowClass";

  WNDCLASSW wc;
  memset(&wc, 0, sizeof(wc));
  wc.lpfnWndProc = window_proc;
  wc.hInstance = instance;
  wc.lpszClassName = class_name;
  wc.hCursor = LoadCursorW(NULL, IDC_ARROW);

  if (RegisterClassW(&wc) == 0) {
    DWORD err = GetLastError();
    if (err != ERROR_CLASS_ALREADY_EXISTS) {
      return PLATFORM_FALSE;
    }
  }

  RECT rect = {0, 0, (LONG)config->width, (LONG)config->height};
  AdjustWindowRect(&rect, WS_OVERLAPPEDWINDOW, FALSE);

  wchar_t *window_title = NULL;
  const wchar_t *window_title_w = L"Browser";
  if (config->title_utf8 != NULL && config->title_utf8[0] != '\0') {
    if (!utf8_to_utf16_alloc(config->title_utf8, &window_title)) {
      return PLATFORM_FALSE;
    }
    window_title_w = window_title;
  }

  g_hwnd = CreateWindowExW(0, class_name, window_title_w, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                           CW_USEDEFAULT, rect.right - rect.left, rect.bottom - rect.top, NULL,
                           NULL, instance, NULL);
  free(window_title);
  if (g_hwnd == NULL) {
    return PLATFORM_FALSE;
  }

  g_event_head = 0;
  g_event_tail = 0;
  g_quit_enqueued = false;
  g_last_width = config->width;
  g_last_height = config->height;

  ShowWindow(g_hwnd, SW_SHOW);
  UpdateWindow(g_hwnd);

  g_dc = GetDC(g_hwnd);
  return (g_dc != NULL) ? PLATFORM_TRUE : PLATFORM_FALSE;
}

uint8_t platform_poll_event(platform_event *out_event) {
  if (out_event == NULL || out_event->struct_size < sizeof(platform_event)) {
    return PLATFORM_FALSE;
  }

  MSG msg;
  while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) {
    TranslateMessage(&msg);
    DispatchMessageW(&msg);
  }

  if (g_event_head == g_event_tail) {
    return PLATFORM_FALSE;
  }

  *out_event = g_events[g_event_head];
  g_event_head = (g_event_head + 1u) % EVENT_CAPACITY;
  return PLATFORM_TRUE;
}

uint8_t platform_present_frame(const platform_frame *frame) {
  if (g_hwnd == NULL || g_dc == NULL || frame == NULL ||
      frame->struct_size < sizeof(platform_frame) || frame->pixels_rgba8 == NULL) {
    return PLATFORM_FALSE;
  }
  if (frame->width == 0 || frame->height == 0) {
    return PLATFORM_FALSE;
  }
  if (frame->stride_bytes < frame->width * 4u) {
    return PLATFORM_FALSE;
  }

  size_t row_bytes = (size_t)frame->width * 4u;
  size_t pixel_bytes = row_bytes * (size_t)frame->height;
  if (!ensure_present_buffer_capacity(pixel_bytes)) {
    return PLATFORM_FALSE;
  }

  for (uint32_t y = 0; y < frame->height; ++y) {
    const uint8_t *src = frame->pixels_rgba8 + ((size_t)y * (size_t)frame->stride_bytes);
    uint8_t *dst = g_present_bgra + ((size_t)y * row_bytes);
    for (uint32_t x = 0; x < frame->width; ++x) {
      uint32_t i = x * 4u;
      dst[i + 0u] = src[i + 2u];
      dst[i + 1u] = src[i + 1u];
      dst[i + 2u] = src[i + 0u];
      dst[i + 3u] = src[i + 3u];
    }
  }

  BITMAPINFO info;
  memset(&info, 0, sizeof(info));
  info.bmiHeader.biSize = sizeof(info.bmiHeader);
  info.bmiHeader.biWidth = (LONG)frame->width;
  info.bmiHeader.biHeight = -((LONG)frame->height);
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;

  RECT client_rect;
  if (!GetClientRect(g_hwnd, &client_rect)) {
    return PLATFORM_FALSE;
  }
  int dst_width = client_rect.right - client_rect.left;
  int dst_height = client_rect.bottom - client_rect.top;
  if (dst_width <= 0 || dst_height <= 0) {
    return PLATFORM_FALSE;
  }

  int result = StretchDIBits(g_dc, 0, 0, dst_width, dst_height, 0, 0, (int)frame->width,
                             (int)frame->height, g_present_bgra, &info, DIB_RGB_COLORS,
                             SRCCOPY);

  return (result != GDI_ERROR) ? PLATFORM_TRUE : PLATFORM_FALSE;
}

void platform_shutdown(void) {
  if (g_dc != NULL && g_hwnd != NULL) {
    ReleaseDC(g_hwnd, g_dc);
    g_dc = NULL;
  }
  if (g_hwnd != NULL) {
    DestroyWindow(g_hwnd);
    g_hwnd = NULL;
  }
  if (g_instance != NULL) {
    UnregisterClassW(L"BrowserWindowClass", g_instance);
    g_instance = NULL;
  }
  free(g_present_bgra);
  g_present_bgra = NULL;
  g_present_bgra_capacity = 0;

  g_event_head = 0;
  g_event_tail = 0;
  g_quit_enqueued = false;
  g_last_width = 0;
  g_last_height = 0;
}
