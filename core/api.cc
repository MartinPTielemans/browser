// The C API in public/browser_core.h, over the core's C++.

#include <algorithm>
#include <vector>

#include "browser_core/main_delegate.h"
#include "browser_core/public/browser_core.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/navigator/browser_navigator.h"
#include "chrome/browser/ui/navigator/browser_navigator_params.h"
#include "chrome/browser/ui/tabs/tab_enums.h"
#include "chrome/browser/ui/tabs/tab_strip_model.h"
#include "components/tabs/public/tab_interface.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/reload_type.h"
#include "content/public/browser/web_contents.h"
#include "ui/gfx/geometry/rect.h"
#include "ui/gfx/geometry/rect_conversions.h"
#include "ui/gfx/geometry/rect_f.h"
#include "url/gurl.h"

namespace {

gfx::Rect ToRect(bc_rect rect) {
  return gfx::ToEnclosingRect(gfx::RectF(rect.x, rect.y, rect.width, rect.height));
}

}  // namespace

int bc_main(int argc, const char** argv, const bc_host* host) {
  return browser_core::Run(argc, argv, host ? *host : bc_host{});
}

void bc_quit(void) {
  browser_core::Quit();
}

bc_id bc_window_open(bc_rect bounds, void* ui_view) {
  browser_core::Window* window = browser_core::Window::Open(ToRect(bounds), ui_view);
  return window ? window->id() : 0;
}

void bc_window_close(bc_id id) {
  if (browser_core::Window* window = browser_core::Window::FromId(id)) {
    window->Close();
  }
}

bc_id bc_space_create(const char* name) {
  return browser_core::Space::Create(name ? name : "")->id();
}

size_t bc_spaces(bc_id* spaces, size_t capacity) {
  std::vector<browser_core::Space*> all = browser_core::Space::All();
  for (size_t i = 0; i < std::min(capacity, all.size()); ++i) {
    spaces[i] = all[i]->id();
  }
  return all.size();
}

size_t bc_space_name(bc_id id, char* name, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  if (!space) {
    return 0;
  }
  const std::string& its = space->name();
  if (capacity > 0) {
    const size_t length = std::min(capacity - 1, its.size());
    std::copy_n(its.begin(), length, name);
    name[length] = '\0';
  }
  return its.size();
}

size_t bc_space_today(bc_id id, bc_id* tabs, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  if (!space) {
    return 0;
  }
  const std::vector<bc_id>& today = space->today();
  std::copy_n(today.begin(), std::min(capacity, today.size()), tabs);
  return today.size();
}

void bc_window_show_space(bc_id window_id, bc_id space) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->ShowSpace(space);
  }
}

void bc_window_set_page_frame(bc_id window_id, bc_rect frame) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetPageFrame(ToRect(frame));
  }
}

void bc_window_select(bc_id window_id, bc_id tab) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->Select(tab);
  }
}

bc_id bc_tab_open(bc_id window_id, const char* url) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  // Chrome made the profile it starts with before the core's calls work.
  Profile* profile = ProfileManager::GetLastUsedProfileIfLoaded();
  content::WebContents* contents = window && profile ? window->OpenTab(profile, GURL(url)) : nullptr;
  if (!contents) {
    return 0;
  }
  const bc_id tab = browser_core::Tab::IdOf(contents);
  if (browser_core::Space* space = browser_core::Space::FromId(window->space())) {
    space->Add(tab, /*opener=*/0, /*foreground=*/false);
  }
  return tab;
}

void bc_tab_show(bc_id id, bc_rect frame) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  browser_core::Window* window =
      tab ? browser_core::Window::ForBrowser(tab->GetBrowserWindowInterface()) : nullptr;
  if (window) {
    window->Show(contents, ToRect(frame));
  }
}

void bc_tab_navigate(bc_id id, const char* url) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!tab) {
    return;
  }
  NavigateParams params(tab->GetBrowserWindowInterface(), GURL(url), ui::PAGE_TRANSITION_TYPED);
  params.disposition = WindowOpenDisposition::CURRENT_TAB;
  params.source_contents = contents;
  Navigate(&params);
}

void bc_tab_go_back(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (contents && contents->GetController().CanGoBack()) {
    contents->GetController().GoBack();
  }
}

void bc_tab_go_forward(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (contents && contents->GetController().CanGoForward()) {
    contents->GetController().GoForward();
  }
}

void bc_tab_reload(bc_id id, bool bypass_cache) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    // As Chrome's, it asks before sending a form again.
    contents->GetController().Reload(
        bypass_cache ? content::ReloadType::BYPASSING_CACHE : content::ReloadType::NORMAL,
        /*check_for_repost=*/true);
  }
}

void bc_tab_stop(bc_id id) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    contents->Stop();
  }
}

void bc_tab_close(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!tab) {
    return;
  }
  TabStripModel* tabs = tab->GetBrowserWindowInterface()->GetTabStripModel();
  tabs->CloseWebContentsAt(tabs->GetIndexOfWebContents(contents),
                           TabCloseTypes::CLOSE_USER_GESTURE);
}
