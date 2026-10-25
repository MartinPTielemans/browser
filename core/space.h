#ifndef BROWSER_CORE_SPACE_H_
#define BROWSER_CORE_SPACE_H_

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

#include "base/functional/function_ref.h"
#include "browser_core/public/browser_core.h"
#include "url/gurl.h"

namespace browser_core {

// A tab container: its pinned tabs and its Today tabs, each top to bottom (decision 5). Spaces are
// process-wide, and a window shows one.
//
// Tabs in a split view show side by side, and sit together in their list, left to right, as one
// row. Each of the Space's calls that moves a tab moves its split with it.
class Space {
 public:
  // The most tabs a split shows.
  static constexpr size_t kMaxPanes = 3;

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
  // 0xRRGGBB in sRGB, if it has one.
  std::optional<uint32_t> color() const { return color_; }
  const std::vector<bc_id>& pinned() const { return pinned_; }
  const std::vector<bc_id>& today() const { return today_; }
  bool Has(bc_id tab) const;
  bool IsPinned(bc_id tab) const;
  // Where |tab| was when it was pinned, which Back to Pinned URL goes to. Empty if it isn't pinned.
  GURL PinnedUrl(bc_id tab) const;
  // The tabs shown with |tab|, left to right, |tab| among them: its split's, else |tab| alone.
  // Empty if the Space hasn't |tab|.
  std::vector<bc_id> Panes(bc_id tab) const;

  void SetName(std::string name);
  void SetColor(std::optional<uint32_t> color);

  // Puts |tab| in Today. One the user opened (|opener| 0) goes at the top, under "+ New Tab". One a
  // page in this Space opened goes in Chrome's place: right under its opener if it opened in front,
  // or under the tabs its opener already opened behind it. A pinned opener's tabs go at the top.
  void Add(bc_id tab, bc_id opener, bool foreground);
  // Puts |tab| at |index| in Today, else at the bottom, and below a split |index| is inside.
  void AddAt(bc_id tab, size_t index);
  // Puts |tab| at |index| in the pinned tabs, else at the bottom, and below a split |index| is
  // inside, pinned at |url|.
  void AddPinned(bc_id tab, size_t index, GURL url);
  // Pins |tab|, a Today tab, at the bottom of the pinned tabs, each of its split at its |url_of|.
  void Pin(bc_id tab, base::FunctionRef<GURL(bc_id)> url_of);
  // Unpins |tab| to the top of Today.
  void Unpin(bc_id tab);
  // Moves |tab| to |index| in the pinned tabs or in Today, without it, else to the bottom. A Today
  // tab moved to the pinned tabs is pinned at its |url_of|, and a pinned tab moved to Today is
  // unpinned.
  void Move(bc_id tab, bool pinned, size_t index, base::FunctionRef<GURL(bc_id)> url_of);
  // Takes |tab| out of the Space, and out of its split.
  void Remove(bc_id tab);

  // Puts |tab| in a split view with |with|, at its right end, making one if |with| is in none.
  // |tab| leaves any split it was in, and goes after the split in its list, pinned at |url| if
  // that's the pinned tabs and it wasn't pinned. False, doing nothing, if |with|'s split is full,
  // or either tab isn't the Space's.
  bool Split(bc_id tab, bc_id with, GURL url);
  // Takes |tab| out of its split, to just below the tabs left in it. The split goes once one is.
  void Separate(bc_id tab);

  // The lowest pinned tab still |loaded|, else 0: the one to show once no Today tab is left.
  bc_id LastLoadedPinned(base::FunctionRef<bool(bc_id)> loaded) const;
  // The tab to show once |leaving| closes or, pinned, unloads. For a Today tab it's Chrome's order:
  // a tab it opened, then another tab its opener opened, then its opener, then the tab below it,
  // then the one above, and after the last Today tab the lowest pinned tab still |loaded|. For a
  // pinned tab it's the top Today tab, else the nearest pinned tab still |loaded|, below it first.
  // In a split it's the tab beside it, the one on its right first. For a tab not in the Space, a
  // favorite, it's the top Today tab, else the lowest pinned tab still |loaded|. 0 if there's none.
  bc_id NextAfterLeaving(bc_id leaving, base::FunctionRef<bool(bc_id)> loaded) const;

 private:
  Space(bc_id id, std::string name);

  bc_id OpenerOf(bc_id tab) const;
  // Makes the tabs |tab| opened its opener's, and |tab| no one's, as Chrome does when a tab moves.
  void ForgetOpener(bc_id tab);
  // The first tab below |index| opened by |opener|, else the nearest above it. 0 if none.
  bc_id NearestOpenedBy(bc_id opener, size_t index) const;
  // The split holding |tab|, or splits_.end().
  std::vector<std::vector<bc_id>>::iterator SplitOf(bc_id tab);
  std::vector<std::vector<bc_id>>::const_iterator SplitOf(bc_id tab) const;
  // |index| in |tabs|, or the index past the split it's inside, so a tab put there isn't in one.
  size_t OutsideSplits(const std::vector<bc_id>& tabs, size_t index) const;
  // Takes |tab| out of its split, leaving it where it is in its list.
  void LeaveSplit(bc_id tab);
  void ReportChanged();

  const bc_id id_;
  std::string name_;
  std::optional<uint32_t> color_;
  std::vector<bc_id> pinned_;
  std::map<bc_id, GURL> pinned_urls_;
  std::vector<bc_id> today_;
  // Each tab a page opened, to the tab it came from.
  std::map<bc_id, bc_id> openers_;
  // Each split's tabs, left to right, as they are in their list.
  std::vector<std::vector<bc_id>> splits_;
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SPACE_H_
