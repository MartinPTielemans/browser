// Hooks that the series in engine/patches/ours adds to Chrome's files, for the core to set. Each is
// defined in the Chrome file that reads it, so Chrome's other targets link without the core.
#ifndef BROWSER_CORE_HOOKS_H_
#define BROWSER_CORE_HOOKS_H_

#include <memory>

#include "components/permissions/permission_prompt.h"
#include "content/public/browser/login_delegate.h"
#include "extensions/browser/extension_install_prompt_client.h"

// chrome/browser/ui/browser.cc: a Browser may be made with a window the core supplies, which has
// none of Chrome's views, and the app has its own main menu. Chrome's files that expect Chrome's
// read it too.
extern bool g_browser_core_supplies_windows;

class BrowserWindowInterface;
class ExtensionInstallPromptShowParams;
class LoginHandler;
struct NavigateParams;

namespace content {
class WebContents;
}

namespace extensions {
class InstallPromptData;
}

namespace net {
class AuthChallengeInfo;
}

namespace ui {
class SimpleMenuModel;
}

// chrome/browser/ui/navigator/browser_navigator.cc: changes where a navigation opens, before Chrome
// picks the Browser for it.
extern void (*g_browser_core_adjust_navigate)(NavigateParams& params);

// chrome/browser/devtools/devtools_window.cc: the core docks DevTools in its windows, which have no
// DevtoolsUIController. Whether |browser|'s window can, and the call as the docked DevTools of
// |inspected|, a tab of |browser|, open, lay out again or close.
extern bool (*g_browser_core_can_dock_devtools)(BrowserWindowInterface* browser);
extern void (*g_browser_core_update_devtools)(BrowserWindowInterface* browser,
                                              content::WebContents* inspected);

// chrome/browser/renderer_context_menu/render_view_context_menu.cc: takes out of a page's context
// menu the items Arc's leaves out and those |browser|'s window can't do, once Chrome has filled it
// in. The menu is the user's: agents act on pages through their own tools and never see it.
extern void (*g_browser_core_trim_page_menu)(BrowserWindowInterface* browser,
                                             ui::SimpleMenuModel& menu);

// chrome/browser/ui/views/permissions/permission_prompt_factory.cc: the prompt for a page's
// permission request, in place of Chrome's, whose bubbles hang from a toolbar the core's windows
// don't have. Null shows none, and Chrome takes the request as ignored.
extern std::unique_ptr<permissions::PermissionPrompt> (*g_browser_core_permission_prompt)(
    content::WebContents* contents,
    permissions::PermissionPrompt::Delegate* delegate);

// chrome/browser/ui/views/extensions/extension_install_dialog_view.cc: asks the user to confirm an
// extension's install, re-enable or new permissions, in place of Chrome's dialog. |done_callback|
// must run, as Chrome's dialog runs it.
extern void (*g_browser_core_extension_install_dialog)(
    std::unique_ptr<ExtensionInstallPromptShowParams> show_params,
    extensions::ExtensionInstallPromptClient::DoneCallback done_callback,
    std::unique_ptr<extensions::InstallPromptData> prompt);

// chrome/browser/ui/views/login_handler_views.cc: asks for a site's HTTP sign-in, in place of
// Chrome's views dialog, which needs a dialog host the core's windows don't have.
extern std::unique_ptr<LoginHandler> (*g_browser_core_login_handler)(
    const net::AuthChallengeInfo& auth_info,
    content::WebContents* web_contents,
    content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback);

#endif  // BROWSER_CORE_HOOKS_H_
