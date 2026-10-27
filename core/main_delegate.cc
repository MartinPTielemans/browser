#include "browser_core/main_delegate.h"

#include <cstdio>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "base/base_switches.h"
#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/logging.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/drm_request.h"
#include "browser_core/hooks.h"
#include "browser_core/host.h"
#include "browser_core/install_question.h"
#include "browser_core/password_offer.h"
#include "browser_core/permission_question.h"
#include "browser_core/platform/keychain.h"
#include "browser_core/platform/native_hosts.h"
#include "browser_core/profiles.h"
#include "browser_core/session.h"
#include "browser_core/system_engine.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "build/build_config.h"
#include "chrome/browser/chrome_browser_main.h"
#include "chrome/browser/chrome_browser_main_extra_parts.h"
#include "chrome/browser/chrome_content_browser_client.h"
#include "chrome/browser/devtools/chrome_devtools_manager_delegate.h"
#include "chrome/browser/lifetime/application_lifetime.h"
#include "chrome/browser/lifetime/termination_notification.h"
#include "chrome/common/chrome_switches.h"
#include "chrome/common/profiler/main_thread_stack_sampling_profiler.h"
#include "components/keep_alive_registry/keep_alive_types.h"
#include "components/keep_alive_registry/scoped_keep_alive.h"
#include "content/public/common/content_features.h"
#include "content/public/common/content_switches.h"
#include "url/gurl.h"

#if BUILDFLAG(IS_WIN) || BUILDFLAG(IS_LINUX)
#include "browser_core/platform/app_library.h"
#endif
#if BUILDFLAG(IS_WIN)
#include "base/strings/utf_string_conversions.h"
#endif
#if BUILDFLAG(IS_LINUX)
#include "ui/ozone/public/ozone_switches.h"
#endif
#if !BUILDFLAG(IS_WIN) && !BUILDFLAG(IS_LINUX)
// chrome_main.cc's entry point, in the same framework.
extern "C" int ChromeMain(int argc, const char** argv);
#endif

namespace browser_core {

namespace {

bc_host g_host;

#if BUILDFLAG(IS_MAC)
// The links another app sent, to the app. Only pages and files: AppController strips Chrome's
// own direct-launch scheme first, and any other scheme an app sends is no page to show.
void OpenLinks(const std::vector<GURL>& urls) {
  std::vector<std::string> specs;
  for (const GURL& url : urls) {
    if (url.is_valid() && (url.SchemeIsHTTPOrHTTPS() || url.SchemeIsFile())) {
      specs.push_back(url.spec());
    }
  }
  if (specs.empty() || !g_host.open_links) {
    return;
  }
  std::vector<const char*> links;
  for (const std::string& spec : specs) {
    links.push_back(spec.c_str());
  }
  g_host.open_links(g_host.context, links.data(), links.size());
}
#endif

// A host's manifest from another browser's folders, said in the log, as the user can't see where
// a host came from.
base::FilePath FindNativeHost(const std::string& name) {
  fprintf(stderr, "browser: native host lookup %s\n", name.c_str());
  base::FilePath manifest = platform::FindOtherBrowsersNativeHost(name);
  if (!manifest.empty()) {
    fprintf(stderr, "browser: native host %s from %s\n", name.c_str(),
            manifest.DirName().AsUTF8Unsafe().c_str());
  }
  return manifest;
}

// Keeps the browser process up while it has no windows, which Chrome would otherwise quit at, and
// tells the app when it can call in and when it has to stop.
class ExtraParts : public ChromeBrowserMainExtraParts {
 public:
  // Before Chrome stops taking factories, as the first profile is made.
  void PreProfileInit() override { system_engine::RegisterProfilePrefs(); }

