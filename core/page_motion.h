#ifndef BROWSER_CORE_PAGE_MOTION_H_
#define BROWSER_CORE_PAGE_MOTION_H_

#include <cstddef>
#include <cstdint>
#include <vector>

#include "base/functional/callback_forward.h"

namespace content {
class WebContents;
}

namespace browser_core {

// Has the views of |contents|, its page's and its docked DevTools', stretch the frame the page last
// drew to fill them as they resize while the app moves the page frame on its own, as the sidebar
// hides or shows, or not once it stops. A page full screen doesn't: the window resizes it. Returns
// how many views it told.
size_t ShowPageMoving(content::WebContents* contents, bool moving);

// Holds the changes to the window |window| going to the screen now, the app's UI with them, until
// the views of |pages|, the pages it shows, and of their docked DevTools have each drawn a frame at
// their new size, for a frame or two at most. Then tells |shown| whether they were late. A page
// full screen doesn't count.
void HoldUntilDrawn(uint64_t window,
                    const std::vector<content::WebContents*>& pages,
                    base::OnceCallback<void(bool late)> shown);

}  // namespace browser_core

#endif  // BROWSER_CORE_PAGE_MOTION_H_
