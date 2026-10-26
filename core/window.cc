#include "browser_core/window.h"

#include <algorithm>
#include <iterator>
#include <numeric>
#include <optional>
#include <set>
#include <utility>
#include <vector>

#include "base/auto_reset.h"
#include "base/functional/bind.h"
#include "base/memory/ptr_util.h"
#include "base/no_destructor.h"
#include "base/numerics/safe_conversions.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "browser_core/archive.h"
#include "browser_core/auto_archive.h"
#include "browser_core/browser_window_stand_in.h"
#include "browser_core/extensions.h"
#include "browser_core/favorites.h"
#include "browser_core/host.h"
#include "browser_core/platform/window_views.h"
#include "browser_core/profiles.h"
#include "browser_core/session.h"
#include "browser_core/space.h"
#include "browser_core/suggest.h"
#include "browser_core/tab.h"
#include "chrome/app/chrome_command_ids.h"
#include "chrome/browser/devtools/devtools_contents_resizing_strategy.h"
#include "chrome/browser/devtools/devtools_window.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/browser/tab_list/tab_removed_reason.h"
#include "chrome/browser/ui/browser_tabrestore.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/browser_window/public/create_browser_window.h"
#include "chrome/browser/ui/browser_window/public/desktop_browser_window_capabilities.h"
#include "chrome/browser/ui/interaction/browser_elements.h"
#include "chrome/browser/ui/navigator/browser_navigator.h"
#include "chrome/browser/ui/navigator/browser_navigator_params.h"
#include "chrome/browser/ui/navigator/browser_navigator_params_utils.h"
#include "chrome/browser/ui/tabs/tab_enums.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "chrome/browser/ui/views/interaction/browser_elements_views_impl.h"
#include "chrome/common/webui_url_constants.h"
#include "components/sessions/core/serialized_navigation_entry.h"
#include "components/sessions/core/serialized_user_agent_override.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/web_contents.h"
#include "content/public/common/url_constants.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/base/page_transition_types.h"
#include "ui/base/window_open_disposition.h"
#include "ui/menus/simple_menu_model.h"
#include "ui/views/view.h"
#include "ui/views/view_tracker.h"
#include "ui/views/widget/widget.h"
#include "ui/views/widget/widget_delegate.h"
#include "ui/views/window/client_view.h"

namespace browser_core {

namespace {

std::map<bc_id, std::unique_ptr<Window>>& Windows() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Window>>> windows;
  return *windows;
}

// A Today tab the user or its page closed.
struct ClosedTab {
  // The id it had.
  bc_id id = 0;
  bc_id space = 0;
  // Its place in the Space's Today tabs.
  size_t index = 0;
  SavedTab saved;
  // A tab of the split view it was in, else 0, its place in the split, and whether the split was
  // stacked.
  bc_id split_with = 0;
  size_t split_index = 0;
  bool stacked = false;
};

// As many as Chrome keeps.
constexpr size_t kMaxClosedTabs = 25;

// Between the pages of a split view, as in Arc.
constexpr int kSplitGap = 10;

// The Spaces going once their last tab closes.
std::set<bc_id>& DeletingSpaces() {
  static base::NoDestructor<std::set<bc_id>> deleting;
  return *deleting;
}

// Newest last.
std::vector<ClosedTab>& ClosedTabs() {
  static base::NoDestructor<std::vector<ClosedTab>> closed;
  return *closed;
}

// The Space deleted last, for UndoDeleteSpace.
struct DeletedSpace {
  bc_id id = 0;
  SavedSpace saved;
  // The Today tabs saved, which went to the archive.
  std::vector<bc_id> today;
  // The tab or favorite the window showed there, or 0.
  bc_id shown = 0;
};

std::optional<DeletedSpace>& LastDeleted() {
  static base::NoDestructor<std::optional<DeletedSpace>> deleted;
  return *deleted;
}

// Keeps |contents|, a tab closing, for ReopenClosedTab, and archives it.
void KeepClosed(content::WebContents* contents) {
  const bc_id id = Tab::IdOf(contents);
  Space* space = Space::Holding(id);
  // One not in a Space never showed, and a pinned tab closes only when its page closes itself.
  if (!space || space->IsPinned(id)) {
    return;
  }
  SavedTab saved = SaveTab(*contents);
  if (saved.history.empty()) {
    return;
  }
  saved.name = Tab::FromWebContents(contents)->name();
  saved.muted = contents->IsAudioMuted();
  Archive(id, saved.history[saved.current].virtual_url(), Tab::FromWebContents(contents)->Title());
  const size_t index = std::ranges::find(space->today(), id) - space->today().begin();
  const std::vector<bc_id> panes = space->Panes(id);
  const size_t split_index = std::ranges::find(panes, id) - panes.begin();
  ClosedTabs().push_back({.id = id,
                          .space = space->id(),
                          .index = index,
                          .saved = std::move(saved),
                          .split_with = panes.size() < 2 ? 0 : panes[split_index == 0 ? 1 : 0],
                          .split_index = split_index,
                          .stacked = space->Stacked(id)});
  if (ClosedTabs().size() > kMaxClosedTabs) {
    ClosedTabs().erase(ClosedTabs().begin());
  }
}

// The closed tab |id|, taken from those ReopenClosedTab brings back, and from the archive.
std::optional<ClosedTab> TakeClosed(bc_id id) {
  auto closed = std::ranges::find(ClosedTabs(), id, &ClosedTab::id);
  if (closed == ClosedTabs().end()) {
    return std::nullopt;
  }
  ClosedTab tab = std::move(*closed);
  ClosedTabs().erase(closed);
  Unarchive(id);
  return tab;
}

// A new tab in |dir|'s profile with |tab|'s history, name and mute, in no Space and not shown, or 0
// if the profile isn't loaded.
bc_id CopyToProfile(bc_id tab, const std::string& dir) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  Window* window = in ? Window::ForBrowser(in->GetBrowserWindowInterface()) : nullptr;
  Profile* profile = ProfileFor(dir);
  if (!window || !profile || DirOf(profile) != dir) {
    return 0;
  }
  const SavedTab saved = SaveTab(*contents);
  content::WebContents* copy =
      saved.history.empty() ? nullptr : window->RestoreTab(profile, saved.history, saved.current);
  if (!copy) {
    return 0;
  }
  const bc_id id = Tab::IdOf(copy);
  Tab::FromWebContents(copy)->set_name(Tab::FromWebContents(contents)->name());
  if (contents->IsAudioMuted()) {
    Tab::FromWebContents(copy)->SetMuted(true);
  }
  Tab::FromWebContents(copy)->Restored();
  return id;
}

