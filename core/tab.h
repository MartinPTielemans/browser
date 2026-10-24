#ifndef BROWSER_CORE_TAB_H_
#define BROWSER_CORE_TAB_H_

#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "browser_core/public/browser_core.h"
#include "components/favicon/core/favicon_driver.h"
#include "components/favicon/core/favicon_driver_observer.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "ui/gfx/image/image.h"

namespace browser_core {

// A tab's id, and what the app hears about its page.
class Tab : public content::WebContentsObserver,
            public content::WebContentsUserData<Tab>,
            public favicon::FaviconDriverObserver {
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
  void DidStartNavigation(content::NavigationHandle* navigation) override;
  void DidFinishNavigation(content::NavigationHandle* navigation) override;
  void DidStartLoading() override;
  void DidStopLoading() override;
  void NavigationEntriesDeleted() override;
  void DidFinishLoad(content::RenderFrameHost* frame, const GURL& url) override;
  void TitleWasSet(content::NavigationEntry* entry) override;
  void DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) override;
  void WebContentsDestroyed() override;

  // favicon::FaviconDriverObserver:
  void OnFaviconUpdated(favicon::FaviconDriver* driver,
                        NotificationIconType type,
                        const GURL& icon_url,
                        bool icon_url_changed,
                        const gfx::Image& image) override;

  // Tells the app the tab's state from a task of its own, so a burst of changes is one call, and
  // none comes during the app's call that made them.
  void StateChanged();
  void ReportState();
  // Within ReportState, if the favicon the tab shows changed.
  void ReportFavicon(bool loading);

  const bc_id id_;
  bool state_report_posted_ = false;
  // The favicon the app was last told of.
  gfx::Image favicon_;
  base::ScopedObservation<favicon::FaviconDriver, favicon::FaviconDriverObserver>
      favicon_observation_{this};
  base::WeakPtrFactory<Tab> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_TAB_H_
