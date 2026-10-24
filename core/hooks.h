// Hooks that the series in engine/patches/ours adds to Chrome's files, for the core to set. Each is
// defined in the Chrome file that reads it, so Chrome's other targets link without the core.
#ifndef BROWSER_CORE_HOOKS_H_
#define BROWSER_CORE_HOOKS_H_

// chrome/browser/ui/browser.cc: a Browser may be made with a window the core supplies, which has
// none of Chrome's views, and the app has its own main menu. Chrome's files that expect Chrome's
// read it too.
extern bool g_browser_core_supplies_windows;

struct NavigateParams;

// chrome/browser/ui/navigator/browser_navigator.cc: changes where a navigation opens, before Chrome
// picks the Browser for it.
extern void (*g_browser_core_adjust_navigate)(NavigateParams& params);

#endif  // BROWSER_CORE_HOOKS_H_
