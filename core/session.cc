#include "browser_core/session.h"

#include <algorithm>
#include <map>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "base/base64.h"
#include "base/callback_list.h"
#include "base/files/file_path.h"
#include "base/files/file_util.h"
#include "base/files/important_file_writer.h"
#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/path_service.h"
#include "base/pickle.h"
#include "base/task/sequenced_task_runner.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "base/values.h"
#include "browser_core/archive.h"
#include "browser_core/favorites.h"
#include "browser_core/profiles.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/lifetime/application_lifetime_desktop.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/common/chrome_paths.h"
#include "components/sessions/content/content_serialized_navigation_builder.h"
#include "components/sessions/core/serialized_navigation_entry.h"
#include "components/sessions/core/session_constants.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/web_contents.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

struct Saved {
  // Left to right.
  std::vector<SavedTab> favorites;
  std::vector<SavedSpace> spaces;
  // The Space the window showed.
  int shown = 0;
  // The window's docked sidebar's width, or 0 for none.
  double sidebar_width = 0;
  std::vector<ArchivedTab> archive;
  Settings settings;
};

// A navigation's page state is left out past this, as in Chrome's sessions.
constexpr int kMaxNavigationSize = 63 * 1024;

// How many folders |id| is in.
int DepthOf(const Space& space, bc_id id) {
  int depth = 0;
  for (bc_id folder = space.FolderOf(id); folder; folder = space.FolderOf(folder)) {
    ++depth;
  }
  return depth;
}

}  // namespace

SavedTab SaveTab(content::NavigationController& history) {
  SavedTab tab;
  const int current = history.GetCurrentEntryIndex();
  const int pending = history.GetPendingEntryIndex();
  const int end =
      std::min(current + sessions::gMaxPersistNavigationCount + 1, history.GetEntryCount());
  for (int i = std::max(current - sessions::gMaxPersistNavigationCount, 0); i < end; ++i) {
    content::NavigationEntry* entry =
        i == pending ? history.GetPendingEntry() : history.GetEntryAtIndex(i);
    // A tab starts with an entry for no page, until its first page commits.
    if (entry->IsInitialEntry()) {
      continue;
    }
    if (i == current) {
      tab.current = tab.history.size();
    }
    tab.history.push_back(sessions::ContentSerializedNavigationBuilder::FromNavigationEntry(
        tab.history.size(), entry));
  }
  // One whose first page hasn't committed keeps the page it's loading.
  if (tab.history.empty() && history.GetPendingEntry()) {
    tab.history.push_back(
        sessions::ContentSerializedNavigationBuilder::FromNavigationEntry(0, history.GetPendingEntry()));
  }
  return tab;
}

SavedSpace SaveSpace(const Space& space, bc_id shown, std::vector<bc_id>* today) {
  SavedSpace to;
  to.name = space.name();
  to.color = space.color();
  to.profile = space.profile();
  int index = 0;
  for (auto [ids, tabs] : {std::pair(&space.pinned_rows(), &to.pinned),
                           std::pair(&space.today(), &to.today)}) {
    bc_id above = 0;
    for (bc_id id : *ids) {
      if (space.IsFolder(id)) {
        to.folders.push_back({.name = space.FolderName(id),
                              .open = space.IsOpen(id),
                              .depth = DepthOf(space, id),
                              .at = to.pinned.size()});
        continue;
      }
      content::WebContents* contents = Tab::FromId(id);
      SavedTab tab = contents ? SaveTab(contents->GetController()) : SavedTab();
      if (tab.history.empty()) {
        continue;
      }
      if (id == shown) {
        to.shown = index;
      }
      tab.split = above && std::ranges::contains(space.Panes(id), above);
      tab.stacked = tab.split && space.Stacked(id);
      if (std::vector<double> shares = space.Shares(id); !shares.empty()) {
        std::vector<bc_id> panes = space.Panes(id);
        tab.share = shares[std::ranges::find(panes, id) - panes.begin()];
      }
      above = id;
      tab.pinned_url = space.PinnedUrl(id);
      tab.name = Tab::FromWebContents(contents)->name();
      tab.depth = DepthOf(space, id);
      if (tabs == &to.today) {
        tab.last_seen = Window::Showing(id) ? base::Time::Now()
                                            : Tab::FromWebContents(contents)->last_seen();
        if (today) {
          today->push_back(id);
        }
      }
      tabs->push_back(std::move(tab));
      ++index;
    }
  }
  return to;
}

