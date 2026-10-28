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

#include <ctype.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include <functional>
#include <map>
#include <set>
#include <string>
#include <vector>

extern "C" char** environ;

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
extern "C" unsigned g_child_watch_add(int pid,
                                      void (*function)(int pid, int status, void* data),
                                      void* data) __attribute__((weak));

// The core's (core/public/browser_core.h), from the chrome that loads the app. Weak, as GLib's.
extern "C" void bc_window_set_title(uint64_t window, const char* title) __attribute__((weak));
extern "C" void bc_window_activate(uint64_t window) __attribute__((weak));

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

// The core's id for each browser window, by the view its UI draws in (host_window_opened).
std::map<void*, uint64_t>& WindowIds() {
  static auto* ids = new std::map<void*, uint64_t>;
  return *ids;
}

uint64_t WindowOf(void* ui_view) {
  auto it = WindowIds().find(ui_view);
  return it == WindowIds().end() ? 0 : it->second;
}

void (*g_key_window_changed)(void* ui_view);
void (*g_became_active)(void);
void (*g_resigning_active)(void);

// The browser windows that have the keyboard, of those whose whole window GPUI hears of: the app is
// active while one does.
std::set<void*>& ActiveWindows() {
  static auto* active = new std::set<void*>;
  return *active;
}

bool g_app_active = false;

// Whether the app became active or stopped being, after the keyboard moved. Going from one
// browser window to another, it leaves the first before it reaches the second, so this waits a
// turn of the loop rather than calling the app inactive between them.
void CheckAppActive() {
  Later([] {
    const bool active = !ActiveWindows().empty();
    if (active == g_app_active) {
      return;
    }
    g_app_active = active;
    // Once the app has stopped being active, where the Mac's comes just before.
    void (*changed)(void) = active ? g_became_active : g_resigning_active;
    if (changed) {
      changed();
    }
  });
}

// The desktop entry the desktop knows the browser by, which the Flatpak installs, from
// branding/id (build.rs).
constexpr char kDesktopId[] = APP_ID ".desktop";

std::vector<std::string> Split(const char* list, const char* fallback) {
  std::vector<std::string> parts;
  std::string all = list && *list ? list : fallback;
  for (size_t start = 0; start <= all.size();) {
    size_t end = all.find(':', start);
    end = end == std::string::npos ? all.size() : end;
    if (end > start) {
      parts.push_back(all.substr(start, end - start));
    }
    start = end + 1;
  }
  return parts;
}

// The mimeapps.list files that say which app opens a type, most telling first, as the XDG MIME
// Applications spec orders them: the user's, the system's, then the old place in each data folder,
// each led by the current desktop's own.
std::vector<std::string> MimeappsLists() {
  const char* home = getenv("HOME");
  const std::string home_dir = home ? home : "";
  const char* config_home = getenv("XDG_CONFIG_HOME");
  const char* data_home = getenv("XDG_DATA_HOME");
  std::vector<std::string> dirs = {config_home && *config_home ? config_home
                                                               : home_dir + "/.config"};
  for (const std::string& dir : Split(getenv("XDG_CONFIG_DIRS"), "/etc/xdg")) {
    dirs.push_back(dir);
  }
  dirs.push_back((data_home && *data_home ? data_home : home_dir + "/.local/share") +
                 std::string("/applications"));
  for (const std::string& dir : Split(getenv("XDG_DATA_DIRS"), "/usr/local/share:/usr/share")) {
    dirs.push_back(dir + "/applications");
  }
  std::vector<std::string> desktops = Split(getenv("XDG_CURRENT_DESKTOP"), "");
  for (std::string& desktop : desktops) {
    for (char& c : desktop) {
      c = static_cast<char>(tolower(static_cast<unsigned char>(c)));
    }
  }
  std::vector<std::string> lists;
  for (const std::string& dir : dirs) {
    for (const std::string& desktop : desktops) {
      lists.push_back(dir + "/" + desktop + "-mimeapps.list");
    }
    lists.push_back(dir + "/mimeapps.list");
  }
  return lists;
}

