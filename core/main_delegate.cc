#include "browser_core/main_delegate.h"

#include <memory>
#include <string>
#include <utility>

#include "base/base_switches.h"
#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/hooks.h"
#include "browser_core/host.h"
#include "browser_core/install_question.h"
#include "browser_core/leave_dialog.h"
#include "browser_core/permission_question.h"
#include "browser_core/platform/keychain.h"
#include "browser_core/profiles.h"
#include "browser_core/session.h"
#include "browser_core/sign_in_dialog.h"
#include "browser_core/window.h"
#include "build/build_config.h"
#include "chrome/browser/chrome_browser_main.h"
#include "chrome/browser/chrome_browser_main_extra_parts.h"
#include "chrome/browser/chrome_content_browser_client.h"
#include "chrome/browser/lifetime/application_lifetime.h"
#include "chrome/browser/lifetime/termination_notification.h"
#include "chrome/common/chrome_switches.h"
#include "chrome/common/profiler/main_thread_stack_sampling_profiler.h"
#include "components/keep_alive_registry/keep_alive_types.h"
#include "components/keep_alive_registry/scoped_keep_alive.h"
#include "content/public/common/content_features.h"
#include "content/public/common/content_switches.h"

// chrome_main.cc's entry point, in the same framework.
extern "C" int ChromeMain(int argc, const char** argv);

namespace browser_core {

namespace {

bc_host g_host;

// Keeps the browser process up while it has no windows, which Chrome would otherwise quit at, and
// tells the app when it can call in and when it has to stop.
class ExtraParts : public ChromeBrowserMainExtraParts {
 public:
  void PreBrowserStart() override {
    // A tab's "Leave site?" asks with the app's dialog, in place of the NSAlert Chrome installed.
    LeaveDialog::Install();
    keep_alive_ = std::make_unique<ScopedKeepAlive>(KeepAliveOrigin::APP_CONTROLLER,
                                                    KeepAliveRestartOption::DISABLED);
    // A quit can start in Chrome too, such as from the Dock or the system logging out, and Chrome
    // only says it's terminating once every Browser has closed.
    app_terminating_ = browser_shutdown::AddAppTerminatingCallback(
        base::BindOnce(&ExtraParts::OnAppTerminating, base::Unretained(this)));
  }

  void PostBrowserStart() override {
    if (Profile* profile = StartProfile()) {
      UseProfile(profile);
    }
    // Once the Spaces the last launch saved are read, and so once the run loop is running.
    LoadSession(base::BindOnce([] {
      if (g_host.started) {
        g_host.started(g_host.context);
      }
    }));
  }

  void PostMainMessageLoopRun() override {
    // Their Browsers are gone: quitting closed them, and the run loop waited for that.
    FlushSession();
    Window::CloseAll();
    ReleaseProfiles();
    if (g_host.stopping) {
      g_host.stopping(g_host.context);
    }
  }

 private:
  void OnAppTerminating() { keep_alive_.reset(); }

  std::unique_ptr<ScopedKeepAlive> keep_alive_;
  base::CallbackListSubscription app_terminating_;
};

class ContentBrowserClient : public ChromeContentBrowserClient {
 public:
  std::unique_ptr<content::BrowserMainParts> CreateBrowserMainParts(
      bool is_integration_test) override {
    std::unique_ptr<content::BrowserMainParts> parts =
        ChromeContentBrowserClient::CreateBrowserMainParts(is_integration_test);
    static_cast<ChromeBrowserMainParts*>(parts.get())->AddParts(std::make_unique<ExtraParts>());
    return parts;
  }
};

}  // namespace

const bc_host& Host() {
  return g_host;
}

bc_id NewId() {
  static bc_id last = 0;
  return ++last;
}

int Run(int argc, const char** argv, const bc_host& host) {
  g_host = host;
  g_browser_core_supplies_windows = true;
  g_browser_core_adjust_navigate = &Window::AdjustNavigate;
  g_browser_core_can_dock_devtools = &Window::CanDockDevTools;
  g_browser_core_update_devtools = &Window::UpdateDevTools;
  g_browser_core_trim_page_menu = &Window::TrimPageMenu;
  g_browser_core_permission_prompt = &PermissionQuestion::Create;
  g_browser_core_extension_install_dialog = &InstallQuestion::Ask;
  g_browser_core_login_handler = &SignInDialog::Create;
  return ChromeMain(argc, argv);
}

void Quit() {
  // Posted, so the quit starts once the event that asked for it is done: started from inside the
  // app's key event for ⌘Q, the tab closed and its Browser never did.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE,
                                                              base::BindOnce(&chrome::AttemptExit));
}

MainDelegate::MainDelegate(const StartupTimestamps& timestamps)
    : ChromeMainDelegate(timestamps) {}

MainDelegate::~MainDelegate() = default;

std::optional<int> MainDelegate::BasicStartupComplete() {
  base::CommandLine* command_line = base::CommandLine::ForCurrentProcess();
  if (!command_line->HasSwitch(switches::kProcessType)) {
    // The app opens windows through the core, and Chrome's first run and default-browser prompts
    // are Chrome's UI, not ours.
    command_line->AppendSwitch(switches::kNoStartupWindow);
    command_line->AppendSwitch(switches::kNoFirstRun);
    command_line->AppendSwitch(switches::kNoDefaultBrowserCheck);
    // A pinned tab the user closes unloads in place, keeping its tab and the id the app knows it by.
    // Chrome's own discards, under memory pressure, then keep theirs too.
    std::string features = command_line->GetSwitchValueASCII(switches::kEnableFeatures);
    features += features.empty() ? "" : ",";
    command_line->AppendSwitchASCII(switches::kEnableFeatures,
                                    features + features::kWebContentsDiscard.name);
#if BUILDFLAG(IS_MAC)
    platform::NameKeychainItem();
#endif
  }
  return ChromeMainDelegate::BasicStartupComplete();
}

content::ContentBrowserClient* MainDelegate::CreateContentBrowserClient() {
  // As ChromeMainDelegate's, with our client in place of Chrome's.
  chrome_content_browser_client_ = std::make_unique<ContentBrowserClient>();
  chrome_content_browser_client_->SetSamplingProfiler(std::move(sampling_profiler_));
  return chrome_content_browser_client_.get();
}

}  // namespace browser_core
