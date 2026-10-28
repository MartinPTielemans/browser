// The app's Windows glue: platform/host.h, as platform/mac/host.mm is on the Mac. The core puts the
// UI's holder window in the browser window as a child under the pages
// (core/platform/win/window_views.cc). What floats over a page, such as the command bar, is a GPUI
// window of its own that the browser window owns, which this places over it.

#include <windows.h>

#include <commctrl.h>
#include <commdlg.h>
#include <dwmapi.h>
#include <shellapi.h>

#include <ctime>
#include <functional>
#include <map>
#include <string>
#include <vector>

#include "../host.h"

namespace {

std::wstring Wide(const char* utf8) {
  if (!utf8) {
    return {};
  }
  const int length = MultiByteToWideChar(CP_UTF8, 0, utf8, -1, nullptr, 0);
  std::wstring wide(length > 0 ? length - 1 : 0, L'\0');
  if (length > 1) {
    MultiByteToWideChar(CP_UTF8, 0, utf8, -1, wide.data(), length);
  }
  return wide;
}

HWND Hwnd(void* view) {
  return static_cast<HWND>(view);
}

// The browser window a UI view is in, once the core has put it there.
HWND Root(void* ui_view) {
  return ui_view ? GetAncestor(Hwnd(ui_view), GA_ROOT) : nullptr;
}

double Scale(HWND hwnd) {
  const UINT dpi = hwnd ? GetDpiForWindow(hwnd) : GetDpiForSystem();
  return (dpi ? dpi : 96) / 96.0;
}

// The browser window's content, in points.
void ContentSize(HWND root, double* width, double* height) {
  RECT client{};
  GetClientRect(root, &client);
  const double scale = Scale(root);
  *width = client.right / scale;
  *height = client.bottom / scale;
}

// A window's size in points.
void WindowSize(HWND hwnd, double* width, double* height) {
  RECT rect{};
  GetWindowRect(hwnd, &rect);
  const double scale = Scale(hwnd);
  *width = (rect.right - rect.left) / scale;
  *height = (rect.bottom - rect.top) / scale;
}

// Runs |task| on the main thread after the event being handled, as AppKit's "after the event" does.
UINT const kRunTask = WM_APP + 1;

LRESULT CALLBACK TaskProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message == kRunTask) {
    auto* task = reinterpret_cast<std::function<void()>*>(lparam);
    (*task)();
    delete task;
    return 0;
  }
  return DefWindowProcW(hwnd, message, wparam, lparam);
}

void Later(std::function<void()> task) {
  static HWND tasks = [] {
    WNDCLASSW window_class{};
    window_class.lpfnWndProc = TaskProc;
    window_class.hInstance = GetModuleHandleW(nullptr);
    window_class.lpszClassName = L"BrowserHostTasks";
    RegisterClassW(&window_class);
    return CreateWindowExW(0, window_class.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE,
                           nullptr, window_class.hInstance, nullptr);
  }();
  PostMessageW(tasks, kRunTask, 0, reinterpret_cast<LPARAM>(new std::function<void()>(task)));
}

// A GPUI window floating over a browser window: owned by it, so it stays in front of it and goes
// with it when it's minimized, and placed in points from its content's top-left.
struct Floating {
  HWND hwnd = nullptr;
  // Clicks go through it to the browser window under it.
  bool mouse_through = false;
  // Called after the event, when the user leaves it for a window that isn't its own.
  void (*dismissed)(void) = nullptr;
  // Set while the host hides it, so that isn't the user leaving it.
  bool hiding = false;
};

std::map<HWND, Floating*>& FloatingByHwnd() {
  static std::map<HWND, Floating*> all;
  return all;
}

LRESULT CALLBACK FloatingProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam,
                              UINT_PTR, DWORD_PTR data) {
  auto* floating = reinterpret_cast<Floating*>(data);
  switch (message) {
    case WM_NCHITTEST:
      // To the browser window under it, which is on this thread.
      if (floating->mouse_through) {
        return HTTRANSPARENT;
      }
      break;
    case WM_MOUSEACTIVATE:
      if (floating->mouse_through) {
        return MA_NOACTIVATE;
      }
      break;
    case WM_ACTIVATE:
      if (LOWORD(wparam) == WA_INACTIVE && floating->dismissed && !floating->hiding &&
          IsWindowVisible(hwnd)) {
        void (*dismissed)(void) = floating->dismissed;
        Later([dismissed] { dismissed(); });
      }
      break;
  }
  return DefSubclassProc(hwnd, message, wparam, lparam);
}

