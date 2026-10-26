// The C API in public/browser_core.h, over the core's C++.

#include <algorithm>
#include <string>
#include <vector>

#include "base/functional/bind.h"
#include "base/functional/callback_helpers.h"
#include "base/numerics/safe_conversions.h"
#include "base/strings/utf_string_conversions.h"
#include "base/time/time.h"
#include "browser_core/archive.h"
#include "browser_core/auto_archive.h"
#include "browser_core/capture.h"
#include "browser_core/clear_site.h"
#include "browser_core/downloads.h"
#include "browser_core/extensions.h"
#include "browser_core/favicon.h"
#include "browser_core/favorites.h"
#include "browser_core/find.h"
#include "browser_core/install_question.h"
#include "browser_core/main_delegate.h"
#include "browser_core/password_offer.h"
#include "browser_core/permission_question.h"
#include "browser_core/profiles.h"
#include "browser_core/public/browser_core.h"
#include "browser_core/search.h"
#include "browser_core/session.h"
#include "browser_core/site_controls.h"
#include "browser_core/space.h"
#include "browser_core/tab.h"
#include "browser_core/window.h"
#include "chrome/browser/devtools/devtools_toggle_action.h"
#include "chrome/browser/devtools/devtools_window.h"
#include "chrome/browser/ui/browser_commands.h"
#include "chrome/browser/ui/browser_window/public/browser_window_interface.h"
#include "chrome/browser/ui/navigator/browser_navigator.h"
#include "chrome/browser/ui/navigator/browser_navigator_params.h"
#include "components/tabs/public/tab_interface.h"
#include "components/zoom/page_zoom.h"
#include "components/zoom/zoom_controller.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/render_frame_host.h"
#include "content/public/browser/web_contents.h"
#include "content/public/common/page_zoom.h"
#include "ui/gfx/geometry/point_f.h"
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

// The page |id| is at, which pinning it pins it at.
GURL PageUrlOf(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  return contents ? browser_core::Tab::PageUrl(contents) : GURL();
}

// After |id| was pinned, unpinned or moved, with the rest of its split.
void SplitMoved(browser_core::Space& space, bc_id id) {
  for (bc_id pane : space.Panes(id)) {
    if (content::WebContents* contents = browser_core::Tab::FromId(pane)) {
      browser_core::Tab* tab = browser_core::Tab::FromWebContents(contents);
      // Handled now, so a pinned tab unpinned unseen has 12 hours in Today before it's archived.
      tab->set_last_seen(base::Time::Now());
      // Whether it's away from its pinned URL.
      tab->StateChanged();
    }
  }
}

// The profile of the Space the window shows, which what it opens and suggests comes from: for a
// private window's, the off-the-record one, or null once its session has gone, as only opening a
// private window starts one.
Profile* ProfileOf(const browser_core::Window& window) {
  const browser_core::Space* space = browser_core::Space::FromId(window.space());
  Profile* profile = browser_core::ProfileFor(space ? space->profile() : std::string());
  return space && space->off_the_record() && profile ? browser_core::PrivateProfileOf(profile)
                                                     : profile;
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

// The loaded profile |profile_id| names, or null.
Profile* LoadedProfile(bc_id profile_id) {
  const std::string dir = browser_core::ProfileDir(profile_id);
  // ProfileFor gives the first profile for one that isn't loaded.
  Profile* profile = dir.empty() ? nullptr : browser_core::ProfileFor(dir);
  return profile && browser_core::DirOf(profile) == dir ? profile : nullptr;
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
  browser_core::Space* space = browser_core::Space::Create(name ? name : "");
  space->SetProfile(browser_core::DirOf(browser_core::StartProfile()));
  return space->id();
}

bc_id bc_space_create_for_agent(const char* name, const char* agent, const char* agent_name) {
  if (!agent || !*agent || !agent_name || !*agent_name) {
    return 0;
  }
  browser_core::Space* space = browser_core::Space::Create(
      name ? name : "", 0, browser_core::Space::Agent{.id = agent, .name = agent_name});
  space->SetProfile(browser_core::DirOf(browser_core::StartProfile()));
  return space->id();
}

size_t bc_space_agent(bc_id id, char* agent, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return CopyString(space && space->agent() ? space->agent()->id : std::string(), agent, capacity);
}

size_t bc_space_agent_name(bc_id id, char* name, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return CopyString(space && space->agent() ? space->agent()->name : std::string(), name,
                    capacity);
}

bool bc_agent_may_act(const char* agent, bc_id tab) {
  return agent && *agent && browser_core::Space::AgentMayAct(agent, tab);
}

bc_id bc_space_profile(bc_id id) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return space ? browser_core::ProfileId(space->profile()) : 0;
}

