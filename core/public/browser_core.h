// The core's API: what the app calls in Chrome's framework. Plain C, so the app binds it without
// sharing a C++ toolchain with Chromium. Everything runs on the main thread, callbacks included.

#ifndef BROWSER_CORE_PUBLIC_BROWSER_CORE_H_
#define BROWSER_CORE_PUBLIC_BROWSER_CORE_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define BC_EXPORT __attribute__((visibility("default")))

// Names a window, a Space or a tab for as long as the process runs; never reused. 0 names nothing.
typedef uint64_t bc_id;

// In points. A window's is on the screen, from the top-left of the main display; a tab's is in its
// window, from the top-left of the window's content.
typedef struct bc_rect {
  double x, y, width, height;
} bc_rect;

// Modifier keys held with a key, as flags.
#define BC_MODIFIER_SHIFT 1
#define BC_MODIFIER_CONTROL 2
// Option on the Mac.
#define BC_MODIFIER_ALT 4
// Command on the Mac.
#define BC_MODIFIER_META 8

// A key going down.
typedef struct bc_key {
  // The key as the web names it, for the keyboard's layout: "t", "T", "Enter", "ArrowLeft".
  const char* key;
  // The BC_MODIFIER_ flags held.
  uint32_t modifiers;
} bc_key;

// Where a tab's page is: what the URL field, back, forward and reload show.
typedef struct bc_tab_state {
  // The URL the tab shows: the one it's going to if the user asked for it, else its page's.
  const char* url;
  // There's a page to go back or forward to.
  bool can_go_back;
  bool can_go_forward;
  // The page is loading, so reload shows as stop. A page changing its own URL in place isn't.
  bool loading;
  // The tab has no page loaded, as a tab put back and not yet shown, or a pinned tab closed. It
  // loads once it's shown.
  bool unloaded;
  // The tab is pinned, and its page isn't the one it was pinned at. bc_tab_back_to_pinned_url
  // goes back there.
  bool away_from_pinned_url;
} bc_tab_state;

// A tab's find in page, as its find bar shows it.
typedef struct bc_find_state {
  // The find bar is open. The tab going to another page closes it, as in Chrome.
  bool open;
  // The find bar's text.
  const char* text;
  // How many matches the page has for it, or -1 before it has looked. It counts as it goes.
  int matches;
  // The selected match, from 1, or 0 for none.
  int selected;
} bc_find_state;

// One of the command bar's suggestions, as Chrome's address bar lists them.
typedef struct bc_suggestion {
  // Where it goes: a page, or a search for what was typed.
  const char* url;
  // What its row shows: a page's title, else its URL, or the words a search is for.
  const char* text;
  // What follows, dimmer: the page's URL when the row shows its title, the search engine, or "".
  const char* detail;
  // A tab already at the page, other than the one the window shows, which the row switches to
  // with bc_window_select. Else 0, as for the first suggestion, which goes where what was typed
  // goes.
  bc_id tab;
  // For the first suggestion, what to show after the text typed, selected, to complete it to this
  // suggestion's URL or title, when bc_window_suggest allowed it. Else "".
  const char* completion;
} bc_suggestion;

