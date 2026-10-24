#include "browser_core/suggest.h"

#include <algorithm>
#include <string>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/autocomplete/chrome_autocomplete_provider_client.h"
#include "chrome/browser/autocomplete/chrome_autocomplete_scheme_classifier.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/search_engines/template_url_service_factory.h"
#include "components/omnibox/browser/autocomplete_enums.h"
#include "components/omnibox/browser/autocomplete_input.h"
#include "components/omnibox/browser/autocomplete_match.h"
#include "components/omnibox/browser/autocomplete_provider.h"
#include "components/omnibox/browser/autocomplete_result.h"
#include "third_party/metrics_proto/omnibox_event.pb.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// Chrome's address bar's sources that read only what's on the device. Search, with search
// suggestions off, adds just the search for what was typed.
constexpr int kProviders = AutocompleteProvider::TYPE_BOOKMARK | AutocompleteProvider::TYPE_BUILTIN |
                           AutocompleteProvider::TYPE_HISTORY_QUICK |
                           AutocompleteProvider::TYPE_HISTORY_URL |
                           AutocompleteProvider::TYPE_SEARCH | AutocompleteProvider::TYPE_SHORTCUTS;

// As Chrome matches a suggestion to an open tab: by URL, without what doesn't change the page.
GURL Stripped(const GURL& url, const TemplateURLService* engines) {
  return AutocompleteMatch::GURLToStrippedGURL(url, AutocompleteInput(), engines,
                                               /*keyword=*/std::u16string(),
                                               /*keep_search_intent_params=*/true);
}

// A tab at |url|, stripped, other than |shown|, first in the Space |window| shows. 0 if none.
bc_id TabAt(const GURL& url, const Window& window, bc_id shown, const TemplateURLService* engines) {
  std::vector<Space*> spaces = Space::All();
  std::ranges::stable_partition(spaces, [&](Space* space) { return space->id() == window.space(); });
  for (Space* space : spaces) {
    for (const std::vector<bc_id>* tabs : {&space->pinned(), &space->today()}) {
      for (bc_id tab : *tabs) {
        content::WebContents* contents = Tab::FromId(tab);
        if (tab != shown && contents && Stripped(Tab::PageUrl(contents), engines) == url) {
          return tab;
        }
      }
    }
  }
  return 0;
}

}  // namespace

Suggestions::Suggestions(bc_id window, Profile* profile)
    : window_(window),
      profile_(profile),
      classifier_(std::make_unique<ChromeAutocompleteSchemeClassifier>(profile)),
      controller_(std::make_unique<ChromeAutocompleteProviderClient>(profile),
                  AutocompleteControllerConfig{.provider_types = kProviders}) {
  observation_.Observe(&controller_);
}

Suggestions::~Suggestions() = default;

void Suggestions::Start(const std::u16string& text, bool complete) {
  text_ = text;
  if (text.empty()) {
    controller_.Stop(AutocompleteStopReason::kClobbered);
    Changed();
    return;
  }
  AutocompleteInput input(text, metrics::OmniboxEventProto::OTHER, *classifier_);
  input.set_prevent_inline_autocomplete(!complete);
  controller_.Start(input);
}

void Suggestions::OnResultChanged(AutocompleteController* controller, bool default_match_changed) {
  Changed();
}

void Suggestions::Changed() {
  if (report_posted_) {
    return;
  }
  report_posted_ = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Suggestions::Report, weak_factory_.GetWeakPtr()));
}

void Suggestions::Report() {
  report_posted_ = false;
  Window* window = Window::FromId(window_);
  if (!window || !Host().window_suggestions) {
    return;
  }
  const TemplateURLService* engines = TemplateURLServiceFactory::GetForProfile(profile_);
  struct Row {
    std::string url, text, detail, completion;
    bc_id tab = 0;
    bool typed_search = false;
  };
  std::vector<Row> rows;
  const AutocompleteResult& result = controller_.result();
  for (const AutocompleteMatch& match : result) {
    if (text_.empty() || !match.destination_url.is_valid()) {
      continue;
    }
    const bool search = AutocompleteMatch::IsSearchType(match.type);
    // A page by its title, then its URL; a search by its words, then its engine.
    const bool titled = !search && !match.description.empty();
    rows.push_back({
        .url = match.destination_url.spec(),
        .text = base::UTF16ToUTF8(titled ? match.description : match.contents),
        .detail = base::UTF16ToUTF8(titled ? match.contents : match.description),
        .completion = &match == result.default_match()
                          ? base::UTF16ToUTF8(match.inline_autocompletion)
                          : std::string(),
        // The first goes where what was typed goes, as ↩ does in Chrome's address bar.
        .tab = search || rows.empty() ? 0
                                      : TabAt(Stripped(match.destination_url, engines), *window,
                                              window->selected(), engines),
        .typed_search = match.type == AutocompleteMatchType::SEARCH_WHAT_YOU_TYPED,
    });
  }
  // As Dia lists them: the search for what was typed right after the top row.
  if (auto search = std::ranges::find_if(rows, &Row::typed_search);
      search != rows.end() && search != rows.begin()) {
    std::rotate(rows.begin() + 1, search, search + 1);
  }
  std::vector<bc_suggestion> suggestions;
  for (const Row& row : rows) {
    suggestions.push_back({.url = row.url.c_str(),
                           .text = row.text.c_str(),
                           .detail = row.detail.c_str(),
                           .tab = row.tab,
                           .completion = row.completion.c_str()});
  }
  const std::string text = base::UTF16ToUTF8(text_);
  Host().window_suggestions(Host().context, window_, text.c_str(), suggestions.data(),
                            suggestions.size());
}

}  // namespace browser_core
