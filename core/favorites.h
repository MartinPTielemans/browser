#ifndef BROWSER_CORE_FAVORITES_H_
#define BROWSER_CORE_FAVORITES_H_

#include <string>
#include <vector>

#include "browser_core/public/browser_core.h"
#include "url/gurl.h"

namespace browser_core {

// The favorites: tabs in no Space, which every Space of their profile shows above its own, as Arc
// shows a profile's favorites in each of its Spaces. Each keeps the URL it was made a favorite at,
// which Back to Favorite URL goes to.
class Favorites {
 public:
  Favorites() = delete;

  // Every profile's, each profile's left to right, then top to bottom.
  static const std::vector<bc_id>& All();
  // The favorites of the profile in the directory |profile|.
  static std::vector<bc_id> Of(const std::string& profile);
  // The directory of the favorite |tab|'s profile, or empty if it isn't a favorite.
  static std::string ProfileOf(bc_id tab);
  static bool Has(bc_id tab);
  // Empty if |tab| isn't a favorite.
  static GURL Url(bc_id tab);

  // Makes |tab|, a tab in no Space, the favorite at |index| of its profile's, else the last, at
  // |url|.
  static void Add(bc_id tab, GURL url, size_t index);
  // Moves the favorite |tab| to |index| of its profile's favorites without it, else to the end.
  static void Move(bc_id tab, size_t index);
  static void Remove(bc_id tab);
};

}  // namespace browser_core

#endif  // BROWSER_CORE_FAVORITES_H_
