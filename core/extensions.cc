#include "browser_core/extensions.h"

#include <algorithm>
#include <map>
#include <memory>
#include <optional>
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
#include "extensions/browser/extension_action_icon_factory.h"
#include "extensions/browser/extension_action_manager.h"
#include "extensions/browser/extension_host.h"
#include "extensions/browser/extension_host_observer.h"
#include "extensions/browser/extension_registry.h"
#include "extensions/browser/extension_registry_observer.h"
#include "extensions/browser/ui_util.h"
#include "extensions/common/extension.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/image/image_skia.h"
#include "ui/gfx/image/image_skia_rep.h"
#include "ui/views/bubble/bubble_anchor.h"
#include "ui/views/bubble/bubble_border.h"

namespace browser_core {

namespace {

// Tells the app of one profile's extensions, once after each call that changed them.
class ProfileExtensions : public extensions::ExtensionRegistryObserver,
                          public ToolbarActionsModel::Observer {
 public:
  explicit ProfileExtensions(Profile* profile) : profile_(profile) {
    observation_.Observe(extensions::ExtensionRegistry::Get(profile));
    pinned_observation_.Observe(ToolbarActionsModel::Get(profile));
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

  // ToolbarActionsModel::Observer, for which are pinned:
  void OnToolbarActionAdded(const ToolbarActionsModel::ActionId& id) override {}
  void OnToolbarActionRemoved(const ToolbarActionsModel::ActionId& id) override {}
  void OnToolbarActionUpdated(const ToolbarActionsModel::ActionId& id) override {}
  void OnToolbarModelInitialized() override { Changed(); }
  void OnToolbarPinnedActionsChanged() override { Changed(); }
  void OnToolbarActionsModelShutdown() override { pinned_observation_.Reset(); }

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
    ToolbarActionsModel* model = pinned_observation_.GetSource();
    std::vector<bc_extension> extensions;
    extensions.reserve(listed.size());
    for (const scoped_refptr<const extensions::Extension>& extension : listed) {
      extensions.push_back({.id = extension->id().c_str(),
                            .name = extension->name().c_str(),
                            .enabled = registry->enabled_extensions().Contains(extension->id()),
                            .pinned = model && model->IsActionPinned(extension->id())});
    }
    Host().extensions_changed(Host().context, ProfileId(DirOf(profile_)), extensions.data(),
                              extensions.size());
  }

  raw_ptr<Profile> profile_;
  base::ScopedObservation<extensions::ExtensionRegistry, extensions::ExtensionRegistryObserver>
      observation_{this};
  base::ScopedObservation<ToolbarActionsModel, ToolbarActionsModel::Observer> pinned_observation_{
      this};
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

// One action's icon, which tells the window's actions once it has loaded.
class WindowActions::Icon : public extensions::ExtensionActionIconFactory::Observer {
 public:
  Icon(const extensions::Extension* extension,
       extensions::ExtensionAction* action,
       WindowActions* actions)
      : factory_(extension, action, this), actions_(actions) {}

  gfx::Image Get(int tab) { return factory_.GetIcon(tab); }

 private:
  // extensions::ExtensionActionIconFactory::Observer:
  void OnIconUpdated() override { actions_->Changed(); }

  extensions::ExtensionActionIconFactory factory_;
  raw_ptr<WindowActions> actions_;
};

WindowActions::WindowActions(bc_id window) : window_(window) {}

WindowActions::~WindowActions() = default;

void WindowActions::Show(content::WebContents* contents) {
  contents_ = contents ? contents->GetWeakPtr() : nullptr;
  Profile* profile =
      contents ? Profile::FromBrowserContext(contents->GetBrowserContext()) : nullptr;
  if (profile != profile_) {
    observation_.Reset();
    icons_.clear();
    profile_ = profile;
    if (profile) {
      observation_.Observe(ToolbarActionsModel::Get(profile));
    }
  }
  Changed();
}

void WindowActions::OnToolbarActionAdded(const ToolbarActionsModel::ActionId& id) {
  Changed();
}
void WindowActions::OnToolbarActionRemoved(const ToolbarActionsModel::ActionId& id) {
  icons_.erase(id);
  Changed();
}
void WindowActions::OnToolbarActionUpdated(const ToolbarActionsModel::ActionId& id) {
  Changed();
}
void WindowActions::OnToolbarModelInitialized() {
  Changed();
}
void WindowActions::OnToolbarPinnedActionsChanged() {
  Changed();
}
// The profile is going.
void WindowActions::OnToolbarActionsModelShutdown() {
  observation_.Reset();
  icons_.clear();
  profile_ = nullptr;
  contents_ = nullptr;
  Changed();
}

void WindowActions::Changed() {
  if (report_posted_) {
    return;
  }
  report_posted_ = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&WindowActions::Report, weak_factory_.GetWeakPtr()));
}

void WindowActions::Report() {
  report_posted_ = false;
  if (!Host().window_extension_actions) {
    return;
  }
  std::vector<Told> shown;
  ToolbarActionsModel* model = observation_.GetSource();
  if (contents_ && model && profile_) {
    const int tab = sessions::SessionTabHelper::IdForTab(contents_.get()).id();
    extensions::ExtensionRegistry* registry = extensions::ExtensionRegistry::Get(profile_);
    for (const ToolbarActionsModel::ActionId& id : model->pinned_action_ids()) {
      const extensions::Extension* extension = registry->enabled_extensions().GetByID(id);
      extensions::ExtensionAction* action =
          extension ? extensions::ExtensionActionManager::Get(profile_)->GetExtensionAction(
                          *extension)
                    : nullptr;
      if (!action) {
        continue;
      }
      std::unique_ptr<Icon>& icon = icons_[id];
      if (!icon) {
        icon = std::make_unique<Icon>(extension, action, this);
      }
      const std::string title = action->GetTitle(tab);
      shown.push_back({.id = id,
                       .title = title.empty() ? extension->name() : title,
                       .icon = icon->Get(tab),
                       .badge = action->GetDisplayBadgeText(tab),
                       .badge_color = action->HasBadgeBackgroundColor(tab)
                                          ? action->GetBadgeBackgroundColor(tab)
                                          : 0,
                       .enabled = action->GetIsVisible(tab)});
    }
  }
  auto same = [](const Told& a, const Told& b) {
    return a.id == b.id && a.title == b.title && a.badge == b.badge &&
           a.badge_color == b.badge_color && a.enabled == b.enabled &&
           a.icon.IsEmpty() == b.icon.IsEmpty() &&
           (a.icon.IsEmpty() || a.icon.AsImageSkia().BackedBySameObjectAs(b.icon.AsImageSkia()));
  };
  if (std::ranges::equal(shown, told_, same)) {
    return;
  }
  told_ = std::move(shown);
  std::vector<std::vector<uint8_t>> pngs;
  std::vector<bc_extension_action> actions;
  pngs.reserve(told_.size());
  for (const Told& told : told_) {
    std::optional<std::vector<uint8_t>> png;
    if (!told.icon.IsEmpty()) {
      png = gfx::PNGCodec::EncodeBGRASkBitmap(
          told.icon.AsImageSkia().GetRepresentation(2).GetBitmap(), /*discard_transparency=*/false);
    }
    pngs.push_back(png.value_or(std::vector<uint8_t>()));
    actions.push_back({.id = told.id.c_str(),
                       .title = told.title.c_str(),
                       .icon = pngs.back().empty() ? nullptr : pngs.back().data(),
                       .icon_size = pngs.back().size(),
                       .badge = told.badge.c_str(),
                       .badge_color = told.badge_color,
                       .enabled = told.enabled});
  }
  Host().window_extension_actions(Host().context, window_, actions.data(), actions.size());
}

void ObserveExtensions(Profile* profile) {
  std::unique_ptr<ProfileExtensions>& observed = Observed()[DirOf(profile)];
  observed = std::make_unique<ProfileExtensions>(profile);
}

void ReportExtensions() {
  for (auto& [dir, observed] : Observed()) {
    observed->Changed();
  }
}

void SetExtensionPinned(Profile* profile, const std::string& id, bool pinned) {
  ToolbarActionsModel* model = ToolbarActionsModel::Get(profile);
  if (model && model->HasAction(id) && model->IsActionPinned(id) != pinned) {
    model->SetActionVisibility(id, pinned);
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