namespace {

Saved Collect() {
  Saved saved;
  const std::vector<Window*> windows = Window::All();
  const Window* window = windows.empty() ? nullptr : windows.front();
  saved.sidebar_width = window ? window->sidebar_width() : 0;
  saved.settings = UserSettings();
  // Each favorite saved, by its place among them.
  std::map<bc_id, int> favorites;
  for (bc_id id : Favorites::All()) {
    content::WebContents* contents = Tab::FromId(id);
    SavedTab tab = contents ? SaveTab(contents->GetController()) : SavedTab();
    if (tab.history.empty()) {
      continue;
    }
    tab.pinned_url = Favorites::Url(id);
    tab.name = Tab::FromWebContents(contents)->name();
    tab.profile = Favorites::ProfileOf(id);
    favorites[id] = saved.favorites.size();
    saved.favorites.push_back(std::move(tab));
  }
  for (Space* space : Space::All()) {
    if (window && space->id() == window->space()) {
      saved.shown = saved.spaces.size();
    }
    const bc_id shown = window ? window->SelectedIn(space->id()) : 0;
    SavedSpace& to = saved.spaces.emplace_back(SaveSpace(*space, shown));
    if (auto favorite = favorites.find(shown); favorite != favorites.end()) {
      to.shown_favorite = favorite->second;
    }
  }
  saved.archive = Archived();
  return saved;
}

// The file holds JSON, with each navigation in Chrome's own pickle, as its sessions write it.
std::optional<std::string> ToJson(const Saved& saved) {
  const auto to_list = [](const std::vector<SavedTab>& tabs) {
    base::ListValue list;
    for (const SavedTab& tab : tabs) {
      base::ListValue history;
      for (const sessions::SerializedNavigationEntry& navigation : tab.history) {
        base::Pickle pickle;
        navigation.WriteToPickle(kMaxNavigationSize, &pickle);
        history.Append(base::Base64Encode(pickle.AsBytes()));
      }
      base::DictValue dict;
      dict.Set("history", std::move(history));
      dict.Set("current", tab.current);
      if (tab.pinned_url.is_valid()) {
        dict.Set("pinned_url", tab.pinned_url.spec());
      }
      if (!tab.last_seen.is_null()) {
        dict.Set("seen", tab.last_seen.InMillisecondsFSinceUnixEpoch());
      }
      if (!tab.name.empty()) {
        dict.Set("name", tab.name);
      }
      if (tab.split) {
        dict.Set("split", true);
      }
      if (tab.stacked) {
        dict.Set("stacked", true);
      }
      if (tab.share) {
        dict.Set("share", tab.share);
      }
      if (tab.depth) {
        dict.Set("depth", tab.depth);
      }
      if (!tab.profile.empty()) {
        dict.Set("profile", tab.profile);
      }
      list.Append(std::move(dict));
    }
    return list;
  };
  base::ListValue spaces;
  for (const SavedSpace& space : saved.spaces) {
    // Each folder in the pinned list, above what's in it.
    base::ListValue pinned = to_list(space.pinned);
    for (auto folder = space.folders.rbegin(); folder != space.folders.rend(); ++folder) {
      pinned.Insert(pinned.begin() + folder->at, base::Value(base::DictValue()
                                                                 .Set("folder", folder->name)
                                                                 .Set("open", folder->open)
                                                                 .Set("depth", folder->depth)));
    }
    base::DictValue dict = base::DictValue()
                               .Set("name", space.name)
                               .Set("pinned", std::move(pinned))
                               .Set("today", to_list(space.today))
                               .Set("shown", space.shown);
    if (space.shown_favorite >= 0) {
      dict.Set("shown_favorite", space.shown_favorite);
    }
    if (space.color) {
      dict.Set("color", static_cast<int>(*space.color));
    }
    if (!space.profile.empty()) {
      dict.Set("profile", space.profile);
    }
    spaces.Append(std::move(dict));
  }
  base::DictValue root;
  root.Set("favorites", to_list(saved.favorites));
  root.Set("spaces", std::move(spaces));
  root.Set("shown", saved.shown);
  if (saved.sidebar_width > 0) {
    root.Set("sidebar_width", saved.sidebar_width);
  }
  base::ListValue archive;
  for (const ArchivedTab& tab : saved.archive) {
    archive.Append(base::DictValue()
                       .Set("url", tab.url.spec())
                       .Set("title", tab.title)
                       .Set("archived", tab.archived.InMillisecondsFSinceUnixEpochIgnoringNull()));
  }
  root.Set("archive", std::move(archive));
  root.Set("settings", base::DictValue()
                           .Set("archive_after", saved.settings.archive_after.InSecondsF())
                           .Set("haptics", saved.settings.haptics)
                           .Set("warn_before_quitting", saved.settings.warn_before_quitting)
                           .Set("warn_moving_profiles", saved.settings.warn_moving_profiles));
  return base::WriteJson(root);
}

std::optional<SavedTab> ReadTab(const base::Value& value) {
  const base::DictValue* dict = value.GetIfDict();
  const base::ListValue* history = dict ? dict->FindList("history") : nullptr;
  if (!history) {
    return std::nullopt;
  }
  SavedTab tab;
  for (const base::Value& navigation : *history) {
    const std::string* base64 = navigation.GetIfString();
    std::optional<std::vector<uint8_t>> bytes =
        base64 ? base::Base64Decode(*base64) : std::nullopt;
    if (!bytes) {
      return std::nullopt;
    }
    base::Pickle pickle = base::Pickle::WithUnownedBuffer(*bytes);
    base::PickleIterator iterator(pickle);
    if (!tab.history.emplace_back().ReadFromPickle(&iterator)) {
      return std::nullopt;
    }
  }
  tab.current = dict->FindInt("current").value_or(0);
  if (tab.current < 0 || tab.current >= static_cast<int>(tab.history.size())) {
    return std::nullopt;
  }
  if (const std::string* url = dict->FindString("pinned_url")) {
    tab.pinned_url = GURL(*url);
  }
  if (std::optional<double> seen = dict->FindDouble("seen")) {
    tab.last_seen = base::Time::FromMillisecondsSinceUnixEpoch(*seen);
  }
  if (const std::string* name = dict->FindString("name")) {
    tab.name = *name;
  }
  tab.split = dict->FindBool("split").value_or(false);
  tab.stacked = tab.split && dict->FindBool("stacked").value_or(false);
  tab.share = dict->FindDouble("share").value_or(0);
  tab.depth = std::max(dict->FindInt("depth").value_or(0), 0);
  if (const std::string* profile = dict->FindString("profile")) {
    tab.profile = *profile;
  }
  return tab;
}

// A tab or Space that can't be read is left out, and the rest come back.
std::optional<Saved> Read(const base::FilePath& path) {
  std::string json;
  if (!base::ReadFileToString(path, &json)) {
    return std::nullopt;
  }
  std::optional<base::DictValue> root = base::JSONReader::ReadDict(json, base::JSON_PARSE_RFC);
  const base::ListValue* spaces = root ? root->FindList("spaces") : nullptr;
  if (!spaces) {
    return std::nullopt;
  }
  Saved saved;
  // Where each favorite saved went among those read, or -1.
  std::vector<int> favorites;
  if (const base::ListValue* list = root->FindList("favorites")) {
    for (const base::Value& value : *list) {
      std::optional<SavedTab> tab = ReadTab(value);
      favorites.push_back(tab ? static_cast<int>(saved.favorites.size()) : -1);
      if (tab) {
        saved.favorites.push_back(std::move(*tab));
      }
    }
  }
  for (const base::Value& value : *spaces) {
    const base::DictValue* space = value.GetIfDict();
    const std::string* name = space ? space->FindString("name") : nullptr;
    const base::ListValue* today = space ? space->FindList("today") : nullptr;
    if (!name || !today) {
      continue;
    }
    SavedSpace& to = saved.spaces.emplace_back();
    to.name = *name;
    if (std::optional<int> color = space->FindInt("color")) {
      to.color = static_cast<uint32_t>(*color) & 0xffffff;
    }
    if (const std::string* profile = space->FindString("profile")) {
      to.profile = *profile;
    }
    const int shown = space->FindInt("shown").value_or(-1);
    const int shown_favorite = space->FindInt("shown_favorite").value_or(-1);
    if (shown_favorite >= 0 && shown_favorite < static_cast<int>(favorites.size())) {
      to.shown_favorite = favorites[shown_favorite];
    }
    // Saved before Spaces had pinned tabs, it has none.
    const base::ListValue no_pinned;
    const base::ListValue* pinned = space->FindList("pinned");
    int index = 0;
    int kept = 0;
    for (auto [list, tabs] : {std::pair(pinned ? pinned : &no_pinned, &to.pinned),
                              std::pair(today, &to.today)}) {
      for (const base::Value& row : *list) {
        const base::DictValue* dict = row.GetIfDict();
        const std::string* folder = dict ? dict->FindString("folder") : nullptr;
        if (folder && tabs == &to.pinned) {
          to.folders.push_back({.name = *folder,
                                .open = dict->FindBool("open").value_or(true),
                                .depth = std::max(dict->FindInt("depth").value_or(0), 0),
                                .at = to.pinned.size()});
          continue;
        }
        std::optional<SavedTab> tab = ReadTab(row);
        if (index++ == shown && tab) {
          to.shown = kept;
        }
        if (tab) {
          tabs->push_back(std::move(*tab));
          ++kept;
        }
      }
    }
  }
  if (saved.spaces.empty()) {
    return std::nullopt;
  }
  saved.shown = std::clamp(root->FindInt("shown").value_or(0), 0,
                           static_cast<int>(saved.spaces.size()) - 1);
  saved.sidebar_width = root->FindDouble("sidebar_width").value_or(0);
  if (const base::ListValue* archive = root->FindList("archive")) {
    for (const base::Value& value : *archive) {
      const base::DictValue* tab = value.GetIfDict();
      const std::string* url = tab ? tab->FindString("url") : nullptr;
      const std::string* title = tab ? tab->FindString("title") : nullptr;
      std::optional<double> archived = tab ? tab->FindDouble("archived") : std::nullopt;
      if (url && title && archived && GURL(*url).is_valid()) {
        saved.archive.push_back(
            {.url = GURL(*url),
             .title = *title,
             .archived = base::Time::FromMillisecondsSinceUnixEpoch(*archived)});
      }
    }
  }
  if (const base::DictValue* settings = root->FindDict("settings")) {
    const double archive_after = settings->FindDouble("archive_after").value_or(0);
    if (archive_after > 0) {
      saved.settings.archive_after = base::Seconds(archive_after);
    }
    saved.settings.haptics = settings->FindBool("haptics").value_or(true);
    saved.settings.warn_before_quitting =
        settings->FindBool("warn_before_quitting").value_or(true);
    saved.settings.warn_moving_profiles =
        settings->FindBool("warn_moving_profiles").value_or(true);
  }
  return saved;
}

class Saver : public base::ImportantFileWriter::BackgroundDataSerializer {
 public:
  explicit Saver(const base::FilePath& path)
      // Blocks shutdown, so the Spaces saved as the browser quits reach the disk.
      : task_runner_(base::ThreadPool::CreateSequencedTaskRunner(
            {base::MayBlock(), base::TaskPriority::USER_VISIBLE,
             base::TaskShutdownBehavior::BLOCK_SHUTDOWN})),
        writer_(path, task_runner_),
        closing_all_(chrome::AddClosingAllBrowsersCallback(
            base::BindRepeating(&Saver::OnClosingAll, base::Unretained(this)))) {}

