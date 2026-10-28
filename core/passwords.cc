#include "browser_core/passwords.h"

#include <algorithm>
#include <map>
#include <memory>
#include <tuple>
#include <utility>

#include "base/functional/bind.h"
#include "base/memory/ptr_util.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/sequenced_task_runner.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/profiles.h"
#include "browser_core/site_controls.h"
#include "chrome/browser/affiliations/affiliation_service_factory.h"
#include "chrome/browser/password_manager/factories/account_password_store_factory.h"
#include "chrome/browser/password_manager/factories/profile_password_store_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "components/keyed_service/core/service_access_type.h"
#include "components/password_manager/core/browser/import/import_results.h"
#include "components/password_manager/core/browser/password_form.h"
#include "components/password_manager/core/browser/password_manager_util.h"
#include "components/password_manager/core/browser/password_ui_utils.h"
#include "components/password_manager/core/browser/ui/credential_ui_entry.h"
#include "net/base/registry_controlled_domains/registry_controlled_domain.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

using password_manager::CredentialUIEntry;

// Each original profile's, once something asked.
std::map<Profile*, std::unique_ptr<Passwords>>& Instances() {
  static base::NoDestructor<std::map<Profile*, std::unique_ptr<Passwords>>> instances;
  return *instances;
}

// The loaded profile of |id|, or null.
Profile* ProfileOf(bc_id id) {
  const std::string dir = ProfileDir(id);
  Profile* profile = dir.empty() ? nullptr : ProfileFor(dir);
  // ProfileFor falls back to the profile the core started with, whose logins aren't these.
  return profile && DirOf(profile) == dir ? profile : nullptr;
}

Passwords::Login LoginOf(const CredentialUIEntry& credential) {
  return {
      .site = password_manager::GetShownOrigin(credential),
      .username = credential.blocked_by_user ? std::string()
                                             : base::UTF16ToUTF8(credential.username),
      .never = credential.blocked_by_user,
      .key = credential.GetFirstSignonRealm(),
  };
}

// The saved logins by site and username, then the sites set to Never by site.
void Sort(std::vector<Passwords::Login>& logins) {
  std::ranges::stable_sort(logins, [](const Passwords::Login& a, const Passwords::Login& b) {
    return std::tie(a.never, a.site, a.username) < std::tie(b.never, b.site, b.username);
  });
}

}  // namespace

Passwords* Passwords::Of(Profile* profile) {
  if (!profile) {
    return nullptr;
  }
  // A private window's logins are saved to its original profile, as in Chrome.
  profile = profile->GetOriginalProfile();
  std::unique_ptr<Passwords>& passwords = Instances()[profile];
  if (!passwords) {
    passwords = base::WrapUnique(new Passwords(profile));
  }
  return passwords.get();
}

void Passwords::Read(bc_id profile) {
  Passwords* passwords = Of(ProfileOf(profile));
  if (!passwords) {
    return;
  }
  passwords->read_ = true;
  if (passwords->ready_) {
    passwords->Changed();
  }
}

void Passwords::Remove(bc_id profile,
                       const std::string& key,
                       const std::string& username,
                       bool never) {
  Passwords* passwords = Of(ProfileOf(profile));
  if (!passwords || !passwords->ready_) {
    return;
  }
  const std::u16string username16 = base::UTF8ToUTF16(username);
  // The lists the app was given: GetSavedCredentials() has no sites set to Never, and passkeys
  // and federated logins aren't listed, so aren't the user's to remove here.
  const std::vector<CredentialUIEntry> listed = never ? passwords->presenter_.GetBlockedSites()
                                                      : passwords->presenter_.GetSavedPasswords();
  for (const CredentialUIEntry& credential : listed) {
    if (credential.GetFirstSignonRealm() != key || (!never && credential.username != username16)) {
      continue;
    }
    // With its copies in the other store, as Chrome's password manager page removes it. The store
    // tells the presenter, which tells this.
    passwords->presenter_.RemoveCredential(credential);
    return;
  }
}

bool Passwords::Import(bc_id profile, const base::FilePath& path) {
  Passwords* passwords = Of(ProfileOf(profile));
  if (!passwords || path.empty() || !passwords->import_path_.empty()) {
    return false;
  }
  passwords->import_path_ = path;
  if (passwords->ready_) {
    passwords->StartImport();
  }
  return true;
}

void Passwords::StartImport() {
  importer_ = std::make_unique<password_manager::PasswordImporter>(presenter_);
  importer_->Import(import_path_, password_manager::PasswordForm::Store::kProfileStore,
                    base::BindOnce(&Passwords::Imported, weak_factory_.GetWeakPtr()));
}

