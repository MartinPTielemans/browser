#include "browser_core/tab.h"

#include <algorithm>
#include <cstdio>
#include <map>
#include <string>
#include <utility>

#include "base/auto_reset.h"
#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/memory/ptr_util.h"
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
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/recently_audible_helper.h"
#include "chrome/browser/ui/tabs/tab_muted_utils.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "components/favicon/content/content_favicon_driver.h"
#include "components/favicon/core/favicon_service.h"
#include "components/favicon_base/favicon_types.h"
#include "components/keyed_service/core/service_access_type.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/favicon_status.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_contents_user_data.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/image/image_skia.h"
#include "ui/gfx/image/image_skia_rep.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// 16 points at 2×, as tab_favicon_changed's.
constexpr int kFaviconPixels = 32;

std::map<bc_id, std::unique_ptr<Tab>>& Tabs() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Tab>>> tabs;
  return *tabs;
}

// The line check.sh times a page's full screen from, on the uptime clock the app's and the host's
// lines read.
void LogFullScreenAsked(bool entered) {
  fprintf(stderr, "browser: full screen %s asked at %.1f ms\n", entered ? "enter" : "exit",
          base::TimeTicks::Now().since_origin().InMillisecondsF());
}

// The tab Wake is giving a page, which takes the first page asked for its id meanwhile: the core
// may hear of the page as Chrome makes it, before Wake does.
Tab* g_waking = nullptr;

// Ties a page to its tab, which outlives it once unloaded.
class TabLink : public content::WebContentsUserData<TabLink> {
 public:
  bc_id id() const { return id_; }

 private:
  friend class content::WebContentsUserData<TabLink>;

  TabLink(content::WebContents* contents, bc_id id)
      : content::WebContentsUserData<TabLink>(*contents), id_(id) {}

  const bc_id id_;

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

WEB_CONTENTS_USER_DATA_KEY_IMPL(TabLink);

// Forgets |id|, and deletes its tab once the caller, which may be the tab, has returned.
void Forget(bc_id id) {
  auto it = Tabs().find(id);
  if (it == Tabs().end()) {
    return;
  }
  std::unique_ptr<Tab> tab = std::move(it->second);
  Tabs().erase(it);
  base::SingleThreadTaskRunner::GetCurrentDefault()->DeleteSoon(FROM_HERE, std::move(tab));
}

}  // namespace

// static
bc_id Tab::IdOf(content::WebContents* contents) {
  if (const TabLink* link = TabLink::FromWebContents(contents)) {
    return link->id();
  }
  if (g_waking && !g_waking->web_contents()) {
    g_waking->Attach(contents);
    return g_waking->id_;
  }
  const bc_id id = NewId();
  Tab* tab = (Tabs()[id] = base::WrapUnique(
                  new Tab(id, Profile::FromBrowserContext(contents->GetBrowserContext()))))
                 .get();
  tab->Attach(contents);
  return id;
}

// static
Tab* Tab::FromWebContents(content::WebContents* contents) {
  const TabLink* link = contents ? TabLink::FromWebContents(contents) : nullptr;
  return link ? Get(link->id()) : nullptr;
}

// static
Tab* Tab::Get(bc_id id) {
  auto it = Tabs().find(id);
  return it == Tabs().end() ? nullptr : it->second.get();
}

// static
content::WebContents* Tab::FromId(bc_id id) {
  const Tab* tab = Get(id);
  return tab ? tab->web_contents() : nullptr;
}

// static
void Tab::LinkTargetChanged(BrowserWindowInterface* browser,
                            content::WebContents* source,
                            const GURL& url) {
  // Other Browsers, such as a DevTools window's, aren't the app's.
  if (Window::ForBrowser(browser)) {
    Get(IdOf(source))->ReportLinkTarget(url);
  }
}

// static
std::vector<Tab*> Tab::All() {
  std::vector<Tab*> all;
  for (auto& [id, tab] : Tabs()) {
    all.push_back(tab.get());
  }
  return all;
}

