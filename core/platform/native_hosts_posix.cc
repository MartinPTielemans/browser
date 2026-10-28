#include "browser_core/platform/native_hosts.h"

#include <memory>

#include "base/command_line.h"
#include "base/files/file_util.h"
#include "build/build_config.h"

#if BUILDFLAG(IS_LINUX)
#include "base/environment.h"
#include "base/nix/xdg_util.h"
#endif

namespace browser_core::platform {

namespace {

// Each browser's user data folder, whose NativeMessagingHosts holds its user-level hosts'
// manifests, as Chrome's does.
#if BUILDFLAG(IS_MAC)
// Under ~/Library/Application Support. Arc keeps its profiles a folder down.
constexpr const char* kBrowsers[] = {
    "Google/Chrome",
    "Google/Chrome Beta",
    "Google/Chrome Dev",
    "Google/Chrome Canary",
    "Chromium",
    "Microsoft Edge",
    "Microsoft Edge Beta",
    "Microsoft Edge Dev",
    "Microsoft Edge Canary",
    "BraveSoftware/Brave-Browser",
    "Arc/User Data",
    "Vivaldi",
};
#else
// Under ~/.config, or $XDG_CONFIG_HOME. Chrome Dev is unstable there, and there's no Arc.
constexpr const char* kBrowsers[] = {
    "google-chrome",
    "google-chrome-beta",
    "google-chrome-unstable",
    "google-chrome-canary",
    "chromium",
    "microsoft-edge",
    "microsoft-edge-beta",
    "microsoft-edge-dev",
    "BraveSoftware/Brave-Browser",
    "vivaldi",
};
#endif

base::FilePath BrowsersFolder() {
  const base::CommandLine& command_line = *base::CommandLine::ForCurrentProcess();
  if (command_line.HasSwitch(kOtherBrowsersDir)) {
    return command_line.GetSwitchValuePath(kOtherBrowsersDir);
  }
#if BUILDFLAG(IS_MAC)
  // The user's real home, as Chrome's own folders are: NSHomeDirectory ignores $HOME.
  return base::GetHomeDir().Append("Library").Append("Application Support");
#else
  std::unique_ptr<base::Environment> env = base::Environment::Create();
  return base::nix::GetXDGDirectory(env.get(), base::nix::kXdgConfigHomeEnvVar,
                                    base::nix::kDotConfigDir);
#endif
}

}  // namespace

base::FilePath FindOtherBrowsersNativeHost(const std::string& name) {
  base::FilePath browsers = BrowsersFolder();
  for (const char* browser : kBrowsers) {
    base::FilePath manifest =
        browsers.Append(browser).Append("NativeMessagingHosts").Append(name + ".json");
    if (base::PathExists(manifest)) {
      return manifest;
    }
  }
  return base::FilePath();
}

}  // namespace browser_core::platform
