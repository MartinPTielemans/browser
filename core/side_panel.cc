#include "browser_core/side_panel.h"

#include <utility>

#include "base/functional/bind.h"
#include "base/values.h"
#include "browser_core/platform/window_views.h"
#include "browser_core/window.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/extensions/extension_side_panel_manager.h"
#include "chrome/browser/ui/side_panel/side_panel_entry.h"
#include "chrome/browser/ui/side_panel/side_panel_entry_id.h"
#include "chrome/browser/ui/side_panel/side_panel_entry_waiter.h"
#include "chrome/browser/ui/side_panel/side_panel_registry.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "chrome/common/pref_names.h"
#include "components/prefs/pref_service.h"
#include "components/prefs/scoped_user_pref_update.h"
#include "content/public/browser/web_contents.h"
#include "extensions/browser/extension_host.h"
#include "extensions/browser/extension_host_registry.h"
#include "extensions/browser/extension_icon_placeholder.h"
#include "extensions/browser/extension_registry.h"
#include "extensions/common/constants.h"
#include "extensions/common/extension.h"
#include "extensions/common/manifest_handlers/icons_handler.h"
#include "ui/base/class_property.h"
#include "ui/views/controls/webview/webview.h"
#include "ui/views/view.h"
#include "ui/views/view_class_properties.h"
#include "ui/views/view_utils.h"
#include "ui/views/widget/widget.h"