bool bc_space_set_profile(bc_id id, bc_id profile) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  const std::string dir = browser_core::ProfileDir(profile);
  if (!space || space->off_the_record() || dir.empty() || !space->pinned_rows().empty() ||
      !space->today().empty()) {
    return false;
  }
  // At once for a profile that's loaded, as a Space's is.
  browser_core::LoadProfiles({dir},
                             base::BindOnce(&browser_core::Window::SetSpaceProfile, id, dir));
  return true;
}

size_t bc_profiles(bc_id* profiles, size_t capacity) {
  const std::vector<bc_id> all = browser_core::Profiles();
  std::copy_n(all.begin(), std::min(capacity, all.size()), profiles);
  return all.size();
}

size_t bc_profile_name(bc_id profile, char* name, size_t capacity) {
  return CopyString(browser_core::ProfileName(profile), name, capacity);
}

bool bc_profile_rename(bc_id profile, const char* name) {
  return browser_core::RenameProfile(profile, name ? name : "");
}

bc_id bc_profile_create(const char* name) {
  return browser_core::CreateProfile(name ? name : "", base::DoNothing());
}

bool bc_profile_delete(bc_id profile) {
  return browser_core::DeleteProfile(profile);
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

size_t bc_space_icon(bc_id id, char* icon, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return space ? CopyString(space->icon(), icon, capacity) : 0;
}

void bc_space_set_icon(bc_id id, const char* icon) {
  if (browser_core::Space* space = browser_core::Space::FromId(id)) {
    space->SetIcon(icon ? icon : "");
  }
}

size_t bc_space_theme(bc_id id, char* theme, size_t capacity) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return space ? CopyString(space->theme(), theme, capacity) : 0;
}

void bc_space_set_theme(bc_id id, const char* theme) {
  if (browser_core::Space* space = browser_core::Space::FromId(id)) {
    space->SetTheme(theme ? theme : "");
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
  const std::vector<bc_id>& pinned = space->pinned_rows();
  std::copy_n(pinned.begin(), std::min(capacity, pinned.size()), tabs);
  return pinned.size();
}

bool bc_folder(bc_id id, bool* open) {
  browser_core::Space* space = browser_core::Space::HoldingFolder(id);
  if (space && open) {
    *open = space->IsOpen(id);
  }
  return space;
}

size_t bc_folder_name(bc_id folder, char* name, size_t capacity) {
  browser_core::Space* space = browser_core::Space::HoldingFolder(folder);
  return space ? CopyString(space->FolderName(folder), name, capacity) : 0;
}

bc_id bc_folder_of(bc_id id) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space) {
    space = browser_core::Space::HoldingFolder(id);
  }
  return space ? space->FolderOf(id) : 0;
}

bc_id bc_folder_create(bc_id id, const char* name, bc_id folder, size_t index) {
  browser_core::Space* space = browser_core::Space::FromId(id);
  return space ? space->AddFolder(name ? name : "", folder, index) : 0;
}

void bc_folder_rename(bc_id folder, const char* name) {
  if (browser_core::Space* space = browser_core::Space::HoldingFolder(folder)) {
    space->RenameFolder(folder, name ? name : "");
  }
}

void bc_folder_set_open(bc_id folder, bool open) {
  if (browser_core::Space* space = browser_core::Space::HoldingFolder(folder)) {
    space->SetOpen(folder, open);
  }
}

void bc_folder_move(bc_id folder, bc_id to, size_t index) {
  if (browser_core::Space* space = browser_core::Space::HoldingFolder(folder)) {
    space->Move(folder, /*pinned=*/true, to, index, &PageUrlOf);
  }
}

void bc_folder_delete(bc_id folder) {
  if (browser_core::Space* space = browser_core::Space::HoldingFolder(folder)) {
    space->DeleteFolder(folder);
  }
}

bc_id bc_tab_profile(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  return contents ? browser_core::ProfileId(browser_core::DirOf(contents)) : 0;
}

size_t bc_favorites(bc_id* tabs, size_t capacity) {
  const std::vector<bc_id> favorites = browser_core::Favorites::All();
  std::copy_n(favorites.begin(), std::min(capacity, favorites.size()), tabs);
  return favorites.size();
}

bc_id bc_favorite_space(bc_id tab) {
  return browser_core::Favorites::SpaceOf(tab);
}

