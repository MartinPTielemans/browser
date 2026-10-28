// The Linux host: what platform/mac/host.mm does for the UI, on X11 for now (decision 3). The
// browser runs as libBrowser.so in Chromium's own chrome, and GPUI's window sits in the browser
// window as the core puts it there.
//
// shortcut: most of it does nothing yet. Menus, alerts and the rest that float over the page need
// child windows of their own; a function with an empty body leaves its part out, and one that
// answers says "no" or "none". The overlays, the command bar, the icon picker and the dialog, are
// overlays.rs's, the find bar and the offer are panels.rs's, ilo's menus are menu_window.rs's, the
// HUD, the drop targets, the toast, the tooltip and the link target are floats.rs's, the Site
// Controls and Capture are popovers.rs's, the sidebar reveal is reveal_window.rs's and Settings is
// settings_window.rs's, as GPUI's Linux platform shows them.

#include "../host.h"

#include <stdio.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include <functional>

namespace {

// When the process started, in seconds since boot: field 22 of /proc/self/stat, in clock ticks.
double StartedSinceBoot() {
  char stat[1024] = {};
  if (FILE* file = fopen("/proc/self/stat", "r")) {
    fread(stat, 1, sizeof(stat) - 1, file);
    fclose(file);
  }
  // After the command's name, which can hold spaces, the third field is the state.
  const char* field = strrchr(stat, ')');
  unsigned long long ticks = 0;
  if (field && sscanf(field + 2, "%*c %*d %*d %*d %*d %*d %*u %*u %*u %*u %*u %*u %*u %*d %*d %*d %*d %*d %*d %llu", &ticks) == 1) {
    return static_cast<double>(ticks) / sysconf(_SC_CLK_TCK);
  }
  return 0;
}

// GLib's, from the chrome that loads the app, whose loop is GLib's (platform/linux in core/). Weak,
// so the preview, which has no GLib loop, links without it.
extern "C" unsigned g_idle_add(int (*function)(void*), void* data) __attribute__((weak));
extern "C" unsigned g_unix_fd_add_full(int priority,
                                       int fd,
                                       unsigned condition,
                                       int (*function)(int fd, unsigned condition, void* data),
                                       void* data,
                                       void (*notify)(void* data)) __attribute__((weak));
constexpr unsigned kGlibIoIn = 1;
// Chromium's fd watches', its Wayland display's among them (G_PRIORITY_DEFAULT_IDLE - 10). GLib
// checks only the sources as urgent as the most urgent ready one, and the display's check is where
// Chromium reads what it prepared to read, or gives that up; above it, GPUI would run with the read
// still prepared, and opening a window, which Vulkan does with a roundtrip, would wait for it forever.
constexpr int kGlibPriorityFdWatch = 200 - 10;

// Runs |task| once the caller has returned, from Chromium's loop: as AppKit's would, the answer
// comes after the call that asked for it.
void Later(std::function<void()> task) {
  if (!g_idle_add) {
    task();
    return;
  }
  g_idle_add(
      [](void* data) {
        auto* task = static_cast<std::function<void()>*>(data);
        (*task)();
        delete task;
        return 0;
      },
      new std::function<void()>(std::move(task)));
}

}  // namespace

