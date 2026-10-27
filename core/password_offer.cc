#include "browser_core/password_offer.h"

#include <algorithm>
#include <map>
#include <memory>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/strings/string_util.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "chrome/browser/ui/passwords/bubble_controllers/save_update_bubble_controller.h"
#include "chrome/browser/ui/passwords/passwords_model_delegate.h"
#include "chrome/grit/generated_resources.h"
#include "components/password_manager/core/browser/password_form.h"
#include "components/password_manager/core/common/password_manager_ui.h"
#include "components/url_formatter/elide_url.h"
#include "ui/base/l10n/l10n_util.h"

namespace browser_core {

namespace {

// The tabs' offers, one a tab at most, as Chrome's BubbleManager shows one bubble at a time.
std::map<bc_id, std::unique_ptr<SaveUpdateBubbleController>>& Offers() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<SaveUpdateBubbleController>>> offers;
  return *offers;
}

// The password's characters, counted by code point so that an emoji draws one dot.
size_t CodePoints(std::u16string_view password) {
  return std::ranges::count_if(password,
                               [](char16_t unit) { return unit < 0xDC00 || unit > 0xDFFF; });
}

bool IsOffer(password_manager::ui::State state) {
  return state == password_manager::ui::PENDING_PASSWORD_STATE ||
         state == password_manager::ui::PENDING_PASSWORD_UPDATE_STATE;
}

// The offer's login is still the one the password manager holds. It moves on without hiding the
// offer, such as when the page logs in again, and answering a login it no longer holds would save
// nothing or fail its checks.
bool Current(const SaveUpdateBubbleController& offer) {
  content::WebContents* contents = offer.GetWebContents();
  base::WeakPtr<PasswordsModelDelegate> delegate =
      contents ? PasswordsModelDelegateFromWebContents(contents) : nullptr;
  return delegate && delegate->GetState() == offer.state();
}

}  // namespace

void PasswordOffer::Prompt(content::WebContents* contents, bool show) {
  const bc_id tab = Tab::IdOf(contents);
  if (!show) {
    Drop(tab);
    return;
  }
  base::WeakPtr<PasswordsModelDelegate> delegate = PasswordsModelDelegateFromWebContents(contents);
  if (!delegate) {
    return;
  }
  if (!IsOffer(delegate->GetState())) {
    // Chrome's other password bubbles, such as the one confirming a save, have no prompt here yet.
    // Each closes as soon as it shows, as if the user had closed it, so the BubbleManager moves on.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<PasswordsModelDelegate> delegate) {
                         if (delegate) {
                           delegate->OnBubbleHidden();
                         }
                       },
                       delegate));
    return;
  }
  Drop(tab);
  // Made as Chrome's bubble makes it, which tells the password manager it's shown.
  Offers()[tab] = std::make_unique<SaveUpdateBubbleController>(
      delegate, PasswordBubbleControllerBase::DisplayReason::kAutomatic);
  Changed(tab);
}

void PasswordOffer::Answer(bc_id tab, int answer, const char* username) {
  auto found = Offers().find(tab);
  if (found == Offers().end()) {
    return;
  }
  SaveUpdateBubbleController& offer = *found->second;
  if (!Current(offer)) {
    Drop(tab);
    return;
  }
  const bool update = offer.state() == password_manager::ui::PENDING_PASSWORD_UPDATE_STATE;
  // As Chrome's bubble, which hands over the username field's text before each of its buttons,
  // with the password it was given.
  if (username) {
    std::u16string edited;
    base::TrimString(base::UTF8ToUTF16(username), u" ", &edited);
    offer.OnCredentialEdited(std::move(edited), offer.pending_password().password_value);
  }
  switch (answer) {
    case BC_OFFER_ACCEPT:
      offer.OnSaveClicked();
      break;
    case BC_OFFER_NEVER:
      // An update has no Never: the site's passwords are already saved.
      if (!update) {
        offer.OnNeverForThisSiteClicked();
      }
      break;
    case BC_OFFER_NOT_NOW:
      if (update) {
        offer.OnNoThanksClicked();
      } else {
        offer.OnNotNowClicked();
      }
      break;
  }
  // Unless answering it showed another.
  if (found = Offers().find(tab); found != Offers().end() && found->second.get() == &offer) {
    Drop(tab);
  }
}

