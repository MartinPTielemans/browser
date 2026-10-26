#include "browser_core/favorites.h"

#include <algorithm>
#include <iterator>
#include <map>
#include <string>
#include <utility>

#include "base/functional/bind.h"
#include "base/location.h"
#include "base/no_destructor.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/profiles.h"
#include "browser_core/session.h"
#include "browser_core/tab.h"

namespace browser_core {

namespace {

struct State {
  std::vector<bc_id> tabs;
  std::map<bc_id, GURL> urls;
  // Each one's profile's directory.
  std::map<bc_id, std::string> profiles;
};

State& Get() {
  static base::NoDestructor<State> state;
  return *state;
}

void ReportChanged() {
  SessionChanged();
  // Posted, like a Space's changes, so the app hears of its own call once it has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce([] {
        if (Host().favorites_changed) {
          Host().favorites_changed(Host().context);
        }
      }));
}

// Puts |tab| in |state.tabs| at |index| of its profile's favorites, else after the last of them.
void Insert(State& state, bc_id tab, size_t index) {
  const std::vector<bc_id> mine = Favorites::Of(state.profiles[tab]);
  auto at = index < mine.size() ? std::ranges::find(state.tabs, mine[index])
            : mine.empty()      ? state.tabs.end()
                                : std::ranges::find(state.tabs, mine.back()) + 1;
  state.tabs.insert(at, tab);
}

}  // namespace

// static
const std::vector<bc_id>& Favorites::All() {
  return Get().tabs;
}

// static
std::vector<bc_id> Favorites::Of(const std::string& profile) {
  std::vector<bc_id> tabs;
  std::ranges::copy_if(Get().tabs, std::back_inserter(tabs),
                       [&](bc_id tab) { return ProfileOf(tab) == profile; });
  return tabs;
}

// static
std::string Favorites::ProfileOf(bc_id tab) {
  auto it = Get().profiles.find(tab);
  return it == Get().profiles.end() ? std::string() : it->second;
}

// static
bool Favorites::Has(bc_id tab) {
  return Get().urls.contains(tab);
}

// static
GURL Favorites::Url(bc_id tab) {
  auto it = Get().urls.find(tab);
  return it == Get().urls.end() ? GURL() : it->second;
}

// static
void Favorites::Add(bc_id tab, GURL url, size_t index) {
  State& state = Get();
  if (state.urls.contains(tab)) {
    return;
  }
  state.profiles[tab] = DirOf(Tab::FromId(tab));
  Insert(state, tab, index);
  state.urls[tab] = std::move(url);
  ReportChanged();
}

// static
void Favorites::Move(bc_id tab, size_t index) {
  State& state = Get();
  if (!state.urls.contains(tab)) {
    return;
  }
  std::erase(state.tabs, tab);
  Insert(state, tab, index);
  ReportChanged();
}

// static
void Favorites::Remove(bc_id tab) {
  State& state = Get();
  if (state.urls.erase(tab)) {
    state.profiles.erase(tab);
    std::erase(state.tabs, tab);
    ReportChanged();
  }
}

}  // namespace browser_core