// |contents|, leaving a window, was seen until now.
void Seen(content::WebContents* contents) {
  if (Tab* tab = contents ? Tab::FromWebContents(contents) : nullptr) {
    tab->set_last_seen(base::Time::Now());
  }
  ScheduleAutoArchive();
}

}  // namespace

// Makes the window's close button Window::Close, so pages get their unload handlers first.
class Window::ClientView : public views::ClientView {
 public:
  ClientView(views::Widget* widget, views::View* contents, base::WeakPtr<Window> window)
      : views::ClientView(widget, contents), window_(std::move(window)) {}

  views::CloseRequestResult OnWindowCloseRequested() override {
    // Posted, since a window without Browsers closes its widget at once, and the widget is asking.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::Close, window_));
    return views::CloseRequestResult::kCannotClose;
  }

 private:
  base::WeakPtr<Window> window_;
};

// static
Window* Window::Open(const gfx::Rect& bounds, void* ui_view) {
  if (browser_shutdown::IsTryingToQuit()) {
    return nullptr;
  }
  const bc_id id = NewId();
  Window* window = new Window(id, bounds, ui_view);
  Windows()[id] = base::WrapUnique(window);
  return window;
}

// static
Window* Window::FromId(bc_id id) {
  auto it = Windows().find(id);
  return it == Windows().end() ? nullptr : it->second.get();
}

// static
std::vector<Window*> Window::All() {
  std::vector<Window*> all;
  for (auto& [id, window] : Windows()) {
    all.push_back(window.get());
  }
  return all;
}

// static
Window* Window::ForBrowser(const BrowserWindowInterface* browser) {
  for (auto& [id, window] : Windows()) {
    for (auto& [profile, held] : window->browsers_) {
      if (held.browser == browser) {
        return window.get();
      }
    }
  }
  return nullptr;
}

// static
void Window::CloseAll() {
  while (!Windows().empty()) {
    Windows().begin()->second->Destroy();
  }
}

// static
bool Window::Showing(bc_id tab) {
  return std::ranges::any_of(Windows(), [tab](const auto& entry) {
    return std::ranges::contains(entry.second->panes_, tab);
  });
}

// static
void Window::PanesChanged() {
  for (auto& [id, window] : Windows()) {
    const bc_id selected = window->selected_;
    // A favorite is in no split.
    Space* space = Space::Holding(selected);
    if (selected && (space ? space->Panes(selected) : std::vector<bc_id>{selected}) !=
                        window->panes_) {
      window->Select(selected);
    } else if (space && (space->Stacked(selected) != window->stacked_ ||
                         space->Shares(selected) != window->shares_)) {
      window->LayOutPage();
    }
  }
}

// static
void Window::OnPageClicked(bc_id tab) {
  for (auto& [id, window] : Windows()) {
    if (window->selected_ != tab && std::ranges::contains(window->panes_, tab)) {
      window->Select(tab);
    }
  }
}

// static
bool Window::CanDockDevTools(BrowserWindowInterface* browser) {
  return ForBrowser(browser);
}

// static
void Window::UpdateDevTools(BrowserWindowInterface* browser, content::WebContents* inspected) {
  Window* window = ForBrowser(browser);
  if (window && std::ranges::contains(window->panes_, Tab::IdOf(inspected))) {
    window->LayOutPage();
    // DevTools closed with the keyboard leave it with the window, and the page takes it back.
    window->FocusPageSoon();
  }
}

// static
void Window::TrimPageMenu(BrowserWindowInterface* browser, ui::SimpleMenuModel& menu) {
  if (!ForBrowser(browser)) {
    return;
  }
  // Items for what the browser doesn't have yet, and items whose UI is in Chrome's own window, a
  // toolbar bubble or the side panel, which the core's windows don't have.
  static constexpr int kTrimmed[] = {
      IDC_CONTENT_CONTEXT_OPENLINKNEWWINDOW,    IDC_CONTENT_CONTEXT_OPENLINKOFFTHERECORD,
      IDC_ROUTE_MEDIA,                          IDC_CONTENT_CONTEXT_OPEN_IN_READING_MODE,
      IDC_CONTENT_CONTEXT_LISTEN_TO_THIS_PAGE,  IDC_CONTENT_CONTEXT_GENERATE_QR_CODE,
      IDC_CONTENT_CONTEXT_TRANSLATE,            IDC_CONTENT_CONTEXT_PARTIAL_TRANSLATE,
  };
  for (int id : kTrimmed) {
    if (std::optional<size_t> index = menu.GetIndexOfCommandId(id)) {
      menu.RemoveItemAt(*index);
    }
  }
  // A section left empty leaves a separator at the top or two together. Chrome takes off the last.
  for (size_t i = 0; i < menu.GetItemCount();) {
    if (menu.GetTypeAt(i) == ui::MenuModel::TYPE_SEPARATOR &&
        (i == 0 || menu.GetTypeAt(i - 1) == ui::MenuModel::TYPE_SEPARATOR)) {
      menu.RemoveItemAt(i);
    } else {
      ++i;
    }
  }
}

