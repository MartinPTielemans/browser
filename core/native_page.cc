#include "browser_core/native_page.h"

#include <memory>

#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/navigation_throttle.h"
#include "content/public/browser/navigation_throttle_registry.h"
#include "content/public/browser/web_contents.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

class NativePageThrottle : public content::NavigationThrottle {
 public:
  using content::NavigationThrottle::NavigationThrottle;

  // content::NavigationThrottle:
  ThrottleCheckResult WillStartRequest() override { return Ask(); }
  // A link that redirects to a site the app shows itself, as a shortened one does.
  ThrottleCheckResult WillRedirectRequest() override { return Ask(); }
  const char* GetNameForLogging() override { return "browser_core::NativePageThrottle"; }

 private:
  ThrottleCheckResult Ask() {
    content::WebContents* contents = navigation_handle()->GetWebContents();
    const GURL& url = navigation_handle()->GetURL();
    // A new tab's page starts loading before Chrome puts it in its window, though it's a tab
    // already.
    if (!Host().tab_navigating || !url.SchemeIsHTTPOrHTTPS() ||
        !tabs::TabInterface::MaybeGetFromContents(contents)) {
      return PROCEED;
    }
    const bc_id id = Tab::IdOf(contents);
    if (Host().tab_navigating(Host().context, id, url.spec().c_str())) {
      return CANCEL_AND_IGNORE;
    }
    Tab::FromWebContents(contents)->SetNativePage(gfx::NativeView());
    return PROCEED;
  }
};

}  // namespace

void AddNativePageThrottle(content::NavigationThrottleRegistry& registry) {
  if (registry.GetNavigationHandle().IsInPrimaryMainFrame()) {
    registry.AddThrottle(std::make_unique<NativePageThrottle>(registry));
  }
}

}  // namespace browser_core