// What the app gives the core when it starts Chromium. Any callback may be null, and a string it's
// passed lasts only for the call.
typedef struct bc_host {
  // Passed back to every callback.
  void* context;
  // Chromium's run loop is running and the core's other calls work, bc_window_restore included.
  void (*started)(void* context);
  // Chromium is quitting. The core's calls stop working once this returns.
  void (*stopping)(void* context);
  // The window has closed, by bc_window_close, the user or quitting. Its tabs closed first.
  void (*window_closed)(void* context, bc_id window);
  // Chrome opened a tab in the window that the app didn't ask for: a page's link to a new tab, or
  // its popup or new window, which open as tabs. |opener| is the tab it came from, or 0. In a window
  // that shows a Space, it goes in its opener's Space where Chrome would put it, under the opener,
  // and the window shows it if |foreground| and the window shows that Space. Elsewhere it isn't
  // shown until bc_tab_show; show it if |foreground|, since Chrome made it the window's active tab.
  void (*tab_opened)(void* context, bc_id window, bc_id tab, bc_id opener, bool foreground);
  // The tab's page finished loading |url|.
  void (*tab_loaded)(void* context, bc_id tab, const char* url);
  // The tab's URL, its history or whether it's loading changed. A burst of changes is one call.
  void (*tab_state_changed)(void* context, bc_id tab, const bc_tab_state* state);
  // The tab's page has a new title.
  void (*tab_title_changed)(void* context, bc_id tab, const char* title);
  // The tab's page has a new favicon: a PNG |size| bytes long, 32 pixels square for 16 points at
  // 2×, or null and 0 once a page that has none has loaded. While a page loads, the tab keeps the
  // favicon it had until the new page's arrives.
  void (*tab_favicon_changed)(void* context, bc_id tab, const uint8_t* png, size_t size);
  // The tab's page went full screen, as a video's full-screen button takes it, or came back. Show
  // it filling its window while it's full screen. The window goes full screen with it unless it
  // already was, and comes back with it. Escape, or the user taking the window out of full screen,
  // brings the page back.
  void (*tab_fullscreen_changed)(void* context, bc_id tab, bool fullscreen);
  // The tab's find in page changed: its find bar opened or closed, its text changed, or the page
  // counted matches or selected another.
  void (*tab_find_changed)(void* context, bc_id tab, const bc_find_state* state);
  // The window's suggestions for |text|, the text bc_window_suggest was last given, best first.
  // The first goes where Chrome's address bar goes for what was typed. They come again as more
  // are found.
  void (*window_suggestions)(void* context, bc_id window, const char* text,
                             const bc_suggestion* suggestions, size_t count);
  // The tab has closed, by bc_tab_close, its page, its window closing or quitting.
  void (*tab_closed)(void* context, bc_id tab);
  // The Space's tabs changed: one opened, closed, moved, pinned or unpinned. Read them with
  // bc_space_pinned and bc_space_today.
  void (*space_changed)(void* context, bc_id space);
  // A Space was made, renamed or deleted. Read them with bc_spaces.
  void (*spaces_changed)(void* context);
  // The window shows another Space. Which of its tabs, if any, follows in window_selected.
  void (*window_space_shown)(void* context, bc_id window, bc_id space);
  // The window shows another tab, or none (0): the app asked, a page opened one in front, the tab
  // shown closed, or the window shows another Space, which may have no tabs.
  void (*window_selected)(void* context, bc_id window, bc_id tab);
  // A key went down while the tab's page has the keyboard, before the page gets it. Return true to
  // keep it from the page, as browsers keep their reserved shortcuts (⌘T, ⌘W, ⌘Q). Keys an input
  // method is composing with don't come here.
  bool (*tab_key_before_page)(void* context, bc_id tab, const bc_key* key);
  // The tab's page didn't keep a key, such as ⌘F on a page without a find of its own. Return true
  // if the app used it; otherwise the window's menus get it, as ⌘M does on the Mac.
  bool (*tab_key_unhandled)(void* context, bc_id tab, const bc_key* key);
} bc_host;

// Runs Chromium in this process with these arguments (Chromium's switches) and returns its exit
// code once it has quit. Call it once, on the main thread; it runs the main run loop.
BC_EXPORT int bc_main(int argc, const char** argv, const bc_host* host);

// Asks Chromium to quit. Pages get their unload handlers first, then bc_main returns.
BC_EXPORT void bc_quit(void);

// Opens a window at |bounds| with the app's UI view in it, under the pages it shows: an NSView on
// the Mac, which the window keeps. |ui_view| may be null. Returns 0 if Chromium is quitting.
BC_EXPORT bc_id bc_window_open(bc_rect bounds, void* ui_view);

