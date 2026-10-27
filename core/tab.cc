#include "browser_core/tab.h"

#include <cstdio>
#include <map>
#include <string>
#include <utility>

#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "browser_core/favorites.h"
#include "browser_core/host.h"
#include "browser_core/link_target.h"
#include "browser_core/session.h"
#include "browser_core/space.h"
#include "browser_core/window.h"
#include "chrome/browser/favicon/favicon_service_factory.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/recently_audible_helper.h"
#include "chrome/browser/ui/tabs/tab_muted_utils.h"
#include "components/favicon/content/content_favicon_driver.h"
#include "components/favicon/core/favicon_service.h"
#include "components/favicon_base/favicon_types.h"
#include "components/keyed_service/core/service_access_type.h"
#include "content/public/browser/favicon_status.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/reload_type.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_process_host.h"
#include "content/public/browser/render_widget_host.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"
#include "content/public/common/result_codes.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/image/image_skia.h"
#include "ui/gfx/image/image_skia_rep.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// 16 points at 2×, as tab_favicon_changed's.
constexpr int kFaviconPixels = 32;

std::map<bc_id, content::WebContents*>& Tabs() {
  static base::NoDestructor<std::map<bc_id, content::WebContents*>> tabs;
  return *tabs;
}

// The line check.sh times a page's full screen from, on the uptime clock the app's and the host's
// lines read.
void LogFullScreenAsked(bool entered) {
  fprintf(stderr, "browser: full screen %s asked at %.1f ms\n", entered ? "enter" : "exit",
          base::TimeTicks::Now().since_origin().InMillisecondsF());
}

// Whether a renderer that went with |status| died, rather than ending as asked, as it does when
// the core closes a page on purpose.
bool Died(base::TerminationStatus status) {
  switch (status) {
    case base::TERMINATION_STATUS_ABNORMAL_TERMINATION:
    case base::TERMINATION_STATUS_PROCESS_WAS_KILLED:
    case base::TERMINATION_STATUS_PROCESS_CRASHED:
    case base::TERMINATION_STATUS_LAUNCH_FAILED:
    case base::TERMINATION_STATUS_OOM:
    case base::TERMINATION_STATUS_EVICTED_FOR_MEMORY:
      return true;
    default:
      return false;
  }
}

}  // namespace

// static
bc_id Tab::IdOf(content::WebContents* contents) {
  return GetOrCreateForWebContents(contents)->id_;
}

// static
content::WebContents* Tab::FromId(bc_id id) {
  auto it = Tabs().find(id);
  return it == Tabs().end() ? nullptr : it->second;
}

// static
void Tab::LinkTargetChanged(BrowserWindowInterface* browser,
                            content::WebContents* source,
                            const GURL& url) {
  // Other Browsers, such as a DevTools window's, aren't the app's.
  if (Window::ForBrowser(browser)) {
    GetOrCreateForWebContents(source)->ReportLinkTarget(url);
  }
}

Tab::Tab(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<Tab>(*contents),
      id_(NewId()) {
  Tabs()[id_] = contents;
  // Chrome's tab helpers make the driver as the tab is made, before anything asks for its id.
  if (auto* driver = favicon::ContentFaviconDriver::FromWebContents(contents)) {
    favicon_observation_.Observe(driver);
  }
  // It keeps a tab audible 2 s after its sound stops, so the speaker doesn't flicker between
  // sounds.
  if (auto* audible = RecentlyAudibleHelper::FromWebContents(contents)) {
    audible_subscription_ = audible->RegisterRecentlyAudibleChangedCallback(
        base::IgnoreArgs<bool>(base::BindRepeating(&Tab::ReportStateSoon, base::Unretained(this))));
  }
  // A tab is often made with its first page already on its way, before anything observes it, so
  // the app hears it's loading now rather than once the page commits.
  state_report_posted_ = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Tab::ReportState, weak_factory_.GetWeakPtr()));
}

Tab::~Tab() {
  Tabs().erase(id_);
}

// static
bool Tab::Loaded(bc_id id) {
  content::WebContents* contents = FromId(id);
  return contents && !contents->GetController().NeedsReload() &&
         !FromWebContents(contents)->waiting_;
}

