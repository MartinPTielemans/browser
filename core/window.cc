#include "browser_core/window.h"

#include <algorithm>
#include <utility>
#include <vector>

#include "base/auto_reset.h"
#include "base/functional/bind.h"
#include "base/memory/ptr_util.h"
#include "base/no_destructor.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/browser_window_stand_in.h"
#include "browser_core/host.h"
#include "browser_core/platform/window_views.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/browser_window/public/create_browser_window.h"
#include "chrome/browser/ui/browser_window/public/desktop_browser_window_capabilities.h"
#include "chrome/browser/ui/interaction/browser_elements.h"
#include "chrome/browser/ui/navigator/browser_navigator.h"
#include "chrome/browser/ui/navigator/browser_navigator_params.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "chrome/browser/ui/views/interaction/browser_elements_views_impl.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/web_contents.h"
#include "ui/base/page_transition_types.h"
#include "ui/base/window_open_disposition.h"
#include "ui/views/widget/widget.h"
#include "ui/views/widget/widget_delegate.h"
#include "ui/views/window/client_view.h"

namespace browser_core {

namespace {

std::map<bc_id, std::unique_ptr<Window>>& Windows() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Window>>> windows;
  return *windows;
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
void Window::AdjustNavigate(NavigateParams& params) {
  // Popups become tabs (decision 3). So do new windows: the app opens its own.
  if ((params.disposition == WindowOpenDisposition::NEW_POPUP ||
       params.disposition == WindowOpenDisposition::NEW_WINDOW) &&
      params.browser && ForBrowser(params.browser)) {
    params.disposition = WindowOpenDisposition::NEW_FOREGROUND_TAB;
  }
}

// static
void Window::OnTabClosing(bc_id tab) {
  Space* space = Space::Holding(tab);
  if (!space) {
    return;
  }
  const bc_id next = space->NextAfterClosing(tab);
  space->Remove(tab);
  for (auto& [id, window] : Windows()) {
    if (window->selected_ == tab) {
      window->selected_ = 0;
      // Posted: Chrome is part way through closing the tab, and its tab strip can't change yet.
      base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
          FROM_HERE,
          base::BindOnce(&Window::SelectAfterClosing, window->weak_factory_.GetWeakPtr(), next));
    }
  }
}

Window::Window(bc_id id, const gfx::Rect& bounds, void* ui_view) : id_(id) {
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
  // If the page it replaces had the keyboard, the window has it once that page leaves, and the
  // new page takes it.
  platform::ShowPage(widget_.get(), contents->GetNativeView(), frame);
  shown_ = contents->GetWeakPtr();
  FocusPageSoon();
}

void Window::ShowSpace(bc_id space) {
  Space* to = Space::FromId(space);
  if (!to || space == space_) {
    return;
  }
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
  auto last = selected_in_space_.find(space);
  if (last != selected_in_space_.end() && std::ranges::contains(to->today(), last->second)) {
    Select(last->second);
  } else if (!to->today().empty()) {
    Select(to->today().front());
  } else {
    ShowNothing();
  }
}

void Window::ShowNothing() {
  selected_ = 0;
  shown_ = nullptr;
  platform::ShowPage(widget_.get(), gfx::NativeView(), page_frame_);
  ReportSelected();
}

void Window::SetPageFrame(const gfx::Rect& frame) {
  page_frame_ = frame;
  if (shown_) {
    Show(shown_.get(), frame);
  }
}

void Window::Select(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  tabs::TabInterface* in = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!in || ForBrowser(in->GetBrowserWindowInterface()) != this) {
    return;
  }
  selected_ = tab;
  Show(contents, page_frame_);
  ReportSelected();
}

void Window::SelectAfterClosing(bc_id next) {
  if (next && Tab::FromId(next)) {
    Select(next);
  } else {
    ReportSelected();
  }
}

void Window::ReportSelected() {
  // Posted, like the Space's changes, so the app hears of its own call once it has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](base::WeakPtr<Window> window, bc_id tab) {
                       if (window && Host().window_selected) {
                         Host().window_selected(Host().context, window->id(), tab);
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
  return gfx::Point((widget_->GetClientAreaBoundsInScreen().width() - size.width()) / 2, 0);
}

gfx::Size Window::GetMaximumDialogSize() {
  return widget_->GetClientAreaBoundsInScreen().size();
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
  if (change.type() != TabStripModelChange::kInserted || opening_tab_) {
    return;
  }
  for (const TabStripModelChange::ContentsWithIndex& inserted : change.GetInsert()->contents) {
    tabs::TabInterface* opener = tab_strip_model->GetOpenerOfTabAt(inserted.index);
    // Posted: the app shows the tab, and a tab strip can't change while it reports a change.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Window::ReportTabOpened, weak_factory_.GetWeakPtr(),
                                  Tab::IdOf(inserted.contents),
                                  opener ? Tab::IdOf(opener->GetContents()) : 0,
                                  tab_strip_model->active_index() == inserted.index));
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

void Window::ReportTabOpened(bc_id tab, bc_id opener, bool foreground) {
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
    // A page in a Space the window doesn't show doesn't bring its tabs to the front.
    if (foreground && space->id() == space_) {
      Select(tab);
    }
  }
  if (Host().tab_opened) {
    Host().tab_opened(Host().context, id_, tab, opener, foreground);
  }
}

}  // namespace browser_core
