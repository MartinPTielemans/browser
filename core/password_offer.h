#ifndef BROWSER_CORE_PASSWORD_OFFER_H_
#define BROWSER_CORE_PASSWORD_OFFER_H_

#include "browser_core/public/browser_core.h"

namespace content {
class WebContents;
}

namespace browser_core {

// Chrome's offer to save or update a submitted login's password, which the app shows in place of
// Chrome's bubble (tab_offer_changed). Chrome's ManagePasswordsUIController asks to show it through
// the tab's BubbleManager, which shows it only while the tab is, and hides it when another tab is
// shown or the tab closes, without bringing it back. The password stays in the core: the app gets
// the username, the password's length and the site.
class PasswordOffer {
 public:
  // g_browser_core_password_prompt.
  static void Prompt(content::WebContents* contents, bool show);
  // See bc_tab_answer_offer.
  static void Answer(bc_id tab, int answer, const char* username);

 private:
  // Closes the tab's offer, as Chrome's widget closing does.
  static void Drop(bc_id tab);
  // Tells the app the tab's offer, or that it has none, from a task of its own: Chrome shows and
  // hides offers inside the app's calls, such as the one that shows a tab.
  static void Changed(bc_id tab);
  static void Report(bc_id tab);
};

}  // namespace browser_core

#endif  // BROWSER_CORE_PASSWORD_OFFER_H_
