#include "browser_core/archive.h"

#include <algorithm>
#include <utility>

#include "base/functional/bind.h"
#include "base/no_destructor.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/session.h"

namespace browser_core {

namespace {

// Ours: Arc's limits aren't known.
constexpr base::TimeDelta kKeepFor = base::Days(30);
constexpr size_t kMaxArchived = 500;

std::vector<ArchivedTab>& Tabs() {
  static base::NoDestructor<std::vector<ArchivedTab>> tabs;
  return *tabs;
}

// What one Delete or Clear took out, each tab with the place it had, in that order.
using Removal = std::vector<std::pair<size_t, ArchivedTab>>;

// Newest last, as many as the closed tabs.
constexpr size_t kMaxRemovals = 25;

std::vector<Removal>& Removals() {
  static base::NoDestructor<std::vector<Removal>> removals;
  return *removals;
}

bool g_report_posted = false;

void Report() {
  g_report_posted = false;
  if (!Host().archive_changed) {
    return;
  }
  std::vector<bc_archived_tab> tabs;
  tabs.reserve(Tabs().size());
  for (const ArchivedTab& tab : Tabs()) {
    tabs.push_back({.id = tab.id,
                    .url = tab.url.spec().c_str(),
                    .title = tab.title.c_str(),
                    .archived = tab.archived.InMillisecondsFSinceUnixEpochIgnoringNull()});
  }
  Host().archive_changed(Host().context, tabs.data(), tabs.size());
}

// Tells the app once, after the call that changed it has returned.
void PostReport() {
  if (g_report_posted) {
    return;
  }
  g_report_posted = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE, base::BindOnce(&Report));
}

// Saves it with the Spaces, and tells the app.
void Changed() {
  SessionChanged();
  PostReport();
}

void Prune() {
  const base::Time oldest = base::Time::Now() - kKeepFor;
  std::erase_if(Tabs(), [oldest](const ArchivedTab& tab) { return tab.archived < oldest; });
  if (Tabs().size() > kMaxArchived) {
    Tabs().resize(kMaxArchived);
  }
}

}  // namespace

const std::vector<ArchivedTab>& Archived() {
  return Tabs();
}

void LoadArchive(std::vector<ArchivedTab> archived) {
  Tabs() = std::move(archived);
  for (ArchivedTab& tab : Tabs()) {
    tab.id = NewId();
  }
  Prune();
  if (!Tabs().empty()) {
    PostReport();
  }
}

void Archive(bc_id tab, const GURL& url, const std::string& title) {
  Tabs().insert(Tabs().begin(), {.id = NewId(),
                                 .url = url,
                                 .title = title,
                                 .archived = base::Time::Now(),
                                 .tab = tab});
  Prune();
  Changed();
}

void Unarchive(bc_id tab) {
  if (std::erase_if(Tabs(), [tab](const ArchivedTab& archived) { return archived.tab == tab; })) {
    Changed();
  }
}

std::optional<ArchivedTab> TakeArchived(bc_id id) {
  auto it = std::ranges::find(Tabs(), id, &ArchivedTab::id);
  if (it == Tabs().end()) {
    return std::nullopt;
  }
  ArchivedTab tab = std::move(*it);
  Tabs().erase(it);
  Changed();
  return tab;
}

void ClearArchive() {
  if (!Tabs().empty()) {
    Tabs().clear();
    Changed();
  }
}

void RemoveArchived(bc_id id) {
  auto it = std::ranges::find(Tabs(), id, &ArchivedTab::id);
  if (it == Tabs().end()) {
    return;
  }
  Removal removal;
  removal.emplace_back(static_cast<size_t>(it - Tabs().begin()), std::move(*it));
  Tabs().erase(it);
  Removals().push_back(std::move(removal));
  if (Removals().size() > kMaxRemovals) {
    Removals().erase(Removals().begin());
  }
  Changed();
}

void RemoveAllArchived() {
  if (Tabs().empty()) {
    return;
  }
  Removal removal;
  for (size_t index = 0; index < Tabs().size(); ++index) {
    removal.emplace_back(index, std::move(Tabs()[index]));
  }
  Tabs().clear();
  Removals().push_back(std::move(removal));
  if (Removals().size() > kMaxRemovals) {
    Removals().erase(Removals().begin());
  }
  Changed();
}

size_t UndoRemoved() {
  if (Removals().empty()) {
    return 0;
  }
  Removal removal = std::move(Removals().back());
  Removals().pop_back();
  const size_t count = removal.size();
  // In order of place, so each lands where it was.
  for (auto& [index, tab] : removal) {
    Tabs().insert(Tabs().begin() + std::min(index, Tabs().size()), std::move(tab));
  }
  Changed();
  return count;
}

}  // namespace browser_core
