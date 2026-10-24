#ifndef BROWSER_CORE_POPULAR_SITES_H_
#define BROWSER_CORE_POPULAR_SITES_H_

#include <string_view>
#include <vector>

namespace browser_core {

// A well-known site, which the command bar lists under the profile's own suggestions, so a new
// profile's command bar isn't bare.
struct PopularSite {
  std::string_view name;
  std::string_view url;
  // Another name it goes by, or "".
  std::string_view also;
};

// The sites |text| could be the start of: of their host without "www.", of a word of their name or
// of the other name they go by. The most visited first.
std::vector<PopularSite> PopularSitesFor(std::u16string_view text);

}  // namespace browser_core

#endif  // BROWSER_CORE_POPULAR_SITES_H_
