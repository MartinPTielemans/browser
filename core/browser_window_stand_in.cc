// Adapted from Ardia's ac_browser_window.mm (github.com/mantrakp04/ardia,
// engine/ardiacore/src/ardia/core, commit fce3a02ffb917d2902af4ce47b398ce8b03a440d).
// Copyright 2026 Mantra Patel. Licensed under the Apache License, Version 2.0; changed for this
// project: cut down to what a Browser needs, with the app's UI in place of Ardia's.

#include "browser_core/browser_window_stand_in.h"

#include <string>
#include <utility>

#include "base/functional/bind.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/themes/theme_service.h"
#include "chrome/browser/ui/autofill/autofill_bubble_base.h"
#include "chrome/browser/ui/autofill/save_address_bubble_controller.h"
#include "chrome/browser/ui/autofill/update_address_bubble_controller.h"
#include "chrome/browser/ui/browser_window/public/browser_window_features.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/exclusive_access/exclusive_access_manager.h"
#include "chrome/browser/ui/exclusive_access/fullscreen_controller.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "chrome/browser/ui/unload_controller.h"
#include "chrome/browser/ui/views/bubble_anchor_util_views.h"
#include "chrome/browser/ui/views/eye_dropper/eye_dropper.h"
#include "components/input/native_web_keyboard_event.h"
#include "content/public/browser/eye_dropper.h"
#include "content/public/browser/keyboard_event_processing_result.h"
#include "content/public/browser/web_contents.h"
#include "ui/base/mojom/window_show_state.mojom.h"
#include "ui/events/keycodes/dom/dom_key.h"
#include "ui/events/keycodes/dom/keycode_converter.h"
#include "ui/gfx/range/range.h"
#include "ui/native_theme/native_theme.h"
#include "ui/views/widget/widget.h"

