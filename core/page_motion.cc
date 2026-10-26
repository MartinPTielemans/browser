#include "browser_core/page_motion.h"

#include <utility>

#include "base/functional/bind.h"
#include "base/functional/callback.h"
#include "base/memory/weak_ptr.h"
#include "browser_core/platform/window_views.h"
#include "chrome/browser/devtools/devtools_window.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"

namespace browser_core {

namespace {

// The views of |contents|: its page's, and its DevTools' when they're docked.
std::vector<content::RenderWidgetHostView*> Views(content::WebContents* contents) {
  std::vector<content::RenderWidgetHostView*> views;
  for (content::WebContents* each :
       {contents, DevToolsWindow::GetInTabWebContents(contents, nullptr)}) {
    if (content::RenderWidgetHostView* view = each ? each->GetRenderWidgetHostView() : nullptr) {
      views.push_back(view);
    }
  }
  return views;
}

bool Drawn(const std::vector<base::WeakPtr<content::WebContents>>& pages) {
  for (const base::WeakPtr<content::WebContents>& page : pages) {
    if (!page || page->IsFullscreen()) {
      continue;
    }
    for (content::RenderWidgetHostView* view : Views(page.get())) {
      if (view->IsShowing() && !platform::DrawnAtSize(view)) {
        return false;
      }
    }
  }
  return true;
}

}  // namespace

size_t ShowPageMoving(content::WebContents* contents, bool moving) {
  // Chromium shows a page's last frame at its old size in the corner until the page draws at the
  // new one (issue #166). Stretched, the edge the app doesn't move stays still, as the sidebar's
  // motion needs; a resize the user or the page makes keeps Chrome's way.
  const bool stretch = moving && !contents->IsFullscreen();
  const std::vector<content::RenderWidgetHostView*> views = Views(contents);
  for (content::RenderWidgetHostView* view : views) {
    platform::StretchOnResize(view, stretch);
  }
  return views.size();
}

void HoldUntilDrawn(uint64_t window,
                    const std::vector<content::WebContents*>& pages,
                    base::OnceCallback<void(bool late)> shown) {
  // The page's browser compositor draws a step or two after AppKit resizes its view, which shows
  // the frame before at its old size in its top-left corner meanwhile: the right edge and the
  // scrollbar would jump with the left edge (issue #166). The renderer may still be late, inside
  // that frame; stretching covers it.
  std::vector<base::WeakPtr<content::WebContents>> weak;
  for (content::WebContents* page : pages) {
    weak.push_back(page->GetWeakPtr());
  }
  platform::HoldCommit(window, base::BindRepeating(&Drawn, std::move(weak)), std::move(shown));
}

}  // namespace browser_core
