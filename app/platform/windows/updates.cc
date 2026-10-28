// Updates on Windows: WinSparkle, reading the Mac's appcast and its EdDSA signatures, for a copy
// installed with winget or from the website. A copy from the Microsoft Store is an MSIX, which the
// Store updates, so it never starts WinSparkle. WinSparkle.dll sits beside Browser.dll and loads
// once the first window shows, so launch never waits for it.

#include <windows.h>

#include <appmodel.h>

#include <cstdio>
#include <string>

#include "../host.h"

// host.cc's: runs |task| on the main thread.
void HostLater(void (*task)(void));

namespace {

// A release build's, from build.rs: the feed, the key its updates are signed with, and the version
// and build number Sparkle's are, as chrome.exe's own version resource is Chromium's.
#if defined(UPDATE_FEED) && defined(UPDATE_PUBLIC_KEY) && defined(APP_VERSION) && defined(APP_BUILD)
constexpr bool kReleaseBuild = true;
#else
constexpr bool kReleaseBuild = false;
#define UPDATE_FEED ""
#define UPDATE_PUBLIC_KEY ""
#define APP_VERSION ""
#define APP_BUILD ""
#endif
#define WIDE2(s) L##s
#define WIDE(s) WIDE2(s)

// Where WinSparkle keeps whether it checks and when it last did, under HKEY_CURRENT_USER.
constexpr wchar_t kSettings[] = L"Software\\ilo\\WinSparkle";

// WinSparkle's functions, looked up rather than linked, so a copy without it runs.
struct WinSparkle {
  void(__cdecl* set_appcast_url)(const char*);
  int(__cdecl* set_eddsa_public_key)(const char*);
  void(__cdecl* set_app_details)(const wchar_t*, const wchar_t*, const wchar_t*);
  void(__cdecl* set_app_build_version)(const wchar_t*);
  void(__cdecl* set_registry_path)(const char*);
  void(__cdecl* set_automatic_check_for_updates)(int);
  void(__cdecl* set_update_check_interval)(int);
  void(__cdecl* set_can_shutdown_callback)(int(__cdecl*)());
  void(__cdecl* set_shutdown_request_callback)(void(__cdecl*)());
  void(__cdecl* init)();
  void(__cdecl* check_update_with_ui)();
};

WinSparkle g_sparkle;
bool g_started = false;
void (*g_quit)(void) = nullptr;

// The folder Browser.dll is in, with a backslash.
std::wstring OwnFolder() {
  HMODULE module = nullptr;
  GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS |
                         GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                     reinterpret_cast<LPCWSTR>(&OwnFolder), &module);
  std::wstring path(MAX_PATH, L'\0');
  for (;;) {
    const DWORD length = GetModuleFileNameW(module, path.data(), static_cast<DWORD>(path.size()));
    if (length < path.size()) {
      path.resize(length);
      break;
    }
    path.resize(path.size() * 2);
  }
  return path.substr(0, path.find_last_of(L'\\') + 1);
}

// An MSIX has a package identity; winget's and the website's installs don't.
bool FromTheStore() {
  UINT32 length = 0;
  return GetCurrentPackageFullName(&length, nullptr) != APPMODEL_ERROR_NO_PACKAGE;
}

int __cdecl CanQuit() {
  return 1;
}

// WinSparkle has started the installer, which waits for the browser to quit.
void __cdecl QuitForUpdate() {
  // From WinSparkle's thread; the core is the main thread's.
  if (g_quit) {
    HostLater(g_quit);
  }
}

template <typename F>
bool Find(HMODULE dll, const char* name, F* function) {
  *function = reinterpret_cast<F>(GetProcAddress(dll, name));
  return *function != nullptr;
}

}  // namespace

bool host_updates_available(void) {
  static const bool available = kReleaseBuild && !FromTheStore() &&
                                GetFileAttributesW((OwnFolder() + L"WinSparkle.dll").c_str()) !=
                                    INVALID_FILE_ATTRIBUTES;
  return available;
}

void host_start_updater(void (*quit)(void)) {
  if (g_started || !host_updates_available()) {
    return;
  }
  g_started = true;
  g_quit = quit;
  HMODULE dll = LoadLibraryExW((OwnFolder() + L"WinSparkle.dll").c_str(), nullptr,
                               LOAD_WITH_ALTERED_SEARCH_PATH);
  WinSparkle& s = g_sparkle;
  if (!dll || !Find(dll, "win_sparkle_set_appcast_url", &s.set_appcast_url) ||
      !Find(dll, "win_sparkle_set_eddsa_public_key", &s.set_eddsa_public_key) ||
      !Find(dll, "win_sparkle_set_app_details", &s.set_app_details) ||
      !Find(dll, "win_sparkle_set_app_build_version", &s.set_app_build_version) ||
      !Find(dll, "win_sparkle_set_registry_path", &s.set_registry_path) ||
      !Find(dll, "win_sparkle_set_automatic_check_for_updates",
            &s.set_automatic_check_for_updates) ||
      !Find(dll, "win_sparkle_set_update_check_interval", &s.set_update_check_interval) ||
      !Find(dll, "win_sparkle_set_can_shutdown_callback", &s.set_can_shutdown_callback) ||
      !Find(dll, "win_sparkle_set_shutdown_request_callback", &s.set_shutdown_request_callback) ||
      !Find(dll, "win_sparkle_init", &s.init) ||
      !Find(dll, "win_sparkle_check_update_with_ui", &s.check_update_with_ui)) {
    fprintf(stderr, "browser: no updates: WinSparkle.dll didn't load\n");
    s.check_update_with_ui = nullptr;
    return;
  }
  s.set_appcast_url(UPDATE_FEED);
  if (!s.set_eddsa_public_key(UPDATE_PUBLIC_KEY)) {
    fprintf(stderr, "browser: no updates: the update key isn't an EdDSA public key\n");
    s.check_update_with_ui = nullptr;
    return;
  }
  s.set_app_details(L"ilo", L"ilo", WIDE(APP_VERSION));
  // The appcast's sparkle:version is the build number, as on the Mac.
  s.set_app_build_version(WIDE(APP_BUILD));
  s.set_registry_path("Software\\ilo\\WinSparkle");  // kSettings
  // Checks are on from the first launch, as Sparkle's are on the Mac, rather than asked about
  // on the second; once WinSparkle has the user's answer, it's theirs.
  if (RegGetValueW(HKEY_CURRENT_USER, kSettings, L"CheckForUpdates", RRF_RT_ANY, nullptr,
                   nullptr, nullptr) != ERROR_SUCCESS) {
    s.set_automatic_check_for_updates(1);
  }
  s.set_update_check_interval(8 * 60 * 60);
  s.set_can_shutdown_callback(CanQuit);
  s.set_shutdown_request_callback(QuitForUpdate);
  s.init();
  fprintf(stderr, "browser: updater started\n");
}

void host_check_for_updates(void) {
  if (g_sparkle.check_update_with_ui) {
    g_sparkle.check_update_with_ui();
  }
}