// static
GURL Tab::PageUrl(content::WebContents* contents) {
  if (const Tab* tab = FromWebContents(contents); tab && tab->waiting_) {
    return tab->waiting_->url;
  }
  // A tab starts with an entry for no page, until its first page commits.
  content::NavigationEntry* entry = contents->GetController().GetLastCommittedEntry();
  return entry && !entry->IsInitialEntry() ? entry->GetURL() : contents->GetVisibleURL();
}

void Tab::Unload() {
  if (waiting_ || web_contents()->GetController().NeedsReload()) {
    return;
  }
  // In place, so the tab keeps its id and its history, and loads again once it's shown.
  web_contents()->Discard(base::DoNothing());
  StateChanged();
}

void Tab::Restored() {
  StateChanged();
  // Posted, as the state is, so the app hears of it once its call has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Tab::ReportTitle, weak_factory_.GetWeakPtr()));
  // Pages don't load until shown, and Chrome finds a favicon once a page commits.
  if (auto* driver = favicon::ContentFaviconDriver::FromWebContents(web_contents())) {
    driver->FetchFavicon(web_contents()->GetController().GetLastCommittedEntry()->GetURL(),
                         /*is_same_document=*/false);
  }
}

void Tab::LoadWhenShown(content::NavigationController::LoadURLParams load) {
  waiting_ = std::make_unique<content::NavigationController::LoadURLParams>(std::move(load));
  StateChanged();
  favicon::FaviconService* service = FaviconServiceFactory::GetForProfile(
      Profile::FromBrowserContext(web_contents()->GetBrowserContext()),
      ServiceAccessType::EXPLICIT_ACCESS);
  if (!service) {
    return;
  }
  // Chrome's favicon driver finds one only for a page that has committed.
  service->GetRawFaviconForPageURL(
      waiting_->url, {favicon_base::IconType::kFavicon}, kFaviconPixels,
      /*fallback_to_host=*/true,
      base::BindOnce(
          [](base::WeakPtr<Tab> tab, const favicon_base::FaviconRawBitmapResult& result) {
            // Once the page loads, its favicon comes from Chrome.
            if (tab && tab->waiting_ && result.is_valid() && Host().tab_favicon_changed) {
              Host().tab_favicon_changed(Host().context, tab->id_, result.bitmap_data->data(),
                                         result.bitmap_data->size());
            }
          },
          weak_factory_.GetWeakPtr()),
      &favicon_tracker_);
}

void Tab::LoadWaiting() {
  if (!waiting_) {
    return;
  }
  const std::unique_ptr<content::NavigationController::LoadURLParams> load = std::move(waiting_);
  web_contents()->GetController().LoadURLWithParams(*load);
  StateChanged();
}

void Tab::SetMuted(bool muted) {
  // The reason extensions read, as for Chrome's speaker button.
  SetTabAudioMuted(web_contents(), muted, TabMutedReason::kAudioIndicator, std::string());
}

void Tab::DidStartNavigation(content::NavigationHandle* navigation) {
  if (navigation->IsInPrimaryMainFrame()) {
    StateChanged();
    // The page under the pointer is going; the new one tells where the pointer is on it. A link
    // that scrolls the same page keeps it.
    if (!navigation->IsSameDocument()) {
      ReportLinkTarget(GURL());
    }
  }
}

void Tab::DidFinishNavigation(content::NavigationHandle* navigation) {
  if (!navigation->IsInPrimaryMainFrame()) {
    return;
  }
  if (navigation->HasCommitted() && !navigation->IsSameDocument()) {
    net_error_ = navigation->IsErrorPage() ? navigation->GetNetErrorCode() : 0;
    ShowOnPageColor();
    ClearHang();
    if (std::exchange(crashed_, false)) {
      LayOutSoon();
    }
  }
  StateChanged();
}

void Tab::DidStartLoading() {
  StateChanged();
}

void Tab::DidStopLoading() {
  StateChanged();
}

void Tab::NavigationEntriesDeleted() {
  StateChanged();
}

void Tab::StateChanged() {
  SessionChanged();
  ReportStateSoon();
}

