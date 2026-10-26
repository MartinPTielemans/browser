#include "browser_core/page_motion.h"

#include "browser_core/platform/window_views.h"
#include "chrome/browser/devtools/devtools_window.h"
#include "content/public/browser/web_contents.h"

namespace browser_core {

size_t ShowPageMoving(content::WebContents* contents, bool moving) {
  // Chromium shows a page's last frame at its old size in the corner until the page draws at the
  // new one (issue #166). Stretched, the edge the app doesn't move stays still, as the sidebar's
  // motion needs; a resize the user or the page makes keeps Chrome's way.
  const bool stretch = moving && !contents->IsFullscreen();
  size_t views = 0;
  for (content::WebContents* each :
       {contents, DevToolsWindow::GetInTabWebContents(contents, nullptr)}) {
    if (content::RenderWidgetHostView* view = each ? each->GetRenderWidgetHostView() : nullptr) {
      platform::StretchOnResize(view, stretch);
      ++views;
    }
  }
  return views;
}

}  // namespace browser_core