// static
void Window::AdjustNavigate(NavigateParams& params) {
  Window* window = params.browser ? ForBrowser(params.browser) : nullptr;
  if (!window) {
    return;
  }
  // ⌥⌘-click on a link opens it beside its page in a split view, as Chrome's does, and so does
  // ⌥⇧-click, as Dia's does, which the page asks for as a new window.
  constexpr int kAltShift = blink::WebInputEvent::kAltKey | blink::WebInputEvent::kShiftKey;
  Tab* source = params.source_contents ? Tab::FromWebContents(params.source_contents) : nullptr;
  const bool split = source && (params.disposition == WindowOpenDisposition::NEW_SPLIT_VIEW ||
                                (params.disposition == WindowOpenDisposition::NEW_WINDOW &&
                                 (source->input_modifiers() & kAltShift) == kAltShift));
  window->split_with_ = split ? Tab::IdOf(params.source_contents) : 0;
  // Popups become tabs (decision 3). So do new windows: the app opens its own.
  if (split || params.disposition == WindowOpenDisposition::NEW_POPUP ||
      params.disposition == WindowOpenDisposition::NEW_WINDOW) {
    params.disposition = WindowOpenDisposition::NEW_FOREGROUND_TAB;
  }
  // A link a page opens behind, as by ⌘-click, loads only once its tab is first shown (Dia). A
  // form sent to a tab behind goes at once, when the user sent it, and a window the page made
  // itself already has its page and the page's hold on it.
  if (source && params.disposition == WindowOpenDisposition::NEW_BACKGROUND_TAB &&
      params.url.SchemeIsHTTPOrHTTPS() && !params.post_data && !params.contents_to_insert) {
    window->OpenBehind(params);
    params.disposition = WindowOpenDisposition::IGNORE_ACTION;
  }
  // Chrome's downloads page, which it opens after the user stopped a quit to let downloads finish,
  // is the Library's Downloads here.
  if (params.url.SchemeIs(content::kChromeUIScheme) &&
      params.url.host() == chrome::kChromeUIDownloadsHost) {
    params.disposition = WindowOpenDisposition::IGNORE_ACTION;
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<Window> window) {
                         if (window && Host().window_show_downloads) {
                           Host().window_show_downloads(Host().context, window->id());
                         }
                       },
                       window->weak_factory_.GetWeakPtr()));
  }
}

// static
void Window::OnTabClosing(bc_id tab) {
  Space* space = Space::Holding(tab);
  if (!space && !Favorites::Has(tab)) {
    return;
  }
  // One closing with its window, or as the browser quits, comes back at the next launch.
  if (browser_shutdown::IsTryingToQuit() ||
      std::ranges::any_of(Windows(), [](const auto& entry) { return entry.second->closing_; })) {
    KeepSession();
  }
  const bc_id next = space ? space->NextAfterLeaving(tab, &Tab::Loaded) : 0;
  if (space) {
    space->Remove(tab);
  } else {
    Favorites::Remove(tab);
  }
  for (auto& [id, window] : Windows()) {
    if (window->selected_ == tab) {
      window->selected_ = 0;
      Space* shown = space ? nullptr : Space::FromId(window->space_);
      window->next_after_closing_ = shown ? shown->NextAfterLeaving(tab, &Tab::Loaded) : next;
      // Posted: Chrome is part way through closing the tab, and its tab strip can't change yet.
      base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
          FROM_HERE,
          base::BindOnce(&Window::SelectAfterClosing, window->weak_factory_.GetWeakPtr()));
    } else if (!window->selected_ && window->next_after_closing_ == tab) {
      // The tab it was to show is closing too, as a split view's tabs close together.
      window->next_after_closing_ = next;
    }
  }
  // A window showing it beside the tab it shows shows the rest of the split.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Window::PanesChanged));
  if (space && space->pinned().empty() && space->today().empty() &&
      DeletingSpaces().erase(space->id())) {
    Space::Delete(space->id());
  }
}

// static
void Window::Unload(bc_id tab, bool split) {
  Space* space = Space::Holding(tab);
  if ((space ? !space->IsPinned(tab) : !Favorites::Has(tab)) || !Tab::FromId(tab)) {
    return;
  }
  const std::vector<bc_id> going = space && split ? space->Panes(tab) : std::vector<bc_id>{tab};
  const auto staying = [&going](bc_id other) {
    return !std::ranges::contains(going, other) && Tab::Loaded(other);
  };
  const bc_id next = space ? space->NextAfterLeaving(tab, staying) : 0;
  // Out of its split, so showing the rest of it doesn't load it again.
  if (space && !split) {
    space->Separate(tab);
  }
  // Another tab first, so the pages unload out of sight.
  for (auto& [id, window] : Windows()) {
    if (std::ranges::contains(going, window->selected_)) {
      Space* shown = space ? nullptr : Space::FromId(window->space_);
      if (const bc_id its_next = shown ? shown->NextAfterLeaving(tab, staying) : next) {
        window->Select(its_next);
      } else {
        window->ShowNothing();
      }
    }
    // Nor does showing another Space that showed it last.
    std::erase_if(window->selected_in_space_, [&going](const auto& entry) {
      return std::ranges::contains(going, entry.second);
    });
  }
  PanesChanged();
  for (bc_id each : going) {
    if (content::WebContents* contents = Tab::FromId(each)) {
      Tab::FromWebContents(contents)->Unload();
    }
  }
}

// static
void Window::MoveToFavorites(bc_id tab, size_t index) {
  if (Favorites::Has(tab)) {
    Favorites::Move(tab, index);
    return;
  }
  content::WebContents* contents = Tab::FromId(tab);
  Space* space = Space::Holding(tab);
  if (!contents || !space) {
    return;
  }
  // In the favorites before it leaves its Space, so the UI never hears of it in neither and
  // forgets its title.
  Favorites::Add(tab, space->IsPinned(tab) ? space->PinnedUrl(tab) : Tab::PageUrl(contents),
                 index);
  space->Remove(tab);
  // A window showing it beside the tab it shows shows the rest of the split.
  PanesChanged();
  // Whether it's away from its URL.
  Tab::FromWebContents(contents)->StateChanged();
}

// static
void Window::Unfavorite(bc_id tab, bool pinned, bc_id folder, size_t index) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  Window* window = in ? ForBrowser(in->GetBrowserWindowInterface()) : nullptr;
  Space* shown = window ? Space::FromId(window->space_) : nullptr;
  if (!shown || !Favorites::Has(tab) || Favorites::ProfileOf(tab) != shown->profile()) {
    return;
  }
  // In the Space before it leaves the favorites, as above.
  if (pinned) {
    shown->AddPinned(tab, folder, index, Favorites::Url(tab));
  } else {
    shown->AddAt(tab, index);
  }
  Favorites::Remove(tab);
  // So it isn't archived at once.
  Tab::FromWebContents(contents)->set_last_seen(base::Time::Now());
  Tab::FromWebContents(contents)->StateChanged();
}

// static
void Window::SetSpaceProfile(bc_id space, const std::string& profile) {
  Space* in = Space::FromId(space);
  if (!in || !in->pinned_rows().empty() || !in->today().empty() || in->profile() == profile) {
    return;
  }
  in->SetProfile(profile);
  // A favorite of the profile it was in no longer shows there.
  for (auto& [id, window] : Windows()) {
    if (Favorites::Has(window->SelectedIn(space)) &&
        Favorites::ProfileOf(window->SelectedIn(space)) != profile) {
      window->selected_in_space_.erase(space);
      if (window->space_ == space) {
        window->ShowNothing();
      }
    }
  }
}

