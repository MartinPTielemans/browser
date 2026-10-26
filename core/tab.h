#ifndef BROWSER_CORE_TAB_H_
#define BROWSER_CORE_TAB_H_

#include <optional>
#include <string>

#include "base/callback_list.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "base/time/time.h"
#include "browser_core/public/browser_core.h"
#include "components/favicon/core/favicon_driver.h"
#include "components/favicon/core/favicon_driver_observer.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "third_party/skia/include/core/SkColor.h"
#include "ui/gfx/image/image.h"
#include "url/gurl.h"

namespace browser_core {

// A tab's id, and what the app hears about its page.
class Tab : public content::WebContentsObserver,
            public content::WebContentsUserData<Tab>,
            public favicon::FaviconDriverObserver {
 public:
  // |contents|'s id, given the first time it's asked for.
  static bc_id IdOf(content::WebContents* contents);
  static content::WebContents* FromId(bc_id id);
  // Whether |id| has its page, which it hasn't once it's unloaded, or put back and not yet shown.
  static bool Loaded(bc_id id);
  // The URL of the page |contents| is at, loaded or not, else of the one it's loading.
  static GURL PageUrl(content::WebContents* contents);

  Tab(const Tab&) = delete;
  Tab& operator=(const Tab&) = delete;
  ~Tab() override;

  // Lets go of the tab's page, keeping its history, as Chrome discards a tab to save memory. Its
  // page loads again once it's shown.
  void Unload();

  // For a tab put back without loading: tells the app its page's URL, history and title as they
  // were, and finds its favicon among the profile's.
  void Restored();

  // Tells the app the tab's state from a task of its own, so a burst of changes is one call, and
  // none comes during the app's call that made them.
  void StateChanged();

  // The name the user gave the tab, which it shows for its title, or empty for its page's.
  const std::string& name() const { return name_; }
  // Names the tab and tells the app its title from a task of its own. Empty gives it back its
  // page's title.
  void Rename(const std::string& name);
  // For a tab put back: its name, which Restored tells the app.
  void set_name(const std::string& name) { name_ = name; }
  // Its name, else its page's title.
  std::string Title();

  // Mutes or unmutes the tab, as Chrome's speaker button does.
  void SetMuted(bool muted);

  // The colour the tab's page shows until it draws its first frame, opaque. Each page view the tab
  // swaps in takes it too, which Chrome's views do on their own only off the Mac.
  void SetPageColor(SkColor color);

  // When a window last stopped showing the tab, else when it opened, for the auto-archive.
  base::Time last_seen() const { return last_seen_; }
  void set_last_seen(base::Time time) { last_seen_ = time; }

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
  void RenderFrameHostChanged(content::RenderFrameHost* old_host,
                              content::RenderFrameHost* new_host) override;
  void RenderViewReady() override;
  void TitleWasSet(content::NavigationEntry* entry) override;
  void DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) override;
  void DidUpdateAudioMutingState(bool muted) override;
  void DidGetUserInteraction(const blink::WebInputEvent& event) override;
  void WebContentsDestroyed() override;

  // favicon::FaviconDriverObserver:
  void OnFaviconUpdated(favicon::FaviconDriver* driver,
                        NotificationIconType type,
                        const GURL& icon_url,
                        bool icon_url_changed,
                        const gfx::Image& image) override;

  // Tells the app the tab's state from a task of its own, as StateChanged does, for a change the
  // session doesn't keep.
  void ReportStateSoon();
  void ReportState();
  // Within ReportState, if the favicon the tab shows changed.
  void ReportFavicon(bool loading);
  void ReportTitle();

  const bc_id id_;
  std::string name_;
  base::Time last_seen_ = base::Time::Now();
  std::optional<SkColor> page_color_;
  bool state_report_posted_ = false;
  // The favicon the app was last told of.
  gfx::Image favicon_;
  base::ScopedObservation<favicon::FaviconDriver, favicon::FaviconDriverObserver>
      favicon_observation_{this};
  base::CallbackListSubscription audible_subscription_;
  base::WeakPtrFactory<Tab> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_TAB_H_
