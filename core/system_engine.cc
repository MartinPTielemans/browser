#include "browser_core/system_engine.h"

#include <algorithm>
#include <memory>
#include <string>
#include <string_view>
#include <utility>

#include "base/command_line.h"
#include "base/containers/fixed_flat_set.h"
#include "base/files/file_util.h"
#include "base/functional/bind.h"
#include "base/json/json_reader.h"
#include "base/hash/sha1.h"
#include "base/no_destructor.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/string_util.h"
#include "base/supports_user_data.h"
#include "base/task/single_thread_task_runner.h"
#include "base/task/thread_pool.h"
#include "base/uuid.h"
#include "base/values.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/browser_process.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/profiles/profile_keyed_service_factory.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/common/chrome_constants.h"
#include "components/keyed_service/core/keyed_service.h"
#include "components/pref_registry/pref_registry_syncable.h"
#include "components/prefs/pref_service.h"
#include "components/prefs/scoped_user_pref_update.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/navigation_throttle.h"
#include "content/public/browser/navigation_throttle_registry.h"
#include "content/public/browser/web_contents.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"

namespace browser_core::system_engine {

namespace {

// The sites a profile keeps for the system's engine, and the id of the store its pages made, if
// they have: the system's engine is only loaded for a profile with one.
constexpr char kSites[] = "browser_core.system_engine.sites";
// The sites whose protected-media note the user closed.
constexpr char kClosedNotes[] = "browser_core.system_engine.closed_notes";
// The listed services the user turned off, by their entries in kListed.
constexpr char kListedOff[] = "browser_core.system_engine.listed_off";
constexpr char kStore[] = "browser_core.system_engine.store";

// Builds no service: Chrome registers a profile's prefs through its factories.
class PrefsFactory : public ProfileKeyedServiceFactory {
 public:
  static PrefsFactory* GetInstance() {
    static base::NoDestructor<PrefsFactory> instance;
    return instance.get();
  }

  PrefsFactory() : ProfileKeyedServiceFactory("BrowserCoreSystemEngine") {}

 private:
  std::unique_ptr<KeyedService> BuildServiceInstanceForBrowserContext(
      content::BrowserContext* context) const override {
    return nullptr;
  }

  void RegisterProfilePrefs(user_prefs::PrefRegistrySyncable* registry) override {
    registry->RegisterListPref(kSites);
    registry->RegisterListPref(kClosedNotes);
    registry->RegisterListPref(kListedOff);
    registry->RegisterStringPref(kStore, std::string());
  }
};

// The streaming services whose main content is protected video or audio, which open in the system's
// engine from the first visit. Add an entry only for such a service, as its own host, or as its
// registrable domain where nothing else is at that domain: each matches its host and subdomains.
constexpr auto kListed = base::MakeFixedFlatSet<std::string_view>({
    "disneyplus.com",
    "hbomax.com",
    "hulu.com",
    "max.com",
    "netflix.com",
    "open.spotify.com",
    "paramountplus.com",
    "peacocktv.com",
    "primevideo.com",
    "tv.apple.com",
});

// For the app's check, which never loads a real service: a host of its own server, listed as the
// services are. Only a dev build of the app passes it (main.rs).
constexpr char kTestHostSwitch[] = "streaming-test-host";

// |url|'s entry in kListed, else empty: a lookup for each of its host's few labels.
std::string_view ListedEntry(const GURL& url) {
  static const base::NoDestructor<std::string> test_host(
      base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(kTestHostSwitch));
  std::string_view host = url.host_piece();
  if (!test_host->empty() && host == *test_host) {
    return *test_host;
  }
  while (!host.empty()) {
    if (auto entry = kListed.find(host); entry != kListed.end()) {
      return *entry;
    }
    const size_t dot = host.find('.');
    host = dot == std::string_view::npos ? std::string_view() : host.substr(dot + 1);
  }
  return std::string_view();
}

// A private session's store, which keeps nothing on disk and goes with the session's profile.
class PrivateStore : public base::SupportsUserData::Data {
 public:
  static constexpr char kKey[] = "browser_core_private_system_store";

  ~PrivateStore() override { platform::SystemPage::ReleaseStore(id); }

