#ifndef BROWSER_CORE_NATIVE_PAGE_H_
#define BROWSER_CORE_NATIVE_PAGE_H_

namespace content {
class NavigationThrottleRegistry;
}

namespace browser_core {

// Adds to |registry|'s navigation, if it loads a tab's page, the throttle that asks the app with
// tab_navigating whether to show that page itself (bc_tab_set_native_page), and stops the load if
// it does.
void AddNativePageThrottle(content::NavigationThrottleRegistry& registry);

}  // namespace browser_core

#endif  // BROWSER_CORE_NATIVE_PAGE_H_