  void Load(base::OnceClosure loaded) {
    task_runner_->PostTaskAndReplyWithResult(
        FROM_HERE, base::BindOnce(&Read, writer_.path()),
        base::BindOnce(&Saver::OnRead, base::Unretained(this), std::move(loaded)));
  }

  void Changed() {
    if (!kept_) {
      writer_.ScheduleWriteWithBackgroundDataSerializer(this);
    }
  }

  void Keep() {
    if (!kept_) {
      writer_.ScheduleWriteWithBackgroundDataSerializer(this);
      writer_.DoScheduledWrite();
      kept_ = true;
    }
  }

  void Flush() {
    if (writer_.HasPendingWrite()) {
      writer_.DoScheduledWrite();
    }
  }

  std::optional<Saved> TakeLoaded() { return std::exchange(loaded_, std::nullopt); }
  const std::optional<Saved>& loaded() const { return loaded_; }

  // base::ImportantFileWriter::BackgroundDataSerializer:
  base::ImportantFileWriter::BackgroundDataProducerCallback
  GetSerializedDataProducerForBackgroundSequence() override {
    // What's in the Spaces is read here, and written out as JSON on the writer's sequence.
    return base::BindOnce(&ToJson, Collect());
  }

 private:
  // At logout, restart or SIGTERM, Chrome writes what it must and exits without waiting for the
  // writer's sequence, so the Spaces are written here, as it allows.
  void OnClosingAll(bool closing) {
    if (!closing || kept_ ||
        browser_shutdown::GetShutdownType() != browser_shutdown::ShutdownType::kEndSession) {
      return;
    }
    kept_ = true;
    if (std::optional<std::string> json = ToJson(Collect())) {
      base::ImportantFileWriter::WriteFileAtomically(writer_.path(), *json);
    }
  }

