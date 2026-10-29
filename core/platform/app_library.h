#ifndef BROWSER_CORE_PLATFORM_APP_LIBRARY_H_
#define BROWSER_CORE_PLATFORM_APP_LIBRARY_H_

#include <string>
#include <vector>

#include "browser_core/public/browser_core.h"

namespace browser_core::platform {

// Where the sandbox needs Chromium's own executable to start every process, the app is a library
// beside it rather than the executable that calls bc_main (bc_main in public/browser_core.h).
// Loads it in the browser process, gives it the arguments the process was |given|, and returns its
// callbacks, which it keeps for as long as the process runs, and adds the switches it would have
// passed bc_main to |switches|. Null with no app to run.
const bc_host* LoadApp(const std::vector<std::string>& given, std::vector<std::string>* switches);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_APP_LIBRARY_H_