namespace browser_core {

namespace {

// On a side panel's view once its page has been asked to load. A view cached and shown again keeps
// its page, and asking again would load it again.
DEFINE_UI_CLASS_PROPERTY_KEY(bool, kPageStarted, false)

content::WebContents* PageOf(views::View* view) {
  views::WebView* web_view = view ? views::AsViewClass<views::WebView>(view) : nullptr;
  return web_view ? web_view->web_contents() : nullptr;
}

}  // namespace

// static
SidePanel* SidePanel::From(BrowserWindowInterface* browser) {
  return static_cast<SidePanel*>(SidePanelUI::From(browser));
}

SidePanel::SidePanel(BrowserWindowInterface* browser)
    : SidePanelUIBase(browser),
      extensions_(std::make_unique<extensions::ExtensionSidePanelManager>(
          browser,
          SidePanelRegistry::From(browser))) {}

SidePanel::~SidePanel() {
  // As the Browser goes, so its tabs may be gone: no entry hears it hide, but the view going tells
  // the extension its panel closed.
  Observe(nullptr);
  SetCurrentKey(std::nullopt);
  view_.reset();
  SidePanelRegistry::From(browser())->ClearCachedEntryViews(/*include_active_entry=*/true);
  extensions_.reset();
}

content::WebContents* SidePanel::contents() const {
  return PageOf(view_.get());
}

std::string SidePanel::extension_id() const {
  return current_key() ? current_key()->key.extension_id().value_or(std::string()) : std::string();
}

gfx::Image SidePanel::icon() const {
  return icon_ ? icon_->image() : gfx::Image();
}

int SidePanel::SavedWidth() const {
  return browser()
      ->GetProfile()
      ->GetPrefs()
      ->GetDict(prefs::kSidePanelIdToWidth)
      .FindInt(SidePanelEntryIdToString(SidePanelEntryId::kExtension))
      .value_or(0);
}

void SidePanel::SaveWidth(int width) {
  ScopedDictPrefUpdate update(browser()->GetProfile()->GetPrefs(), prefs::kSidePanelIdToWidth);
  update.Get().Set(SidePanelEntryIdToString(SidePanelEntryId::kExtension), base::Value(width));
}

void SidePanel::Close(SidePanelEntryHideReason reason, bool suppress_animations) {
  if (!IsSidePanelShowing()) {
    return;
  }
  SidePanelEntry* entry = GetEntryForUniqueKey(*current_key());
  if (entry) {
    entry->OnEntryWillHide(reason);
  }
  // As Chrome's window does once its panel has hidden: the key first, so the entry hearing it
  // hid can't close it again.
  SetCurrentKey(std::nullopt);
  Observe(nullptr);
  if (entry) {
    entry->OnEntryHidden();
  }
  if (SidePanelRegistry* contextual = GetActiveContextualRegistry()) {
    contextual->ResetActiveEntry();
  }
  SidePanelRegistry::From(browser())->ResetActiveEntry();
  // The views of panels still active in other tabs stay, for when those tabs show again.
  SidePanelRegistry::From(browser())->ClearCachedEntryViews();
  for (tabs::TabInterface* tab : *browser()->GetTabStripModel()) {
    if (SidePanelRegistry* registry = SidePanelRegistry::From(tab)) {
      registry->ClearCachedEntryViews();
    }
  }
  view_.reset();
  Changed();
}

void SidePanel::Toggle(SidePanelEntryKey key, SidePanelOpenTrigger open_trigger) {
  if (IsSidePanelEntryShowing(key)) {
    Close();
    return;
  }
  if (std::optional<UniqueKey> unique_key = GetUniqueKeyForKey(key)) {
    Show(*unique_key, open_trigger, /*suppress_animations=*/false);
  }
}

void SidePanel::ShowFrom(SidePanelEntryKey entry_key, gfx::Rect starting_bounds) {
  SidePanelUI::Show(entry_key);
}

content::WebContents* SidePanel::GetWebContentsForTest(SidePanelEntryId id) {
  return contents();
}

void SidePanel::SetNoDelaysForTesting(bool no_delays_for_testing) {
  waiter()->SetNoDelaysForTesting(no_delays_for_testing);
}

void SidePanel::Show(const UniqueKey& input,
                     std::optional<SidePanelOpenTrigger> open_trigger,
                     bool suppress_animations) {
  SidePanelEntry* entry = GetEntryForUniqueKey(input);
  if (!entry || browser()->GetType() != BrowserWindowInterface::Type::TYPE_NORMAL) {
    return;
  }
  if (IsSidePanelShowing() && *current_key() == input) {
    waiter()->ResetLoadingEntryIfNecessary();
    return;
  }
  waiter()->WaitForEntry(entry, base::BindOnce(&SidePanel::PopulateSidePanel,
                                               base::Unretained(this), suppress_animations, input,
                                               open_trigger));
}

void SidePanel::PopulateSidePanel(bool suppress_animations,
                                  const UniqueKey& unique_key,
                                  std::optional<SidePanelOpenTrigger> open_trigger,
                                  SidePanelEntry* entry,
                                  std::optional<SidePanelNativeView> content_view) {
  Window* window = Window::ForBrowser(browser());
  SidePanelNativeView view =
      content_view ? std::move(content_view.value()) : entry->GetContent();
  if (!window || !view) {
    return;
  }
  entry->set_last_open_trigger(open_trigger);
  const bool tab_changed = open_trigger == SidePanelOpenTrigger::kTabChanged;
  SidePanelEntry* previous = IsSidePanelShowing() ? GetEntryForUniqueKey(*current_key()) : nullptr;
  if (previous && view_) {
    previous->OnEntryWillHide(tab_changed ? SidePanelEntryHideReason::kBackgrounded
                                          : SidePanelEntryHideReason::kReplaced);
    previous->CacheView(std::move(view_));
  }
  view_ = std::move(view);
  // Never in a widget, so key events reach the window's focus manager through this, as WebUI's
  // browser window does for its panel.
  view_->SetProperty(views::kDetachedViewFocusManagerKey, window->widget()->GetFocusManager());
  if (GetActiveContextualRegistry()) {
    GetActiveContextualRegistry()->ResetActiveEntry();
  }
  SetCurrentKey(unique_key);

  content::WebContents* page = contents();
  Observe(page);
  // A view in a widget starts its page as it's attached; this one never is.
  if (page && !view_->GetProperty(kPageStarted)) {
    view_->SetProperty(kPageStarted, true);
    if (extensions::ExtensionHost* host =
            extensions::ExtensionHostRegistry::Get(page->GetBrowserContext())
                ->GetExtensionHostForPrimaryMainFrame(page->GetPrimaryMainFrame())) {
      host->CreateRendererSoon();
    }
  }
  const std::string id = extension_id();
  const extensions::Extension* extension =
      extensions::ExtensionRegistry::Get(browser()->GetProfile())
          ->enabled_extensions()
          .GetByID(id);
  if (extension && (!icon_ || icon_extension_ != id)) {
    icon_extension_ = id;
    icon_ = std::make_unique<extensions::IconImage>(
        browser()->GetProfile(), extension, extensions::IconsInfo::GetIcons(extension),
        extension_misc::EXTENSION_ICON_BITTY,
        extensions::ExtensionIconPlaceholder::CreateImage(extension_misc::EXTENSION_ICON_BITTY,
                                                          extension->name())
            .AsImageSkia(),
        this);
  }
  // In the window before the entry hears it shows, as it gives its page the keyboard then.
  Changed();
  if (tab_changed) {
    // Chrome's window gives the keyboard back to the tab after a tab switch; this one leaves it
    // where it was, for the window to give the tab shown.
    platform::KeepKeyboard(window->widget(), base::BindOnce(&SidePanelEntry::OnEntryShown,
                                                            base::Unretained(entry)));
  } else {
    entry->OnEntryShown();
  }
  if (previous) {
    previous->OnEntryHidden();
  }
  NotifyShownCallbacks();
}

void SidePanel::MaybeShowEntryOnTabStripModelChanged(SidePanelRegistry* old_contextual_registry,
                                                     SidePanelRegistry* new_contextual_registry) {
  // As Chrome's window: the new tab's panel, else the panel shown if it's for every tab, else
  // none.
  if (IsSidePanelShowing()) {
    if (std::optional<UniqueKey> unique_key = GetNewActiveKeyOnTabChanged()) {
      Show(*unique_key, SidePanelOpenTrigger::kTabChanged, /*suppress_animations=*/true);
      return;
    }
    // The old tab's panel keeps its page, for when that tab shows again.
    std::optional<SidePanelEntry*> active =
        old_contextual_registry ? old_contextual_registry->GetActiveEntry() : std::nullopt;
    if (active && current_key()->tab_handle && (*active)->key() == current_key()->key && view_) {
      Observe(nullptr);
      (*active)->CacheView(std::move(view_));
    }
    Close(SidePanelEntryHideReason::kBackgrounded, /*suppress_animations=*/true);
  } else if (std::optional<SidePanelEntry*> active =
                 new_contextual_registry ? new_contextual_registry->GetActiveEntry()
                                         : std::nullopt) {
    Show({browser()->GetActiveTabInterface()->GetHandle(), (*active)->key()},
         SidePanelOpenTrigger::kTabChanged, /*suppress_animations=*/true);
  }
}

void SidePanel::TitleWasSet(content::NavigationEntry* entry) {
  if (Window* window = Window::ForBrowser(browser())) {
    window->ReportSidePanelSoon();
  }
}

void SidePanel::OnExtensionIconImageChanged(extensions::IconImage* image) {
  if (Window* window = Window::ForBrowser(browser())) {
    window->ReportSidePanelSoon();
  }
}

void SidePanel::Changed() {
  if (Window* window = Window::ForBrowser(browser())) {
    window->SidePanelChanged(browser());
  }
}

}  // namespace browser_core