void Adopt(Floating& floating, void* view, bool mouse_through, void (*dismissed)(void)) {
  floating.hwnd = Hwnd(view);
  floating.mouse_through = mouse_through;
  floating.dismissed = dismissed;
  ShowWindow(floating.hwnd, SW_HIDE);
  if (mouse_through) {
    SetWindowLongPtrW(floating.hwnd, GWL_EXSTYLE,
                      GetWindowLongPtrW(floating.hwnd, GWL_EXSTYLE) | WS_EX_NOACTIVATE);
  }
  SetWindowSubclass(floating.hwnd, FloatingProc, 1, reinterpret_cast<DWORD_PTR>(&floating));
  FloatingByHwnd()[floating.hwnd] = &floating;
  // Windows 11 rounds it, as the Mac's shape and radius do.
  DWM_WINDOW_CORNER_PREFERENCE corners = DWMWCP_ROUND;
  DwmSetWindowAttribute(floating.hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corners, sizeof corners);
}

// Shows |floating| over |ui_view|'s browser window at |left|, |top| points in its content, |width|
// by |height| points, or its own size where one is negative, with the keyboard if |keyboard|.
void Show(Floating& floating, void* ui_view, double left, double top, double width, double height,
          bool keyboard) {
  HWND root = Root(ui_view);
  if (!floating.hwnd || !root) {
    return;
  }
  double own_width, own_height;
  WindowSize(floating.hwnd, &own_width, &own_height);
  width = width < 0 ? own_width : width;
  height = height < 0 ? own_height : height;
  const double scale = Scale(root);
  POINT origin{static_cast<LONG>(left * scale), static_cast<LONG>(top * scale)};
  ClientToScreen(root, &origin);
  SetWindowLongPtrW(floating.hwnd, GWLP_HWNDPARENT, reinterpret_cast<LONG_PTR>(root));
  SetWindowPos(floating.hwnd, HWND_TOP, origin.x, origin.y, static_cast<int>(width * scale),
               static_cast<int>(height * scale),
               SWP_SHOWWINDOW | (keyboard ? 0 : SWP_NOACTIVATE));
  if (keyboard) {
    SetForegroundWindow(floating.hwnd);
    SetFocus(floating.hwnd);
  }
}

// Hides |floating|, giving its browser window the keyboard back if it had it.
void Hide(Floating& floating) {
  if (!floating.hwnd || !IsWindowVisible(floating.hwnd)) {
    return;
  }
  const bool had_keyboard = GetForegroundWindow() == floating.hwnd;
  HWND owner = GetWindow(floating.hwnd, GW_OWNER);
  floating.hiding = true;
  ShowWindow(floating.hwnd, SW_HIDE);
  floating.hiding = false;
  if (had_keyboard && owner) {
    SetForegroundWindow(owner);
  }
}

void Resize(Floating& floating, double height) {
  if (!floating.hwnd) {
    return;
  }
  RECT rect{};
  GetWindowRect(floating.hwnd, &rect);
  SetWindowPos(floating.hwnd, nullptr, 0, 0, rect.right - rect.left,
               static_cast<int>(height * Scale(floating.hwnd)),
               SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
}

// The UI views host_ui_view made, in order, for the watchers to find a browser window's.
std::vector<HWND>& UiViews() {
  static std::vector<HWND> views;
  return views;
}

void* UiViewOf(HWND root) {
  for (HWND view : UiViews()) {
    if (IsWindow(view) && GetAncestor(view, GA_ROOT) == root) {
      return view;
    }
  }
  return nullptr;
}

// The holder keeps GPUI's window, its only child, filling it.
LRESULT CALLBACK HolderProc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam) {
  switch (message) {
    case WM_SIZE:
      if (HWND ui = GetWindow(hwnd, GW_CHILD)) {
        SetWindowPos(ui, nullptr, 0, 0, LOWORD(lparam), HIWORD(lparam),
                     SWP_NOZORDER | SWP_NOACTIVATE);
      }
      return 0;
    case WM_PARENTNOTIFY:
      // A click in the UI takes the keyboard from the page, as one in a top-level window would.
      if (LOWORD(wparam) == WM_LBUTTONDOWN || LOWORD(wparam) == WM_RBUTTONDOWN ||
          LOWORD(wparam) == WM_MBUTTONDOWN) {
        if (HWND ui = GetWindow(hwnd, GW_CHILD); ui && GetFocus() != ui) {
          SetFocus(ui);
        }
      }
      return 0;
    case WM_ERASEBKGND:
      return 1;
  }
  return DefWindowProcW(hwnd, message, wparam, lparam);
}

// The watchers, which Windows tells of each window that comes to the front.
void (*g_key_window_changed)(void*) = nullptr;
void (*g_resigning_active)(void) = nullptr;
void (*g_became_active)(void) = nullptr;
bool g_active = true;

void CALLBACK ForegroundChanged(HWINEVENTHOOK, DWORD, HWND hwnd, LONG, LONG, DWORD, DWORD) {
  DWORD process = 0;
  GetWindowThreadProcessId(hwnd, &process);
  const bool ours = process == GetCurrentProcessId();
  if (ours && !g_active && g_became_active) {
    g_became_active();
  } else if (!ours && g_active && g_resigning_active) {
    // shortcut: Windows says so once another app is in front, where AppKit says so just before;
    // what's shown over the window hides a moment late.
    g_resigning_active();
  }
  g_active = ours;
  if (ours && g_key_window_changed) {
    if (void* ui_view = UiViewOf(GetAncestor(hwnd, GA_ROOT))) {
      g_key_window_changed(ui_view);
    }
  }
}