int bc_favorites_scope(void) {
  return browser_core::Favorites::scope();
}

void bc_set_favorites_scope(int scope) {
  browser_core::Favorites::SetScope(scope);
  // Each window stops showing a favorite its Space no longer has.
  browser_core::Window::HiddenFavoritesGone();
}

size_t bc_tab_split(bc_id id, bc_id* tabs, size_t capacity) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space) {
    return 0;
  }
  const std::vector<bc_id> panes = space->Panes(id);
  std::copy_n(panes.begin(), std::min(capacity, panes.size()), tabs);
  return panes.size();
}

bool bc_tab_split_with(bc_id id, bc_id with, bool left) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space || browser_core::Space::Holding(with) != space ||
      !space->Split(id, with, PageUrlOf(id), left)) {
    return false;
  }
  SplitMoved(*space, id);
  browser_core::Window::PanesChanged();
  return true;
}

void bc_tab_separate(bc_id id) {
  if (browser_core::Space* space = browser_core::Space::Holding(id)) {
    space->Separate(id);
    browser_core::Window::PanesChanged();
  }
}

bool bc_tab_move_in_split(bc_id id, size_t index) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space || !space->MoveInSplit(id, index)) {
    return false;
  }
  browser_core::Window::PanesChanged();
  return true;
}

bool bc_tab_split_stacked(bc_id id) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  return space && space->Stacked(id);
}

double bc_tab_split_share(bc_id id) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  std::vector<bc_id> panes = space ? space->Panes(id) : std::vector<bc_id>();
  if (panes.size() < 2) {
    return 0;
  }
  std::vector<double> shares = space->Shares(id);
  return shares.empty() ? 1.0 / panes.size() : shares[std::ranges::find(panes, id) - panes.begin()];
}

bool bc_tab_set_split_shares(bc_id id, const double* shares, size_t count) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space ||
      !space->SetShares(id, shares ? std::vector<double>(shares, shares + count)
                                   : std::vector<double>())) {
    return false;
  }
  browser_core::Window::PanesChanged();
  return true;
}

bool bc_tab_set_split_stacked(bc_id id, bool stacked) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if (!space || !space->SetStacked(id, stacked)) {
    return false;
  }
  browser_core::Window::PanesChanged();
  return true;
}

void bc_window_show_space(bc_id window_id, bc_id space) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->ShowSpace(space);
  }
}

void bc_window_show_space_empty(bc_id window_id, bc_id space) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->ShowSpace(space, false);
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

void bc_window_set_focus_color(bc_id window_id, uint32_t color) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetFocusColor(color & 0xffffff);
  }
}

void bc_window_set_page_color(bc_id window_id, uint32_t color) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetPageColor(color & 0xffffff);
  }
}

void bc_window_page_moving(bc_id window_id, bool moving) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetPageMoving(moving);
  }
}

void bc_window_select(bc_id window_id, bc_id tab) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->Select(tab);
  }
}

bc_id bc_window_clear(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->Clear() : 0;
}

size_t bc_window_undo_clear(bc_id window_id, bc_id clear) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->UndoClear(clear) : 0;
}

bc_id bc_window_undo_delete_space(bc_id window_id, bc_id space) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->UndoDeleteSpace(space) : 0;
}

bc_id bc_window_reopen_closed_tab(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->ReopenClosedTab() : 0;
}

bc_id bc_window_reopen_tab(bc_id window_id, bc_id closed) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->ReopenClosed(closed, /*show=*/false) : 0;
}

bc_id bc_window_restore_archived(bc_id window_id, bc_id archived) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->RestoreArchived(archived) : 0;
}

void bc_extension_answer(bc_id question, bool accept) {
  browser_core::InstallQuestion::Answer(question, accept);
}

void bc_extensions_report() {
  browser_core::ReportExtensions();
}

void bc_extension_set_pinned(bc_id profile_id, const char* id, bool pinned) {
  if (Profile* profile = LoadedProfile(profile_id); profile && id) {
    browser_core::SetExtensionPinned(profile, id, pinned);
  }
}

bool bc_extension_remove(bc_id profile_id, const char* id) {
  Profile* profile = LoadedProfile(profile_id);
  return profile && id && browser_core::RemoveExtension(profile, id);
}

bool bc_window_run_extension(bc_id window_id, const char* id, bc_rect anchor) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window && id && window->RunExtension(id, ToRect(anchor));
}

