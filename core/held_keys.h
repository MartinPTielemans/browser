#ifndef BROWSER_CORE_HELD_KEYS_H_
#define BROWSER_CORE_HELD_KEYS_H_

#include <cstddef>

#include "base/auto_reset.h"
#include "browser_core/public/browser_core.h"

namespace input {
struct NativeWebKeyboardEvent;
}

// The keys typed in a page after a shortcut that opens a field of the app's, held from the page
// until the field has the keyboard (bc_hold_keys, bc_give_held_keys).
namespace browser_core::held_keys {

// The app is asked about a key going down in |tab|'s page until the reset returned goes, which is
// the page bc_hold_keys holds the keys of.
[[nodiscard]] base::AutoReset<bc_id> AskingAbout(bc_id tab);

// Holds the keys typed from now in the page the app is being asked about.
void Hold();

// Whether |event|, going to |tab|'s page, is held from it. Then the page doesn't get it.
bool Holds(bc_id tab, const input::NativeWebKeyboardEvent& event);

// Ends the hold, giving the keys held, if any, to |take|.
void Give(void (*take)(void* context, const bc_key* keys, size_t count), void* context);

}  // namespace browser_core::held_keys

#endif  // BROWSER_CORE_HELD_KEYS_H_