  const std::string id = base::Uuid::GenerateRandomV4().AsLowercaseString();
};

// The store of the profile at |path|: a UUID made from the path, version 5's way, so the profile
// has the same one at each launch and one deleted unloaded can still be found.
std::string StoreIdFor(const base::FilePath& path) {
  std::string digest = base::SHA1HashString("ilo system store " + path.AsUTF8Unsafe());
  digest[6] = static_cast<char>((digest[6] & 0x0f) | 0x50);
  digest[8] = static_cast<char>((digest[8] & 0x3f) | 0x80);
  const std::string hex = base::ToLowerASCII(base::HexEncode(digest.substr(0, 16)));
  return hex.substr(0, 8) + "-" + hex.substr(8, 4) + "-" + hex.substr(12, 4) + "-" +
         hex.substr(16, 4) + "-" + hex.substr(20, 12);
}

// The id of the store the profile at |path| made, read from its preferences on disk, for a
// profile that isn't loaded. Empty for none.
std::string StoreOnDisk(const base::FilePath& path) {
  std::string json;
  if (!base::ReadFileToString(path.Append(chrome::kPreferencesFilename), &json)) {
    return std::string();
  }
  std::optional<base::DictValue> prefs = base::JSONReader::ReadDict(json, base::JSON_PARSE_RFC);
  const std::string* id = prefs ? prefs->FindStringByDottedPath(kStore) : nullptr;
  return id ? *id : std::string();
}

void DeleteStoreOf(const std::string& id) {
  if (!id.empty()) {
    platform::SystemPage::DeleteStore(id);
  }
}

class Throttle : public content::NavigationThrottle {
 public:
  using content::NavigationThrottle::NavigationThrottle;

  ThrottleCheckResult WillStartRequest() override { return Check(); }
  ThrottleCheckResult WillRedirectRequest() override { return Check(); }
  const char* GetNameForLogging() override { return "BrowserCoreSystemEngineThrottle"; }

 private:
  ThrottleCheckResult Check() {
    content::NavigationHandle* navigation = navigation_handle();
    content::WebContents* contents = navigation->GetWebContents();
    if (!contents || !navigation->IsInPrimaryMainFrame()) {
      return PROCEED;
    }
    if (Tab* tab = Tab::FromWebContents(contents)) {
      return Decide(tab);
    }
    // A tab Chrome makes starts its first load before the core has the tab or its window has
    // placed it in a Space, as ⌘T's does: one going to a site kept for the system's engine waits
    // a turn for them. Any other goes on at once, so new tabs cost nothing more.
    if (!Kept()) {
      return PROCEED;
    }
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Throttle::Placed, weak_factory_.GetWeakPtr()));
    return DEFER;
  }

  // Whether the navigation goes to a site the profile keeps for the system's engine, and could
  // load there.
  bool Kept() {
    content::NavigationHandle* navigation = navigation_handle();
    const GURL& url = navigation->GetURL();
    // A form's answer stays: the system's engine would load it without what was sent.
    return !navigation->IsPost() && url.SchemeIsHTTPOrHTTPS() &&
           Remembered(Profile::FromBrowserContext(navigation->GetWebContents()->GetBrowserContext()),
                      url);
  }

  // The deferred load, once its tab is made and placed. A page of no window's goes on in
  // Chromium.
  void Placed() {
    content::WebContents* contents = navigation_handle()->GetWebContents();
    Tab* tab = contents ? Tab::FromWebContents(contents) : nullptr;
    const ThrottleCheckResult result = tab ? Decide(tab) : PROCEED;
    if (result.action() == PROCEED) {
      Resume();
    } else {
      // Last: the navigation, and this with it, can go here.
      CancelDeferredNavigation(result);
    }
  }

  ThrottleCheckResult Decide(Tab* tab) {
    // Its page is the system's, and Chromium's waits, as when Chrome reloads a discarded tab.
    if (tab->system_page()) {
      return CANCEL_AND_IGNORE;
    }
    // It loads the page it waits for once shown, as a tab unloaded from a system page does, not the
    // page Chromium kept, which it reloads then, and which may be at a site kept for the system's
    // engine.
    if (tab->waiting()) {
      return CANCEL_AND_IGNORE;
    }
    content::WebContents* contents = navigation_handle()->GetWebContents();
    if (!Kept() || !Allowed(Tab::IdOf(contents))) {
      return PROCEED;
    }
    // Posted: the navigation is part way through.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<content::WebContents> contents, const GURL& url) {
                         if (Tab* tab = contents ? Tab::FromWebContents(contents.get()) : nullptr) {
                           tab->UseSystemEngine(url);
                         }
                       },
                       contents->GetWeakPtr(), navigation_handle()->GetURL()));
    return CANCEL_AND_IGNORE;
  }

  base::WeakPtrFactory<Throttle> weak_factory_{this};
};

}  // namespace

void RegisterProfilePrefs() {
  PrefsFactory::GetInstance();
}

