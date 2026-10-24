#include "browser_core/tab.h"

#include <map>
#include <string>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/session.h"
#include "browser_core/window.h"
#include "components/favicon/content/content_favicon_driver.h"
#include "content/public/browser/favicon_status.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
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
  const bc_tab_state state = {
      .url = url.c_str(),
      .can_go_back = history.CanGoBack(),
      .can_go_forward = history.CanGoForward(),
      .loading = web_contents()->ShouldShowLoadingUI(),
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
  if (frame->IsInPrimaryMainFrame() && Host().tab_loaded) {
    Host().tab_loaded(Host().context, id_, url.spec().c_str());
  }
}

void Tab::TitleWasSet(content::NavigationEntry* entry) {
  SessionChanged();
  ReportTitle();
}

void Tab::ReportTitle() {
  if (Host().tab_title_changed) {
    const std::string title = base::UTF16ToUTF8(web_contents()->GetTitle());
    Host().tab_title_changed(Host().context, id_, title.c_str());
  }
}

void Tab::DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) {
  if (Host().tab_fullscreen_changed) {
    Host().tab_fullscreen_changed(Host().context, id_, entered_fullscreen);
  }
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
