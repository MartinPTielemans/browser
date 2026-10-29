#include "browser_core/autofill_offer.h"

#include <map>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "base/containers/span.h"
#include "base/functional/bind.h"
#include "base/memory/raw_ptr.h"
#include "base/no_destructor.h"
#include "base/strings/string_split.h"
#include "base/strings/string_util.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "chrome/browser/ui/autofill/autofill_bubble_base.h"
#include "chrome/browser/ui/autofill/payments/save_card_bubble_controller.h"
#include "chrome/browser/ui/autofill/save_address_bubble_controller.h"
#include "components/autofill/core/browser/data_model/payments/credit_card.h"
#include "components/autofill/core/browser/foundations/autofill_client.h"
#include "components/autofill/core/browser/ui/payments/payments_ui_closed_reasons.h"
#include "components/url_formatter/elide_url.h"
#include "content/public/browser/web_contents.h"
#include "url/origin.h"

namespace browser_core {

namespace {

using autofill::PaymentsUiClosedReason;

class Offer;

// The tabs' open offers, one a tab at most, as Chrome's BubbleManager shows one bubble at a time.
std::map<bc_id, std::unique_ptr<Offer>>& Offers() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Offer>>> offers;
  return *offers;
}

void Report(bc_id tab);

// Tells the app from a task of its own, as Chrome shows and hides bubbles inside the app's calls.
void Changed(bc_id tab) {
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE,
                                                              base::BindOnce(&Report, tab));
}

// The bubble the controller holds while the app shows its offer, in place of Chrome's view, and
// which answers and closes as that view does.
class Offer : public autofill::AutofillBubbleBase {
 public:
  Offer(bc_id tab, std::unique_ptr<autofill::SaveAddressBubbleController> address)
      : tab_(tab), address_(std::move(address)) {}
  Offer(bc_id tab, autofill::SaveCardBubbleController* card) : tab_(tab), card_(card) {}

  // The controller hides it: another tab shows, another bubble takes its place, or the tab goes.
  void Hide() override { Close(PaymentsUiClosedReason::kNotInteracted); }
  bool IsMouseHovered() const override { return false; }

  void Answer(int answer) {
    const bool accept = answer == BC_OFFER_ACCEPT;
    // As Chrome's view's buttons do, before its widget closes.
    if (address_) {
      if (accept) {
        address_->OnUserDecision(autofill::AutofillClient::AddressPromptUserDecision::kAccepted,
                                 address_->GetAutofillProfile());
      } else {
        address_->OnUserDecision(address_->GetCancelCallbackValue(), std::nullopt);
      }
    } else if (card_ && accept) {
      card_->OnSaveButton({});
    }
    Close(accept ? PaymentsUiClosedReason::kAccepted : PaymentsUiClosedReason::kCancelled);
  }

  // What the app shows. Null once closed.
  std::optional<bc_offer> ToC(content::WebContents* contents) {
    if (!address_ && !card_) {
      return std::nullopt;
    }
    std::vector<std::u16string> details;
    if (address_) {
      // Chrome's view shows the address as lines, then the email and the phone. The first line is
      // the name, or the street without one.
      std::vector<std::u16string> lines = base::SplitString(
          address_->GetAddressSummary(), u"\n", base::TRIM_WHITESPACE, base::SPLIT_WANT_NONEMPTY);
      lines.push_back(address_->GetProfileEmail());
      lines.push_back(address_->GetProfilePhone());
      std::erase(lines, std::u16string());
      if (!lines.empty()) {
        details.push_back(lines.front());
        details.push_back(base::JoinString(base::span(lines).subspan(1u), u", "));
      }
      title_ = base::UTF16ToUTF8(address_->GetWindowTitle());
      accept_ = base::UTF16ToUTF8(address_->GetOkButtonLabel());
      decline_ = base::UTF16ToUTF8(address_->GetNegativeButtonLabel());
    } else {
      // The card as Chrome's view shows it: its network and last four digits, then its name and
      // when it expires.
      const autofill::CreditCard& card = card_->GetCard();
      details.push_back(card.NetworkAndLastFourDigits());
      std::vector<std::u16string> rest = {card.GetRawInfo(autofill::CREDIT_CARD_NAME_FULL),
                                          card.AbbreviatedExpirationDateForDisplay(false)};
      std::erase(rest, std::u16string());
      details.push_back(base::JoinString(rest, u", "));
      title_ = base::UTF16ToUTF8(card_->GetWindowTitle());
      accept_ = base::UTF16ToUTF8(card_->GetAcceptButtonText());
      decline_ = base::UTF16ToUTF8(card_->GetDeclineButtonText());
    }
    std::erase(details, std::u16string());
    details_.clear();
    for (const std::u16string& detail : details) {
      details_.push_back(base::UTF16ToUTF8(detail));
    }
    detail_pointers_.clear();
    for (const std::string& detail : details_) {
      detail_pointers_.push_back(detail.c_str());
    }
    site_ = base::UTF16ToUTF8(url_formatter::FormatOriginForSecurityDisplay(
        url::Origin::Create(contents->GetLastCommittedURL()),
        url_formatter::SchemeDisplay::OMIT_HTTP_AND_HTTPS));
    return bc_offer{
        .kind = address_ ? BC_OFFER_KIND_ADDRESS : BC_OFFER_KIND_CARD,
        .title = title_.c_str(),
        .site = site_.c_str(),
        .username = "",
        .usernames = nullptr,
        .username_count = 0,
        .password_length = 0,
        .accept = accept_.c_str(),
        .decline = decline_.c_str(),
        .never = "",
        .details = detail_pointers_.data(),
        .detail_count = detail_pointers_.size(),
    };
  }

