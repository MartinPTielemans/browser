#ifndef BROWSER_CORE_AUTO_ARCHIVE_H_
#define BROWSER_CORE_AUTO_ARCHIVE_H_

namespace browser_core {

// Arc's auto-archive: a Today tab no window has shown for 12 hours, or the time the user chose in
// Settings, closes into the archive, but for one playing sound, and a download that ended that
// long ago leaves the Library's list, its file staying. One timer waits for whichever is due
// first, on the wall clock, so it keeps time through sleep and a quit (the Spaces keep when each
// tab was last seen, and Chrome's history when each download ended).
//
// Looks again for that tab or download, from a task of its own, as the Spaces, the tabs windows
// show, the downloads or the time chosen change.
void ScheduleAutoArchive();

// The note on the first tabs auto-archive takes: shown from then until the user dismisses it, and
// never again after that (bc_archive_note).
bool ArchiveNoteShown();
void DismissArchiveNote();

}  // namespace browser_core

#endif  // BROWSER_CORE_AUTO_ARCHIVE_H_
