#ifndef BROWSER_CORE_TAB_H_
#define BROWSER_CORE_TAB_H_

#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "base/callback_list.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "base/task/cancelable_task_tracker.h"
#include "base/time/time.h"
#include "browser_core/public/browser_core.h"
#include "browser_core/session.h"
#include "components/favicon/core/favicon_driver.h"
#include "components/favicon/core/favicon_driver_observer.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/web_contents_observer.h"
#include "third_party/skia/include/core/SkColor.h"
#include "ui/gfx/image/image.h"
#include "url/gurl.h"

class BrowserWindowInterface;
class Profile;

namespace browser_core {

class Window;

// A tab's id, and what the app hears about it. A tab may have no page: one put back at launch and
// not yet shown, or one unloaded. It keeps its history then, and what its row shows, and gets its
// page back once a window shows it (Wake), as Chrome loads a restored tab once it's shown.
class Tab : public content::WebContentsObserver, public favicon::FaviconDriverObserver {
 public:
  // |contents|'s id, given the first time it's asked for.
  static bc_id IdOf(content::WebContents* contents);
  // |contents|'s tab, or null if nothing asked for its id.
  static Tab* FromWebContents(content::WebContents* contents);
  // |id|'s tab, with its page or without, or null.
  static Tab* Get(bc_id id);
  // |id|'s page, or null, as for a tab without one.
  static content::WebContents* FromId(bc_id id);
  // Every tab, with its page or without.
  static std::vector<Tab*> All();
  // Whether |id| has its page loaded, which it hasn't without one, nor once unloaded in place by
  // Chrome, nor while opened behind and not yet shown.
  static bool Loaded(bc_id id);
  // The URL of the page |contents| is at, loaded or not, else of the one it's loading.
  static GURL PageUrl(content::WebContents* contents);
  // The hook for Chrome's status bubble (hooks.h): |source|'s page has the pointer on the link
  // |url|, or on none and it's empty. Tells the app, if |source| is a tab of one of the core's
  // windows.
  static void LinkTargetChanged(BrowserWindowInterface* browser,
                                content::WebContents* source,
                                const GURL& url);

  // A tab without its page, in |profile| and in no Space, as |saved| keeps it: its history, name,
  // icon, mute and when it was last seen. Tells the app its state, title and the favicon the
  // profile has for its URL. Returns its id, or 0 without history.
  static bc_id Restore(Profile* profile, const SavedTab& saved);

  Tab(const Tab&) = delete;
  Tab& operator=(const Tab&) = delete;
  ~Tab() override;

  bc_id id() const { return id_; }
  // Its profile, or null once that's gone.
  Profile* profile() const { return profile_.get(); }
  // The URL of the page it's at, loaded or not, else of the one it's loading.
  GURL Url() const;
  // Its history, with its name, icon and mute, as the session keeps it.
  SavedTab Save() const;

  // Lets go of the tab's page, keeping its history, name and icon, and its place: it's a tab without
  // its page (Wake). Chrome's tab strip deletes the page, so none of its unload handlers run.
  void Unload();
  // Whether Unload is deleting its page.
  bool unloading() const { return unloading_; }
  // For a tab without its page: gives it one in |window|'s Browser for its profile, at its history,
  // not shown, which loads once it is. Returns it, else null, and the tab stays without.
  content::WebContents* Wake(Window& window);
  // Closes a tab without its page, as one closes with its page: out of its Space or the favorites,
  // and the app hears it closed. Deletes it.
  void Close();

  // For a copy of a page with its history, as a duplicate: tells the app its page's URL, history
  // and title, and finds its favicon among the profile's, as the page doesn't load until shown.
  void Restored();

