#ifndef BROWSER_CORE_FAVORITES_H_
#define BROWSER_CORE_FAVORITES_H_

#include <vector>

#include "browser_core/public/browser_core.h"
#include "url/gurl.h"

namespace browser_core {

// The favorites: tabs in no Space, which every Space shows above its own, as Arc shows a profile's
// favorites in each of its Spaces. There's one profile for now, so they're the process's. Each
// keeps the URL it was made a favorite at, which Back to Favorite URL goes to.
class Favorites {
 public:
  Favorites() = delete;

  // Left to right, then top to bottom.
  static const std::vector<bc_id>& All();
  static bool Has(bc_id tab);
  // Empty if |tab| isn't a favorite.
  static GURL Url(bc_id tab);

  // Makes |tab|, a tab in no Space, the last favorite, at |url|.
  static void Add(bc_id tab, GURL url);
  static void Remove(bc_id tab);
};

}  // namespace browser_core

#endif  // BROWSER_CORE_FAVORITES_H_
