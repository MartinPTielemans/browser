#include "browser_core/platform/keyboard.h"

#include <windows.h>

#include "components/input/native_web_keyboard_event.h"
#include "third_party/blink/public/common/input/web_input_event.h"

namespace browser_core::platform {

std::string LatinKey(const input::NativeWebKeyboardEvent& event) {
  // Windows' layouts give a key that isn't Latin the virtual key of its place on the US layout,
  // such as VK_T for Russian's "е", and Chrome's shortcuts match by it.
  const int code = event.windows_key_code;
  const bool shift = event.GetModifiers() & blink::WebInputEvent::kShiftKey;
  if (code >= 'A' && code <= 'Z') {
    return std::string(1, static_cast<char>(shift ? code : code - 'A' + 'a'));
  }
  // The US layout's characters, unshifted and with ⇧, as Chromium names a page's key with ⇧.
  struct Key {
    int code;
    char typed;
    char shifted;
  };
  static constexpr Key kKeys[] = {
      {'0', '0', ')'},          {'1', '1', '!'},          {'2', '2', '@'},
      {'3', '3', '#'},          {'4', '4', '$'},          {'5', '5', '%'},
      {'6', '6', '^'},          {'7', '7', '&'},          {'8', '8', '*'},
      {'9', '9', '('},          {VK_OEM_1, ';', ':'},     {VK_OEM_PLUS, '=', '+'},
      {VK_OEM_COMMA, ',', '<'}, {VK_OEM_MINUS, '-', '_'}, {VK_OEM_PERIOD, '.', '>'},
      {VK_OEM_2, '/', '?'},     {VK_OEM_3, '`', '~'},     {VK_OEM_4, '[', '{'},
      {VK_OEM_5, '\\', '|'},    {VK_OEM_6, ']', '}'},     {VK_OEM_7, '\'', '"'},
  };
  for (const Key& key : kKeys) {
    if (key.code == code) {
      return std::string(1, shift ? key.shifted : key.typed);
    }
  }
  return std::string();
}

}  // namespace browser_core::platform
