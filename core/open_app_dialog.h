#ifndef BROWSER_CORE_OPEN_APP_DIALOG_H_
#define BROWSER_CORE_OPEN_APP_DIALOG_H_

#include <optional>
#include <string>

#include "base/memory/weak_ptr.h"
#include "browser_core/page_dialog.h"
#include "content/public/browser/weak_document_ptr.h"
#include "content/public/browser/web_contents_observer.h"
#include "url/gurl.h"
#include "url/origin.h"

namespace browser_core {

// Whether a page may open another app for a link of its scheme, such as Mail for mailto:, which the
// app asks with its dialog over the tab's card (PageDialog) in place of Chrome's views dialog
// (hooks.h). Chrome's ExternalProtocolHandler still decides when it's asked, and keeps the user's
// "Always allow" in the profile's prefs. As Chrome's, it goes once its page goes to another site.
class OpenAppDialog : public content::WebContentsObserver {
 public:
  // g_browser_core_external_protocol_dialog. It deletes itself once answered or gone.
  static void Ask(const GURL& url,
                  content::WebContents* web_contents,
                  const std::optional<url::Origin>& initiating_origin,
                  content::WeakDocumentPtr initiator_document,
                  const std::u16string& program_name);

  // Whether |contents|'s page may open another app at all: none in an agent's Space may, as
  // agents never act outside the browser, so neither may the user's clicks there.
  static bool MayOpen(content::WebContents* contents);

  OpenAppDialog(const OpenAppDialog&) = delete;
  OpenAppDialog& operator=(const OpenAppDialog&) = delete;

 private:
  OpenAppDialog(content::WebContents* web_contents,
                const GURL& url,
                const std::optional<url::Origin>& initiating_origin,
                content::WeakDocumentPtr initiator_document);
  ~OpenAppDialog() override;

  void Answered(const PageDialog::Reply& reply);

  // content::WebContentsObserver:
  void DidFinishNavigation(content::NavigationHandle* navigation) override;
  void WebContentsDestroyed() override;

  const GURL url_;
  const std::optional<url::Origin> initiating_origin_;
  const content::WeakDocumentPtr initiator_document_;
  bc_id id_ = 0;
  base::WeakPtrFactory<OpenAppDialog> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_OPEN_APP_DIALOG_H_
