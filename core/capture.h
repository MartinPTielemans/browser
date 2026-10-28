#ifndef BROWSER_CORE_CAPTURE_H_
#define BROWSER_CORE_CAPTURE_H_

#include <cstdint>
#include <optional>
#include <vector>

#include "base/containers/flat_map.h"
#include "base/functional/callback.h"
#include "base/memory/scoped_refptr.h"
#include "base/memory/weak_ptr.h"
#include "base/values.h"
#include "content/public/browser/devtools_agent_host_client.h"
#include "content/public/browser/render_widget_host_view.h"
#include "content/public/browser/web_contents_user_data.h"
#include "ui/gfx/geometry/point_f.h"
#include "ui/gfx/geometry/rect_f.h"

namespace content {
class DevToolsAgentHost;
}  // namespace content

namespace browser_core {

// A tab's Capture: the element under the pointer and the picture of the page, which the app's
// capture mode draws. DevTools hit tests the page and draws it beyond its view, without the
// page's JavaScript running, through a client of the tab's own that attaches only while the user
// captures.
class PageCapture : public content::WebContentsUserData<PageCapture>,
                    public content::DevToolsAgentHostClient {
 public:
  PageCapture(const PageCapture&) = delete;
  PageCapture& operator=(const PageCapture&) = delete;
  ~PageCapture() override;

  // See bc_tab_element_at.
  void ElementAt(gfx::PointF point);
  // See bc_tab_capture.
  void Capture(uint64_t request, bool full_page, gfx::RectF rect);
  // See bc_tab_capture_end.
  void End();

 private:
  friend class content::WebContentsUserData<PageCapture>;

  using Reply = base::OnceCallback<void(const base::DictValue* result)>;

  explicit PageCapture(content::WebContents* contents);

  // Sends a DevTools command, attaching first if need be. |reply| gets its result, or null for an
  // error or a page that went.
  void Send(const char* method, base::DictValue params, Reply reply);
  void Detach();

  // The steps of the question about |point|: where the page has scrolled, the element there, and
  // its box.
  void AskElementAt(gfx::PointF point);
  void HitTest(gfx::PointF point, double zoom, const base::DictValue* metrics);
  void Box(gfx::PointF point, double zoom, const base::DictValue* node);
  void Answer(gfx::PointF point, gfx::RectF box);

  void CopiedSurface(uint64_t request, const content::CopyFromSurfaceResult& result);
  void Screenshot(uint64_t request, const base::DictValue* result);
  // Answers bc_tab_capture's |request| with |png|, or nothing, from a task of its own.
  void Captured(uint64_t request, std::optional<std::vector<uint8_t>> png);
  void ReportCaptured(uint64_t request, std::optional<std::vector<uint8_t>> png);

  // The zoom the tab's page is shown at, by which a CSS pixel is that many points.
  double Zoom();

  // content::DevToolsAgentHostClient:
  void DispatchProtocolMessage(content::DevToolsAgentHost* agent_host,
                               base::span<const uint8_t> message) override;
  void AgentHostClosed(content::DevToolsAgentHost* agent_host) override;

  scoped_refptr<content::DevToolsAgentHost> agent_host_;
  int next_command_ = 1;
  base::flat_map<int, Reply> replies_;
  // A question about an element is in the page, and the point the pointer moved to since, which
  // replaces any it moved to before.
  bool asking_ = false;
  std::optional<gfx::PointF> waiting_;
  base::WeakPtrFactory<PageCapture> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

}  // namespace browser_core

#endif  // BROWSER_CORE_CAPTURE_H_