void WatchForeground() {
  static HWINEVENTHOOK hook = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND,
                                              nullptr, ForegroundChanged, 0, 0,
                                              WINEVENT_OUTOFCONTEXT);
  (void)hook;
}

Floating g_overlays[3];
struct Corner {
  Floating floating;
  double right = 0;
  double top = 0;
};
std::map<void*, Corner> g_find_bars;
std::map<void*, Corner> g_offers;
std::map<void*, std::pair<double, double>> g_pane_corners;
Floating g_hud;
double g_hud_right = 0, g_hud_top = 0;
Floating g_drop_targets;
Floating g_dialog;
Floating g_site_controls;
Floating g_capture;
Floating g_toast;
Floating g_tooltip;
double g_tooltip_margin = 0;
Floating g_link_target;
double g_link_margin = 0;
void (*g_link_pointer)(double, double) = nullptr;
void* g_link_shown_over = nullptr;
struct Reveal {
  Floating floating;
  void (*pointer_moved)(void*, double) = nullptr;
};
std::map<void*, Reveal> g_reveals;
Floating g_settings;
bool g_settings_placed = false;

// While the link target or a reveal shows, the pointer's moves over the app's windows.
HHOOK g_mouse_hook = nullptr;

LRESULT CALLBACK MouseMoved(int code, WPARAM wparam, LPARAM lparam) {
  if (code == HC_ACTION && wparam == WM_MOUSEMOVE) {
    POINT point = reinterpret_cast<MOUSEHOOKSTRUCT*>(lparam)->pt;
    if (g_link_pointer && g_link_shown_over && IsWindowVisible(g_link_target.hwnd)) {
      HWND root = Root(g_link_shown_over);
      POINT in = point;
      ScreenToClient(root, &in);
      const double scale = Scale(root);
      g_link_pointer(in.x / scale, in.y / scale);
    }
    for (auto& [ui_view, reveal] : g_reveals) {
      if (reveal.pointer_moved && IsWindowVisible(reveal.floating.hwnd)) {
        HWND root = Root(ui_view);
        POINT in = point;
        ScreenToClient(root, &in);
        reveal.pointer_moved(ui_view, in.x / Scale(root));
      }
    }
  }
  return CallNextHookEx(nullptr, code, wparam, lparam);
}

void WatchPointer() {
  if (!g_mouse_hook) {
    g_mouse_hook = SetWindowsHookExW(WH_MOUSE, MouseMoved, nullptr, GetCurrentThreadId());
  }
}

// The pointer stops being watched once nothing needs it, so an idle browser isn't woken by it.
void MaybeStopWatchingPointer() {
  bool needed = g_link_target.hwnd && IsWindowVisible(g_link_target.hwnd);
  for (auto& [ui_view, reveal] : g_reveals) {
    needed = needed || IsWindowVisible(reveal.floating.hwnd);
  }
  if (!needed && g_mouse_hook) {
    UnhookWindowsHookEx(g_mouse_hook);
    g_mouse_hook = nullptr;
  }
}

void ShowCorner(Corner& corner, void* ui_view, bool keyboard) {
  HWND root = Root(ui_view);
  double content_width, content_height, width, height;
  ContentSize(root, &content_width, &content_height);
  WindowSize(corner.floating.hwnd, &width, &height);
  auto [extra_right, extra_top] = g_pane_corners[ui_view];
  Show(corner.floating, ui_view, content_width - corner.right - extra_right - width,
       corner.top + extra_top, -1, -1, keyboard);
}

// Control's release, for host_watch_control.
void (*g_control_released)(void) = nullptr;
HHOOK g_key_hook = nullptr;

LRESULT CALLBACK KeyPressed(int code, WPARAM wparam, LPARAM lparam) {
  if (code == HC_ACTION && g_control_released && (GetKeyState(VK_CONTROL) & 0x8000) == 0) {
    void (*released)(void) = g_control_released;
    g_control_released = nullptr;
    Later([released] { released(); });
    UnhookWindowsHookEx(g_key_hook);
    g_key_hook = nullptr;
  }
  return CallNextHookEx(nullptr, code, wparam, lparam);
}

}  // namespace

