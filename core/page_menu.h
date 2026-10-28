#ifndef BROWSER_CORE_PAGE_MENU_H_
#define BROWSER_CORE_PAGE_MENU_H_

#include <memory>
#include <utility>
#include <vector>

#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "browser_core/public/browser_core.h"
#include "content/public/browser/web_contents_observer.h"

class RenderViewContextMenuBase;

namespace ui {
class MenuModel;
}

namespace browser_core {

// A page's context menu, which the app shows in its own menu in place of Chrome's (tab_page_menu).
// Chrome builds it as for its own, with extension items, spelling suggestions and Inspect, and the
// core runs the item the user picks through its model, as Chrome's menu runners do. One shows at a
// time. The last stays, closed, until the next or its page goes, as Chrome keeps its own, since an
// item picked may finish its work later.
class PageMenu : public content::WebContentsObserver {
 public:
  // g_browser_core_show_page_menu: takes |menu| for the app to show, for a tab a window shows, or
  // gives it back for Chrome to show, as for DevTools.
  static std::unique_ptr<RenderViewContextMenuBase> Show(
      content::WebContents* contents,
      std::unique_ptr<RenderViewContextMenuBase> menu);
  // See bc_tab_close_page_menu.
  static void Close(bc_id tab, int command);

  PageMenu(const PageMenu&) = delete;
  PageMenu& operator=(const PageMenu&) = delete;
  ~PageMenu() override;

 private:
  PageMenu(content::WebContents* contents, std::unique_ptr<RenderViewContextMenuBase> menu);

  // Tells the app the menu, its rows in order, each submenu's after the row that opens it.
  void Report(bc_id window, double x, double y);
  // Runs the row with |command|, if any, then tells the menu it closed.
  void Run(int command);
  // Tells the menu's models it closed, once, as Chrome's menus do, which tells the page.
  void TellClosed();

  // content::WebContentsObserver:
  void WebContentsDestroyed() override;

  const bc_id tab_;
  std::unique_ptr<RenderViewContextMenuBase> menu_;
  // Each row the app was told, by its index there: its model and its index in it.
  std::vector<std::pair<raw_ptr<ui::MenuModel>, size_t>> rows_;
  // The menu's model and each submenu's that the app was told, the root first, which hear that
  // the menu closes, as they heard it show.
  std::vector<raw_ptr<ui::MenuModel>> models_;
  bool open_ = false;
  // Its models heard that it closed, or its page went.
  bool told_closed_ = false;
  base::WeakPtrFactory<PageMenu> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_PAGE_MENU_H_
