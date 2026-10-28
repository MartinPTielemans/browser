#ifndef BROWSER_CORE_PASSWORDS_H_
#define BROWSER_CORE_PASSWORDS_H_

#include <memory>
#include <string>
#include <vector>

#include "base/files/file_path.h"
#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "base/scoped_observation.h"
#include "browser_core/public/browser_core.h"
#include "chrome/browser/profiles/profile_observer.h"
#include "components/password_manager/core/browser/import/password_importer.h"
#include "components/password_manager/core/browser/ui/saved_passwords_presenter.h"

class GURL;
class Profile;

namespace browser_core {

// A profile's saved logins as the app lists them and the user removes them, in place of Chrome's
// password manager page and its manage bubble (profile_logins_changed, bc_site_controls' logins).
// Read the first time something asks for them, and kept up to date from then while the profile is
// loaded. The passwords stay in the core: the app gets each login's site and username.
class Passwords : public password_manager::SavedPasswordsPresenter::Observer,
                  public ProfileObserver {
 public:
  // A login as bc_login passes it, with the strings it points to.
  struct Login {
    std::string site;
    std::string username;
    bool never = false;
    std::string key;
  };

  // |profile|'s, which a private profile shares with its original, made and reading the store if
  // it has none yet. Null without a profile.
  static Passwords* Of(Profile* profile);
  // See bc_profile_read_logins and bc_profile_remove_login.
  static void Read(bc_id profile);
  static void Remove(bc_id profile,
                     const std::string& key,
                     const std::string& username,
                     bool never);
  // See bc_profile_import_passwords.
  static bool Import(bc_id profile, const base::FilePath& path);
  // The C API's view of |logins|, which it points into.
  static std::vector<bc_login> ToC(const std::vector<Login>& logins);

  Passwords(const Passwords&) = delete;
  Passwords& operator=(const Passwords&) = delete;
  ~Passwords() override;

  // Whether the store has been read, before which the lists are empty.
  bool ready() const { return ready_; }
  // The logins kept for the page at |url|: the saved ones that Chrome's password manager offers
  // across its registrable domain, then Never, if its site is set to it.
  std::vector<Login> ForPage(const GURL& url);

 private:
  explicit Passwords(Profile* profile);

  // Every login, the saved ones by site and username, then the sites set to Never.
  std::vector<Login> All();

  // SavedPasswordsPresenter::Observer: the store was read, or changed.
  void OnSavedPasswordsChanged(const password_manager::PasswordStoreChangeList& changes) override;
  // ProfileObserver: the presenter can't outlive the profile's stores. This destroys this.
  void OnProfileWillBeDestroyed(Profile* profile) override;

  // Imports |import_path_| once the store is read, as the importer weighs the file against it.
  void StartImport();
  // The importer's answer: conflicts, which keep what the profile has, or the end.
  void Imported(const password_manager::ImportResults& results);

  // Tells the app, and the Site Controls, from a task of its own.
  void Changed();
  void Report();

  const raw_ptr<Profile> profile_;
  password_manager::SavedPasswordsPresenter presenter_;
  bool ready_ = false;
  // Whether the app asked for the list, which it then hears again as it changes.
  bool read_ = false;
  // The file being imported, from bc_profile_import_passwords until passwords_imported.
  base::FilePath import_path_;
  // Logins of the file the profile holds with another password, which it kept.
  size_t conflicted_ = 0;
  // While a file is imported. After the presenter, which it points to.
  std::unique_ptr<password_manager::PasswordImporter> importer_;
  base::ScopedObservation<Profile, ProfileObserver> profile_observation_{this};
  base::WeakPtrFactory<Passwords> weak_factory_{this};
};

}  // namespace browser_core

#endif  // BROWSER_CORE_PASSWORDS_H_
