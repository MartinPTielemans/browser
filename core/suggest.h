#ifndef BROWSER_CORE_SUGGEST_H_
#define BROWSER_CORE_SUGGEST_H_

#include <memory>
#include <string>

#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "browser_core/public/browser_core.h"
#include "components/omnibox/browser/autocomplete_controller.h"

class ChromeAutocompleteSchemeClassifier;
class Profile;

namespace browser_core {

// A window's command bar suggestions: what Chrome's address bar lists for the text typed, from the
// profile's history, bookmarks and search engines. Nothing is asked of the search engine, since
// ungoogled-chromium turns search suggestions off, so what's typed stays on the device.
class Suggestions : public AutocompleteController::Observer {
 public:
  Suggestions(bc_id window, Profile* profile);
  Suggestions(const Suggestions&) = delete;
  Suggestions& operator=(const Suggestions&) = delete;
  ~Suggestions() override;

  Profile* profile() const { return profile_; }

  // Lists suggestions for |text|, and again as more are found. Empty text lists none. |complete|
  // lets the first complete |text| inline.
  void Start(const std::u16string& text, bool complete);

 private:
  // AutocompleteController::Observer:
  void OnResultChanged(AutocompleteController* controller, bool default_match_changed) override;

  // Tells the app from a task of its own, so a burst of changes is one call, and none comes during
  // the app's call that made them.
  void Changed();
  void Report();

  const bc_id window_;
  const raw_ptr<Profile> profile_;
  std::unique_ptr<ChromeAutocompleteSchemeClassifier> classifier_;
  AutocompleteController controller_;
  std::u16string text_;
  bool report_posted_ = false;
  base::ScopedObservation<AutocompleteController, AutocompleteController::Observer> observation_{
      this};
  base::WeakPtrFactory<Suggestions> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SUGGEST_H_
