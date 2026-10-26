#ifndef BROWSER_CORE_PAGE_MOTION_H_
#define BROWSER_CORE_PAGE_MOTION_H_

#include <cstddef>

namespace content {
class WebContents;
}

namespace browser_core {

// Has the views of |contents|, its page's and its docked DevTools', stretch the frame they last
// drew to fill them as they resize while the app moves the page frame on its own, as the sidebar
// hides or shows, or not once it stops. A page full screen doesn't: the window resizes it. Returns
// how many views it told.
size_t ShowPageMoving(content::WebContents* contents, bool moving);

}  // namespace browser_core

#endif  // BROWSER_CORE_PAGE_MOTION_H_
