#ifndef BROWSER_CORE_SESSION_H_
#define BROWSER_CORE_SESSION_H_

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "base/functional/callback_forward.h"
#include "base/time/time.h"
#include "browser_core/public/browser_core.h"
#include "components/sessions/core/serialized_navigation_entry.h"
#include "ui/gfx/geometry/rect.h"
#include "url/gurl.h"

class Profile;

namespace content {
class WebContents;
}

namespace browser_core {

class Space;
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
  // The emoji the user gave it to show in place of its favicon, if any.
  std::string icon;
  // When a window last stopped showing a Today tab, if it was saved with one.
  base::Time last_seen;
  // Whether it's in a split view with the tab saved above it, and whether that split is stacked.
  bool split = false;
  bool stacked = false;
  // Its share of its split view, or 0 while the split's shares are even.
  double share = 0;
  // How many folders it's in, among the pinned tabs.
  int depth = 0;
  // Whether it was muted, for a tab closed and reopened. The session file doesn't keep it.
  bool muted = false;
  // A favorite's profile's directory. A Space's tabs are in the Space's.
  std::string profile;
};

// |contents|'s history, with the page it's loading, or waiting to load, if it has no page yet.
SavedTab SaveTab(content::WebContents& contents);

// A folder of pinned tabs, as the session file keeps it.
struct SavedFolder {
  std::string name;
  bool open = true;
  // How many folders it's in.
  int depth = 0;
  // The pinned tab saved under it, counted among them.
  size_t at = 0;
};

// A Space, as the session file keeps it.
struct SavedSpace {
  std::string name;
  std::optional<uint32_t> color;
  // The emoji the user gave it, if any.
  std::string icon;
  // Its profile's directory, or empty for the profile the core started with.
  std::string profile;
  // Each top to bottom.
  std::vector<SavedTab> pinned;
  std::vector<SavedFolder> folders;
  std::vector<SavedTab> today;
  // The tab the window showed there, counting the pinned tabs then the Today tabs, or -1.
  int shown = -1;
  // The favorite the window showed there instead, or -1.
  int shown_favorite = -1;
};

// |space|, where a window shows |shown|, without its shown favorite. Puts the Today tabs it saved
// in |today| and every tab it saved, by its place among them, in |tabs|, if they aren't null.
SavedSpace SaveSpace(const Space& space,
                     bc_id shown,
                     std::vector<bc_id>* today = nullptr,
                     std::vector<bc_id>* tabs = nullptr);

// Makes |saved| a Space, its tabs unloaded in |window| and |profile|, with the id |id|, a deleted
// Space's, if it isn't 0. The window shows its shown tab there. Puts each tab's id in |ids|, by its
// place among the pinned then Today tabs saved, or 0 for one it couldn't make, if |ids| isn't
// null. Returns it.
Space* RestoreSpace(Window& window,
                    Profile* profile,
                    const SavedSpace& saved,
                    bc_id id = 0,
                    std::vector<bc_id>* ids = nullptr);

// A window, as the session file keeps it. What it shows is its own; the Spaces are shared.
struct SavedWindow {
  // On the screen, as it was out of full screen.
  gfx::Rect bounds;
  // The Space it showed, by its place among the Spaces.
  int shown = 0;
  // Its docked sidebar's width, or 0 for none.
  double sidebar_width = 0;
  // For each Space, the tab it showed there as SavedSpace::shown counts it, and the favorite it
  // showed there instead, each -1 for none.
  std::vector<std::pair<int, int>> tabs;
};

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

// The windows the last launch saved, the most recently active first. None if nothing was saved, or
// if a build without windows saved the Spaces, which come back in one window.
const std::vector<SavedWindow>& SavedWindows();

// What the user chose in the app's Settings. The core keeps it with the Spaces, and reads it back
// with them, before the app opens its window.
struct Settings {
  // How long a Today tab no window shows waits before it goes in the archive.
  base::TimeDelta archive_after = base::Hours(12);
  // Whether the app taps the trackpad at its detents.
  bool haptics = true;
  // Whether the app asks before the user's Quit quits.
  bool warn_before_quitting = true;
  // Whether the app warns before moving a tab to another profile's Space, which it does once.
  bool warn_moving_profiles = true;
};

// The user's settings. Call SessionChanged after changing them.
Settings& UserSettings();

// Puts the favorites and Spaces the last launch saved in |window|, which shows the Space it
// showed, unless there are Spaces already. Returns whether it did. That's |index| 0: a later one
// gives |window| what the |index|-th of SavedWindows showed, once index 0 has put the Spaces back.
bool RestoreSession(Window& window, size_t index);

}  // namespace browser_core

#endif  // BROWSER_CORE_SESSION_H_
