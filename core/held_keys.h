#ifndef BROWSER_CORE_HELD_KEYS_H_
#define BROWSER_CORE_HELD_KEYS_H_

#include <cstddef>

#include "base/auto_reset.h"
#include "browser_core/public/browser_core.h"

namespace input {
struct NativeWebKeyboardEvent;
}

// The keys typed in a page after a shortcut that opens a field of the app's, held from the page
// until the field has the keyboard (bc_hold_keys, bc_give_held_keys), or until the page's answer to
// the shortcut says no field opens.
namespace browser_core::held_keys {

// The key going down in a page that the app is asked about.
struct Asking {
  bc_id tab = 0;
  const input::NativeWebKeyboardEvent* key = nullptr;
};

// The app is asked about |key| going down in |tab|'s page until the reset returned goes, which is
// the key bc_hold_keys holds the keys after.
[[nodiscard]] base::AutoReset<Asking> AskingAbout(bc_id tab,
                                                  const input::NativeWebKeyboardEvent& key);

// Holds the keys typed from now in the page the app is being asked about, until the page has
// answered the key and no field opens for it, or the field has the keyboard. With |in_core| the
// core holds them itself, else the host does, and is told when no field takes them.
void Hold(bool in_core);

// Whether |event|, going to |tab|'s page, is held from it. Then the page doesn't get it.
bool Holds(bc_id tab, const input::NativeWebKeyboardEvent& event);

// The page didn't keep |key|, and the app, asked about it then, |used| it or not.
void Answered(bc_id tab, const input::NativeWebKeyboardEvent& key, bool used);

// Ends the hold, giving the keys held, if any, to |take|.
void Give(void (*take)(void* context, const bc_key* keys, size_t count), void* context);

}  // namespace browser_core::held_keys

#endif  // BROWSER_CORE_HELD_KEYS_H_
