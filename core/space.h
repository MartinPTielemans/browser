#ifndef BROWSER_CORE_SPACE_H_
#define BROWSER_CORE_SPACE_H_

#include <map>
#include <string>
#include <vector>

#include "base/functional/function_ref.h"
#include "browser_core/public/browser_core.h"
#include "url/gurl.h"

namespace browser_core {

// A tab container: its pinned tabs and its Today tabs, each top to bottom (decision 5). Spaces are
// process-wide, and a window shows one.
class Space {
 public:
  static Space* Create(std::string name);
  // Deletes the Space, which windows no longer show.
  static void Delete(bc_id id);
  static Space* FromId(bc_id id);
  // Every Space, in the order they were made.
  static std::vector<Space*> All();
  // The Space holding |tab|, pinned or in Today, if any.
  static Space* Holding(bc_id tab);

  Space(const Space&) = delete;
  Space& operator=(const Space&) = delete;

  bc_id id() const { return id_; }
  const std::string& name() const { return name_; }
  const std::vector<bc_id>& pinned() const { return pinned_; }
  const std::vector<bc_id>& today() const { return today_; }
  bool Has(bc_id tab) const;
  bool IsPinned(bc_id tab) const;
  // Where |tab| was when it was pinned, which Back to Pinned URL goes to. Empty if it isn't pinned.
  GURL PinnedUrl(bc_id tab) const;

  void SetName(std::string name);

  // Puts |tab| in Today. One the user opened (|opener| 0) goes at the top, under "+ New Tab". One a
  // page in this Space opened goes in Chrome's place: right under its opener if it opened in front,
  // or under the tabs its opener already opened behind it. A pinned opener's tabs go at the top.
  void Add(bc_id tab, bc_id opener, bool foreground);
  // Puts |tab| at |index| in Today, else at the bottom.
  void AddAt(bc_id tab, size_t index);
  // Puts |tab| at the bottom of the pinned tabs, pinned at |url|.
  void AddPinned(bc_id tab, GURL url);
  // Pins |tab|, a Today tab, at |url| at the bottom of the pinned tabs.
  void Pin(bc_id tab, GURL url);
  // Unpins |tab| to the top of Today.
  void Unpin(bc_id tab);
  void Remove(bc_id tab);

  // The lowest pinned tab still |loaded|, else 0: the one to show once no Today tab is left.
  bc_id LastLoadedPinned(base::FunctionRef<bool(bc_id)> loaded) const;
  // The tab to show once |leaving| closes or, pinned, unloads. For a Today tab it's Chrome's order:
  // a tab it opened, then another tab its opener opened, then its opener, then the tab below it,
  // then the one above, and after the last Today tab the lowest pinned tab still |loaded|. For a
  // pinned tab it's the top Today tab, else the nearest pinned tab still |loaded|, below it first.
  // 0 if there's none.
  bc_id NextAfterLeaving(bc_id leaving, base::FunctionRef<bool(bc_id)> loaded) const;

 private:
  Space(bc_id id, std::string name);

  bc_id OpenerOf(bc_id tab) const;
  // The first tab below |index| opened by |opener|, else the nearest above it. 0 if none.
  bc_id NearestOpenedBy(bc_id opener, size_t index) const;
  void ReportChanged();

  const bc_id id_;
  std::string name_;
  std::vector<bc_id> pinned_;
  std::map<bc_id, GURL> pinned_urls_;
  std::vector<bc_id> today_;
  // Each tab a page opened, to the tab it came from.
  std::map<bc_id, bc_id> openers_;
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SPACE_H_
