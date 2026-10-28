#include "browser_core/history_import.h"

#include <algorithm>
#include <memory>
#include <utility>

#include "base/location.h"
#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "base/task/cancelable_task_tracker.h"
#include "base/time/time.h"
#include "browser_core/host.h"
#include "chrome/browser/history/history_service_factory.h"
#include "chrome/browser/profiles/profile.h"
#include "components/history/core/browser/history_backend.h"
#include "components/history/core/browser/history_database.h"
#include "components/history/core/browser/history_db_task.h"
#include "components/history/core/browser/history_service.h"
#include "components/history/core/browser/history_types.h"
#include "components/history/core/browser/url_row.h"
#include "components/keyed_service/core/service_access_type.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

// Runs on the history database's thread, where it can see what the profile has before adding.
class AddPages : public history::HistoryDBTask {
 public:
  AddPages(bc_id profile, history::URLRows rows) : profile_(profile), rows_(std::move(rows)) {}

  bool RunOnDBThread(history::HistoryBackend* backend, history::HistoryDatabase* db) override {
    if (!db) {
      return true;
    }
    history::URLRows add;
    for (history::URLRow& row : rows_) {
      history::URLRow had;
      history::URLID id = db->GetRowForURL(row.url(), &had);
      if (id && had.last_visit() >= row.last_visit()) {
        continue;
      }
      if (id) {
        // AddPagesWithDetails adds only the visit to a page it has, so the page's own last visit
        // moves here, which a second import of it then finds.
        had.set_last_visit(row.last_visit());
        had.set_visit_count(had.visit_count() + 1);
        db->UpdateURLRow(id, had);
      }
      add.push_back(std::move(row));
    }
    added_ = add.size();
    // Marked imported, as Chrome's importer marks another browser's pages.
    backend->AddPagesWithDetails(add, history::SOURCE_IE_IMPORTED);
    return true;
  }

  void DoneRunOnMainThread() override {
    if (Host().history_added) {
      Host().history_added(Host().context, profile_, added_);
    }
  }

 private:
  const bc_id profile_;
  history::URLRows rows_;
  size_t added_ = 0;
};

base::CancelableTaskTracker& Tracker() {
  static base::NoDestructor<base::CancelableTaskTracker> tracker;
  return *tracker;
}

}  // namespace

bool AddImportedHistory(Profile* profile, bc_id id, const bc_history_page* pages, size_t count) {
  if (!profile || profile->IsOffTheRecord()) {
    return false;
  }
  history::HistoryService* history =
      HistoryServiceFactory::GetForProfile(profile, ServiceAccessType::EXPLICIT_ACCESS);
  if (!history) {
    return false;
  }
  history::URLRows rows;
  rows.reserve(count);
  for (size_t i = 0; i < count; ++i) {
    const bc_history_page& page = pages[i];
    GURL url(page.url ? page.url : "");
    if (!url.is_valid() || page.last_visit <= 0) {
      continue;
    }
    history::URLRow row(url);
    row.set_title(base::UTF8ToUTF16(page.title ? page.title : ""));
    row.set_visit_count(std::max(page.visits, 1));
    row.set_typed_count(std::max(page.typed, 0));
    row.set_last_visit(base::Time::UnixEpoch() + base::Microseconds(page.last_visit));
    rows.push_back(std::move(row));
  }
  history->ScheduleDBTask(FROM_HERE, std::make_unique<AddPages>(id, std::move(rows)), &Tracker());
  return true;
}

}  // namespace browser_core
