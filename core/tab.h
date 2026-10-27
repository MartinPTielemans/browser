#ifndef BROWSER_CORE_TAB_H_
#define BROWSER_CORE_TAB_H_

#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "base/callback_list.h"
#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "base/task/cancelable_task_tracker.h"
#include "base/time/time.h"
#include "browser_core/platform/system_page.h"
#include "browser_core/public/browser_core.h"
#include "components/favicon/core/favicon_driver.h"
#include "components/favicon/core/favicon_driver_observer.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "third_party/skia/include/core/SkColor.h"
#include "ui/gfx/image/image.h"
#include "ui/gfx/native_ui_types.h"
#include "url/gurl.h"

class BrowserWindowInterface;
namespace content {
class RenderWidgetHost;
}

namespace browser_core {

// A tab's id, and what the app hears about its page.
class Tab : public content::WebContentsObserver,
            public content::WebContentsUserData<Tab>,
            public favicon::FaviconDriverObserver,
            public platform::SystemPage::Delegate {
 public:
  // |contents|'s id, given the first time it's asked for.
  static bc_id IdOf(content::WebContents* contents);
  static content::WebContents* FromId(bc_id id);
  // Whether |id| has its page, which it hasn't once it's unloaded, or put back or opened behind and
  // not yet shown.
  static bool Loaded(bc_id id);
  // The URL of the page |contents| is at, loaded or not, else of the one it's loading.
  static GURL PageUrl(content::WebContents* contents);
  // The hook for Chrome's status bubble (hooks.h): |source|'s page has the pointer on the link
  // |url|, or on none and it's empty. Tells the app, if |source| is a tab of one of the core's
  // windows.
  static void LinkTargetChanged(BrowserWindowInterface* browser,
                                content::WebContents* source,
                                const GURL& url);
  // Every tab's page.
  static std::vector<content::WebContents*> All();
  // The view a window shows for |contents|'s page: its system page's, else its own.
  static gfx::NativeView PageViewOf(content::WebContents* contents);
  // Gives |contents|'s page the keyboard.
  static void FocusPage(content::WebContents* contents);
  // Asks the app, through |callback|, about a key going down in |tab|'s page, whose focused element
  // takes text if |editing|. Returns whether the app used it.
  static bool AskAppAboutKey(bool (*callback)(void*, bc_id, const bc_key*),
                             bc_id tab,
                             const input::NativeWebKeyboardEvent& event,
                             bool editing);

  Tab(const Tab&) = delete;
  Tab& operator=(const Tab&) = delete;
  ~Tab() override;

  // Lets go of the tab's page, keeping its history, as Chrome discards a tab to save memory. Its
  // page loads again once it's shown.
  void Unload();

  // For a tab put back without loading: tells the app its page's URL, history and title as they
  // were, and finds its favicon among the profile's.
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
  // For a tab put back: its name, which Restored tells the app.
  void set_name(const std::string& name) { name_ = name; }
  // Its name, else its page's title.
  std::string Title();
  // The emoji the user gave the tab to show in place of its page's favicon, or empty.
  const std::string& icon() const { return icon_; }
  // Gives the tab |icon|, and tells the app with its state. Empty gives it back its favicon.
  void SetIcon(const std::string& icon);
  // For a tab put back: its icon, which its first state report tells the app.
  void set_icon(const std::string& icon) { icon_ = icon; }

  // Mutes or unmutes the tab, as Chrome's speaker button does.
  void SetMuted(bool muted);

  // The page the system's engine shows in place of Chromium's (system_engine.h), else null. Its
  // WebContents stays, unloaded, with the history it had.
  platform::SystemPage* system_page() const { return system_page_.get(); }
  // Shows |url| in the system's engine. Going back from its first page goes back to Chromium's.
  // False, doing nothing, if the tab may not.
  bool UseSystemEngine(const GURL& url);
  // Back to Chromium, which loads |url|, or if it's empty its page where it was.
  void UseChromium(const GURL& url = GURL());
  // For a tab showing a system page, as the user loads |url| in it: there for the page's site or
  // one the profile keeps for it, else back in Chromium, which the caller has load it. Returns
  // whether the system page took it.
  bool LoadInSystemPage(const GURL& url);
  // Back, forward, reload and stop, as the app's buttons, in whichever engine shows the page.
  bool CanGoBack();
  bool CanGoForward();
  void GoBack();
  void GoForward();
  void Reload(bool bypass_cache);
  void Stop();

  // The colour the tab's page shows until it draws its first frame, opaque. Each page view the tab
  // swaps in takes it too, which Chrome's views do on their own only off the Mac.
  void SetPageColor(SkColor color);

  // For g_browser_core_page_hung (//browser_core/hooks.h): |contents|'s page hasn't answered input
  // for Chrome's hang timeout. Its tab is BC_PAGE_HUNG until it answers, or until Wait or Reload.
  static void PageHung(content::WebContents* contents,
                       content::RenderWidgetHost* widget,
                       base::RepeatingClosure restart_hang_monitor);
  // For g_browser_core_offline_game (//browser_core/hooks.h): while a run of the offline page's
  // game is going, Chromium's auto-reload waits, and it loads the page once the run ends if the
  // network came back meanwhile, so a page coming back never throws a run away.
  static void GameRunning(content::WebContents* contents, bool running);
  // Loads the page again, as bc_tab_reload says, ending a hung page's renderer first.
  void Reload(bool bypass_cache);
  // Gives a hung page Chrome's hang timeout again, as Chrome's Wait does.
  void Wait();
  // For a window that stopped showing the tab: a page whose renderer died is unloaded rather than
  // kept with nothing to show, so it loads again once it's shown.
  void LeftWindow();
  // Whether the page's renderer died while a window showed it, so its pane shows the app's card in
  // place of the page until it's loaded again.
  bool crashed() const { return crashed_; }

  // The modifier keys held at the user's last click or key in the page, as blink::WebInputEvent's
  // flags: what a link it opens was opened with.
  int input_modifiers() const { return input_modifiers_; }

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
  void PrimaryMainFrameRenderProcessGone(base::TerminationStatus status) override;
  void OnRendererResponsive(content::RenderProcessHost* process) override;
  void TitleWasSet(content::NavigationEntry* entry) override;
  void DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) override;
  void DidUpdateAudioMutingState(bool muted) override;
  void DidGetUserInteraction(const blink::WebInputEvent& event) override;
  void OnVisibilityChanged(content::Visibility visibility) override;
  void WebContentsDestroyed() override;

