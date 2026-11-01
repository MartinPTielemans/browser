#ifndef BROWSER_CORE_SEARCH_DEFAULT_H_
#define BROWSER_CORE_SEARCH_DEFAULT_H_

class PrefService;

namespace user_prefs {
class PrefRegistrySyncable;
}

namespace browser_core {

// The profile pref ForgetPickedDefault keeps.
void RegisterSearchDefaultPrefs(user_prefs::PrefRegistrySyncable* registry);

// DuckDuckGo is the default engine (engine/patches/ours/chromium-search-default-duckduckgo.patch).
// Before, the core picked it for a profile as though the user had, and a recommended
// DefaultSearchProvider policy never outranks the user. This forgets that pick, once a profile, so
// a pick the user makes after it stays theirs.
void ForgetPickedDefault(PrefService* prefs);

}  // namespace browser_core

#endif  // BROWSER_CORE_SEARCH_DEFAULT_H_
