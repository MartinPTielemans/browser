#ifndef BROWSER_CORE_AUTO_ARCHIVE_H_
#define BROWSER_CORE_AUTO_ARCHIVE_H_

namespace browser_core {

// Arc's auto-archive: a Today tab no window has shown for 12 hours, or the time the user chose in
// Settings, closes into the archive, but for one playing sound. One timer waits for the tab unseen
// longest, on the wall clock, so it keeps time through sleep and a quit (the Spaces keep when each
// tab was last seen).
//
// Looks again for that tab, from a task of its own, as the Spaces, the tabs windows show or the
// time chosen change.
void ScheduleAutoArchive();

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTO_ARCHIVE_H_
