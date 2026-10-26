#include "browser_core/search.h"

#include <vector>

#include "base/callback_list.h"
#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "chrome/browser/autocomplete/autocomplete_classifier_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/search_engines/template_url_service_factory.h"
#include "components/omnibox/browser/autocomplete_classifier.h"
#include "components/omnibox/browser/autocomplete_match.h"
#include "components/search_engines/default_search_manager.h"
#include "components/search_engines/template_url.h"
#include "components/search_engines/template_url_service.h"
#include "third_party/metrics_proto/omnibox_event.pb.h"
#include "third_party/search_engines_data/resources/definitions/prepopulated_engines.h"

namespace browser_core {

namespace {

// ungoogled-chromium's engine in Google's place, which "searches" by going to http://<the words>.
constexpr char16_t kNoSearch[] = u"nosearch";

void PickDefaultEngine(TemplateURLService* engines) {
  const TemplateURL* engine = engines->GetDefaultSearchProvider();
  // FROM_FALLBACK until the user, a policy or an extension picks one.
  if (!engine || engine->keyword() != kNoSearch ||
      engines->default_search_provider_source() != DefaultSearchManager::FROM_FALLBACK) {
    return;
  }
  for (TemplateURL* candidate : engines->GetTemplateURLs()) {
    if (candidate->prepopulate_id() == TemplateURLPrepopulateData::duckduckgo.id) {
      engines->SetUserSelectedDefaultSearchProvider(candidate);
      return;
    }
  }
}

}  // namespace

void SetUpSearch(Profile* profile) {
  TemplateURLService* engines = TemplateURLServiceFactory::GetForProfile(profile);
  if (!engines) {
    return;
  }
  if (engines->loaded()) {
    PickDefaultEngine(engines);
    return;
  }
  // The engines load from the profile's database, after which the default can change. The service
  // lives as long as the profile, which the core keeps loaded.
  static base::NoDestructor<std::vector<base::CallbackListSubscription>> loaded;
  loaded->push_back(engines->RegisterOnLoadedCallback(
      base::BindOnce(&PickDefaultEngine, base::Unretained(engines))));
  engines->Load();
}

GURL TypedUrl(Profile* profile, std::string_view text) {
  AutocompleteMatch match;
  // As Chrome's paste and go does: the address bar's choice without completing what was typed.
  AutocompleteClassifierFactory::GetForProfile(profile)->Classify(
      base::UTF8ToUTF16(text), /*in_keyword_mode=*/false, /*allow_exact_keyword_match=*/false,
      metrics::OmniboxEventProto::OTHER, &match, /*alternate_nav_url=*/nullptr);
  return match.destination_url;
}

GURL SearchUrl(Profile* profile, const std::u16string& keyword, std::string_view text) {
  TemplateURLService* engines = TemplateURLServiceFactory::GetForProfile(profile);
  const TemplateURL* engine = !engines         ? nullptr
                              : keyword.empty() ? engines->GetDefaultSearchProvider()
                                                : engines->GetTemplateURLForKeyword(keyword);
  if (!engine) {
    return GURL();
  }
  return GURL(engine->url_ref().ReplaceSearchTerms(
      TemplateURLRef::SearchTermsArgs(base::UTF8ToUTF16(text)), engines->search_terms_data()));
}

}  // namespace browser_core
