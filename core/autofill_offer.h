#ifndef BROWSER_CORE_AUTOFILL_OFFER_H_
#define BROWSER_CORE_AUTOFILL_OFFER_H_

#include <memory>

#include "browser_core/public/browser_core.h"

namespace autofill {
class AutofillBubbleBase;
class AutofillBubbleHandler;
class SaveAddressBubbleController;
class SaveCardBubbleController;
}  // namespace autofill
namespace content {
class WebContents;
}

namespace browser_core {

// Chrome's offers to save the address or the card a form submitted, which the app shows in the
// password offer's place (tab_offer_changed), in Chrome's words. Chrome's controllers ask the
// window's autofill bubble handler to show them, through the tab's BubbleManager, which shows one
// bubble at a time and hides it when another tab is shown, as it does a password's.
class AutofillOffer {
 public:
  // The core's windows' autofill bubble handler: the bubble the controller holds while the app
  // shows the offer.
  static autofill::AutofillBubbleBase* SaveAddress(
      content::WebContents* contents,
      std::unique_ptr<autofill::SaveAddressBubbleController> controller);
  static autofill::AutofillBubbleBase* SaveCard(content::WebContents* contents,
                                                autofill::SaveCardBubbleController* controller);
  // The autofill bubble handler of the core's window holding |contents|, which its stand-in
  // registered on the window's Browser (g_browser_core_autofill_bubble_handler). Null for a tab of
  // no window's.
  static autofill::AutofillBubbleHandler* HandlerFor(content::WebContents* contents);
  // Whether |tab| has one open.
  static bool Has(bc_id tab);
  // See bc_tab_answer_offer. Returns false if |tab| has none.
  static bool Answer(bc_id tab, int answer);
};

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTOFILL_OFFER_H_