 private:
  // Closes the app's offer, and tells the controller it closed, as Chrome's view does as it hides,
  // which lets go of the controller.
  void Close(PaymentsUiClosedReason reason) {
    auto found = Offers().find(tab_);
    if (found == Offers().end() || found->second.get() != this) {
      return;
    }
    std::unique_ptr<Offer> self = std::move(found->second);
    Offers().erase(found);
    Changed(tab_);
    if (auto address = std::move(address_)) {
      address->OnBubbleClosed();
    } else if (autofill::SaveCardBubbleController* card = std::exchange(card_, nullptr)) {
      card->OnBubbleClosed(reason);
    }
    // The controller may still be in its call that hid this.
    base::SingleThreadTaskRunner::GetCurrentDefault()->DeleteSoon(FROM_HERE, std::move(self));
  }

  const bc_id tab_;
  std::unique_ptr<autofill::SaveAddressBubbleController> address_;
  raw_ptr<autofill::SaveCardBubbleController> card_ = nullptr;
  // What ToC points to.
  std::string title_, site_, accept_, decline_;
  std::vector<std::string> details_;
  std::vector<const char*> detail_pointers_;
};

void Report(bc_id tab) {
  if (!Host().tab_offer_changed) {
    return;
  }
  content::WebContents* contents = Tab::FromId(tab);
  auto found = Offers().find(tab);
  std::optional<bc_offer> offer =
      contents && found != Offers().end() ? found->second->ToC(contents) : std::nullopt;
  Host().tab_offer_changed(Host().context, tab, offer ? &*offer : nullptr);
}

autofill::AutofillBubbleBase* Open(bc_id tab, std::unique_ptr<Offer> offer) {
  // Chrome's BubbleManager hides one before it shows the next, but a tab has one at most anyway.
  if (auto found = Offers().find(tab); found != Offers().end()) {
    found->second->Hide();
  }
  Offer* shown = (Offers()[tab] = std::move(offer)).get();
  Changed(tab);
  return shown;
}

}  // namespace

// static
autofill::AutofillBubbleBase* AutofillOffer::SaveAddress(
    content::WebContents* contents,
    std::unique_ptr<autofill::SaveAddressBubbleController> controller) {
  const bc_id tab = Tab::IdOf(contents);
  return Open(tab, std::make_unique<Offer>(tab, std::move(controller)));
}

// static
autofill::AutofillBubbleBase* AutofillOffer::SaveCard(
    content::WebContents* contents,
    autofill::SaveCardBubbleController* controller) {
  const bc_id tab = Tab::IdOf(contents);
  return Open(tab, std::make_unique<Offer>(tab, controller));
}

// static
bool AutofillOffer::Has(bc_id tab) {
  return Offers().contains(tab);
}

// static
bool AutofillOffer::Answer(bc_id tab, int answer) {
  auto found = Offers().find(tab);
  if (found == Offers().end()) {
    return false;
  }
  found->second->Answer(answer);
  return true;
}

}  // namespace browser_core