// static
bc_id Tab::Restore(Profile* profile, const SavedTab& saved) {
  if (!profile || saved.history.empty()) {
    return 0;
  }
  const bc_id id = NewId();
  Tab* tab = (Tabs()[id] = base::WrapUnique(new Tab(id, profile))).get();
  // Only what the page would keep: the rest of |saved| is its place, which the caller sees to.
  SavedTab& unloaded = tab->unloaded_.emplace();
  unloaded.history = saved.history;
  unloaded.current = std::clamp(saved.current, 0, static_cast<int>(saved.history.size()) - 1);
  unloaded.muted = saved.muted;
  tab->name_ = saved.name;
  tab->icon_ = saved.icon;
  if (!saved.last_seen.is_null()) {
    tab->last_seen_ = saved.last_seen;
  }
  // check.sh waits for this line, an error since our builds print no warnings.
  LOG(ERROR) << "browser: tab " << id << " without its page";
  tab->StateChanged();
  // Posted, as the state is, so the app hears of it once its call has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Tab::ReportTitle, tab->weak_factory_.GetWeakPtr()));
  tab->FindFavicon(tab->Url());
  return id;
}

Tab::Tab(bc_id id, Profile* profile) : id_(id), profile_(profile->GetWeakPtr()) {}

Tab::~Tab() = default;

void Tab::Attach(content::WebContents* contents) {
  Observe(contents);
  TabLink::CreateForWebContents(contents, id_);
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
  ReportStateSoon();
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

GURL Tab::Url() const {
  if (web_contents()) {
    return PageUrl(web_contents());
  }
  return unloaded_ ? unloaded_->history[unloaded_->current].virtual_url() : GURL();
}

SavedTab Tab::Save() const {
  SavedTab saved = web_contents() ? SaveTab(*web_contents()) : unloaded_.value_or(SavedTab());
  saved.name = name_;
  saved.icon = icon_;
  saved.muted = muted();
  return saved;
}

void Tab::Unload() {
  content::WebContents* contents = web_contents();
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!in) {
    return;
  }
  SavedTab saved = SaveTab(*contents);
  if (saved.history.empty()) {
    return;
  }
  unloaded_.emplace();
  unloaded_->history = std::move(saved.history);
  unloaded_->current = saved.current;
  unloaded_->muted = contents->IsAudioMuted();
  // Saved with the rest of its history.
  waiting_.reset();
  TabStripModel* tabs = in->GetBrowserWindowInterface()->GetTabStripModel();
  {
    base::AutoReset<bool> unloading(&unloading_, true);
    tabs->DetachAndDeleteWebContentsAt(tabs->GetIndexOfWebContents(contents));
  }
  LOG(ERROR) << "browser: tab " << id_ << " without its page";
  StateChanged();
}

content::WebContents* Tab::Wake(Window& window) {
  if (web_contents() || !unloaded_ || !profile()) {
    return web_contents();
  }
  content::WebContents* contents = nullptr;
  {
    base::AutoReset<Tab*> waking(&g_waking, this);
    contents = window.RestoreTab(profile(), unloaded_->history, unloaded_->current);
  }
  if (!contents) {
    return nullptr;
  }
  if (!web_contents()) {
    Attach(contents);
  }
  DCHECK_EQ(web_contents(), contents);
  const bool muted = std::exchange(unloaded_, std::nullopt)->muted;
  if (muted) {
    SetMuted(true);
  }
  LOG(ERROR) << "browser: tab " << id_ << " has its page";
  Restored();
  return contents;
}

void Tab::Close() {
  if (web_contents()) {
    return;
  }
  ReportClosed();
  Forget(id_);
}

void Tab::ReportClosed() {
  Window::OnTabClosing(id_);
  if (Host().tab_closed) {
    Host().tab_closed(Host().context, id_);
  }
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
  FindFavicon(waiting_->url);
}

