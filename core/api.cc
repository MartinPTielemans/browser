// The C API in public/browser_core.h, over the core's C++.

#include <algorithm>
#include <string>
#include <vector>

#include "base/strings/utf_string_conversions.h"
#include "browser_core/find.h"
#include "browser_core/main_delegate.h"
#include "browser_core/public/browser_core.h"
#include "browser_core/search.h"
#include "browser_core/session.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/profiles/profile_manager.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/navigator/browser_navigator.h"
#include "chrome/browser/ui/navigator/browser_navigator_params.h"
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

// Copies up to |capacity| bytes of |text| into |to|, ending in a NUL, and returns its length.
size_t CopyString(const std::string& text, char* to, size_t capacity) {
  if (capacity > 0) {
    const size_t length = std::min(capacity - 1, text.size());
    std::copy_n(text.begin(), length, to);
    to[length] = '\0';
  }
  return text.size();
}

// Loads |url| in the tab, as Chrome loads what's typed in its location bar or a bookmark.
void LoadInTab(bc_id id, const GURL& url, ui::PageTransition transition) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!tab) {
    return;
  }
  NavigateParams params(tab->GetBrowserWindowInterface(), url, transition);
  params.disposition = WindowOpenDisposition::CURRENT_TAB;
  params.source_contents = contents;
  Navigate(&params);
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

bool bc_window_restore(bc_id id) {
  browser_core::Window* window = browser_core::Window::FromId(id);
  return window && browser_core::RestoreSession(*window);
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
  return space ? CopyString(space->name(), name, capacity) : 0;
}

void bc_space_set_name(bc_id id, const char* name) {
  if (browser_core::Space* space = browser_core::Space::FromId(id)) {
    space->SetName(name ? name : "");
  }
}

bool bc_space_color(bc_id id, uint32_t* color) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  if (!space || !space->color()) {
    return false;
  }
  *color = *space->color();
  return true;
}

void bc_space_set_color(bc_id id, bool has_color, uint32_t color) {
  if (browser_core::Space* space = browser_core::Space::FromId(id)) {
    space->SetColor(has_color ? std::optional(color & 0xffffff) : std::nullopt);
  }
}

bool bc_space_delete(bc_id id) {
  return browser_core::Window::DeleteSpace(id);
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

size_t bc_space_pinned(bc_id id, bc_id* tabs, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  if (!space) {
    return 0;
  }
  const std::vector<bc_id>& pinned = space->pinned();
  std::copy_n(pinned.begin(), std::min(capacity, pinned.size()), tabs);
  return pinned.size();
}

void bc_window_show_space(bc_id window_id, bc_id space) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->ShowSpace(space);
  }
}

double bc_window_sidebar_width(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->sidebar_width() : 0;
}

void bc_window_set_sidebar_width(bc_id window_id, double width) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetSidebarWidth(width);
  }
}

bc_id bc_window_space_tab(bc_id window_id, bc_id space) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->TabToShowIn(space) : 0;
}

void bc_window_set_page_frame(bc_id window_id, bc_rect frame, double corner_radius) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetPageFrame(ToRect(frame), corner_radius);
  }
}

void bc_window_select(bc_id window_id, bc_id tab) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->Select(tab);
  }
}

size_t bc_window_clear(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->Clear() : 0;
}

size_t bc_window_undo_clear(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->UndoClear() : 0;
}

bc_id bc_window_reopen_closed_tab(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->ReopenClosedTab() : 0;
}

void bc_window_suggest(bc_id window_id, const char* text, bool complete) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  // Chrome made the profile it starts with before the core's calls work.
  Profile* profile = ProfileManager::GetLastUsedProfileIfLoaded();
  if (window && profile) {
    window->Suggest(profile, text ? base::UTF8ToUTF16(text) : std::u16string(), complete);
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

size_t bc_typed_url(const char* text, char* url, size_t capacity) {
  // Chrome made the profile it starts with before the core's calls work.
  Profile* profile = ProfileManager::GetLastUsedProfileIfLoaded();
  const GURL typed = profile && text ? browser_core::TypedUrl(profile, text) : GURL();
  return typed.is_valid() ? CopyString(typed.spec(), url, capacity) : 0;
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
  LoadInTab(id, GURL(url), ui::PAGE_TRANSITION_TYPED);
}

void bc_tab_back_to_pinned_url(bc_id id) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!space || !space->IsPinned(id) || !contents) {
    return;
  }
  const GURL url = space->PinnedUrl(id);
  // A tab already there doesn't load it again.
  if (browser_core::Tab::PageUrl(contents) != url) {
    LoadInTab(id, url, ui::PAGE_TRANSITION_AUTO_BOOKMARK);
  }
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

size_t bc_tab_find_open(bc_id id, char* text, size_t capacity) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  const std::u16string open =
      contents ? browser_core::FindInPage::GetOrCreateForWebContents(contents)->Open()
               : std::u16string();
  return CopyString(base::UTF16ToUTF8(open), text, capacity);
}

void bc_tab_find(bc_id id, const char* text, bool forward) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id); contents && text) {
    browser_core::FindInPage::GetOrCreateForWebContents(contents)->Find(base::UTF8ToUTF16(text),
                                                                        forward);
  }
}

void bc_tab_find_close(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (browser_core::FindInPage* find =
          contents ? browser_core::FindInPage::FromWebContents(contents) : nullptr) {
    find->Close();
  }
}

void bc_tab_set_pinned(bc_id id, bool pinned) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!space || !contents) {
    return;
  }
  if (pinned) {
    space->Pin(id, browser_core::Tab::PageUrl(contents));
  } else {
    space->Unpin(id);
  }
  // Whether it's away from its pinned URL.
  browser_core::Tab::FromWebContents(contents)->StateChanged();
}

void bc_tab_move(bc_id id, bool pinned, size_t index) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!space || !contents) {
    return;
  }
  space->Move(id, pinned, index, browser_core::Tab::PageUrl(contents));
  // Whether it's away from its pinned URL.
  browser_core::Tab::FromWebContents(contents)->StateChanged();
}

bc_id bc_tab_duplicate(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  browser_core::Window* window =
      tab ? browser_core::Window::ForBrowser(tab->GetBrowserWindowInterface()) : nullptr;
  return window ? window->Duplicate(id) : 0;
}

void bc_tab_close_below(bc_id id) {
  browser_core::Window::CloseBelow(id);
}

void bc_tab_move_to_space(bc_id id, bc_id space) {
  browser_core::Window::MoveToSpace(id, space);
}

void bc_tab_close(bc_id id) {
  if (browser_core::Space* space = browser_core::Space::Holding(id); space && space->IsPinned(id)) {
    browser_core::Window::UnloadPinned(id);
    return;
  }
  browser_core::Window::CloseTab(id);
}
