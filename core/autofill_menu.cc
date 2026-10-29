#include "browser_core/autofill_menu.h"

#include <optional>

#include "base/logging.h"
#include "base/memory/weak_ptr.h"
#include "browser_core/tab.h"
#include "components/autofill/content/browser/content_autofill_client.h"
#include "components/autofill/core/browser/foundations/autofill_driver.h"
#include "components/autofill/core/browser/foundations/autofill_manager.h"
#include "components/autofill/core/browser/foundations/scoped_autofill_managers_observation.h"
#include "components/autofill/core/common/aliases.h"
#include "components/autofill/core/common/unique_ids.h"
#include "content/public/browser/web_contents_user_data.h"

namespace browser_core {

namespace {

using autofill::AutofillManager;

// The form field of a page's frames that has the keyboard, as their autofill managers hear it.
class FocusedField : public content::WebContentsUserData<FocusedField>,
                     public AutofillManager::Observer {
 public:
  FocusedField(const FocusedField&) = delete;
  FocusedField& operator=(const FocusedField&) = delete;
  ~FocusedField() override = default;

  // Opens the dropdown |source| brings at the field. Returns false without one.
  bool Trigger(autofill::AutofillSuggestionTriggerSource source) {
    if (!manager_ || !field_) {
      return false;
    }
    manager_->driver().RendererShouldTriggerSuggestions(*field_, source);
    return true;
  }

 private:
  friend class content::WebContentsUserData<FocusedField>;

  FocusedField(content::WebContents* contents, autofill::ContentAutofillClient* client)
      : content::WebContentsUserData<FocusedField>(*contents) {
    observation_.Observe(client, autofill::ScopedAutofillManagersObservation::
                                     InitializationPolicy::kObservePreexistingManagers);
  }

  // AutofillManager::Observer:
  void OnAfterFocusOnFormField(AutofillManager& manager,
                               autofill::FormGlobalId form,
                               autofill::FieldGlobalId field) override {
    manager_ = manager.GetWeakPtr();
    field_ = field;
  }
  void OnAfterFocusOnNonFormField(AutofillManager& manager) override { Forget(manager); }
  // A frame that navigates or goes resets its manager, and the field is gone with its document.
  void OnAutofillManagerStateChanged(AutofillManager& manager,
                                     AutofillManager::LifecycleState previous,
                                     AutofillManager::LifecycleState state) override {
    if (state != AutofillManager::LifecycleState::kActive) {
      Forget(manager);
    }
  }

  void Forget(AutofillManager& manager) {
    if (manager_.get() == &manager) {
      manager_ = nullptr;
      field_.reset();
    }
  }

  autofill::ScopedAutofillManagersObservation observation_{this};
  base::WeakPtr<AutofillManager> manager_;
  std::optional<autofill::FieldGlobalId> field_;

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

WEB_CONTENTS_USER_DATA_KEY_IMPL(FocusedField);

}  // namespace

void AutofillMenu::Watch(content::WebContents* contents) {
  // Chrome's tab helpers make the client as the tab is made. A page without one has no autofill.
  if (auto* client = autofill::ContentAutofillClient::FromWebContents(contents)) {
    FocusedField::CreateForWebContents(contents, client);
  }
}

bool AutofillMenu::Open(bc_id tab, int kind) {
  content::WebContents* contents = Tab::FromId(tab);
  FocusedField* focused = contents ? FocusedField::FromWebContents(contents) : nullptr;
  // Passwords… lists every saved login, as Chrome's context menu's Passwords does, whatever the
  // field. Chrome 156 has no such menu for addresses and cards, so Contact… and Credit Card… ask
  // as a click in the field does, which lists what fits its kind.
  using Source = autofill::AutofillSuggestionTriggerSource;
  const bool opened =
      focused && focused->Trigger(kind == BC_AUTOFILL_PASSWORDS ? Source::kManualFallbackPasswords
                                                                : Source::kFormControlElementClicked);
  // check.sh waits for this line, an error since our builds print no warnings.
  LOG(ERROR) << "browser: autofill " << tab << " " << kind << (opened ? " opened" : " no field");
  return opened;
}

}  // namespace browser_core
