// The native messaging hosts other Chromium browsers know. A desktop app such as 1Password writes
// its host's manifest only into the folders of the browsers it knows, so the core looks in theirs
// for one the browser's own folders lack. That trusts nothing new: their folders are the user's,
// as the browser's own is, and the manifest's allowed_origins still say which extensions connect.

#ifndef BROWSER_CORE_PLATFORM_NATIVE_HOSTS_H_
#define BROWSER_CORE_PLATFORM_NATIVE_HOSTS_H_

#include <string>

#include "base/files/file_path.h"

namespace browser_core::platform {

// The folder that holds the other browsers' folders, in place of the platform's: the check's own,
// so it never reads or writes the user's. Not on Windows, whose browsers keep registry keys.
inline constexpr char kOtherBrowsersDir[] = "other-browsers-dir";

// The manifest of the host |name| that the first of Chrome's channels, Chromium, Edge's channels,
// Brave, Arc and Vivaldi has, in that order and of those the platform has, or an empty path.
// Blocks on the disk.
base::FilePath FindOtherBrowsersNativeHost(const std::string& name);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_NATIVE_HOSTS_H_
