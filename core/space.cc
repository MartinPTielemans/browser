#include "browser_core/space.h"

#include <algorithm>
#include <iterator>
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
#include "browser_core/session.h"

namespace browser_core {

namespace {

// By id, which is the order they were made in.
std::map<bc_id, std::unique_ptr<Space>>& Spaces() {
  static base::NoDestructor<std::map<bc_id, std::unique_ptr<Space>>> spaces;
  return *spaces;
}

// Posted, like a Space's changes.
void ReportSpacesChanged() {
  SessionChanged();
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce([] {
        if (Host().spaces_changed) {
          Host().spaces_changed(Host().context);
        }
      }));
}

}  // namespace

// static
Space* Space::Create(std::string name) {
  const bc_id id = NewId();
  Space* space = new Space(id, std::move(name));
  Spaces()[id] = base::WrapUnique(space);
  ReportSpacesChanged();
  return space;
}

// static
void Space::Delete(bc_id id) {
  if (Spaces().erase(id)) {
    ReportSpacesChanged();
  }
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
    if (space->Has(tab)) {
      return space.get();
    }
  }
  return nullptr;
}

Space::Space(bc_id id, std::string name) : id_(id), name_(std::move(name)) {}

bool Space::Has(bc_id tab) const {
  return IsPinned(tab) || std::ranges::contains(today_, tab);
}

bool Space::IsPinned(bc_id tab) const {
  return std::ranges::contains(pinned_, tab);
}

GURL Space::PinnedUrl(bc_id tab) const {
  auto it = pinned_urls_.find(tab);
  return it == pinned_urls_.end() ? GURL() : it->second;
}

void Space::SetName(std::string name) {
  name_ = std::move(name);
  ReportSpacesChanged();
}

void Space::Add(bc_id tab, bc_id opener, bool foreground) {
  const bool pinned_opener = opener && IsPinned(opener);
  // A pinned opener sits above the Today tabs.
  auto at = pinned_opener ? today_.begin() : std::ranges::find(today_, opener);
  if (!opener || (!pinned_opener && at == today_.end())) {
    today_.insert(today_.begin(), tab);
    ReportChanged();
    return;
  }
  openers_[tab] = opener;
  if (!pinned_opener) {
    ++at;
  }
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

void Space::AddPinned(bc_id tab, GURL url) {
  pinned_.push_back(tab);
  pinned_urls_[tab] = std::move(url);
  ReportChanged();
}

void Space::Pin(bc_id tab, GURL url) {
  if (std::erase(today_, tab) == 0) {
    return;
  }
  AddPinned(tab, std::move(url));
}

void Space::Unpin(bc_id tab) {
  if (std::erase(pinned_, tab) == 0) {
    return;
  }
  pinned_urls_.erase(tab);
  // Where a tab the user opens goes.
  today_.insert(today_.begin(), tab);
  ReportChanged();
}

void Space::Remove(bc_id tab) {
  if (std::erase(today_, tab) == 0 && std::erase(pinned_, tab) == 0) {
    return;
  }
  pinned_urls_.erase(tab);
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

bc_id Space::NextAfterLeaving(bc_id leaving, base::FunctionRef<bool(bc_id)> loaded) const {
  if (auto pinned = std::ranges::find(pinned_, leaving); pinned != pinned_.end()) {
    // As Dia: the top Today tab, else a pinned tab already loaded, so closing one loads no other.
    if (!today_.empty()) {
      return today_.front();
    }
    auto below = std::find_if(pinned + 1, pinned_.end(), loaded);
    if (below != pinned_.end()) {
      return *below;
    }
    auto above = std::find_if(std::make_reverse_iterator(pinned), pinned_.rend(), loaded);
    return above != pinned_.rend() ? *above : 0;
  }
  auto it = std::ranges::find(today_, leaving);
  if (it == today_.end()) {
    return 0;
  }
  const size_t index = it - today_.begin();
  if (bc_id child = NearestOpenedBy(leaving, index)) {
    return child;
  }
  if (bc_id opener = OpenerOf(leaving)) {
    if (bc_id sibling = NearestOpenedBy(opener, index)) {
      return sibling;
    }
    if (Has(opener)) {
      return opener;
    }
  }
  if (index + 1 < today_.size()) {
    return today_[index + 1];
  }
  if (index > 0) {
    return today_[index - 1];
  }
  auto pinned = std::find_if(pinned_.rbegin(), pinned_.rend(), loaded);
  return pinned != pinned_.rend() ? *pinned : 0;
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
  SessionChanged();
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