void Tab::FindFavicon(const GURL& url) {
  favicon::FaviconService* service =
      profile() ? FaviconServiceFactory::GetForProfile(profile(), ServiceAccessType::EXPLICIT_ACCESS)
                : nullptr;
  if (!service) {
    return;
  }
  service->GetRawFaviconForPageURL(
      url, {favicon_base::IconType::kFavicon}, kFaviconPixels,
      /*fallback_to_host=*/true,
      base::BindOnce(
          [](base::WeakPtr<Tab> tab, const favicon_base::FaviconRawBitmapResult& result) {
            // Once the page loads, its favicon comes from Chrome.
            if (tab && !Loaded(tab->id_) && result.is_valid() && Host().tab_favicon_changed) {
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
  if (!web_contents()) {
    if (unloaded_) {
      unloaded_->muted = muted;
    }
    ReportStateSoon();
    return;
  }
  // The reason extensions read, as for Chrome's speaker button.
  SetTabAudioMuted(web_contents(), muted, TabMutedReason::kAudioIndicator, std::string());
}

bool Tab::muted() const {
  return web_contents() ? web_contents()->IsAudioMuted() : unloaded_ && unloaded_->muted;
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
  if (navigation->IsInPrimaryMainFrame()) {
    StateChanged();
  }
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
  const Space* space = Space::Holding(id_);
  const GURL pinned_url = space ? space->PinnedUrl(id_) : Favorites::Url(id_);
  content::WebContents* contents = web_contents();
  if (!contents) {
    // Its history says where it is and where it can go.
    const std::string url = Url().spec();
    const int count = unloaded_ ? static_cast<int>(unloaded_->history.size()) : 0;
    const int current = unloaded_ ? unloaded_->current : 0;
    const bc_tab_state state = {
        .url = url.c_str(),
        .can_go_back = current > 0,
        .can_go_forward = current + 1 < count,
        .loading = false,
        .unloaded = true,
        .away_from_pinned_url = pinned_url.is_valid() && Url() != pinned_url,
        .audible = false,
        .muted = muted(),
        .icon = icon_.c_str(),
    };
    Host().tab_state_changed(Host().context, id_, &state);
    return;
  }
  content::NavigationController& history = contents->GetController();
  // The visible URL is the one Chrome's location bar shows: one a page starts loading by itself
  // shows only once it commits, so a page can't put another site's URL over its own. One waiting
  // to load is the link the user opened.
  const std::string url = (waiting_ ? waiting_->url : contents->GetVisibleURL()).spec();
  const RecentlyAudibleHelper* audible = RecentlyAudibleHelper::FromWebContents(contents);
  const bc_tab_state state = {
      .url = url.c_str(),
      .can_go_back = history.CanGoBack(),
      .can_go_forward = history.CanGoForward(),
      .loading = contents->ShouldShowLoadingUI(),
      .unloaded = history.NeedsReload() || waiting_,
      .away_from_pinned_url = pinned_url.is_valid() && PageUrl(contents) != pinned_url,
      .audible = audible && audible->WasRecentlyAudible(),
      .muted = contents->IsAudioMuted(),
      .icon = icon_.c_str(),
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
  } else if (loading || web_contents()->GetController().NeedsReload()) {
    // The new page's favicon comes once it has committed, from the history database or the page,
    // so the old one stays rather than flashing none, as it does for a page given back and not yet
    // loaded.
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
  content::RenderWidgetHostView* view =
      web_contents() ? web_contents()->GetRenderWidgetHostView() : nullptr;
  if (view) {
    view->SetBackgroundColor(color);
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
  if (!name_.empty()) {
    return name_;
  }
  if (web_contents()) {
    return base::UTF16ToUTF8(web_contents()->GetTitle());
  }
  if (!unloaded_) {
    return std::string();
  }
  // As Chrome titles a page without one.
  const sessions::SerializedNavigationEntry& entry = unloaded_->history[unloaded_->current];
  return entry.title().empty() ? entry.virtual_url().spec() : base::UTF16ToUTF8(entry.title());
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

// The page's renderer went away, crashed or killed. The page shows nothing until it's reloaded, and
// check.sh waits for this line, an error since our builds print no warnings.
void Tab::PrimaryMainFrameRenderProcessGone(base::TerminationStatus status) {
  // Unloading deletes the page rather than ending its renderer, so Chrome says nothing here then,
  // and nothing the core does on purpose reads as a page that died.
  if (unloading_) {
    return;
  }
  LOG(ERROR) << "browser: tab " << id_ << " page gone, status " << static_cast<int>(status);
}

void Tab::WebContentsDestroyed() {
  // The driver is the page's too, and may go before the tab does.
  favicon_observation_.Reset();
  audible_subscription_ = {};
  Observe(nullptr);
  // Unloaded, it stays without its page.
  if (unloading_) {
    return;
  }
  ReportClosed();
  Forget(id_);
}

}  // namespace browser_core
