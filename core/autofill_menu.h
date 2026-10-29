#ifndef BROWSER_CORE_AUTOFILL_MENU_H_
#define BROWSER_CORE_AUTOFILL_MENU_H_

#include "browser_core/public/browser_core.h"

namespace content {
class WebContents;
}  // namespace content

namespace browser_core {

// Edit › AutoFill (bc_window_show_autofill): Chrome's dropdown under the field focused in a page,
// as the user would open it. Each tab remembers the form field its page has focused, as Chrome's
// autofill hears of it, so the menu needn't ask the page.

// Starts remembering |contents|'s focused field, as its tab is made.
void WatchFocusedField(content::WebContents* contents);

// Opens Chrome's dropdown for |kind|, a BC_AUTOFILL_ value, under the field focused in |tab|'s
// page, if the page has the keyboard and the tab isn't an agent's. Returns whether it did.
bool ShowAutofill(bc_id tab, int kind);

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTOFILL_MENU_H_
