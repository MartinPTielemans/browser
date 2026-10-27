#include "browser_core/platform/system_page.h"

#include <utility>

#include "base/functional/callback.h"

namespace browser_core::platform {

// No system engine yet off the Mac: WebView2 on Windows comes later (decision 1).

bool SystemPage::Available() {
  return false;
}

std::unique_ptr<SystemPage> SystemPage::Create(const SystemStore&, const GURL&, Delegate*) {
  return nullptr;
}

void SystemPage::DeleteStore(const std::string&) {}

void SystemPage::ReleaseStore(const std::string&) {}

void SystemPage::ClearSite(const SystemStore&, const std::string&, bool, base::OnceClosure done) {
  std::move(done).Run();
}

}  // namespace browser_core::platform