// static
void Window::CloseTab(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!in) {
    return;
  }
  TabStripModel* tabs = in->GetBrowserWindowInterface()->GetTabStripModel();
  tabs->CloseWebContentsAt(tabs->GetIndexOfWebContents(contents),
                           TabCloseTypes::CLOSE_USER_GESTURE);
}

// static
void Window::CloseBelow(bc_id tab) {
  Space* space = Space::Holding(tab);
  if (!space || space->IsPinned(tab)) {
    return;
  }
  const std::vector<bc_id>& today = space->today();
  // Below its split, if it's in one.
  const std::vector<bc_id> below(std::ranges::find(today, space->Panes(tab).back()) + 1,
                                 today.end());
  for (auto& [id, window] : Windows()) {
    if (std::ranges::contains(below, window->selected_)) {
      window->Select(tab);
    }
  }
  for (auto id = below.rbegin(); id != below.rend(); ++id) {
    CloseTab(*id);
  }
}

// static
void Window::MoveToSpace(bc_id tab, bc_id space) {
  Space* from = Space::Holding(tab);
  Space* to = Space::FromId(space);
  if (!from || !to || from == to || DeletingSpaces().contains(space)) {
    return;
  }
  // A page can't take its cookies and logins to another profile, so it opens there again.
  const bc_id moved = from->profile() == to->profile() ? tab : CopyToProfile(tab, to->profile());
  if (!moved) {
    return;
  }
  const bool pinned = from->IsPinned(tab);
  const GURL url = from->PinnedUrl(tab);
  const bc_id next = from->NextAfterLeaving(tab, &Tab::Loaded);
  // In its new Space before it leaves the old, so the UI never hears of it in neither and forgets
  // its title.
  if (pinned) {
    to->AddPinned(moved, 0, to->pinned_rows().size(), url);
  } else {
    to->AddAt(moved, 0);
  }
  from->Remove(tab);
  for (auto& [id, window] : Windows()) {
    if (window->selected_ == tab) {
      // ShowSpace keeps |next| as the tab the old Space shows.
      window->selected_ = next;
      window->SetSelectedIn(space, moved);
      window->ShowSpace(space);
    } else if (window->SelectedIn(from->id()) == tab) {
      window->SetSelectedIn(from->id(), next);
    }
  }
  // Out of its Space, it isn't kept for ReopenClosedTab or archived.
  if (moved != tab) {
    CloseTab(tab);
  }
  PanesChanged();
}

// static
bool Window::DeleteSpace(bc_id space) {
  std::vector<Space*> staying = Space::All();
  std::erase_if(staying, [](Space* each) { return DeletingSpaces().contains(each->id()); });
  auto it = std::ranges::find(staying, space, &Space::id);
  if (it == staying.end() || staying.size() < 2) {
    return false;
  }
  Space* neighbour = it != staying.begin() ? *(it - 1) : *(it + 1);
  // Before the windows leave it, so it keeps the tab the first one showed there.
  DeletedSpace& deleted = LastDeleted().emplace();
  deleted.id = space;
  deleted.shown = Windows().empty() ? 0 : Windows().begin()->second->SelectedIn(space);
  deleted.saved = SaveSpace(**it, deleted.shown, &deleted.today);
  for (auto& [id, window] : Windows()) {
    if (window->space_ == space) {
      window->ShowSpace(neighbour->id());
    }
    window->selected_in_space_.erase(space);
  }
  std::vector<bc_id> tabs = (*it)->pinned();
  std::ranges::copy((*it)->today(), std::back_inserter(tabs));
  if (tabs.empty()) {
    Space::Delete(space);
    return true;
  }
  DeletingSpaces().insert(space);
  // As the user closes them, so ⇧⌘T reopens the Today tabs. The last one deletes the Space.
  for (bc_id tab : tabs) {
    CloseTab(tab);
  }
  return true;
}

Window::Window(bc_id id, const gfx::Rect& bounds, void* ui_view)
    : id_(id), sidebar_width_(SavedSidebarWidth()) {
  delegate_ = std::make_unique<views::WidgetDelegate>();
  delegate_->SetCanFullscreen(true);
  delegate_->SetCanResize(true);
  delegate_->SetCanMaximize(true);
  delegate_->SetCanMinimize(true);
  delegate_->SetClientViewFactory(base::BindOnce(
      [](base::WeakPtr<Window> window, views::Widget* widget,
         views::View* contents) -> std::unique_ptr<views::ClientView> {
        return std::make_unique<ClientView>(widget, contents, std::move(window));
      },
      weak_factory_.GetWeakPtr()));

  widget_ = std::make_unique<views::Widget>();
  views::Widget::InitParams params(views::Widget::InitParams::CLIENT_OWNS_WIDGET,
                                   views::Widget::InitParams::TYPE_WINDOW);
  params.delegate = delegate_.get();
  params.native_widget = platform::NewNativeWidget(widget_.get());
  params.bounds = bounds;
  params.name = "BrowserCoreWindow";
  widget_->Init(std::move(params));
  widget_observation_.Observe(widget_.get());
  platform::AddWindowViews(widget_.get(), ui_view);
  widget_->Show();
}

Window::~Window() {
  for (web_modal::ModalDialogHostObserver& observer : dialog_observers_) {
    observer.OnHostDestroying();
  }
}

