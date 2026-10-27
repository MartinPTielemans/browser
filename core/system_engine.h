#ifndef BROWSER_CORE_SYSTEM_ENGINE_H_
#define BROWSER_CORE_SYSTEM_ENGINE_H_

#include <string>

#include "base/files/file_path.h"
#include "browser_core/platform/system_page.h"
#include "browser_core/public/browser_core.h"
#include "url/gurl.h"

class Profile;

namespace content {
class NavigationThrottleRegistry;
}

namespace browser_core::system_engine {

// The system's engine, WebKit on the Mac, shows a tab's page in place of Chromium's for a site
// whose protected video only it plays (decision 1). The user switches a tab to it once per site,
// from the protected-media note, and the profile keeps the site: its pages open in the system's
// engine from then on, until Site Controls turns it off. A tab goes back to Chromium at another
// site. Agents never act in it, as the browser can't keep their rules in an engine whose pages it
// doesn't control, and an agent's Space never switches.

// Registers what profiles keep for it. Before the first profile is made.
void RegisterProfilePrefs();

// Whether the platform has one.
bool Available();

// Whether |tab| may show its page in the system's engine: one of the user's tabs, not an agent's.
bool Allowed(bc_id tab);

// The site of |url| as the profile keeps it: its registrable domain, else its host.
std::string SiteOf(const GURL& url);

// Whether |profile| keeps |url|'s site for the system's engine. A private window's profile keeps
// what its profile does, and what it adds goes with its session.
bool Remembered(Profile* profile, const GURL& url);
void Remember(Profile* profile, const GURL& url);
void Forget(Profile* profile, const GURL& url);

// Where |profile|'s system pages keep their cookies: a store of its own, apart from Chromium's and
// Safari's, or for a private window's profile one that keeps nothing.
platform::SystemStore StoreOf(Profile* profile);
// Whether |profile| ever had a system page, so its store has anything to clear.
bool StoreUsed(Profile* profile);

// Deletes the store of the profile at |path|, as the profile goes.
void DeleteStore(const base::FilePath& path);

// For ContentBrowserClient::CreateThrottlesForNavigation: a navigation of a tab's page in
// Chromium to a site its profile keeps for the system's engine loads there instead, and a tab
// showing a system page loads nothing in Chromium.
void CreateThrottlesForNavigation(content::NavigationThrottleRegistry& registry);

}  // namespace browser_core::system_engine

#endif  // BROWSER_CORE_SYSTEM_ENGINE_H_