bool bc_window_run_extension_command(bc_id window_id, const char* id, const char* name) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window && id && name && window->RunExtensionCommand(id, name);
}

bc_id bc_archive_clear(void) {
  return browser_core::RemoveAllArchived();
}

bc_id bc_archive_remove(bc_id archived) {
  return browser_core::RemoveArchived(archived);
}

size_t bc_archive_undo_remove(bc_id removal) {
  return browser_core::UndoRemoved(removal);
}

double bc_archive_after(void) {
  return browser_core::UserSettings().archive_after.InSecondsF();
}

void bc_set_archive_after(double seconds) {
  if (!(seconds > 0)) {
    return;
  }
  browser_core::UserSettings().archive_after = base::Seconds(seconds);
  browser_core::ScheduleAutoArchive();
  browser_core::SessionChanged();
}

bool bc_haptics(void) {
  return browser_core::UserSettings().haptics;
}

void bc_set_haptics(bool haptics) {
  browser_core::UserSettings().haptics = haptics;
  browser_core::SessionChanged();
}

bool bc_warn_before_quitting(void) {
  return browser_core::UserSettings().warn_before_quitting;
}

void bc_set_warn_before_quitting(bool warn) {
  browser_core::UserSettings().warn_before_quitting = warn;
  browser_core::SessionChanged();
}

bool bc_warn_moving_profiles(void) {
  return browser_core::UserSettings().warn_moving_profiles;
}

void bc_set_warn_moving_profiles(bool warn) {
  browser_core::UserSettings().warn_moving_profiles = warn;
  browser_core::SessionChanged();
}

void bc_page_favicon(const char* url) {
  if (url) {
    browser_core::FindFavicon(url);
  }
}

void bc_window_suggest(bc_id window_id, const char* keyword, const char* text, bool complete) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  Profile* profile = window ? ProfileOf(*window) : nullptr;
  if (window && profile) {
    window->Suggest(profile, keyword ? base::UTF8ToUTF16(keyword) : std::u16string(),
                    text ? base::UTF8ToUTF16(text) : std::u16string(), complete);
  }
}

void bc_window_remove_suggestion(bc_id window_id, const char* url) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  if (window && url) {
    window->RemoveSuggestion(GURL(url));
  }
}

bc_id bc_tab_open(bc_id window_id, const char* url, bool when_shown) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  Profile* profile = window ? ProfileOf(*window) : nullptr;
  content::WebContents* contents =
      window && profile ? window->OpenTab(profile, GURL(url), when_shown) : nullptr;
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
  Profile* profile = browser_core::StartProfile();
  const GURL typed = profile && text ? browser_core::TypedUrl(profile, text) : GURL();
  return typed.is_valid() ? CopyString(typed.spec(), url, capacity) : 0;
}

size_t bc_search_url(const char* keyword, const char* text, char* url, size_t capacity) {
  Profile* profile = browser_core::StartProfile();
  const std::u16string engine = keyword ? base::UTF8ToUTF16(keyword) : std::u16string();
  const GURL search = profile && text ? browser_core::SearchUrl(profile, engine, text) : GURL();
  return search.is_valid() ? CopyString(search.spec(), url, capacity) : 0;
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
  const GURL url = space ? space->PinnedUrl(id) : browser_core::Favorites::Url(id);
  if (!url.is_valid() || !contents) {
    return;
  }
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
    browser_core::Tab::FromWebContents(contents)->Reload(bypass_cache);
  }
}

void bc_tab_wait(bc_id id) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::Tab::FromWebContents(contents)->Wait();
  }
}

void bc_tab_clear_site(bc_id id, bool cookies) {
  browser_core::ClearSiteAndReload(id, cookies);
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

void bc_tab_element_at(bc_id id, double x, double y) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::PageCapture::GetOrCreateForWebContents(contents)->ElementAt(gfx::PointF(x, y));
  }
}

void bc_tab_capture(bc_id id, uint64_t request, bool full_page, bc_rect rect) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::PageCapture::GetOrCreateForWebContents(contents)->Capture(
        request, full_page, gfx::RectF(rect.x, rect.y, rect.width, rect.height));
  }
}

void bc_tab_capture_end(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (browser_core::PageCapture* capture =
          contents ? browser_core::PageCapture::FromWebContents(contents) : nullptr) {
    capture->End();
  }
}

void bc_tab_answer_permission(bc_id id, int answer) {
  browser_core::PermissionQuestion::Answer(id, answer);
}

void bc_tab_site_controls(bc_id id) {
  browser_core::SiteControls::Open(id);
}

