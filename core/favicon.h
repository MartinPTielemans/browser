#ifndef BROWSER_CORE_FAVICON_H_
#define BROWSER_CORE_FAVICON_H_

#include <string>

namespace browser_core {

// Finds the favicon the profile's history has for the page at |url|, else for its host, and gives
// it to the app's page_favicon with |url| as the app wrote it.
void FindFavicon(const std::string& url);

}  // namespace browser_core

#endif  // BROWSER_CORE_FAVICON_H_
