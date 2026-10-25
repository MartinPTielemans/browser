#include "browser_core/favorites.h"

#include <map>
#include <utility>

#include "base/functional/bind.h"
#include "base/location.h"
#include "base/no_destructor.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"
#include "browser_core/session.h"

namespace browser_core {

namespace {

struct State {
  std::vector<bc_id> tabs;
  std::map<bc_id, GURL> urls;
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

}  // namespace

// static
const std::vector<bc_id>& Favorites::All() {
  return Get().tabs;
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
void Favorites::Add(bc_id tab, GURL url) {
  State& state = Get();
  if (state.urls.contains(tab)) {
    return;
  }
  state.tabs.push_back(tab);
  state.urls[tab] = std::move(url);
  ReportChanged();
}

// static
void Favorites::Remove(bc_id tab) {
  State& state = Get();
  if (state.urls.erase(tab)) {
    std::erase(state.tabs, tab);
    ReportChanged();
  }
}

}  // namespace browser_core
