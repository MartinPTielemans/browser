#ifndef BROWSER_CORE_SESSION_H_
#define BROWSER_CORE_SESSION_H_

#include "base/functional/callback_forward.h"

namespace browser_core {

class Window;

// The Spaces as the user left them: their names, their tabs with each tab's history, the tab each
// showed and the Space the window showed. The core saves them a while after they change, in the
// user data directory since Spaces aren't one profile's, and the next launch puts them back.

// Reads what the last launch saved, then runs |loaded|.
void LoadSession(base::OnceClosure loaded);

// Saves the Spaces soon, as something in them changed.
void SessionChanged();

// Saves the Spaces now and stops saving: the tabs closing go with their window or with the quit,
// and come back at the next launch.
void KeepSession();

// Writes a save still waiting, as Chrome shuts down.
void FlushSession();

// Puts the Spaces the last launch saved in |window|, which shows the Space it showed, unless there
// are Spaces already. Returns whether it did.
bool RestoreSession(Window& window);

}  // namespace browser_core

#endif  // BROWSER_CORE_SESSION_H_
