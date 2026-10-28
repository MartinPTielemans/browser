#include "browser_core/page_menu.h"

#include <string>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/memory/ptr_util.h"
#include "base/no_destructor.h"
#include "base/strings/string_util.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "components/renderer_context_menu/render_view_context_menu_base.h"
#include "content/public/browser/context_menu_params.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"
#include "ui/base/models/menu_model.h"
#include "ui/gfx/geometry/point.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/vector2d.h"
#include "ui/gfx/text_utils.h"
#include "ui/menus/simple_menu_model.h"
#include "ui/views/widget/widget.h"

namespace browser_core {

namespace {

// The menu shown, or the last, closed.
std::unique_ptr<PageMenu>& Current() {
  static base::NoDestructor<std::unique_ptr<PageMenu>> current;
  return *current;
}

// A row of the menu, for the app.
struct Row {
  int command = 0;
  std::string title;
  int parent = -1;
  bool enabled = false;
  bool checked = false;
};

// A row's title as Chrome's menus show it: without the mnemonic's "&", and with an ellipsis for
// "...", as on the Mac.
std::string Title(const std::u16string& label) {
  std::u16string title = gfx::RemoveAccelerator(label);
  base::ReplaceSubstringsAfterOffset(&title, 0, u"...", u"…");
  return base::UTF16ToUTF8(title);
}

// Adds |model|'s rows that show to |rows|, under the row at |parent| or -1, each submenu's right
// after the row that opens it, with the model and index each came from in |from|. |model| hears
// that it shows first, as Chrome's menus tell it, since some fill in their rows then, and goes in
// |models|. A line goes only between rows, as Chrome's menus draw them.
void AddRows(ui::MenuModel* model,
             int parent,
             std::vector<Row>& rows,
             std::vector<std::pair<raw_ptr<ui::MenuModel>, size_t>>& from,
             std::vector<raw_ptr<ui::MenuModel>>& models) {
  model->MenuWillShow();
  models.push_back(model);
  bool any = false;
  bool line = false;
  for (size_t i = 0; i < model->GetItemCount(); ++i) {
    if (!model->IsVisibleAt(i)) {
      continue;
    }
    const ui::MenuModel::ItemType type = model->GetTypeAt(i);
    if (type == ui::MenuModel::TYPE_SEPARATOR) {
      line = any;
      continue;
    }
    if (line) {
      rows.push_back({.parent = parent});
      from.emplace_back(nullptr, 0);
      line = false;
    }
    any = true;
    const bool check = type == ui::MenuModel::TYPE_CHECK || type == ui::MenuModel::TYPE_RADIO;
    rows.push_back({
        .command = model->GetCommandIdAt(i),
        .title = Title(model->GetLabelAt(i)),
        .parent = parent,
        // A title row names the rows under it, and isn't picked.
        .enabled = type != ui::MenuModel::TYPE_TITLE && model->IsEnabledAt(i),
        .checked = check && model->IsItemCheckedAt(i),
    });
    from.emplace_back(model, i);
    ui::MenuModel* submenu = type == ui::MenuModel::TYPE_SUBMENU ||
                                     type == ui::MenuModel::TYPE_ACTIONABLE_SUBMENU
                                 ? model->GetSubmenuModelAt(i)
                                 : nullptr;
    if (submenu) {
      AddRows(submenu, static_cast<int>(rows.size()) - 1, rows, from, models);
    }
  }
}

}  // namespace

PageMenu::PageMenu(content::WebContents* contents,
                   std::unique_ptr<RenderViewContextMenuBase> menu)
    : content::WebContentsObserver(contents), tab_(Tab::IdOf(contents)), menu_(std::move(menu)) {}

PageMenu::~PageMenu() {
  // One closed whose row hasn't run, as the next menu came first, still tells its page, which
  // would otherwise open no other.
  if (!open_) {
    TellClosed();
  }
}

// static
std::unique_ptr<RenderViewContextMenuBase> PageMenu::Show(
    content::WebContents* contents,
    std::unique_ptr<RenderViewContextMenuBase> menu) {
  Window* window = Tab::FromWebContents(contents) ? Window::Showing(Tab::IdOf(contents)) : nullptr;
  content::RenderWidgetHostView* view = contents->GetTopLevelRenderWidgetHostView();
  if (!window || !window->widget() || !view || !Host().tab_page_menu) {
    return menu;
  }
  // A page that asks again while its menu shows, as a second right-click before it showed can,
  // gets none, as in Chrome.
  if (Current() && Current()->open_) {
    return nullptr;
  }
  // Chrome gives the point in the page's view, which a frame's own point was moved to.
  const gfx::Point point = view->GetViewBounds().origin() +
                           gfx::Vector2d(menu->params().x, menu->params().y) -
                           window->widget()->GetClientAreaBoundsInScreen().OffsetFromOrigin();
  Current() = base::WrapUnique(new PageMenu(contents, std::move(menu)));
  Current()->Report(window->id(), point.x(), point.y());
  return nullptr;
}

void PageMenu::Report(bc_id window, double x, double y) {
  // Chrome's menu runners take the model of the menu they own as this, which the core owns.
  auto& model = const_cast<ui::SimpleMenuModel&>(menu_->menu_model());
  std::vector<Row> rows;
  AddRows(&model, -1, rows, rows_, models_);
  open_ = true;
  std::vector<bc_page_menu_item> items;
  items.reserve(rows.size());
  for (const Row& row : rows) {
    items.push_back({row.command, row.title.c_str(), row.parent, row.enabled, row.checked});
  }
  Host().tab_page_menu(Host().context, window, tab_, x, y, items.data(), items.size());
}

// static
void PageMenu::Close(bc_id tab, int command) {
  PageMenu* menu = Current().get();
  if (!menu || !menu->open_ || menu->tab_ != tab) {
    return;
  }
  menu->open_ = false;
  // Posted, as Chrome's menus run a row once their own loop is over: the app's call comes from
  // inside its update, and a row can call back into the app.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&PageMenu::Run, menu->weak_factory_.GetWeakPtr(), command));
}

void PageMenu::Run(int command) {
  base::WeakPtr<PageMenu> alive = weak_factory_.GetWeakPtr();
  // The row runs before the menu hears it closed, as in Chrome's menus: a page's own row goes to
  // the page through the menu, which closing lets go of.
  for (auto [model, index] : rows_) {
    if (command && model && model->GetCommandIdAt(index) == command &&
        model->GetTypeAt(index) != ui::MenuModel::TYPE_SUBMENU && model->IsEnabledAt(index)) {
      model->ActivatedAt(index);
      break;
    }
  }
  if (alive) {
    TellClosed();
  }
}

void PageMenu::TellClosed() {
  if (std::exchange(told_closed_, true)) {
    return;
  }
  // The submenus first, then the menu, which tells the page its menu closed.
  for (auto it = models_.rbegin(); it != models_.rend(); ++it) {
    (*it)->MenuWillClose();
  }
}

void PageMenu::WebContentsDestroyed() {
  const bool open = open_;
  const bc_id tab = tab_;
  // The page is going, and hears nothing more.
  told_closed_ = true;
  // Before the page goes, as Chrome's goes with the page's view. This is gone after.
  Current().reset();
  if (open && Host().tab_page_menu) {
    Host().tab_page_menu(Host().context, 0, tab, 0, 0, nullptr, 0);
  }
}

}  // namespace browser_core
