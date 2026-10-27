#include "browser_core/sign_in_dialog.h"

#include <utility>

#include "base/functional/bind.h"
#include "base/memory/ptr_util.h"
#include "base/strings/utf_string_conversions.h"
#include "browser_core/tab.h"
#include "components/strings/grit/components_strings.h"
#include "content/public/browser/web_contents.h"
#include "ui/base/l10n/l10n_util.h"
#include "ui/strings/grit/ui_strings.h"

namespace browser_core {

// static
std::unique_ptr<LoginHandler> SignInDialog::Create(
    const net::AuthChallengeInfo& auth_info,
    content::WebContents* web_contents,
    content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback) {
  // The constructor is LoginHandler's, which is protected.
  return base::WrapUnique(
      new SignInDialog(auth_info, web_contents, std::move(auth_required_callback)));
}

SignInDialog::SignInDialog(
    const net::AuthChallengeInfo& auth_info,
    content::WebContents* web_contents,
    content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback)
    : LoginHandler(auth_info, web_contents, std::move(auth_required_callback)) {}

SignInDialog::~SignInDialog() {
  PageDialog::Withdraw(id_);
}

bool SignInDialog::BuildViewImpl(const std::u16string& authority,
                                 const std::u16string& explanation,
                                 LoginModelData* login_model_data) {
  // As Chrome's, which needs a tab's dialog host: with none, LoginHandler cancels the sign-in.
  if (!web_contents() || !Tab::FromWebContents(web_contents())) {
    return false;
  }
  PageDialog::Words words;
  words.kind = BC_DIALOG_SIGN_IN;
  words.title = l10n_util::GetStringUTF8(IDS_LOGIN_DIALOG_TITLE);
  words.message = base::UTF16ToUTF8(authority);
  if (!explanation.empty()) {
    words.message += "\n" + base::UTF16ToUTF8(explanation);
  }
  words.accept = l10n_util::GetStringUTF8(IDS_LOGIN_DIALOG_OK_BUTTON_LABEL);
  words.cancel = l10n_util::GetStringUTF8(IDS_APP_CANCEL);
  words.username = l10n_util::GetStringUTF8(IDS_LOGIN_DIALOG_USERNAME_FIELD);
  words.password = l10n_util::GetStringUTF8(IDS_LOGIN_DIALOG_PASSWORD_FIELD);
  id_ = PageDialog::Ask(Tab::IdOf(web_contents()), std::move(words),
                        base::BindOnce(&SignInDialog::Answered, weak_factory_.GetWeakPtr()));
  return true;
}

void SignInDialog::CloseDialog() {
  PageDialog::Withdraw(id_);
  id_ = 0;
}

void SignInDialog::Answered(const PageDialog::Reply& reply) {
  // Answered, so there's nothing to take back.
  id_ = 0;
  // Either can delete this, as the tab's LoginTabHelper lets go of it once it's answered.
  if (reply.accept) {
    SetAuth(reply.text, reply.password);
  } else {
    CancelAuth(/*notify_others=*/true);
  }
}

}  // namespace browser_core