void bc_site_controls_close(void) {
  browser_core::SiteControls::Close();
}

void bc_tab_set_site_permission(bc_id id, int type, int value) {
  browser_core::SiteControls::SetPermission(id, type, value);
}

void bc_tab_reset_site_permissions(bc_id id) {
  browser_core::SiteControls::Reset(id);
}

void bc_tab_picture_in_picture(bc_id id) {
  browser_core::SiteControls::PictureInPicture(id);
}

void bc_tab_show_certificate(bc_id id) {
  browser_core::SiteControls::ShowCertificate(id);
}

void bc_tab_answer_offer(bc_id id, int answer, const char* username) {
  browser_core::PasswordOffer::Answer(id, answer, username);
}

void bc_tab_jump_to_selection(bc_id id) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    contents->CenterSelection();
  }
}

void bc_tab_set_pinned(bc_id id, bool pinned) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!space || !contents) {
    return;
  }
  if (pinned) {
    space->Pin(id, &PageUrlOf);
  } else {
    space->Unpin(id);
  }
  SplitMoved(*space, id);
}

void bc_tab_move_to_favorites(bc_id id, size_t index) {
  browser_core::Window::MoveToFavorites(id, index);
}

void bc_tab_move(bc_id id, bool pinned, bc_id folder, size_t index) {
  if (browser_core::Favorites::Has(id)) {
    browser_core::Window::Unfavorite(id, pinned, folder, index);
    return;
  }
  browser_core::Space* space = browser_core::Space::Holding(id);
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!space || !contents) {
    return;
  }
  space->Move(id, pinned, folder, index, &PageUrlOf);
  SplitMoved(*space, id);
}

int bc_tab_zoom(bc_id id, int step) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  zoom::ZoomController* zoom = contents ? zoom::ZoomController::FromWebContents(contents) : nullptr;
  if (!zoom) {
    return 100;
  }
  zoom::PageZoom::Zoom(contents, static_cast<content::PageZoom>(std::clamp(step, -1, 1)));
  return zoom->GetZoomPercent();
}

void bc_tab_devtools(bc_id id, int what) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (!tab) {
    return;
  }
  // As Chrome's ⌥⌘I, ⌥⌘J and ⌥⌘C, on the Browser's active tab, which is the one its window shows.
  switch (what) {
    case BC_DEVTOOLS_CONSOLE:
      DevToolsWindow::ToggleDevToolsWindow(tab->GetBrowserWindowInterface(),
                                           DevToolsToggleAction::ShowConsolePanel(),
                                           DevToolsOpenedByAction::kConsoleShortcut);
      break;
    case BC_DEVTOOLS_INSPECT:
      DevToolsWindow::ToggleDevToolsWindow(tab->GetBrowserWindowInterface(),
                                           DevToolsToggleAction::Inspect(),
                                           DevToolsOpenedByAction::kInspectorModeShortcut);
      break;
    default:
      DevToolsWindow::ToggleDevToolsWindow(tab->GetBrowserWindowInterface(),
                                           DevToolsToggleAction::Toggle(),
                                           DevToolsOpenedByAction::kToggleShortcut);
  }
}

void bc_tab_view_source(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (contents && contents->GetController().CanViewSource()) {
    // Content opens it as a tab its page opened, which goes under it.
    contents->GetPrimaryMainFrame()->ViewSource();
  }
}

void bc_tab_print(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  // Chrome's command prints its Browser's active tab, which is the one its window shows.
  if (tab && tab->IsActivated() && chrome::CanPrint(tab->GetBrowserWindowInterface())) {
    chrome::Print(tab->GetBrowserWindowInterface());
  }
}

void bc_tab_save_page(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  if (tab && tab->IsActivated() && chrome::CanSavePage(tab->GetBrowserWindowInterface())) {
    chrome::SavePage(tab->GetBrowserWindowInterface());
  }
}

void bc_window_task_manager(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  content::WebContents* contents = window ? browser_core::Tab::FromId(window->selected()) : nullptr;
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  // Chrome's selects its Browser's active tab, the one the window shows.
  chrome::OpenTaskManager(tab ? tab->GetBrowserWindowInterface() : nullptr);
}

bc_id bc_tab_duplicate(bc_id id) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  tabs::TabInterface* tab = contents ? tabs::TabInterface::MaybeGetFromContents(contents) : nullptr;
  browser_core::Window* window =
      tab ? browser_core::Window::ForBrowser(tab->GetBrowserWindowInterface()) : nullptr;
  return window ? window->Duplicate(id) : 0;
}

