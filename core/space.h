#ifndef BROWSER_CORE_SPACE_H_
#define BROWSER_CORE_SPACE_H_

#include <map>
#include <string>
#include <vector>

#include "browser_core/public/browser_core.h"

namespace browser_core {

// A tab container: its Today tabs, top to bottom (decision 5). Spaces are process-wide, and a window
// shows one.
class Space {
 public:
  static Space* Create(std::string name);
  // Deletes the Space, which windows no longer show.
  static void Delete(bc_id id);
  static Space* FromId(bc_id id);
  // Every Space, in the order they were made.
  static std::vector<Space*> All();
  // The Space holding |tab|, if any.
  static Space* Holding(bc_id tab);

  Space(const Space&) = delete;
  Space& operator=(const Space&) = delete;

  bc_id id() const { return id_; }
  const std::string& name() const { return name_; }
  const std::vector<bc_id>& today() const { return today_; }

  void SetName(std::string name);

  // Puts |tab| in Today. One the user opened (|opener| 0) goes at the top, under "+ New Tab". One a
  // page in this Space opened goes in Chrome's place: right under its opener if it opened in front,
  // or under the tabs its opener already opened behind it.
  void Add(bc_id tab, bc_id opener, bool foreground);
  void Remove(bc_id tab);

  // The tab to show once |closing| closes, in Chrome's order: a tab it opened, then another tab its
  // opener opened, then its opener, then the tab below it, then the one above. 0 if it's the last.
  bc_id NextAfterClosing(bc_id closing) const;

 private:
  Space(bc_id id, std::string name);

  bc_id OpenerOf(bc_id tab) const;
  // The first tab below |index| opened by |opener|, else the nearest above it. 0 if none.
  bc_id NearestOpenedBy(bc_id opener, size_t index) const;
  void ReportChanged();

  const bc_id id_;
  std::string name_;
  std::vector<bc_id> today_;
  // Each tab a page opened, to the tab it came from.
  std::map<bc_id, bc_id> openers_;
};

}  // namespace browser_core

#endif  // BROWSER_CORE_SPACE_H_
