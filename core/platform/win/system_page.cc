// The system page on Windows: Microsoft Edge's WebView2, on the Evergreen runtime the OS has, for
// the protected video Chromium's own PlayReady can't play here (decision 1). The loader is linked
// statically and nothing downloads the runtime: without it there's no system engine, and the note
// says so. Nothing of WebView2 starts before the first system page, or before a store a page made
// is cleared or deleted; Available reads only the runtime's version, from the registry.

#include "browser_core/platform/system_page.h"

#include <windows.h>

#include <objbase.h>
#include <wrl/client.h>
#include <wrl/event.h>

#include <cstdio>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "WebView2.h"
#include "base/command_line.h"
#include "base/files/file_path.h"
#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/functional/callback_helpers.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/logging.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/ref_counted.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/path_service.h"
#include "base/strings/strcat.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "base/values.h"
#include "base/win/scoped_co_mem.h"
#include "browser_core/platform/playready.h"
#include "browser_core/public/browser_core.h"
#include "chrome/common/chrome_paths.h"
#include "components/input/native_web_keyboard_event.h"
#include "media/base/media_switches.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"
#include "net/cookies/canonical_cookie.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/aura/window.h"
#include "ui/aura/window_observer.h"
#include "ui/aura/window_tree_host.h"
#include "ui/compositor/layer_type.h"
#include "ui/display/win/screen_win.h"
#include "ui/events/event.h"
#include "ui/gfx/geometry/rect.h"
#include "url/gurl.h"
#include "url/origin.h"
#include "url/url_constants.h"

namespace browser_core::platform {

namespace {

using Microsoft::WRL::Callback;
using Microsoft::WRL::ComPtr;

// A callback WebView2's handlers can share, which must be copyable: the first to answer runs it.
template <typename T>
using Held = scoped_refptr<base::RefCountedData<T>>;
template <typename T>
Held<T> Hold(T value) {
  return base::MakeRefCounted<base::RefCountedData<T>>(std::move(value));
}

std::string Utf8(const wchar_t* text) {
  return text ? base::WideToUTF8(text) : std::string();
}

// How long a page has to answer for its localStorage, before its tab goes back to Chromium
// without it.
constexpr base::TimeDelta kStorageWait = base::Seconds(2);

// Whether the OS has WebView2's Evergreen runtime. Asked once: one installed later counts from the
// next launch.
bool RuntimeInstalled() {
  static const bool installed = [] {
    base::win::ScopedCoMem<wchar_t> version;
    return SUCCEEDED(GetAvailableCoreWebView2BrowserVersionString(nullptr, &version)) &&
           version.get() != nullptr;
  }();
  return installed;
}

// The hidden window a WebView2 waits in while no pane shows it: a controller needs a parent.
HWND Parking() {
  static HWND const parking =
      CreateWindowExW(WS_EX_TOOLWINDOW, L"STATIC", L"", WS_POPUP, 0, 0, 0, 0, nullptr, nullptr,
                      GetModuleHandle(nullptr), nullptr);
  return parking;
}

// WebView2's environment, one for the browser: one Edge browser process, with a WebView2 profile
// for each store, in a folder of ilo's own profile directory.
class Engine {
 public:
  using Ready = base::OnceCallback<void(ICoreWebView2Environment10*)>;

  static Engine& Get() {
    static base::NoDestructor<Engine> engine;
    return *engine;
  }

  // Runs |ready| with the environment, started the first time for |why|, or with null if it
  // couldn't start.
  void Use(const char* why, Ready ready) {
    if (environment_) {
      std::move(ready).Run(environment_.Get());
      return;
    }
    waiting_.push_back(std::move(ready));
    if (waiting_.size() > 1) {
      return;
    }
    // check.sh reads this line, as the Mac's.
    fprintf(stderr, "browser: system engine loaded for %s\n", why);
    base::FilePath folder;
    base::PathService::Get(chrome::DIR_USER_DATA, &folder);
    folder = folder.Append(FILE_PATH_LITERAL("System Engine"));
    const HRESULT result = CreateCoreWebView2EnvironmentWithOptions(
        nullptr, folder.value().c_str(), nullptr,
        Callback<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler>(
            [](HRESULT result, ICoreWebView2Environment* made) -> HRESULT {
              Engine::Get().Made(result, made);
              return S_OK;
            })
            .Get());
    if (FAILED(result)) {
      Made(result, nullptr);
    }
  }

  // Its browser process went: the next use starts another.
  void Lost() { environment_.Reset(); }

 private:
  void Made(HRESULT result, ICoreWebView2Environment* made) {
    ComPtr<ICoreWebView2Environment10> environment;
    if (SUCCEEDED(result) && made) {
      made->QueryInterface(IID_PPV_ARGS(&environment));
    }
    if (!environment) {
      LOG(ERROR) << "browser: system engine failed: 0x" << std::hex
                 << static_cast<uint32_t>(result);
    }
    environment_ = environment;
    for (Ready& ready : std::exchange(waiting_, {})) {
      std::move(ready).Run(environment.Get());
    }
  }

  ComPtr<ICoreWebView2Environment10> environment_;
  std::vector<Ready> waiting_;
};

using ControllerMade = base::OnceCallback<void(ComPtr<ICoreWebView2Controller>)>;

// Makes a hidden WebView2 in |store|'s profile, in private mode for a private session's, which
// keeps nothing on disk, then runs |made| with it, or with null.
void MakeController(const SystemStore& store, const char* why, ControllerMade made) {
  Engine::Get().Use(
      why, base::BindOnce(
               [](std::wstring profile, bool persistent, ControllerMade made,
                  ICoreWebView2Environment10* environment) {
                 ComPtr<ICoreWebView2ControllerOptions> options;
                 if (!environment ||
                     FAILED(environment->CreateCoreWebView2ControllerOptions(&options))) {
                   std::move(made).Run(nullptr);
                   return;
                 }
                 options->put_ProfileName(profile.c_str());
                 options->put_IsInPrivateModeEnabled(!persistent);
                 auto held = Hold(std::move(made));
                 const HRESULT result = environment->CreateCoreWebView2ControllerWithOptions(
                     Parking(), options.Get(),
                     Callback<ICoreWebView2CreateCoreWebView2ControllerCompletedHandler>(
                         [held](HRESULT result, ICoreWebView2Controller* controller) -> HRESULT {
                           if (controller) {
                             controller->put_IsVisible(FALSE);
                           }
                           ComPtr<ICoreWebView2Controller> made;
                           if (SUCCEEDED(result)) {
                             made = controller;
                           }
                           if (held->data) {
                             std::move(held->data).Run(std::move(made));
                           }
                           return S_OK;
                         })
                         .Get());
                 if (FAILED(result) && held->data) {
                   std::move(held->data).Run(nullptr);
                 }
               },
               base::UTF8ToWide(store.id), store.persistent, std::move(made)));
}

// One store's own hidden WebView2, which holds its profile: its cookies are read and written, and
// its data cleared, with no page of it open.
class Store {
 public:
  using Ready = base::OnceCallback<void(Store*)>;

