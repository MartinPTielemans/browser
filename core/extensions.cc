#include "browser_core/extensions.h"

#include <algorithm>
#include <map>
#include <memory>
#include <string>
#include <vector>

#include "base/functional/bind.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/profiles.h"
#include "chrome/browser/extensions/extension_action_runner.h"
#include "chrome/browser/extensions/extension_view_host.h"
#include "chrome/browser/extensions/extension_view_host_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/extensions/extension_popup_types.h"
#include "chrome/browser/ui/views/extensions/extension_popup.h"
#include "components/sessions/content/session_tab_helper.h"
#include "content/public/browser/web_contents.h"
#include "extensions/browser/extension_action.h"
#include "extensions/browser/extension_action_manager.h"
#include "extensions/browser/extension_host.h"
#include "extensions/browser/extension_host_observer.h"
#include "extensions/browser/extension_registry.h"
#include "extensions/browser/extension_registry_observer.h"
#include "extensions/browser/ui_util.h"
#include "extensions/common/extension.h"
#include "ui/views/bubble/bubble_anchor.h"
#include "ui/views/bubble/bubble_border.h"

namespace browser_core {

namespace {

// Tells the app of one profile's extensions, once after each call that changed them.
class ProfileExtensions : public extensions::ExtensionRegistryObserver {
 public:
  explicit ProfileExtensions(Profile* profile) : profile_(profile) {
    observation_.Observe(extensions::ExtensionRegistry::Get(profile));
    Changed();
  }

  void Changed() {
    if (report_posted_) {
      return;
    }
    report_posted_ = true;
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&ProfileExtensions::Report, weak_factory_.GetWeakPtr()));
  }

 private:
  // extensions::ExtensionRegistryObserver:
  void OnExtensionLoaded(content::BrowserContext* context,
                         const extensions::Extension* extension) override {
    Changed();
  }
  void OnExtensionUnloaded(content::BrowserContext* context,
                           const extensions::Extension* extension,
                           extensions::UnloadedExtensionReason reason) override {
    Changed();
  }
  void OnExtensionInstalled(content::BrowserContext* context,
                            const extensions::Extension* extension,
                            bool is_update) override {
    Changed();
  }
  void OnExtensionUninstalled(content::BrowserContext* context,
                              const extensions::Extension* extension,
                              extensions::UninstallReason reason) override {
    Changed();
  }
  // The profile is going, and its extensions with it.
  void OnShutdown(extensions::ExtensionRegistry* registry) override { observation_.Reset(); }

  void Report() {
    report_posted_ = false;
    if (!Host().extensions_changed || !observation_.IsObserving()) {
      return;
    }
    extensions::ExtensionRegistry* registry = extensions::ExtensionRegistry::Get(profile_);
    std::vector<scoped_refptr<const extensions::Extension>> listed;
    for (const scoped_refptr<const extensions::Extension>& extension :
         registry->GenerateInstalledExtensionsSet()) {
      if (extensions::ui_util::ShouldDisplayInExtensionSettings(*extension)) {
        listed.push_back(extension);
      }
    }
    std::ranges::sort(listed, {}, [](const scoped_refptr<const extensions::Extension>& extension) {
      return extension->name();
    });
    std::vector<bc_extension> extensions;
    extensions.reserve(listed.size());
    for (const scoped_refptr<const extensions::Extension>& extension : listed) {
      extensions.push_back({.id = extension->id().c_str(),
                            .name = extension->name().c_str(),
                            .enabled = registry->enabled_extensions().Contains(extension->id())});
    }
    Host().extensions_changed(Host().context, ProfileId(DirOf(profile_)), extensions.data(),
                              extensions.size());
  }

  raw_ptr<Profile> profile_;
  base::ScopedObservation<extensions::ExtensionRegistry, extensions::ExtensionRegistryObserver>
      observation_{this};
  bool report_posted_ = false;
  base::WeakPtrFactory<ProfileExtensions> weak_factory_{this};
};

std::map<std::string, std::unique_ptr<ProfileExtensions>>& Observed() {
  static base::NoDestructor<std::map<std::string, std::unique_ptr<ProfileExtensions>>> observed;
  return *observed;
}

// The popup opening or shown, one at a time as in Chrome, which tells the app as it shows and as
// it goes. Chrome's popup closes itself when it loses the keyboard or its window shows another tab.
class Popup : public extensions::ExtensionHostObserver {
 public:
  void Open(extensions::ExtensionHost* host, bc_id window, const std::string& extension) {
    observation_.Reset();
    Report(window_, "");
    observation_.Observe(host);
    window_ = window;
    extension_ = extension;
  }

  void Shown(extensions::ExtensionHost* host) {
    if (host && observation_.IsObservingSource(host)) {
      Report(window_, extension_);
    }
  }

 private:
  // extensions::ExtensionHostObserver:
  void OnExtensionHostDestroyed(extensions::ExtensionHost* host) override {
    observation_.Reset();
    Report(window_, "");
    window_ = 0;
  }

  static void Report(bc_id window, const std::string& extension) {
    if (window && Host().window_extension_popup) {
      Host().window_extension_popup(Host().context, window, extension.c_str());
    }
  }

  bc_id window_ = 0;
  std::string extension_;
  base::ScopedObservation<extensions::ExtensionHost, extensions::ExtensionHostObserver>
      observation_{this};
};

Popup& ThePopup() {
  static base::NoDestructor<Popup> popup;
  return *popup;
}

}  // namespace

void ObserveExtensions(Profile* profile) {
  std::unique_ptr<ProfileExtensions>& observed = Observed()[DirOf(profile)];
  observed = std::make_unique<ProfileExtensions>(profile);
}

void ReportExtensions() {
  for (auto& [dir, observed] : Observed()) {
    observed->Changed();
  }
}

bool RunExtension(bc_id window,
                  BrowserWindowInterface* browser,
                  content::WebContents* contents,
                  const std::string& id,
                  views::View* anchor) {
  Profile* profile = Profile::FromBrowserContext(contents->GetBrowserContext());
  const extensions::Extension* extension =
      extensions::ExtensionRegistry::Get(profile)->enabled_extensions().GetByID(id);
  extensions::ExtensionActionRunner* runner =
      extensions::ExtensionActionRunner::GetForWebContents(contents);
  if (!extension || !runner) {
    return false;
  }
  if (runner->RunAction(extension, /*grant_tab_permissions=*/true) !=
      extensions::ExtensionAction::ShowAction::kShowPopup) {
    return true;
  }
  extensions::ExtensionAction* action =
      extensions::ExtensionActionManager::Get(profile)->GetExtensionAction(*extension);
  const int tab = sessions::SessionTabHelper::IdForTab(contents).id();
  if (!action || !action->HasPopup(tab)) {
    return true;
  }
  std::unique_ptr<extensions::ExtensionViewHost> host =
      extensions::ExtensionViewHostFactory::CreatePopupHost(*extension, action->GetPopupUrl(tab),
                                                            browser);
  if (!host) {
    return true;
  }
  ThePopup().Open(host.get(), window, id);
  ExtensionPopup::ShowPopup(
      browser, std::move(host), views::BubbleAnchor(anchor), views::BubbleBorder::TOP_LEFT,
      PopupShowAction::kShow,
      base::BindOnce([](extensions::ExtensionHost* host) { ThePopup().Shown(host); }));
  return true;
}

}  // namespace browser_core