  void OnRead(base::OnceClosure loaded, std::optional<Saved> saved) {
    if (saved) {
      LoadArchive(std::exchange(saved->archive, {}));
      UserSettings() = saved->settings;
    }
    // The profiles the Spaces and favorites are in load before they come back.
    std::set<std::string> profiles;
    if (saved) {
      for (const SavedTab& favorite : saved->favorites) {
        profiles.insert(favorite.profile);
      }
      for (const SavedSpace& space : saved->spaces) {
        profiles.insert(space.profile);
      }
      profiles.erase(std::string());
    }
    loaded_ = std::move(saved);
    LoadProfiles(profiles, std::move(loaded));
  }

  scoped_refptr<base::SequencedTaskRunner> task_runner_;
  base::ImportantFileWriter writer_;
  base::CallbackListSubscription closing_all_;
  std::optional<Saved> loaded_;
  bool kept_ = false;
};

// Made by LoadSession and never deleted, as Chrome's shutdown may still be writing its file.
Saver* g_saver = nullptr;

}  // namespace

void LoadSession(base::OnceClosure loaded) {
  base::FilePath dir;
  base::PathService::Get(chrome::DIR_USER_DATA, &dir);
  g_saver = new Saver(dir.Append(FILE_PATH_LITERAL("Spaces.json")));
  g_saver->Load(std::move(loaded));
}