void Passwords::Imported(const password_manager::ImportResults& results) {
  using password_manager::ImportResults;
  if (results.status == ImportResults::CONFLICTS) {
    // Chrome's page asks which to replace. The saved password is the one the user signs in with
    // here, so it stays, and the counts say how many did. Continued from a task of its own, as the
    // importer calls this before it's done with the call.
    conflicted_ = results.displayed_entries.size();
    base::SequencedTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(
                       [](base::WeakPtr<Passwords> passwords) {
                         if (passwords && passwords->importer_) {
                           passwords->importer_->ContinueImport(
                               {}, base::BindOnce(&Passwords::Imported, passwords));
                         }
                       },
                       weak_factory_.GetWeakPtr()));
    return;
  }
  int result = BC_PASSWORDS_IMPORT_FAILED;
  switch (results.status) {
    case ImportResults::SUCCESS:
      result = BC_PASSWORDS_IMPORTED;
      break;
    case ImportResults::IO_ERROR:
      result = BC_PASSWORDS_IMPORT_UNREADABLE;
      break;
    case ImportResults::BAD_FORMAT:
      result = BC_PASSWORDS_IMPORT_NOT_EXPORT;
      break;
    case ImportResults::MAX_FILE_SIZE:
    case ImportResults::NUM_PASSWORDS_EXCEEDED:
      result = BC_PASSWORDS_IMPORT_TOO_LARGE;
      break;
    default:
      break;
  }
  const bool imported = result == BC_PASSWORDS_IMPORTED;
  // After a success, the rows it lists are the ones it couldn't take.
  const size_t skipped = imported ? results.displayed_entries.size() : 0;
  const size_t conflicted = std::exchange(conflicted_, 0);
  import_path_.clear();
  // The importer is still in the call that brought this.
  base::SequencedTaskRunner::GetCurrentDefault()->DeleteSoon(FROM_HERE, std::move(importer_));
  if (Host().passwords_imported) {
    Host().passwords_imported(Host().context, ProfileId(DirOf(profile_)), result,
                              imported ? results.number_imported : 0, imported ? conflicted : 0,
                              skipped);
  }
}

std::vector<bc_login> Passwords::ToC(const std::vector<Login>& logins) {
  std::vector<bc_login> c_logins;
  c_logins.reserve(logins.size());
  for (const Login& login : logins) {
    c_logins.push_back({
        .site = login.site.c_str(),
        .username = login.username.c_str(),
        .never = login.never,
        .key = login.key.c_str(),
    });
  }
  return c_logins;
}

Passwords::Passwords(Profile* profile)
    : profile_(profile),
      presenter_(AffiliationServiceFactory::GetForProfile(profile),
                 ProfilePasswordStoreFactory::GetForProfile(profile,
                                                            ServiceAccessType::EXPLICIT_ACCESS),
                 AccountPasswordStoreFactory::GetForProfile(profile,
                                                            ServiceAccessType::EXPLICIT_ACCESS)) {
  profile_observation_.Observe(profile);
  presenter_.AddObserver(this);
  presenter_.Init();
  // Without a store, which some configurations have, there's nothing to read.
  if (!ProfilePasswordStoreFactory::GetForProfile(profile, ServiceAccessType::EXPLICIT_ACCESS)) {
    ready_ = true;
  }
}

Passwords::~Passwords() {
  presenter_.RemoveObserver(this);
}

std::vector<Passwords::Login> Passwords::ForPage(const GURL& url) {
  std::vector<Login> logins;
  if (!url.SchemeIsHTTPOrHTTPS()) {
    return logins;
  }
  for (const CredentialUIEntry& credential : presenter_.GetSavedPasswords()) {
    // As Chrome's password manager fills a login on the other hosts of its domain, by the public
    // suffix list, and on its affiliated sites.
    const bool matches = std::ranges::any_of(credential.facets, [&url](const auto& facet) {
      return net::registry_controlled_domains::SameDomainOrHost(
          url, facet.url, net::registry_controlled_domains::INCLUDE_PRIVATE_REGISTRIES);
    });
    if (matches) {
      logins.push_back(LoginOf(credential));
    }
  }
  // Never is kept for one site, and the offer still comes on the domain's others.
  const std::string realm = password_manager_util::GetSignonRealm(url);
  for (const CredentialUIEntry& credential : presenter_.GetBlockedSites()) {
    if (std::ranges::any_of(credential.facets,
                            [&realm](const auto& facet) { return facet.signon_realm == realm; })) {
      logins.push_back(LoginOf(credential));
    }
  }
  Sort(logins);
  return logins;
}

std::vector<Passwords::Login> Passwords::All() {
  std::vector<Login> logins;
  for (const CredentialUIEntry& credential : presenter_.GetSavedPasswords()) {
    logins.push_back(LoginOf(credential));
  }
  for (const CredentialUIEntry& credential : presenter_.GetBlockedSites()) {
    logins.push_back(LoginOf(credential));
  }
  Sort(logins);
  return logins;
}

void Passwords::OnSavedPasswordsChanged(const password_manager::PasswordStoreChangeList& changes) {
  const bool first = !ready_;
  ready_ = true;
  if (first && !import_path_.empty()) {
    StartImport();
  }
  Changed();
}

void Passwords::OnProfileWillBeDestroyed(Profile* profile) {
  Instances().erase(profile);
}

void Passwords::Changed() {
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Passwords::Report, weak_factory_.GetWeakPtr()));
}

void Passwords::Report() {
  SiteControls::LoginsChanged(profile_);
  if (!read_ || !Host().profile_logins_changed) {
    return;
  }
  const std::vector<Login> logins = All();
  const std::vector<bc_login> c_logins = ToC(logins);
  Host().profile_logins_changed(Host().context, ProfileId(DirOf(profile_)), c_logins.data(),
                                c_logins.size());
}

}  // namespace browser_core
