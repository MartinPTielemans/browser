#include "browser_core/platform/app_library.h"

#include <windows.h>

#include "base/files/file_path.h"
#include "base/path_service.h"

namespace browser_core::platform {

const bc_host* LoadApp(const std::vector<std::string>& given,
                       std::vector<std::string>* switches) {
  base::FilePath dir;
  base::PathService::Get(base::DIR_EXE, &dir);
  HMODULE app = LoadLibraryW(dir.Append(BC_APP_LIBRARY).value().c_str());
  auto entry = app ? reinterpret_cast<bc_app_entry>(GetProcAddress(app, BC_APP_ENTRY)) : nullptr;
  if (!entry) {
    return nullptr;
  }
  std::vector<const char*> given_argv;
  for (const std::string& arg : given) {
    given_argv.push_back(arg.c_str());
  }
  int argc = static_cast<int>(given_argv.size());
  const char** argv = given_argv.data();
  const bc_host* host = entry(&argc, &argv);
  for (int i = 0; i < argc; ++i) {
    switches->push_back(argv[i]);
  }
  return host;
}

}  // namespace browser_core::platform
