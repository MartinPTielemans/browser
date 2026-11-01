#include "testing/gtest/include/gtest/gtest.h"
#include "ui/events/ozone/layout/xkb/xkb_evdev_codes.h"
#include "ui/events/ozone/layout/xkb/xkb_keyboard_layout_engine.h"

namespace browser_core {
namespace {

// A Wayland seat's keyboard with no keymap, as fcitx5's virtual one can be, still sends its
// modifiers, which once crashed the engine (#1138, patches/ours/chromium-xkb-no-keymap.patch).
TEST(XkbNoKeymapTest, ModifiersWithNoKeymapAreNone) {
  ui::XkbEvdevCodes codes;
  ui::XkbKeyboardLayoutEngine engine(codes);
  EXPECT_EQ(engine.UpdateModifiers(/*depressed=*/1, /*latched=*/0, /*locked=*/0, /*group=*/0), 0);
}

}  // namespace
}  // namespace browser_core
