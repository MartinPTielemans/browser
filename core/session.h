#ifndef BROWSER_CORE_SESSION_H_
#define BROWSER_CORE_SESSION_H_

#include <string>
#include <vector>

#include "base/functional/callback_forward.h"
#include "base/time/time.h"
#include "components/sessions/core/serialized_navigation_entry.h"
#include "url/gurl.h"

namespace content {
class NavigationController;
}

namespace browser_core {

class Window;

// A tab's history, as the session file keeps it.
struct SavedTab {
  // Up to Chrome's count of navigations each side of the current one, as its sessions keep.
  std::vector<sessions::SerializedNavigationEntry> history;
  int current = 0;
  // A pinned tab's pinned URL, or a favorite's URL.
  GURL pinned_url;
  // The name the user gave it, if any.
  std::string name;
  // When a window last stopped showing a Today tab, if it was saved with one.
  base::Time last_seen;
  // Whether it's in a split view with the tab saved above it.
  bool split = false;
  // How many folders it's in, among the pinned tabs.
  int depth = 0;
};

// |history|, with the page it's loading if it has no page yet.
SavedTab SaveTab(content::NavigationController& history);

// The Spaces as the user left them: their names, their tabs with each tab's history, the tab each
// showed and the Space the window showed, and the favorites. The core saves them a while after
// they change, in the user data directory since Spaces aren't one profile's, and the next launch
// puts them back.

// Reads what the last launch saved, then runs |loaded|.
void LoadSession(base::OnceClosure loaded);

// Saves the Spaces soon, as something in them changed.
void SessionChanged();

// Saves the Spaces now and stops saving: the tabs closing go with their window or with the quit,
// and come back at the next launch.
void KeepSession();

// Writes a save still waiting, as Chrome shuts down.
void FlushSession();

// The docked sidebar's width the last launch saved for its window, or 0.
double SavedSidebarWidth();

// What the user chose in the app's Settings. The core keeps it with the Spaces, and reads it back
// with them, before the app opens its window.
struct Settings {
  // How long a Today tab no window shows waits before it goes in the archive.
  base::TimeDelta archive_after = base::Hours(12);
  // Whether the app taps the trackpad at its detents.
  bool haptics = true;
};

// The user's settings. Call SessionChanged after changing them.
Settings& UserSettings();

// Puts the favorites and Spaces the last launch saved in |window|, which shows the Space it
// showed, unless there are Spaces already. Returns whether it did.
bool RestoreSession(Window& window);

}  // namespace browser_core

#endif  // BROWSER_CORE_SESSION_H_
