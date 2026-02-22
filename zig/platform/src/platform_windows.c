#include "platform.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdbool.h>
#include <string.h>

static HWND g_hwnd = NULL;
static HDC g_dc = NULL;
static bool g_quit_enqueued = false;
static uint32_t g_last_width = 0;
static uint32_t g_last_height = 0;

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

  g_hwnd = CreateWindowExW(0, class_name, L"Browser", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                           CW_USEDEFAULT, rect.right - rect.left, rect.bottom - rect.top, NULL,
                           NULL, instance, NULL);
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

  BITMAPINFO info;
  memset(&info, 0, sizeof(info));
  info.bmiHeader.biSize = sizeof(info.bmiHeader);
  info.bmiHeader.biWidth = (LONG)frame->width;
  info.bmiHeader.biHeight = -((LONG)frame->height);
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;

  int result = StretchDIBits(g_dc, 0, 0, (int)frame->width, (int)frame->height, 0, 0,
                             (int)frame->width, (int)frame->height, frame->pixels_rgba8, &info,
                             DIB_RGB_COLORS, SRCCOPY);

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

  g_event_head = 0;
  g_event_tail = 0;
  g_quit_enqueued = false;
  g_last_width = 0;
  g_last_height = 0;
}
