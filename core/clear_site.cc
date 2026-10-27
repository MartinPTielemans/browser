#include "browser_core/clear_site.h"

#include <memory>
#include <string>
#include <utility>

#include "base/scoped_observation.h"
#include "base/time/time.h"
#include "browser_core/tab.h"
#include "chrome/browser/profiles/profile.h"
#include "content/public/browser/browser_context.h"
#include "content/public/browser/browsing_data_filter_builder.h"
#include "content/public/browser/browsing_data_remover.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/reload_type.h"
#include "content/public/browser/web_contents.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// Loads the tab's page again once its site's data is gone, if the tab's still open, then goes.
class ReloadWhenCleared : public content::BrowsingDataRemover::Observer {
 public:
  ReloadWhenCleared(content::BrowsingDataRemover* remover, bc_id tab, bool bypass_cache)
      : tab_(tab), bypass_cache_(bypass_cache) {
    // The remover tells only the observers it knows when their task is done.
    observation_.Observe(remover);
  }

  void OnBrowsingDataRemoverDone(uint64_t failed_data_types) override {
    if (content::WebContents* contents = Tab::FromId(tab_)) {
      contents->GetController().Reload(
          bypass_cache_ ? content::ReloadType::BYPASSING_CACHE : content::ReloadType::NORMAL,
          /*check_for_repost=*/true);
    }
    delete this;
  }

 private:
  const bc_id tab_;
  const bool bypass_cache_;
  base::ScopedObservation<content::BrowsingDataRemover, content::BrowsingDataRemover::Observer>
      observation_{this};
};

}  // namespace

void ClearSiteAndReload(bc_id tab, bool cookies) {
  // A tab without its page has nothing to reload, and loads afresh once shown.
  const Tab* clearing = Tab::Get(tab);
  if (!clearing || !clearing->profile()) {
    return;
  }
  const GURL url = clearing->Url();
  if (!url.SchemeIsHTTPOrHTTPS()) {
    return;
  }
  // A site's cookies can be any of its hosts', so the filter takes the site's registrable domain,
  // as the Clear-Site-Data header does. An IP address or a bare host name is a site of its own.
  std::string domain = net::registry_controlled_domains::GetDomainAndRegistry(
      url, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
  if (domain.empty()) {
    domain = url.host();
  }
  auto filter = content::BrowsingDataFilterBuilder::Create(
      content::BrowsingDataFilterBuilder::Mode::kDelete);
  filter->AddRegisterableDomain(domain);
  content::BrowsingDataRemover* remover = clearing->profile()->GetBrowsingDataRemover();
  // The cache's reload also fetches again what the page loads from other sites.
  remover->RemoveWithFilterAndReply(
      base::Time(), base::Time::Max(),
      cookies ? content::BrowsingDataRemover::DATA_TYPE_COOKIES
              : content::BrowsingDataRemover::DATA_TYPE_CACHE,
      content::BrowsingDataRemover::ORIGIN_TYPE_UNPROTECTED_WEB, std::move(filter),
      new ReloadWhenCleared(remover, tab, !cookies));
}

}  // namespace browser_core