void PasswordOffer::Drop(bc_id tab) {
  auto found = Offers().find(tab);
  if (found == Offers().end()) {
    return;
  }
  // Chrome's widget destroys the controller, which tells the password manager the bubble is gone,
  // from a task: here, inside the BubbleManager's own call to hide it, it would show the next.
  base::SingleThreadTaskRunner::GetCurrentDefault()->DeleteSoon(FROM_HERE,
                                                                std::move(found->second));
  Offers().erase(found);
  Changed(tab);
}

void PasswordOffer::Changed(bc_id tab) {
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&PasswordOffer::Report, tab));
}

void PasswordOffer::Report(bc_id tab) {
  if (!Host().tab_offer_changed) {
    return;
  }
  auto found = Offers().find(tab);
  if (found != Offers().end() && !Current(*found->second)) {
    Drop(tab);
    return;
  }
  if (found == Offers().end()) {
    Host().tab_offer_changed(Host().context, tab, nullptr);
    return;
  }
  const SaveUpdateBubbleController& offer = *found->second;
  const bool update = offer.state() == password_manager::ui::PENDING_PASSWORD_UPDATE_STATE;
  const password_manager::PasswordForm& form = offer.pending_password();
  // As Chrome's username field offers them: the one the page submitted, then the other fields'
  // values that might be the username.
  std::vector<std::string> usernames;
  auto add = [&usernames](const std::u16string& username) {
    std::string utf8 = base::UTF16ToUTF8(username);
    if (!utf8.empty() && !std::ranges::contains(usernames, utf8)) {
      usernames.push_back(std::move(utf8));
    }
  };
  add(form.username_value);
  for (const password_manager::AlternativeElement& alternative : form.all_alternative_usernames) {
    add(alternative.value);
  }
  std::vector<const char*> names;
  for (const std::string& username : usernames) {
    names.push_back(username.c_str());
  }
  const std::string title = base::UTF16ToUTF8(offer.GetTitle());
  const std::string username = base::UTF16ToUTF8(form.username_value);
  // As Chrome's bubble names the site when it isn't the page's.
  const std::string site = base::UTF16ToUTF8(url_formatter::FormatOriginForSecurityDisplay(
      PasswordsModelDelegateFromWebContents(offer.GetWebContents())->GetOrigin(),
      url_formatter::SchemeDisplay::OMIT_HTTP_AND_HTTPS));
  // Chrome's three-button save bubble, and its two-button update bubble.
  const std::string accept = l10n_util::GetStringUTF8(
      update ? IDS_PASSWORD_MANAGER_UPDATE_BUTTON : IDS_PASSWORD_MANAGER_SAVE_BUTTON);
  const std::string decline = l10n_util::GetStringUTF8(IDS_PASSWORD_MANAGER_CANCEL_BUTTON);
  const std::string never =
      update ? std::string()
             : l10n_util::GetStringUTF8(IDS_PASSWORD_MANAGER_BUBBLE_BLOCKLIST_BUTTON);
  const bc_offer reported = {
      .kind = update ? BC_OFFER_KIND_PASSWORD_UPDATE : BC_OFFER_KIND_PASSWORD,
      .title = title.c_str(),
      .site = site.c_str(),
      .username = username.c_str(),
      .usernames = names.data(),
      .username_count = names.size(),
      .password_length = CodePoints(form.password_value.secure_value()),
      .accept = accept.c_str(),
      .decline = decline.c_str(),
      .never = never.c_str(),
  };
  Host().tab_offer_changed(Host().context, tab, &reported);
}

}  // namespace browser_core
