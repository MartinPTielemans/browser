#include "browser_core/find.h"

#include <algorithm>

#include "base/functional/bind.h"
#include "base/strings/string_util.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "components/find_in_page/find_notification_details.h"
#include "components/find_in_page/find_types.h"
#include "content/public/browser/navigation_details.h"
#include "content/public/browser/navigation_handle.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"

namespace browser_core {

FindInPage::FindInPage(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<FindInPage>(*contents) {
  // Chrome's tab helpers make it as the tab is made.
  if (auto* helper = find_in_page::FindTabHelper::FromWebContents(contents)) {
    observation_.Observe(helper);
  }
}

FindInPage::~FindInPage() = default;

std::u16string FindInPage::Open() {
  find_in_page::FindTabHelper* helper = this->helper();
  if (!helper) {
    return std::u16string();
  }
  if (!helper->find_ui_active()) {
    helper->set_find_ui_active(true);
    text_ = helper->GetInitialSearchText();
    typed_ = false;
    Changed();
  }
  // As Chrome's find bar does: text selected in the page is what the user wants to find, unless
  // they've typed something since the bar opened.
  if (std::u16string selected = typed_ ? std::u16string() : SelectedText();
      !selected.empty() && selected != text_) {
    text_ = std::move(selected);
    Changed();
  }
  if (!text_.empty()) {
    // Counts and highlights the matches without going to one, which would scroll the page to text
    // the user may not be after.
    helper->StartFinding(text_, /*forward_direction=*/true, /*case_sensitive=*/false,
                         /*find_match=*/false);
  }
  return text_;
}

void FindInPage::Find(std::u16string text, bool forward) {
  find_in_page::FindTabHelper* helper = this->helper();
  if (!helper) {
    return;
  }
  if (!helper->find_ui_active()) {
    helper->set_find_ui_active(true);
    typed_ = false;
  }
  if (text != text_) {
    typed_ = !text.empty();
    text_ = std::move(text);
  }
  // The text the page already finds goes on to its next match.
  helper->StartFinding(text_, forward, /*case_sensitive=*/false, /*find_match=*/true);
  Changed();
}

void FindInPage::Close() {
  find_in_page::FindTabHelper* helper = this->helper();
  if (!helper || !helper->find_ui_active()) {
    return;
  }
  // As when the user closes Chrome's find bar: the selected match stays selected, to copy.
  helper->StopFinding(find_in_page::SelectionAction::kKeep);
  Changed();
}

find_in_page::FindTabHelper* FindInPage::helper() {
  return observation_.GetSource();
}

std::u16string FindInPage::SelectedText() {
  content::RenderWidgetHostView* view = web_contents()->GetRenderWidgetHostView();
  std::u16string selected = view ? view->GetSelectedText() : std::u16string();
  // As pasting it into the bar would.
  base::TrimWhitespace(selected, base::TRIM_ALL, &selected);
  // Chrome's limit: more is a passage, not a query.
  return selected.size() <= 250 ? selected : std::u16string();
}

void FindInPage::DidStartNavigation(content::NavigationHandle* navigation) {
  if (navigation->IsInPrimaryMainFrame()) {
    open_at_navigation_ = helper() && helper()->find_ui_active();
  }
}

void FindInPage::NavigationEntryCommitted(const content::LoadCommittedDetails& details) {
  find_in_page::FindTabHelper* helper = this->helper();
  if (!helper || !helper->find_ui_active() || !details.is_main_frame ||
      !details.is_navigation_to_different_page()) {
    return;
  }
  // As Chrome's find bar does (crbug.com/469819146): a new page closes it, unless the user opened
  // it while the page loaded, to search that. Then only what it found on the old page goes, and the
  // next search is the new page's.
  helper->StopFinding(open_at_navigation_.value_or(true) ? find_in_page::SelectionAction::kKeep
                                                         : find_in_page::SelectionAction::kClear);
  open_at_navigation_.reset();
  Changed();
}

void FindInPage::DidFinishNavigation(content::NavigationHandle* navigation) {
  if (navigation->IsInPrimaryMainFrame()) {
    open_at_navigation_.reset();
  }
}

void FindInPage::OnFindResultAvailable(content::WebContents* contents) {
  Changed();
}

void FindInPage::OnFindEmptyText(content::WebContents* contents) {
  Changed();
}

void FindInPage::OnFindTabHelperDestroyed(find_in_page::FindTabHelper* helper) {
  observation_.Reset();
}

void FindInPage::Changed() {
  if (report_posted_) {
    return;
  }
  report_posted_ = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&FindInPage::Report, weak_factory_.GetWeakPtr()));
}

void FindInPage::Report() {
  report_posted_ = false;
  find_in_page::FindTabHelper* helper = this->helper();
  if (!helper || !Host().tab_find_changed) {
    return;
  }
  const find_in_page::FindNotificationDetails& result = helper->find_result();
  int matches = result.number_of_matches();
  if (matches > -1) {
    // As Chrome's find bar does: a page starting a search guesses one match before it has counted
    // them, which would flash "1 of 1" over the count before as the user types.
    if (reported_matches_ > 0 && matches == 1 && !result.final_update() &&
        result.active_match_ordinal() == reported_selected_) {
      matches = reported_matches_;
    } else {
      reported_matches_ = matches;
      reported_selected_ = result.active_match_ordinal();
    }
  }
  const std::string text = base::UTF16ToUTF8(text_);
  const bc_find_state state = {
      .open = helper->find_ui_active(),
      .text = text.c_str(),
      .matches = matches,
      .selected = matches > 0 ? std::max(result.active_match_ordinal(), 0) : 0,
  };
  Host().tab_find_changed(Host().context, Tab::IdOf(web_contents()), &state);
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(FindInPage);

}  // namespace browser_core
