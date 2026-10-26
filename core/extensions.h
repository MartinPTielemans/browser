#ifndef BROWSER_CORE_EXTENSIONS_H_
#define BROWSER_CORE_EXTENSIONS_H_

#include <string>

#include "browser_core/public/browser_core.h"

class BrowserWindowInterface;
class Profile;

namespace content {
class WebContents;
}

namespace views {
class View;
}

namespace browser_core {

// Tells the app of |profile|'s extensions from now on, those Chrome's extensions page lists. Then
// extensions_changed.
void ObserveExtensions(Profile* profile);

// Then extensions_changed again, for each profile observed.
void ReportExtensions();

// Runs extension |id|'s action on |contents|, the tab |window| shows in |browser|, as a click on its
// toolbar button does in Chrome: it's granted the tab, and its popup opens from |anchor| or its
// action.onClicked fires. Returns false if the extension isn't turned on in the tab's profile.
bool RunExtension(bc_id window,
                  BrowserWindowInterface* browser,
                  content::WebContents* contents,
                  const std::string& id,
                  views::View* anchor);

}  // namespace browser_core

#endif  // BROWSER_CORE_EXTENSIONS_H_