  explicit Store(std::string id) : id_(std::move(id)) {}
  Store(const Store&) = delete;
  Store& operator=(const Store&) = delete;
  ~Store() {
    if (keeper_) {
      keeper_->Close();
    }
    // Later, as the store goes inside the map's erase: whatever waited gets nothing.
    for (Ready& ready : std::exchange(waiting_, {})) {
      base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
          FROM_HERE, base::BindOnce(std::move(ready), static_cast<Store*>(nullptr)));
    }
  }

  static std::map<std::string, std::unique_ptr<Store>>& All() {
    static base::NoDestructor<std::map<std::string, std::unique_ptr<Store>>> stores;
    return *stores;
  }

  // Runs |ready| with |store|'s, made the first time with the system's engine loaded for |why|,
  // or with null if it can't be.
  static void Use(const SystemStore& store, const char* why, Ready ready) {
    std::unique_ptr<Store>& entry = All()[store.id];
    if (!entry) {
      entry = std::make_unique<Store>(store.id);
    }
    Store* each = entry.get();
    if (each->cookies_) {
      std::move(ready).Run(each);
      return;
    }
    each->waiting_.push_back(std::move(ready));
    if (each->waiting_.size() > 1) {
      return;
    }
    MakeController(store, why,
                   base::BindOnce(
                       [](base::WeakPtr<Store> store, ComPtr<ICoreWebView2Controller> controller) {
                         if (store) {
                           store->Made(std::move(controller));
                         } else if (controller) {
                           controller->Close();
                         }
                       },
                       each->weak_factory_.GetWeakPtr()));
  }

  ICoreWebView2CookieManager* cookies() { return cookies_.Get(); }
  ICoreWebView2Profile* profile() { return profile_.Get(); }

 private:
  void Made(ComPtr<ICoreWebView2Controller> controller) {
    keeper_ = controller;
    ComPtr<ICoreWebView2> web;
    ComPtr<ICoreWebView2_2> web2;
    ComPtr<ICoreWebView2_13> web13;
    if (controller && SUCCEEDED(controller->get_CoreWebView2(&web)) && SUCCEEDED(web.As(&web2)) &&
        SUCCEEDED(web.As(&web13))) {
      web2->get_CookieManager(&cookies_);
      web13->get_Profile(&profile_);
    }
    std::vector<Ready> waiting = std::exchange(waiting_, {});
    if (!cookies_ || !profile_) {
      // Gone, so the next use tries again.
      const std::string id = id_;
      All().erase(id);
      for (Ready& ready : waiting) {
        std::move(ready).Run(nullptr);
      }
      return;
    }
    base::WeakPtr<Store> self = weak_factory_.GetWeakPtr();
    for (Ready& ready : waiting) {
      std::move(ready).Run(self.get());
    }
  }

