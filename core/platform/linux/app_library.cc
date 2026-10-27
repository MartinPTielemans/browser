#include "browser_core/platform/app_library.h"

#include <dlfcn.h>

#include "base/files/file_path.h"
#include "base/logging.h"
#include "base/path_service.h"

namespace browser_core::platform {

const bc_host* LoadApp(std::vector<std::string>* switches) {
  base::FilePath dir;
  base::PathService::Get(base::DIR_EXE, &dir);
  const base::FilePath path = dir.Append(BC_APP_LIBRARY);
  // Resolved now, so a library built against another engine's C API fails here, by name, rather
  // than at its first call. It finds the API in chrome, which exports it (chrome.map).
  void* app = dlopen(path.value().c_str(), RTLD_NOW | RTLD_LOCAL);
  if (!app) {
    LOG(ERROR) << dlerror();
    return nullptr;
  }
  auto entry = reinterpret_cast<bc_app_entry>(dlsym(app, BC_APP_ENTRY));
  if (!entry) {
    return nullptr;
  }
  int argc = 0;
  const char** argv = nullptr;
  const bc_host* host = entry(&argc, &argv);
  for (int i = 0; i < argc; ++i) {
    switches->push_back(argv[i]);
  }
  // The library stays loaded for as long as the process runs: Chromium calls into it until exit.
  return host;
}

}  // namespace browser_core::platform
