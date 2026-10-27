// Hooks that the series in engine/patches/ours adds to Chrome's files, for the core to set. Each is
// defined in the Chrome file that reads it, so Chrome's other targets link without the core.
#ifndef BROWSER_CORE_HOOKS_H_
#define BROWSER_CORE_HOOKS_H_

#include <memory>
#include <vector>

#include "base/functional/callback_forward.h"
#include "components/permissions/permission_prompt.h"
#include "extensions/browser/extension_install_prompt_client.h"

// chrome/browser/ui/browser.cc: a Browser may be made with a window the core supplies, which has
// none of Chrome's views, and the app has its own main menu. Chrome's files that expect Chrome's
// read it too.
extern bool g_browser_core_supplies_windows;

class BrowserWindowInterface;
class ExtensionInstallPromptShowParams;
class GURL;
class Profile;
struct NavigateParams;

namespace content {
class RenderFrameHost;
class RenderWidgetHost;
class WebContents;
}

namespace extensions {
class InstallPromptData;
}

namespace mojo {
template <typename ContextType>
class BinderMapWithContext;
}

namespace policy {
class ConfigurationPolicyProvider;
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

// chrome/browser/chrome_content_browser_client_receiver_bindings.cc: adds the core's interfaces to
// the ones a frame's page can ask the browser for, once Chrome has added its own. Only what a page
// may be trusted with goes here: its calls are the page's own words.
extern void (*g_browser_core_populate_frame_binders)(
    mojo::BinderMapWithContext<content::RenderFrameHost*>* map);

// chrome/browser/ui/views/tab_dialogs_views.cc: shows or hides the offer to save a submitted
// login's password, in place of Chrome's bubble, which hangs from a toolbar the core's windows
// don't have.
extern void (*g_browser_core_password_prompt)(content::WebContents* contents, bool show);
// chrome/browser/app_controller_mac.mm: opens the links another app sent, such as a link clicked in
// Mail or an .html file from Finder, in place of Chrome's tabs for them. Those that launched the
// browser come once it has finished launching, before the app has started.
extern void (*g_browser_core_open_links)(const std::vector<GURL>& urls);
// chrome/browser/ui/browser_web_contents_delegate/browser_web_contents_delegate.cc: the page of
// |source|, a tab of |browser|, has the pointer on the link |url|, or on none and |url| is empty.
// Chrome draws it in a status bubble, which the core's windows don't have.
extern void (*g_browser_core_update_target_url)(BrowserWindowInterface* browser,
                                                content::WebContents* source,
                                                const GURL& url);

// chrome/browser/ui/views/tab_dialogs_views.cc: |contents|'s page hasn't answered input for
// Chrome's hang timeout, in place of Chrome's Page Unresponsive dialog, a tab-modal dialog the
// core's windows would show over the page. |restart_hang_monitor| is Chrome's Wait.
extern void (*g_browser_core_page_hung)(content::WebContents* contents,
                                        content::RenderWidgetHost* widget,
                                        base::RepeatingClosure restart_hang_monitor);

// chrome/browser/net/net_error_tab_helper.cc: the core's offline page says whether a run of its
// game is going. Only the error document in the tab's main frame can say it.
extern void (*g_browser_core_offline_game)(content::WebContents* contents, bool running);

// chrome/browser/extensions/extension_service.cc: loads the extensions the browser comes with into
// |profile|, after the ones the command line names and before its extensions are ready.
extern void (*g_browser_core_load_bundled_extensions)(Profile* profile);

// chrome/browser/policy/profile_policy_connector.cc: policy the core sets for every profile,
// under the platform's and the command line's, as for the extensions the browser comes with.
// The core keeps it for as long as the browser runs.
extern policy::ConfigurationPolicyProvider* (*g_browser_core_policy_provider)();

#endif  // BROWSER_CORE_HOOKS_H_
