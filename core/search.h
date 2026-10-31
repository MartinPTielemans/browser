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

// The engines the user picks |profile|'s default from, as Chrome's search engine settings list
// them, each "keyword\tname" on a line of its own. Empty until its engines have loaded, or while a
// policy or an extension picks the default.
std::string DefaultEngineChoices(Profile* profile);
// The keyword of |profile|'s default engine, or empty until its engines have loaded.
std::u16string DefaultEngine(Profile* profile);
// Makes the engine of |keyword| among DefaultEngineChoices |profile|'s default, and returns whether
// it did.
bool SetDefaultEngine(Profile* profile, const std::u16string& keyword);

}  // namespace browser_core

#endif  // BROWSER_CORE_SEARCH_H_
