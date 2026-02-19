#include "platform.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdlib.h>
#include <string.h>

static HWND g_hwnd = NULL;
static HDC g_dc = NULL;

#define EVENT_CAPACITY 256
static platform_event g_events[EVENT_CAPACITY];
static unsigned int g_event_head = 0;
static unsigned int g_event_tail = 0;

static uint8_t *g_present_buffer = NULL;
static size_t g_present_buffer_size = 0;

static void push_event(const platform_event *event) {
  unsigned int next = (g_event_tail + 1u) % EVENT_CAPACITY;
  if (next == g_event_head) {
    return;
  }

  g_events[g_event_tail] = *event;
  g_event_tail = next;
}

static wchar_t *utf8_to_utf16(const char *utf8) {
  if (utf8 == NULL) {
    return NULL;
  }

  int chars_needed = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, NULL, 0);
  if (chars_needed <= 0) {
    return NULL;
  }

  wchar_t *wide = (wchar_t *)malloc((size_t)chars_needed * sizeof(wchar_t));
  if (wide == NULL) {
    return NULL;
  }

  if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, utf8, -1, wide, chars_needed) <= 0) {
    free(wide);
    return NULL;
  }

  return wide;
}

static LRESULT CALLBACK window_proc(HWND hwnd, UINT msg, WPARAM wparam, LPARAM lparam) {
  (void)hwnd;
  platform_event event;
  memset(&event, 0, sizeof(event));

  switch (msg) {
    case WM_CLOSE:
      event.kind = PLATFORM_EVENT_QUIT;
      push_event(&event);
      return 0;
    case WM_DESTROY:
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
    case WM_SIZE:
      event.kind = PLATFORM_EVENT_RESIZE;
      event.width = (uint32_t)LOWORD(lparam);
      event.height = (uint32_t)HIWORD(lparam);
      push_event(&event);
      return 0;
    default:
      return DefWindowProcW(hwnd, msg, wparam, lparam);
  }
}

bool platform_init_window(const platform_config *config) {
  if (config == NULL || config->abi_version != PLATFORM_ABI_VERSION || config->width == 0 ||
      config->height == 0) {
    return false;
  }

  HINSTANCE instance = GetModuleHandleW(NULL);
  const wchar_t *class_name = L"TesseraWindowClass";

  WNDCLASSW wc;
  memset(&wc, 0, sizeof(wc));
  wc.lpfnWndProc = window_proc;
  wc.hInstance = instance;
  wc.lpszClassName = class_name;
  wc.hCursor = LoadCursorW(NULL, IDC_ARROW);

  if (RegisterClassW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) {
    return false;
  }

  RECT rect = {0, 0, (LONG)config->width, (LONG)config->height};
  if (!AdjustWindowRect(&rect, WS_OVERLAPPEDWINDOW, FALSE)) {
    return false;
  }

  wchar_t *title_wide = utf8_to_utf16(config->title_utf8);
  const wchar_t *window_title = title_wide != NULL ? title_wide : L"Tessera";

  g_hwnd = CreateWindowExW(0, class_name, window_title, WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                           CW_USEDEFAULT, rect.right - rect.left, rect.bottom - rect.top, NULL,
                           NULL, instance, NULL);
  free(title_wide);

  if (g_hwnd == NULL) {
    return false;
  }

  ShowWindow(g_hwnd, SW_SHOWDEFAULT);
  UpdateWindow(g_hwnd);

  g_dc = GetDC(g_hwnd);
  if (g_dc == NULL) {
    DestroyWindow(g_hwnd);
    g_hwnd = NULL;
    return false;
  }

  g_event_head = 0;
  g_event_tail = 0;
  return true;
}

bool platform_poll_event(platform_event *out_event) {
  if (out_event == NULL) {
    return false;
  }

  MSG msg;
  while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) {
    if (msg.message == WM_QUIT) {
      platform_event event;
      memset(&event, 0, sizeof(event));
      event.kind = PLATFORM_EVENT_QUIT;
      push_event(&event);
      continue;
    }

    TranslateMessage(&msg);
    DispatchMessageW(&msg);
  }

  if (g_event_head == g_event_tail) {
    return false;
  }

  *out_event = g_events[g_event_head];
  g_event_head = (g_event_head + 1u) % EVENT_CAPACITY;
  return true;
}

bool platform_present_frame(const platform_frame *frame) {
  if (g_hwnd == NULL || g_dc == NULL || frame == NULL || frame->pixels_rgba8 == NULL) {
    return false;
  }

  if (frame->width == 0 || frame->height == 0 || frame->stride_bytes < frame->width * 4u) {
    return false;
  }

  const size_t tight_stride = (size_t)frame->width * 4u;
  const size_t packed_size = tight_stride * (size_t)frame->height;
  const uint8_t *pixels = frame->pixels_rgba8;

  if (frame->stride_bytes != tight_stride) {
    if (g_present_buffer_size < packed_size) {
      uint8_t *new_buffer = (uint8_t *)realloc(g_present_buffer, packed_size);
      if (new_buffer == NULL) {
        return false;
      }

      g_present_buffer = new_buffer;
      g_present_buffer_size = packed_size;
    }

    for (uint32_t y = 0; y < frame->height; y++) {
      const uint8_t *src_row = frame->pixels_rgba8 + ((size_t)y * frame->stride_bytes);
      uint8_t *dst_row = g_present_buffer + ((size_t)y * tight_stride);
      memcpy(dst_row, src_row, tight_stride);
    }
    pixels = g_present_buffer;
  }

  struct {
    BITMAPINFOHEADER header;
    DWORD masks[3];
  } info;
  memset(&info, 0, sizeof(info));

  info.header.biSize = sizeof(info.header);
  info.header.biWidth = (LONG)frame->width;
  info.header.biHeight = -((LONG)frame->height);
  info.header.biPlanes = 1;
  info.header.biBitCount = 32;
  info.header.biCompression = BI_BITFIELDS;
  info.masks[0] = 0x000000FFu;
  info.masks[1] = 0x0000FF00u;
  info.masks[2] = 0x00FF0000u;

  RECT client_rect;
  if (!GetClientRect(g_hwnd, &client_rect)) {
    return false;
  }

  int dst_width = client_rect.right - client_rect.left;
  int dst_height = client_rect.bottom - client_rect.top;
  if (dst_width <= 0 || dst_height <= 0) {
    return true;
  }

  int result = StretchDIBits(g_dc, 0, 0, dst_width, dst_height, 0, 0, (int)frame->width,
                             (int)frame->height, pixels, (BITMAPINFO *)&info, DIB_RGB_COLORS,
                             SRCCOPY);

  return result != GDI_ERROR;
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

  free(g_present_buffer);
  g_present_buffer = NULL;
  g_present_buffer_size = 0;
  g_event_head = 0;
  g_event_tail = 0;
}
