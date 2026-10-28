#ifndef BROWSER_CORE_HISTORY_IMPORT_H_
#define BROWSER_CORE_HISTORY_IMPORT_H_

#include <stddef.h>

#include "browser_core/public/browser_core.h"

class Profile;

namespace browser_core {

// Adds another browser's |pages| to |profile|'s history, skipping each it has with a visit as
// late, then tells the app's history_added how many it added for |id|. False for a private profile
// or one without history.
bool AddImportedHistory(Profile* profile, bc_id id, const bc_history_page* pages, size_t count);

}  // namespace browser_core

#endif  // BROWSER_CORE_HISTORY_IMPORT_H_