BrowserWindowInterface* Window::BrowserFor(Profile* profile) {
  if (closing_ || browser_shutdown::IsTryingToQuit()) {
    return nullptr;
  }
  if (auto it = browsers_.find(profile); it != browsers_.end()) {
    BrowserWindowInterface* browser = it->second.browser;
    // One closes once its last tab has, and the profile's next tab goes in a new one.
    if (!browser->IsDeleteScheduled() && !browser->capabilities()->IsAttemptingToCloseBrowser()) {
      return browser;
    }
  }
  if (GetBrowserWindowCreationStatusForProfile(*profile) !=
      BrowserWindowInterface::CreationStatus::kOk) {
    return nullptr;
  }

  // The Browser owns its window and deletes it as it goes.
  auto* stand_in = new BrowserWindowStandIn(weak_factory_.GetWeakPtr());
  BrowserWindowCreateParams params(BrowserWindowInterface::TYPE_NORMAL, *profile,
                                   /*from_user_gesture=*/false);
  params.window = stand_in;
  // The app keeps the tabs it restores, in Spaces, so Chrome's sessions stay out of it.
  params.omit_from_session_restore = true;
  params.should_trigger_session_restore = false;
  BrowserWindowInterface* browser = CreateBrowserWindow(std::move(params));
  stand_in->SetBrowser(browser);
  browser->GetTabStripModel()->AddObserver(this);
  // Chrome's bubbles find the widget to anchor to through this.
  if (auto* elements = BrowserElements::From(browser)->AsA<BrowserElementsViewsImpl>()) {
    elements->Init(widget_->GetRootView());
  }
  browsers_[profile] = Held{
      browser, browser->RegisterBrowserCloseCancelled(base::BindRepeating(
                   [](base::WeakPtr<Window> window, BrowserWindowInterface*,
                      BrowserWindowInterface::ClosingStatus) {
                     // A page's beforeunload handler or the user kept it open.
                     if (window) {
                       window->closing_ = false;
                     }
                   },
                   weak_factory_.GetWeakPtr()))};
  return browser;
}

content::WebContents* Window::RestoreTab(
    Profile* profile,
    base::span<const sessions::SerializedNavigationEntry> history,
    int current) {
  BrowserWindowInterface* browser = BrowserFor(profile);
  if (!browser) {
    return nullptr;
  }
  base::AutoReset<bool> opening(&opening_tab_, true);
  return chrome::AddRestoredTab(
      browser, history, browser->GetTabStripModel()->count(), current, /*extension_app_id=*/"",
      /*group=*/std::nullopt, /*select=*/false, /*pin=*/false, base::TimeTicks(), base::Time(),
      /*storage_namespace=*/nullptr, sessions::SerializedUserAgentOverride(),
      /*extra_data=*/{}, /*from_session_restore=*/true, /*is_active_browser=*/std::nullopt);
}

void Window::Suggest(Profile* profile,
                     const std::u16string& keyword,
                     const std::u16string& text,
                     bool complete) {
  if (!suggestions_ || suggestions_->profile() != profile) {
    suggestions_ = std::make_unique<Suggestions>(id_, profile);
  }
  suggestions_->Start(keyword, text, complete);
}

void Window::RemoveSuggestion(const GURL& url) {
  if (suggestions_) {
    suggestions_->Remove(url);
  }
}

content::WebContents* Window::OpenTab(Profile* profile, const GURL& url) {
  BrowserWindowInterface* browser = BrowserFor(profile);
  if (!browser) {
    return nullptr;
  }
  NavigateParams params(browser, url, ui::PAGE_TRANSITION_TYPED);
  params.disposition = WindowOpenDisposition::NEW_BACKGROUND_TAB;
  // The app hears only of the tabs it didn't open.
  base::AutoReset<bool> opening(&opening_tab_, true);
  Navigate(&params);
  return params.navigated_or_inserted_contents;
}

void Window::Show(content::WebContents* contents, const gfx::Rect& frame) {
  // The Browser's active tab is the one Chrome's keyboard shortcuts, find and fullscreen act on.
  TabStripModel* tabs =
      tabs::TabInterface::GetFromContents(contents)->GetBrowserWindowInterface()->GetTabStripModel();
  tabs->ActivateTabAt(tabs->GetIndexOfWebContents(contents));
  const bc_id tab = Tab::IdOf(contents);
  Space* space = Space::Holding(tab);
  std::vector<bc_id> panes = space ? space->Panes(tab) : std::vector<bc_id>{tab};
  for (bc_id pane : panes_) {
    if (!std::ranges::contains(panes, pane)) {
      Seen(Tab::FromId(pane));
    }
  }
  panes_ = std::move(panes);
  shown_ = contents->GetWeakPtr();
  page_frame_ = frame;
  // If the page it replaces had the keyboard, the window has it once that page leaves, and the
  // new page takes it.
  LayOutPage();
  FocusPageSoon();
}

void Window::LayOutPage() {
  std::vector<content::WebContents*> shown;
  for (bc_id pane : panes_) {
    if (content::WebContents* contents = Tab::FromId(pane)) {
      shown.push_back(contents);
    }
  }
  // A page full screen takes the window, as in Chrome, and the rest of its split view waits.
  if (shown_ && shown_->IsFullscreen()) {
    shown = {shown_.get()};
  }
  Space* space = panes_.empty() ? nullptr : Space::Holding(panes_.front());
  stacked_ = space && space->Stacked(panes_.front());
  shares_ = space ? space->Shares(panes_.front()) : std::vector<double>();
  std::vector<platform::Pane> panes;
  size_t focused = 0;
  const int count = static_cast<int>(shown.size());
  // Side by side or stacked, each page takes its share of the frame with the gap after it, else
  // is as wide or as tall as the others. A page full screen, alone, takes the whole frame.
  const int start = stacked_ ? page_frame_.y() : page_frame_.x();
  const int span = (stacked_ ? page_frame_.height() : page_frame_.width()) + kSplitGap;
  const bool even = shares_.size() != shown.size();
  auto edge = [&](int index) {
    return start + (even ? span * index / count
                         : base::ClampRound(span * std::accumulate(shares_.begin(),
                                                                   shares_.begin() + index, 0.0)));
  };
  for (int i = 0; i < count; ++i) {
    content::WebContents* contents = shown[i];
    if (contents == shown_.get()) {
      focused = i;
    }
    if (Tab* tab = Tab::FromWebContents(contents); tab && page_color_) {
      tab->SetPageColor(SkColorSetRGB(*page_color_ >> 16, *page_color_ >> 8 & 0xff,
                                      *page_color_ & 0xff));
    }
    const int from = edge(i);
    const int to = edge(i + 1) - kSplitGap;
    platform::Pane& pane = panes.emplace_back(
        contents->GetNativeView(),
        stacked_ ? gfx::Rect(page_frame_.x(), from, page_frame_.width(), to - from)
                 : gfx::Rect(from, page_frame_.y(), to - from, page_frame_.height()));
    DevToolsContentsResizingStrategy strategy;
    if (content::WebContents* devtools = DevToolsWindow::GetInTabWebContents(contents, &strategy)) {
      // As Chrome's window does: DevTools fill the frame, and the page goes where they leave it.
      platform::DockedDevTools docked = {devtools->GetNativeView(), gfx::Rect(),
                                         strategy.hide_inspected_contents()};
      gfx::Rect devtools_frame;
      ApplyDevToolsContentsResizingStrategy(strategy, gfx::Rect(pane.frame.size()),
                                            &devtools_frame, &docked.page_frame);
      pane.devtools = docked;
    }
  }
  platform::ShowPages(widget_.get(), panes, page_corner_radius_, focused, focus_color_);
  const gfx::Rect focused_frame = panes.empty() ? page_frame_ : panes[focused].frame;
  if (focused_frame != focused_frame_) {
    focused_frame_ = focused_frame;
    for (web_modal::ModalDialogHostObserver& observer : dialog_observers_) {
      observer.OnPositionRequiresUpdate();
    }
  }
}

