#include "browser_core/crash_reports.h"

#include <cstdio>

#include "base/command_line.h"
#include "base/files/file_path.h"
#include "base/functional/bind.h"
#include "base/immediate_crash.h"
#include "base/location.h"
#include "base/path_service.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "build/build_config.h"
#include "chrome/common/chrome_paths.h"
#include "chrome/common/chrome_switches.h"
#include "third_party/crashpad/crashpad/client/crashpad_info.h"

#if BUILDFLAG(IS_MAC)
#include <mach/mach.h>

#include "base/apple/scoped_mach_port.h"
#include "components/crash/core/app/crashpad.h"
#include "third_party/crashpad/crashpad/client/crashpad_client.h"
#endif

namespace browser_core::crash_reports {

namespace {

// Not Chrome's --crash-test, which crashes the browser before its first window.
constexpr char kCrashTest[] = "crash-after-start";

// Whether Crashpad's handler takes crashes yet. On the Mac the browser doesn't wait for it to start
// (chromium-crashpad-mac-async-start.patch): it takes them once it has the receive right of the
// port the browser's crashes go to.
bool HandlerReady() {
#if BUILDFLAG(IS_MAC)
  base::apple::ScopedMachSendRight port =
      crash_reporter::GetCrashpadClient().GetHandlerMachPort();
  mach_port_type_t type = 0;
  return port.is_valid() && mach_port_type(mach_task_self(), port.get(), &type) == KERN_SUCCESS &&
         !(type & MACH_PORT_TYPE_RECEIVE);
#else
  return true;
#endif
}

void CrashOnceReady() {
  if (!HandlerReady()) {
    // Only with the test's switch, so no other launch wakes for this.
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostDelayedTask(
        FROM_HERE, base::BindOnce(&CrashOnceReady), base::Milliseconds(50));
    return;
  }
  // check.sh waits for this line.
  fprintf(stderr, "browser: crash test\n");
  // A crash asked for is no news for macOS's crash reporter to show the user.
  crashpad::CrashpadInfo::GetCrashpadInfo()->set_system_crash_reporter_forwarding(
      crashpad::TriState::kDisabled);
  base::ImmediateCrash();
}

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
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(FROM_HERE,
                                                              base::BindOnce(&CrashOnceReady));
}

}  // namespace browser_core::crash_reports