extern "C" {

// shortcut: no test driver on Linux yet; BROWSER_DRIVE does nothing.
void drive_start(void) {}

// Wayland gives a client no say in where its window goes, nor sight of the displays' layout: the
// compositor places it, Hyprland on the display with the focus. X11 window managers mostly place
// a new window themselves too. So the origin is left for them, and BROWSER_DISPLAY has nothing
// to pick.
void host_centered(double width, double height, double* x, double* y) {
  (void)width;
  (void)height;
  *x = 0;
  *y = 0;
}

void host_show_launch_window(const char* profile_dir, double x, double y, double width, double height) {}

void host_close_launch_window(void) {}

bool host_save_launch_picture(void* ui_view, const char* profile_dir, double width) {
  return false;
}

// The UI's view on Linux is GPUI's own (gpui_linux::new_host_window), which the app makes
// itself.
void* host_ui_view(void) {
  return nullptr;
}

void host_watch_fd(int fd, void (*ready)(void)) {
  if (!g_unix_fd_add_full) {
    return;
  }
  g_unix_fd_add_full(
      kGlibPriorityFdWatch, fd, kGlibIoIn,
      [](int, unsigned, void* ready) {
        reinterpret_cast<void (*)(void)>(ready)();
        return 1;
      },
      reinterpret_cast<void*>(ready), nullptr);
}

void host_show_window(void* ui_view) {}

void host_show_window_behind(void* ui_view, void* above_ui_view) {
  host_show_window(ui_view);
}

// No sight of the displays' layout (host_centered), so a window opens where the compositor puts it.
size_t host_screens(double* frames, size_t count) {
  return 0;
}

bool host_window_frame(void* ui_view, double* x, double* y, double* width, double* height) {
  return false;
}

// shortcut: links a launch carries come in on the command line, so there's nothing to wait for.
void host_watch_finished_launching(void (*finished)(void)) {
  finished();
}

// No Dock.
void host_dock_dot_launch(const char* profile_dir) {}

void host_dock_dot(bool colored, uint32_t color) {}

void host_cascaded(void* ui_view, double* x, double* y, double* width, double* height) {}

void host_set_window_title(void* ui_view, const char* title) {}

void host_activate(void* ui_view) {}

void host_watch_key_window(void (*changed)(void* ui_view)) {}

void host_watch_full_screen(void (*changed)(void* ui_view, bool entered)) {}

void host_watch_swipes(bool (*swiped)(void* ui_view, double x, double y, bool back)) {}
bool host_after_menus(void (*apply)(void)) { return false; }

void host_watch_resigning_active(void (*resigning)(void)) {}

void host_watch_active(void (*became_active)(void)) {}

// shortcut: the default browser is the desktop's to say, through xdg-settings; not asked yet.
bool host_is_default_browser(void) {
  return false;
}

void host_make_default_browser(void (*answered)(void)) {
  Later([answered] { answered(); });
}

double host_since_launch(void) {
  timespec now{};
  clock_gettime(CLOCK_BOOTTIME, &now);
  return (now.tv_sec + now.tv_nsec / 1e9 - StartedSinceBoot()) * 1000;
}

// shortcut: overlays don't shrink or cast a shadow on Linux yet (overlays.rs).
void host_set_overlay_shrink(int overlay) {}

void host_set_overlay_elevation(int overlay,
                                float outline_opacity,
                                double outline_radius,
                                float shadow_opacity,
                                double shadow_radius,
                                double shadow_drop) {}

bool host_can_draw_emoji(const char*) {
  return true;
}

void host_hold_keys(void) {}

void host_copy_png(const uint8_t* png, size_t size) {}

void host_save_png(void* ui_view, const uint8_t* png, size_t size) {}

void host_share_png(const uint8_t* png, size_t size, void* view, double x, double y, double width, double height, bool messages) {}

void host_focus(void* view) {}

int host_ask(const char* message, const char* detail, const char* const* buttons, size_t count) {
  return -1;
}

void host_show_window_buttons(void* ui_view, bool shown) {}

void host_press_window_button(void* ui_view, int button) {}

bool host_close_other_window(void) {
  return false;
}

void host_detent(void) {}

void host_watch_control(void (*released)(void)) {}

void host_take_drops(bool (*dragged)(void* ui_view, int phase, double x, double y, const host_dragged* items, size_t count)) {}

bool host_trash(const char* path) {
  return false;
}

bool host_drag_file(const char* path, void (*ended)(bool taken)) {
  return false;
}

// shortcut: no file previews yet; the downloads list shows its generic icon.
void host_file_preview(const char*,
                       uint64_t id,
                       void (*done)(uint64_t, const uint8_t*, size_t, double, double, bool)) {
  Later([done, id] { done(id, nullptr, 0, 0, 0, false); });
}

void host_copy(const char* text) {}

void host_open_in_safari(const char* url) {}

void host_open_full_disk_access(void) {}

void host_share(void* ui_view, double x, double y, double width, double height, const char* url, const char* title) {}

void host_share_services(const char* url, void (*each)(void* context, const char* name), void* context) {}

void host_share_with(const char* url, const char* title, size_t index) {}

void host_day_starts(double starts[4]) {
  const time_t now = time(nullptr);
  tm today{};
  localtime_r(&now, &today);
  today.tm_hour = today.tm_min = today.tm_sec = 0;
  today.tm_isdst = -1;
  tm yesterday = today;
  yesterday.tm_mday -= 1;
  // shortcut: weeks start on Monday, as most locales have it; the locale's own isn't read yet.
  const int first_day = 1;  // As tm_wday counts, from Sunday.
  tm week = today;
  week.tm_mday -= (today.tm_wday - first_day + 7) % 7;
  tm last_week = week;
  last_week.tm_mday -= 7;
  starts[0] = mktime(&today) * 1000.0;
  starts[1] = mktime(&yesterday) * 1000.0;
  starts[2] = mktime(&week) * 1000.0;
  starts[3] = mktime(&last_week) * 1000.0;
}

bool host_choose(const char* title) {
  return false;
}

void* host_browser_window(size_t index) {
  return nullptr;
}

}  // extern "C"