std::string Trimmed(const std::string& text) {
  const size_t start = text.find_first_not_of(" \t\r\n");
  const size_t end = text.find_last_not_of(" \t\r\n");
  return start == std::string::npos ? "" : text.substr(start, end - start + 1);
}

// The desktop entry that opens |type|: the first the first list naming one gives. The desktop
// skips one that isn't installed, which this doesn't, so a default browser since removed reads as
// the default still, and this one as not.
std::string DefaultFor(const std::string& type) {
  for (const std::string& path : MimeappsLists()) {
    FILE* file = fopen(path.c_str(), "r");
    if (!file) {
      continue;
    }
    std::string found;
    bool defaults = false;
    char line[4096];
    while (found.empty() && fgets(line, sizeof(line), file)) {
      const std::string text = Trimmed(line);
      if (!text.empty() && text[0] == '[') {
        defaults = text == "[Default Applications]";
        continue;
      }
      const size_t equals = text.find('=');
      if (defaults && equals != std::string::npos && Trimmed(text.substr(0, equals)) == type) {
        const std::string apps = text.substr(equals + 1);
        found = Trimmed(apps.substr(0, apps.find(';')));
      }
    }
    fclose(file);
    if (!found.empty()) {
      return found;
    }
  }
  return "";
}

}  // namespace

