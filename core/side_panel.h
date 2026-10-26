#ifndef BROWSER_CORE_SIDE_PANEL_H_
#define BROWSER_CORE_SIDE_PANEL_H_

#include <memory>
#include <optional>
#include <string>

#include "chrome/browser/ui/side_panel/side_panel_ui_base.h"
#include "content/public/browser/web_contents_observer.h"
#include "extensions/browser/extension_icon_image.h"

class BrowserWindowInterface;

namespace extensions {
class ExtensionSidePanelManager;
}

namespace views {
class View;
}

namespace browser_core {

// A Browser's side panel, which Chrome's extension code opens and closes through SidePanelUI as it
// does Chrome's window's: an extension's page, for the window to show beside the page shown. Its
// views are never drawn; the window shows the page's own native view where the app says.
class SidePanel : public SidePanelUIBase,
                  public content::WebContentsObserver,
                  public extensions::IconImage::Observer {
 public:
  // The side panel of |browser|, one of the core's.
  static SidePanel* From(BrowserWindowInterface* browser);

  explicit SidePanel(BrowserWindowInterface* browser);
  SidePanel(const SidePanel&) = delete;
  SidePanel& operator=(const SidePanel&) = delete;
  ~SidePanel() override;

  // The page shown, or null.
  content::WebContents* contents() const;
  // The extension whose page shows, or empty.
  std::string extension_id() const;
  // Its icon, 16 points square, once it shows.
  gfx::Image icon() const;

  // The width the user last gave extensions' side panels in |browser|'s profile, or 0, and saving
  // a new one, as Chrome's window does.
  int SavedWidth() const;
  void SaveWidth(int width);

  // SidePanelUI:
  using SidePanelUI::Close;
  void Close(SidePanelEntryHideReason reason, bool suppress_animations) override;
  void Toggle(SidePanelEntryKey key, SidePanelOpenTrigger open_trigger) override;
  void ShowFrom(SidePanelEntryKey entry_key, gfx::Rect starting_bounds) override;
  content::WebContents* GetWebContentsForTest(SidePanelEntryId id) override;
  void DisableAnimationsForTesting() override {}
  void SetNoDelaysForTesting(bool no_delays_for_testing) override;

 protected:
  // SidePanelUIBase:
  void Show(const UniqueKey& entry,
            std::optional<SidePanelOpenTrigger> open_trigger,
            bool suppress_animations) override;
  void PopulateSidePanel(bool suppress_animations,
                         const UniqueKey& unique_key,
                         std::optional<SidePanelOpenTrigger> open_trigger,
                         SidePanelEntry* entry,
                         std::optional<SidePanelNativeView> content_view) override;
  void MaybeShowEntryOnTabStripModelChanged(SidePanelRegistry* old_contextual_registry,
                                            SidePanelRegistry* new_contextual_registry) override;

 private:
  // content::WebContentsObserver:
  void TitleWasSet(content::NavigationEntry* entry) override;

  // extensions::IconImage::Observer:
  void OnExtensionIconImageChanged(extensions::IconImage* image) override;

  // Has the window show the panel as it is now.
  void Changed();

  // Registers each extension's side panel for the whole Browser, as Chrome's window does.
  std::unique_ptr<extensions::ExtensionSidePanelManager> extensions_;
  // The view of the entry shown, which holds its page.
  std::unique_ptr<views::View> view_;
  // The icon of the extension shown last, and its id.
  std::unique_ptr<extensions::IconImage> icon_;
  std::string icon_extension_;
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SIDE_PANEL_H_
