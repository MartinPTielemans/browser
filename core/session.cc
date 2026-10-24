#include "browser_core/session.h"

#include <algorithm>
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
#include "base/values.h"
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
  // Each top to bottom.
  std::vector<SavedTab> pinned;
  std::vector<SavedTab> today;
  // The tab the window showed there, counting the pinned tabs then the Today tabs, or -1.
  int shown = -1;
};

struct Saved {
  std::vector<SavedSpace> spaces;
  // The Space the window showed.
  int shown = 0;
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
  for (Space* space : Space::All()) {
    if (window && space->id() == window->space()) {
      saved.shown = saved.spaces.size();
    }
    SavedSpace& to = saved.spaces.emplace_back();
    to.name = space->name();
    const bc_id shown = window ? window->SelectedIn(space->id()) : 0;
    int index = 0;
    for (auto [ids, tabs] : {std::pair(&space->pinned(), &to.pinned),
                             std::pair(&space->today(), &to.today)}) {
      for (bc_id id : *ids) {
        content::WebContents* contents = Tab::FromId(id);
        SavedTab tab = contents ? SaveTab(contents->GetController()) : SavedTab();
        if (tab.history.empty()) {
          continue;
        }
        if (id == shown) {
          to.shown = index;
        }
        tab.pinned_url = space->PinnedUrl(id);
        tabs->push_back(std::move(tab));
        ++index;
      }
    }
  }
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
      list.Append(std::move(dict));
    }
    return list;
  };
  base::ListValue spaces;
  for (const SavedSpace& space : saved.spaces) {
    spaces.Append(base::DictValue()
                      .Set("name", space.name)
                      .Set("pinned", to_list(space.pinned))
                      .Set("today", to_list(space.today))
                      .Set("shown", space.shown));
  }
  return base::WriteJson(
      base::DictValue().Set("spaces", std::move(spaces)).Set("shown", saved.shown));
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
  for (const base::Value& value : *spaces) {
    const base::DictValue* space = value.GetIfDict();
    const std::string* name = space ? space->FindString("name") : nullptr;
    const base::ListValue* today = space ? space->FindList("today") : nullptr;
    if (!name || !today) {
      continue;
    }
    SavedSpace& to = saved.spaces.emplace_back();
    to.name = *name;
    const int shown = space->FindInt("shown").value_or(-1);
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

bool RestoreSession(Window& window) {
  std::optional<Saved> saved = g_saver ? g_saver->TakeLoaded() : std::nullopt;
  // Chrome made the profile it starts with before the core's calls work.
  Profile* profile = ProfileManager::GetLastUsedProfileIfLoaded();
  if (!saved || !profile || !Space::All().empty()) {
    return false;
  }
  bc_id shown = 0;
  for (size_t i = 0; i < saved->spaces.size(); ++i) {
    const SavedSpace& saved_space = saved->spaces[i];
    Space* space = Space::Create(saved_space.name);
    if (static_cast<int>(i) == saved->shown) {
      shown = space->id();
    }
    const auto restore = [&](const SavedTab& saved_tab, int index) -> bc_id {
      content::WebContents* contents =
          window.RestoreTab(profile, saved_tab.history, saved_tab.current);
      if (!contents) {
        return 0;
      }
      const bc_id tab = Tab::IdOf(contents);
      Tab::FromWebContents(contents)->Restored();
      if (index == saved_space.shown) {
        window.SetSelectedIn(space->id(), tab);
      }
      return tab;
    };
    const int pinned = static_cast<int>(saved_space.pinned.size());
    for (int j = 0; j < pinned; ++j) {
      const SavedTab& saved_tab = saved_space.pinned[j];
      if (bc_id tab = restore(saved_tab, j)) {
        // Saved before pinned tabs kept their pinned URL, it's pinned where it is.
        space->AddPinned(tab, saved_tab.pinned_url.is_valid()
                                  ? saved_tab.pinned_url
                                  : saved_tab.history[saved_tab.current].virtual_url());
      }
    }
    // Bottom up, as each goes at the top.
    for (size_t j = saved_space.today.size(); j-- > 0;) {
      if (bc_id tab = restore(saved_space.today[j], pinned + static_cast<int>(j))) {
        space->Add(tab, /*opener=*/0, /*foreground=*/false);
      }
    }
  }
  window.ShowSpace(shown);
  return true;
}

}  // namespace browser_core
