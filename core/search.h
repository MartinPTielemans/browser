#ifndef BROWSER_CORE_SEARCH_H_
#define BROWSER_CORE_SEARCH_H_

#include <string>
#include <string_view>

#include "url/gurl.h"

class Profile;

namespace browser_core {

// Has |profile| search with DuckDuckGo until the user picks an engine. ungoogled-chromium puts an
// engine that searches nowhere where Chrome's default was.
void SetUpSearch(Profile* profile);

// The page Chrome's address bar goes to for |text| typed in it: the one it names, else a search for
// it with the default search engine. Invalid if it names neither.
GURL TypedUrl(Profile* profile, std::string_view text);

// A search for |text| with |profile|'s search engine whose keyword is |keyword|, or its default one
// if |keyword| is empty. Invalid if it has none.
GURL SearchUrl(Profile* profile, const std::u16string& keyword, std::string_view text);

}  // namespace browser_core

#endif  // BROWSER_CORE_SEARCH_H_
