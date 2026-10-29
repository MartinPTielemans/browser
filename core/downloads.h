#ifndef BROWSER_CORE_DOWNLOADS_H_
#define BROWSER_CORE_DOWNLOADS_H_

#include <optional>

#include "base/time/time.h"
#include "browser_core/public/browser_core.h"

class Profile;

namespace browser_core {

// Tells the app of |profile|'s downloads from now on, those Chrome's history put back from the
// last launch too, in one list with the other profiles', as Arc's Library lists them. Then
// downloads_changed.
void ObserveDownloads(Profile* profile);

// What bc_download_act does to the download |id|.
void ActOnDownload(bc_id id, int action);

// Takes the downloads not running or paused off the list, keeping their files.
void ClearDownloads();

// When the first download not running or paused goes off the list, |after| since it ended, as
// Arc archives downloads with its tabs. Nothing while none is listed.
std::optional<base::Time> NextOldDownload(base::TimeDelta after);

// Takes the downloads that ended |after| ago or longer off the list, keeping their files.
void RemoveOldDownloads(base::TimeDelta after);

// Finds the icon the system shows for the download's file. Then download_icon.
void FindDownloadIcon(bc_id id);

}  // namespace browser_core

#endif  // BROWSER_CORE_DOWNLOADS_H_