  const std::string id_;
  ComPtr<ICoreWebView2Controller> keeper_;
  ComPtr<ICoreWebView2CookieManager> cookies_;
  ComPtr<ICoreWebView2Profile> profile_;
  std::vector<Ready> waiting_;
  base::WeakPtrFactory<Store> weak_factory_{this};
};

// |cookie| as WebView2's store takes it, or null for one it can't hold: it has no partitions.
ComPtr<ICoreWebView2Cookie> WebViewCookie(ICoreWebView2CookieManager* manager,
                                          const net::CanonicalCookie& cookie) {
  ComPtr<ICoreWebView2Cookie> made;
  // A host-only cookie's domain has no dot, which WebView2 takes as host-only too.
  if (cookie.IsPartitioned() ||
      FAILED(manager->CreateCookie(base::UTF8ToWide(cookie.Name()).c_str(),
                                   base::UTF8ToWide(cookie.Value()).c_str(),
                                   base::UTF8ToWide(cookie.Domain()).c_str(),
                                   base::UTF8ToWide(cookie.Path()).c_str(), &made))) {
    return nullptr;
  }
  made->put_IsSecure(cookie.SecureAttribute());
  made->put_IsHttpOnly(cookie.IsHttpOnly());
  if (cookie.IsPersistent()) {
    made->put_Expires(cookie.ExpiryDate().InSecondsFSinceUnixEpoch());
  }
  switch (cookie.SameSite()) {
    case net::CookieSameSite::STRICT_MODE:
      made->put_SameSite(COREWEBVIEW2_COOKIE_SAME_SITE_KIND_STRICT);
      break;
    case net::CookieSameSite::LAX_MODE:
      made->put_SameSite(COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX);
      break;
    case net::CookieSameSite::NO_RESTRICTION:
      made->put_SameSite(COREWEBVIEW2_COOKIE_SAME_SITE_KIND_NONE);
      break;
    default:
      // Unspecified keeps WebView2's default, which treats it as Chromium does.
      break;
  }
  return made;
}

// |cookie| as Chromium's store takes it, or null for one it wouldn't.
std::unique_ptr<net::CanonicalCookie> ChromiumCookie(ICoreWebView2Cookie* cookie) {
  base::win::ScopedCoMem<wchar_t> name, value, domain_text, path;
  BOOL secure = FALSE, http_only = FALSE, session = TRUE;
  double expires = -1;
  COREWEBVIEW2_COOKIE_SAME_SITE_KIND kind = COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX;
  if (FAILED(cookie->get_Name(&name)) || FAILED(cookie->get_Value(&value)) ||
      FAILED(cookie->get_Domain(&domain_text)) || FAILED(cookie->get_Path(&path))) {
    return nullptr;
  }
  cookie->get_IsSecure(&secure);
  cookie->get_IsHttpOnly(&http_only);
  cookie->get_IsSession(&session);
  cookie->get_Expires(&expires);
  cookie->get_SameSite(&kind);
  const base::Time now = base::Time::Now();
  const base::Time expiry =
      session ? base::Time() : base::Time::FromSecondsSinceUnixEpoch(expires);
  if (!expiry.is_null() && expiry <= now) {
    return nullptr;
  }
  const std::string domain = Utf8(domain_text.get());
  const bool host_only = !domain.starts_with('.');
  const GURL url(std::string(secure ? "https://" : "http://") +
                 (host_only ? domain : domain.substr(1)) + "/");
  const net::CookieSameSite same_site =
      kind == COREWEBVIEW2_COOKIE_SAME_SITE_KIND_STRICT ? net::CookieSameSite::STRICT_MODE
      : kind == COREWEBVIEW2_COOKIE_SAME_SITE_KIND_LAX  ? net::CookieSameSite::LAX_MODE
                                                        : net::CookieSameSite::NO_RESTRICTION;
  return net::CanonicalCookie::CreateSanitizedCookie(
      url, Utf8(name.get()), Utf8(value.get()), host_only ? std::string() : domain,
      Utf8(path.get()), now, expiry, now, secure, http_only, same_site,
      net::COOKIE_PRIORITY_DEFAULT, /*partition_key=*/std::nullopt, /*status=*/nullptr);
}

// The cookies in |list| of |site| and of the hosts under it.
std::vector<ComPtr<ICoreWebView2Cookie>> OfSite(ICoreWebView2CookieList* list,
                                                std::string_view site) {
  std::vector<ComPtr<ICoreWebView2Cookie>> of_site;
  UINT count = 0;
  if (!list || FAILED(list->get_Count(&count))) {
    return of_site;
  }
  for (UINT i = 0; i < count; ++i) {
    ComPtr<ICoreWebView2Cookie> cookie;
    base::win::ScopedCoMem<wchar_t> domain_text;
    if (FAILED(list->GetValueAtIndex(i, &cookie)) || FAILED(cookie->get_Domain(&domain_text))) {
      continue;
    }
    const std::string domain = Utf8(domain_text.get());
    std::string_view host = domain;
    if (host.starts_with('.')) {
      host.remove_prefix(1);
    }
    if (HostOfSite(host, site)) {
      of_site.push_back(std::move(cookie));
    }
  }
  return of_site;
}

// A script for the first load of a page, in the browser's world at each document's start, that
// sets the items |storage| has for the document's origin and no other's. Arrays, not an object, so
// no key the page used can name a prototype.
std::optional<std::string> StorageScript(const StorageByOrigin& storage) {
  base::Value::List origins;
  for (const auto& [origin, items] : storage) {
    base::Value::List pairs;
    for (const auto& [key, value] : items) {
      pairs.Append(base::Value::List().Append(key).Append(value));
    }
    origins.Append(base::Value::List().Append(origin.Serialize()).Append(std::move(pairs)));
  }
  std::string json;
  if (!base::JSONWriter::Write(origins, &json)) {
    return std::nullopt;
  }
  return base::StrCat({"(() => {\n  const items = new Map(", json,
                       ").get(location.origin) || [];\n"
                       "  for (const [key, value] of items) {\n"
                       "    try { localStorage.setItem(key, value); } catch (e) {}\n"
                       "  }\n"
                       "})();"});
}

// The bytes |stream| holds, written from its start.
std::optional<std::vector<uint8_t>> Bytes(IStream* stream) {
  STATSTG stat = {};
  HGLOBAL global = nullptr;
  if (FAILED(stream->Stat(&stat, STATFLAG_NONAME)) || stat.cbSize.QuadPart == 0 ||
      FAILED(GetHGlobalFromStream(stream, &global))) {
    return std::nullopt;
  }
  const auto* start = static_cast<const uint8_t*>(GlobalLock(global));
  if (!start) {
    return std::nullopt;
  }
  std::vector<uint8_t> bytes(start, start + stat.cbSize.QuadPart);
  GlobalUnlock(global);
  return bytes;
}

bool SameSite(const GURL& a, const GURL& b) {
  return net::registry_controlled_domains::SameDomainOrHost(
      a, b, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
}

// What the page's main frame may show: the web, and the blank and blob pages it makes itself.
bool Shown(const GURL& url) {
  return url.SchemeIsHTTPOrHTTPS() || url.SchemeIs(url::kAboutScheme) || url.SchemeIsBlob();
}

class SystemPageWin : public SystemPage, public aura::WindowObserver {
 public:
  SystemPageWin(const SystemStore& store, const GURL& url, Delegate* delegate)
      : delegate_(delegate), first_url_(url), window_(std::make_unique<aura::Window>(nullptr)) {
    // The pane's place for WebView2's own window, which draws over it.
    window_->Init(ui::LAYER_NOT_DRAWN);
    window_->SetName("SystemPage");
    window_->AddObserver(this);
    MakeController(store, "a page",
                   base::BindOnce(
                       [](base::WeakPtr<SystemPageWin> page,
                          ComPtr<ICoreWebView2Controller> controller) {
                         if (page) {
                           page->Made(std::move(controller));
                         } else if (controller) {
                           controller->Close();
                         }
                       },
                       weak_factory_.GetWeakPtr()));
  }

  ~SystemPageWin() override {
    window_->RemoveObserver(this);
    if (!controller_) {
      return;
    }
    Park();
    // Closed on the next turn: the tab can go inside a call the page made, such as Ctrl+W passed
    // on from its key, with WebView2's handler still on the stack.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce([](ComPtr<ICoreWebView2Controller> controller) {
                     controller->Close();
                   },
                                  std::move(controller_)));
  }

  // SystemPage:
  gfx::NativeView view() override { return window_.get(); }

  void Focus() override {
    wants_focus_ = true;
    if (controller_ && window_->GetRootWindow()) {
      controller_->MoveFocus(COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);
    }
  }

  void Start(StorageByOrigin storage) override {
    if (started_) {
      return;
    }
    started_ = true;
    storage_ = std::move(storage);
    if (web_) {
      Begin();
    }
  }

  void GetStorage(base::OnceCallback<void(url::Origin, StorageItems)> done) override {
    // Whichever comes first, the page's answer or the wait's end, answers.
    auto held = Hold(std::move(done));
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostDelayedTask(
        FROM_HERE,
        base::BindOnce(
            [](Held<base::OnceCallback<void(url::Origin, StorageItems)>> held) {
              if (held->data) {
                std::move(held->data).Run(url::Origin(), StorageItems());
              }
            },
            held),
        kStorageWait);
    const url::Origin origin = url::Origin::Create(url());
    if (!web_ || origin.opaque()) {
      std::move(held->data).Run(url::Origin(), StorageItems());
      return;
    }
    base::Value::Dict id;
    id.Set("securityOrigin", origin.Serialize());
    id.Set("isLocalStorage", true);
    base::Value::Dict params;
    params.Set("storageId", std::move(id));
    // DevTools' protocol, as WebView2 has no other way to read an origin's storage without running
    // a script in the page's world.
    Cdp("DOMStorage.enable", base::Value::Dict(), base::DoNothing());
    Cdp("DOMStorage.getDOMStorageItems", std::move(params),
        base::BindOnce(
            [](Held<base::OnceCallback<void(url::Origin, StorageItems)>> held, url::Origin origin,
               std::optional<base::Value::Dict> answer) {
              if (!held->data) {
                return;
              }
              const base::Value::List* entries = answer ? answer->FindList("entries") : nullptr;
              if (!entries) {
                std::move(held->data).Run(url::Origin(), StorageItems());
                return;
              }
              StorageItems items;
              for (const base::Value& entry : *entries) {
                const base::Value::List* pair = entry.GetIfList();
                if (pair && pair->size() == 2 && (*pair)[0].is_string() &&
                    (*pair)[1].is_string()) {
                  items[base::UTF8ToUTF16((*pair)[0].GetString())] =
                      base::UTF8ToUTF16((*pair)[1].GetString());
                }
              }
              std::move(held->data).Run(origin, std::move(items));
            },
            held, origin));
  }

  void Snapshot(base::OnceCallback<void(std::optional<std::vector<uint8_t>>)> done) override {
    ComPtr<IStream> stream;
    if (!web_ || FAILED(CreateStreamOnHGlobal(nullptr, TRUE, &stream))) {
      std::move(done).Run(std::nullopt);
      return;
    }
    auto held = Hold(std::move(done));
    // A PNG at the view's pixels, which the display's scale sets, in sRGB as Chromium's pages are.
    const HRESULT result = web_->CapturePreview(
        COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, stream.Get(),
        Callback<ICoreWebView2CapturePreviewCompletedHandler>(
            [held, stream](HRESULT result) -> HRESULT {
              if (held->data) {
                std::move(held->data).Run(SUCCEEDED(result) ? Bytes(stream.Get()) : std::nullopt);
              }
              return S_OK;
            })
            .Get());
    if (FAILED(result) && held->data) {
      std::move(held->data).Run(std::nullopt);
    }
  }

  // Until its first document commits, the URL it will load.
  GURL url() override {
    base::win::ScopedCoMem<wchar_t> source;
    if (!committed_once_ || !web_ || FAILED(web_->get_Source(&source))) {
      return first_url_;
    }
    return GURL(Utf8(source.get()));
  }
  std::u16string title() override {
    base::win::ScopedCoMem<wchar_t> title;
    if (!web_ || FAILED(web_->get_DocumentTitle(&title)) || !title.get()) {
      return std::u16string();
    }
    return base::WideToUTF16(title.get());
  }
  bool loading() override { return !started_ || !navigated_ || loading_; }
  bool can_go_back() override {
    BOOL can = FALSE;
    return web_ && SUCCEEDED(web_->get_CanGoBack(&can)) && can;
  }
  bool can_go_forward() override {
    BOOL can = FALSE;
    return web_ && SUCCEEDED(web_->get_CanGoForward(&can)) && can;
  }
  // WebView2 says nothing of mixed content, which Chromium under it blocks or upgrades, and stops
  // at a certificate error: an https page is a secure one.
  bool secure() override { return url().SchemeIs(url::kHttpsScheme); }
  bool audible() override {
    ComPtr<ICoreWebView2_8> web;
    BOOL playing = FALSE;
    return web_ && SUCCEEDED(web_.As(&web)) &&
           SUCCEEDED(web->get_IsDocumentPlayingAudio(&playing)) && playing;
  }
  bool muted() override { return muted_; }

  void Load(const GURL& url) override {
    if (!navigated_) {
      first_url_ = url;
      return;
    }
    Navigate(url);
  }
  void GoBack() override {
    if (web_) {
      web_->GoBack();
    }
  }
  void GoForward() override {
    if (web_) {
      web_->GoForward();
    }
  }
  void Reload(bool bypass_cache) override {
    if (!web_) {
      return;
    }
    if (!bypass_cache) {
      web_->Reload();
      return;
    }
    // WebView2's own Reload has no way past the cache.
    base::Value::Dict params;
    params.Set("ignoreCache", true);
    Cdp("Page.reload", std::move(params), base::DoNothing());
  }
  void Stop() override {
    if (web_) {
      web_->Stop();
    }
  }

  void SetMuted(bool muted) override {
    muted_ = muted;
    ComPtr<ICoreWebView2_8> web;
    if (web_ && SUCCEEDED(web_.As(&web))) {
      web->put_IsMuted(muted);
    }
    delegate_->SystemPageChanged();
  }

  // aura::WindowObserver:
  void OnWindowAddedToRootWindow(aura::Window* window) override { Place(); }
  void OnWindowRemovingFromRootWindow(aura::Window* window, aura::Window* new_root) override {
    Park();
  }
  void OnWindowBoundsChanged(aura::Window* window,
                             const gfx::Rect& old_bounds,
                             const gfx::Rect& new_bounds,
                             ui::PropertyChangeReason reason) override {
    Place();
  }
  void OnWindowVisibilityChanged(aura::Window* window, bool visible) override { Place(); }

 private:
  void Made(ComPtr<ICoreWebView2Controller> controller) {
    if (!controller || FAILED(controller->get_CoreWebView2(&web_))) {
      LOG(ERROR) << "browser: system page not made for " << first_url_.spec();
      if (controller) {
        controller->Close();
      }
      return;
    }
    controller_ = controller;
    Configure();
    Listen();
    // As Chromium's pages are for the checks, or as the tab was muted before WebView2 came.
    if (muted_ || base::CommandLine::ForCurrentProcess()->HasSwitch(switches::kMuteAudio)) {
      SetMuted(true);
    }
    Place();
    if (wants_focus_) {
      Focus();
    }
    if (started_) {
      Begin();
    }
  }

  void Configure() {
    ComPtr<ICoreWebView2Settings> settings;
    if (FAILED(web_->get_Settings(&settings))) {
      return;
    }
    settings->put_IsStatusBarEnabled(FALSE);
    // The page talks to nothing of the browser's.
    settings->put_IsWebMessageEnabled(FALSE);
    settings->put_AreHostObjectsAllowed(FALSE);
    // The app's shortcuts, not WebView2's own find bar, print preview and reload keys.
    ComPtr<ICoreWebView2Settings3> keys;
    if (SUCCEEDED(settings.As(&keys))) {
      keys->put_AreBrowserAcceleratorKeysEnabled(FALSE);
    }
  }

  void Listen() {
    base::WeakPtr<SystemPageWin> weak = weak_factory_.GetWeakPtr();
    EventRegistrationToken token;
    web_->add_NavigationStarting(
        Callback<ICoreWebView2NavigationStartingEventHandler>(
            [weak](ICoreWebView2*, ICoreWebView2NavigationStartingEventArgs* args) -> HRESULT {
              if (weak) {
                weak->Starting(args);
              }
              return S_OK;
            })
            .Get(),
        &token);
    web_->add_ContentLoading(
        Callback<ICoreWebView2ContentLoadingEventHandler>(
            [weak](ICoreWebView2*, ICoreWebView2ContentLoadingEventArgs*) -> HRESULT {
              if (weak) {
                weak->Committed();
              }
              return S_OK;
            })
            .Get(),
        &token);
    web_->add_NavigationCompleted(
        Callback<ICoreWebView2NavigationCompletedEventHandler>(
            [weak](ICoreWebView2*, ICoreWebView2NavigationCompletedEventArgs*) -> HRESULT {
              if (weak) {
                weak->Completed();
              }
              return S_OK;
            })
            .Get(),
        &token);
    web_->add_SourceChanged(Callback<ICoreWebView2SourceChangedEventHandler>(
                                [weak](ICoreWebView2*, ICoreWebView2SourceChangedEventArgs*)
                                    -> HRESULT {
                                  if (weak) {
                                    weak->delegate_->SystemPageChanged();
                                  }
                                  return S_OK;
                                })
                                .Get(),
                            &token);
    web_->add_HistoryChanged(Callback<ICoreWebView2HistoryChangedEventHandler>(
                                 [weak](ICoreWebView2*, IUnknown*) -> HRESULT {
                                   if (weak) {
                                     weak->delegate_->SystemPageChanged();
                                   }
                                   return S_OK;
                                 })
                                 .Get(),
                             &token);
    web_->add_DocumentTitleChanged(Callback<ICoreWebView2DocumentTitleChangedEventHandler>(
                                       [weak](ICoreWebView2*, IUnknown*) -> HRESULT {
                                         if (weak) {
                                           weak->delegate_->SystemPageChanged();
                                         }
                                         return S_OK;
                                       })
                                       .Get(),
                                   &token);
    // A link with a target, or window.open: the tab opens it, as WebKit's page asks.
    web_->add_NewWindowRequested(
        Callback<ICoreWebView2NewWindowRequestedEventHandler>(
            [weak](ICoreWebView2*, ICoreWebView2NewWindowRequestedEventArgs* args) -> HRESULT {
              args->put_Handled(TRUE);
              base::win::ScopedCoMem<wchar_t> uri;
              const GURL url = SUCCEEDED(args->get_Uri(&uri)) ? GURL(Utf8(uri.get())) : GURL();
              if (weak && url.SchemeIsHTTPOrHTTPS()) {
                weak->Soon(base::BindOnce(
                    [](const GURL& url, Delegate* delegate) { delegate->SystemPageOpen(url); },
                    url));
              }
              return S_OK;
            })
            .Get(),
        &token);
    // It asks for nothing, as WebKit's page doesn't: Chromium's permissions aren't its. Autoplay
    // keeps WebView2's own answer, which plays a video the user started.
    web_->add_PermissionRequested(
        Callback<ICoreWebView2PermissionRequestedEventHandler>(
            [](ICoreWebView2*, ICoreWebView2PermissionRequestedEventArgs* args) -> HRESULT {
              COREWEBVIEW2_PERMISSION_KIND kind;
              if (SUCCEEDED(args->get_PermissionKind(&kind)) &&
                  kind != COREWEBVIEW2_PERMISSION_KIND_AUTOPLAY) {
                args->put_State(COREWEBVIEW2_PERMISSION_STATE_DENY);
              }
              return S_OK;
            })
            .Get(),
        &token);
    web_->add_ProcessFailed(
        Callback<ICoreWebView2ProcessFailedEventHandler>(
            [weak](ICoreWebView2*, ICoreWebView2ProcessFailedEventArgs* args) -> HRESULT {
              COREWEBVIEW2_PROCESS_FAILED_KIND kind;
              if (weak && SUCCEEDED(args->get_ProcessFailedKind(&kind))) {
                weak->Failed(kind);
              }
              return S_OK;
            })
            .Get(),
        &token);
    // Nor does it download, nor hand a link to another app.
    if (ComPtr<ICoreWebView2_4> web; SUCCEEDED(web_.As(&web))) {
      web->add_DownloadStarting(
          Callback<ICoreWebView2DownloadStartingEventHandler>(
              [](ICoreWebView2*, ICoreWebView2DownloadStartingEventArgs* args) -> HRESULT {
                args->put_Cancel(TRUE);
                LOG(WARNING) << "browser: system page refused a download";
                return S_OK;
              })
              .Get(),
          &token);
    }
    if (ComPtr<ICoreWebView2_18> web; SUCCEEDED(web_.As(&web))) {
      web->add_LaunchingExternalUriScheme(
          Callback<ICoreWebView2LaunchingExternalUriSchemeEventHandler>(
              [](ICoreWebView2*, ICoreWebView2LaunchingExternalUriSchemeEventArgs* args)
                  -> HRESULT {
                args->put_Cancel(TRUE);
                return S_OK;
              })
              .Get(),
          &token);
    }
    if (ComPtr<ICoreWebView2_8> web; SUCCEEDED(web_.As(&web))) {
      web->add_IsDocumentPlayingAudioChanged(
          Callback<ICoreWebView2IsDocumentPlayingAudioChangedEventHandler>(
              [weak](ICoreWebView2*, IUnknown*) -> HRESULT {
                if (weak) {
                  weak->delegate_->SystemPageChanged();
                }
                return S_OK;
              })
              .Get(),
          &token);
    }
    controller_->add_AcceleratorKeyPressed(
        Callback<ICoreWebView2AcceleratorKeyPressedEventHandler>(
            [weak](ICoreWebView2Controller*, ICoreWebView2AcceleratorKeyPressedEventArgs* args)
                -> HRESULT {
              if (weak) {
                weak->Key(args);
              }
              return S_OK;
            })
            .Get(),
        &token);
    // WebView2 says nothing of a click in its window, only that it took the keyboard, which a
    // click in the page does.
    controller_->add_GotFocus(Callback<ICoreWebView2FocusChangedEventHandler>(
                                  [weak](ICoreWebView2Controller*, IUnknown*) -> HRESULT {
                                    if (weak) {
                                      weak->Soon(base::BindOnce(&Delegate::SystemPageClicked));
                                    }
                                    return S_OK;
                                  })
                                  .Get(),
                              &token);
  }

  // Has the first load's storage script in place, then starts the load.
  void Begin() {
    std::optional<std::string> script =
        storage_.empty() ? std::nullopt : StorageScript(storage_);
    storage_.clear();
    if (!script) {
      Navigate(first_url_);
      return;
    }
    base::Value::Dict params;
    params.Set("source", std::move(*script));
    params.Set("worldName", "ilo");
    Cdp("Page.enable", base::Value::Dict(), base::DoNothing());
    Cdp("Page.addScriptToEvaluateOnNewDocument", std::move(params),
        base::BindOnce(
            [](base::WeakPtr<SystemPageWin> page, std::optional<base::Value::Dict> answer) {
              if (!page) {
                return;
              }
              if (const std::string* id = answer ? answer->FindString("identifier") : nullptr) {
                page->seed_script_ = *id;
              }
              page->Navigate(page->first_url_);
            },
            weak_factory_.GetWeakPtr()));
  }

  void Navigate(const GURL& url) {
    if (!web_) {
      return;
    }
    navigated_ = true;
    own_navigation_ = true;
    web_->Navigate(base::UTF8ToWide(url.spec()).c_str());
  }

  // Runs |method| of DevTools' protocol on the page with |params|, then |done| with its answer, or
  // nothing if it failed.
  void Cdp(const char* method,
           base::Value::Dict params,
           base::OnceCallback<void(std::optional<base::Value::Dict>)> done) {
    std::string json;
    base::JSONWriter::Write(params, &json);
    auto held = Hold(std::move(done));
    const HRESULT result = web_->CallDevToolsProtocolMethod(
        base::UTF8ToWide(method).c_str(), base::UTF8ToWide(json).c_str(),
        Callback<ICoreWebView2CallDevToolsProtocolMethodCompletedHandler>(
            [held](HRESULT result, LPCWSTR answer) -> HRESULT {
              if (held->data) {
                std::move(held->data)
                    .Run(SUCCEEDED(result) && answer ? base::JSONReader::ReadDict(Utf8(answer))
                                                     : std::nullopt);
              }
              return S_OK;
            })
            .Get());
    if (FAILED(result) && held->data) {
      std::move(held->data).Run(std::nullopt);
    }
  }

  // Shows WebView2's window over the pane's place, in the window that shows the pane.
  void Place() {
    if (!controller_) {
      return;
    }
    aura::WindowTreeHost* host = window_->GetHost();
    if (!host) {
      Park();
      return;
    }
    HWND parent = host->GetAcceleratedWidget();
    if (parent != parent_) {
      controller_->put_ParentWindow(parent);
      parent_ = parent;
    }
    const float scale = display::win::GetScreenWin()->GetScaleFactorForHWND(parent);
    controller_->put_Bounds(
        gfx::ScaleToEnclosingRect(window_->GetBoundsInRootWindow(), scale).ToRECT());
    controller_->put_IsVisible(window_->IsVisible());
  }

  // Hides it in the parking window while no pane shows it, as the pane's window may go first.
  void Park() {
    if (!controller_ || parent_ == Parking()) {
      return;
    }
    controller_->put_IsVisible(FALSE);
    controller_->put_ParentWindow(Parking());
    parent_ = Parking();
  }

  // Whether the main frame starts loading a navigation's URL: a link the user followed to
  // another site doesn't, as the tab takes it. A redirect, the tab's own load, history and a
  // reload load where they go.
  void Starting(ICoreWebView2NavigationStartingEventArgs* args) {
    base::win::ScopedCoMem<wchar_t> uri;
    BOOL user = FALSE, redirect = FALSE;
    args->get_Uri(&uri);
    args->get_IsUserInitiated(&user);
    args->get_IsRedirected(&redirect);
    COREWEBVIEW2_NAVIGATION_KIND kind = COREWEBVIEW2_NAVIGATION_KIND_NEW_DOCUMENT;
    if (ComPtr<ICoreWebView2NavigationStartingEventArgs3> more; SUCCEEDED(args->QueryInterface(
            IID_PPV_ARGS(&more)))) {
      more->get_NavigationKind(&kind);
    }
    const GURL url(Utf8(uri.get()));
    // WebView2 counts its host's Navigate as the user's.
    const bool own = !redirect && std::exchange(own_navigation_, false);
    if (!Shown(url)) {
      LOG(WARNING) << "browser: system page refused " << url.possibly_invalid_spec();
      args->put_Cancel(TRUE);
      return;
    }
    // WebView2 doesn't tell a link from a form or a script the user's click ran, so each of those
    // to another site leaves too.
    if (user && !redirect && !own && kind == COREWEBVIEW2_NAVIGATION_KIND_NEW_DOCUMENT &&
        url.SchemeIsHTTPOrHTTPS() && committed_once_ && !SameSite(url, this->url())) {
      args->put_Cancel(TRUE);
      Soon(base::BindOnce(
          [](const GURL& url, Delegate* delegate) { delegate->SystemPageLeft(url); }, url));
      return;
    }
    loading_ = true;
    delegate_->SystemPageChanged();
  }

  void Committed() {
    committed_ = true;
    committed_once_ = true;
    Soon(base::BindOnce(&Delegate::SystemPageCommitted));
  }

  void Completed() {
    loading_ = false;
    // A navigation cancelled before its commit leaves the page it was on.
    if (std::exchange(committed_, false)) {
      Seeded();
      Soon(base::BindOnce(&Delegate::SystemPageLoaded));
    }
    delegate_->SystemPageChanged();
  }

  // The first load ended, past its commit: the storage it took is set, and later loads take none.
  void Seeded() {
    if (seed_script_.empty()) {
      return;
    }
    base::Value::Dict params;
    params.Set("identifier", std::exchange(seed_script_, std::string()));
    Cdp("Page.removeScriptToEvaluateOnNewDocument", std::move(params), base::DoNothing());
  }

  void Failed(COREWEBVIEW2_PROCESS_FAILED_KIND kind) {
    if (kind == COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED) {
      // Every WebView2 went with it, and the stores' with them: the next use starts another.
      LOG(ERROR) << "browser: system engine gone at " << url().spec();
      Engine::Get().Lost();
      Store::All().clear();
      return;
    }
    if (kind == COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED) {
      // check.sh waits for this line, an error as Chromium's tab's is.
      LOG(ERROR) << "browser: system page gone at " << url().spec();
      web_->Reload();
    }
  }

  // WebView2 hands over a key the browser may want before the page sees it, and never one the
  // page didn't keep, nor what has the keyboard in the page. So a shortcut the app doesn't reserve
  // is its own too as it goes down, with Ctrl or Alt held, and |editing| keeps Undo and Redo with
  // the page.
  void Key(ICoreWebView2AcceleratorKeyPressedEventArgs* args) {
    COREWEBVIEW2_KEY_EVENT_KIND kind;
    UINT key = 0;
    INT lparam = 0;
    if (FAILED(args->get_KeyEventKind(&kind)) || FAILED(args->get_VirtualKey(&key)) ||
        FAILED(args->get_KeyEventLParam(&lparam)) ||
        (kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN &&
         kind != COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN)) {
      return;
    }
    CHROME_MSG message = {};
    message.hwnd = parent_;
    message.message = kind == COREWEBVIEW2_KEY_EVENT_KIND_SYSTEM_KEY_DOWN ? WM_SYSKEYDOWN
                                                                           : WM_KEYDOWN;
    message.wParam = key;
    message.lParam = lparam;
    message.time = GetMessageTime();
    const ui::KeyEvent event(message);
    const input::NativeWebKeyboardEvent native(event);
    // The app's shortcut can close the tab, and this with it.
    base::WeakPtr<SystemPageWin> alive = weak_factory_.GetWeakPtr();
    bool used = delegate_->SystemPageKey(native, /*before_page=*/true, /*editing=*/true);
    if (!used && alive &&
        (native.GetModifiers() &
         (blink::WebInputEvent::kControlKey | blink::WebInputEvent::kAltKey))) {
      used = delegate_->SystemPageKey(native, /*before_page=*/false, /*editing=*/true);
    }
    if (used || !alive) {
      args->put_Handled(TRUE);
    }
  }

  // Tells the tab from a task of its own: WebView2 is part way through its call, and the tab may
  // let the page go.
  void Soon(base::OnceCallback<void(Delegate*)> call) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<SystemPageWin> page,
                          base::OnceCallback<void(Delegate*)> call) {
                         if (page) {
                           std::move(call).Run(page->delegate_.get());
                         }
                       },
                       weak_factory_.GetWeakPtr(), std::move(call)));
  }

  const raw_ptr<Delegate> delegate_;
  // What the first load loads, once Start has the page's storage in place.
  GURL first_url_;
  std::unique_ptr<aura::Window> window_;
  ComPtr<ICoreWebView2Controller> controller_;
  ComPtr<ICoreWebView2> web_;
  // The window WebView2's is in: the pane's window's, or the parking window.
  HWND parent_ = nullptr;
  StorageByOrigin storage_;
  // The first load's storage script, by DevTools' id, until that load ends.
  std::string seed_script_;
  bool started_ = false;
  bool navigated_ = false;
  // The next navigation to start is the tab's own load.
  bool own_navigation_ = false;
  bool loading_ = false;
  // A document committed since the navigation began, and one ever has.
  bool committed_ = false;
  bool committed_once_ = false;
  bool wants_focus_ = false;
  bool muted_ = false;
  base::WeakPtrFactory<SystemPageWin> weak_factory_{this};
};

}  // namespace

