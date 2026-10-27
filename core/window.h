#ifndef BROWSER_CORE_WINDOW_H_
#define BROWSER_CORE_WINDOW_H_

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "base/callback_list.h"
#include "base/containers/span.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/observer_list.h"
#include "base/scoped_observation.h"
#include "browser_core/public/browser_core.h"
#include "browser_core/session.h"
#include "chrome/browser/ui/tabs/tab_strip_model_observer.h"
#include "components/web_modal/web_contents_modal_dialog_host.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/views/view_tracker.h"
#include "ui/views/widget/widget_observer.h"

class BrowserWindowInterface;
class GURL;
class Profile;
struct NavigateParams;

namespace content {
class WebContents;
}

namespace sessions {
class SerializedNavigationEntry;
}

namespace ui {
class SimpleMenuModel;
}  // namespace ui

namespace views {
class Widget;
class WidgetDelegate;
}  // namespace views

namespace browser_core {

class SidePanel;
class Suggestions;
class WindowActions;

// One native window: the app's UI, the page it shows over that, and a Browser per profile holding
// the window's tabs (decision 3). Closing it closes its Browsers, and it goes once they have.
class Window : public web_modal::WebContentsModalDialogHost,
               public TabStripModelObserver,
               public views::WidgetObserver {
 public:
  // Returns null once Chromium is quitting.
  static Window* Open(const gfx::Rect& bounds, void* ui_view);
  // A private window for |profile|'s off-the-record profile, showing a Space of its own
  // (Space::CreateOffTheRecord). See bc_window_open_private. Null once Chromium is quitting.
  static Window* OpenPrivate(const gfx::Rect& bounds, void* ui_view, Profile* profile);
  static Window* FromId(bc_id id);
  // Every window, the most recently active first.
  static std::vector<Window*> All();
  // The window that holds |browser|'s tabs, if the core made |browser|.
  static Window* ForBrowser(const BrowserWindowInterface* browser);
  // Closes every window at once, at quit, once their Browsers are gone.
  static void CloseAll();
  // The window that shows |tab|'s page, focused or beside the tab it shows in a split view, else
  // null. A window whose page another window took doesn't show it.
  static Window* Showing(bc_id tab);
  // The window whose Browser holds |tab|'s page, else, for a tab without its page, the one it would
  // get it back in: the private window showing its Space, or the regular window active last. Null
  // if there's none.
  static Window* Holding(bc_id tab);
  // |tab|'s page, given back first, not shown, in the window Holding names if it has none, for a
  // call on its page that doesn't show it. Null if there's no window for it.
  static content::WebContents* PageOf(bc_id tab);
  // Has each window whose shown tab's split changed show it as it is now.
  static void PanesChanged();
  // From |tab|'s page, which the user clicked: a window showing it beside the tab it shows, in a
  // split view, shows it focused instead.
  static void OnPageClicked(bc_id tab);
  // Chrome's navigator calls this before it picks the Browser for a navigation.
  static void AdjustNavigate(NavigateParams& params);
  // From Chrome's DevTools: whether |browser|'s window docks them, as the core's windows do, and
  // that the docked DevTools of |inspected|, a tab of |browser|, opened, laid out again or closed.
  static bool CanDockDevTools(BrowserWindowInterface* browser);
  static void UpdateDevTools(BrowserWindowInterface* browser, content::WebContents* inspected);
  // From Chrome's page context menu, once filled in: takes out the items Arc's doesn't have and
  // those |browser|'s window can't do, if it's one of the core's.
  static void TrimPageMenu(BrowserWindowInterface* browser, ui::SimpleMenuModel& menu);
  // As |tab| closes: takes it out of its Space or the favorites, and has each window that showed it
  // show the tab its Space picks next, or for a favorite the Space the window shows.
  static void OnTabClosing(bc_id tab);
  // Unloads |tab|, a pinned tab or a favorite, which stays where it is: each window that showed it
  // shows the tab its Space picks next, or for a favorite the Space the window shows, else no page.
  // It leaves its split view, unless |split| unloads the rest of the split with it.
  static void Unload(bc_id tab, bool split);
  // Makes |tab| the favorite at |index|, out of its Space, or moves it there among the favorites.
  // See bc_tab_move_to_favorites.
  static void MoveToFavorites(bc_id tab, size_t index);
  // Takes the favorite |tab| to |index| in the pinned tabs or |folder| or the Today tabs of the
  // Space its window shows. See bc_tab_move.
  static void Unfavorite(bc_id tab, bool pinned, bc_id folder, size_t index);
  // Has each window that shows |space| show the Space before it, else the one after it, and closes
  // its tabs. The Space goes with the last of them. False, doing nothing, if it's the last Space
  // not going.
  static bool DeleteSpace(bc_id space);
  // Puts |space|, while it has no tabs, in the profile in the directory |profile|, which has
  // loaded. A window showing a favorite of the profile it was in there shows no page instead.
  static void SetSpaceProfile(bc_id space, const std::string& profile);
  // Every window stops showing, and stops remembering, a favorite its Space no longer shows, as
  // when the favorites' scope or a Space's profile changed. It shows that Space's own tab instead.
  static void HiddenFavoritesGone();
  // Closes |tab| as the user does: its page gets its unload handlers, which may keep it open.
  static void CloseTab(bc_id tab);
  // Closes the Today tabs below |tab| as the user does, bottom up, so ⇧⌘T reopens each at its
  // place. A window showing one shows |tab| first, so none shows or loads on the way out.
  static void CloseBelow(bc_id tab);
  // Moves |tab| to |space|: a pinned tab to the bottom of its pinned tabs, still pinned at its
  // pinned URL, a Today tab to the top of its Today tabs. A window showing the tab goes with it, as
  // Chrome's active tab does to another window, and its old Space shows next what it would once the
  // tab closed. Into another profile's Space it goes as a new tab, as bc_tab_move_to_space says.
  static void MoveToSpace(bc_id tab, bc_id space);

  Window(const Window&) = delete;
  Window& operator=(const Window&) = delete;
  ~Window() override;

  bc_id id() const { return id_; }
  views::Widget* widget() const { return widget_.get(); }
  // The Space the window shows, or 0.
  bc_id space() const { return space_; }
  // The tab the window shows, or 0. In a split view, the one focused.
  bc_id selected() const { return selected_; }
  // Whether it's a private window, which shows its own Space alone and is never saved.
  bool off_the_record() const { return off_the_record_; }
  // Whether its Browsers stay with no tabs left: a private window's does while the window is
  // open, as it holds the private session, which ends once the profile's last Browser has gone.
  bool KeepsBrowsers() const;

  // Shows |space| and the tab the window last showed there, else its top Today tab, else its top
  // pinned tab, else no page, passing over tabs another window shows. With |page| false, no page,
  // as a new window opens.
  void ShowSpace(bc_id space, bool page = true);
  // The tab the window shows in |space|, or showed there last. 0 if none.
  bc_id SelectedIn(bc_id space) const;
  // The tab ShowSpace shows for |space|, or the tab shown for the Space shown, which may be a
  // favorite. 0 for none.
  bc_id TabToShowIn(bc_id space) const;
  // Has ShowSpace show |tab| in |space|, as if the window showed it there last.
  void SetSelectedIn(bc_id space, bc_id tab);
  // The docked sidebar's width the UI last gave the window, kept with the Spaces, or 0 for none. A
  // window opens with the one the last launch saved.
  double sidebar_width() const { return sidebar_width_; }
  void SetSidebarWidth(double width);
  // Where the selected tab's page goes, in the window, and the radius of its corners. A split view
  // shares it out between its pages.
  void SetPageFrame(const gfx::Rect& frame, double corner_radius);
  // The ring the focused page of a split view has inside its edge, 0xRRGGBB.
  void SetFocusColor(uint32_t color);
  // What the pages it shows show until their first frame, 0xRRGGBB.
  void SetPageColor(uint32_t color);
  // Whether the app is moving the page frame on its own. The pages shown stretch the frame they
  // last drew to fill their views as the frame resizes them, until they draw at the new size.
  void SetPageMoving(bool moving);
  // Shows |tab| at the page frame, and its Space if the window shows another. In a split view, it
  // shows the split with |tab| focused. A page another window shows moves here (Adopt).
  void Select(bc_id tab);

  // Closes the Today tabs of the Space the window shows, all but those any window shows, or all of
  // them if it shows every one, for UndoClear to bring back. Returns how many.
  size_t Clear();
  // Brings back the tabs the last Clear closed, at their places in their Space's Today tabs and
  // unloaded, and shows the tab shown it closed. Once only, and not those ReopenClosedTab brought
  // back. Returns how many.
  size_t UndoClear();
  // Brings back the Space DeleteSpace deleted last, at its place and with its id, and shows it. See
  // bc_window_undo_delete_space. Returns its id, or 0.
  bc_id UndoDeleteSpace();
  // Reopens the tab closed last, in any window, with its history: at its place in its Space's Today
  // tabs, else at the top of the Space the window shows. Shows it, and its Space. Returns its id, or
  // 0 if there's none.
  bc_id ReopenClosedTab();
  // Reopens the archived tab |id| at the top of the Today tabs of the Space the window shows, and
  // shows it: with its history if it closed since the browser started, else at its page. It leaves
  // the archive. Returns its id, or 0.
  bc_id RestoreArchived(bc_id id);
  // Runs extension |id|'s action on the tab the window shows, its popup hanging from |anchor|. See
  // bc_window_run_extension.
  bool RunExtension(const std::string& id, const gfx::Rect& anchor);
  // Runs extension |id|'s keyboard command |name| on the tab the window shows. See
  // bc_window_run_extension_command.
  bool RunExtensionCommand(const std::string& id, const std::string& name);
  // The side panel of the Browser of the tab shown: where its page goes, closing it, whether
  // extension |id| has one for the tab shown, opening or closing its, and saving the width the user
  // gave it. See bc_window_set_side_panel_frame and those after it.
  void SetSidePanelFrame(const gfx::Rect& frame);
  void CloseSidePanel();
  bool HasSidePanel(const std::string& id);
  void ToggleSidePanel(const std::string& id);
  void SetSidePanelWidth(int width);
  // From |browser|'s side panel, which opened, closed or changed pages: shows it as it is now if
  // the window shows a tab of |browser|.
  void SidePanelChanged(BrowserWindowInterface* browser);
  // Tells the app soon of the side panel shown, if it differs from what it last told.
  void ReportSidePanelSoon();
  // Opens a copy of |tab|, one of this window's, with its history: under it in Today, or at the
  // top of Today for a pinned tab, where a pinned tab's links open, and in the Space the window
  // shows for a favorite. Shows it, and its page loads. Returns its id, or 0.
  bc_id Duplicate(bc_id tab);

  // Lists the command bar's suggestions for |text|, from |profile|, in |keyword|'s site search
  // unless it's empty. See bc_window_suggest.
  void Suggest(Profile* profile,
               const std::u16string& keyword,
               const std::u16string& text,
               bool complete);

  // Takes a suggestion out of the profile's history. See bc_window_remove_suggestion.
  void RemoveSuggestion(const GURL& url);

  // Opens a tab for |profile| loading |url|, not shown, or with |when_shown| an http(s) |url| only
  // once it's first shown. Null while the window is closing or Chromium is quitting.
  content::WebContents* OpenTab(Profile* profile, const GURL& url, bool when_shown);
  // Opens a tab for |profile| with |history|, at its |current| page, not shown. It loads once it's
  // shown. Null while the window is closing or Chromium is quitting.
  content::WebContents* RestoreTab(Profile* profile,
                                   base::span<const sessions::SerializedNavigationEntry> history,
                                   int current);

  // Shows |contents|, a tab in one of this window's Browsers, at |frame| in place of the page shown
  // before, taking the keyboard if that page had it. In a split view, it shows the split with
  // |contents| focused.
  void Show(content::WebContents* contents, const gfx::Rect& frame);

  // Hands the window's tabs to the regular window active before it, if there's one, and closes. A
  // private window, and the last regular one, ask each Browser to close instead, which runs their
  // pages' unload handlers; a page can keep it open.
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
  // A tab Clear closed, and its place in Today before.
  struct Cleared {
    bc_id tab;
    size_t index;
    // Whether it was in a split view with the tab above it.
    bool split;
    // Whether it was the tab the window showed.
    bool selected;
  };
  Window(bc_id id, const gfx::Rect& bounds, void* ui_view);

  // The Browser for |profile|'s tabs in this window, made when there's none or it's closing. Null
  // while the window is closing or Chromium is quitting.
  BrowserWindowInterface* BrowserFor(Profile* profile);

  // Opens |saved|, a closed tab, without its page, at |index| in |space|'s Today tabs, else at the
  // top of the Space the window shows. Returns its id, or 0.
  bc_id Reopen(bc_id space, size_t index, const SavedTab& saved);
  // A copy of |contents| in |browser|, one of the window's, not shown, with its tab's name, icon and
  // mute. Returns its id.
  bc_id CloneInto(BrowserWindowInterface& browser, content::WebContents& contents);
  // |split_with| is the tab whose page opened it to show beside it, or 0.
  void ReportTabOpened(bc_id tab, bc_id opener, bool foreground, bc_id split_with);
  // Opens the tab |params| asks for, behind, with nothing loaded until it's first shown.
  content::WebContents* OpenBehind(NavigateParams& params);
  // Moves the pages of |tab| and its split view into this window's Browsers from the windows
  // holding them, live, as Chrome's tab dragging does. A window showing one lets it go first.
  void Adopt(bc_id tab);
  // Shows no page, keeping the tab selected, as another window took its page, and says so
  // (window_page_elsewhere) unless |tell| is false.
  void LetGo(bool tell = true);
  // Sets let_go_, and tells the app when it changes.
  void SetLetGo(bool let_go);
  // Moves every tab of this window's Browsers into |to|'s, unshown, which empties them.
  void HandOver(Window& to);
  // Whether another window shows |tab|'s page, which an implicit pick passes over.
  bool ShownElsewhere(bc_id tab) const;
  // Shows |tab|, or if another window shows it the first tab of the Space shown no window shows,
  // else no page: for the window's own picks, which never take another window's page.
  void Pick(bc_id tab);
  // Shows next_after_closing_, as the tab shown has closed.
  void SelectAfterClosing();
  // Shows no page, as for a Space without tabs.
  void ShowNothing();
  // Puts the pages shown, and their docked DevTools if they have them, at the page frame, side by
  // side, and the side panel of their Browser at its frame.
  void LayOutPage();
  // The side panel of the Browser of the tab shown, or null.
  SidePanel* ShownSidePanel() const;
  void ReportSidePanel();
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
  // The page focused, and the tabs shown, in their order: its split's, else it alone. Whether
  // they're stacked, and their shares of the frame, as last laid out.
  base::WeakPtr<content::WebContents> shown_;
  std::vector<bc_id> panes_;
  bool stacked_ = false;
  std::vector<double> shares_;
  uint32_t focus_color_ = 0;
  std::optional<uint32_t> page_color_;
  bc_id space_ = 0;
  bc_id selected_ = 0;
  // Once the tab shown closes, the one to show next, as its Space picked it.
  bc_id next_after_closing_ = 0;
  // The tab each Space the window showed had shown when it left that Space.
  std::map<bc_id, bc_id> selected_in_space_;
  gfx::Rect page_frame_;
  // The focused page's part of it in a split view, else all of it: where the tab's dialogs go.
  gfx::Rect focused_frame_;
  double page_corner_radius_ = 0;
  // Whether the app is moving the page frame on its own (SetPageMoving).
  bool page_moving_ = false;
  double sidebar_width_;
  std::map<Profile*, Held> browsers_;
  // The tabs the last Clear closed, top to bottom.
  std::vector<Cleared> cleared_;
  // Made when the command bar first asks for suggestions.
  std::unique_ptr<Suggestions> suggestions_;
  bool closing_ = false;
  bool off_the_record_ = false;
  // Another window took the page shown. The tab stays selected, and the window shows no page.
  bool let_go_ = false;
  bool opening_tab_ = false;
  // The tab whose page the navigation being made opens a tab beside, until that tab is inserted.
  bc_id split_with_ = 0;
  base::ObserverList<web_modal::ModalDialogHostObserver> dialog_observers_;
  // Where the app last asked an extension's popup to hang from.
  views::ViewTracker popup_anchor_;
  // The extension actions it shows, made with its first tab.
  std::unique_ptr<WindowActions> actions_;
  // What the app last heard of the side panel shown.
  struct ToldSidePanel {
    std::string extension;
    std::string title;
    std::vector<uint8_t> icon;
    int width;
    friend bool operator==(const ToldSidePanel&, const ToldSidePanel&) = default;
  };
  gfx::Rect side_panel_frame_;
  // The side panel's page as last laid out, to tell the app when another shows.
  base::WeakPtr<content::WebContents> side_panel_laid_out_;
  std::optional<ToldSidePanel> side_panel_told_;
  bool side_panel_report_posted_ = false;
  base::WeakPtrFactory<Window> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_WINDOW_H_
