#ifndef BROWSER_CORE_SEARCH_H_
#define BROWSER_CORE_SEARCH_H_

#include <string>
#include <string_view>

#include "url/gurl.h"

class Profile;

namespace browser_core {

// Registers the pref SetUpSearch keeps in each profile. Before the first profile is made.
void RegisterSearchPrefs();
// Loads |profile|'s engines, whose default is DuckDuckGo until the user, a policy or an extension
// picks one (search_default.h).
void SetUpSearch(Profile* profile);

// The page Chrome's address bar goes to for |text| typed in it: the one it names, else a search for
// it with the default search engine. Invalid if it names neither.
GURL TypedUrl(Profile* profile, std::string_view text);

// The page |text| names if it's typed with http:// or https://, whatever its host, else invalid.
// Chrome's address bar searches for a host that can't resolve, as a .invalid one can't (RFC 6761),
// but a scheme typed says it's an address, and one that doesn't resolve shows the error page.
GURL TypedWithScheme(std::string_view text);

// A search for |text| with |profile|'s search engine whose keyword is |keyword|, or its default one
// if |keyword| is empty. Invalid if it has none.
GURL SearchUrl(Profile* profile, const std::u16string& keyword, std::string_view text);

}  // namespace browser_core

#endif  // BROWSER_CORE_SEARCH_H_
