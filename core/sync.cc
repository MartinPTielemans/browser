#include "browser_core/sync.h"

#include <memory>
#include <optional>
#include <string>
#include <utility>

#include "base/environment.h"
#include "base/functional/bind.h"
#include "base/memory/raw_ptr.h"
#include "base/observer_list.h"
#include "base/strings/cstring_view.h"
#include "base/task/sequenced_task_runner.h"
#include "base/time/time.h"
#include "base/uuid.h"
#include "browser_core/hooks.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/sync/sync_invalidations_service_factory.h"
#include "components/signin/public/identity_manager/access_token_info.h"
#include "components/signin/public/identity_manager/account_info.h"
#include "components/sync/invalidations/fcm_registration_token_observer.h"
#include "components/sync/invalidations/interested_data_types_handler.h"
#include "components/sync/invalidations/invalidations_listener.h"
#include "components/sync/invalidations/sync_invalidations_service.h"
#include "components/sync/service/sync_auth_manager.h"
#include "google_apis/gaia/core_account_id.h"
#include "google_apis/gaia/gaia_id.h"
#include "google_apis/gaia/google_service_auth_error.h"

namespace browser_core {

namespace {

std::string FromEnvironment(base::cstring_view name) {
  return base::Environment::Create()->GetVar(name).value_or(std::string());
}

// The account the browser's own account service signed in. Consented, so the profile's own data
// syncs, as Sync-the-feature did, rather than the separate account stores of a signed-in user.
syncer::SyncAccountInfo SyncAccount(const signin::IdentityManager* identity_manager) {
  const std::string id = FromEnvironment("BROWSER_SYNC_ACCOUNT");
  if (id.empty()) {
    return syncer::SyncAccountInfo();
  }
  CoreAccountInfo account;
  account.account_id = CoreAccountId::FromString(id);
  account.gaia = GaiaId(id);
  account.email = id + "@ilo.invalid";
  return {.account_info = account,
          .is_sync_consented = true,
          .managed_status = signin::AccountManagedStatusFinderOutcome::kConsumerNotWellKnown};
}

// The account's access token, answered after the call returns, as Google's token fetcher answers.
// The spike's token lasts the hour the server gives it; the product asks the app for a new one
// when this is called again after the server refused it or it expired.
void SyncToken(const signin::IdentityManager* identity_manager,
               base::OnceCallback<void(GoogleServiceAuthError, signin::AccessTokenInfo)> done) {
  const std::string token = FromEnvironment("BROWSER_SYNC_TOKEN");
  GoogleServiceAuthError error =
      token.empty() ? GoogleServiceAuthError::FromServiceUnavailable("no sync token")
                    : GoogleServiceAuthError::AuthErrorNone();
  base::SequencedTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE,
      base::BindOnce(std::move(done), std::move(error),
                     signin::AccessTokenInfo(token, base::Time::Now() + base::Hours(1),
                                             std::string())));
}

// Change notifications from the browser's relay. Its token names this device to the relay, and
// sync publishes it in the device's DeviceInfo, where the server reads where to send a change, as
// it reads FCM's token.
class RelayInvalidations : public syncer::SyncInvalidationsService {
 public:
  RelayInvalidations()
      : token_("ilo-relay:" + base::Uuid::GenerateRandomV4().AsLowercaseString()) {}

  void Deliver(const std::string& payload) {
    if (!listening_) {
      return;
    }
    for (syncer::InvalidationsListener& listener : listeners_) {
      listener.OnInvalidationReceived(payload);
    }
  }

  // syncer::SyncInvalidationsService:
  void StartListening() override {
    listening_ = true;
    NotifyToken();
  }
  void StopListening() override { listening_ = false; }
  void StopListeningPermanently() override {
    listening_ = false;
    NotifyToken();
  }
  void AddListener(syncer::InvalidationsListener* listener) override {
    if (!listeners_.HasObserver(listener)) {
      listeners_.AddObserver(listener);
    }
  }
  bool HasListener(syncer::InvalidationsListener* listener) override {
    return listeners_.HasObserver(listener);
  }
  void RemoveListener(syncer::InvalidationsListener* listener) override {
    listeners_.RemoveObserver(listener);
  }
  void AddTokenObserver(syncer::FCMRegistrationTokenObserver* observer) override {
    token_observers_.AddObserver(observer);
  }
  void RemoveTokenObserver(syncer::FCMRegistrationTokenObserver* observer) override {
    token_observers_.RemoveObserver(observer);
  }
  std::optional<std::string> GetFCMRegistrationToken() const override {
    return listening_ ? token_ : std::string();
  }
  void SetInterestedDataTypesHandler(syncer::InterestedDataTypesHandler* handler) override {
    interested_data_types_handler_ = handler;
  }
  std::optional<syncer::DataTypeSet> GetInterestedDataTypes() const override {
    return interested_data_types_;
  }
  void SetInterestedDataTypes(const syncer::DataTypeSet& data_types) override {
    interested_data_types_ = data_types;
    if (interested_data_types_handler_) {
      interested_data_types_handler_->OnInterestedDataTypesChanged();
    }
  }
  void SetCommittedAdditionalInterestedDataTypesCallback(
      InterestedDataTypesAppliedCallback callback) override {
    if (interested_data_types_handler_) {
      interested_data_types_handler_->SetCommittedAdditionalInterestedDataTypesCallback(
          std::move(callback));
    }
  }

 private:
  void NotifyToken() {
    for (syncer::FCMRegistrationTokenObserver& observer : token_observers_) {
      observer.OnFCMRegistrationTokenChanged();
    }
  }

  const std::string token_;
  bool listening_ = false;
  base::ObserverList<syncer::InvalidationsListener> listeners_;
  base::ObserverList<syncer::FCMRegistrationTokenObserver> token_observers_;
  raw_ptr<syncer::InterestedDataTypesHandler> interested_data_types_handler_ = nullptr;
  std::optional<syncer::DataTypeSet> interested_data_types_;
};

std::unique_ptr<syncer::SyncInvalidationsService> CreateRelayInvalidations(Profile* profile) {
  return std::make_unique<RelayInvalidations>();
}

}  // namespace

void InstallSyncHooks() {
  if (FromEnvironment("BROWSER_SYNC_ACCOUNT").empty()) {
    return;
  }
  g_browser_core_sync_account = &SyncAccount;
  g_browser_core_sync_token = &SyncToken;
  g_browser_core_sync_invalidations = &CreateRelayInvalidations;
}

void DeliverSyncInvalidation(Profile* profile, const std::string& payload) {
  if (!g_browser_core_sync_invalidations) {
    return;
  }
  static_cast<RelayInvalidations*>(SyncInvalidationsServiceFactory::GetForProfile(profile))
      ->Deliver(payload);
}

}  // namespace browser_core
