// Hooks that the series in engine/patches/ours adds to Chrome's files, for the core to set. Each is
// defined in the Chrome file that reads it, so Chrome's other targets link without the core.
#ifndef BROWSER_CORE_HOOKS_H_
#define BROWSER_CORE_HOOKS_H_

#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/functional/callback_forward.h"
#include "components/permissions/permission_prompt.h"
#include "components/sync/base/data_type.h"
#include "components/sync/base/passphrase_enums.h"
#include "components/sync/engine/commit_and_get_updates_types.h"
#include "content/public/browser/login_delegate.h"
#include "content/public/browser/weak_document_ptr.h"
#include "extensions/browser/extension_install_prompt_client.h"
#include "url/gurl.h"
#include "url/origin.h"

// chrome/browser/ui/browser.cc: a Browser may be made with a window the core supplies, which has
// none of Chrome's views, and the app has its own main menu. Chrome's files that expect Chrome's
// read it too.
extern bool g_browser_core_supplies_windows;

class BrowserWindowInterface;
class GoogleServiceAuthError;
class ExtensionInstallPromptShowParams;
class RenderViewContextMenuBase;
class GURL;
class LoginHandler;
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

namespace net {
class AuthChallengeInfo;
}

namespace policy {
class ConfigurationPolicyProvider;
}

namespace signin {
struct AccessTokenInfo;
class IdentityManager;
}

namespace syncer {
class DataTypeController;
struct SyncAccountInfo;
class SyncInvalidationsService;
class SyncService;
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

// chrome/browser/ui/views/tab_contents/chrome_web_contents_view_delegate_views_mac.mm, and
// chrome_web_contents_view_delegate_views.cc on the other platforms: the context menu of
// |contents|'s page, as Chrome built it to show. Returns |menu| for Chrome to show, or null once
// the core has taken it, to show in the app's own menu.
extern std::unique_ptr<RenderViewContextMenuBase> (*g_browser_core_show_page_menu)(
    content::WebContents* contents,
    std::unique_ptr<RenderViewContextMenuBase> menu);

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

// chrome/browser/ui/tabs/tab_strip_model.cc: whether |contents| and |other| are tabs of one of the
// core's split views, between which a switch isn't timed, as Chrome doesn't time one between the
// tabs of its own split: both pages are on screen already.
extern bool (*g_browser_core_split_together)(content::WebContents* contents,
                                             content::WebContents* other);

// chrome/browser/extensions/api/messaging/launch_context.cc: the manifest of the native messaging
// host |name| when neither the browser's user folder nor its system one has it, or an empty path.
// Runs where Chrome looks in its own, on a thread that may block.
extern base::FilePath (*g_browser_core_find_native_host)(const std::string& name);

// ui/events/platform/wayland/wayland_event_watcher.cc: Chromium has dispatched its Wayland
// display's events, which the app's UI shares (platform/linux/wayland.cc). Never called while
// Chromium has a read prepared.
extern void (*g_browser_core_wayland_dispatched)();

// chrome/browser/ui/views/login_handler_views.cc: asks for a site's HTTP sign-in, in place of
// Chrome's views dialog, which needs a dialog host the core's windows don't have.
extern std::unique_ptr<LoginHandler> (*g_browser_core_login_handler)(
    const net::AuthChallengeInfo& auth_info,
    content::WebContents* web_contents,
    content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback);

// chrome/browser/ui/views/external_protocol_dialog.cc: asks whether |url|'s page may open
// |program_name|, the app for its scheme, in place of Chrome's views dialog, once Chrome has found
// that it asks.
extern void (*g_browser_core_external_protocol_dialog)(
    const GURL& url,
    content::WebContents* web_contents,
    const std::optional<url::Origin>& initiating_origin,
    content::WeakDocumentPtr initiator_document,
    const std::u16string& program_name);
// chrome/browser/extensions/extension_service.cc: loads the extensions the browser comes with into
// |profile|, after the ones the command line names and before its extensions are ready.
extern void (*g_browser_core_load_bundled_extensions)(Profile* profile);

// chrome/browser/policy/profile_policy_connector.cc: policy the core sets for every profile,
// under the platform's and the command line's, as for the extensions the browser comes with.
// The core keeps it for as long as the browser runs.
extern policy::ConfigurationPolicyProvider* (*g_browser_core_policy_provider)();

// components/sync/service/sync_auth_manager.cc: the account to sync and its access tokens, from the
// browser's own account in place of Google's sign-in, for the profile |identity_manager| is
// |profile|'s. |done| runs after the call returns. |changed| is the call to make when the account
// changes, as IdentityManager's events would, and does nothing once that profile's sync is gone.
extern syncer::SyncAccountInfo (*g_browser_core_sync_account)(
    const signin::IdentityManager* identity_manager);
extern void (*g_browser_core_sync_token)(
    const signin::IdentityManager* identity_manager,
    base::OnceCallback<void(GoogleServiceAuthError, signin::AccessTokenInfo)> done);
extern void (*g_browser_core_sync_account_watch)(const signin::IdentityManager* identity_manager,
                                                 base::RepeatingClosure changed);

// chrome/browser/sync/sync_invalidations_service_factory.cc: |profile|'s sync change
// notifications, from the browser's relay in place of FCM.
extern std::unique_ptr<syncer::SyncInvalidationsService> (*g_browser_core_sync_invalidations)(
    Profile* profile);

// components/sync/service/sync_service_impl.cc: the server sync talks to, read as its engine
// starts, in place of --sync-url's, which is read once.
extern GURL (*g_browser_core_sync_url)();

// components/sync/engine/data_type_worker.cc: whether |type|'s changes wait to commit, given
// whether the type is encrypted and with which passphrase, and a last look at each commit before
// it goes to the server. Both run on sync's own sequence.
extern bool (*g_browser_core_sync_hold_commit)(syncer::DataType type,
                                               bool encrypted,
                                               syncer::PassphraseType passphrase);
extern void (*g_browser_core_sync_check_commit)(syncer::DataType type,
                                                const syncer::CommitRequestDataList& commit);

// components/password_manager/core/browser/sync/password_proto_utils.cc: a password's client tag,
// from Chromium's, on the password store's sequence.
extern std::string (*g_browser_core_sync_password_tag)(const std::string& tag);

// chrome/browser/sync/sync_service_factory.cc: the controllers of the core's own data types for
// |profile|'s sync, such as ILO_SPACE's, after Chrome's.
extern std::vector<std::unique_ptr<syncer::DataTypeController>> (*g_browser_core_sync_controllers)(
    Profile* profile,
    syncer::SyncService* sync_service);

#endif  // BROWSER_CORE_HOOKS_H_