void SessionChanged() {
  if (g_saver) {
    g_saver->Changed();
  }
}

void KeepSession() {
  if (g_saver) {
    g_saver->Keep();
  }
}

void FlushSession() {
  if (g_saver) {
    g_saver->Flush();
  }
}

Settings& UserSettings() {
  static Settings settings;
  return settings;
}

double SavedSidebarWidth() {
  return g_saver && g_saver->loaded() ? g_saver->loaded()->sidebar_width : 0;
}

namespace {

// Opens |saved| unloaded in |window|, as the tab it was. Returns its id, or 0.
bc_id RestoreSavedTab(Window& window, Profile* profile, const SavedTab& saved) {
  content::WebContents* contents = window.RestoreTab(profile, saved.history, saved.current);
  if (!contents) {
    return 0;
  }
  // Which makes its Tab.
  const bc_id tab = Tab::IdOf(contents);
  Tab::FromWebContents(contents)->set_name(saved.name);
  Tab::FromWebContents(contents)->Restored();
  if (!saved.last_seen.is_null()) {
    Tab::FromWebContents(contents)->set_last_seen(saved.last_seen);
  }
  return tab;
}

}  // namespace

Space* RestoreSpace(Window& window, Profile* profile, const SavedSpace& saved_space, bc_id id) {
  Space* space = Space::Create(saved_space.name, id);
  space->SetColor(saved_space.color);
  space->SetProfile(DirOf(profile));
  const auto restore = [&](const SavedTab& saved_tab, int index) -> bc_id {
    const bc_id tab = RestoreSavedTab(window, profile, saved_tab);
    if (tab && index == saved_space.shown) {
      window.SetSelectedIn(space->id(), tab);
    }
    return tab;
  };
  const int pinned = static_cast<int>(saved_space.pinned.size());
  std::vector<bc_id> pinned_ids(saved_space.pinned.size());
  std::vector<bc_id> today_ids(saved_space.today.size());
  // The folders holding the row being put back, outermost first.
  std::vector<bc_id> holders;
  const auto holder = [&](int depth) {
    holders.resize(std::min(static_cast<size_t>(depth), holders.size()));
    return holders.empty() ? 0 : holders.back();
  };
  size_t folders = 0;
  const auto restore_folders = [&](size_t above) {
    for (; folders < saved_space.folders.size() && saved_space.folders[folders].at <= above;
         ++folders) {
      const SavedFolder& saved_folder = saved_space.folders[folders];
      const bc_id folder = space->AddFolder(saved_folder.name, holder(saved_folder.depth),
                                            space->pinned_rows().size());
      space->SetOpen(folder, saved_folder.open);
      holders.push_back(folder);
    }
  };
  for (int j = 0; j < pinned; ++j) {
    restore_folders(j);
    const SavedTab& saved_tab = saved_space.pinned[j];
    if (bc_id tab = pinned_ids[j] = restore(saved_tab, j)) {
      // Saved before pinned tabs kept their pinned URL, it's pinned where it is.
      space->AddPinned(tab, holder(saved_tab.depth), space->pinned_rows().size(),
                       saved_tab.pinned_url.is_valid()
                           ? saved_tab.pinned_url
                           : saved_tab.history[saved_tab.current].virtual_url());
    }
  }
  restore_folders(saved_space.pinned.size());
  // Bottom up, as each goes at the top.
  for (size_t j = saved_space.today.size(); j-- > 0;) {
    if (bc_id tab = today_ids[j] = restore(saved_space.today[j], pinned + static_cast<int>(j))) {
      space->Add(tab, /*opener=*/0, /*foreground=*/false);
    }
  }
  for (auto [tabs, ids] : {std::pair(&saved_space.pinned, &pinned_ids),
                           std::pair(&saved_space.today, &today_ids)}) {
    // The shares of the split being remade, which it takes once its last tab is in it.
    std::vector<double> shares;
    for (size_t j = 0; j < ids->size(); ++j) {
      const SavedTab& tab = (*tabs)[j];
      if (!tab.split || j == 0) {
        shares.clear();
      } else if ((*ids)[j] && (*ids)[j - 1] && space->Split((*ids)[j], (*ids)[j - 1], GURL()) &&
                 tab.stacked) {
        space->SetStacked((*ids)[j], true);
      }
      if ((*ids)[j]) {
        shares.push_back(tab.share);
      }
      if (tab.split && (*ids)[j] && (j + 1 == ids->size() || !(*tabs)[j + 1].split)) {
        space->SetShares((*ids)[j], shares);
      }
    }
  }
  return space;
}

