#include "browser_core/autofill_menu.h"

#include <optional>

#include "base/memory/weak_ptr.h"
#include "browser_core/agent_autofill.h"
#include "browser_core/tab.h"
#include "components/autofill/content/browser/content_autofill_client.h"
#include "components/autofill/core/browser/foundations/autofill_driver.h"
#include "components/autofill/core/browser/foundations/autofill_manager.h"
#include "components/autofill/core/browser/foundations/scoped_autofill_managers_observation.h"
#include "components/autofill/core/common/aliases.h"
#include "components/autofill/core/common/unique_ids.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"
#include "content/public/browser/web_contents_user_data.h"

namespace browser_core {

namespace {

using autofill::AutofillManager;

// The form field focused in a tab's page, in whichever of its frames, and that frame's manager.
class FocusedField : public content::WebContentsUserData<FocusedField>,
                     public AutofillManager::Observer {
 public:
  ~FocusedField() override = default;

  bool Show(int kind) {
    if (!manager_ || !field_) {
      return false;
    }
    // Passwords: every login saved for the site, on any field Chrome can fill one in, as its
    // context menu's Passwords does. Chrome dropped its address and card fallbacks, so the others
    // open what Chrome has for the field, as a click in it does.
    manager_->driver().RendererShouldTriggerSuggestions(
        *field_, kind == BC_AUTOFILL_PASSWORDS
                     ? autofill::AutofillSuggestionTriggerSource::kManualFallbackPasswords
                     : autofill::AutofillSuggestionTriggerSource::kFormControlElementClicked);
    return true;
  }

 private:
  friend class content::WebContentsUserData<FocusedField>;

  explicit FocusedField(content::WebContents* contents)
      : content::WebContentsUserData<FocusedField>(*contents) {
    if (auto* client = autofill::ContentAutofillClient::FromWebContents(contents)) {
      observation_.Observe(
          client,
          autofill::ScopedAutofillManagersObservation::InitializationPolicy::
              kObservePreexistingManagers);
    }
  }

  // AutofillManager::Observer:
  void OnAfterFocusOnFormField(AutofillManager& manager,
                               autofill::FormGlobalId form,
                               autofill::FieldGlobalId field) override {
    manager_ = manager.GetWeakPtr();
    field_ = field;
  }

  void OnAfterFocusOnNonFormField(AutofillManager& manager) override {
    if (manager_.get() == &manager) {
      Forget();
    }
  }

  void OnAutofillManagerStateChanged(AutofillManager& manager,
                                     AutofillManager::LifecycleState previous,
                                     AutofillManager::LifecycleState new_state) override {
    if (manager_.get() == &manager &&
        new_state != AutofillManager::LifecycleState::kActive) {
      Forget();
    }
  }

  void Forget() {
    manager_ = nullptr;
    field_.reset();
  }

  base::WeakPtr<AutofillManager> manager_;
  std::optional<autofill::FieldGlobalId> field_;
  autofill::ScopedAutofillManagersObservation observation_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

WEB_CONTENTS_USER_DATA_KEY_IMPL(FocusedField);

}  // namespace

void WatchFocusedField(content::WebContents* contents) {
  FocusedField::CreateForWebContents(contents);
}

bool ShowAutofill(bc_id tab, int kind) {
  content::WebContents* contents = Tab::FromId(tab);
  // An agent's tab gets no saved data, whoever asks (agent_autofill.h).
  if (!contents || IsAgentTab(contents)) {
    return false;
  }
  // A field the page has focused while the keyboard is in the app's UI isn't the one in use.
  content::RenderWidgetHostView* view = contents->GetRenderWidgetHostView();
  FocusedField* focused = FocusedField::FromWebContents(contents);
  return view && view->HasFocus() && focused && focused->Show(kind);
}

}  // namespace browser_core