  // For a new tab opened behind, as by ⌘-click on a link: |load| waits until the tab is first
  // shown, and runs then as Chrome would have run it at once. Until then the tab has no page, and
  // the app hears of the URL and the site's favicon among the profile's.
  void LoadWhenShown(content::NavigationController::LoadURLParams load);
  // The load waiting for the tab to be shown, else null.
  const content::NavigationController::LoadURLParams* waiting() const { return waiting_.get(); }

  // Tells the app the tab's state from a task of its own, so a burst of changes is one call, and
  // none comes during the app's call that made them.
  void StateChanged();

  // The name the user gave the tab, which it shows for its title, or empty for its page's.
  const std::string& name() const { return name_; }
  // Names the tab and tells the app its title from a task of its own. Empty gives it back its
  // page's title.
  void Rename(const std::string& name);
  // For a copy: its name, which Restored tells the app.
  void set_name(const std::string& name) { name_ = name; }
  // Its name, else its page's title.
  std::string Title();
  // The emoji the user gave the tab to show in place of its page's favicon, or empty.
  const std::string& icon() const { return icon_; }
  // Gives the tab |icon|, and tells the app with its state. Empty gives it back its favicon.
  void SetIcon(const std::string& icon);
  // For a copy: its icon, which its first state report tells the app.
  void set_icon(const std::string& icon) { icon_ = icon; }

  // Mutes or unmutes the tab, as Chrome's speaker button does.
  void SetMuted(bool muted);
  bool muted() const;

  // The colour the tab's page shows until it draws its first frame, opaque. Each page view the tab
  // swaps in takes it too, which Chrome's views do on their own only off the Mac.
  void SetPageColor(SkColor color);

  // The modifier keys held at the user's last click or key in the page, as blink::WebInputEvent's
  // flags: what a link it opens was opened with.
  int input_modifiers() const { return input_modifiers_; }

  // When a window last stopped showing the tab, else when it opened, for the auto-archive and for
  // unloading tabs unseen a while.
  base::Time last_seen() const { return last_seen_; }
  void set_last_seen(base::Time time) { last_seen_ = time; }

 private:
  Tab(bc_id id, Profile* profile);

  // Makes |contents| the tab's page.
  void Attach(content::WebContents* contents);
  // Tells the app the tab closed, as its page did or as Close closes it.
  void ReportClosed();
  // Finds the favicon the profile has for |url|, for a tab whose page hasn't loaded, as Chrome's
  // favicon driver finds one only for a page that has committed.
  void FindFavicon(const GURL& url);

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
  void PrimaryMainFrameRenderProcessGone(base::TerminationStatus status) override;
  void TitleWasSet(content::NavigationEntry* entry) override;
  void DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) override;
  void DidUpdateAudioMutingState(bool muted) override;
  void DidGetUserInteraction(const blink::WebInputEvent& event) override;
  void OnVisibilityChanged(content::Visibility visibility) override;
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
  void LoadWaiting();
  // Tells the app the link the pointer is on, if it's another than it was last told of.
  void ReportLinkTarget(const GURL& url);

  const bc_id id_;
  base::WeakPtr<Profile> profile_;
  // Without its page, its history and mute.
  std::optional<SavedTab> unloaded_;
  bool unloading_ = false;
  std::string name_;
  std::string icon_;
  base::Time last_seen_ = base::Time::Now();
  std::optional<SkColor> page_color_;
  bool state_report_posted_ = false;
  int input_modifiers_ = 0;
  // Whether the app was last told the page is full screen.
  bool fullscreen_ = false;
  // Whether the app was last told the pointer is on a link.
  bool link_target_ = false;
  // The favicon the app was last told of.
  gfx::Image favicon_;
  base::ScopedObservation<favicon::FaviconDriver, favicon::FaviconDriverObserver>
      favicon_observation_{this};
  base::CallbackListSubscription audible_subscription_;
  std::unique_ptr<content::NavigationController::LoadURLParams> waiting_;
  base::CancelableTaskTracker favicon_tracker_;
  base::WeakPtrFactory<Tab> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_TAB_H_