void Window::ShowSpace(bc_id space) {
  if (!Space::FromId(space) || space == space_) {
    return;
  }
  const bc_id tab = TabToShowIn(space);
  if (space_) {
    selected_in_space_[space_] = selected_;
  }
  space_ = space;
  // Posted before the tab is, so the app knows the Space when it hears which tab shows.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](base::WeakPtr<Window> window, bc_id space) {
                       if (window && Host().window_space_shown) {
                         Host().window_space_shown(Host().context, window->id(), space);
                       }
                     },
                     weak_factory_.GetWeakPtr(), space));
  if (tab) {
    Select(tab);
  } else {
    ShowNothing();
  }
}

bc_id Window::TabToShowIn(bc_id space) const {
  Space* in = Space::FromId(space);
  if (!in || space == space_) {
    return in ? selected_ : 0;
  }
  auto last = selected_in_space_.find(space);
  if (last != selected_in_space_.end() &&
      (in->Has(last->second) || (Favorites::Has(last->second) &&
                                 Favorites::ProfileOf(last->second) == in->profile()))) {
    return last->second;
  }
  if (!in->today().empty()) {
    return in->today().front();
  }
  return in->pinned().empty() ? 0 : in->pinned().front();
}

bc_id Window::SelectedIn(bc_id space) const {
  if (space == space_) {
    return selected_;
  }
  auto it = selected_in_space_.find(space);
  return it == selected_in_space_.end() ? 0 : it->second;
}

void Window::SetSelectedIn(bc_id space, bc_id tab) {
  selected_in_space_[space] = tab;
}

void Window::SetSidebarWidth(double width) {
  sidebar_width_ = width;
  SessionChanged();
}

void Window::ShowNothing() {
  selected_ = 0;
  for (bc_id pane : std::exchange(panes_, {})) {
    Seen(Tab::FromId(pane));
  }
  shown_ = nullptr;
  LayOutPage();
  ReportSelected();
}

void Window::SetPageFrame(const gfx::Rect& frame, double corner_radius) {
  page_frame_ = frame;
  page_corner_radius_ = corner_radius;
  if (shown_) {
    Show(shown_.get(), frame);
  }
}

void Window::SetFocusColor(uint32_t color) {
  if (focus_color_ != color) {
    focus_color_ = color;
    LayOutPage();
  }
}

void Window::SetPageColor(uint32_t color) {
  if (page_color_ != color) {
    page_color_ = color;
    LayOutPage();
  }
}

void Window::Select(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!in || ForBrowser(in->GetBrowserWindowInterface()) != this) {
    return;
  }
  if (Space* space = Space::Holding(tab); space && space_ && space->id() != space_) {
    // As the tab its Space shows, so the one it showed before doesn't load on the way.
    SetSelectedIn(space->id(), tab);
    ShowSpace(space->id());
    return;
  }
  const Space* shown = Space::FromId(space_);
  if (shown && Favorites::Has(tab) && Favorites::ProfileOf(tab) != shown->profile()) {
    // A favorite shows in the Spaces of its profile: the first of them.
    for (Space* space : Space::All()) {
      if (space->profile() == Favorites::ProfileOf(tab)) {
        SetSelectedIn(space->id(), tab);
        ShowSpace(space->id());
        return;
      }
    }
    return;
  }
  selected_ = tab;
  Show(contents, page_frame_);
  ReportSelected();
}

size_t Window::Clear() {
  Space* space = Space::FromId(space_);
  if (!space) {
    return 0;
  }
  cleared_.clear();
  const std::vector<bc_id>& today = space->today();
  for (size_t index = 0; index < today.size(); ++index) {
    if (!std::ranges::contains(panes_, today[index])) {
      const bool split = index > 0 && std::ranges::contains(space->Panes(today[index]),
                                                            today[index - 1]);
      cleared_.push_back({today[index], index, split});
    }
  }
  // Bottom up, so ReopenClosedTab brings back the top one first, at its place.
  for (auto cleared = cleared_.rbegin(); cleared != cleared_.rend(); ++cleared) {
    CloseTab(cleared->tab);
  }
  return cleared_.size();
}

size_t Window::UndoClear() {
  size_t reopened = 0;
  bc_id above = 0;
  // Top down, each at its place before the Clear, which puts back the order with the tab kept.
  for (const Cleared& cleared : std::exchange(cleared_, {})) {
    // Not one whose page kept it open, or one already reopened.
    std::optional<ClosedTab> tab = TakeClosed(cleared.tab);
    const bc_id id = tab ? Reopen(tab->space, cleared.index, tab->saved) : 0;
    if (id) {
      ++reopened;
      // Back in its split, if the tab beside it came back too.
      if (Space* space = Space::Holding(id);
          space && cleared.split && above && space->Split(id, above, GURL()) && tab->stacked) {
        space->SetStacked(id, true);
      }
    }
    above = id;
  }
  return reopened;
}

