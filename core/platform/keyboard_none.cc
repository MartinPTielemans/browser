#include "browser_core/platform/keyboard.h"

namespace browser_core::platform {

// On Linux, a shortcut's key comes from the layout as it types.
std::string LatinKey(const input::NativeWebKeyboardEvent&) {
  return std::string();
}

}  // namespace browser_core::platform
