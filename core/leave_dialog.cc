#include "browser_core/leave_dialog.h"

#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "build/build_config.h"
#include "components/javascript_dialogs/app_modal_dialog_controller.h"
#include "components/javascript_dialogs/app_modal_dialog_manager.h"
#include "components/strings/grit/components_strings.h"
#include "content/public/browser/web_contents.h"
#include "ui/base/l10n/l10n_util.h"
#include "ui/strings/grit/ui_strings.h"

#if BUILDFLAG(IS_MAC)
#include "chrome/browser/chrome_browser_application_mac.h"
#endif

namespace browser_core {

namespace {

using Factory = javascript_dialogs::AppModalDialogManager::AppModalViewFactory;

// The view Chrome installed, for the dialogs of pages that aren't tabs'.
Factory& ChromeFactory() {
  static base::NoDestructor<Factory> factory;
  return *factory;
}

javascript_dialogs::AppModalDialogView* Create(
    std::unique_ptr<javascript_dialogs::AppModalDialogController> controller) {
  content::WebContents* contents = controller->web_contents();
  if (!contents || !Tab::FromWebContents(contents)) {
    return ChromeFactory().Run(std::move(controller));
  }
  return new LeaveDialog(std::move(controller));
}

}  // namespace

// static
void LeaveDialog::Install() {
  javascript_dialogs::AppModalDialogManager* manager =
      javascript_dialogs::AppModalDialogManager::GetInstance();
  ChromeFactory() = *manager->view_factory();
  manager->SetNativeDialogFactory(base::BindRepeating(&Create));
}

LeaveDialog::LeaveDialog(std::unique_ptr<javascript_dialogs::AppModalDialogController> controller)
    : controller_(std::move(controller)) {}

LeaveDialog::~LeaveDialog() {
  PageDialog::Withdraw(id_);
}

void LeaveDialog::ShowAppModalDialog() {
  showing_ = true;
  const bc_id tab = Tab::IdOf(controller_->web_contents());
  // As Chrome's activates the tab: one no window shows, such as one closed from its row, comes to
  // the front window to be answered, as every other app-wide dialog waits for it.
  if (!Window::Showing(tab)) {
    const std::vector<Window*> windows = Window::All();
    if (!windows.empty()) {
      windows.front()->Select(tab);
    }
  }
  PageDialog::Words words;
  words.title = base::UTF16ToUTF8(controller_->title());
  words.message = base::UTF16ToUTF8(controller_->message_text());
  words.accept = l10n_util::GetStringUTF8(IDS_APP_OK);
  if (controller_->is_before_unload_dialog()) {
    words.kind = BC_DIALOG_LEAVE;
    words.accept = l10n_util::GetStringUTF8(controller_->is_reload()
                                                ? IDS_BEFORERELOAD_MESSAGEBOX_OK_BUTTON_LABEL
                                                : IDS_BEFOREUNLOAD_MESSAGEBOX_OK_BUTTON_LABEL);
    // Chrome's says Cancel; Stay says what it does.
    words.cancel = "Stay";
  } else {
    switch (controller_->javascript_dialog_type()) {
      case content::JAVASCRIPT_DIALOG_TYPE_ALERT:
        words.kind = BC_DIALOG_ALERT;
        break;
      case content::JAVASCRIPT_DIALOG_TYPE_CONFIRM:
        words.kind = BC_DIALOG_CONFIRM;
        words.cancel = l10n_util::GetStringUTF8(IDS_APP_CANCEL);
        break;
      case content::JAVASCRIPT_DIALOG_TYPE_PROMPT:
        words.kind = BC_DIALOG_PROMPT;
        words.text = base::UTF16ToUTF8(controller_->default_prompt_text());
        words.cancel = l10n_util::GetStringUTF8(IDS_APP_CANCEL);
        break;
    }
  }
  // Chrome's spam guard: a page that asks again soon after is offered no more of its dialogs.
  if (controller_->display_suppress_checkbox()) {
    words.check = l10n_util::GetStringUTF8(IDS_JAVASCRIPT_MESSAGEBOX_SUPPRESS_OPTION);
  }
  id_ = PageDialog::Ask(tab, std::move(words),
                        base::BindOnce(&LeaveDialog::Answered, weak_factory_.GetWeakPtr()));
}

void LeaveDialog::ActivateAppModalDialog() {}

void LeaveDialog::CloseAppModalDialog() {
  Finish(Answer::kClose, std::u16string(), false);
}

void LeaveDialog::AcceptAppModalDialog() {
  Finish(Answer::kAccept, controller_->default_prompt_text(), false);
}

void LeaveDialog::CancelAppModalDialog() {
  Finish(Answer::kCancel, std::u16string(), false);
}

bool LeaveDialog::IsShowing() const {
  return showing_;
}

void LeaveDialog::Answered(const PageDialog::Reply& reply) {
  // Answered, so there's nothing to take back.
  id_ = 0;
  Finish(reply.accept ? Answer::kAccept : Answer::kCancel, reply.text, reply.checked);
}

void LeaveDialog::Finish(Answer answer, const std::u16string& text, bool suppress) {
  switch (answer) {
    case Answer::kAccept:
      controller_->OnAccept(text, suppress);
      break;
    case Answer::kCancel:
#if BUILDFLAG(IS_MAC)
      // Staying stops a quit that's under way, as Chrome's NSAlert does.
      if (controller_->is_before_unload_dialog()) {
        chrome_browser_application_mac::CancelTerminate();
      }
#endif
      controller_->OnCancel(suppress);
      break;
    case Answer::kClose:
      controller_->OnClose();
      break;
  }
  delete this;
}

}  // namespace browser_core
