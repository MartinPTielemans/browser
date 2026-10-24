#ifndef BROWSER_CORE_FIND_H_
#define BROWSER_CORE_FIND_H_

#include <optional>
#include <string>

#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "components/find_in_page/find_result_observer.h"
#include "components/find_in_page/find_tab_helper.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"

namespace browser_core {

// A tab's find in page, by the rules of Chrome's find bar, which the app draws. Chrome's
// FindTabHelper finds the text in the page and keeps the tab's last query.
class FindInPage : public content::WebContentsObserver,
                   public content::WebContentsUserData<FindInPage>,
                   public find_in_page::FindResultObserver {
 public:
  FindInPage(const FindInPage&) = delete;
  FindInPage& operator=(const FindInPage&) = delete;
  ~FindInPage() override;

  // Opens the find bar, if it isn't open, and returns its text. See bc_tab_find_open.
  std::u16string Open();
  // See bc_tab_find.
  void Find(std::u16string text, bool forward);
  void Close();

 private:
  friend class content::WebContentsUserData<FindInPage>;

  explicit FindInPage(content::WebContents* contents);

  // Null only while the tab closes.
  find_in_page::FindTabHelper* helper();
  // What's selected in the page, if it's short enough to be what the user wants to find.
  std::u16string SelectedText();

  // content::WebContentsObserver:
  void DidStartNavigation(content::NavigationHandle* navigation) override;
  void NavigationEntryCommitted(const content::LoadCommittedDetails& details) override;
  void DidFinishNavigation(content::NavigationHandle* navigation) override;

  // find_in_page::FindResultObserver:
  void OnFindResultAvailable(content::WebContents* contents) override;
  void OnFindEmptyText(content::WebContents* contents) override;
  void OnFindTabHelperDestroyed(find_in_page::FindTabHelper* helper) override;

  // Tells the app from a task of its own, so a burst of changes is one call, and none comes during
  // the app's call that made them.
  void Changed();
  void Report();

  // The find bar's text. A page committing under the open bar clears what it found, not this.
  std::u16string text_;
  // The user changed the text since the bar opened, so the page's selection doesn't replace it.
  bool typed_ = false;
  // Whether the bar was open when the page's navigation started, until it commits.
  std::optional<bool> open_at_navigation_;
  // The count the app was last told, from 1, which an early guess of the page's doesn't replace.
  int reported_matches_ = -1;
  int reported_selected_ = 0;
  bool report_posted_ = false;
  base::ScopedObservation<find_in_page::FindTabHelper, find_in_page::FindResultObserver>
      observation_{this};
  base::WeakPtrFactory<FindInPage> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_FIND_H_