// static
bool SystemPage::Available() {
  const int needs = PlayReadyNeeds();
  return (needs == BC_PLAYREADY_WINDOWS_11 || needs == BC_PLAYREADY_HARDWARE) &&
         RuntimeInstalled();
}

// static
bool SystemPage::OpensListedSites() {
  // Windows 10, where Chromium's PlayReady plays nothing.
  return PlayReadyNeeds() == BC_PLAYREADY_WINDOWS_11 && Available();
}

// static
bool SystemPage::Offered(std::string_view system, bool private_window) {
  if (!Available()) {
    return false;
  }
  if (PlayReadyNeeds() == BC_PLAYREADY_WINDOWS_11) {
    return true;
  }
  // A PC without PlayReady's hardware, where WebView2's PlayReady plays what Chromium's refused,
  // but not in a private window, whose store keeps nothing.
  return !private_window && (system.empty() || system == "com.microsoft.playready");
}

// static
std::unique_ptr<SystemPage> SystemPage::Create(const SystemStore& store,
                                               const GURL& url,
                                               Delegate* delegate) {
  if (!Available()) {
    return nullptr;
  }
  return std::make_unique<SystemPageWin>(store, url, delegate);
}

// static
void SystemPage::DeleteStore(const std::string& id) {
  if (!RuntimeInstalled()) {
    return;
  }
  // WebView2 deletes the profile's folder once its browser process lets it go.
  Store::Use({id, /*persistent=*/true}, "deleting a profile's store",
             base::BindOnce(
                 [](std::string id, Store* store) {
                   ComPtr<ICoreWebView2Profile8> profile;
                   if (store &&
                       SUCCEEDED(store->profile()->QueryInterface(IID_PPV_ARGS(&profile)))) {
                     profile->Delete();
                   }
                   Store::All().erase(id);
                 },
                 id));
}