void bc_download_act(bc_id download, int action) {
  browser_core::ActOnDownload(download, action);
}

void bc_downloads_clear() {
  browser_core::ClearDownloads();
}

void bc_download_icon(bc_id download) {
  browser_core::FindDownloadIcon(download);
}

size_t bc_tab_name(bc_id id, char* name, size_t capacity) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  return CopyString(contents ? browser_core::Tab::FromWebContents(contents)->name() : std::string(),
                    name, capacity);
}

size_t bc_tab_pinned_url(bc_id id, char* url, size_t capacity) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  const GURL pinned = space ? space->PinnedUrl(id) : browser_core::Favorites::Url(id);
  return CopyString(pinned.spec(), url, capacity);
}

void bc_tab_set_pinned_url(bc_id id, const char* url) {
  content::WebContents* contents = browser_core::Tab::FromId(id);
  if (!url || !contents) {
    return;
  }
  if (browser_core::Space* space = browser_core::Space::Holding(id)) {
    space->SetPinnedUrl(id, GURL(url));
  } else {
    browser_core::Favorites::SetUrl(id, GURL(url));
  }
  // Whether the page is away from it may have changed.
  browser_core::Tab::FromWebContents(contents)->StateChanged();
}

void bc_tab_rename(bc_id id, const char* name) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::Tab::FromWebContents(contents)->Rename(name);
  }
}

void bc_tab_set_icon(bc_id id, const char* icon) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::Tab::FromWebContents(contents)->SetIcon(icon ? icon : "");
  }
}

void bc_tab_set_muted(bc_id id, bool muted) {
  if (content::WebContents* contents = browser_core::Tab::FromId(id)) {
    browser_core::Tab::FromWebContents(contents)->SetMuted(muted);
  }
}

void bc_tab_close_below(bc_id id) {
  browser_core::Window::CloseBelow(id);
}

void bc_tab_move_to_space(bc_id id, bc_id space) {
  browser_core::Window::MoveToSpace(id, space);
}

void bc_tab_close(bc_id id, bool split) {
  browser_core::Space* space = browser_core::Space::Holding(id);
  if ((space && space->IsPinned(id)) || browser_core::Favorites::Has(id)) {
    browser_core::Window::Unload(id, split);
    return;
  }
  // Each closes at once unless its page keeps it, and the window shows what its Space picks after
  // the last.
  const std::vector<bc_id> tabs = space && split ? space->Panes(id) : std::vector<bc_id>{id};
  for (bc_id tab : tabs) {
    browser_core::Window::CloseTab(tab);
  }
}

size_t bc_windows(bc_id* windows, size_t capacity) {
  // Private windows are the user's alone.
  std::vector<browser_core::Window*> all = browser_core::Window::All();
  std::erase_if(all, [](const browser_core::Window* each) { return each->off_the_record(); });
  for (size_t i = 0; i < all.size() && i < capacity && windows; ++i) {
    windows[i] = all[i]->id();
  }
  return all.size();
}

bc_id bc_window_space(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->space() : 0;
}

bc_id bc_window_selected(bc_id window_id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window ? window->selected() : 0;
}

bc_id bc_tab_window(bc_id tab) {
  browser_core::Window* window = browser_core::Window::Showing(tab);
  return window ? window->id() : 0;
}

bc_id bc_window_open_private(bc_rect bounds, void* ui_view, bc_id profile_id) {
  Profile* profile = LoadedProfile(profile_id);
  browser_core::Window* window =
      profile ? browser_core::Window::OpenPrivate(ToRect(bounds), ui_view, profile) : nullptr;
  return window ? window->id() : 0;
}

void bc_window_set_side_panel_frame(bc_id window_id, bc_rect frame) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetSidePanelFrame(ToRect(frame));
  }
}

void bc_window_close_side_panel(bc_id window_id) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->CloseSidePanel();
  }
}

bool bc_window_has_side_panel(bc_id window_id, const char* id) {
  browser_core::Window* window = browser_core::Window::FromId(window_id);
  return window && id && window->HasSidePanel(id);
}

void bc_window_toggle_side_panel(bc_id window_id, const char* id) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id); window && id) {
    window->ToggleSidePanel(id);
  }
}

void bc_window_set_side_panel_width(bc_id window_id, double width) {
  if (browser_core::Window* window = browser_core::Window::FromId(window_id)) {
    window->SetSidePanelWidth(base::ClampRound(width));
  }
}
