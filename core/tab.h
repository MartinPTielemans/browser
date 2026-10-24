#ifndef BROWSER_CORE_TAB_H_
#define BROWSER_CORE_TAB_H_

#include "browser_core/public/browser_core.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"

namespace browser_core {

// A tab's id, and what the app hears about its page.
class Tab : public content::WebContentsObserver, public content::WebContentsUserData<Tab> {
 public:
  // |contents|'s id, given the first time it's asked for.
  static bc_id IdOf(content::WebContents* contents);
  static content::WebContents* FromId(bc_id id);

  Tab(const Tab&) = delete;
  Tab& operator=(const Tab&) = delete;
  ~Tab() override;

 private:
  friend class content::WebContentsUserData<Tab>;

  explicit Tab(content::WebContents* contents);

  // content::WebContentsObserver:
  void DidFinishLoad(content::RenderFrameHost* frame, const GURL& url) override;
  void TitleWasSet(content::NavigationEntry* entry) override;
  void DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) override;
  void WebContentsDestroyed() override;

  const bc_id id_;

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_TAB_H_
