#include "browser_core/drm_request.h"

#include <set>
#include <string>
#include <string_view>
#include <utility>

#include "base/functional/bind.h"
#include "base/memory/weak_ptr.h"
#include "base/strings/string_util.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/drm_request.mojom.h"
#include "browser_core/host.h"
#include "browser_core/platform/system_page.h"
#include "browser_core/system_engine.h"
#include "browser_core/tab.h"
#include "browser_core/widevine.h"
#include "chrome/browser/profiles/profile.h"
#include "content/public/browser/document_service.h"
#include "content/public/browser/page.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "mojo/public/cpp/bindings/pending_receiver.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// Longer than any key system's name, so the page can't make the browser look at a long string.
constexpr size_t kLongestKeySystem = 128;

// The protected-media system a page's key system belongs to, in the words the app hears, else
// empty. The page chooses the string, so the app gets one of these two and never the page's own.
std::string_view SystemOf(std::string_view key_system) {
  if (key_system.size() > kLongestKeySystem) {
    return {};
  }
  if (key_system == "com.widevine.alpha") {
    return "com.widevine.alpha";
  }
  if (base::StartsWith(key_system, "com.microsoft.playready")) {
    return "com.microsoft.playready";
  }
  // ClearKey plays, and Apple's FairPlay is what Safari asks for: Chromium has no FairPlay on a
  // Mac, so no site asks for it here.
  return {};
}

// Whether ilo can play what |system| protects: Widevine once Chromium loaded Google's module,
// which on Linux the user can have it download from the note (widevine.h). ungoogled-chromium ships
// none, and PlayReady has no CDM here.
bool CanPlay(std::string_view system) {
  return system == "com.widevine.alpha" && widevine::Loaded();
}

// Whether two pages are at the same site, as the note goes with the site.
bool SameSite(const GURL& a, const GURL& b) {
  return net::registry_controlled_domains::SameDomainOrHost(
      a, b, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
}

// What the app has been told about one tab: the system a page of it was refused that ilo can't
// play, until the tab goes to another site.
class Unsupported : public content::WebContentsObserver,
                    public content::WebContentsUserData<Unsupported> {
 public:
  Unsupported(const Unsupported&) = delete;
  Unsupported& operator=(const Unsupported&) = delete;
  ~Unsupported() override = default;

  // The tab's document was refused |system|.
  void Asked(const std::string& system) {
    // Once for each system a document asks for, as pages ask again at every play. A site that was
    // given one plays with it, as Netflix asks for Widevine before PlayReady on Windows.
    const GURL& url = web_contents()->GetLastCommittedURL();
    if (NoteClosed(url) || (!played_at_.is_empty() && SameSite(url, played_at_)) ||
        !asked_.insert(system).second) {
      return;
    }
    if (shown_.empty()) {
      shown_at_ = web_contents()->GetLastCommittedURL();
    }
    shown_ = system;
    Changed();
  }

  bool shown() const { return !shown_.empty(); }
  const std::string& system() const { return shown_; }

  // Whether the user closed the note for |url|'s site in the tab's profile.
  bool NoteClosed(const GURL& url) {
    return system_engine::NoteClosed(
        Profile::FromBrowserContext(web_contents()->GetBrowserContext()), url);
  }

  // The user closed the note for a site: the tab takes it back if it's at that site.
  void Closed() {
    if (shown() && NoteClosed(shown_at_)) {
      Forget();
    }
  }

  // The tab's document was given a system the note names: the site plays, and has no note while
  // the tab stays at it.
  void Played() {
    played_at_ = web_contents()->GetLastCommittedURL();
    Forget();
  }

  // The tab's page is no longer Chromium's, or its site's note was closed: what it asked for goes
  // with it.
  void Forget() {
    asked_.clear();
    if (!shown_.empty()) {
      shown_.clear();
      Changed();
    }
  }

 private:
  friend class content::WebContentsUserData<Unsupported>;

  explicit Unsupported(content::WebContents* contents)
      : content::WebContentsObserver(contents),
        content::WebContentsUserData<Unsupported>(*contents) {}

  // content::WebContentsObserver:
  void PrimaryPageChanged(content::Page& page) override {
    // A new document asks again.
    asked_.clear();
    const GURL& now = page.GetMainDocument().GetLastCommittedURL();
    if (!played_at_.is_empty() && !SameSite(now, played_at_)) {
      played_at_ = GURL();
    }
    if (shown_.empty()) {
      return;
    }
    if (!SameSite(now, shown_at_)) {
      shown_.clear();
      Changed();
    }
  }

  // Tells the app from a task of its own, as the tab's other reports are: a navigation can end
  // inside the app's call that made it.
  void Changed() {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Unsupported::Report, weak_factory_.GetWeakPtr()));
  }

  void Report() {
    // Only what changed, as a burst of navigations posts several.
    if (told_ == shown_ || !Host().tab_media_unsupported) {
      return;
    }
    told_ = shown_;
    Host().tab_media_unsupported(Host().context, Tab::IdOf(web_contents()), shown_.c_str());
  }

  // What this document asked for.
  std::set<std::string> asked_;
  // The system asked for last that the app should show the note for, else empty, and the page the
  // note began at.
  std::string shown_;
  GURL shown_at_;
  // A page at the site that was given a system, if the tab is still at it.
  GURL played_at_;
  // What the app was last told.
  std::string told_;
  base::WeakPtrFactory<Unsupported> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