bool Available() {
  return platform::SystemPage::Available();
}

bool Allowed(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  // Placed or about to be: a tab Chrome just made is in no Space yet.
  const Space* space = contents ? Window::SpaceOf(contents) : Space::Holding(tab);
  return Available() && !(space && space->agent());
}

std::string SiteOf(const GURL& url) {
  const std::string domain = net::registry_controlled_domains::GetDomainAndRegistry(
      url, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
  // An IP address or a bare host name is a site of its own, as the core clears sites.
  return domain.empty() ? std::string(url.host()) : domain;
}

bool Listed(const GURL& url) {
  return url.SchemeIsHTTPOrHTTPS() && !ListedEntry(url).empty();
}

bool Remembered(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS() || !Available()) {
    return false;
  }
  if (const std::string_view entry = ListedEntry(url); !entry.empty()) {
    const base::ListValue& off = profile->GetPrefs()->GetList(kListedOff);
    if (!std::ranges::contains(off, base::Value(entry))) {
      return true;
    }
  }
  const base::ListValue& sites = profile->GetPrefs()->GetList(kSites);
  return !sites.empty() && std::ranges::contains(sites, base::Value(SiteOf(url)));
}

void Remember(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS()) {
    return;
  }
  // A listed service is never kept: turning it on takes back the user's off.
  if (const std::string_view entry = ListedEntry(url); !entry.empty()) {
    ScopedListPrefUpdate(profile->GetPrefs(), kListedOff)->EraseValue(base::Value(entry));
    return;
  }
  if (!Remembered(profile, url)) {
    ScopedListPrefUpdate(profile->GetPrefs(), kSites)->Append(SiteOf(url));
  }
}

void Forget(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS()) {
    return;
  }
  ScopedListPrefUpdate(profile->GetPrefs(), kSites)->EraseValue(base::Value(SiteOf(url)));
  const std::string_view entry = ListedEntry(url);
  if (!entry.empty() &&
      !std::ranges::contains(profile->GetPrefs()->GetList(kListedOff), base::Value(entry))) {
    ScopedListPrefUpdate(profile->GetPrefs(), kListedOff)->Append(std::string(entry));
  }
}

bool NoteClosed(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS()) {
    return false;
  }
  const base::ListValue& sites = profile->GetPrefs()->GetList(kClosedNotes);
  return !sites.empty() && std::ranges::contains(sites, base::Value(SiteOf(url)));
}

void CloseNote(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS() || NoteClosed(profile, url)) {
    return;
  }
  ScopedListPrefUpdate(profile->GetPrefs(), kClosedNotes)->Append(SiteOf(url));
}

platform::SystemStore StoreOf(Profile* profile) {
  if (profile->IsOffTheRecord()) {
    if (!profile->GetUserData(PrivateStore::kKey)) {
      profile->SetUserData(PrivateStore::kKey, std::make_unique<PrivateStore>());
    }
    return {static_cast<PrivateStore*>(profile->GetUserData(PrivateStore::kKey))->id, false};
  }
  return {StoreIdFor(profile->GetPath()), true};
}

void StoreMade(Profile* profile) {
  if (!profile->IsOffTheRecord()) {
    profile->GetPrefs()->SetString(kStore, StoreIdFor(profile->GetPath()));
  }
}

bool StoreUsed(Profile* profile) {
  return profile->IsOffTheRecord() ? profile->GetUserData(PrivateStore::kKey) != nullptr
                                   : !profile->GetPrefs()->GetString(kStore).empty();
}

void DeleteStore(const base::FilePath& path, base::OnceClosure then) {
  ProfileManager* manager = g_browser_process->profile_manager();
  if (Profile* loaded = manager ? manager->GetProfileByPath(path) : nullptr) {
    DeleteStoreOf(loaded->GetPrefs()->GetString(kStore));
    std::move(then).Run();
    return;
  }
  base::ThreadPool::PostTaskAndReplyWithResult(
      FROM_HERE, {base::MayBlock(), base::TaskPriority::USER_VISIBLE},
      base::BindOnce(&StoreOnDisk, path),
      base::BindOnce(
          [](base::OnceClosure then, const std::string& id) {
            DeleteStoreOf(id);
            std::move(then).Run();
          },
          std::move(then)));
}

void CreateThrottlesForNavigation(content::NavigationThrottleRegistry& registry) {
  if (Available() && registry.GetNavigationHandle().IsInPrimaryMainFrame()) {
    registry.AddThrottle(std::make_unique<Throttle>(registry));
  }
}

}  // namespace browser_core::system_engine