bool RestoreSession(Window& window) {
  std::optional<Saved> saved = g_saver ? g_saver->TakeLoaded() : std::nullopt;
  if (!saved || !StartProfile() || !Space::All().empty()) {
    return false;
  }
  // First, as a Space may show one. Each in its profile, which loaded with the session, else in the
  // one the core started with.
  std::vector<bc_id> favorites;
  for (const SavedTab& saved_tab : saved->favorites) {
    const bc_id tab = favorites.emplace_back(
        RestoreSavedTab(window, ProfileFor(saved_tab.profile), saved_tab));
    if (tab) {
      Favorites::Add(tab,
                     saved_tab.pinned_url.is_valid()
                         ? saved_tab.pinned_url
                         : saved_tab.history[saved_tab.current].virtual_url(),
                     Favorites::All().size());
    }
  }
  bc_id shown = 0;
  for (size_t i = 0; i < saved->spaces.size(); ++i) {
    const SavedSpace& saved_space = saved->spaces[i];
    Space* space = RestoreSpace(window, ProfileFor(saved_space.profile), saved_space);
    if (static_cast<int>(i) == saved->shown) {
      shown = space->id();
    }
    if (saved_space.shown_favorite >= 0 && favorites[saved_space.shown_favorite]) {
      window.SetSelectedIn(space->id(), favorites[saved_space.shown_favorite]);
    }
  }
  window.ShowSpace(shown);
  return true;
}

}  // namespace browser_core
