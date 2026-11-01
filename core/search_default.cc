#include "browser_core/search_default.h"

#include <memory>

#include "base/values.h"
#include "components/pref_registry/pref_registry_syncable.h"
#include "components/prefs/pref_service.h"
#include "components/search_engines/default_search_manager.h"
#include "components/search_engines/search_engines_pref_names.h"
#include "components/search_engines/template_url_data.h"
#include "components/search_engines/template_url_data_util.h"
#include "third_party/search_engines_data/resources/definitions/prepopulated_engines.h"

namespace browser_core {

namespace {

constexpr char kPickForgotten[] = "browser_core.search_pick_forgotten";

}  // namespace

void RegisterSearchDefaultPrefs(user_prefs::PrefRegistrySyncable* registry) {
  registry->RegisterBooleanPref(kPickForgotten, false);
}

void ForgetPickedDefault(PrefService* prefs) {
  if (prefs->GetBoolean(kPickForgotten)) {
    return;
  }
  prefs->SetBoolean(kPickForgotten, true);
  const base::Value* picked =
      prefs->GetUserPrefValue(DefaultSearchManager::kDefaultSearchProviderDataPrefName);
  std::unique_ptr<TemplateURLData> engine =
      picked && picked->is_dict() ? TemplateURLDataFromDictionary(picked->GetDict()) : nullptr;
  if (!engine || engine->prepopulate_id != TemplateURLPrepopulateData::duckduckgo.id) {
    return;
  }
  // Chrome reads a cleared choice as one it reset, and would tell the user so.
  const bool reset = prefs->GetBoolean(prefs::kUnacknowledgedDefaultSearchEngineResetOccurred);
  prefs->ClearPref(DefaultSearchManager::kDefaultSearchProviderDataPrefName);
  prefs->SetBoolean(prefs::kUnacknowledgedDefaultSearchEngineResetOccurred, reset);
}

}  // namespace browser_core
