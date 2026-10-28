#ifndef BROWSER_CORE_IDLE_UNLOAD_H_
#define BROWSER_CORE_IDLE_UNLOAD_H_

#include "base/time/time.h"

namespace browser_core {

// Gives back the memory of pages the user hasn't looked at in a while (bc_unload_idle_tabs): a tab
// no window has shown for 2 hours lets go of its page (Tab::Unload), and gets it back once it's
// shown. No timer wakes an idle browser for it. It's checked as a window shows another tab or
// becomes key, which is also how the user comes back to the app, and as the system runs short of
// memory, when the tabs unseen longest go first, whatever their age.

// Checks soon, once the work at hand is done.
void UnloadIdleTabsSoon();

// For check.sh: makes every tab look |by| longer unseen, for the next check, and from now has each
// check log "browser: idle tabs checked" once it's done.
void AgeTabsForTesting(base::TimeDelta by);

}  // namespace browser_core

#endif  // BROWSER_CORE_IDLE_UNLOAD_H_
