#include "browser_core/capture.h"

#include <algorithm>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "base/base64.h"
#include "base/containers/span.h"
#include "base/functional/bind.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/strings/string_number_conversions.h"
#include "base/task/thread_pool.h"
#include "base/time/time.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "components/viz/common/frame_sinks/copy_output_result.h"
#include "content/public/browser/devtools_agent_host.h"
#include "content/public/browser/host_zoom_map.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents.h"
#include "third_party/blink/public/common/page/page_zoom.h"
#include "third_party/skia/include/core/SkBitmap.h"
#include "third_party/skia/include/core/SkColorSpace.h"
#include "third_party/skia/include/core/SkImageInfo.h"
#include "ui/gfx/codec/png_codec.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/rect_conversions.h"

namespace browser_core {

namespace {

// A PNG's size, from its header, which DevTools' screenshot doesn't otherwise give.
gfx::Size PngSize(const std::vector<uint8_t>& png) {
  if (png.size() < 24) {
    return gfx::Size();
  }
  auto read = [&](size_t at) {
    return static_cast<int>(png[at] << 24 | png[at + 1] << 16 | png[at + 2] << 8 | png[at + 3]);
  };
  return gfx::Size(read(16), read(20));
}

// A picture of a page for a window that shows it elsewhere, in sRGB, as the PNGs are, and in the
// BGRA order GPUI draws, so the app reads it as it is.
struct Pixels {
  std::vector<uint8_t> bgra;
  int width = 0;
  int height = 0;
};

std::optional<Pixels> ToPixels(const SkBitmap& bitmap) {
  const SkImageInfo info = SkImageInfo::Make(bitmap.width(), bitmap.height(),
                                             kBGRA_8888_SkColorType, kUnpremul_SkAlphaType,
                                             SkColorSpace::MakeSRGB());
  Pixels pixels{std::vector<uint8_t>(info.computeMinByteSize()), bitmap.width(), bitmap.height()};
  if (pixels.bgra.empty() ||
      !bitmap.readPixels(info, pixels.bgra.data(), info.minRowBytes(), 0, 0)) {
    return std::nullopt;
  }
  return pixels;
}

void Pictured(bc_id tab, uint64_t request, std::optional<Pixels> pixels) {
  LogHandover("picture made", "tab " + base::NumberToString(tab));
  if (Host().tab_pictured) {
    Host().tab_pictured(Host().context, tab, request, pixels ? pixels->bgra.data() : nullptr,
                        pixels ? pixels->width : 0, pixels ? pixels->height : 0);
  }
}

}  // namespace

void PicturePage(content::WebContents* contents, uint64_t request) {
  const bc_id tab = Tab::IdOf(contents);
  content::RenderWidgetHostView* view = contents->GetRenderWidgetHostView();
  if (!view) {
    Pictured(tab, request, std::nullopt);
    return;
  }
  // The compositor's last frame, at the display's scale, made the app's on a worker: a page in a
  // big window is millions of pixels, which the UI thread, drawing the page's next frames, has no
  // time for. A PNG would take longer still, made here and read in the app.
  view->CopyFromSurface(
      gfx::Rect(), gfx::Size(), base::TimeDelta(),
      base::BindOnce(
          [](bc_id tab, uint64_t request, const content::CopyFromSurfaceResult& result) {
            LogHandover("picture copied", "tab " + base::NumberToString(tab));
            if (!result.has_value() || result->bitmap.drawsNothing()) {
              Pictured(tab, request, std::nullopt);
              return;
            }
            base::ThreadPool::PostTaskAndReplyWithResult(
                FROM_HERE, {base::TaskPriority::USER_BLOCKING},
                base::BindOnce(&ToPixels, result->bitmap),
                base::BindOnce(&Pictured, tab, request));
          },
          tab, request));
}

PageCapture::PageCapture(content::WebContents* contents)
    : content::WebContentsUserData<PageCapture>(*contents) {}

PageCapture::~PageCapture() {
  Detach();
}

void PageCapture::ElementAt(gfx::PointF point) {
  if (asking_) {
    waiting_ = point;
    return;
  }
  AskElementAt(point);
}

void PageCapture::AskElementAt(gfx::PointF point) {
  asking_ = true;
  // Read where the page has scrolled with each question, as the user may scroll while capturing.
  Send("Page.getLayoutMetrics", base::DictValue(),
       base::BindOnce(&PageCapture::HitTest, weak_factory_.GetWeakPtr(), point, Zoom()));
}

void PageCapture::HitTest(gfx::PointF point, double zoom, const base::DictValue* metrics) {
  const base::DictValue* viewport = metrics ? metrics->FindDict("cssLayoutViewport") : nullptr;
  if (!viewport) {
    Answer(point, gfx::RectF());
    return;
  }
  // DevTools' hit test takes the page's CSS pixels from the top of the document.
  const double x = point.x() / zoom + viewport->FindDouble("pageX").value_or(0);
  const double y = point.y() / zoom + viewport->FindDouble("pageY").value_or(0);
  Send("DOM.getNodeForLocation",
       base::DictValue()
           .Set("x", static_cast<int>(x))
           .Set("y", static_cast<int>(y))
           .Set("ignorePointerEventsNone", true),
       base::BindOnce(&PageCapture::Box, weak_factory_.GetWeakPtr(), point, zoom));
}

void PageCapture::Box(gfx::PointF point, double zoom, const base::DictValue* node) {
  std::optional<int> id = node ? node->FindInt("backendNodeId") : std::nullopt;
  if (!id) {
    Answer(point, gfx::RectF());
    return;
  }
  Send("DOM.getBoxModel", base::DictValue().Set("backendNodeId", *id),
       base::BindOnce(
           [](base::WeakPtr<PageCapture> capture, gfx::PointF point, double zoom,
              const base::DictValue* result) {
             if (!capture) {
               return;
             }
             const base::DictValue* model = result ? result->FindDict("model") : nullptr;
             const base::ListValue* quad = model ? model->FindList("border") : nullptr;
             if (!quad || quad->size() != 8) {
               capture->Answer(point, gfx::RectF());
               return;
             }
             // The border's quad, in the view's CSS pixels, is a rectangle unless the page
             // transforms it, so its bounds are what the user sees highlighted.
             double left = (*quad)[0].GetIfDouble().value_or(0), right = left;
             double top = (*quad)[1].GetIfDouble().value_or(0), bottom = top;
             for (size_t i = 2; i < 8; i += 2) {
               const double x = (*quad)[i].GetIfDouble().value_or(0);
               const double y = (*quad)[i + 1].GetIfDouble().value_or(0);
               left = std::min(left, x);
               right = std::max(right, x);
               top = std::min(top, y);
               bottom = std::max(bottom, y);
             }
             capture->Answer(point, gfx::RectF(left * zoom, top * zoom, (right - left) * zoom,
                                               (bottom - top) * zoom));
           },
           weak_factory_.GetWeakPtr(), point, zoom));
}

void PageCapture::Answer(gfx::PointF point, gfx::RectF box) {
  asking_ = false;
  // Every answer goes to the app, as one for a point the pointer just left is closer than none
  // while it keeps moving.
  if (Host().tab_element_box) {
    Host().tab_element_box(Host().context, Tab::IdOf(&GetWebContents()), point.x(), point.y(),
                           bc_rect{box.x(), box.y(), box.width(), box.height()});
  }
  if (waiting_) {
    AskElementAt(*std::exchange(waiting_, std::nullopt));
  }
}

void PageCapture::Capture(uint64_t request, bool full_page, gfx::RectF rect) {
  if (full_page) {
    Send("Page.captureScreenshot",
         base::DictValue().Set("format", "png").Set("captureBeyondViewport", true),
         base::BindOnce(&PageCapture::Screenshot, weak_factory_.GetWeakPtr(), request));
    return;
  }
  content::RenderWidgetHostView* view = GetWebContents().GetRenderWidgetHostView();
  if (!view) {
    Captured(request, nullptr);
    return;
  }
  // The compositor's last frame, which is what the user saw when they chose, at the display's
  // scale, as the output size left empty keeps it.
  view->CopyFromSurface(gfx::ToEnclosingRect(rect), gfx::Size(), base::TimeDelta(),
                        base::BindOnce(&PageCapture::CopiedSurface, weak_factory_.GetWeakPtr(),
                                       request));
}

void PageCapture::CopiedSurface(uint64_t request, const content::CopyFromSurfaceResult& result) {
  std::optional<std::vector<uint8_t>> png;
  if (result.has_value() && !result->bitmap.drawsNothing()) {
    // The frame is in the display's colour space, and the PNG has no profile, so it reads as sRGB,
    // in capture mode's picture and wherever it's pasted: converted, it keeps the page's colours.
    SkBitmap bitmap = result->bitmap;
    if (bitmap.colorSpace() && !bitmap.colorSpace()->isSRGB()) {
      SkBitmap srgb;
      if (srgb.tryAllocPixels(bitmap.info().makeColorSpace(SkColorSpace::MakeSRGB())) &&
          bitmap.readPixels(srgb.pixmap())) {
        bitmap = srgb;
      }
    }
    png = gfx::PNGCodec::EncodeBGRASkBitmap(bitmap, /*discard_transparency=*/true);
  }
  Captured(request, png ? &*png : nullptr);
}

void PageCapture::Screenshot(uint64_t request, const base::DictValue* result) {
  const std::string* data = result ? result->FindString("data") : nullptr;
  std::optional<std::vector<uint8_t>> png = data ? base::Base64Decode(*data) : std::nullopt;
  Captured(request, png ? &*png : nullptr);
}

void PageCapture::Captured(uint64_t request, const std::vector<uint8_t>* png) {
  if (!Host().tab_captured) {
    return;
  }
  const gfx::Size size = png ? PngSize(*png) : gfx::Size();
  Host().tab_captured(Host().context, Tab::IdOf(&GetWebContents()), request,
                      png ? png->data() : nullptr, png ? png->size() : 0, size.width(),
                      size.height());
}

void PageCapture::End() {
  asking_ = false;
  waiting_.reset();
  Detach();
}

void PageCapture::Send(const char* method, base::DictValue params, Reply reply) {
  if (!agent_host_) {
    scoped_refptr<content::DevToolsAgentHost> host =
        content::DevToolsAgentHost::GetOrCreateFor(&GetWebContents());
    // The profile's policy may keep DevTools from the page, which leaves it without Capture too.
    if (!host || !host->AttachClient(this)) {
      std::move(reply).Run(nullptr);
      return;
    }
    agent_host_ = std::move(host);
  }
  const int id = next_command_++;
  std::optional<std::string> json = base::WriteJson(base::DictValue()
                                                        .Set("id", id)
                                                        .Set("method", method)
                                                        .Set("params", std::move(params)));
  if (!json) {
    std::move(reply).Run(nullptr);
    return;
  }
  replies_[id] = std::move(reply);
  agent_host_->DispatchProtocolMessage(this, base::as_byte_span(*json));
}

void PageCapture::Detach() {
  // Answers still owed go unanswered: the app asked to end, or the tab is closing.
  replies_.clear();
  if (scoped_refptr<content::DevToolsAgentHost> host = std::move(agent_host_)) {
    host->DetachClient(this);
  }
}

void PageCapture::DispatchProtocolMessage(content::DevToolsAgentHost* agent_host,
                                          base::span<const uint8_t> message) {
  std::optional<base::DictValue> dict = base::JSONReader::ReadDict(
      std::string_view(reinterpret_cast<const char*>(message.data()), message.size()),
      base::JSON_PARSE_RFC);
  std::optional<int> id = dict ? dict->FindInt("id") : std::nullopt;
  // Events, which the client doesn't ask for, have no id.
  if (!id) {
    return;
  }
  auto reply = replies_.find(*id);
  if (reply == replies_.end()) {
    return;
  }
  Reply run = std::move(reply->second);
  replies_.erase(reply);
  std::move(run).Run(dict->FindDict("result"));
}

void PageCapture::AgentHostClosed(content::DevToolsAgentHost* agent_host) {
  agent_host_ = nullptr;
  // The page's renderer went: what was asked is answered as nothing, so the app isn't left waiting.
  base::flat_map<int, Reply> replies = std::move(replies_);
  replies_.clear();
  for (auto& [id, reply] : replies) {
    std::move(reply).Run(nullptr);
  }
}

double PageCapture::Zoom() {
  return blink::ZoomLevelToZoomFactor(content::HostZoomMap::GetZoomLevel(&GetWebContents()));
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(PageCapture);

}  // namespace browser_core
