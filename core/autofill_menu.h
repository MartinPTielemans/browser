#ifndef BROWSER_CORE_AUTOFILL_MENU_H_
#define BROWSER_CORE_AUTOFILL_MENU_H_

#include "browser_core/public/browser_core.h"

namespace content {
class WebContents;
}

namespace browser_core {

// Edit › AutoFill's items: Chrome's dropdown at the form field that has the keyboard in a tab's
// page. Chrome's renderer tells its autofill manager of each focus change, and the core keeps the
// last, since the browser can't ask which field has it.
class AutofillMenu {
 public:
  // Follows the focus in |contents|'s frames from now on. The tab calls it as it takes a page.
  static void Watch(content::WebContents* contents);
  // See bc_tab_autofill.
  static bool Open(bc_id tab, int kind);
};

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTOFILL_MENU_H_
