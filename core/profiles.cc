#include "browser_core/profiles.h"

#include <algorithm>
#include <map>
#include <memory>
#include <utility>

#include "base/barrier_closure.h"
#include "base/files/file_path.h"
#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/functional/callback_helpers.h"
#include "base/no_destructor.h"
#include "base/scoped_observation.h"
#include "base/strings/string_number_conversions.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/downloads.h"
#include "browser_core/extensions.h"
#include "browser_core/favorites.h"
#include "browser_core/host.h"
#include "browser_core/search.h"
#include "browser_core/space.h"
#include "chrome/browser/browser_process.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "chrome/browser/profiles/delete_profile_helper.h"
#include "chrome/browser/profiles/keep_alive/profile_keep_alive_types.h"
#include "chrome/browser/profiles/keep_alive/scoped_profile_keep_alive.h"
#include "chrome/browser/profiles/nuke_profile_directory_utils.h"
#include "chrome/browser/profiles/profile.h"
#include "chrome/browser/profiles/profile_attributes_entry.h"
#include "chrome/browser/profiles/profile_attributes_storage.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/browser/profiles/profile_metrics.h"
#include "chrome/browser/profiles/profile_observer.h"
#include "content/public/browser/web_contents.h"

namespace browser_core {

namespace {

// The profiles the core keeps loaded, by directory.
std::map<std::string, std::unique_ptr<ScopedProfileKeepAlive>>& Used() {
  static base::NoDestructor<std::map<std::string, std::unique_ptr<ScopedProfileKeepAlive>>> used;
  return *used;
}

// Each profile's id, by directory, and back.
std::map<std::string, bc_id>& Ids() {
  static base::NoDestructor<std::map<std::string, bc_id>> ids;
  return *ids;
}

base::FilePath PathOf(const std::string& dir) {
  ProfileManager* manager = g_browser_process->profile_manager();
  return manager ? manager->user_data_dir().AppendASCII(dir) : base::FilePath();
}

// Tells the app, after the call that changed them has returned, as the Spaces do.
void ReportChanged() {
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce([] {
        if (Host().spaces_changed) {
          Host().spaces_changed(Host().context);
        }
      }));
}

// Keeps |profile| once it has loaded, if it did, then runs |loaded|.
void OnLoaded(base::OnceClosure loaded, Profile* profile) {
  if (profile) {
    UseProfile(profile);
  }
  std::move(loaded).Run();
}

// Tells the app once a private session ends, as Chrome destroys its off-the-record profile.
class SessionEnd : public ProfileObserver {
 public:
  explicit SessionEnd(Profile* profile) { observation_.Observe(profile); }

  void OnProfileWillBeDestroyed(Profile* profile) override;

 private:
  base::ScopedObservation<Profile, ProfileObserver> observation_{this};
};

// The private sessions running, by their off-the-record profile.
std::map<Profile*, std::unique_ptr<SessionEnd>>& Sessions() {
  static base::NoDestructor<std::map<Profile*, std::unique_ptr<SessionEnd>>> sessions;
  return *sessions;
}

void SessionEnd::OnProfileWillBeDestroyed(Profile* profile) {
  const bc_id id = ProfileId(DirOf(profile));
  // Deletes this.
  Sessions().erase(profile);
  // A quit ends them all, and the app has stopped hearing.
  if (browser_shutdown::IsTryingToQuit()) {
    return;
  }
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](bc_id profile) {
                       if (Host().private_session_ended) {
                         Host().private_session_ended(Host().context, profile);
                       }
                     },
                     id));
}

// Chrome numbers the directories it makes in order: "Default", then "Profile 1", "Profile 2".
int Order(const std::string& dir) {
  int number = 0;
  if (dir == "Default" || !base::StringToInt(dir.substr(dir.rfind(' ') + 1), &number)) {
    return dir == "Default" ? -1 : 0;
  }
  return number;
}

// What Chrome keeps about |profile|, such as its name, or null.
ProfileAttributesEntry* AttributesOf(bc_id profile) {
  ProfileManager* manager = g_browser_process->profile_manager();
  const std::string dir = ProfileDir(profile);
  return manager && !dir.empty()
             ? manager->GetProfileAttributesStorage().GetProfileAttributesWithPath(PathOf(dir))
             : nullptr;
}

}  // namespace

Profile* StartProfile() {
  // Chrome made the profile it starts with before the core's calls work. Its last used changes as
  // other profiles' Browsers become active, and this one doesn't.
  static Profile* start = nullptr;
  if (!start) {
    start = ProfileManager::GetLastUsedProfileIfLoaded();
  }
  return start;
}

Profile* ProfileFor(const std::string& dir) {
  ProfileManager* manager = g_browser_process->profile_manager();
  Profile* profile = manager && !dir.empty() ? manager->GetProfileByPath(PathOf(dir)) : nullptr;
  return profile ? profile : StartProfile();
}

std::string DirOf(Profile* profile) {
  return profile ? profile->GetBaseName().AsUTF8Unsafe() : std::string();
}

std::string DirOf(content::WebContents* contents) {
  return contents ? DirOf(Profile::FromBrowserContext(contents->GetBrowserContext()))
                  : std::string();
}

