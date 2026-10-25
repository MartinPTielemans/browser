#ifndef BROWSER_CORE_CLEAR_SITE_H_
#define BROWSER_CORE_CLEAR_SITE_H_

#include "browser_core/public/browser_core.h"

namespace browser_core {

// Deletes the cookies, or the cache, that |tab|'s profile keeps for its page's site, then loads
// the page again. See bc_tab_clear_site.
void ClearSiteAndReload(bc_id tab, bool cookies);

}  // namespace browser_core

#endif  // BROWSER_CORE_CLEAR_SITE_H_