void Tab::ReportStateSoon() {
  if (state_report_posted_) {
    return;
  }
  state_report_posted_ = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Tab::ReportState, weak_factory_.GetWeakPtr()));
}

void Tab::ReportState() {
  state_report_posted_ = false;
  if (!Host().tab_state_changed) {
    return;
  }
  content::NavigationController& history = web_contents()->GetController();
  // The visible URL is the one Chrome's location bar shows: one a page starts loading by itself
  // shows only once it commits, so a page can't put another site's URL over its own. One waiting
  // to load is the link the user opened.
  const std::string url = (waiting_ ? waiting_->url : web_contents()->GetVisibleURL()).spec();
  const Space* space = Space::Holding(id_);
  const GURL pinned_url = space ? space->PinnedUrl(id_) : Favorites::Url(id_);
  const RecentlyAudibleHelper* audible = RecentlyAudibleHelper::FromWebContents(web_contents());
  const bc_tab_state state = {
      .url = url.c_str(),
      .can_go_back = history.CanGoBack(),
      .can_go_forward = history.CanGoForward(),
      .loading = web_contents()->ShouldShowLoadingUI(),
      .unloaded = history.NeedsReload() || waiting_,
      .away_from_pinned_url = pinned_url.is_valid() && PageUrl(web_contents()) != pinned_url,
      .audible = audible && audible->WasRecentlyAudible(),
      .muted = web_contents()->IsAudioMuted(),
      .icon = icon_.c_str(),
      .trouble = Trouble(),
      .net_error = Trouble() == BC_PAGE_NETWORK_ERROR ? net_error_ : 0,
  };
  Host().tab_state_changed(Host().context, id_, &state);
  ReportFavicon(state.loading);
}

void Tab::ReportFavicon(bool loading) {
  if (!Host().tab_favicon_changed) {
    return;
  }
  content::NavigationEntry* entry = web_contents()->GetController().GetLastCommittedEntry();
  gfx::Image favicon;
  if (entry && entry->GetFavicon().valid) {
    favicon = entry->GetFavicon().image;
  } else if (loading) {
    // The new page's favicon comes once it has committed, from the history database or the page,
    // so the old one stays rather than flashing none.
    return;
  }
  if (favicon.AsImageSkia().BackedBySameObjectAs(favicon_.AsImageSkia())) {
    return;
  }
  favicon_ = favicon;
  std::optional<std::vector<uint8_t>> png;
  if (!favicon.IsEmpty()) {
    png = gfx::PNGCodec::EncodeBGRASkBitmap(favicon.AsImageSkia().GetRepresentation(2).GetBitmap(),
                                            /*discard_transparency=*/false);
  }
  Host().tab_favicon_changed(Host().context, id_, png ? png->data() : nullptr,
                             png ? png->size() : 0);
}

void Tab::OnFaviconUpdated(favicon::FaviconDriver* driver,
                           NotificationIconType type,
                           const GURL& icon_url,
                           bool icon_url_changed,
                           const gfx::Image& image) {
  if (type == NON_TOUCH_16_DIP) {
    StateChanged();
  }
}

void Tab::DidFinishLoad(content::RenderFrameHost* frame, const GURL& url) {
  // An unloaded page's empty document finishes loading at the page's URL, which isn't the page.
  if (frame->IsInPrimaryMainFrame() && !web_contents()->WasDiscarded() && Host().tab_loaded) {
    Host().tab_loaded(Host().context, id_, url.spec().c_str());
  }
}

void Tab::SetPageColor(SkColor color) {
  page_color_ = color;
  if (content::RenderWidgetHostView* view = web_contents()->GetRenderWidgetHostView()) {
    view->SetBackgroundColor(color);
  }
  ShowOnPageColor();
}

void Tab::ShowOnPageColor() {
  // The core's error page has no background of its own (//browser_core/error_page.cc), so it sits
  // on the colour the pane shows before a page draws, as the app's cards sit on theirs. Every other
  // page keeps Chrome's white under it.
  const std::optional<SkColor> color = net_error_ ? page_color_ : std::nullopt;
  if (color != shown_on_) {
    shown_on_ = color;
    web_contents()->SetPageBaseBackgroundColor(color);
  }
}

