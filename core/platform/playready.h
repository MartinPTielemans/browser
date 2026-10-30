#ifndef BROWSER_CORE_PLATFORM_PLAYREADY_H_
#define BROWSER_CORE_PLATFORM_PLAYREADY_H_

namespace browser_core::platform {

// What Chromium's PlayReady needs that this PC lacks, as far as the browser can tell before a page
// asks: a BC_PLAYREADY_ value (bc_playready_needs).
int PlayReadyNeeds();

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_PLAYREADY_H_