  // platform::SystemPage::Delegate:
  void SystemPageChanged() override;
  void SystemPageCommitted() override;
  void SystemPageLoaded() override;
  void SystemPageLeft(const GURL& url) override;
  void SystemPageOpen(const GURL& url) override;
  bool SystemPageKey(const input::NativeWebKeyboardEvent& event,
                     bool before_page,
                     bool editing) override;
  void SystemPageClicked() override;

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
  // A BC_PAGE_ value, for the app and agents.
  int Trouble() const;
  // Says the page is fine again if it hung, without telling the app.
  void ClearHang();
  // Lays out each window showing the tab, from a task of its own, so a crashed page's pane shows
  // the app's card, or the page again.
  void LayOutSoon();
  // Puts the core's error page on the pane's colour, and every other page back on Chrome's white.
  void ShowOnPageColor();
  // Drops the system page, and has the window show Chromium's view in its place.
  void DropSystemPage();
  // Tells the window, Site Controls and the app that the tab's page is now the other engine's.
  void EngineChanged();
  // Finds the favicon of the system page's URL among the profile's, as Chrome's driver can't.
  void FetchSystemPageFavicon();

  const bc_id id_;
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
  bool crashed_ = false;
  // While the page is hung: Chrome's Wait, and the id of the renderer that hung, which Reload ends.
  base::RepeatingClosure restart_hang_monitor_;
  int hung_process_ = 0;
  // A reload waiting for the hung renderer Reload ended to go.
  std::optional<bool> reload_after_kill_;
  // Chromium's net error code while the page is its error page, else 0.
  int net_error_ = 0;
  // The page base colour the tab last gave its pages, if not Chrome's.
  std::optional<SkColor> shown_on_;
  std::unique_ptr<platform::SystemPage> system_page_;
  // The system page's favicon, PNG, and its title, which the app was last told of.
  std::vector<uint8_t> system_page_favicon_;
  std::string system_page_title_;
  base::CancelableTaskTracker favicon_tracker_;
  base::WeakPtrFactory<Tab> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_TAB_H_
