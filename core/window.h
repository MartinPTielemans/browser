#ifndef BROWSER_CORE_WINDOW_H_
#define BROWSER_CORE_WINDOW_H_

#include <map>
#include <memory>

#include "base/callback_list.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/observer_list.h"
#include "base/scoped_observation.h"
#include "browser_core/public/browser_core.h"
#include "chrome/browser/ui/tabs/tab_strip_model_observer.h"
#include "components/web_modal/web_contents_modal_dialog_host.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/views/widget/widget_observer.h"

class BrowserWindowInterface;
class GURL;
class Profile;
struct NavigateParams;

namespace content {
class WebContents;
}

namespace views {
class Widget;
class WidgetDelegate;
}  // namespace views

namespace browser_core {

// One native window: the app's UI, the page it shows over that, and a Browser per profile holding
// the window's tabs (decision 3). Closing it closes its Browsers, and it goes once they have.
class Window : public web_modal::WebContentsModalDialogHost,
               public TabStripModelObserver,
               public views::WidgetObserver {
 public:
  // Returns null once Chromium is quitting.
  static Window* Open(const gfx::Rect& bounds, void* ui_view);
  static Window* FromId(bc_id id);
  // The window that holds |browser|'s tabs, if the core made |browser|.
  static Window* ForBrowser(const BrowserWindowInterface* browser);
  // Closes every window at once, at quit, once their Browsers are gone.
  static void CloseAll();
  // Chrome's navigator calls this before it picks the Browser for a navigation.
  static void AdjustNavigate(NavigateParams& params);
  // As |tab| closes: takes it out of its Space, and has each window that showed it show the tab its
  // Space picks next.
  static void OnTabClosing(bc_id tab);

  Window(const Window&) = delete;
  Window& operator=(const Window&) = delete;
  ~Window() override;

  bc_id id() const { return id_; }
  views::Widget* widget() const { return widget_.get(); }
  // The Space the window shows, or 0.
  bc_id space() const { return space_; }

  // Shows |space| and the tab the window last showed there, else its top tab, else no page.
  void ShowSpace(bc_id space);
  // Where the selected tab's page goes, in the window.
  void SetPageFrame(const gfx::Rect& frame);
  // Shows |tab|, if it's one of this window's, at the page frame.
  void Select(bc_id tab);

  // Opens a tab for |profile| loading |url|, not shown. Null while the window is closing or
  // Chromium is quitting.
  content::WebContents* OpenTab(Profile* profile, const GURL& url);

  // Shows |contents|, a tab in one of this window's Browsers, at |frame| in place of the page shown
  // before, taking the keyboard if that page had it.
  void Show(content::WebContents* contents, const gfx::Rect& frame);

  // Asks each Browser to close, which runs its pages' unload handlers; a page can keep it open.
  void Close();

  // From the stand-in, as |browser| goes.
  void OnBrowserGone(BrowserWindowInterface* browser);

  // web_modal::WebContentsModalDialogHost:
  gfx::NativeView GetHostView() const override;
  gfx::Point GetDialogPosition(const gfx::Size& size) override;
  gfx::Size GetMaximumDialogSize() override;
  void AddObserver(web_modal::ModalDialogHostObserver* observer) override;
  void RemoveObserver(web_modal::ModalDialogHostObserver* observer) override;

  // TabStripModelObserver:
  void OnTabStripModelChanged(TabStripModel* tab_strip_model,
                              const TabStripModelChange& change,
                              const TabStripSelectionChange& selection) override;

  // views::WidgetObserver:
  void OnWidgetActivationChanged(views::Widget* widget, bool active) override;

 private:
  class ClientView;
  struct Held {
    raw_ptr<BrowserWindowInterface> browser;
    base::CallbackListSubscription close_cancelled;
  };

  Window(bc_id id, const gfx::Rect& bounds, void* ui_view);

  // The Browser for |profile|'s tabs in this window, made when there's none or it's closing. Null
  // while the window is closing or Chromium is quitting.
  BrowserWindowInterface* BrowserFor(Profile* profile);

  void ReportTabOpened(bc_id tab, bc_id opener, bool foreground);
  void SelectAfterClosing(bc_id next);
  // Shows no page, as for a Space without tabs.
  void ShowNothing();
  void ReportSelected();

  // Gives the page shown the keyboard if the window is key and its keyboard is the page's.
  void FocusPageSoon();
  void FocusPage();

  // Closes the native window and reports it. Deletes this.
  void Destroy();

  const bc_id id_;
  std::unique_ptr<views::WidgetDelegate> delegate_;
  std::unique_ptr<views::Widget> widget_;
  base::ScopedObservation<views::Widget, views::WidgetObserver> widget_observation_{this};
  base::WeakPtr<content::WebContents> shown_;
  bc_id space_ = 0;
  bc_id selected_ = 0;
  // The tab each Space the window showed had shown when it left that Space.
  std::map<bc_id, bc_id> selected_in_space_;
  gfx::Rect page_frame_;
  std::map<Profile*, Held> browsers_;
  bool closing_ = false;
  bool opening_tab_ = false;
  base::ObserverList<web_modal::ModalDialogHostObserver> dialog_observers_;
  base::WeakPtrFactory<Window> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_WINDOW_H_
