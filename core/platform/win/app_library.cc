#include "browser_core/platform/app_library.h"

#include <windows.h>

#include "base/files/file_path.h"
#include "base/logging.h"
#include "base/path_service.h"
#include "base/strings/utf_string_conversions.h"

namespace browser_core::platform {

const bc_host* LoadApp(base::CommandLine::StringVector* switches) {
  base::FilePath dir;
  base::PathService::Get(base::DIR_EXE, &dir);
  HMODULE app = LoadLibraryW(dir.AppendASCII(BC_APP_LIBRARY).value().c_str());
  if (!app) {
    PLOG(ERROR) << "LoadLibrary";
    return nullptr;
  }
  auto entry = reinterpret_cast<bc_app_entry>(GetProcAddress(app, BC_APP_ENTRY));
  if (!entry) {
    return nullptr;
  }
  int argc = 0;
  const char** argv = nullptr;
  const bc_host* host = entry(&argc, &argv);
  for (int i = 1; i < argc; ++i) {
    switches->push_back(base::UTF8ToWide(argv[i]));
  }
  // The library stays loaded for as long as the process runs: Chromium calls into it until exit.
  return host;
}

}  // namespace browser_core::platform