extern "C" {

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

// No launch window. It shows the last sidebar where the first browser window opens, which Wayland
// gives no client a say in (host_centered), and a tiling compositor such as Hyprland would lay the
// screen out twice, for it and then for the browser window. Chromium's window goes on screen as the
// core opens it instead.
void host_show_launch_window(const char* profile_dir, double x, double y, double width, double height) {}

void host_close_launch_window(void) {}

// No launch window shows it (host_show_launch_window), so none is kept.
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

// The core shows the browser window as it opens it, in front or for a saved window behind,
// since the window is Chromium's.
void host_show_window(void* ui_view) {}

void host_show_window_behind(void* ui_view, void* above_ui_view) {}

void host_window_opened(void* ui_view, uint64_t window) {
  WindowIds()[ui_view] = window;
}

void host_window_active(void* ui_view, bool active, bool whole_window) {
  if (active && g_key_window_changed) {
    g_key_window_changed(ui_view);
  }
  // On X11 a click in the page takes the keyboard from GPUI's window but leaves the app active,
  // and GPUI hears nothing of the browser window's own.
  if (!whole_window) {
    return;
  }
  if (active) {
    ActiveWindows().insert(ui_view);
  } else {
    ActiveWindows().erase(ui_view);
  }
  CheckAppActive();
}

void host_capture_name(char* name, size_t capacity) {
  const time_t now = time(nullptr);
  tm local{};
  localtime_r(&now, &local);
  if (!strftime(name, capacity, "Capture %Y-%m-%d at %H.%M.%S.png", &local) && capacity) {
    name[0] = 0;
  }
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

// No dock dot. The docks that badge a launcher, KDE's, Dash to Dock and Plank among them, read the
// Unity LauncherEntry API, which gives it a count, progress or urgency but no colour, so the
// Space's has nothing to show in.
void host_dock_dot_launch(const char* profile_dir) {}

void host_dock_dot(bool colored, uint32_t color) {}

// The compositor places each window (host_centered), so the next opens where it's put, at the size
// the app asks for, the default one, as nothing here reads the size of the one before.
void host_cascaded(void* ui_view, double* x, double* y, double* width, double* height) {}

// Through the core, as the window is Chromium's: under Wayland its toplevel, which GPUI's window is
// a subsurface in.
void host_set_window_title(void* ui_view, const char* title) {
  if (const uint64_t window = WindowOf(ui_view); window && bc_window_set_title) {
    bc_window_set_title(window, title);
  }
}

// Through the core, as host_set_window_title. Under Wayland the compositor decides whether the
// window comes forward or only asks for attention, as Hyprland's focus_on_activate setting does.
void host_activate(void* ui_view) {
  if (const uint64_t window = WindowOf(ui_view); window && bc_window_activate) {
    bc_window_activate(window);
  }
}

// The app reports each browser window GPUI's window in it hears become active (host_window_active).
void host_watch_key_window(void (*changed)(void* ui_view)) {
  g_key_window_changed = changed;
}

// shortcut: nothing says when the browser window goes full screen, as the window manager's own
// shortcut does it, and only Chromium's window hears it; the core's widget would report it.
void host_watch_full_screen(void (*changed)(void* ui_view, bool entered)) {}

// GPUI's Wayland client hands the app the touchpad's three-finger swipes over the page
// (gpui_linux::watch_embedder_swipes, in main.rs), which the compositor sends unless it keeps them
// for its own, such as for workspaces. X11 sends them to the page's window alone, Chromium's.
void host_watch_swipes(bool (*swiped)(void* ui_view, double x, double y, bool back)) {}
bool host_after_menus(void (*apply)(void)) { return false; }

void host_watch_resigning_active(void (*resigning)(void)) {
  g_resigning_active = resigning;
}

void host_watch_active(void (*became_active)(void)) {
  g_became_active = became_active;
}

// Read as xdg-settings reads it on most desktops. A Flatpak sees its own config rather than the
// desktop's, and no portal reads the setting, so there it reads as not the default.
bool host_is_default_browser(void) {
  return DefaultFor("x-scheme-handler/http") == kDesktopId &&
         DefaultFor("x-scheme-handler/https") == kDesktopId;
}

// xdg-settings sets it as each desktop keeps it, as Chromium does on Linux, and asks the user
// nothing. A Flatpak has no xdg-settings and no portal that sets it, so there the answer comes
// with nothing changed, and the row's detail says where to set it.
void host_make_default_browser(void (*answered)(void)) {
  const char* argv[] = {"xdg-settings", "set", "default-web-browser", kDesktopId, nullptr};
  pid_t pid = 0;
  if (!g_child_watch_add ||
      posix_spawnp(&pid, argv[0], nullptr, nullptr, const_cast<char**>(argv), environ) != 0) {
    Later([answered] { answered(); });
    return;
  }
  g_child_watch_add(
      pid, [](int, int, void* answered) { reinterpret_cast<void (*)(void)>(answered)(); },
      reinterpret_cast<void*>(answered));
}

double host_since_launch(void) {
  timespec now{};
  clock_gettime(CLOCK_BOOTTIME, &now);
  return (now.tv_sec + now.tv_nsec / 1e9 - StartedSinceBoot()) * 1000;
}

void host_window_closed(void* ui_view) {
  WindowIds().erase(ui_view);
  if (ActiveWindows().erase(ui_view)) {
    CheckAppActive();
  }
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

// shortcut: under Wayland Chromium and GPUI each get every key from a wl_keyboard of their own,
// so the host can't keep the keys typed after the command bar's shortcut from the page; holding
// them is for the core's key handling to do.
void host_hold_keys(void) {}

// GPUI's clipboard and its file chooser portal copy and save a capture (platform/linux/system.rs).
void host_copy_png(const uint8_t* png, size_t size) {}

void host_save_png(void* ui_view, const uint8_t* png, size_t size) {}

// No system share menu on Linux: capture copies instead, and says so (capture.rs).
void host_share_png(const uint8_t* png, size_t size, void* view, double x, double y, double width, double height, bool messages) {}

// shortcut: no hover card on Linux yet; its window is never shown, so the card is off.
void host_add_hover_card(void* card_view, double margin, void (*away)(void)) {}

void host_show_hover_card(void* ui_view, double left, double top, double width, double height) {}

void host_hide_hover_card(double fade) {}

void host_watch_memory_pressure(void (*pressed)(void)) {}

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
