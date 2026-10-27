#ifndef BROWSER_CORE_LEAVE_DIALOG_H_
#define BROWSER_CORE_LEAVE_DIALOG_H_

#include <memory>
#include <string>

#include "base/memory/weak_ptr.h"
#include "browser_core/page_dialog.h"
#include "components/javascript_dialogs/app_modal_dialog_view.h"

namespace javascript_dialogs {
class AppModalDialogController;
}

namespace browser_core {

// A tab's dialog that Chrome asks one at a time for the whole app, above all beforeunload's "Leave
// site?", which the app asks with its dialog over the tab's card (PageDialog) in place of Chrome's
// NSAlert. Chrome's AppModalDialogQueue still decides when it shows, and whether the page may ask.
// A dialog of a page that isn't a tab's, such as an extension's background page, keeps Chrome's.
class LeaveDialog : public javascript_dialogs::AppModalDialogView {
 public:
  // In place of the view Chrome installed, which it does before the core's ExtraParts start.
  static void Install();

  explicit LeaveDialog(std::unique_ptr<javascript_dialogs::AppModalDialogController> controller);
  LeaveDialog(const LeaveDialog&) = delete;
  LeaveDialog& operator=(const LeaveDialog&) = delete;
  ~LeaveDialog() override;

  // javascript_dialogs::AppModalDialogView:
  void ShowAppModalDialog() override;
  void ActivateAppModalDialog() override;
  void CloseAppModalDialog() override;
  void AcceptAppModalDialog() override;
  void CancelAppModalDialog() override;
  bool IsShowing() const override;

 private:
  enum class Answer { kAccept, kCancel, kClose };

  void Answered(const PageDialog::Reply& reply);
  // Tells Chrome, as its NSAlert does once it's gone, and deletes this.
  void Finish(Answer answer, const std::u16string& text, bool suppress);

  // Owned, as Chrome's views own theirs.
  std::unique_ptr<javascript_dialogs::AppModalDialogController> controller_;
  bc_id id_ = 0;
  bool showing_ = false;
  base::WeakPtrFactory<LeaveDialog> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_LEAVE_DIALOG_H_