WEB_CONTENTS_USER_DATA_KEY_IMPL(Unsupported);

// The page's side of the connection. It goes with the connection or the document: a page makes one
// for each request.
class DrmRequest final : public content::DocumentService<mojom::DrmRequest> {
 public:
  static void Bind(content::RenderFrameHost* frame,
                   mojo::PendingReceiver<mojom::DrmRequest> receiver) {
    new DrmRequest(*frame, std::move(receiver));
  }

  ~DrmRequest() override = default;

 private:
  DrmRequest(content::RenderFrameHost& frame, mojo::PendingReceiver<mojom::DrmRequest> receiver)
      : content::DocumentService<mojom::DrmRequest>(frame, std::move(receiver)) {}

  // mojom::DrmRequest:
  void KeySystemAnswered(const std::string& key_system, bool granted) override {
    content::RenderFrameHost& frame = render_frame_host();
    // The page the user is on, not one that's loading behind it or kept for going back.
    if (!frame.GetPage().IsPrimary()) {
      return;
    }
    const std::string_view system = SystemOf(key_system);
    content::WebContents* contents = content::WebContents::FromRenderFrameHost(&frame);
    if (system.empty() || !contents) {
      return;
    }
    if (granted) {
      Unsupported::GetOrCreateForWebContents(contents)->Played();
      return;
    }
    if (!CanPlay(system)) {
      Unsupported::GetOrCreateForWebContents(contents)->Asked(std::string(system));
    }
  }
};

}  // namespace

void PopulateFrameBinders(mojo::BinderMapWithContext<content::RenderFrameHost*>* map) {
  map->Add<mojom::DrmRequest>(&DrmRequest::Bind);
}

void ForgetMediaUnsupported(content::WebContents* contents) {
  if (Unsupported* unsupported = Unsupported::FromWebContents(contents)) {
    unsupported->Forget();
  }
}

bool MediaUnsupported(content::WebContents* contents) {
  Unsupported* unsupported = Unsupported::FromWebContents(contents);
  return unsupported && unsupported->shown();
}

bool SystemEngineOffered(content::WebContents* contents) {
  Unsupported* unsupported = Unsupported::FromWebContents(contents);
  return unsupported && unsupported->shown() &&
         platform::SystemPage::Offered(unsupported->system(),
                                       contents->GetBrowserContext()->IsOffTheRecord());
}

void CloseMediaNote(content::WebContents* contents) {
  system_engine::CloseNote(Profile::FromBrowserContext(contents->GetBrowserContext()),
                           Tab::PageUrl(contents));
  // Each asks its own profile, so a private window's closing leaves its profile's tabs alone, and
  // the profile's reaches its private windows, which read its sites.
  for (content::WebContents* each : Tab::Pages()) {
    if (Unsupported* unsupported = Unsupported::FromWebContents(each)) {
      unsupported->Closed();
    }
  }
}

}  // namespace browser_core