extern "C" {

void drive_start(void) {
  // shortcut: the test driver (platform/mac/drive.mm) isn't ported, so check.sh doesn't run on
  // Windows; port it when the checks do.
  fputs("browser: no test driver on Windows yet\n", stderr);
}

// Centered in the primary display's work area, in GPUI's logical pixels from its top-left.
void host_centered(double width, double height, double* x, double* y) {
  RECT work;
  SystemParametersInfoW(SPI_GETWORKAREA, 0, &work, 0);
  const double scale = GetDpiForSystem() / 96.0;
  *x = (work.left + work.right) / 2.0 / scale - width / 2;
  *y = (work.top + work.bottom) / 2.0 / scale - height / 2;
}

// shortcut: no launch window on Windows yet: the browser window shows once the UI has drawn. Port
// the Mac's picture of the last sidebar when launch is measured on Windows.
void host_show_launch_window(const char*, double, double, double, double) {}
void host_close_launch_window(void) {}
bool host_save_launch_picture(void*, const char*, double) {
  return false;
}

// shortcut: links a launch carries come in on the command line, so there's nothing to wait for.
void host_watch_finished_launching(void (*finished)(void)) {
  finished();
}

// shortcut: no screens or window frame yet, so windows open where host_centered puts them and
// aren't brought back where they were.
size_t host_screens(double*, size_t) {
  return 0;
}

bool host_window_frame(void*, double*, double*, double*, double*) {
  return false;
}

// Linux's alone: GPUI's loop runs inside Chromium's there. On Windows it runs on window messages.
void host_watch_fd(int, void (*)(void)) {}

void* host_ui_view(void) {
  static const wchar_t* holder_class = [] {
    WNDCLASSW window_class{};
    window_class.lpfnWndProc = HolderProc;
    window_class.hInstance = GetModuleHandleW(nullptr);
    window_class.hCursor = LoadCursorW(nullptr, IDC_ARROW);
    window_class.lpszClassName = L"BrowserUI";
    RegisterClassW(&window_class);
    return window_class.lpszClassName;
  }();
  WatchForeground();
  // A window of its own until the core makes it a child of the browser window.
  HWND holder = CreateWindowExW(0, holder_class, L"", WS_POPUP | WS_CLIPCHILDREN, 0, 0, 0, 0,
                                nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
  UiViews().push_back(holder);
  return holder;
}

void host_show_window(void* ui_view) {
  HWND root = Root(ui_view);
  ShowWindow(root, SW_SHOW);
  SetForegroundWindow(root);
}

// shortcut: shows it in front, as host_show_window, not behind the window it follows.
void host_show_window_behind(void* ui_view, void*) {
  host_show_window(ui_view);
}

void host_cascaded(void* ui_view, double* x, double* y, double* width, double* height) {
  HWND root = Root(ui_view);
  RECT rect{};
  GetWindowRect(root, &rect);
  const double scale = Scale(root);
  const double step = GetSystemMetrics(SM_CYCAPTION) / scale;
  *x = rect.left / scale + step;
  *y = rect.top / scale + step;
  *width = (rect.right - rect.left) / scale;
  *height = (rect.bottom - rect.top) / scale;
}

void host_set_window_title(void* ui_view, const char* title) {
  SetWindowTextW(Root(ui_view), Wide(title).c_str());
}

void host_activate(void* ui_view) {
  HWND root = Root(ui_view);
  if (IsIconic(root)) {
    ShowWindow(root, SW_RESTORE);
  }
  SetForegroundWindow(root);
}

void host_watch_key_window(void (*changed)(void* ui_view)) {
  g_key_window_changed = changed;
  WatchForeground();
}

// shortcut: Windows' full screen isn't hooked yet; the core's page full screen still works.
void host_watch_full_screen(void (*)(void*, bool)) {}

// shortcut: Windows sends a touchpad's swipes as scrolls, which the page takes.
void host_watch_swipes(bool (*)(void*, double, double, bool)) {}

// A window's menu bar isn't replaced while it's open on Windows: it's modal and runs no tasks.
bool host_after_menus(void (*)(void)) { return false; }

void host_watch_resigning_active(void (*resigning)(void)) {
  g_resigning_active = resigning;
  WatchForeground();
}

void host_watch_active(void (*became_active)(void)) {
  g_became_active = became_active;
  WatchForeground();
}

// shortcut: reads as not the default; check the http UserChoice ProgId once the installer
// registers the app.
bool host_is_default_browser(void) {
  return false;
}

void host_make_default_browser(void (*answered)(void)) {
  // Windows lets only the user choose, in Settings.
  ShellExecuteW(nullptr, L"open", L"ms-settings:defaultapps", nullptr, nullptr, SW_SHOWNORMAL);
  Later([answered] { answered(); });
}

double host_since_launch(void) {
  FILETIME created, exited, kernel, user, now;
  GetProcessTimes(GetCurrentProcess(), &created, &exited, &kernel, &user);
  GetSystemTimePreciseAsFileTime(&now);
  auto ticks = [](FILETIME time) {
    return (static_cast<ULONGLONG>(time.dwHighDateTime) << 32) | time.dwLowDateTime;
  };
  return (ticks(now) - ticks(created)) / 1e4;
}

void host_forget_window(void* ui_view) {
  HWND root = Root(ui_view);
  for (auto& [hwnd, floating] : FloatingByHwnd()) {
    if (GetWindow(hwnd, GW_OWNER) == root) {
      Hide(*floating);
      SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
    }
  }
  g_find_bars.erase(ui_view);
  g_offers.erase(ui_view);
  g_reveals.erase(ui_view);
  g_pane_corners.erase(ui_view);
  std::erase(UiViews(), Hwnd(ui_view));
}

void host_add_overlay(int overlay, void* overlay_view, double, void (*dismissed)(void)) {
  Adopt(g_overlays[overlay], overlay_view, false, dismissed);
}

// shortcut: an overlay's shape, radius, elevation and growth are Windows 11's rounded corners and
// shadow, with no blur behind it and no motion as it shows or hides.
void host_set_overlay_shape(int, const double*, size_t) {}
void host_set_overlay_radius(int, double) {}
void host_set_overlay_elevation(int, float, double, float, double, double) {}
void host_set_overlay_growth(int, double, double) {}
void host_set_overlay_shrink(int) {}

void host_show_overlay(int overlay,
                       void* ui_view,
                       bool centered,
                       double left,
                       double top,
                       double width) {
  for (int each = 0; each < 3; ++each) {
    Floating& other = g_overlays[each];
    if (each != overlay && other.hwnd && IsWindowVisible(other.hwnd)) {
      Hide(other);
      if (other.dismissed) {
        void (*dismissed)(void) = other.dismissed;
        Later([dismissed] { dismissed(); });
      }
    }
  }
  double content_width, content_height;
  ContentSize(Root(ui_view), &content_width, &content_height);
  if (centered) {
    left = (content_width - width) / 2;
    top = content_height / 2 - top;
  } else if (left + width > content_width - 8) {
    left = content_width - 8 - width;
  }
  Show(g_overlays[overlay], ui_view, left, top, width, -1, true);
}

void host_set_overlay_height(int overlay, double height) {
  Resize(g_overlays[overlay], height);
}

// shortcut: Segoe UI Emoji is taken to draw every emoji; check with DirectWrite's glyphs when an
// older Windows shows blanks in the icon picker.
bool host_can_draw_emoji(const char*) {
  return true;
}

void host_hide_overlay(int overlay) {
  Hide(g_overlays[overlay]);
}

void host_dismiss_overlay(int overlay) {
  Floating& floating = g_overlays[overlay];
  Hide(floating);
  if (floating.dismissed) {
    floating.dismissed();
  }
}

// shortcut: keys typed between a shortcut and its bar taking the keyboard aren't held yet.
void host_hold_keys(void) {}

void host_add_find_bar(void* ui_view, void* find_bar_view, double right, double top) {
  Corner& corner = g_find_bars[ui_view];
  Adopt(corner.floating, find_bar_view, false, nullptr);
  corner.right = right;
  corner.top = top;
}

void host_show_find_bar(void* ui_view, bool keyboard) {
  if (auto it = g_find_bars.find(ui_view); it != g_find_bars.end()) {
    ShowCorner(it->second, ui_view, keyboard);
  }
}

void host_hide_find_bar(void* ui_view) {
  if (auto it = g_find_bars.find(ui_view); it != g_find_bars.end()) {
    Hide(it->second.floating);
  }
}

void host_add_offer(void* ui_view, void* offer_view, double right, double top) {
  Corner& corner = g_offers[ui_view];
  Adopt(corner.floating, offer_view, false, nullptr);
  corner.right = right;
  corner.top = top;
}

void host_show_offer(void* ui_view) {
  if (auto it = g_offers.find(ui_view); it != g_offers.end()) {
    ShowCorner(it->second, ui_view, false);
  }
}

void host_hide_offer(void* ui_view) {
  if (auto it = g_offers.find(ui_view); it != g_offers.end()) {
    Hide(it->second.floating);
  }
}

void host_set_pane_corner(void* ui_view, double right, double top) {
  g_pane_corners[ui_view] = {right, top};
}

void host_add_hud(void* hud_view, double right, double top, double) {
  Adopt(g_hud, hud_view, true, nullptr);
  g_hud_right = right;
  g_hud_top = top;
}

void host_show_hud(void* ui_view) {
  double content_width, content_height, width, height;
  ContentSize(Root(ui_view), &content_width, &content_height);
  WindowSize(g_hud.hwnd, &width, &height);
  auto [extra_right, extra_top] = g_pane_corners[ui_view];
  Show(g_hud, ui_view, content_width - g_hud_right - extra_right - width, g_hud_top + extra_top, -1,
       -1, false);
}

void host_hide_hud(void) {
  Hide(g_hud);
}

void host_add_drop_targets(void* view) {
  Adopt(g_drop_targets, view, true, nullptr);
}

void host_show_drop_targets(void* ui_view) {
  double width, height;
  ContentSize(Root(ui_view), &width, &height);
  Show(g_drop_targets, ui_view, 0, 0, width, height, false);
}

void host_hide_drop_targets(void) {
  Hide(g_drop_targets);
}

void host_add_dialog(void* dialog_view) {
  Adopt(g_dialog, dialog_view, false, nullptr);
}

void host_show_dialog(void* ui_view, double x, double y, double width, double height) {
  // Without a rect, it covers all of the window.
  if (width <= 0) {
    x = y = 0;
    ContentSize(Root(ui_view), &width, &height);
  }
  Show(g_dialog, ui_view, x, y, width, height, true);
}

void host_hide_dialog(void) {
  Hide(g_dialog);
}

void host_add_site_controls(void* view, void (*dismissed)(void)) {
  Adopt(g_site_controls, view, false, dismissed);
}

void host_set_site_controls_shape(const double*, size_t) {}

void host_show_site_controls(void* ui_view, double left, double top, double width, double height) {
  Show(g_site_controls, ui_view, left, top, width, height, true);
}

void host_set_site_controls_height(double height) {
  Resize(g_site_controls, height);
}

void host_site_controls_drew(double, double, double, double) {}

void host_release_site_controls(void) {
  g_site_controls.mouse_through = true;
  if (GetForegroundWindow() == g_site_controls.hwnd) {
    g_site_controls.hiding = true;
    SetForegroundWindow(GetWindow(g_site_controls.hwnd, GW_OWNER));
    g_site_controls.hiding = false;
  }
}

void host_hide_site_controls(void) {
  Hide(g_site_controls);
  g_site_controls.mouse_through = false;
}

void host_add_capture(void* view, void (*dismissed)(void)) {
  Adopt(g_capture, view, false, dismissed);
}

void host_show_capture(void* ui_view, double left, double top, double width, double height) {
  Show(g_capture, ui_view, left, top, width, height, true);
}

void host_capture_camera(bool camera) {
  SetCursor(LoadCursorW(nullptr, camera ? IDC_CROSS : IDC_ARROW));
}

void host_hide_capture(void) {
  Hide(g_capture);
}

void host_copy_png(const uint8_t* png, size_t size) {
  static const UINT png_format = RegisterClipboardFormatW(L"PNG");
  if (!OpenClipboard(nullptr)) {
    return;
  }
  EmptyClipboard();
  if (HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, size)) {
    memcpy(GlobalLock(memory), png, size);
    GlobalUnlock(memory);
    if (!SetClipboardData(png_format, memory)) {
      GlobalFree(memory);
    }
  }
  CloseClipboard();
}

void host_save_png(void* ui_view, const uint8_t* png, size_t size) {
  wchar_t path[MAX_PATH];
  SYSTEMTIME now;
  GetLocalTime(&now);
  swprintf_s(path, L"Capture %04d-%02d-%02d at %02d.%02d.%02d.png", now.wYear, now.wMonth,
             now.wDay, now.wHour, now.wMinute, now.wSecond);
  OPENFILENAMEW save{};
  save.lStructSize = sizeof save;
  save.hwndOwner = Root(ui_view);
  save.lpstrFilter = L"PNG image\0*.png\0";
  save.lpstrFile = path;
  save.nMaxFile = MAX_PATH;
  save.lpstrDefExt = L"png";
  save.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST;
  if (!GetSaveFileNameW(&save)) {
    return;
  }
  HANDLE file = CreateFileW(path, GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, 0, nullptr);
  if (file != INVALID_HANDLE_VALUE) {
    DWORD written;
    WriteFile(file, png, static_cast<DWORD>(size), &written, nullptr);
    CloseHandle(file);
  }
}

// shortcut: no sharing on Windows yet; port it to the Share sheet (DataTransferManager).
void host_share_png(const uint8_t*, size_t, void*, double, double, double, double, bool) {}

void host_add_toast(void* toast_view) {
  Adopt(g_toast, toast_view, true, nullptr);
}

void host_toast_takes_clicks(bool takes) {
  g_toast.mouse_through = !takes;
}

// shortcut: while it takes clicks, its whole window does, the margin round the plate included.
void host_set_toast_plate(double, double, double, double) {}

void host_show_toast(void* ui_view) {
  double content_width, content_height, width, height;
  ContentSize(Root(ui_view), &content_width, &content_height);
  WindowSize(g_toast.hwnd, &width, &height);
  Show(g_toast, ui_view, content_width - width, 0, -1, -1, false);
}

void host_hide_toast(void) {
  Hide(g_toast);
}

void host_add_tooltip(void* tooltip_view, double margin, void (*)(void)) {
  Adopt(g_tooltip, tooltip_view, true, nullptr);
  g_tooltip_margin = margin;
}

// shortcut: tooltips and the link target show and hide at once, without their fade.
void host_show_tooltip(void* ui_view, double left, double top, double) {
  Show(g_tooltip, ui_view, left - g_tooltip_margin, top - g_tooltip_margin, -1, -1, false);
}

void host_hide_tooltip(double) {
  Hide(g_tooltip);
}

void host_add_link_target(void* link_view, double margin, void (*pointer)(double x, double y)) {
  Adopt(g_link_target, link_view, true, nullptr);
  g_link_margin = margin;
  g_link_pointer = pointer;
}

void host_show_link_target(void* ui_view, double left, double top, double) {
  g_link_shown_over = ui_view;
  Show(g_link_target, ui_view, left - g_link_margin, top - g_link_margin, -1, -1, false);
  WatchPointer();
}

void host_hide_link_target(double) {
  Hide(g_link_target);
  MaybeStopWatchingPointer();
}

void host_focus(void* view) {
  HWND hwnd = Hwnd(view);
  HWND root = GetAncestor(hwnd, GA_ROOT);
  if (GetForegroundWindow() != root) {
    SetForegroundWindow(root);
  }
  SetFocus(hwnd);
}

// shortcut: menus, ilo's own windows on the Mac (mac/menu_window.h), don't show on Windows yet.
void host_add_menu_window(void*,
                          bool (*)(int, double, double),
                          void (*)(void),
                          bool (*)(const char*)) {}

void host_show_menu_window(void*, bool, double, double, double, double) {}

void host_hide_menu_window(void) {}

void host_menu_area(void*, bool, double* area) {
  area[0] = area[1] = area[2] = area[3] = 0;
}

int host_ask(const char* message, const char* detail, const char* const* buttons, size_t count) {
  std::wstring title = Wide(message), text = Wide(detail);
  std::vector<std::wstring> labels;
  std::vector<TASKDIALOG_BUTTON> task_buttons;
  for (size_t i = 0; i < count; ++i) {
    labels.push_back(Wide(buttons[i]));
  }
  int cancel = -1;
  for (size_t i = 0; i < count; ++i) {
    // Escape and the close box answer as Cancel.
    if (labels[i] == L"Cancel") {
      cancel = static_cast<int>(i);
      task_buttons.push_back({IDCANCEL, labels[i].c_str()});
    } else {
      task_buttons.push_back({static_cast<int>(100 + i), labels[i].c_str()});
    }
  }
  TASKDIALOGCONFIG config{};
  config.cbSize = sizeof config;
  config.hwndParent = GetForegroundWindow();
  config.dwFlags = cancel >= 0 ? TDF_ALLOW_DIALOG_CANCELLATION : 0;
  config.pszMainInstruction = title.c_str();
  config.pszContent = text.c_str();
  config.cButtons = static_cast<UINT>(task_buttons.size());
  config.pButtons = task_buttons.data();
  config.nDefaultButton = task_buttons.empty() ? 0 : task_buttons[0].nButtonID;
  // Common Controls 6, which has task dialogs, only where the process's manifest asks for it.
  using TaskDialogIndirectFn = HRESULT(WINAPI*)(const TASKDIALOGCONFIG*, int*, int*, BOOL*);
  auto task_dialog = reinterpret_cast<TaskDialogIndirectFn>(
      GetProcAddress(LoadLibraryW(L"comctl32.dll"), "TaskDialogIndirect"));
  int pressed = 0;
  if (task_dialog && SUCCEEDED(task_dialog(&config, &pressed, nullptr, nullptr))) {
    return pressed == IDCANCEL ? cancel : pressed - 100;
  }
  // The first button as OK, and Cancel if there is one.
  const int answer = MessageBoxW(config.hwndParent, (title + L"\n\n" + text).c_str(), L"",
                                 cancel >= 0 ? MB_OKCANCEL : MB_OK);
  return answer == IDCANCEL ? cancel : 0;
}

// shortcut: Windows' own title bar has the window's buttons, so the UI's aren't shown or hidden.
void host_show_window_buttons(void*, bool) {}

void host_press_window_button(void* ui_view, int button) {
  HWND root = Root(ui_view);
  const WPARAM command = button == 0   ? SC_CLOSE
                         : button == 1 ? SC_MINIMIZE
                         : IsZoomed(root) ? SC_RESTORE
                                          : SC_MAXIMIZE;
  PostMessageW(root, WM_SYSCOMMAND, command, 0);
}

void host_add_reveal(void* ui_view,
                     void* reveal_view,
                     void (*pointer_moved)(void* ui_view, double x)) {
  Reveal& reveal = g_reveals[ui_view];
  Adopt(reveal.floating, reveal_view, false, nullptr);
  reveal.pointer_moved = pointer_moved;
}

void host_show_reveal(void* ui_view, double width) {
  auto it = g_reveals.find(ui_view);
  if (it == g_reveals.end()) {
    return;
  }
  double content_width, content_height;
  ContentSize(Root(ui_view), &content_width, &content_height);
  Show(it->second.floating, ui_view, 0, 0, width, content_height, false);
  WatchPointer();
}

void host_hide_reveal(void* ui_view) {
  if (auto it = g_reveals.find(ui_view); it != g_reveals.end()) {
    Hide(it->second.floating);
  }
  MaybeStopWatchingPointer();
}

void host_add_settings(void* settings_view) {
  g_settings.hwnd = Hwnd(settings_view);
  ShowWindow(g_settings.hwnd, SW_HIDE);
}

void host_show_settings(void) {
  if (!g_settings_placed) {
    // Centred over the browser window in front, the first time.
    if (HWND front = GetForegroundWindow(); UiViewOf(front)) {
      RECT over{}, own{};
      GetWindowRect(front, &over);
      GetWindowRect(g_settings.hwnd, &own);
      const int width = own.right - own.left, height = own.bottom - own.top;
      SetWindowPos(g_settings.hwnd, nullptr, (over.left + over.right - width) / 2,
                   (over.top + over.bottom - height) / 2, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
    }
    g_settings_placed = true;
  }
  ShowWindow(g_settings.hwnd, SW_SHOW);
  SetForegroundWindow(g_settings.hwnd);
}

void host_hide_settings(void) {
  ShowWindow(g_settings.hwnd, SW_HIDE);
}

// shortcut: Ctrl+W closes only browser windows' tabs; closing Chrome's own windows, such as an
// undocked DevTools, comes with their close buttons.
bool host_close_other_window(void) {
  return false;
}

void host_detent(void) {}

// No Dock. shortcut: the taskbar button's overlay icon could carry the Space's color.
void host_dock_dot_launch(const char*) {}
void host_dock_dot(bool, uint32_t) {}

void host_watch_control(void (*released)(void)) {
  if ((GetKeyState(VK_CONTROL) & 0x8000) == 0) {
    Later([released] { released(); });
    return;
  }
  g_control_released = released;
  if (!g_key_hook) {
    g_key_hook = SetWindowsHookExW(WH_KEYBOARD, KeyPressed, nullptr, GetCurrentThreadId());
  }
}

// shortcut: drags onto the UI from pages and other apps aren't taken yet; GPUI's own drop target
// has the window.
void host_take_drops(bool (*)(void*, int, double, double, const host_dragged*, size_t)) {}

bool host_trash(const char* path) {
  // Double-null-terminated, as the shell asks.
  std::wstring from = Wide(path) + L'\0';
  SHFILEOPSTRUCTW operation{};
  operation.wFunc = FO_DELETE;
  operation.pFrom = from.c_str();
  operation.fFlags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT;
  return SHFileOperationW(&operation) == 0 ||
         GetFileAttributesW(Wide(path).c_str()) == INVALID_FILE_ATTRIBUTES;
}

// shortcut: a download can't be dragged out yet.
bool host_drag_file(const char*, void (*)(bool)) {
  return false;
}

// shortcut: no file previews yet; the downloads list shows its generic icon.
void host_file_preview(const char*,
                       uint64_t id,
                       void (*done)(uint64_t, const uint8_t*, size_t, double, double, bool)) {
  Later([done, id] { done(id, nullptr, 0, 0, 0, false); });
}

void host_copy(const char* text) {
  std::wstring wide = Wide(text);
  if (!OpenClipboard(nullptr)) {
    return;
  }
  EmptyClipboard();
  const size_t bytes = (wide.size() + 1) * sizeof(wchar_t);
  if (HGLOBAL memory = GlobalAlloc(GMEM_MOVEABLE, bytes)) {
    memcpy(GlobalLock(memory), wide.c_str(), bytes);
    GlobalUnlock(memory);
    if (!SetClipboardData(CF_UNICODETEXT, memory)) {
      GlobalFree(memory);
    }
  }
  CloseClipboard();
}

// There's no Safari: Windows plays protected video with Chromium's own PlayReady (decision 1).
void host_open_in_safari(const char* url) {
  fprintf(stderr, "browser: no system engine for %s\n", url);
}

void host_share(void*, double, double, double, double, const char*, const char*) {}
void host_share_services(const char*, void (*)(void*, const char*), void*) {}
void host_share_with(const char*, const char*, size_t) {}

void host_day_starts(double starts[4]) {
  const time_t now = time(nullptr);
  tm today{};
  localtime_s(&today, &now);
  today.tm_hour = today.tm_min = today.tm_sec = 0;
  today.tm_isdst = -1;
  tm yesterday = today;
  yesterday.tm_mday -= 1;
  // The user's first day of the week: 0 is Monday.
  wchar_t first[2] = L"0";
  GetLocaleInfoEx(LOCALE_NAME_USER_DEFAULT, LOCALE_IFIRSTDAYOFWEEK, first, 2);
  const int first_day = ((first[0] - L'0') + 1) % 7;  // As tm_wday counts, from Sunday.
  tm week = today;
  week.tm_mday -= (today.tm_wday - first_day + 7) % 7;
  tm last_week = week;
  last_week.tm_mday -= 7;
  starts[0] = mktime(&today) * 1000.0;
  starts[1] = mktime(&yesterday) * 1000.0;
  starts[2] = mktime(&week) * 1000.0;
  starts[3] = mktime(&last_week) * 1000.0;
}

bool host_choose(const char*) {
  return false;
}

void* host_browser_window(size_t) {
  return nullptr;
}

}  // extern "C"
