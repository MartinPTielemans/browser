#include "browser_core/favicon.h"

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/task/cancelable_task_tracker.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/profiles.h"
#include "chrome/browser/favicon/favicon_service_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "components/favicon/core/favicon_service.h"
#include "components/favicon_base/favicon_types.h"
#include "components/keyed_service/core/service_access_type.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// 16 points at 2×, as tab_favicon_changed's.
constexpr int kFaviconPixels = 32;

void Report(const std::string& url, const favicon_base::FaviconRawBitmapResult& result) {
  if (!Host().page_favicon) {
    return;
  }
  const bool found = result.is_valid();
  Host().page_favicon(Host().context, url.c_str(), found ? result.bitmap_data->data() : nullptr,
                      found ? result.bitmap_data->size() : 0);
}

}  // namespace

void FindFavicon(const std::string& url) {
  // The history of the profile the core started with.
  Profile* profile = StartProfile();
  favicon::FaviconService* service =
      profile ? FaviconServiceFactory::GetForProfile(profile, ServiceAccessType::EXPLICIT_ACCESS)
              : nullptr;
  if (!service) {
    // Posted, as the service's answer is, so the app hears of it once its call has returned.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Report, url, favicon_base::FaviconRawBitmapResult()));
    return;
  }
  static base::NoDestructor<base::CancelableTaskTracker> tracker;
  service->GetRawFaviconForPageURL(GURL(url), {favicon_base::IconType::kFavicon}, kFaviconPixels,
                                   /*fallback_to_host=*/true, base::BindOnce(&Report, url),
                                   tracker.get());
}

}  // namespace browser_core