bc_id Window::UndoDeleteSpace() {
  std::optional<DeletedSpace>& deleted = LastDeleted();
  Profile* profile = deleted ? ProfileFor(deleted->saved.profile) : nullptr;
  // Not while a page keeps one of its tabs open, which keeps the Space.
  if (!deleted || !profile || Space::FromId(deleted->id)) {
    return 0;
  }
  DeletedSpace space = *std::exchange(deleted, std::nullopt);
  // Its Today tabs leave the archive and the closed tabs as they come back, but for those that
  // came back some other way, such as ⇧⌘T, or left the archive.
  const int pinned = static_cast<int>(space.saved.pinned.size());
  const int shown = space.saved.shown;
  std::vector<SavedTab> today;
  bool above = false;
  for (size_t i = 0; i < space.today.size(); ++i) {
    const bc_id tab = space.today[i];
    const int index = pinned + static_cast<int>(i);
    if (!std::ranges::contains(Archived(), tab, &ArchivedTab::tab)) {
      if (index == shown) {
        space.saved.shown = -1;
      } else if (index < shown) {
        --space.saved.shown;
      }
      above = false;
      continue;
    }
    if (!TakeClosed(tab)) {
      Unarchive(tab);
    }
    SavedTab& saved = today.emplace_back(std::move(space.saved.today[i]));
    saved.split = saved.split && above;
    above = true;
  }
  space.saved.today = std::move(today);
  RestoreSpace(*this, profile, space.saved, space.id);
  if (Favorites::Has(space.shown)) {
    SetSelectedIn(space.id, space.shown);
  }
  ShowSpace(space.id);
  return space.id;
}

bc_id Window::ReopenClosedTab() {
  if (ClosedTabs().empty()) {
    return 0;
  }
  const ClosedTab closed = *TakeClosed(ClosedTabs().back().id);
  const bc_id tab = Reopen(closed.space, closed.index, closed.saved);
  if (!tab) {
    return 0;
  }
  // Tabs that closed with it in its split view, as its row's ✕ closes them, rejoin it as they come
  // back.
  for (ClosedTab& other : ClosedTabs()) {
    if (other.split_with == closed.id) {
      other.split_with = tab;
    }
  }
  // Back in its split view, at its place, if a tab of it is still open and the split has room.
  Space* space = Space::Holding(tab);
  const bool remade = space && space->Panes(closed.split_with).size() == 1;
  if (space && closed.split_with &&
      space->Split(tab, closed.split_with, Tab::PageUrl(Tab::FromId(tab)), /*left=*/false)) {
    space->MoveInSplit(tab, closed.split_index);
    // A split it remakes is as it was. One still open keeps how it's shown now.
    if (remade && closed.stacked) {
      space->SetStacked(tab, true);
    }
    PanesChanged();
  }
  Select(tab);
  return tab;
}

bool Window::RunExtension(const std::string& id, const gfx::Rect& anchor) {
  content::WebContents* contents = Tab::FromId(selected_);
  BrowserWindowInterface* browser =
      contents ? BrowserFor(Profile::FromBrowserContext(contents->GetBrowserContext())) : nullptr;
  if (!browser) {
    return false;
  }
  // Chrome's popup hangs from a view, so a view of no content stands where the app's anchor is.
  if (!popup_anchor_.view()) {
    popup_anchor_.SetView(
        widget_->GetContentsView()->AddChildView(std::make_unique<views::View>()));
  }
  popup_anchor_.view()->SetBoundsRect(anchor);
  return browser_core::RunExtension(id_, browser, contents, id, popup_anchor_.view());
}

bool Window::RunExtensionCommand(const std::string& id, const std::string& name) {
  content::WebContents* contents = Tab::FromId(selected_);
  return contents && browser_core::RunExtensionCommand(contents, id, name);
}

bc_id Window::RestoreArchived(bc_id id) {
  std::optional<ArchivedTab> archived = TakeArchived(id);
  if (!archived) {
    return 0;
  }
  // One closed since the browser started comes back with its history, else at its page.
  std::optional<ClosedTab> closed = TakeClosed(archived->tab);
  SavedTab saved;
  if (closed) {
    saved = std::move(closed->saved);
  } else {
    sessions::SerializedNavigationEntry& page = saved.history.emplace_back();
    page.set_virtual_url(archived->url);
    page.set_original_request_url(archived->url);
    page.set_title(base::UTF8ToUTF16(archived->title));
  }
  const bc_id tab = Reopen(space_, 0, saved);
  Select(tab);
  return tab;
}

bc_id Window::Duplicate(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  Space* space = Favorites::Has(tab) ? Space::FromId(space_) : Space::Holding(tab);
  if (!in || !space || ForBrowser(in->GetBrowserWindowInterface()) != this ||
      DirOf(contents) != space->profile()) {
    return 0;
  }
  TabStripModel* tabs = in->GetBrowserWindowInterface()->GetTabStripModel();
  std::unique_ptr<content::WebContents> copy = contents->Clone();
  content::WebContents* copied = copy.get();
  {
    base::AutoReset<bool> opening(&opening_tab_, true);
    tabs->InsertWebContentsAt(tabs->count(), std::move(copy), AddTabTypes::ADD_NONE);
  }
  const bc_id id = Tab::IdOf(copied);
  Tab::FromWebContents(copied)->set_name(Tab::FromWebContents(contents)->name());
  if (contents->IsAudioMuted()) {
    Tab::FromWebContents(copied)->SetMuted(true);
  }
  Tab::FromWebContents(copied)->Restored();
  auto at = std::ranges::find(space->today(), tab);
  space->AddAt(id, at == space->today().end() ? 0 : at - space->today().begin() + 1);
  Select(id);
  return id;
}

bc_id Window::Reopen(bc_id space_id, size_t index, const SavedTab& saved) {
  Space* space = Space::FromId(space_id);
  if (!space) {
    space = Space::FromId(space_);
    index = 0;
  }
  Profile* profile = space ? ProfileFor(space->profile()) : nullptr;
  content::WebContents* contents =
      space && profile ? RestoreTab(profile, saved.history, saved.current) : nullptr;
  if (!contents) {
    return 0;
  }
  const bc_id tab = Tab::IdOf(contents);
  Tab::FromWebContents(contents)->set_name(saved.name);
  if (saved.muted) {
    Tab::FromWebContents(contents)->SetMuted(true);
  }
  Tab::FromWebContents(contents)->Restored();
  space->AddAt(tab, index);
  return tab;
}

void Window::SelectAfterClosing() {
  const bc_id next = std::exchange(next_after_closing_, 0);
  if (next && Tab::FromId(next)) {
    Select(next);
  } else {
    ReportSelected();
  }
}

void Window::ReportSelected() {
  SessionChanged();
  // Posted, like the Space's changes, so the app hears of its own call once it has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](base::WeakPtr<Window> window, bc_id tab) {
                       if (window && Host().window_selected) {
                         Host().window_selected(Host().context, window->id(), tab);
                       }
                       // After the app has heard of the switch, which it times.
                       if (window) {
                         if (!window->actions_) {
                           window->actions_ = std::make_unique<WindowActions>(window->id());
                         }
                         window->actions_->Show(Tab::FromId(tab));
                       }
                     },
                     weak_factory_.GetWeakPtr(), selected_));
}

