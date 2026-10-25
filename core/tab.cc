#include "browser_core/tab.h"

#include <map>
#include <string>

#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/favorites.h"
#include "browser_core/host.h"
#include "browser_core/session.h"
#include "browser_core/space.h"
#include "browser_core/window.h"
#include "components/favicon/content/content_favicon_driver.h"
#include "content/public/browser/favicon_status.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/image/image_skia.h"
#include "ui/gfx/image/image_skia_rep.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

std::map<bc_id, content::WebContents*>& Tabs() {
  static base::NoDestructor<std::map<bc_id, content::WebContents*>> tabs;
  return *tabs;
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

Tab::Tab(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<Tab>(*contents),
      id_(NewId()) {
  Tabs()[id_] = contents;
  // Chrome's tab helpers make the driver as the tab is made, before anything asks for its id.
  if (auto* driver = favicon::ContentFaviconDriver::FromWebContents(contents)) {
    favicon_observation_.Observe(driver);
  }
}

Tab::~Tab() {
  Tabs().erase(id_);
}

// static
bool Tab::Loaded(bc_id id) {
  content::WebContents* contents = FromId(id);
  return contents && !contents->GetController().NeedsReload();
}

// static
GURL Tab::PageUrl(content::WebContents* contents) {
  // A tab starts with an entry for no page, until its first page commits.
  content::NavigationEntry* entry = contents->GetController().GetLastCommittedEntry();
  return entry && !entry->IsInitialEntry() ? entry->GetURL() : contents->GetVisibleURL();
}

void Tab::Unload() {
  if (web_contents()->GetController().NeedsReload()) {
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

void Tab::DidStartNavigation(content::NavigationHandle* navigation) {
  if (navigation->IsInPrimaryMainFrame()) {
    StateChanged();
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
  // shows only once it commits, so a page can't put another site's URL over its own.
  const std::string url = web_contents()->GetVisibleURL().spec();
  const Space* space = Space::Holding(id_);
  const GURL pinned_url = space ? space->PinnedUrl(id_) : Favorites::Url(id_);
  const bc_tab_state state = {
      .url = url.c_str(),
      .can_go_back = history.CanGoBack(),
      .can_go_forward = history.CanGoForward(),
      .loading = web_contents()->ShouldShowLoadingUI(),
      .unloaded = history.NeedsReload(),
      .away_from_pinned_url = pinned_url.is_valid() && PageUrl(web_contents()) != pinned_url,
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

std::string Tab::Title() {
  return name_.empty() ? base::UTF16ToUTF8(web_contents()->GetTitle()) : name_;
}

void Tab::ReportTitle() {
  if (Host().tab_title_changed) {
    Host().tab_title_changed(Host().context, id_, Title().c_str());
  }
}

void Tab::DidGetUserInteraction(const blink::WebInputEvent& event) {
  // A click, not the keyboard coming to the page: the core moves the keyboard between the pages of
  // a split view itself, and focus reports posted from those moves arrive after the next one.
  if (event.GetType() == blink::WebInputEvent::Type::kMouseDown) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::OnPageClicked, id_));
  }
}

void Tab::DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) {
  // Posted, as the state is: Chrome ends a page's full screen within the app's call that switches
  // or closes its tab. By id, so the app hears it once the tab has gone too.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](bc_id id, bool fullscreen) {
                       if (Host().tab_fullscreen_changed) {
                         Host().tab_fullscreen_changed(Host().context, id, fullscreen);
                       }
                     },
                     id_, entered_fullscreen));
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
