#include "browser_core/notes.h"

#include <memory>

#include "base/no_destructor.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/profiles/profile_keyed_service_factory.h"
#include "components/keyed_service/core/keyed_service.h"
#include "components/pref_registry/pref_registry_syncable.h"
#include "components/prefs/pref_service.h"

namespace browser_core::notes {

namespace {

constexpr char kService[] = "browser_core.notes.service";

// Builds no service: Chrome registers a profile's prefs through its factories.
class PrefsFactory : public ProfileKeyedServiceFactory {
 public:
  static PrefsFactory* GetInstance() {
    static base::NoDestructor<PrefsFactory> instance;
    return instance.get();
  }

  PrefsFactory() : ProfileKeyedServiceFactory("BrowserCoreNotes") {}

 private:
  std::unique_ptr<KeyedService> BuildServiceInstanceForBrowserContext(
      content::BrowserContext* context) const override {
    return nullptr;
  }

  void RegisterProfilePrefs(user_prefs::PrefRegistrySyncable* registry) override {
    registry->RegisterStringPref(kService, std::string());
  }
};

}  // namespace

void RegisterProfilePrefs() {
  PrefsFactory::GetInstance();
}

std::string Service(Profile* profile) {
  return profile->GetPrefs()->GetString(kService);
}

void SetService(Profile* profile, const std::string& service) {
  profile->GetPrefs()->SetString(kService, service);
}

}  // namespace browser_core::notes