void Tab::RenderFrameHostChanged(content::RenderFrameHost* old_host,
                                 content::RenderFrameHost* new_host) {
  // A page from another site comes in a view of its own, white until its first frame.
  if (page_color_ && new_host == web_contents()->GetPrimaryMainFrame() && new_host->GetView()) {
    new_host->GetView()->SetBackgroundColor(*page_color_);
  }
}

void Tab::RenderViewReady() {
  if (page_color_) {
    SetPageColor(*page_color_);
  }
  // A page that died has a renderer again, loading it: its pane shows it again.
  if (std::exchange(crashed_, false)) {
    LayOutSoon();
    StateChanged();
  }
}

void Tab::TitleWasSet(content::NavigationEntry* entry) {
  SessionChanged();
  ReportTitle();
}

void Tab::Rename(const std::string& name) {
  name_ = name;
  SessionChanged();
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Tab::ReportTitle, weak_factory_.GetWeakPtr()));
}

void Tab::SetIcon(const std::string& icon) {
  icon_ = icon;
  StateChanged();
}

std::string Tab::Title() {
  return name_.empty() ? base::UTF16ToUTF8(web_contents()->GetTitle()) : name_;
}

void Tab::ReportTitle() {
  if (Host().tab_title_changed) {
    Host().tab_title_changed(Host().context, id_, Title().c_str());
  }
}

void Tab::DidGetUserInteraction(const blink::WebInputEvent& event) {
  input_modifiers_ = event.GetModifiers();
  // A click, not the keyboard coming to the page: the core moves the keyboard between the pages of
  // a split view itself, and focus reports posted from those moves arrive after the next one.
  if (event.GetType() == blink::WebInputEvent::Type::kMouseDown) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::OnPageClicked, id_));
  }
}

void Tab::ReportLinkTarget(const GURL& url) {
  // Only as it changes: nothing is told while the pointer is on no link and none was shown.
  if (url.is_empty() && !link_target_) {
    return;
  }
  link_target_ = !url.is_empty();
  if (!Host().tab_link_target) {
    return;
  }
  const LinkTarget target = FormatLinkTarget(url);
  Host().tab_link_target(Host().context, id_, target.text.c_str(), target.domain_begin,
                         target.domain_end);
}

void Tab::DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) {
  // Once each way: a page leaving full screen itself is reported as Chrome ends its full screen
  // and again as the page's request returns.
  if (entered_fullscreen == fullscreen_) {
    return;
  }
  fullscreen_ = entered_fullscreen;
  LogFullScreenAsked(entered_fullscreen);
  // At once, as only the page's request takes it full screen: Chrome has asked the window to go
  // with it, in a task it posted, and the app lays the page out at the window's new size only if
  // it hears first.
  if (entered_fullscreen) {
    if (Host().tab_fullscreen_changed) {
      Host().tab_fullscreen_changed(Host().context, id_, true);
    }
    return;
  }
  // Posted, as the state is: Chrome ends a page's full screen within the app's call that switches
  // or closes its tab. By id, so the app hears it once the tab has gone too. Still before the
  // window leaves full screen, which Chrome posts after.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](bc_id id) {
                       if (Host().tab_fullscreen_changed) {
                         Host().tab_fullscreen_changed(Host().context, id, false);
                       }
                     },
                     id_));
}

void Tab::DidUpdateAudioMutingState(bool muted) {
  ReportStateSoon();
}

void Tab::OnVisibilityChanged(content::Visibility visibility) {
  // As Chrome loads a restored tab once it's shown. Posted: the page view is being shown now.
  if (visibility == content::Visibility::VISIBLE && waiting_) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Tab::LoadWaiting, weak_factory_.GetWeakPtr()));
  }
}

