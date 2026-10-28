#ifndef BROWSER_CORE_CRASH_REPORTS_H_
#define BROWSER_CORE_CRASH_REPORTS_H_

#include <string>

namespace browser_core::crash_reports {

// Crashpad writes a minidump of every crash, in the profile folder's Crashpad
// (engine/patches/ours/chromium-crashpad-start.patch), and its handler never sends one. The core
// sends them, only while the user has Send Crash Reports on (bc_set_send_crash_reports), to the
// collector the app names with --crash-upload-url: each report not yet sent, one at a time, once
// the browser has started and when the user turns it on, deleting each once the collector has it.
// One that doesn't go waits for the next launch: no timer retries it. Turning it off deletes every
// report not yet sent.

// In the browser process, before Chromium starts Crashpad: its reports go in the profile folder the
// app gave, not Chromium's default one, which another Chromium on the machine shares.
void PlaceReports();

// In each process, once Chromium has started Crashpad there: its reports leave out the memory that
// pointers on the stack lead to, which can be a page's, and Chrome's dumps of processes that didn't
// crash, which record URLs. The page's URL that Chrome would record, the core's content client
// leaves out (main_delegate.cc).
void LeaveOutPages();

// The collector's host, or empty with none, when nothing is sent and Settings doesn't offer to.
std::string CollectorHost();

// Whether the user has Send Crash Reports on. Turning it on sends the reports not yet sent, and off
// deletes them.
bool Sending();
void SetSending(bool send);

// Once the app has started: sends the reports not yet sent, if the user has said to, once the
// browser has finished starting. With --crash-after-start, crashes the browser instead, once
// Crashpad's handler is up, so the checks see it write its minidump (app/check/crash.sh). The prod
// app doesn't pass that switch on.
void Started();

}  // namespace browser_core::crash_reports

#endif  // BROWSER_CORE_CRASH_REPORTS_H_