namespace browser_core {

namespace {

// Asks the app, through |callback|, about a key going down in |browser|'s page, which is the one
// its window shows. Returns whether the app used it.
bool AskApp(bool (*callback)(void*, bc_id, const bc_key*),
            BrowserWindowInterface* browser,
            const input::NativeWebKeyboardEvent& event) {
  if (!callback || !browser || event.GetType() != blink::WebInputEvent::Type::kRawKeyDown ||
      event.skip_if_unhandled) {
    return false;
  }
  content::WebContents* contents = browser->GetTabStripModel()->GetActiveWebContents();
  if (!contents) {
    return false;
  }
  const std::string name = ui::KeycodeConverter::DomKeyToKeyString(ui::DomKey(event.dom_key));
  const int held = event.GetModifiers();
  uint32_t modifiers = 0;
  for (auto [web, flag] : {std::pair{blink::WebInputEvent::kShiftKey, BC_MODIFIER_SHIFT},
                           std::pair{blink::WebInputEvent::kControlKey, BC_MODIFIER_CONTROL},
                           std::pair{blink::WebInputEvent::kAltKey, BC_MODIFIER_ALT},
                           std::pair{blink::WebInputEvent::kMetaKey, BC_MODIFIER_META}}) {
    if (held & web) {
      modifiers |= flag;
    }
  }
  const bc_key key = {name.c_str(), modifiers};
  return callback(Host().context, Tab::IdOf(contents), &key);
}

}  // namespace

NoLocationBar::NoLocationBar() : LocationBar(nullptr) {}
NoLocationBar::~NoLocationBar() = default;

OmniboxView* NoLocationBar::GetOmniboxView() {
  return nullptr;
}
OmniboxPopupView* NoLocationBar::GetOmniboxPopupView() {
  return nullptr;
}
OmniboxController* NoLocationBar::GetOmniboxController() {
  return nullptr;
}
bool NoLocationBar::ShouldCloseOmniboxPopup(ui::MouseEvent* event) {
  return false;
}
content::WebContents* NoLocationBar::GetWebContents() {
  return browser_ ? browser_->GetTabStripModel()->GetActiveWebContents() : nullptr;
}
LocationBarModel* NoLocationBar::GetLocationBarModel() {
  return nullptr;
}
std::optional<bubble_anchor_util::AnchorConfiguration> NoLocationBar::GetChipAnchor() {
  return std::nullopt;
}
ChipController* NoLocationBar::GetChipController() {
  return nullptr;
}
ui::TrackedElement* NoLocationBar::GetAnchorOrNull() {
  return nullptr;
}
BrowserWindowInterface* NoLocationBar::GetBrowser() {
  return browser_;
}
Profile* NoLocationBar::GetProfile() {
  return browser_ ? browser_->GetProfile() : nullptr;
}
bool NoLocationBar::IsInitialized() const {
  return true;
}
bool NoLocationBar::IsVisible() const {
  return false;
}
bool NoLocationBar::IsDrawn() const {
  return false;
}
bool NoLocationBar::IsFullscreen() const {
  return false;
}
bool NoLocationBar::IsEditingOrEmpty() const {
  return false;
}
bool NoLocationBar::IsMouseHovered() const {
  return false;
}
bool NoLocationBar::IsFocusWithin() const {
  return false;
}
gfx::Rect NoLocationBar::Bounds() const {
  return gfx::Rect();
}
gfx::Rect NoLocationBar::BoundsInScreen() const {
  return gfx::Rect();
}
gfx::Size NoLocationBar::MinimumSize() const {
  return gfx::Size();
}
gfx::Size NoLocationBar::PreferredSize() const {
  return gfx::Size();
}
bool NoLocationBar::HasSecurityStateChanged() {
  return false;
}
LocationBarTesting* NoLocationBar::GetLocationBarForTesting() {
  return nullptr;
}

namespace {

class NoBubble : public autofill::AutofillBubbleBase {
 public:
  void Hide() override {}
  bool IsMouseHovered() const override { return false; }
};

autofill::AutofillBubbleBase* NoBubbleShown() {
  static NoBubble bubble;
  return &bubble;
}

}  // namespace

autofill::AutofillBubbleBase* NoAutofillBubbles::ShowSaveCreditCardBubble(
    content::WebContents*,
    autofill::SaveCardBubbleController*,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowIbanBubble(content::WebContents*,
                                                                autofill::IbanBubbleController*,
                                                                bool,
                                                                autofill::IbanBubbleType) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowOfferNotificationBubble(
    content::WebContents*,
    autofill::OfferNotificationBubbleController*,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowSaveAutofillAiDataBubble(
    content::WebContents*,
    autofill::AutofillAiImportDataController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowAutofillAiLocalSaveNotification(
    content::WebContents*,
    autofill::AutofillAiImportDataController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowSaveAddressProfileBubble(
    content::WebContents*,
    std::unique_ptr<autofill::SaveAddressBubbleController>,
    bool) {
  return NoBubbleShown();
}
#if BUILDFLAG(ENABLE_DICE_SUPPORT)
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowAddressSignInPromo(
    content::WebContents*,
    const autofill::AutofillProfile&) {
  return NoBubbleShown();
}
#endif
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowUpdateAddressProfileBubble(
    content::WebContents*,
    std::unique_ptr<autofill::UpdateAddressBubbleController>,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowFilledCardInformationBubble(
    content::WebContents*,
    autofill::FilledCardInformationBubbleController*,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowVirtualCardEnrollBubble(
    content::WebContents*,
    autofill::VirtualCardEnrollBubbleController*,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowVirtualCardEnrollConfirmationBubble(
    content::WebContents*,
    autofill::VirtualCardEnrollBubbleController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowMandatoryReauthBubble(
    content::WebContents*,
    autofill::MandatoryReauthBubbleController*,
    bool,
    autofill::MandatoryReauthBubbleType) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowSaveCardConfirmationBubble(
    content::WebContents*,
    autofill::SaveCardBubbleController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowSaveIbanConfirmationBubble(
    content::WebContents*,
    autofill::IbanBubbleController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowOmniboxAutofillBubble(
    content::WebContents*,
    autofill::OmniboxAutofillBubbleController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowPaymentsChurnedUsersBubble(
    content::WebContents*,
    autofill::PaymentsChurnedUsersBubbleController*,
    bool) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowPaymentsChurnedUsersConfirmationBubble(
    content::WebContents*,
    autofill::PaymentsChurnedUsersBubbleController*) {
  return NoBubbleShown();
}
autofill::AutofillBubbleBase* NoAutofillBubbles::ShowWalletReminderNoticeBubble(
    content::WebContents*,
    autofill::WalletReminderNoticeBubbleController*,
    bool) {
  return NoBubbleShown();
}

NoFindBar::NoFindBar() = default;
NoFindBar::~NoFindBar() = default;

FindBarController* NoFindBar::GetFindBarController() const {
  return controller_;
}
void NoFindBar::SetFindBarController(FindBarController* controller) {
  controller_ = controller;
}
void NoFindBar::Show(bool animate, bool focus) {
  visible_ = true;
}
void NoFindBar::Hide(bool animate) {
  visible_ = false;
}
void NoFindBar::SetFindTextAndSelectedRange(const std::u16string& find_text,
                                            const gfx::Range& selected_range) {
  find_text_ = find_text;
}
std::u16string_view NoFindBar::GetFindText() const {
  return find_text_;
}
gfx::Range NoFindBar::GetSelectedRange() const {
  return gfx::Range();
}
bool NoFindBar::IsFindBarVisible() const {
  return visible_;
}
bool NoFindBar::HasGlobalFindPasteboard() const {
  return BUILDFLAG(IS_MAC);
}
bool NoFindBar::CanPopulateFromSelectedText() {
  return true;
}
const FindBarTesting* NoFindBar::GetFindBarTesting() const {
  return nullptr;
}
bool NoFindBar::HasFocus() const {
  return false;
}
views::Widget* NoFindBar::GetHostWidget() {
  return nullptr;
}

BrowserWindowStandIn::BrowserWindowStandIn(base::WeakPtr<Window> window)
    : window_(std::move(window)), location_bar_(std::make_unique<NoLocationBar>()) {}

BrowserWindowStandIn::~BrowserWindowStandIn() {
  if (browser_) {
    // BrowserWidget does this for Chrome's windows: features may read the window as they go.
    browser_->GetFeatures().TearDownPreBrowserWindowDestruction();
  }
  if (window_) {
    window_->OnBrowserGone(browser_);
  }
}

void BrowserWindowStandIn::SetBrowser(BrowserWindowInterface* browser) {
  browser_ = browser;
  location_bar_->set_browser(browser);
  if (views::Widget* widget = this->widget()) {
    widget_observation_.Observe(widget);
  }
}

void BrowserWindowStandIn::DeleteBrowserWindow() {
  delete this;
}

views::Widget* BrowserWindowStandIn::widget() const {
  return window_ ? window_->widget() : nullptr;
}

bool BrowserWindowStandIn::IsActive() const {
  return widget() && widget()->IsActive();
}

bool BrowserWindowStandIn::IsMaximized() const {
  return widget() && widget()->IsMaximized();
}

bool BrowserWindowStandIn::IsMinimized() const {
  return widget() && widget()->IsMinimized();
}

bool BrowserWindowStandIn::IsFullscreen() const {
  return widget() && widget()->IsFullscreen();
}

gfx::NativeWindow BrowserWindowStandIn::GetNativeWindow() const {
  return widget() ? widget()->GetNativeWindow() : gfx::NativeWindow();
}

gfx::Rect BrowserWindowStandIn::GetRestoredBounds() const {
  return widget() ? widget()->GetRestoredBounds() : gfx::Rect();
}

ui::mojom::WindowShowState BrowserWindowStandIn::GetRestoredState() const {
  return ui::mojom::WindowShowState::kNormal;
}

gfx::Rect BrowserWindowStandIn::GetBounds() const {
  return widget() ? widget()->GetWindowBoundsInScreen() : gfx::Rect();
}

void BrowserWindowStandIn::Show() {
  if (widget()) {
    widget()->Show();
  }
}

bool BrowserWindowStandIn::IsVisible() const {
  return widget() && widget()->IsVisible();
}

void BrowserWindowStandIn::ShowInactive() {
  if (widget()) {
    widget()->ShowInactive();
  }
}

void BrowserWindowStandIn::Close() {
  // As BrowserView does when its widget is asked to close: the unload controller runs the pages'
  // beforeunload handlers, closes the tabs, and comes back here once they're gone to finish.
  if (browser_) {
    UnloadController::From(browser_)->OnWindowClosing();
  }
}

void BrowserWindowStandIn::Activate() {
  if (widget()) {
    widget()->Activate();
  }
}

void BrowserWindowStandIn::Maximize() {
  if (widget()) {
    widget()->Maximize();
  }
}

void BrowserWindowStandIn::Minimize() {
  if (widget()) {
    widget()->Minimize();
  }
}

void BrowserWindowStandIn::Restore() {
  if (widget()) {
    widget()->Restore();
  }
}

void BrowserWindowStandIn::SetBounds(const gfx::Rect& bounds) {
  if (widget()) {
    widget()->SetBounds(bounds);
  }
}

void BrowserWindowStandIn::FlashFrame(bool flash) {
  if (widget()) {
    widget()->FlashFrame(flash);
  }
}

ui::ZOrderLevel BrowserWindowStandIn::GetZOrderLevel() const {
  return widget() ? widget()->GetZOrderLevel() : ui::ZOrderLevel::kNormal;
}

void BrowserWindowStandIn::SetZOrderLevel(ui::ZOrderLevel order) {
  if (widget()) {
    widget()->SetZOrderLevel(order);
  }
}

bool BrowserWindowStandIn::IsOnCurrentWorkspace() const {
  return true;
}

bool BrowserWindowStandIn::IsVisibleOnScreen() const {
  return IsVisible();
}

bool BrowserWindowStandIn::DoBrowserControlsShrinkRendererSize(
    const content::WebContents* contents) const {
  return false;
}

ui::NativeTheme* BrowserWindowStandIn::GetNativeTheme() {
  return ui::NativeTheme::GetInstanceForNativeUi();
}

const ui::ThemeProvider* BrowserWindowStandIn::GetThemeProvider() const {
  return browser_ ? &ThemeService::GetThemeProviderForProfile(browser_->GetProfile()) : nullptr;
}

const ui::ColorProvider* BrowserWindowStandIn::GetColorProvider() const {
  return widget() ? widget()->GetColorProvider() : nullptr;
}

int BrowserWindowStandIn::GetTopControlsHeight() const {
  return 0;
}

std::vector<StatusBubble*> BrowserWindowStandIn::GetStatusBubbles() {
  return {};
}

gfx::Size BrowserWindowStandIn::GetContentsSize() const {
  return widget() ? widget()->GetClientAreaBoundsInScreen().size() : gfx::Size();
}

autofill::AutofillBubbleHandler* BrowserWindowStandIn::GetAutofillBubbleHandler() {
  return &autofill_bubbles_;
}

LocationBar* BrowserWindowStandIn::GetLocationBar() const {
  return location_bar_.get();
}

ui::AcceleratorProvider* BrowserWindowStandIn::GetAcceleratorProvider() {
  return this;
}

bool BrowserWindowStandIn::UpdateToolbarSecurityState() {
  return false;
}

bool BrowserWindowStandIn::IsTabStripEditable() const {
  return true;
}

bool BrowserWindowStandIn::IsToolbarVisible() const {
  return false;
}

bool BrowserWindowStandIn::IsToolbarShowing() const {
  return false;
}

bool BrowserWindowStandIn::IsLocationBarVisible() const {
  return false;
}

ShowTranslateBubbleResult BrowserWindowStandIn::ShowTranslateBubble(
    content::WebContents* contents,
    translate::TranslateStep step,
    const std::string& source_language,
    const std::string& target_language,
    translate::TranslateErrors error_type,
    bool is_user_gesture) {
  return ShowTranslateBubbleResult::kBrowserWindowNotValid;
}

DownloadBubbleUIController* BrowserWindowStandIn::GetDownloadBubbleUIController() {
  return nullptr;
}

void BrowserWindowStandIn::ConfirmBrowserCloseWithPendingDownloads(
    int download_count,
    DownloadCloseType dialog_type,
    base::OnceCallback<void(bool)> callback) {
  // Closes, cancelling the downloads, until the app can ask the user.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(std::move(callback), true));
}

content::KeyboardEventProcessingResult BrowserWindowStandIn::PreHandleKeyboardEvent(
    const input::NativeWebKeyboardEvent& event) {
  return AskApp(Host().tab_key_before_page, browser_, event)
             ? content::KeyboardEventProcessingResult::HANDLED
             : content::KeyboardEventProcessingResult::NOT_HANDLED;
}

bool BrowserWindowStandIn::HandleKeyboardEvent(const input::NativeWebKeyboardEvent& event) {
  if (AskApp(Host().tab_key_unhandled, browser_, event)) {
    return true;
  }
  // As BrowserView does: what neither the page nor the app used goes to the window's menus.
  views::Widget* widget = this->widget();
  return widget && widget->GetFocusManager() &&
         unhandled_keyboard_event_handler_.HandleKeyboardEvent(event, widget->GetFocusManager());
}

std::unique_ptr<FindBar> BrowserWindowStandIn::CreateFindBar() {
  return std::make_unique<NoFindBar>();
}

web_modal::WebContentsModalDialogHost* BrowserWindowStandIn::GetWebContentsModalDialogHost() {
  return window_.get();
}

web_modal::WebContentsModalDialogHost* BrowserWindowStandIn::GetWebContentsModalDialogHostFor(
    content::WebContents* web_contents) {
  return window_.get();
}

void BrowserWindowStandIn::ShowHatsDialog(
    const std::string& site_id,
    const std::optional<std::string>& hats_histogram_name,
    const std::optional<uint64_t> hats_survey_ukm_id,
    base::OnceClosure success_callback,
    base::OnceClosure failure_callback,
    const SurveyBitsData& product_specific_bits_data,
    const SurveyStringData& product_specific_string_data) {
  std::move(failure_callback).Run();
}

ExclusiveAccessContext* BrowserWindowStandIn::GetExclusiveAccessContext() {
  return this;
}

std::string BrowserWindowStandIn::GetWorkspace() const {
  return std::string();
}

bool BrowserWindowStandIn::IsVisibleOnAllWorkspaces() const {
  return false;
}

void BrowserWindowStandIn::ShowEmojiPanel() {
  if (widget()) {
    widget()->ShowEmojiPanel();
  }
}

std::unique_ptr<content::EyeDropper> BrowserWindowStandIn::OpenEyeDropper(
    content::RenderFrameHost* frame,
    content::EyeDropperListener* listener) {
  return ShowEyeDropper(frame, listener);
}

bool BrowserWindowStandIn::IsUnframedModeEnabled() const {
  return false;
}

bool BrowserWindowStandIn::GetCanResize() {
  return true;
}

ui::mojom::WindowShowState BrowserWindowStandIn::GetWindowShowState() const {
  if (IsMinimized()) {
    return ui::mojom::WindowShowState::kMinimized;
  }
  if (IsFullscreen()) {
    return ui::mojom::WindowShowState::kFullscreen;
  }
  if (IsMaximized()) {
    return ui::mojom::WindowShowState::kMaximized;
  }
  return ui::mojom::WindowShowState::kNormal;
}

BrowserView* BrowserWindowStandIn::AsBrowserView() {
  return nullptr;
}

Profile* BrowserWindowStandIn::GetProfile() {
  return browser_ ? browser_->GetProfile() : nullptr;
}

void BrowserWindowStandIn::EnterFullscreen(const url::Origin& origin,
                                           ExclusiveAccessBubbleType bubble_type,
                                           FullscreenTabParams fullscreen_tab_params) {
  // Chrome's fullscreen controller asks when a page goes full screen in a window that isn't, and
  // back only if the window went for the page. The app hears of the page from its tab.
  if (widget()) {
    widget()->SetFullscreen(true, fullscreen_tab_params.display_id);
  }
}

void BrowserWindowStandIn::ExitFullscreen() {
  if (widget()) {
    widget()->SetFullscreen(false);
  }
}

void BrowserWindowStandIn::UpdateExclusiveAccessBubble(
    const ExclusiveAccessBubbleParams& params,
    ExclusiveAccessBubbleHideCallback first_hide_callback) {
  // No bubble yet: the app will say how to leave fullscreen.
  if (first_hide_callback) {
    std::move(first_hide_callback).Run(ExclusiveAccessBubbleHideReason::kNotShown);
  }
}

bool BrowserWindowStandIn::IsExclusiveAccessBubbleDisplayed() const {
  return false;
}

content::WebContents* BrowserWindowStandIn::GetWebContentsForExclusiveAccess() {
  return browser_ ? browser_->GetTabStripModel()->GetActiveWebContents() : nullptr;
}

bool BrowserWindowStandIn::CanUserEnterFullscreen() const {
  return true;
}

bool BrowserWindowStandIn::CanUserExitFullscreen() const {
  return true;
}

bool BrowserWindowStandIn::GetAcceleratorForCommandId(int command_id,
                                                      ui::Accelerator* accelerator) const {
  return false;
}

void BrowserWindowStandIn::OnWidgetShowStateChanged(views::Widget* widget) {
  // As Chrome's windows do once a fullscreen change is done, or has failed. The controller takes a
  // full-screen page back when the window has left full screen, as by its green button.
  if (ExclusiveAccessManager* manager = ExclusiveAccessManager::From(browser_)) {
    manager->fullscreen_controller()->WindowFullscreenStateChanged();
  }
}

void BrowserWindowStandIn::OnWidgetDestroying(views::Widget* widget) {
  widget_observation_.Reset();
}

}  // namespace browser_core
