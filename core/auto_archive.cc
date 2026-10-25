#include "browser_core/auto_archive.h"

#include <algorithm>
#include <optional>
#include <string>
#include <vector>

#include "base/command_line.h"
#include "base/functional/bind.h"
#include "base/functional/function_ref.h"
#include "base/no_destructor.h"
#include "base/strings/string_number_conversions.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "base/timer/wall_clock_timer.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/lifetime/browser_shutdown.h"
#include "content/public/browser/web_contents.h"

namespace browser_core {

namespace {

// Arc's default. Its other intervals, in its settings, aren't offered yet.
constexpr base::TimeDelta kArchiveAfter = base::Hours(12);

// For the app's check, which can't wait 12 hours.
constexpr char kArchiveAfterSwitch[] = "archive-tabs-after";

base::TimeDelta ArchiveAfter() {
  static const base::TimeDelta after = [] {
    int seconds = 0;
    const std::string value =
        base::CommandLine::ForCurrentProcess()->GetSwitchValueASCII(kArchiveAfterSwitch);
    return base::StringToInt(value, &seconds) && seconds > 0 ? base::Seconds(seconds)
                                                             : kArchiveAfter;
  }();
  return after;
}

base::WallClockTimer& Timer() {
  static base::NoDestructor<base::WallClockTimer> timer;
  return *timer;
}

bool g_schedule_posted = false;

// Each Today tab no window shows.
void ForEachUnseen(base::FunctionRef<void(bc_id, content::WebContents*, Tab*)> each) {
  for (Space* space : Space::All()) {
    // A copy, as closing a tab takes it out.
    for (bc_id id : std::vector<bc_id>(space->today())) {
      content::WebContents* contents = Tab::FromId(id);
      if (contents && !Window::Showing(id)) {
        each(id, contents, Tab::FromWebContents(contents));
      }
    }
  }
}

void Schedule();

void ArchiveDue() {
  if (browser_shutdown::IsTryingToQuit()) {
    return;
  }
  const base::Time now = base::Time::Now();
  std::vector<bc_id> due;
  ForEachUnseen([&](bc_id id, content::WebContents* contents, Tab* tab) {
    if (tab->last_seen() + ArchiveAfter() > now) {
      return;
    }
    // Seen now, so one playing sound, or one its page keeps open, waits another interval.
    tab->set_last_seen(now);
    if (!contents->IsCurrentlyAudible()) {
      due.push_back(id);
    }
  });
  for (bc_id id : due) {
    Window::CloseTab(id);
  }
  Schedule();
}

void Schedule() {
  g_schedule_posted = false;
  std::optional<base::Time> next;
  ForEachUnseen([&](bc_id, content::WebContents*, Tab* tab) {
    const base::Time due = tab->last_seen() + ArchiveAfter();
    next = next ? std::min(*next, due) : due;
  });
  if (next) {
    Timer().Start(FROM_HERE, *next, base::BindOnce(&ArchiveDue));
  } else {
    Timer().Stop();
  }
}

}  // namespace

void ScheduleAutoArchive() {
  if (g_schedule_posted) {
    return;
  }
  g_schedule_posted = true;
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE, base::BindOnce(&Schedule));
}

}  // namespace browser_core
