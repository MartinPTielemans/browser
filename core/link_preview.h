// A link the user clicks in a page, to another site than the page's, which the app may open in a
// preview over the window instead of in the tab (bc_host's tab_link_to_site).

#ifndef BROWSER_CORE_LINK_PREVIEW_H_
#define BROWSER_CORE_LINK_PREVIEW_H_

namespace content {
class NavigationThrottleRegistry;
}

namespace browser_core {

// Asks the app about |registry|'s navigation as its request starts, if it's a plain click on a link
// in a tab's own page: in the tab, from the user, not a form or a page's script without a click.
void AddLinkPreviewThrottle(content::NavigationThrottleRegistry& registry);

}  // namespace browser_core

#endif  // BROWSER_CORE_LINK_PREVIEW_H_