// The page's renderer went away, crashed, killed or ended on purpose. check.sh waits for this line,
// an error since our builds print no warnings.
void Tab::PrimaryMainFrameRenderProcessGone(base::TerminationStatus status) {
  LOG(ERROR) << "browser: tab " << id_ << " page gone, status " << static_cast<int>(status);
  ClearHang();
  if (reload_after_kill_) {
    // Posted: Chrome is still telling the page's observers it went.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Tab::Reload, weak_factory_.GetWeakPtr(),
                                  *std::exchange(reload_after_kill_, std::nullopt)));
  } else if (Died(status) && !browser_shutdown::HasShutdownStarted()) {
    if (Window::Showing(id_)) {
      crashed_ = true;
      LayOutSoon();
    } else {
      // Behind, it has nothing to show the user, so it's unloaded, as Chrome does with a page it
      // ended for memory, and loads again once it's shown.
      web_contents()->GetController().SetNeedsReload();
    }
  }
  StateChanged();
}

// static
void Tab::PageHung(content::WebContents* contents,
                   content::RenderWidgetHost* widget,
                   base::RepeatingClosure restart_hang_monitor) {
  // A page in none of the core's tabs, such as DevTools', waits as it would after Chrome's Wait.
  Tab* tab = FromWebContents(contents);
  if (!tab) {
    return;
  }
  tab->restart_hang_monitor_ = std::move(restart_hang_monitor);
  tab->hung_process_ = widget->GetProcess()->GetDeprecatedID();
  // check.sh waits for this line.
  LOG(ERROR) << "browser: tab " << tab->id_ << " page hung";
  tab->ReportStateSoon();
}

void Tab::OnRendererResponsive(content::RenderProcessHost* process) {
  if (hung_process_ && process->GetDeprecatedID() == hung_process_) {
    ClearHang();
    ReportStateSoon();
  }
}

void Tab::Wait() {
  if (!restart_hang_monitor_) {
    return;
  }
  base::RepeatingClosure restart = std::move(restart_hang_monitor_);
  ClearHang();
  restart.Run();
  ReportStateSoon();
}

void Tab::Reload(bool bypass_cache) {
  content::RenderProcessHost* hung =
      hung_process_ ? content::RenderProcessHost::FromID(hung_process_) : nullptr;
  ClearHang();
  if (hung && hung->IsInitializedAndNotDead()) {
    const bool page = hung == web_contents()->GetPrimaryMainFrame()->GetProcess();
    // As Chrome's Exit page: a reload would wait on the renderer that hung.
    hung->Shutdown(content::RESULT_CODE_HUNG);
    // The page reloads once its renderer has gone. A frame's alone ends without it.
    if (page) {
      reload_after_kill_ = bypass_cache;
      ReportStateSoon();
      return;
    }
  }
  // As Chrome's, it asks before sending a form again.
  web_contents()->GetController().Reload(
      bypass_cache ? content::ReloadType::BYPASSING_CACHE : content::ReloadType::NORMAL,
      /*check_for_repost=*/true);
}

void Tab::ClearHang() {
  restart_hang_monitor_.Reset();
  hung_process_ = 0;
}

void Tab::LeftWindow() {
  if (!crashed_) {
    return;
  }
  // Posted: the window letting go of the tab still lists it, and another may be taking it.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](base::WeakPtr<Tab> tab) {
                       if (tab && tab->crashed_ && !Window::Showing(tab->id_)) {
                         tab->crashed_ = false;
                         tab->web_contents()->GetController().SetNeedsReload();
                         tab->StateChanged();
                       }
                     },
                     weak_factory_.GetWeakPtr()));
}

void Tab::LayOutSoon() {
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Window::LayOutShowing, id_));
}

int Tab::Trouble() const {
  if (crashed_) {
    return BC_PAGE_CRASHED;
  }
  // An unloaded page has no renderer to hang and no error page to show.
  if (waiting_ || web_contents()->GetController().NeedsReload()) {
    return BC_PAGE_FINE;
  }
  if (hung_process_) {
    return BC_PAGE_HUNG;
  }
  return net_error_ ? BC_PAGE_NETWORK_ERROR : BC_PAGE_FINE;
}

void Tab::WebContentsDestroyed() {
  // The driver is the page's too, and may go before the tab does.
  favicon_observation_.Reset();
  Window::OnTabClosing(id_);
  if (Host().tab_closed) {
    Host().tab_closed(Host().context, id_);
  }
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(Tab);

}  // namespace browser_core
