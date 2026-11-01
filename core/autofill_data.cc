#include "browser_core/autofill_data.h"

#include <map>
#include <memory>
#include <string>
#include <utility>
#include <vector>

#include "base/functional/bind.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/profiles.h"
#include "chrome/browser/autofill/personal_data_manager_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/profiles/profile_observer.h"
#include "components/autofill/core/browser/data_manager/addresses/address_data_manager.h"
#include "components/autofill/core/browser/data_manager/payments/payments_data_manager.h"
#include "components/autofill/core/browser/data_manager/personal_data_manager.h"
#include "components/autofill/core/browser/data_manager/personal_data_manager_observer.h"
#include "components/autofill/core/browser/data_model/addresses/autofill_profile.h"
#include "components/autofill/core/browser/data_model/payments/credit_card.h"
#include "components/autofill/core/browser/field_types.h"

namespace browser_core::autofill_data {

namespace {

using autofill::PersonalDataManager;

// An entry as bc_autofill_entry passes it, with the strings it points to.
struct Entry {
  std::string key;
  std::string title;
  std::string detail;
  bool card = false;
};

// |a| and |b| with a comma between, or whichever isn't empty.
std::u16string Joined(const std::u16string& a, const std::u16string& b) {
  return a.empty() || b.empty() ? a + b : a + u", " + b;
}

// Tells the app as a profile's addresses and cards change, once it has asked for them, until the
// profile goes.
class Watcher : public autofill::PersonalDataManagerObserver, public ProfileObserver {
 public:
  Watcher(Profile* profile, PersonalDataManager* data) : profile_(profile), data_(data) {
    data_observation_.Observe(data);
    profile_observation_.Observe(profile);
  }

  // Tells the app, from a task of its own, as the logins do.
  void Changed() {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Watcher::Report, weak_factory_.GetWeakPtr()));
  }

  PersonalDataManager* data() const { return data_; }

 private:
  // The saved addresses, then the cards on this device, each as Settings lists it.
  std::vector<Entry> All() const {
    std::vector<Entry> entries;
    for (const autofill::AutofillProfile* address :
         data_->address_data_manager().GetProfiles(
             autofill::AddressDataManager::ProfileOrder::kMostRecentlyModifiedDesc)) {
      std::u16string title = address->GetRawInfo(autofill::NAME_FULL);
      std::u16string detail = Joined(address->GetRawInfo(autofill::ADDRESS_HOME_LINE1),
                                     address->GetRawInfo(autofill::ADDRESS_HOME_CITY));
      if (title.empty()) {
        title = std::exchange(detail, address->GetRawInfo(autofill::EMAIL_ADDRESS));
      }
      entries.push_back({.key = address->guid(),
                         .title = base::UTF16ToUTF8(title),
                         .detail = base::UTF16ToUTF8(detail)});
    }
    for (const autofill::CreditCard* card :
         data_->payments_data_manager().GetLocalCreditCards()) {
      // No digits of its number, not even the last four.
      std::u16string title = card->nickname();
      if (title.empty()) {
        title = card->GetRawInfo(autofill::CREDIT_CARD_NAME_FULL);
      }
      entries.push_back({.key = card->guid(),
                         .title = base::UTF16ToUTF8(title),
                         .detail = base::UTF16ToUTF8(
                             Joined(card->NetworkForDisplay(),
                                    card->AbbreviatedExpirationDateForDisplay(
                                        /*with_prefix=*/true))),
                         .card = true});
    }
    return entries;
  }

  void Report() {
    if (!Host().profile_autofill_changed) {
      return;
    }
    const std::vector<Entry> entries = All();
    std::vector<bc_autofill_entry> c_entries;
    for (const Entry& entry : entries) {
      c_entries.push_back({
          .title = entry.title.c_str(),
          .detail = entry.detail.c_str(),
          .card = entry.card,
          .key = entry.key.c_str(),
      });
    }
    Host().profile_autofill_changed(Host().context, ProfileId(DirOf(profile_)), c_entries.data(),
                                    c_entries.size());
  }

  // PersonalDataManagerObserver: they were read, or changed.
  void OnPersonalDataChanged() override { Changed(); }

  // ProfileObserver: the manager goes with the profile. This destroys this.
  void OnProfileWillBeDestroyed(Profile* profile) override;

  const raw_ptr<Profile> profile_;
  const raw_ptr<PersonalDataManager> data_;
  base::ScopedObservation<PersonalDataManager, autofill::PersonalDataManagerObserver>
      data_observation_{this};
  base::ScopedObservation<Profile, ProfileObserver> profile_observation_{this};
  base::WeakPtrFactory<Watcher> weak_factory_{this};
};

// Each profile's, once the app asked.
std::map<Profile*, std::unique_ptr<Watcher>>& Watchers() {
  static base::NoDestructor<std::map<Profile*, std::unique_ptr<Watcher>>> watchers;
  return *watchers;
}

void Watcher::OnProfileWillBeDestroyed(Profile* profile) {
  Watchers().erase(profile);
}

// The loaded profile of |id|'s watcher, made if there's none yet, or null.
Watcher* WatcherOf(bc_id id) {
  const std::string dir = ProfileDir(id);
  Profile* profile = dir.empty() ? nullptr : ProfileFor(dir);
  // ProfileFor falls back to the profile the core started with, whose data isn't this.
  if (!profile || DirOf(profile) != dir) {
    return nullptr;
  }
  std::unique_ptr<Watcher>& watcher = Watchers()[profile];
  if (!watcher) {
    PersonalDataManager* data = autofill::PersonalDataManagerFactory::GetForBrowserContext(profile);
    if (!data) {
      Watchers().erase(profile);
      return nullptr;
    }
    watcher = std::make_unique<Watcher>(profile, data);
  }
  return watcher.get();
}

}  // namespace

void Read(bc_id profile) {
  if (Watcher* watcher = WatcherOf(profile)) {
    watcher->Changed();
  }
}

void Remove(bc_id profile, const std::string& key) {
  Watcher* watcher = WatcherOf(profile);
  if (!watcher || key.empty()) {
    return;
  }
  PersonalDataManager* data = watcher->data();
  if (data->address_data_manager().GetProfileByGUID(key)) {
    data->address_data_manager().RemoveProfile(key);
  } else if (const autofill::CreditCard* card =
                 data->payments_data_manager().GetCreditCardByGUID(key);
             card && card->record_type() == autofill::CreditCard::RecordType::kLocalCard) {
    data->payments_data_manager().RemoveByGUID(key);
  }
  // The manager tells its observers once the database has it.
}

}  // namespace browser_core::autofill_data
