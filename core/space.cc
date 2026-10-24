#include "browser_core/space.h"

#include <algorithm>
#include <map>
#include <memory>
#include <set>
#include <utility>

#include "base/functional/bind.h"
#include "base/location.h"
#include "base/memory/ptr_util.h"
#include "base/no_destructor.h"
#include "base/task/single_thread_task_runner.h"
#include "browser_core/host.h"

namespace browser_core {

namespace {

// By id, which is the order they were made in.
std::map<bc_id, std::unique_ptr<Space>>& Spaces() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Space>>> spaces;
  return *spaces;
}

}  // namespace

// static
Space* Space::Create(std::string name) {
  const bc_id id = NewId();
  Space* space = new Space(id, std::move(name));
  Spaces()[id] = base::WrapUnique(space);
  // Posted, like the Space's changes.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce([] {
        if (Host().spaces_changed) {
          Host().spaces_changed(Host().context);
        }
      }));
  return space;
}

// static
Space* Space::FromId(bc_id id) {
  auto it = Spaces().find(id);
  return it == Spaces().end() ? nullptr : it->second.get();
}

// static
std::vector<Space*> Space::All() {
  std::vector<Space*> all;
  for (auto& [id, space] : Spaces()) {
    all.push_back(space.get());
  }
  return all;
}

// static
Space* Space::Holding(bc_id tab) {
  for (auto& [id, space] : Spaces()) {
    if (std::ranges::contains(space->today_, tab)) {
      return space.get();
    }
  }
  return nullptr;
}

Space::Space(bc_id id, std::string name) : id_(id), name_(std::move(name)) {}

void Space::Add(bc_id tab, bc_id opener, bool foreground) {
  auto at = std::ranges::find(today_, opener);
  if (!opener || at == today_.end()) {
    today_.insert(today_.begin(), tab);
    ReportChanged();
    return;
  }
  openers_[tab] = opener;
  ++at;
  if (!foreground) {
    // Past the opener's family below it: the tabs it opened, and the tabs they opened.
    std::set<bc_id> family = {opener};
    for (; at != today_.end() && family.contains(OpenerOf(*at)); ++at) {
      family.insert(*at);
    }
  }
  today_.insert(at, tab);
  ReportChanged();
}

void Space::Remove(bc_id tab) {
  if (std::erase(today_, tab) == 0) {
    return;
  }
  // The tabs it opened count as its opener's from now on, as in Chrome.
  const bc_id opener = OpenerOf(tab);
  openers_.erase(tab);
  for (auto& [child, its_opener] : openers_) {
    if (its_opener == tab) {
      its_opener = opener;
    }
  }
  std::erase_if(openers_, [](const auto& entry) { return entry.second == 0; });
  ReportChanged();
}

bc_id Space::NextAfterClosing(bc_id closing) const {
  auto it = std::ranges::find(today_, closing);
  if (it == today_.end()) {
    return 0;
  }
  const size_t index = it - today_.begin();
  if (bc_id child = NearestOpenedBy(closing, index)) {
    return child;
  }
  if (bc_id opener = OpenerOf(closing)) {
    if (bc_id sibling = NearestOpenedBy(opener, index)) {
      return sibling;
    }
    if (std::ranges::contains(today_, opener)) {
      return opener;
    }
  }
  if (index + 1 < today_.size()) {
    return today_[index + 1];
  }
  return index > 0 ? today_[index - 1] : 0;
}

bc_id Space::OpenerOf(bc_id tab) const {
  auto it = openers_.find(tab);
  return it == openers_.end() ? 0 : it->second;
}

bc_id Space::NearestOpenedBy(bc_id opener, size_t index) const {
  for (size_t i = index + 1; i < today_.size(); ++i) {
    if (OpenerOf(today_[i]) == opener) {
      return today_[i];
    }
  }
  for (size_t i = index; i-- > 0;) {
    if (OpenerOf(today_[i]) == opener) {
      return today_[i];
    }
  }
  return 0;
}

void Space::ReportChanged() {
  // Posted: the app often changed the Space itself, and hears of it once its call has returned.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(
                     [](bc_id id) {
                       if (FromId(id) && Host().space_changed) {
                         Host().space_changed(Host().context, id);
                       }
                     },
                     id_));
}

}  // namespace browser_core
