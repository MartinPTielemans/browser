#include "browser_core/session.h"

#include <algorithm>
#include <map>
#include <optional>
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

struct SavedSpace {
  std::string name;
  std::optional<uint32_t> color;
  // Each top to bottom.
  std::vector<SavedTab> pinned;
  std::vector<SavedTab> today;
  // The tab the window showed there, counting the pinned tabs then the Today tabs, or -1.
  int shown = -1;
  // The favorite the window showed there instead, or -1.
  int shown_favorite = -1;
};

struct Saved {
  // Left to right.
  std::vector<SavedTab> favorites;
  std::vector<SavedSpace> spaces;
  // The Space the window showed.
  int shown = 0;
  // The window's docked sidebar's width, or 0 for none.
  double sidebar_width = 0;
  std::vector<ArchivedTab> archive;
};

// A navigation's page state is left out past this, as in Chrome's sessions.
constexpr int kMaxNavigationSize = 63 * 1024;

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

namespace {

Saved Collect() {
  Saved saved;
  const std::vector<Window*> windows = Window::All();
  const Window* window = windows.empty() ? nullptr : windows.front();
  saved.sidebar_width = window ? window->sidebar_width() : 0;
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
    favorites[id] = saved.favorites.size();
    saved.favorites.push_back(std::move(tab));
  }
  for (Space* space : Space::All()) {
    if (window && space->id() == window->space()) {
      saved.shown = saved.spaces.size();
    }
    SavedSpace& to = saved.spaces.emplace_back();
    to.name = space->name();
    to.color = space->color();
    const bc_id shown = window ? window->SelectedIn(space->id()) : 0;
    if (auto favorite = favorites.find(shown); favorite != favorites.end()) {
      to.shown_favorite = favorite->second;
    }
    int index = 0;
    for (auto [ids, tabs] : {std::pair(&space->pinned(), &to.pinned),
                             std::pair(&space->today(), &to.today)}) {
      bc_id above = 0;
      for (bc_id id : *ids) {
        content::WebContents* contents = Tab::FromId(id);
        SavedTab tab = contents ? SaveTab(contents->GetController()) : SavedTab();
        if (tab.history.empty()) {
          continue;
        }
        if (id == shown) {
          to.shown = index;
        }
        tab.split = above && std::ranges::contains(space->Panes(id), above);
        above = id;
        tab.pinned_url = space->PinnedUrl(id);
        tab.name = Tab::FromWebContents(contents)->name();
        if (tabs == &to.today) {
          tab.last_seen = Window::Showing(id) ? base::Time::Now()
                                              : Tab::FromWebContents(contents)->last_seen();
        }
        tabs->push_back(std::move(tab));
        ++index;
      }
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
      list.Append(std::move(dict));
    }
    return list;
  };
  base::ListValue spaces;
  for (const SavedSpace& space : saved.spaces) {
    base::DictValue dict = base::DictValue()
                               .Set("name", space.name)
                               .Set("pinned", to_list(space.pinned))
                               .Set("today", to_list(space.today))
                               .Set("shown", space.shown);
    if (space.shown_favorite >= 0) {
      dict.Set("shown_favorite", space.shown_favorite);
    }
    if (space.color) {
      dict.Set("color", static_cast<int>(*space.color));
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
      for (const base::Value& value : *list) {
        std::optional<SavedTab> tab = ReadTab(value);
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
    }
    loaded_ = std::move(saved);
    std::move(loaded).Run();
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

double SavedSidebarWidth() {
  return g_saver && g_saver->loaded() ? g_saver->loaded()->sidebar_width : 0;
}

bool RestoreSession(Window& window) {
  std::optional<Saved> saved = g_saver ? g_saver->TakeLoaded() : std::nullopt;
  // Chrome made the profile it starts with before the core's calls work.
  Profile* profile = ProfileManager::GetLastUsedProfileIfLoaded();
  if (!saved || !profile || !Space::All().empty()) {
    return false;
  }
  const auto restore_tab = [&](const SavedTab& saved_tab) -> bc_id {
    content::WebContents* contents =
        window.RestoreTab(profile, saved_tab.history, saved_tab.current);
    if (!contents) {
      return 0;
    }
    // Which makes its Tab.
    const bc_id tab = Tab::IdOf(contents);
    Tab::FromWebContents(contents)->set_name(saved_tab.name);
    Tab::FromWebContents(contents)->Restored();
    if (!saved_tab.last_seen.is_null()) {
      Tab::FromWebContents(contents)->set_last_seen(saved_tab.last_seen);
    }
    return tab;
  };
  // First, as a Space may show one.
  std::vector<bc_id> favorites;
  for (const SavedTab& saved_tab : saved->favorites) {
    const bc_id tab = favorites.emplace_back(restore_tab(saved_tab));
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
    Space* space = Space::Create(saved_space.name);
    space->SetColor(saved_space.color);
    if (static_cast<int>(i) == saved->shown) {
      shown = space->id();
    }
    if (saved_space.shown_favorite >= 0 && favorites[saved_space.shown_favorite]) {
      window.SetSelectedIn(space->id(), favorites[saved_space.shown_favorite]);
    }
    const auto restore = [&](const SavedTab& saved_tab, int index) -> bc_id {
      const bc_id tab = restore_tab(saved_tab);
      if (tab && index == saved_space.shown) {
        window.SetSelectedIn(space->id(), tab);
      }
      return tab;
    };
    const int pinned = static_cast<int>(saved_space.pinned.size());
    std::vector<bc_id> pinned_ids(saved_space.pinned.size());
    std::vector<bc_id> today_ids(saved_space.today.size());
    for (int j = 0; j < pinned; ++j) {
      const SavedTab& saved_tab = saved_space.pinned[j];
      if (bc_id tab = pinned_ids[j] = restore(saved_tab, j)) {
        // Saved before pinned tabs kept their pinned URL, it's pinned where it is.
        space->AddPinned(tab, space->pinned().size(),
                         saved_tab.pinned_url.is_valid()
                             ? saved_tab.pinned_url
                             : saved_tab.history[saved_tab.current].virtual_url());
      }
    }
    // Bottom up, as each goes at the top.
    for (size_t j = saved_space.today.size(); j-- > 0;) {
      if (bc_id tab = today_ids[j] = restore(saved_space.today[j], pinned + static_cast<int>(j))) {
        space->Add(tab, /*opener=*/0, /*foreground=*/false);
      }
    }
    for (auto [tabs, ids] : {std::pair(&saved_space.pinned, &pinned_ids),
                             std::pair(&saved_space.today, &today_ids)}) {
      for (size_t j = 1; j < ids->size(); ++j) {
        if ((*tabs)[j].split && (*ids)[j] && (*ids)[j - 1]) {
          space->Split((*ids)[j], (*ids)[j - 1], GURL());
        }
      }
    }
  }
  window.ShowSpace(shown);
  return true;
}

}  // namespace browser_core
