#include "browser_core/platform/keyboard.h"

#include <xkbcommon/xkbcommon.h>

#include <memory>

#include "base/strings/string_util.h"
#include "browser_core/platform/linux/wayland.h"
#include "components/input/native_web_keyboard_event.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/events/event.h"
#include "ui/events/event_constants.h"
#include "ui/events/event_utils.h"
#include "ui/events/keycodes/dom/dom_code.h"
#include "ui/events/keycodes/dom/keycode_converter.h"
#include "ui/events/keycodes/keyboard_code_conversion.h"
#include "ui/events/keycodes/scoped_xkb.h"
#include "ui/events/ozone/layout/keyboard_layout_engine_manager.h"
#include "ui/events/ozone/layout/xkb/xkb_keyboard_layout_engine.h"
#include "ui/gfx/x/connection.h"
#include "ui/gfx/x/xproto.h"

namespace browser_core::platform {

namespace {

// Ozone's layout engine on Wayland keeps the compositor's keymap and the layout typing in its XKB
// state, which it doesn't expose, and its Lookup reads only the layout typing.
struct WaylandEngine : ui::XkbKeyboardLayoutEngine {
  static xkb_state* State() {
    auto* engine = static_cast<ui::XkbKeyboardLayoutEngine*>(
        ui::KeyboardLayoutEngineManager::GetKeyboardLayoutEngine());
    return engine ? (engine->*&WaylandEngine::xkb_state_).get() : nullptr;
  }
};

// The XKB layouts the user switches between, as "us,il" has two: how many, and the one typing.
// On Wayland they're the engine's keymap's, looked up in a state of our own, as the engine's is
// the one it types with; on X11 the server's, through the X connection, where the key's event
// carries the group typing.
struct Layouts {
  uint32_t count = 0;
  uint32_t current = 0;
  std::unique_ptr<xkb_state, ui::XkbStateDeleter> wayland;

  explicit operator bool() const { return count; }

  // The character |keycode| types on |layout|, with ⇧ or without.
  uint32_t Typed(uint32_t keycode, uint32_t layout, bool shift) const {
    if (wayland) {
      xkb_keymap* keymap = xkb_state_get_keymap(wayland.get());
      const xkb_mod_mask_t mods =
          shift ? 1u << xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_SHIFT) : 0;
      xkb_state_update_mask(wayland.get(), mods, 0, 0, 0, 0, layout);
      return xkb_state_key_get_utf32(wayland.get(), keycode);
    }
    // The group is a state's bits 13 and 14.
    const uint32_t state =
        (layout << 13) | (shift ? static_cast<uint32_t>(x11::KeyButMask::Shift) : 0);
    return xkb_keysym_to_utf32(
        x11::Connection::Get()->KeycodeToKeysym(static_cast<x11::KeyCode>(keycode), state));
  }
};

Layouts WaylandLayouts() {
  xkb_state* engine = WaylandEngine::State();
  if (!engine) {
    return {};
  }
  xkb_keymap* keymap = xkb_state_get_keymap(engine);
  // Every lookup the engine makes sets the layout typing as its state's.
  return {xkb_keymap_num_layouts(keymap),
          xkb_state_serialize_layout(engine, XKB_STATE_LAYOUT_EFFECTIVE),
          std::unique_ptr<xkb_state, ui::XkbStateDeleter>(xkb_state_new(keymap))};
}

Layouts X11Layouts(const ui::Event* event) {
  const ui::Event::Properties* properties = event ? event->properties() : nullptr;
  if (!properties) {
    return {};
  }
  auto group = properties->find(ui::kPropertyKeyboardGroup);
  if (group == properties->end() || group->second.empty()) {
    return {};
  }
  // X has four groups, wrapped onto those the keymap has.
  return {4, group->second[0], nullptr};
}

}  // namespace

std::string LatinKey(const input::NativeWebKeyboardEvent& event) {
  const auto code = static_cast<ui::DomCode>(event.dom_code);
  const bool shift = event.GetModifiers() & blink::WebInputEvent::kShiftKey;
  const Layouts layouts = OnWayland() ? WaylandLayouts() : X11Layouts(event.os_event);
  if (!layouts) {
    return std::string();
  }
  // A layout is Latin when a key in the letters' places types a Latin letter. AZERTY and Dvorak
  // are, and keep their own characters: AZERTY's Ctrl on the W key is Ctrl+Z.
  auto latin = [&](uint32_t layout) {
    for (auto letter = static_cast<uint32_t>(ui::DomCode::US_A);
         letter <= static_cast<uint32_t>(ui::DomCode::US_Z); ++letter) {
      const uint32_t keycode =
          ui::KeycodeConverter::DomCodeToXkbKeycode(static_cast<ui::DomCode>(letter));
      if (base::IsAsciiAlpha(layouts.Typed(keycode, layout, false))) {
        return true;
      }
    }
    return false;
  };
  if (latin(layouts.current)) {
    return std::string();
  }
  // The first Latin layout the user switches to, else the key's place on the US layout.
  uint32_t typed = ui::DomCodeToUsLayoutCharacter(code, shift ? ui::EF_SHIFT_DOWN : ui::EF_NONE);
  for (uint32_t layout = 0; layout < layouts.count; ++layout) {
    if (latin(layout)) {
      typed = layouts.Typed(ui::KeycodeConverter::DomCodeToXkbKeycode(code), layout, shift);
      break;
    }
  }
  if (typed <= ' ' || typed >= 0x7f) {
    return std::string();
  }
  return std::string(1, static_cast<char>(typed));
}

}  // namespace browser_core::platform
