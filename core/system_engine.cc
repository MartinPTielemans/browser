#include "browser_core/system_engine.h"

#include <algorithm>
#include <memory>
#include <string>
#include <utility>

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
    registry->RegisterStringPref(kStore, std::string());
  }
};

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
    Tab* tab = contents ? Tab::FromWebContents(contents) : nullptr;
    if (!tab || !navigation->IsInPrimaryMainFrame()) {
      return PROCEED;
    }
    // Its page is the system's, and Chromium's waits, as when Chrome reloads a discarded tab.
    if (tab->system_page()) {
      return CANCEL_AND_IGNORE;
    }
    const GURL& url = navigation->GetURL();
    Profile* profile = Profile::FromBrowserContext(contents->GetBrowserContext());
    // A form's answer stays: the system's engine would load it without what was sent.
    if (navigation->IsPost() || !url.SchemeIsHTTPOrHTTPS() || !Allowed(Tab::IdOf(contents)) ||
        !Remembered(profile, url)) {
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
                       contents->GetWeakPtr(), url));
    return CANCEL_AND_IGNORE;
  }
};

}  // namespace

void RegisterProfilePrefs() {
  PrefsFactory::GetInstance();
}

bool Available() {
  return platform::SystemPage::Available();
}

bool Allowed(bc_id tab) {
  const Space* space = Space::Holding(tab);
  return Available() && !(space && space->agent());
}

std::string SiteOf(const GURL& url) {
  const std::string domain = net::registry_controlled_domains::GetDomainAndRegistry(
      url, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
  // An IP address or a bare host name is a site of its own, as the core clears sites.
  return domain.empty() ? std::string(url.host()) : domain;
}

bool Remembered(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS() || !Available()) {
    return false;
  }
  const base::ListValue& sites = profile->GetPrefs()->GetList(kSites);
  return !sites.empty() && std::ranges::contains(sites, base::Value(SiteOf(url)));
}

void Remember(Profile* profile, const GURL& url) {
  if (!profile || !url.SchemeIsHTTPOrHTTPS() || Remembered(profile, url)) {
    return;
  }
  ScopedListPrefUpdate(profile->GetPrefs(), kSites)->Append(SiteOf(url));
}

void Forget(Profile* profile, const GURL& url) {
  if (profile && url.SchemeIsHTTPOrHTTPS()) {
    ScopedListPrefUpdate(profile->GetPrefs(), kSites)->EraseValue(base::Value(SiteOf(url)));
  }
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
