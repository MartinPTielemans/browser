#include "browser_core/capture_gesture.h"

#include "base/functional/bind.h"
#include "base/memory/weak_ptr.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "build/build_config.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "third_party/blink/public/common/input/web_input_event.h"
#include "ui/gfx/geometry/vector2d_f.h"

namespace browser_core {

namespace {

using blink::WebInputEvent;

// How far the pointer moves with the button down before a press is a drag, capture mode's (ours).
constexpr float kDrag = 4;

// ⇧⌘ on the Mac, ⇧⌃ off it, where the app's ⌘ is ⌃.
constexpr int kKeys = WebInputEvent::kShiftKey |
                      (BUILDFLAG(IS_MAC) ? WebInputEvent::kMetaKey : WebInputEvent::kControlKey);
constexpr int kAnyKeys = WebInputEvent::kShiftKey | WebInputEvent::kControlKey |
                         WebInputEvent::kAltKey | WebInputEvent::kMetaKey;

void Report(content::WebContents* contents, int phase, gfx::PointF at) {
  if (contents && Host().tab_capture_drag) {
    Host().tab_capture_drag(Host().context, Tab::IdOf(contents), phase, at.x(), at.y());
  }
}

}  // namespace

CaptureGesture::CaptureGesture(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<CaptureGesture>(*contents) {
  // A callback the widget can tell from another's, to take it back, which a weak pointer can't be
  // bound in: it returns a value.
  callback_ = base::BindRepeating(
      [](base::WeakPtr<CaptureGesture> gesture, const blink::WebMouseEvent& event) {
        return gesture && gesture->Mouse(event);
      },
      weak_factory_.GetWeakPtr());
  Watch(contents->GetPrimaryMainFrame()->GetRenderWidgetHost());
}

CaptureGesture::~CaptureGesture() {
  Unwatch();
}

void CaptureGesture::Watch(content::RenderWidgetHost* widget) {
  if (widget == widget_) {
    return;
  }
  Unwatch();
  if (!widget) {
    return;
  }
  widget_ = widget;
  widget_->AddMouseEventCallback(callback_);
  widget_->AddObserver(this);
}

void CaptureGesture::Unwatch() {
  if (!widget_) {
    return;
  }
  // A drag the page's new widget won't finish.
  if (dragging_) {
    Report(web_contents(), BC_CAPTURE_DRAG_ENDED, press_->PositionInWidget());
  }
  press_.reset();
  dragging_ = false;
  widget_->RemoveMouseEventCallback(callback_);
  widget_->RemoveObserver(this);
  widget_ = nullptr;
}

bool CaptureGesture::Mouse(const blink::WebMouseEvent& event) {
  if (replaying_) {
    return false;
  }
  const gfx::PointF at = event.PositionInWidget();
  const bool left = event.button == blink::WebPointerProperties::Button::kLeft;
  switch (event.GetType()) {
    case WebInputEvent::Type::kMouseDown:
      if (press_ || !left || (event.GetModifiers() & kAnyKeys) != kKeys) {
        return false;
      }
      press_ = event;
      return true;
    case WebInputEvent::Type::kMouseMove:
      if (!press_) {
        return false;
      }
      // The button went up where the page didn't see it, as over another app.
      if (!(event.GetModifiers() & WebInputEvent::kLeftButtonDown)) {
        if (dragging_) {
          Report(web_contents(), BC_CAPTURE_DRAG_ENDED, at);
        }
        press_.reset();
        dragging_ = false;
        return false;
      }
      if (!dragging_) {
        if ((at - press_->PositionInWidget()).Length() < kDrag) {
          return true;
        }
        dragging_ = true;
        Report(web_contents(), BC_CAPTURE_DRAG_BEGAN, press_->PositionInWidget());
      }
      Report(web_contents(), BC_CAPTURE_DRAG_MOVED, at);
      return true;
    case WebInputEvent::Type::kMouseUp:
      if (!press_ || !left) {
        return false;
      }
      if (dragging_) {
        Report(web_contents(), BC_CAPTURE_DRAG_ENDED, at);
        press_.reset();
        dragging_ = false;
        return true;
      }
      // A click: the page gets its press now, then this, so ⇧⌘-click on a link opens it in a tab.
      {
        const blink::WebMouseEvent press = *press_;
        press_.reset();
        replaying_ = true;
        widget_->ForwardMouseEvent(press);
        replaying_ = false;
      }
      return false;
    default:
      return false;
  }
}

void CaptureGesture::RenderFrameHostChanged(content::RenderFrameHost* old_host,
                                            content::RenderFrameHost* new_host) {
  if (new_host && new_host == web_contents()->GetPrimaryMainFrame()) {
    Watch(new_host->GetRenderWidgetHost());
  }
}

void CaptureGesture::WebContentsDestroyed() {
  Unwatch();
}

void CaptureGesture::RenderWidgetHostDestroyed(content::RenderWidgetHost* widget) {
  if (widget == widget_) {
    Unwatch();
  }
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(CaptureGesture);

}  // namespace browser_core
