#include "browser_core/platform/focus_ring_aura.h"

#include <algorithm>

#include "cc/paint/paint_flags.h"
#include "ui/aura/window.h"
#include "ui/compositor/paint_recorder.h"
#include "ui/gfx/canvas.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/rect_f.h"

namespace browser_core::platform {

namespace {

// Arc's: 2 points, inside the page's edge.
constexpr float kWidth = 2;

}  // namespace

FocusRing::FocusRing() {
  layer_->set_delegate(this);
  layer_->SetFillsBoundsOpaquely(false);
  layer_->SetName("BrowserCoreFocusRing");
}

FocusRing::~FocusRing() = default;

void FocusRing::Show(aura::Window* holder, uint32_t color, float radius) {
  const SkColor sk_color = SkColorSetRGB((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
  if (sk_color != color_ || radius != radius_) {
    color_ = sk_color;
    radius_ = radius;
    layer_->SchedulePaint(gfx::Rect(layer_->size()));
  }
  if (layer_->parent() != holder->layer()) {
    Hide();
    holder->layer()->Add(layer_.get());
  }
  holder->layer()->StackAtTop(layer_.get());
  Fit();
}

void FocusRing::Hide() {
  if (ui::Layer* parent = layer_->parent()) {
    parent->Remove(layer_.get());
  }
}

void FocusRing::Fit() {
  const ui::Layer* parent = layer_->parent();
  if (!parent) {
    return;
  }
  const gfx::Rect bounds(parent->bounds().size());
  if (bounds != layer_->bounds()) {
    layer_->SetBounds(bounds);
    layer_->SchedulePaint(bounds);
  }
}

bool FocusRing::In(const aura::Window* holder) const {
  return layer_->parent() && layer_->parent() == holder->layer();
}

void FocusRing::OnPaintLayer(const ui::PaintContext& context) {
  ui::PaintRecorder recorder(context, layer_->size());
  cc::PaintFlags flags;
  flags.setAntiAlias(true);
  flags.setStyle(cc::PaintFlags::kStroke_Style);
  flags.setStrokeWidth(kWidth);
  flags.setColor(color_);
  // The stroke's middle, half its width inside the edge, round as the page's corners are.
  gfx::RectF ring{gfx::SizeF(layer_->size())};
  ring.Inset(kWidth / 2);
  recorder.canvas()->DrawRoundRect(ring, std::max(radius_ - kWidth / 2, 0.f), flags);
}

}  // namespace browser_core::platform