// Puts back the Spaces as the user left them when the browser last quit or its window closed, in
// the window, which shows the Space it showed. The core saves them as they change, in Chrome's user
// data directory: their names, their pinned and Today tabs with each tab's history, and the tab
// each showed. A tab loads once it's shown. Does nothing and returns false if nothing was saved or
// there are Spaces already. Then spaces_changed, space_changed, each tab's tab_state_changed and
// tab_title_changed, tab_favicon_changed once a tab's favicon is found, window_space_shown and
// window_selected.
BC_EXPORT bool bc_window_restore(bc_id window);

// Closes the window: its pages get their unload handlers, then window_closed.
BC_EXPORT void bc_window_close(bc_id window);

// Makes an empty Space named |name|, in the default profile, after the others. Any window can show
// it. Then spaces_changed.
BC_EXPORT bc_id bc_space_create(const char* name);

// Copies up to |capacity| of the Spaces into |spaces|, in order, and returns how many there are.
BC_EXPORT size_t bc_spaces(bc_id* spaces, size_t capacity);

// Copies up to |capacity| bytes of the Space's name into |name|, UTF-8 and ending in a NUL, and
// returns the name's length without it.
BC_EXPORT size_t bc_space_name(bc_id space, char* name, size_t capacity);

// Renames the Space to |name|, UTF-8. Then spaces_changed.
BC_EXPORT void bc_space_set_name(bc_id space, const char* name);

// Deletes the Space if it has no tabs, pinned or Today, and isn't the only one, and returns whether
// it did. A window showing it shows the Space before it, else the one after it. Then
// spaces_changed.
BC_EXPORT bool bc_space_delete(bc_id space);

// Copies up to |capacity| of the Space's Today tabs into |tabs|, top to bottom, and returns how many
// it has. The ones the user opened are newest first; one a page opened sits under the page's tab.
BC_EXPORT size_t bc_space_today(bc_id space, bc_id* tabs, size_t capacity);

// Copies up to |capacity| of the Space's pinned tabs into |tabs|, top to bottom, and returns how
// many it has. They sit above its Today tabs.
BC_EXPORT size_t bc_space_pinned(bc_id space, bc_id* tabs, size_t capacity);

// Makes |space| the one the window shows: its tabs are the ones bc_window_select shows, and the
// window's new tabs go in it. The window shows the tab it last showed there, else the Space's top
// Today tab, else its top pinned tab, else no page. Then window_space_shown and window_selected.
BC_EXPORT void bc_window_show_space(bc_id window, bc_id space);

// Where the window shows its tab's page, in the window. Set it again when the layout changes; the
// page keeps its distance from the window's edges as the window resizes.
BC_EXPORT void bc_window_set_page_frame(bc_id window, bc_rect frame);

// Shows |tab|, a tab in the window, at its page frame, in place of the page shown before, and its
// Space if the window shows another. It takes the keyboard if that page had it. Then
// window_space_shown if the Space changed, and window_selected.
BC_EXPORT void bc_window_select(bc_id window, bc_id tab);

// Clears the Space the window shows: closes its Today tabs, as bc_tab_close does, for
// bc_window_undo_clear to bring back. If one was shown, the window shows the lowest pinned tab still
// loaded, else no page, first. Returns how many tabs it closed.
BC_EXPORT size_t bc_window_clear(bc_id window);

// Brings back the tabs the window's last bc_window_clear closed, at the top of their Space's Today
// tabs and unloaded, showing the one it closed shown. Once only, and not those
// bc_window_reopen_closed_tab brought back. Then space_changed.
BC_EXPORT void bc_window_undo_clear(bc_id window);

// Reopens the Today tab closed last, in any window, with its history: at its place in its Space's
// Today tabs, else at the top of the Space the window shows. The window shows it, and its Space.
// Tabs that close with their window or as the browser quits come back at the next launch instead.
// Returns the tab, or 0 if none is closed.
BC_EXPORT bc_id bc_window_reopen_closed_tab(bc_id window);

