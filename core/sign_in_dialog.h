#ifndef BROWSER_CORE_SIGN_IN_DIALOG_H_
#define BROWSER_CORE_SIGN_IN_DIALOG_H_

#include <memory>
#include <string>

#include "base/memory/weak_ptr.h"
#include "browser_core/page_dialog.h"
#include "chrome/browser/ui/login/login_handler.h"

namespace browser_core {

// A site's HTTP sign-in, basic or digest, which the app asks with its dialog over the tab's card
// (PageDialog) in place of Chrome's views dialog (hooks.h). Chrome's LoginHandler still decides
// when it's asked, and closes it once another tab signs in to the same realm or the page goes.
class SignInDialog : public LoginHandler {
 public:
  // g_browser_core_login_handler.
  static std::unique_ptr<LoginHandler> Create(
      const net::AuthChallengeInfo& auth_info,
      content::WebContents* web_contents,
      content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback);

  SignInDialog(const SignInDialog&) = delete;
  SignInDialog& operator=(const SignInDialog&) = delete;
  ~SignInDialog() override;

 protected:
  // LoginHandler:
  bool BuildViewImpl(const std::u16string& authority,
                     const std::u16string& explanation,
                     LoginModelData* login_model_data) override;
  void CloseDialog() override;

 private:
  SignInDialog(const net::AuthChallengeInfo& auth_info,
               content::WebContents* web_contents,
               content::LoginDelegate::LoginAuthRequiredCallback auth_required_callback);

  void Answered(const PageDialog::Reply& reply);

  bc_id id_ = 0;
  base::WeakPtrFactory<SignInDialog> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SIGN_IN_DIALOG_H_
