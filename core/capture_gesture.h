#ifndef BROWSER_CORE_CAPTURE_GESTURE_H_
#define BROWSER_CORE_CAPTURE_GESTURE_H_

#include <optional>

#include "base/memory/raw_ptr.h"
#include "base/memory/weak_ptr.h"
#include "content/public/browser/render_widget_host.h"
#include "content/public/browser/render_widget_host_observer.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"
#include "third_party/blink/public/common/input/web_mouse_event.h"

namespace browser_core {

// ⇧⌘-drag on a tab's page, ⇧⌃ off the Mac, which captures the rectangle dragged at once
// (docs/ux-spec.md › Capture). The core sees the page's mouse events before the page does: a
// press with those keys is held back, and once it moves past the drag distance the page gets none
// of the drag, which goes to the app as tab_capture_drag. A press let go first is given to the
// page after all, so ⇧⌘-click on a link still opens it in a tab. It watches the main frame's
// widget, so a drag that starts over a frame of another site's is that frame's.
class CaptureGesture : public content::WebContentsObserver,
                       public content::WebContentsUserData<CaptureGesture>,
                       public content::RenderWidgetHostObserver {
 public:
  CaptureGesture(const CaptureGesture&) = delete;
  CaptureGesture& operator=(const CaptureGesture&) = delete;
  ~CaptureGesture() override;

 private:
  friend class content::WebContentsUserData<CaptureGesture>;

  explicit CaptureGesture(content::WebContents* contents);

  // Watches |widget|'s mouse events, and no other widget's.
  void Watch(content::RenderWidgetHost* widget);
  void Unwatch();
  // Whether the page doesn't get |event|.
  bool Mouse(const blink::WebMouseEvent& event);

  // content::WebContentsObserver:
  void RenderFrameHostChanged(content::RenderFrameHost* old_host,
                              content::RenderFrameHost* new_host) override;
  void WebContentsDestroyed() override;

  // content::RenderWidgetHostObserver:
  void RenderWidgetHostDestroyed(content::RenderWidgetHost* widget) override;

  raw_ptr<content::RenderWidgetHost> widget_ = nullptr;
  content::RenderWidgetHost::MouseEventCallback callback_;
  // The press held back, until it's a drag or let go.
  std::optional<blink::WebMouseEvent> press_;
  // A drag is going to the app.
  bool dragging_ = false;
  // The held press is on its way to the page, past Mouse.
  bool replaying_ = false;
  base::WeakPtrFactory<CaptureGesture> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_CAPTURE_GESTURE_H_