// Lists suggestions for |text| typed in the window's command bar, as Chrome's address bar does:
// pages from the profile's history and bookmarks, open tabs among them, and a search for |text|.
// Well-known sites |text| could start fill the rows left, under those. Nothing typed leaves the
// device, since search suggestions are off. Then window_suggestions, and
// again as more are found. Empty text lists none. |complete| lets the first suggestion complete
// |text|, which Chrome allows only while the user types at the end of the text: not after deleting
// or pasting, with the caret elsewhere, or while an input method composes.
BC_EXPORT void bc_window_suggest(bc_id window, const char* text, bool complete);

// Opens a tab in the window, loading |url|, in the default profile, at the top of the Today tabs of
// the Space it shows. It isn't shown until bc_window_select or bc_tab_show. Returns 0 if the window
// is closing or gone.
BC_EXPORT bc_id bc_tab_open(bc_id window, const char* url);

// Copies up to |capacity| bytes of the URL Chrome's address bar goes to for |text| typed in it into
// |url|, UTF-8 and ending in a NUL, and returns the URL's length without it: the page |text| names,
// else a search for it with the default search engine, DuckDuckGo until the user picks another.
// Returns 0 if |text| names neither.
BC_EXPORT size_t bc_typed_url(const char* text, char* url, size_t capacity);

// Shows the tab's page at |frame| in its window, in place of the page the window showed. It keeps
// its distance from the window's edges as the window resizes, and takes the keyboard if that page
// had it. For a window that shows no Space; one that does uses bc_window_select.
BC_EXPORT void bc_tab_show(bc_id tab, bc_rect frame);

// Loads |url| in the tab.
BC_EXPORT void bc_tab_navigate(bc_id tab, const char* url);

// Goes back, or forward, in the tab's history, if it has a page that way.
BC_EXPORT void bc_tab_go_back(bc_id tab);
BC_EXPORT void bc_tab_go_forward(bc_id tab);

// Loads the tab's page again. |bypass_cache| fetches all of it from the network, as a force reload
// does.
BC_EXPORT void bc_tab_reload(bc_id tab, bool bypass_cache);

// Stops the tab loading.
BC_EXPORT void bc_tab_stop(bc_id tab);

// Opens the tab's find bar, if it isn't open, with the text it had when it last closed, else the
// text last found in another of the profile's tabs. Text selected in the page takes its place, as
// in Chrome, unless the user has changed the text since the bar opened. Highlights the matches
// without selecting one, so the page doesn't scroll. Copies up to |capacity| bytes of the bar's
// text into |text|, UTF-8 and ending in a NUL, and returns its length without it. Then
// tab_find_changed.
BC_EXPORT size_t bc_tab_find_open(bc_id tab, char* text, size_t capacity);

// Finds |text| in the tab's page, opening its find bar if it isn't open: highlights the matches and
// selects the next, or with |forward| false the one before. Empty text finds nothing. Then
// tab_find_changed, as the page counts them.
BC_EXPORT void bc_tab_find(bc_id tab, const char* text, bool forward);

// Closes the tab's find bar. The highlights go, and the selected match stays selected. Then
// tab_find_changed.
BC_EXPORT void bc_tab_find_close(bc_id tab);

// Pins the tab in its Space at the page it's at, at the bottom of the pinned tabs, or unpins it to
// the top of the Today tabs. Then space_changed and tab_state_changed.
BC_EXPORT void bc_tab_set_pinned(bc_id tab, bool pinned);

// Loads a pinned tab's pinned URL, unless it's there already. Then tab_state_changed.
BC_EXPORT void bc_tab_back_to_pinned_url(bc_id tab);

// Closes the tab: its page gets its unload handlers, which may keep it open, then tab_closed. The
// window stays open without it. A pinned tab stays in its Space instead, and only its page goes: a
// window showing it shows the Space's top Today tab, else the nearest pinned tab still loaded, else
// no page. Then tab_state_changed, and window_selected.
BC_EXPORT void bc_tab_close(bc_id tab);

#ifdef __cplusplus
}
#endif

#endif  // BROWSER_CORE_PUBLIC_BROWSER_CORE_H_
