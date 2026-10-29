// The ring inside a split view's focused page's edge, over the page, where the pages are aura
// windows, on Linux and Windows, as BrowserCoreFocusRing is on the Mac (mac/window_views.mm). A
// layer with no window of its own, so clicks go through it to the page and it never has the
// keyboard.

#ifndef BROWSER_CORE_PLATFORM_FOCUS_RING_AURA_H_
#define BROWSER_CORE_PLATFORM_FOCUS_RING_AURA_H_

#include <cstdint>
#include <memory>
#include <optional>

#include "third_party/skia/include/core/SkColor.h"
#include "ui/compositor/layer.h"
#include "ui/compositor/layer_delegate.h"
#include "ui/gfx/geometry/rect.h"

namespace aura {
class Window;
}

namespace views {
class Widget;
}

namespace browser_core::platform {

class FocusRing : public ui::LayerDelegate {
 public:
  FocusRing();
  FocusRing(const FocusRing&) = delete;
  FocusRing& operator=(const FocusRing&) = delete;
  ~FocusRing() override;

  // Over the pages |holder| holds, filling it, in |color|, 0xRRGGBB, with corners of |radius|. A
  // page aura adds to the holder later goes over it, so the holder's pages are in first.
  void Show(aura::Window* holder, uint32_t color, float radius);
  void Hide();
  // Fills its holder again, once that's resized.
  void Fit();
  // Whether it shows in |holder|.
  bool In(const aura::Window* holder) const;

 private:
  // ui::LayerDelegate:
  void OnPaintLayer(const ui::PaintContext& context) override;
  void OnDeviceScaleFactorChanged(float old_scale, float new_scale) override {}

  std::unique_ptr<ui::Layer> layer_ = ui::Layer::Create(ui::LAYER_TEXTURED);
  SkColor color_ = SK_ColorTRANSPARENT;
  float radius_ = 0;
};

// Where the ring shows in |widget|'s window, from its content's top-left, if it does: for the test
// driver, which sees no layer, as the Mac's sees BrowserCoreFocusRing (bc_window_focus_ring).
std::optional<gfx::Rect> FocusRingFrame(views::Widget* widget);

}  // namespace browser_core::platform

#endif  // BROWSER_CORE_PLATFORM_FOCUS_RING_AURA_H_
