#ifndef BROWSER_CORE_CRASH_REPORTS_H_
#define BROWSER_CORE_CRASH_REPORTS_H_

namespace browser_core::crash_reports {

// Crashpad writes a minidump of every crash, in the profile folder's Crashpad
// (engine/patches/ours/chromium-crashpad-start.patch), and its handler never sends one.

// In the browser process, before Chromium starts Crashpad: its reports go in the profile folder the
// app gave, not Chromium's default one, which another Chromium on the machine shares.
void PlaceReports();

// Once the app has started, with --crash-test: the browser crashes, so the checks see Crashpad write
// its minidump (app/check/crash.sh). The prod app doesn't pass the switch on.
void CrashIfTesting();

}  // namespace browser_core::crash_reports

#endif  // BROWSER_CORE_CRASH_REPORTS_H_