void LoadProfiles(const std::set<std::string>& dirs, base::OnceClosure done) {
  ProfileManager* manager = g_browser_process->profile_manager();
  if (!manager) {
    std::move(done).Run();
    return;
  }
  base::RepeatingClosure loaded = base::BarrierClosure(dirs.size(), std::move(done));
  for (const std::string& dir : dirs) {
    // Waits for one still being made. One deleted by hand comes back empty.
    manager->CreateProfileAsync(PathOf(dir), base::BindOnce(&OnLoaded, loaded));
  }
}

void UseProfile(Profile* profile) {
  std::unique_ptr<ScopedProfileKeepAlive>& kept = Used()[DirOf(profile)];
  if (kept) {
    return;
  }
  // Chrome unloads a profile once its last Browser closes, and loading it again would hold up the
  // next new tab, so the core keeps the ones its Spaces use.
  kept = std::make_unique<ScopedProfileKeepAlive>(profile, ProfileKeepAliveOrigin::kAppWindow);
  SetUpSearch(profile);
  ObserveDownloads(profile);
  ObserveExtensions(profile);
}

void ReleaseProfiles() {
  Used().clear();
}

Profile* PrivateProfileOf(Profile* profile) {
  Profile* off_the_record = profile->GetPrimaryOTRProfile(/*create_if_needed=*/true);
  if (off_the_record && !Sessions().contains(off_the_record)) {
    Sessions()[off_the_record] = std::make_unique<SessionEnd>(off_the_record);
  }
  return off_the_record;
}

std::vector<bc_id> Profiles() {
  ProfileManager* manager = g_browser_process->profile_manager();
  if (!manager) {
    return {};
  }
  std::vector<std::string> dirs;
  for (ProfileAttributesEntry* entry :
       manager->GetProfileAttributesStorage().GetAllProfilesAttributes()) {
    if (!entry->IsOmitted() && !entry->IsEphemeral() &&
        !IsProfileDirectoryMarkedForDeletion(entry->GetPath())) {
      dirs.push_back(entry->GetPath().BaseName().AsUTF8Unsafe());
    }
  }
  std::ranges::sort(dirs, {}, &Order);
  // The one the core started with first: it can't be deleted.
  std::ranges::stable_partition(dirs,
                                [](const std::string& dir) { return dir == DirOf(StartProfile()); });
  std::vector<bc_id> profiles;
  for (const std::string& dir : dirs) {
    profiles.push_back(ProfileId(dir));
  }
  return profiles;
}

bc_id ProfileId(const std::string& dir) {
  auto [it, made] = Ids().try_emplace(dir, 0);
  if (made) {
    it->second = NewId();
  }
  return it->second;
}

std::string ProfileDir(bc_id profile) {
  for (const auto& [dir, id] : Ids()) {
    if (id == profile) {
      return dir;
    }
  }
  return std::string();
}

std::string ProfileName(bc_id profile) {
  ProfileAttributesEntry* entry = AttributesOf(profile);
  if (!entry) {
    return std::string();
  }
  // Chrome's name for a profile no one named, "Your Chromium", names the browser, not the profile.
  return entry->IsUsingDefaultName() ? "Default" : base::UTF16ToUTF8(entry->GetLocalProfileName());
}

bool RenameProfile(bc_id profile, const std::string& name) {
  ProfileAttributesEntry* entry = AttributesOf(profile);
  if (!entry || name.empty()) {
    return false;
  }
  entry->SetLocalProfileName(base::UTF8ToUTF16(name), /*is_default_name=*/false);
  ReportChanged();
  return true;
}

bc_id CreateProfile(const std::string& name, base::OnceCallback<void(Profile*)> made) {
  ProfileManager* manager = g_browser_process->profile_manager();
  if (!manager) {
    return 0;
  }
  const base::FilePath path = manager->GenerateNextProfileDirectoryPath();
  manager->CreateProfileAsync(
      path, base::BindOnce(
                [](std::u16string name, base::OnceCallback<void(Profile*)> made,
                   Profile* profile) {
                  if (profile) {
                    if (ProfileAttributesEntry* entry =
                            g_browser_process->profile_manager()
                                ->GetProfileAttributesStorage()
                                .GetProfileAttributesWithPath(profile->GetPath())) {
                      entry->SetLocalProfileName(name, /*is_default_name=*/false);
                    }
                    UseProfile(profile);
                  }
                  ReportChanged();
                  std::move(made).Run(profile);
                },
                base::UTF8ToUTF16(name), std::move(made)));
  return ProfileId(path.BaseName().AsUTF8Unsafe());
}

bool DeleteProfile(bc_id profile) {
  ProfileManager* manager = g_browser_process->profile_manager();
  const std::string dir = ProfileDir(profile);
  if (!manager || dir.empty() || dir == DirOf(StartProfile()) || Profiles().size() < 2) {
    return false;
  }
  for (Space* space : Space::All()) {
    if (space->profile() == dir) {
      return false;
    }
  }
  if (!Favorites::Of(dir).empty()) {
    return false;
  }
  // Nor while a private window of it is open.
  if (Profile* loaded = ProfileFor(dir);
      DirOf(loaded) == dir && loaded->HasPrimaryOTRProfile()) {
    return false;
  }
  // Let go first: Chrome deletes it once it's unloaded.
  Used().erase(dir);
  manager->GetDeleteProfileHelper().MaybeScheduleProfileForDeletion(
      PathOf(dir), base::DoNothing(), ProfileMetrics::DELETE_PROFILE_SETTINGS);
  Ids().erase(dir);
  ReportChanged();
  return true;
}

}  // namespace browser_core
