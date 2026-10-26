#ifndef BROWSER_CORE_INSTALL_QUESTION_H_
#define BROWSER_CORE_INSTALL_QUESTION_H_

#include <memory>

#include "browser_core/public/browser_core.h"
#include "content/public/browser/web_contents_observer.h"
#include "extensions/browser/extension_install_prompt_client.h"

class ExtensionInstallPromptShowParams;

namespace extensions {
class InstallPromptData;
}

namespace browser_core {

// An extension's install, re-enable or new permissions, which the app asks the user to confirm in
// place of Chrome's dialog (window_extension_question), in Chrome's words. A window asks one at a
// time, in the order Chrome asked. Chrome hears the question was dropped if the window closes, or
// the tab that asked does, before the user answers.
class InstallQuestion : public content::WebContentsObserver {
 public:
  using Result = extensions::ExtensionInstallPromptClient::Result;

  // g_browser_core_extension_install_dialog.
  static void Ask(std::unique_ptr<ExtensionInstallPromptShowParams> show_params,
                  extensions::ExtensionInstallPromptClient::DoneCallback done,
                  std::unique_ptr<extensions::InstallPromptData> prompt);
  // See bc_extension_answer.
  static void Answer(bc_id question, bool accept);
  // Drops |window|'s questions, as it closes.
  static void WindowClosing(bc_id window);

  InstallQuestion(const InstallQuestion&) = delete;
  InstallQuestion& operator=(const InstallQuestion&) = delete;
  ~InstallQuestion() override;

 private:
  InstallQuestion(bc_id window,
                  content::WebContents* tab,
                  extensions::ExtensionInstallPromptClient::DoneCallback done,
                  std::unique_ptr<extensions::InstallPromptData> prompt);

  // content::WebContentsObserver:
  void WebContentsDestroyed() override;

  // Forgets the question and gives Chrome |result|, if it's still asked.
  static void Finish(bc_id question, Result result);
  // Tells the app the window's first question, or that it has none, from a task of its own: Chrome
  // asks inside calls of the app's, such as the one that answers the question before.
  static void Changed(bc_id window);
  static void Report(bc_id window);

  const bc_id id_;
  const bc_id window_;
  extensions::ExtensionInstallPromptClient::DoneCallback done_;
  std::unique_ptr<extensions::InstallPromptData> prompt_;
};

}  // namespace browser_core

#endif  // BROWSER_CORE_INSTALL_QUESTION_H_
