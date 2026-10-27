#ifndef BROWSER_CORE_ERROR_PAGE_H_
#define BROWSER_CORE_ERROR_PAGE_H_

#include <string>

#include "base/values.h"

namespace browser_core {

// The page a tab shows for a load that failed, in place of Chromium's neterror page, made from the
// strings Chromium made for its own (components/error_page/common/localized_error.cc): one line,
// the error's code under it and a Reload button where Chromium has one, and offline, ilo's game.
// Called in the renderer (engine/patches/ours/chromium-core-error-page.patch), so it needs nothing
// of the browser's.
std::string ErrorPageHtml(const base::DictValue& strings);

}  // namespace browser_core

#endif  // BROWSER_CORE_ERROR_PAGE_H_