void Window::Close() {
  if (closing_) {
    return;
  }
  closing_ = true;
  if (browsers_.empty()) {
    Destroy();
    return;
  }
  std::vector<BrowserWindowInterface*> browsers;
  for (auto& [profile, held] : browsers_) {
    browsers.push_back(held.browser);
  }
  for (BrowserWindowInterface* browser : browsers) {
    browser->GetWindow()->Close();
  }
}

void Window::OnBrowserGone(BrowserWindowInterface* browser) {
  std::erase_if(browsers_, [browser](const auto& entry) { return entry.second.browser == browser; });
  if (closing_ && browsers_.empty()) {
    // Posted: the Browser is part way through its destructor.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::Destroy, weak_factory_.GetWeakPtr()));
  }
}

void Window::Destroy() {
  // Out of the list first, so the app can't reach the window from its callback.
  auto it = Windows().find(id_);
  std::unique_ptr<Window> self = std::move(it->second);
  Windows().erase(it);
  widget_->CloseNow();
  if (Host().window_closed) {
    Host().window_closed(Host().context, id_);
  }
}

gfx::NativeView Window::GetHostView() const {
  return widget_->GetNativeView();
}

gfx::Point Window::GetDialogPosition(const gfx::Size& size) {
  // As in Chrome's window: a tab's dialog, such as print preview, is centred over the page, at its
  // top. In a split view, over the focused page.
  return gfx::Point(focused_frame_.CenterPoint().x() - size.width() / 2, focused_frame_.y());
}

gfx::Size Window::GetMaximumDialogSize() {
  return focused_frame_.size();
}

void Window::AddObserver(web_modal::ModalDialogHostObserver* observer) {
  dialog_observers_.AddObserver(observer);
}

void Window::RemoveObserver(web_modal::ModalDialogHostObserver* observer) {
  dialog_observers_.RemoveObserver(observer);
}

void Window::OnTabStripModelChanged(TabStripModel* tab_strip_model,
                                    const TabStripModelChange& change,
                                    const TabStripSelectionChange& selection) {
  if (change.type() == TabStripModelChange::kRemoved) {
    // Tabs closing with the window, or as the browser quits, come back at the next launch instead.
    if (closing_ || browser_shutdown::IsTryingToQuit()) {
      return;
    }
    for (const TabStripModelChange::RemovedTab& removed : change.GetRemove()->contents) {
      if (removed.remove_reason == TabRemovedReason::kDeleted) {
        KeepClosed(removed.contents);
      }
    }
    return;
  }
  if (change.type() != TabStripModelChange::kInserted || opening_tab_) {
    return;
  }
  const bc_id split_with = std::exchange(split_with_, 0);
  for (const TabStripModelChange::ContentsWithIndex& inserted : change.GetInsert()->contents) {
    tabs::TabInterface* opener = tab_strip_model->GetOpenerOfTabAt(inserted.index);
    // Posted: the app shows the tab, and a tab strip can't change while it reports a change.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::ReportTabOpened, weak_factory_.GetWeakPtr(),
                                  Tab::IdOf(inserted.contents),
                                  opener ? Tab::IdOf(opener->GetContents()) : 0,
                                  tab_strip_model->active_index() == inserted.index, split_with));
  }
}

void Window::OnWidgetActivationChanged(views::Widget* widget, bool active) {
  // Chrome's focus manager gives the page the keyboard back as its window becomes key, but the
  // core shows pages outside views. Without this, a page that commits while the window isn't key
  // leaves the keyboard with nothing: Chromium gives none to its new view, and the old one is gone.
  if (active) {
    FocusPageSoon();
  }
}

void Window::FocusPageSoon() {
  // Posted: AppKit can be part way through moving the keyboard, as when a committing page hides the
  // view that had it and Chromium makes the window key. A page view focused inside that move
  // believes it has the keyboard after AppKit has given it elsewhere.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Window::FocusPage, weak_factory_.GetWeakPtr()));
}

void Window::FocusPage() {
  if (shown_ && widget_->IsActive() && platform::KeyboardForPage(widget_.get())) {
    shown_->Focus();
  }
}

void Window::OpenBehind(NavigateParams& params) {
  content::WebContents::CreateParams create(params.browser->GetProfile());
  create.initially_hidden = true;
  // As for a tab put back at launch: no renderer until it loads.
  create.desired_renderer_state = content::WebContents::CreateParams::kNoRendererProcess;
  std::unique_ptr<content::WebContents> made = content::WebContents::Create(create);
  content::WebContents* contents = made.get();
  {
    base::AutoReset<bool> opening(&opening_tab_, true);
    params.browser->GetTabStripModel()->AddWebContents(std::move(made), -1, params.transition,
                                                       AddTabTypes::ADD_NONE);
  }
  // The load Chrome would have started in it now.
  Tab::GetOrCreateForWebContents(contents)->LoadWhenShown(
      LoadURLParamsFromNavigateParams(contents, &params));
  // Posted, as for the tabs Chrome opens.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Window::ReportTabOpened, weak_factory_.GetWeakPtr(),
                                Tab::IdOf(contents), Tab::IdOf(params.source_contents),
                                /*foreground=*/false, /*split_with=*/0));
}

void Window::ReportTabOpened(bc_id tab, bc_id opener, bool foreground, bc_id split_with) {
  // A popup can close itself at once.
  if (!Tab::FromId(tab)) {
    return;
  }
  Space* space = Space::Holding(opener);
  if (!space) {
    space = Space::FromId(space_);
  }
  if (space) {
    space->Add(tab, opener, foreground);
    // Beside the page it came from, unless that page's split view is full.
    const bool split = split_with && space->Split(tab, split_with, Tab::PageUrl(Tab::FromId(tab)),
                                                  /*left=*/false);
    // A page in a Space the window doesn't show doesn't bring its tabs to the front.
    if (foreground && space->id() == space_) {
      Select(tab);
    }
    if (split) {
      PanesChanged();
    }
  }
  if (Host().tab_opened) {
    Host().tab_opened(Host().context, id_, tab, opener, foreground);
  }
}

}  // namespace browser_core
