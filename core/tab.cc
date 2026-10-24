#include "browser_core/tab.h"

#include <map>
#include <string>

#include "base/no_destructor.h"
#include "base/strings/utf_string_conversions.h"
#include "browser_core/host.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

std::map<bc_id, content::WebContents*>& Tabs() {
  static base::NoDestructor<std::map<bc_id, content::WebContents*>> tabs;
  return *tabs;
}

}  // namespace

// static
bc_id Tab::IdOf(content::WebContents* contents) {
  return GetOrCreateForWebContents(contents)->id_;
}

// static
content::WebContents* Tab::FromId(bc_id id) {
  auto it = Tabs().find(id);
  return it == Tabs().end() ? nullptr : it->second;
}

Tab::Tab(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<Tab>(*contents),
      id_(NewId()) {
  Tabs()[id_] = contents;
}

Tab::~Tab() {
  Tabs().erase(id_);
}

void Tab::DidFinishLoad(content::RenderFrameHost* frame, const GURL& url) {
  if (frame->IsInPrimaryMainFrame() && Host().tab_loaded) {
    Host().tab_loaded(Host().context, id_, url.spec().c_str());
  }
}

void Tab::TitleWasSet(content::NavigationEntry* entry) {
  if (Host().tab_title_changed) {
    const std::string title = base::UTF16ToUTF8(web_contents()->GetTitle());
    Host().tab_title_changed(Host().context, id_, title.c_str());
  }
}

void Tab::DidToggleFullscreenModeForTab(bool entered_fullscreen, bool will_cause_resize) {
  if (Host().tab_fullscreen_changed) {
    Host().tab_fullscreen_changed(Host().context, id_, entered_fullscreen);
  }
}

void Tab::WebContentsDestroyed() {
  if (Host().tab_closed) {
    Host().tab_closed(Host().context, id_);
  }
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(Tab);

}  // namespace browser_core
