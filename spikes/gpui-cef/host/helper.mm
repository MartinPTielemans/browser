// Chromium's GPU, renderer and utility processes. On macOS each runs from its own helper app
// inside the main bundle's Frameworks directory.
#include "include/cef_app.h"
#include "include/wrapper/cef_library_loader.h"

int main(int argc, char* argv[]) {
  CefScopedLibraryLoader library;
  if (!library.LoadInHelper()) {
    return 1;
  }
  return CefExecuteProcess(CefMainArgs(argc, argv), nullptr, nullptr);
}