// static
void SystemPage::ReleaseStore(const std::string& id) {
  Store::All().erase(id);
}

// static
void SystemPage::ClearSite(const SystemStore& store,
                           const std::string& domain,
                           bool cookies,
                           base::OnceClosure done) {
  if (!RuntimeInstalled()) {
    std::move(done).Run();
    return;
  }
  Store::Use(
      store, "clearing a site",
      base::BindOnce(
          [](std::string domain, bool cookies, base::OnceClosure done, Store* store) {
            if (!store) {
              std::move(done).Run();
              return;
            }
            auto held = Hold(std::move(done));
            HRESULT result = E_FAIL;
            if (cookies) {
              ComPtr<ICoreWebView2CookieManager> manager = store->cookies();
              result = manager->GetCookies(
                  L"", Callback<ICoreWebView2GetCookiesCompletedHandler>(
                           [held, manager, domain](HRESULT result,
                                                   ICoreWebView2CookieList* list) -> HRESULT {
                             if (SUCCEEDED(result)) {
                               for (const ComPtr<ICoreWebView2Cookie>& cookie :
                                    OfSite(list, domain)) {
                                 manager->DeleteCookie(cookie.Get());
                               }
                             }
                             if (held->data) {
                               std::move(held->data).Run();
                             }
                             return S_OK;
                           })
                           .Get());
            } else if (ComPtr<ICoreWebView2Profile2> profile;
                       SUCCEEDED(store->profile()->QueryInterface(IID_PPV_ARGS(&profile)))) {
              // WebView2 clears a profile's whole cache, as it keeps none by site.
              result = profile->ClearBrowsingData(
                  COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
                  Callback<ICoreWebView2ClearBrowsingDataCompletedHandler>(
                      [held](HRESULT) -> HRESULT {
                        if (held->data) {
                          std::move(held->data).Run();
                        }
                        return S_OK;
                      })
                      .Get());
            }
            if (FAILED(result) && held->data) {
              std::move(held->data).Run();
            }
          },
          domain, cookies, std::move(done)));
}

