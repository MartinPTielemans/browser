#include "browser_core/crash_reports.h"

#include <cstdio>

#include "base/command_line.h"
#include "base/files/file_path.h"
#include "base/functional/bind.h"
#include "base/immediate_crash.h"
#include "base/location.h"
#include "base/path_service.h"
#include "base/task/single_thread_task_runner.h"
#include "chrome/common/chrome_paths.h"
#include "chrome/common/chrome_switches.h"
#include "third_party/crashpad/crashpad/client/crashpad_info.h"

namespace browser_core::crash_reports {

namespace {

constexpr char kCrashTest[] = "crash-test";

}  // namespace

void PlaceReports() {
  base::FilePath profiles =
      base::CommandLine::ForCurrentProcess()->GetSwitchValuePath(switches::kUserDataDir);
  if (!profiles.empty()) {
    base::PathService::Override(chrome::DIR_CRASH_DUMPS,
                                profiles.Append(FILE_PATH_LITERAL("Crashpad")));
  }
}

void CrashIfTesting() {
  if (!base::CommandLine::ForCurrentProcess()->HasSwitch(kCrashTest)) {
    return;
  }
  // Posted, so the app has done with being told it started.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE, base::BindOnce([] {
    // check.sh waits for this line.
    fprintf(stderr, "browser: crash test\n");
    // A crash asked for is no news for macOS's crash reporter to show the user.
    crashpad::CrashpadInfo::GetCrashpadInfo()->set_system_crash_reporter_forwarding(
        crashpad::TriState::kDisabled);
    base::ImmediateCrash();
  }));
}

}  // namespace browser_core::crash_reports
