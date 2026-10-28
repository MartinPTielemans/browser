#include "browser_core/platform/native_hosts.h"

#include <windows.h>

#include <string>

#include "base/strings/utf_string_conversions.h"
#include "base/win/registry.h"

namespace browser_core::platform {

namespace {

// Each browser's key, whose subkey per host names its manifest. Every channel of a browser shares
// one. Chromium's and Chrome's aren't here: Chromium already reads both (launch_context_win.cc).
// Arc's isn't known.
constexpr const wchar_t* kBrowsers[] = {
    L"SOFTWARE\\Microsoft\\Edge\\NativeMessagingHosts",
    L"SOFTWARE\\BraveSoftware\\Brave-Browser\\NativeMessagingHosts",
    L"SOFTWARE\\Vivaldi\\NativeMessagingHosts",
};

}  // namespace

base::FilePath FindOtherBrowsersNativeHost(const std::string& name) {
  std::wstring host = base::UTF8ToWide(name);
  // The user's keys first, then the machine's, each in the 32-bit view and then the 64-bit one, as
  // Chrome reads its own.
  for (HKEY root : {HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE}) {
    for (const wchar_t* browser : kBrowsers) {
      for (REGSAM view : {KEY_WOW64_32KEY, KEY_WOW64_64KEY}) {
        base::win::RegKey key;
        std::wstring manifest;
        if (key.Open(root, browser, KEY_QUERY_VALUE | view) == ERROR_SUCCESS &&
            key.OpenKey(host.c_str(), KEY_QUERY_VALUE | view) == ERROR_SUCCESS &&
            key.ReadValue(nullptr, &manifest) == ERROR_SUCCESS) {
          return base::FilePath(manifest);
        }
      }
    }
  }
  return base::FilePath();
}

}  // namespace browser_core::platform