// static
void SystemPage::SetCookies(const SystemStore& store,
                            const std::vector<net::CanonicalCookie>& cookies,
                            base::OnceCallback<void(size_t)> done) {
  if (cookies.empty() || !RuntimeInstalled()) {
    std::move(done).Run(0);
    return;
  }
  Store::Use(store, "copying cookies",
             base::BindOnce(
                 [](std::vector<net::CanonicalCookie> cookies,
                    base::OnceCallback<void(size_t)> done, Store* store) {
                   size_t took = 0;
                   for (const net::CanonicalCookie& cookie : cookies) {
                     ComPtr<ICoreWebView2Cookie> made =
                         store ? WebViewCookie(store->cookies(), cookie) : nullptr;
                     if (made && SUCCEEDED(store->cookies()->AddOrUpdateCookie(made.Get()))) {
                       ++took;
                     }
                   }
                   std::move(done).Run(took);
                 },
                 cookies, std::move(done)));
}

// static
void SystemPage::GetCookies(const SystemStore& store,
                            const std::string& site,
                            base::OnceCallback<void(std::vector<net::CanonicalCookie>)> done) {
  if (!RuntimeInstalled()) {
    std::move(done).Run({});
    return;
  }
  using Done = base::OnceCallback<void(std::vector<net::CanonicalCookie>)>;
  Store::Use(
      store, "copying cookies",
      base::BindOnce(
          [](std::string site, Done done, Store* store) {
            if (!store) {
              std::move(done).Run({});
              return;
            }
            auto held = Hold(std::move(done));
            const HRESULT result = store->cookies()->GetCookies(
                L"", Callback<ICoreWebView2GetCookiesCompletedHandler>(
                         [held, site](HRESULT result, ICoreWebView2CookieList* list) -> HRESULT {
                           if (!held->data) {
                             return S_OK;
                           }
                           UINT all = 0;
                           std::vector<ComPtr<ICoreWebView2Cookie>> matched;
                           if (SUCCEEDED(result) && list) {
                             list->get_Count(&all);
                             matched = OfSite(list, site);
                           }
                           std::vector<net::CanonicalCookie> of_site;
                           for (const ComPtr<ICoreWebView2Cookie>& cookie : matched) {
                             if (std::unique_ptr<net::CanonicalCookie> made =
                                     ChromiumCookie(cookie.Get())) {
                               of_site.push_back(std::move(*made));
                             }
                           }
                           // Counts only: never a name or a value. With the copy back's own line,
                           // it says which step dropped a cookie.
                           fprintf(stderr,
                                   "browser: system store has %u cookies, %zu of %s, %zu Chromium "
                                   "takes\n",
                                   all, matched.size(), site.c_str(), of_site.size());
                           std::move(held->data).Run(std::move(of_site));
                           return S_OK;
                         })
                         .Get());
            if (FAILED(result) && held->data) {
              std::move(held->data).Run({});
            }
          },
          site, std::move(done)));
}

}  // namespace browser_core::platform