  void PreBrowserStart() override {
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
  void OnAppTerminating() {
    keep_alive_.reset();
    // A remote-debugging switch, with the window Chrome doesn't open at startup, has Chrome hold
    // the browser up until a client closes it over the protocol, so the user's quit never ended.
    ChromeDevToolsManagerDelegate::AllowBrowserToClose();
  }

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

  void CreateThrottlesForNavigation(content::NavigationThrottleRegistry& registry) override {
    ChromeContentBrowserClient::CreateThrottlesForNavigation(registry);
    system_engine::CreateThrottlesForNavigation(registry);
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

// Keeps the app's callbacks and puts the core's hooks in Chrome.
void Install(const bc_host& host) {
  g_host = host;
  g_browser_core_supplies_windows = true;
  g_browser_core_adjust_navigate = &Window::AdjustNavigate;
  g_browser_core_can_dock_devtools = &Window::CanDockDevTools;
  g_browser_core_update_devtools = &Window::UpdateDevTools;
  g_browser_core_trim_page_menu = &Window::TrimPageMenu;
  g_browser_core_update_target_url = &Tab::LinkTargetChanged;
  g_browser_core_permission_prompt = &PermissionQuestion::Create;
  g_browser_core_extension_install_dialog = &InstallQuestion::Ask;
  g_browser_core_populate_frame_binders = &PopulateFrameBinders;
  g_browser_core_password_prompt = &PasswordOffer::Prompt;
#if BUILDFLAG(IS_MAC)
  g_browser_core_open_links = &OpenLinks;
#endif
  g_browser_core_page_hung = &Tab::PageHung;
  g_browser_core_offline_game = &Tab::GameRunning;
  g_browser_core_find_native_host = &FindNativeHost;
}

int Run(int argc, const char** argv, const bc_host& host) {
#if BUILDFLAG(IS_WIN) || BUILDFLAG(IS_LINUX)
  // Chromium's own executable starts Chromium here, and loads the app (platform/app_library.h).
  return 1;
#else
  Install(host);
  return ChromeMain(argc, argv);
#endif
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
#if BUILDFLAG(IS_WIN) || BUILDFLAG(IS_LINUX)
    // First, so the features below join the app's rather than being replaced by them. The app's
    // switches go after the ones Chromium was given, which still win.
    std::vector<std::string> app_switches;
    const bc_host* host = platform::LoadApp(&app_switches);
    if (!host) {
      LOG(ERROR) << "no app: the engine has no " << BC_APP_LIBRARY << " beside it with "
                 << BC_APP_ENTRY;
      return 1;
    }
    Install(*host);
    base::CommandLine::StringVector app_argv(1);
    for (const std::string& app_switch : app_switches) {
#if BUILDFLAG(IS_WIN)
      app_argv.push_back(base::UTF8ToWide(app_switch));
#else
      app_argv.push_back(app_switch);
#endif
    }
    command_line->AppendArguments(base::CommandLine(app_argv), /*include_program=*/false);
#endif
#if BUILDFLAG(IS_LINUX)
    // X11, under XWayland on a Wayland desktop: the app's UI is a window of its own that the core
    // puts inside the browser window, which Wayland can't do across the two's connections
    // (platform/linux/window_views.cc).
    if (!command_line->HasSwitch(switches::kOzonePlatform)) {
      command_line->AppendSwitchASCII(switches::kOzonePlatform, "x11");
    }
#endif
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

std::optional<int> MainDelegate::PreBrowserMain() {
  std::optional<int> exit_code = ChromeMainDelegate::PreBrowserMain();
  // Chrome's PreBrowserMain made NSApp, which the app can't make before it: Chrome's class reads
  // the command line as it's made.
  if (!exit_code && g_host.launching) {
    g_host.launching(g_host.context);
  }
  return exit_code;
}

content::ContentBrowserClient* MainDelegate::CreateContentBrowserClient() {
  // As ChromeMainDelegate's, with our client in place of Chrome's.
  chrome_content_browser_client_ = std::make_unique<ContentBrowserClient>();
  chrome_content_browser_client_->SetSamplingProfiler(std::move(sampling_profiler_));
  return chrome_content_browser_client_.get();
}

}  // namespace browser_core
