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
  // The page's focused element takes text, as a text field does, so editing keys such as ⌘Z are
  // its own.
  bool editing;
} bc_key;

// What's wrong with a tab's page, if anything.
#define BC_PAGE_FINE 0
// Its renderer died: it crashed, was killed, ran out of memory or never started. The page shows
// nothing until bc_tab_reload. Only while a window shows the tab: one that isn't shown, or stops
// being, is unloaded instead and loads once it's shown.
#define BC_PAGE_CRASHED 1
// Its renderer hasn't answered the user's input for Chrome's hang timeout. It may come back by
// itself; bc_tab_wait waits longer, and bc_tab_reload ends the renderer and loads the page again.
#define BC_PAGE_HUNG 2
// It's Chromium's error page for a load that failed, such as for no network: net_error says why.
// Chromium loads it again once the network is back, while it's shown.
#define BC_PAGE_NETWORK_ERROR 3

// Where a tab's page is, and its sound: what the URL field, back, forward, reload and the tab's
// speaker show.
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
  // The tab is pinned or a favorite, and its page isn't the one it was pinned or made a favorite
  // at. bc_tab_back_to_pinned_url goes back there.
  bool away_from_pinned_url;
  // The page is playing sound, muted or not, or was in the last 2 s, as Chrome's speaker shows.
  bool audible;
  // The tab's sound is muted, by bc_tab_set_muted or an extension.
  bool muted;
  // The emoji bc_tab_set_icon gave the tab to show in place of its page's favicon, UTF-8, or empty.
  const char* icon;
  // What's wrong with the page: a BC_PAGE_ value. The UI and agents read the same one.
  int trouble;
  // With BC_PAGE_NETWORK_ERROR, Chromium's net error code, such as -106 for no network, else 0.
  int net_error;
  // The system's engine shows the page, WebKit on the Mac, in place of Chromium
  // (bc_tab_set_system_engine).
  bool system_engine;
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
  // Whether bc_window_remove_suggestion can take it out of the profile's history.
  bool removable;
  // The keyword of the search engine whose site search Tab enters on this row, as Chrome's address
  // bar hints "Search <site>": what was typed names the engine's site, or the row's page does.
  // Else "".
  const char* keyword;
  // That engine's name, such as "DuckDuckGo", or "".
  const char* site;
} bc_suggestion;

// One of the archive's tabs: a Today tab that closed, as Arc's Library lists them.
typedef struct bc_archived_tab {
  // For bc_window_restore_archived.
  bc_id id;
  // The page it was at, and its title.
  const char* url;
  const char* title;
  // When it closed, in milliseconds since the Unix epoch.
  double archived;
} bc_archived_tab;

// One of an extension's keyboard commands that has a key, as Chrome's Keyboard Shortcuts page
// lists them.
typedef struct bc_extension_command {
  // Its name in the extension's manifest, for bc_window_run_extension_command, or
  // "_execute_action" for its action, which bc_window_run_extension runs.
  const char* name;
  // What it does, in the extension's words.
  const char* description;
  // Its key, as bc_key names one, and the BC_MODIFIER_ flags with it.
  const char* key;
  uint32_t modifiers;
  // Whether the key is the one the extension suggested, not one the user set.
  bool suggested;
} bc_extension_command;

// One of a profile's extensions, as Chrome's extensions page lists them.
typedef struct bc_extension {
  // Chrome's id for it, for bc_window_run_extension.
  const char* id;
  const char* name;
  // Whether it's turned on. A turned-off extension's action doesn't run.
  bool enabled;
  // Whether its action is pinned, for the windows to show (bc_extension_set_pinned).
  bool pinned;
  // The page its options are on, for a tab to open as Chrome's Options does: its own, or Chrome's
  // extensions page showing them. "" if it has none.
  const char* options;
  const bc_extension_command* commands;
  size_t command_count;
  // Whether the user may remove it (bc_extension_remove). The ad blocker the browser comes with is
  // turned off in Settings instead (bc_set_ad_blocking).
  bool removable;
} bc_extension;

// An extension's action as a window shows it, for the tab it shows.
typedef struct bc_extension_action {
  const char* id;
  // What its button's tooltip says: the title the extension gave the tab, or else its name.
  const char* title;
  // Its icon for the tab, a PNG at twice its 16-point size.
  const uint8_t* icon;
  size_t icon_size;
  // The text of its badge over the icon's bottom, often "", and the badge's colour, 0xAARRGGBB,
  // or 0 for Chrome's.
  const char* badge;
  uint32_t badge_color;
  // Whether it acts on the tab. An action that doesn't shows dimmed, as in Chrome.
  bool enabled;
} bc_extension_action;

// An extension's install, re-enable or new permissions, for the user to confirm, in Chrome's words.
typedef struct bc_extension_question {
  // For bc_extension_answer.
  bc_id id;
  // The extension's id and name.
  const char* extension;
  const char* name;
  // Its icon, a PNG of Chrome's 128-pixel install icon, or null and 0.
  const uint8_t* icon;
  size_t icon_size;
  // What Chrome's dialog says: its title, such as "Add "Name"?", the line over the warnings, such
  // as "It can:", and its buttons, such as "Add extension" and "Cancel". |accept| is "" for a
  // question with nothing to accept.
  const char* title;
  const char* heading;
  const char* accept;
  const char* cancel;
  // Chrome's permission warnings, such as "Read your browsing history", |warning_count| of them,
  // each with its details: "" or lines, one per site or device, split by "\n".
  const char* const* warnings;
  const char* const* details;
  size_t warning_count;
} bc_extension_question;

// Where a download is.
#define BC_DOWNLOAD_RUNNING 0
#define BC_DOWNLOAD_PAUSED 1
#define BC_DOWNLOAD_DONE 2
#define BC_DOWNLOAD_CANCELLED 3
#define BC_DOWNLOAD_FAILED 4

// One of the profile's downloads, as Arc's Library lists them.
typedef struct bc_download {
  // For bc_download_act and bc_download_icon.
  bc_id id;
  // Its file's name, and the URL it comes from.
  const char* name;
  const char* url;
  // Where its file is once it's done, a full path in UTF-8, or "" until that's known.
  const char* path;
  // The bytes it has, and the bytes it will have in all, or 0 while that isn't known.
  int64_t received;
  int64_t total;
  // A BC_DOWNLOAD_ state.
  int state;
  // It opens once it's done.
  bool open_when_done;
  // Chrome holds it, as a file that could harm the computer or one that came over plain HTTP,
  // until BC_DOWNLOAD_KEEP, and BC_DOWNLOAD_REMOVE deletes it.
  bool held;
  // When it started, in milliseconds since the Unix epoch.
  double started;
} bc_download;

// The values a site's permission takes in the Site Controls, as Chrome's page info words them: the
// site may, it may not, or it asks first.
#define BC_SITE_ALLOW 0
#define BC_SITE_BLOCK 1
#define BC_SITE_ASK 2

// A permission of the site of a tab's page, as Chrome's page info lists it.
typedef struct bc_site_permission {
  // Chrome's ContentSettingsType for it, which bc_tab_set_site_permission takes.
  int type;
  // Chrome's name for it, such as "Location".
  const char* name;
  // A BC_SITE_ value.
  int value;
  // The values the user may give it, a bit (1 << value) each. 0 for one that isn't the user's to
  // change, as a policy's or an extension's isn't.
  uint32_t choices;
  // Chrome's lasting name for it, such as "geolocation", which stays when the types renumber.
  const char* key;
  // Chrome's page info's words for its value under its name, such as "Allowed".
  const char* state;
} bc_site_permission;

// What the connection to a tab's page is, as the Site Controls' footer says.
#define BC_CONNECTION_SECURE 0
#define BC_CONNECTION_NOT_SECURE 1
// A file, or one of the browser's own pages.
#define BC_CONNECTION_INTERNAL 2

// What Picture in Picture can do for a tab's page: nothing, take its playing video, or bring the
// video it has back.
#define BC_PICTURE_IN_PICTURE_NONE 0
#define BC_PICTURE_IN_PICTURE_ENTER 1
#define BC_PICTURE_IN_PICTURE_EXIT 2

// Whether the ad blocker runs on the site of a tab's page, as its Site Controls show: not the
// user's to change there (a page that isn't http(s), a private window, or ad blocking off), on, or
// turned off for the site.
#define BC_AD_BLOCKING_NONE 0
#define BC_AD_BLOCKING_ON 1
#define BC_AD_BLOCKING_OFF 2

// The site of a tab's page, for its Site Controls.
typedef struct bc_site_controls {
  // The site as the user should see it, such as "example.com".
  const char* site;
  // A BC_CONNECTION_ value, and Chrome's words for it, such as "Connection is secure".
  int connection;
  const char* connection_summary;
  // Those Chrome's page info lists: the ones the site was given, has asked for or uses, and none
  // for a file or one of the browser's pages.
  const bc_site_permission* permissions;
  size_t count;
  // Chrome's words for the button that sets them back to their defaults, such as "Reset
  // permissions", or "" when it has none, as when the user set none of them.
  const char* reset;
  // A BC_PICTURE_IN_PICTURE_ value.
  int picture_in_picture;
  // The page came with a certificate, which bc_tab_show_certificate shows.
  bool certificate;
  // The profile keeps the site for the system's engine, which shows its pages
  // (bc_tab_set_system_engine). A tab showing one lists no permissions: the system's engine asks
  // for none.
  bool system_engine;
  // A BC_AD_BLOCKING_ value, which bc_tab_set_ad_blocking changes.
  int ad_blocking;
} bc_site_controls;

// An agent's call the core checked, for the activity log (bc_agent_may_act).
typedef struct bc_agent_activity {
  // The agent's id, as its client gives it.
  const char* agent;
  bc_id tab;
  // What it asked to do, as the agent code names it, such as "navigate".
  const char* action;
  // Why the core refused it, for the user and the agent to read, or empty if the agent may act.
  const char* refused;
} bc_agent_activity;

// An extension's side panel as a window shows it, beside the page of the tab it shows.
typedef struct bc_side_panel {
  // The extension's id and name.
  const char* extension;
  const char* name;
  // The extension's icon, a PNG at twice its 16-point size, or null.
  const uint8_t* icon;
  size_t icon_size;
  // The title of the panel's page.
  const char* title;
  // The width the user last gave extensions' side panels in the profile, or 0 for none yet.
  double width;
} bc_side_panel;

// What a tab's offer offers: to save the password of a login its page submitted, or to update the
// password saved for it.
#define BC_OFFER_KIND_PASSWORD 0
#define BC_OFFER_KIND_PASSWORD_UPDATE 1

// An offer from Chrome's password manager, in Chrome's words. The password stays in the core.
typedef struct bc_offer {
  // A BC_OFFER_KIND_ kind.
  int kind;
  // Such as "Save password?", and the site whose password it is, as the user should see it.
  const char* title;
  const char* site;
  // The username it saves unless the user edits it, and the values the page's other fields had
  // that might be it instead, |username_count| of them, the first that one, for a picker.
  const char* username;
  const char* const* usernames;
  size_t username_count;
  // How many characters (code points) the password has, to show it masked.
  size_t password_length;
  // The labels of its buttons for BC_OFFER_ACCEPT, BC_OFFER_NOT_NOW and BC_OFFER_NEVER, the last ""
  // where it has none, as an update.
  const char* accept;
  const char* decline;
  const char* never;
} bc_offer;

// What the app gives the core when it starts Chromium. Any callback may be null, and a string it's
// passed lasts only for the call.
typedef struct bc_host {
  // Passed back to every callback.
  void* context;
  // Chromium has made NSApp and the browser is about to start: the first moment the app can put a
  // window on screen. None of the core's calls work yet.
  void (*launching)(void* context);
  // Chromium's run loop is running and the core's other calls work, bc_window_restore included.
  void (*started)(void* context);
  // Chromium is quitting. The core's calls stop working once this returns.
  void (*stopping)(void* context);
  // The window has closed, by bc_window_close, the user or quitting. With other windows open, its
  // tabs went to the one active before it, their pages live. Else they closed first.
  void (*window_closed)(void* context, bc_id window);
  // Chrome opened a tab in the window that the app didn't ask for: a page's link to a new tab, or
  // its popup or new window, which open as tabs. |opener| is the tab it came from, or 0. In a window
  // that shows a Space, it goes in its opener's Space where Chrome would put it, under the opener,
  // and the window shows it if |foreground| and the window shows that Space. A link ⌥⌘- or
  // ⌥⇧-clicked goes in a split view with its page instead, unless the split has three tabs; then
  // space_changed. Elsewhere it isn't shown until bc_tab_show; show it if |foreground|, since
  // Chrome made it the window's active tab.
  void (*tab_opened)(void* context, bc_id window, bc_id tab, bc_id opener, bool foreground);
  // The tab's page finished loading |url|.
  void (*tab_loaded)(void* context, bc_id tab, const char* url);
  // The tab's URL, its history or whether it's loading changed. A burst of changes is one call.
  void (*tab_state_changed)(void* context, bc_id tab, const bc_tab_state* state);
  // The tab has a new title: its page's, or the name bc_tab_rename gave it.
  void (*tab_title_changed)(void* context, bc_id tab, const char* title);
  // The tab's page has a new favicon: a PNG |size| bytes long, 32 pixels square for 16 points at
  // 2×, or null and 0 once a page that has none has loaded. While a page loads, the tab keeps the
  // favicon it had until the new page's arrives.
  void (*tab_favicon_changed)(void* context, bc_id tab, const uint8_t* png, size_t size);
  // The tab's page went full screen, as a video's full-screen button takes it, or came back. Show
  // it filling its window while it's full screen. The window goes full screen with it unless it
  // already was, and comes back with it if it went for it, after this call either way, so the
  // window's resize can lay the page out. Escape, the user taking the window out of full screen,
  // and switching or closing the tab bring the page back; the last two report it for a tab no
  // longer shown, or gone. Each change is reported once.
  void (*tab_fullscreen_changed)(void* context, bc_id tab, bool fullscreen);
  // The tab's find in page changed: its find bar opened or closed, its text changed, or the page
  // counted matches or selected another.
  void (*tab_find_changed)(void* context, bc_id tab, const bc_find_state* state);
  // The tab's page asks to use something, such as the camera: |site| is who asks, as the user
  // should see it, and |permissions| what it asks for, |count| of them, each a phrase that follows
  // "wants to", such as "Use your camera". Ask the user while the tab is shown, and answer with
  // bc_tab_answer_permission. It comes again with |count| 0 once the question has gone: answered,
  // or the tab went to another page, closed or was no longer shown. Chrome keeps the request of a
  // tab no longer shown, and it comes again once the tab is.
  void (*tab_permission_asked)(void* context,
                               bc_id tab,
                               const char* site,
                               const char* const* permissions,
                               size_t count);
  // The window's suggestions for |text|, the text bc_window_suggest was last given, after its
  // keyword and a space if it had one, best first. The first goes where Chrome's address bar goes
  // for what was typed. They come again as more are found.
  void (*window_suggestions)(void* context, bc_id window, const char* text,
                             const bc_suggestion* suggestions, size_t count);
  // The tab has closed, by bc_tab_close, its page, its window closing or quitting.
  void (*tab_closed)(void* context, bc_id tab);
  // The Space's tabs changed: one opened, closed, moved, pinned or unpinned. Read them with
  // bc_space_pinned and bc_space_today.
  void (*space_changed)(void* context, bc_id space);
  // A Space was made, renamed, deleted or given another profile, or a profile was made, renamed or
  // deleted. Read them with bc_spaces and bc_profiles.
  void (*spaces_changed)(void* context);
  // A tab was made a favorite, moved among them or taken out of them, or the favorites' scope
  // changed. Read them with bc_favorites.
  void (*favorites_changed)(void* context);
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
  // The archive changed, or what the last launch archived was read: |tabs| is all of it, newest
  // first.
  void (*archive_changed)(void* context, const bc_archived_tab* tabs, size_t count);
  // The favicon bc_page_favicon asked for, for |url| as the app wrote it: a PNG |size| bytes long,
  // 32 pixels square for 16 points at 2×, or null and 0 if the profile's history has none.
  void (*page_favicon)(void* context, const char* url, const uint8_t* png, size_t size);
  // The profile's downloads changed, or the ones Chrome kept from the last launch were read:
  // |downloads| is all of them, newest first. While one runs, about twice a second.
  void (*downloads_changed)(void* context, const bc_download* downloads, size_t count);
  // The icon bc_download_icon asked for: a PNG |size| bytes long, 32 pixels square for 16 points
  // at 2×, or null and 0 if the system has none.
  void (*download_icon)(void* context, bc_id download, const uint8_t* png, size_t size);
  // Chrome asked to show |window|'s downloads, such as after the user stopped a quit to let them
  // finish. In place of Chrome's downloads page, which the window never opens.
  void (*window_show_downloads)(void* context, bc_id window);
  // The profile's extensions changed, or were first read: one was added, removed, turned on or
  // off, pinned or unpinned, a command's key changed, or bc_extensions_report asked. |extensions|
  // is all of them that Chrome's extensions page lists, by name.
  void (*extensions_changed)(void* context,
                             bc_id profile,
                             const bc_extension* extensions,
                             size_t count);
  // An extension's popup showed over the window's page, |extension| being its id, or the popup
  // shown went, and |extension| is "". Chrome's popup goes when it loses the keyboard, when the
  // window shows another tab, or when its page closes it.
  void (*window_extension_popup)(void* context, bc_id window, const char* extension);
  // The actions the window shows changed: those pinned in the profile of the tab it shows, in
  // their order, as they are for that tab. Once the tab shown changes, if they're different for it.
  void (*window_extension_actions)(void* context,
                                   bc_id window,
                                   const bc_extension_action* actions,
                                   size_t count);
  // The private session of |profile| ended: its last private window closed, and Chrome destroyed
  // the off-the-record profile they shared, with its cookies, cache and the rest. Not at quit. The
  // user's alone: agents hear none of it.
  void (*private_session_ended)(void* context, bc_id profile);
  // The site of the tab's page, for its Site Controls, after bc_tab_site_controls, and again as it
  // changes, until bc_site_controls_close: a permission changed or was used, the tab went to
  // another page, or a video started or stopped.
  void (*tab_site_controls)(void* context, bc_id tab, const bc_site_controls* controls);
  // The box of the element at (|x|, |y|) bc_tab_element_at asked for, its border's in points from
  // the top-left of the page's view, or a box 0 wide if there's none or the page went.
  void (*tab_element_box)(void* context, bc_id tab, double x, double y, bc_rect box);
  // The picture bc_tab_capture asked for as |request|: a PNG |size| bytes long, |width| by
  // |height| pixels, or null and 0 if the page couldn't give one.
  void (*tab_captured)(void* context,
                       bc_id tab,
                       uint64_t request,
                       const uint8_t* png,
                       size_t size,
                       int width,
                       int height);
  // The user chose Open Link in New Window or Open Link in Incognito Window in the page menu of a
  // tab in |window|. Open |url| in a window as ⌘N's, or with |private_window| in a private one in
  // |profile|'s session, as ⇧⌘N's. A link from a private window asks for a private one either way,
  // as Chrome's does. The user's alone: agents have no page menu.
  void (*window_open_link)(void* context,
                           bc_id window,
                           const char* url,
                           bc_id profile,
                           bool private_window);
  // The page of |tab|, the tab |window| has selected, went to another window, which shows it live,
  // and |window| shows no page; or |tab| is 0, and |window| shows a page again, or has selected
  // another tab. Once for each change. window_selected doesn't say it: the tab stays selected.
  // bc_tab_capture still gives a picture of the page, for the window to show in its place. The
  // user's alone: agents have no windows.
  void (*window_page_elsewhere)(void* context, bc_id window, bc_id tab);
  // Chrome asks the user to confirm an extension's install, its re-enabling once it asks for more
  // permissions, or more permissions it asks for, over the window, in place of its dialog. Answer
  // with bc_extension_answer. A window asks one at a time, and this comes again once the question
  // has gone, answered or because the tab that asked closed, with the window's next, or with
  // |question| null if it has none. Chrome hears none was given if the window closes first.
  void (*window_extension_question)(void* context,
                                    bc_id window,
                                    const bc_extension_question* question);
  // The side panel the window shows changed: an extension opened or closed it, its page's title
  // or the extension's icon changed, or the window shows a tab with another panel or none. |panel|
  // is null for none. Once the tab shown changes, if it's different for it.
  void (*window_side_panel)(void* context, bc_id window, const bc_side_panel* panel);
  // A page of |tab| asked for a protected-media key system ilo can't play, such as Widevine, which
  // Netflix and Spotify play their video with: |key_system| is "com.widevine.alpha" or
  // "com.microsoft.playready", never the page's own string. Once for each key system a document
  // asks for, unless it's the one the app was last told. It comes with an empty |key_system| once
  // the tab goes to another site, or once bc_tab_set_system_engine switches it: the note it asks
  // for is gone. bc_tab_set_system_engine answers it where bc_system_engine_available. Agents get
  // none: the note hands the user's page to another engine or app.
  void (*tab_media_unsupported)(void* context, bc_id tab, const char* key_system);
  // The tab's offer changed: Chrome's password manager offers |offer|, or with null, it no longer
  // does: it was answered, or another tab was shown or the tab closed. Show it while the tab is,
  // and answer with bc_tab_answer_offer. A tab shown again doesn't get it back. The user's alone:
  // agents hear none, as it names the user's login.
  void (*tab_offer_changed)(void* context, bc_id tab, const bc_offer* offer);
  // Another app asked the browser to open |urls|, |count| http, https or file URLs: a link clicked
  // in Mail or Slack, `open https://…`, an .html file from Finder. Open each in a new tab of the
  // user's, in front. Those that launched the browser come before started; open them over the
  // Spaces bc_window_restore brings back. The user's alone: a link a page opens, an agent's too,
  // comes in tab_opened, and agents get no way to make the browser the system's default, which
  // only the user does, in the app's Settings or the system's.
  void (*open_links)(void* context, const char* const* urls, size_t count);
  // The pointer is on a link in the page of |tab|, or has left it and |url| is "". |url| is the
  // link's address as the app shows it: UTF-8, without https://, and a name in another script as
  // Chrome's address bar shows it, which is as ASCII if it could pass for another site's. The
  // bytes [|domain_begin|, |domain_end|) of it are the host's registrable domain, such as
  // example.co.uk in login.example.co.uk, or the whole host if it has no registry; both are 0 for
  // an address with no host. Told as the link changes, and "" when the page navigates away, never
  // polled. The user's alone: agents have no pointer.
  void (*tab_link_target)(void* context,
                          bc_id tab,
                          const char* url,
                          size_t domain_begin,
                          size_t domain_end);
  // An entry for the activity log: the core checked an agent's call with bc_agent_may_act, and
  // let it act or refused it.
  void (*agent_activity)(void* context, const bc_agent_activity* entry);
} bc_host;

// Runs Chromium in this process with these arguments (Chromium's switches) and returns its exit
// code once it has quit. Call it once, on the main thread; it runs the main run loop.
BC_EXPORT int bc_main(int argc, const char** argv, const bc_host* host);

// Asks Chromium to quit. Pages get their unload handlers first, then bc_main returns.
BC_EXPORT void bc_quit(void);

// Opens a window at |bounds| with the app's UI view in it, under the pages it shows: an NSView on
// the Mac, which the window keeps. |ui_view| may be null. Returns 0 if Chromium is quitting. Every
// window shows the same Spaces. The user's: agents get none, as it changes what the user sees.
BC_EXPORT bc_id bc_window_open(bc_rect bounds, void* ui_view);

// Puts back the favorites and the Spaces as the user left them when the browser last quit or its
// window closed, in the window, which shows the Space it showed. The core saves them as they
// change, in Chrome's user data directory: the favorites, the Spaces' names and whose they are,
// their pinned and Today tabs with each tab's history, and the tab each showed. A tab loads once
// it's shown. Does nothing and returns false if nothing was saved or there are Spaces already.
// Then favorites_changed, spaces_changed, space_changed, each tab's tab_state_changed and
// tab_title_changed, tab_favicon_changed once a tab's favicon is found, window_space_shown and
// window_selected.
BC_EXPORT bool bc_window_restore(bc_id window);

// Closes the window. With other windows open, its tabs go, their pages live, to the one active
// before it, which shows its tab live again if it showed one this window took. Else its pages get
// their unload handlers. Then window_closed. The user's: agents get none.
BC_EXPORT void bc_window_close(bc_id window);

// Makes an empty Space named |name|, in the profile the core started with, after the others. Any
// window can show it. It's the user's. Then spaces_changed.
BC_EXPORT bc_id bc_space_create(const char* name);

// A Space is the user's or an agent's, from when it's made for as long as it lives, and saved with
// it (decision 8). An agent is an AI client, such as Claude Code, driving the browser through the
// core. The rules its calls keep, once there are any:
// - It works only in its own Space: bc_agent_may_act answers which tabs are its to act on. The
//   user's tabs and favorites never are, and no tab moves into an agent's Space, by
//   bc_tab_move_to_space or bc_tab_move, until the user can give an agent a tab, with its
//   activity-log entry. A tab the user opens in one, while a window shows it, is the agent's like
//   the rest.
// - Its actions never move the user's focus: no window shows another Space or tab, and no page
//   takes the keyboard, because of one.
// - The user can watch it, as an agent's Space shows in the windows like the others, take over in
//   it, and hand back. To keep what an agent made, the user moves its tabs into a Space of theirs.
// An agent's Space shows, switches, renames, clears and deletes as the user's Spaces do, and
// bc_window_undo_delete_space brings it back the agent's.

// Makes an empty Space named |name| for the agent with the id |agent| and the name |agent_name|,
// UTF-8, as its client gives them, in the profile the core started with, after the others.
// bc_space_set_profile puts it in another. The user's actions never call this: New Space makes the
// user's. Returns 0, making none, if |agent| or |agent_name| is empty. Then spaces_changed.
BC_EXPORT bc_id bc_space_create_for_agent(const char* name,
                                          const char* agent,
                                          const char* agent_name);

// Copies the id of the agent the Space belongs to into |agent| as bc_space_name does, or nothing,
// an empty string, for the user's own. It's the one the agent's client gave, stable across its
// connections.
BC_EXPORT size_t bc_space_agent(bc_id space, char* agent, size_t capacity);

// Copies the name of the agent the Space belongs to, for the user to see, into |name| as
// bc_space_name does, or an empty string for the user's own Space.
BC_EXPORT size_t bc_space_agent_name(bc_id space, char* name, size_t capacity);

// Whether the agent with the id |agent| may do |action| to |tab|: only if the tab is in that
// agent's own Space, and Chromium shows its page, not the system's engine, where the browser can't
// keep an agent's rules. Agent code calls it once for each call an agent makes on a tab, before
// acting. If refused, copies why into |reason| as bc_space_name does, else an empty string. Either
// way, then agent_activity, for the activity log.
BC_EXPORT bool bc_agent_may_act(const char* agent,
                                bc_id tab,
                                const char* action,
                                char* reason,
                                size_t capacity);

// The profile the Space's tabs open in, which its favorites are in.
BC_EXPORT bc_id bc_space_profile(bc_id space);

// Puts the Space in |profile|, loading it first if it isn't, and returns whether it will: only an
// empty Space changes, as its tabs' pages are in their profile. A window showing a favorite the
// Space no longer shows shows nothing. Under Each Space its favorites open again in |profile|, and
// the old tabs close. Then spaces_changed, and favorites_changed for favorites.
BC_EXPORT bool bc_space_set_profile(bc_id space, bc_id profile);

// Copies up to |capacity| of the profiles into |profiles| and returns how many there are: the one
// the core started with, then the others in the order they were made. A profile holds cookies,
// history, logins and extensions, which the Spaces in it share (decision 5).
BC_EXPORT size_t bc_profiles(bc_id* profiles, size_t capacity);

// Copies the profile's name into |name| as bc_space_name does. "Default" for one no one named.
BC_EXPORT size_t bc_profile_name(bc_id profile, char* name, size_t capacity);

// Names the profile |name|, UTF-8, and returns whether it did: not with an empty name. Then
// spaces_changed.
BC_EXPORT bool bc_profile_rename(bc_id profile, const char* name);

// Makes a profile named |name|, UTF-8, and returns it, or 0. It's listed once Chrome has made it,
// then spaces_changed.
BC_EXPORT bc_id bc_profile_create(const char* name);

// Deletes the profile, with everything Chrome keeps in it, and returns whether it will. Not one a
// Space or a favorite is in, the one the core started with or the last. Then spaces_changed.
BC_EXPORT bool bc_profile_delete(bc_id profile);

// Copies up to |capacity| of the Spaces into |spaces|, in order, and returns how many there are.
BC_EXPORT size_t bc_spaces(bc_id* spaces, size_t capacity);

// Copies up to |capacity| bytes of the Space's name into |name|, UTF-8 and ending in a NUL, and
// returns the name's length without it.
BC_EXPORT size_t bc_space_name(bc_id space, char* name, size_t capacity);

// Renames the Space to |name|, UTF-8. Then spaces_changed.
BC_EXPORT void bc_space_set_name(bc_id space, const char* name);

// Puts the Space's colour, 0xRRGGBB in sRGB, in |color|, and returns true. False if it has none,
// and the window follows the system's accent colour.
BC_EXPORT bool bc_space_color(bc_id space, uint32_t* color);

// Gives the Space |color|, 0xRRGGBB in sRGB, or no colour if |has_color| is false. Then
// spaces_changed.
BC_EXPORT void bc_space_set_color(bc_id space, bool has_color, uint32_t color);

// Copies up to |capacity| bytes of the Space's icon into |icon| as bc_space_name does: what
// bc_space_set_icon gave it, UTF-8, or empty.
BC_EXPORT size_t bc_space_icon(bc_id space, char* icon, size_t capacity);

// Gives the Space |icon|, UTF-8, kept as its name is, for the UI to show in place of its dot and
// before its name: an emoji, or "glyph:" and the name of one of the UI's glyphs, such as
// "glyph:star". The core keeps it as it comes; empty takes the icon away. Then spaces_changed.
BC_EXPORT void bc_space_set_icon(bc_id space, const char* icon);

// Deletes the Space, unless it's the last, and returns whether it will. A window showing it shows
// the Space before it, else the one after it. Its tabs close, and it goes with the last of them; a
// page that keeps its tab open keeps the Space until it closes. Its Today tabs reopen with
// bc_window_reopen_closed_tab, in the Space shown, and the Space with
// bc_window_undo_delete_space. Under BC_FAVORITES_SPACE its favorites close with it and come back
// with it. Then spaces_changed.
BC_EXPORT bool bc_space_delete(bc_id space);

// Copies up to |capacity| of the Space's Today tabs into |tabs|, top to bottom, and returns how many
// it has. The ones the user opened are newest first; one a page opened sits under the page's tab.
BC_EXPORT size_t bc_space_today(bc_id space, bc_id* tabs, size_t capacity);

// Copies up to |capacity| of the Space's pinned tabs and folders into |tabs|, top to bottom, and
// returns how many it has. Each folder is followed by what's in it. They sit above its Today tabs.
BC_EXPORT size_t bc_space_pinned(bc_id space, bc_id* tabs, size_t capacity);

// Whether |id| is a folder, and if it is, whether it shows what's in it in |open|.
BC_EXPORT bool bc_folder(bc_id id, bool* open);

// Copies the folder's name into |name| as bc_space_name does.
BC_EXPORT size_t bc_folder_name(bc_id folder, char* name, size_t capacity);

// The folder holding the pinned tab or folder |id|, else 0.
BC_EXPORT bc_id bc_folder_of(bc_id id);

// Makes an open folder named |name| at |index| in |folder| if it isn't 0, else in the Space's
// pinned tabs, counted as bc_tab_move counts, or at the end past it. Returns it. Then
// space_changed.
BC_EXPORT bc_id bc_folder_create(bc_id space, const char* name, bc_id folder, size_t index);

// Then space_changed.
BC_EXPORT void bc_folder_rename(bc_id folder, const char* name);
BC_EXPORT void bc_folder_set_open(bc_id folder, bool open);

// Moves the folder, with what's in it, to |index| in |folder| if it isn't 0, else in the pinned
// tabs, counted as bc_tab_move counts. Not into itself. Then space_changed.
BC_EXPORT void bc_folder_move(bc_id folder, bc_id to, size_t index);

// Deletes the folder, leaving what was in it where it was, in the folder that held it. Then
// space_changed.
BC_EXPORT void bc_folder_delete(bc_id folder);

// Where the favorites are kept, as the user chose: who shows the same ones above their own tabs. A
// favorite is a site, and each profile that shows it has a tab of its own for it, in no Space,
// which a Space opens in its own profile.
// - BC_FAVORITES_EVERY: every Space shows the same sites, whatever its profile.
// - BC_FAVORITES_PROFILE: the Spaces of one profile show the same favorites, as Arc's do. The
//   default.
// - BC_FAVORITES_SPACE: every Space has its own.
#define BC_FAVORITES_EVERY 0
#define BC_FAVORITES_PROFILE 1
#define BC_FAVORITES_SPACE 2

// Copies up to |capacity| of the favorites into |tabs|, and returns how many there are: every
// profile's, and under BC_FAVORITES_SPACE every Space's. A Space shows those that are its profile's
// or, under BC_FAVORITES_SPACE, its own (bc_favorite_space), above its own tabs, in their order
// among them.
BC_EXPORT size_t bc_favorites(bc_id* tabs, size_t capacity);

// The Space the favorite belongs to, under BC_FAVORITES_SPACE, which alone shows it. Else, or for a
// tab that isn't a favorite, 0: every Space of the favorite's profile (bc_tab_profile) shows it.
BC_EXPORT bc_id bc_favorite_space(bc_id tab);

// The scope of the favorites, a BC_FAVORITES_ value, and changes it, which the core keeps with the
// Spaces. Switching never loses a favorite. Narrowing gives each Space or profile its own copy, its
// tabs unloaded but for those already loaded, which stay with one Space. Widening merges the lists
// by URL: the list the Space the window active last shows first, then each other list's extras in
// the order of the Spaces, and a duplicate's extra tabs close, keeping a loaded one over an
// unloaded one. A window showing a favorite its Space no longer shows shows the Space's top tab
// instead. Then favorites_changed. The user's preference: agents have no call to change it, and
// the calls they have for favorites follow the scope.
BC_EXPORT int bc_favorites_scope(void);
BC_EXPORT void bc_set_favorites_scope(int scope);

// The profile the tab's page is in, or 0.
BC_EXPORT bc_id bc_tab_profile(bc_id tab);

// Copies up to |capacity| of the tabs shown with |tab| into |tabs|, left to right or top to
// bottom, |tab| among them, and returns how many there are: those of its split view, else |tab|
// alone. 0 for a tab in no Space. A split's tabs sit together in their list, in this order, as one
// row.
BC_EXPORT size_t bc_tab_split(bc_id tab, bc_id* tabs, size_t capacity);

// Puts |tab| in a split view with |with|, at its left end if |left|, else its right end, making
// one if |with| is in none: a window showing either shows them side by side. |tab| leaves any
// split it was in and moves into the split's place in its list, pinned at its page if |with| is
// pinned. Returns false, doing nothing, if |with|'s split has three tabs already, or the two are
// in different Spaces. Then space_changed.
BC_EXPORT bool bc_tab_split_with(bc_id tab, bc_id with, bool left);

// Takes |tab| out of its split view, to just below the tabs left in it. A window showing it shows
// it alone. The split goes once one tab is left. Then space_changed.
BC_EXPORT void bc_tab_separate(bc_id tab);

// Moves |tab| to |index| in its split view, counted from the left: a window showing it shows the
// pages in their new order. Returns false, doing nothing, if |tab| is in no split view or |index|
// is past its end. Then space_changed.
BC_EXPORT bool bc_tab_move_in_split(bc_id tab, size_t index);

// Whether |tab|'s split view shows its pages stacked, top to bottom, rather than side by side.
BC_EXPORT bool bc_tab_split_stacked(bc_id tab);

// Stacks the pages of |tab|'s split view top to bottom, or puts them side by side: a window
// showing it shows them so, in the same order. A split view starts side by side. Returns false,
// doing nothing, if |tab| is in no split view. Then space_changed.
BC_EXPORT bool bc_tab_set_split_stacked(bc_id tab, bool stacked);

// The share of its split view's width, or height stacked, that |tab|'s page takes with the 10 pt
// gap after it, so a split's shares add up to 1. They start even, and are again whenever a tab
// joins or leaves the split. 0 if |tab| is in no split view.
BC_EXPORT double bc_tab_split_share(bc_id tab);

// Gives the pages of |tab|'s split view |shares|, |count| of them in its order, scaled to add up
// to 1, or even shares if |shares| is null: a window showing it lays it out so. Returns false,
// doing nothing, if |tab| is in no split view, or |shares| aren't one above 0 for each page. Then
// space_changed.
BC_EXPORT bool bc_tab_set_split_shares(bc_id tab, const double* shares, size_t count);

// Makes |space| the one the window shows: its tabs are the ones bc_window_select shows, and the
// window's new tabs go in it. The window shows the tab it last showed there, else the Space's top
// Today tab, else its top pinned tab, else no page, passing over tabs another window shows. Then
// window_space_shown and window_selected.
BC_EXPORT void bc_window_show_space(bc_id window, bc_id space);

// The width the app last gave the window's docked sidebar, or 0 for none. The core keeps it with
// the Spaces, and a window opens with the one the last launch saved.
BC_EXPORT double bc_window_sidebar_width(bc_id window);
BC_EXPORT void bc_window_set_sidebar_width(bc_id window, double width);

// The tab bc_window_show_space would show for |space|, or the tab shown for the Space shown. 0 for
// none.
BC_EXPORT bc_id bc_window_space_tab(bc_id window, bc_id space);

// Where the window shows its tab's page, in the window, and the radius of the page's corners, 0 for
// square ones. Set it again when the layout changes; the page keeps its distance from the window's
// edges as the window resizes. A split view's pages share it, 10 apart, each as wide as the others
// side by side, or as tall stacked.
BC_EXPORT void bc_window_set_page_frame(bc_id window, bc_rect frame, double corner_radius);

// The colour of the ring inside the edge of the focused page of a split view, 0xRRGGBB.
BC_EXPORT void bc_window_set_focus_color(bc_id window, uint32_t color);

// What the window's pages show until they draw their first frame, in place of white, 0xRRGGBB.
BC_EXPORT void bc_window_set_page_color(bc_id window, uint32_t color);

// Whether the app is moving the window's page frame on its own, as the sidebar hides or shows or
// the gap in a split view drags: each step is a new size for the pages. Chrome shows a page that
// hasn't drawn at its new size yet at its old size in the top-left corner, so its right edge and
// scrollbar trail the frame's and snap. While moving, the pages shown, their docked DevTools and
// the side panel's page show that frame stretched to fill the view instead. Call it with true
// before the motion's first bc_window_set_page_frame or bc_window_set_split_shares, and false after
// its last. A page full screen keeps Chrome's way.
BC_EXPORT void bc_window_page_moving(bc_id window, bool moving);

// Shows |tab| at the window's page frame, in place of the page shown before, and its Space if the
// window shows another. It takes the keyboard if that page had it. A tab in a split view shows with
// the rest of its split, focused. A click in another page of the split focuses that one. A page
// another window shows moves here live, with its split; that window keeps the tab selected and
// shows no page until it shows a tab again. Then window_space_shown if the Space changed, and
// window_selected. For a tab another window shows, the user's: agents get none.
BC_EXPORT void bc_window_select(bc_id window, bc_id tab);

// Clears the Space the window shows: closes its Today tabs but those any window shows, as
// bc_tab_close does, for bc_window_undo_clear to bring back. While it shows every Today tab, a tab
// alone or a split that is all of Today, it closes those too, and the window shows the tab the
// close rule picks. Returns the id of the Clear, for bc_window_undo_clear, or 0 if it closed none.
// The window keeps its last ten.
BC_EXPORT bc_id bc_window_clear(bc_id window);

// Brings back the tabs the window's bc_window_clear |clear| closed, at their places in their
// Space's Today tabs and unloaded, once. Not those bc_window_reopen_closed_tab brought back. Then
// space_changed. If the clear closed the tab shown, it shows that tab again, with its split, and
// its Space, then window_selected. Returns how many it brought back, or 0 if that clear is gone:
// it was undone, or ten others came after it, or it was another window's.
BC_EXPORT size_t bc_window_undo_clear(bc_id window, bc_id clear);

// Brings back the Space |space|, which bc_space_delete deleted, with its id at its place among the
// Spaces, its name, colour, folders and pinned tabs, and the Today tabs that went to the archive
// and haven't come back some other way, all unloaded and leaving the archive. The window shows it,
// at the tab it showed there. The last 25 deleted, by any window, can come back, in any order. Not
// while a page keeps one of its tabs open. Then spaces_changed and window_space_shown. Returns the
// Space, or 0 if it can't: it is back, or 25 others were deleted since.
BC_EXPORT bc_id bc_window_undo_delete_space(bc_id window, bc_id space);

// Reopens the Today tab closed last, in any window, with its history: at its place in its Space's
// Today tabs, else at the top of the Space the window shows. One closed from a split view goes back
// to its place in it, if a tab of the split is open and it has room. The window shows it, and its
// Space, and it leaves the archive.
// Tabs that close with their window or as the browser quits come back at the next launch instead.
// Returns the tab, or 0 if none is closed.
BC_EXPORT bc_id bc_window_reopen_closed_tab(bc_id window);

// As bc_window_reopen_closed_tab for the Today tab that had the id |closed| when it closed, rather
// than the one closed last, so a tab closed before another can come back first. It doesn't show
// the tab: the one shown stays, so several can come back behind it, and the caller shows the one
// that was shown when it closed. Returns the new tab's id, which is not |closed|, or 0 if that tab
// isn't among those that reopen: it came back already, or 25 others closed since.
BC_EXPORT bc_id bc_window_reopen_tab(bc_id window, bc_id closed);

// Reopens the archived tab at the top of the Today tabs of the Space the window shows, unloaded,
// and shows it: with its history if it closed since the browser started, else at its page. It
// leaves the archive. Returns the tab, or 0 if the archive has no such tab. Then space_changed,
// archive_changed and window_selected.
BC_EXPORT bc_id bc_window_restore_archived(bc_id window, bc_id archived);

// Runs the extension's action on the tab the window shows, as a click on its button in Chrome's
// toolbar does: the extension is granted the tab, and its popup opens, hanging from |anchor| in the
// window, or its action.onClicked fires. Returns false if it didn't run: the window shows no tab,
// or the extension isn't turned on in the tab's profile. Then window_extension_popup, for a popup.
BC_EXPORT bool bc_window_run_extension(bc_id window, const char* id, bc_rect anchor);

// Runs the extension's keyboard command |name| on the tab the window shows, as its key does in
// Chrome: the extension is granted the tab, and its commands.onCommand hears the name and the tab.
// Returns false if it didn't run: the window shows no tab, the extension isn't turned on in the
// tab's profile, or it isn't listening.
BC_EXPORT bool bc_window_run_extension_command(bc_id window, const char* id, const char* name);

// Pins the extension's action in the profile, after the others, or unpins it, as Chrome's
// extensions menu does. Then extensions_changed and window_extension_actions.
BC_EXPORT void bc_extension_set_pinned(bc_id profile, const char* id, bool pinned);

// Removes the extension from the profile, with the data it keeps, as Chrome's Remove does once the
// user confirms, so the app asks first. Returns false if it can't be removed, as one that policy
// installed or the ad blocker the browser comes with. Then extensions_changed.
BC_EXPORT bool bc_extension_remove(bc_id profile, const char* id);

// Answers the question window_extension_question put, accepting it or not, if it's still asked:
// the extension installs, is turned on again or gets the permissions, or not. Then
// window_extension_question, and extensions_changed once the extension is added or turned on.
// The user's: agents get none, as an extension can read what the user sees and does on the sites
// it's given.
BC_EXPORT void bc_extension_answer(bc_id question, bool accept);

// Tells the app again of the extensions of each profile the core uses, since the first reports may
// come before it can take them. Then extensions_changed, for each.
BC_EXPORT void bc_extensions_report(void);

// Empties the archive. Tabs closed since the browser started still reopen with
// bc_window_reopen_closed_tab. Then archive_changed. Returns the removal's id, for
// bc_archive_undo_remove, or 0 if the archive was empty.
BC_EXPORT bc_id bc_archive_clear(void);

// Takes the archived tab out of the archive, as bc_archive_clear does with all of them. Its tab,
// if it closed since the browser started, still reopens with bc_window_reopen_closed_tab. Then
// archive_changed, unless the archive has no such tab. Returns the removal's id, as
// bc_archive_clear does, or 0 if it has none.
BC_EXPORT bc_id bc_archive_remove(bc_id archived);

// Puts back what the bc_archive_remove or bc_archive_clear that returned |removal| took out, each
// tab with its id where it was. The last 25, by any window, can come back, in any order. Then
// archive_changed. Returns how many tabs it put back, or 0 if that removal is gone.
BC_EXPORT size_t bc_archive_undo_remove(bc_id removal);

// How long, in seconds, a Today tab no window shows waits before it goes in the archive: 12 hours,
// else the time the user chose. Setting it archives at once the tabs that have waited that long.
// The core keeps it with the Spaces.
BC_EXPORT double bc_archive_after(void);
BC_EXPORT void bc_set_archive_after(double seconds);

// Whether the app taps the trackpad at its detents, as the user chose. The core keeps it with the
// Spaces.
BC_EXPORT bool bc_haptics(void);
BC_EXPORT void bc_set_haptics(bool haptics);
// Whether the app asks before the user's Quit quits, as the user chose. The core keeps it with the
// Spaces.
BC_EXPORT bool bc_warn_before_quitting(void);
BC_EXPORT void bc_set_warn_before_quitting(bool warn);
// Whether the app warns before moving a tab to another profile's Space, which it does once, as Dia
// does. The core keeps it with the Spaces.
BC_EXPORT bool bc_warn_moving_profiles(void);
BC_EXPORT void bc_set_warn_moving_profiles(bool warn);
// Whether the browser blocks ads and trackers, in every profile and its private windows, with the
// uBlock Origin it comes with: on unless the user turned it off. Off turns uBlock Origin off in
// each profile, as its switch in Chrome's extensions page does. The core keeps it with the Spaces.
// The user's: agents get none, as it keeps trackers off the user's pages.
BC_EXPORT bool bc_ad_blocking(void);
BC_EXPORT void bc_set_ad_blocking(bool on);

// What bc_download_act does: opens the file, or for one still running, opens it once it's done or
// no longer; shows it in the system's file manager; pauses, resumes or cancels it; takes it off
// the list, keeping a finished file and deleting any other; or keeps one Chrome held.
#define BC_DOWNLOAD_OPEN 0
#define BC_DOWNLOAD_SHOW 1
#define BC_DOWNLOAD_PAUSE 2
#define BC_DOWNLOAD_RESUME 3
#define BC_DOWNLOAD_CANCEL 4
#define BC_DOWNLOAD_REMOVE 5
#define BC_DOWNLOAD_KEEP 6

// Does what |action|, a BC_DOWNLOAD_ action, says to the download. Then downloads_changed. The
// user's: agents get none, as opening a file runs it outside the browser.
BC_EXPORT void bc_download_act(bc_id download, int action);

// Takes the downloads that aren't running or paused off the list, keeping their files. Then
// downloads_changed.
BC_EXPORT void bc_downloads_clear(void);

// Asks for the icon the system shows for the download's file. Then download_icon.
BC_EXPORT void bc_download_icon(bc_id download);

// Asks for the favicon the profile's history has for the page at |url|, else for its host. Then
// page_favicon.
BC_EXPORT void bc_page_favicon(const char* url);

// Lists suggestions for |text| typed in the window's command bar, as Chrome's address bar does:
// pages from the profile's history and bookmarks, open tabs among them, and a search for |text|.
// Well-known sites |text| could start fill the rows left, under those. Nothing typed leaves the
// device, since search suggestions are off. Then window_suggestions, and
// again as more are found. Empty text lists none. |complete| lets the first suggestion complete
// |text|, which Chrome allows only while the user types at the end of the text: not after deleting
// or pasting, with the caret elsewhere, or while an input method composes. Unless null, |keyword|
// names the search engine whose site search the user entered, and the suggestions are its searches
// for |text|, as in Chrome's keyword mode; window_suggestions then gives "<keyword> <text>".
BC_EXPORT void bc_window_suggest(bc_id window, const char* keyword, const char* text,
                                 bool complete);

// Takes the window's removable suggestion going to |url| out of the profile's history and
// bookmarks' suggestions, as ⇧⌦ does in Chrome's address bar. Then window_suggestions without it.
BC_EXPORT void bc_window_remove_suggestion(bc_id window, const char* url);

// Opens a tab in the window, loading |url|, in the default profile, at the top of the Today tabs of
// the Space it shows. It isn't shown until bc_window_select or bc_tab_show. With |when_shown|, an
// http(s) |url| loads only once the tab is first shown, as a link a page opens behind does. Returns
// 0 if the window is closing or gone.
BC_EXPORT bc_id bc_tab_open(bc_id window, const char* url, bool when_shown);

// Copies up to |capacity| bytes of the URL Chrome's address bar goes to for |text| typed in it into
// |url|, UTF-8 and ending in a NUL, and returns the URL's length without it: the page |text| names,
// else a search for it with the default search engine, DuckDuckGo until the user picks another.
// Returns 0 if |text| names neither.
BC_EXPORT size_t bc_typed_url(const char* text, char* url, size_t capacity);

// As bc_typed_url, the URL of a search for |text| with the search engine whose keyword is
// |keyword|, or the default one if that's null, even when |text| names a page. Returns 0 if there's
// no such engine.
BC_EXPORT size_t bc_search_url(const char* keyword, const char* text, char* url, size_t capacity);

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
// does. A hung page's renderer is ended first, as Chrome's Exit page does, with every page it
// shows: a reload would wait on it.
BC_EXPORT void bc_tab_reload(bc_id tab, bool bypass_cache);

// For a tab whose page is BC_PAGE_HUNG: waits for it, as Chrome's Wait does. The tab is fine again
// until Chrome's hang timeout runs out once more. The UI's and agents', which may reload instead.
BC_EXPORT void bc_tab_wait(bc_id tab);

// Stops the tab loading.
BC_EXPORT void bc_tab_stop(bc_id tab);

// Deletes the profile's cookies for the site of the tab's page if |cookies|, else its cache for
// the site, then loads the page again, bypassing the cache for the cache. A site is a registrable
// domain, such as example.co.uk, or an IP address. Does nothing for a page that isn't http(s). The
// system's engine's store loses the site's too, once the profile has used it.
BC_EXPORT void bc_tab_clear_site(bc_id tab, bool cookies);

// The system's engine, WebKit on the Mac, plays the protected video Chromium can't (decision 1).
// A tab shows a site's pages in it once the user switches it there, from the protected-media note,
// and the profile keeps the site: its pages open there from then on, in any of the profile's tabs,
// until bc_tab_set_system_engine turns it off. Its own store, apart from Chromium's and Safari's,
// one a profile, and one that keeps nothing for a private window. The user's session with the site
// goes along inside the profile: the site's cookies and localStorage in Chromium are copied into
// the system's store before a tab's first load there, and the system's back to Chromium as the
// site is turned off. A tab goes back to Chromium for a link the user follows to another site,
// a URL typed for one, or back from its first page; redirects through other sites, as a sign-in's,
// stay. Agents' tabs never switch, and bc_agent_may_act refuses agents a tab that has.

// Whether the platform has a system engine that keeps a store a profile: macOS 14 or later.
// Answers without loading it.
BC_EXPORT bool bc_system_engine_available(void);

// With |on|, shows the tab's page in the system's engine, loading its URL there, and has the
// profile keep its site for it. Else has the profile forget the site, and each of the profile's
// tabs that shows one of its pages there goes back to Chromium at that page, once the site's
// cookies and those pages' localStorage are in Chromium's store. Returns whether it
// did: not for a page that isn't http(s), nor where there's no system engine, nor for an agent's
// tab. Then tab_state_changed, tab_media_unsupported with an empty key system, and
// tab_site_controls if watched. The user's: agents get none.
BC_EXPORT bool bc_tab_set_system_engine(bc_id tab, bool on);

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

// Asks for the box of the element at (|x|, |y|), in points from the top-left of the tab's page
// view, as DevTools' element picker hit tests, without running the page's JavaScript. One question
// is in the page at a time, and a newer one replaces any still waiting, so a moving pointer asks
// only for where it is now. Then tab_element_box. The user's: agents get none, as Capture is.
BC_EXPORT void bc_tab_element_at(bc_id tab, double x, double y);

// Captures the tab's page as a PNG: |rect|, in points from the top-left of the page's view, or the
// whole view if |rect| is empty, as the compositor last drew it; or with |full_page| the whole
// page, beyond the view, as DevTools' full-size screenshot does, which lays the page out at its
// full height while it draws. In pixels at the display's scale. Then tab_captured with |request|.
// The user's: agents get none, as a picture shows what a page holds, and agents read pages through
// their own tool, by its rules.
BC_EXPORT void bc_tab_capture(bc_id tab, uint64_t request, bool full_page, bc_rect rect);

// Ends what bc_tab_element_at and bc_tab_capture began: drops their questions and lets go of the
// page's DevTools, so a page that isn't being captured pays nothing.
BC_EXPORT void bc_tab_capture_end(bc_id tab);

// Answers to the question tab_permission_asked put: the site may, now and whenever it asks again;
// it may not, now or again; or not now, and it may ask again.
#define BC_PERMISSION_ALLOW 0
#define BC_PERMISSION_BLOCK 1
#define BC_PERMISSION_DISMISS 2

// Answers the tab's page's question with |answer|, a BC_PERMISSION_ answer, if it's still asking.
// The profile keeps an allow or a block for the site, which Chrome's site settings list and
// change. Then tab_permission_asked. The user's: agents get none, as an answer can give a site the
// camera or where the user is.
BC_EXPORT void bc_tab_answer_permission(bc_id tab, int answer);

// Reads the site of the tab's page for its Site Controls, as Chrome's page info does, and watches
// it until bc_site_controls_close or another tab's bc_tab_site_controls. Then tab_site_controls.
BC_EXPORT void bc_tab_site_controls(bc_id tab);

// Stops watching the site bc_tab_site_controls read.
BC_EXPORT void bc_site_controls_close(void);

// Gives the site of the tab's page |value|, a BC_SITE_ value, for the permission of |type|, as
// Chrome's page info does, if it's one of the permission's choices and bc_tab_site_controls is
// watching that tab. The profile keeps it, as Chrome's site settings do. Then tab_site_controls.
// The user's: agents get none, as a change can give a site the camera or where the user is, and
// take back what the user blocked.
BC_EXPORT void bc_tab_set_site_permission(bc_id tab, int type, int value);

// Sets the site's permissions the user set back to their defaults, as Chrome's page info's "Reset
// permissions" does, if bc_tab_site_controls is watching the tab. Then tab_site_controls. The
// user's, as bc_tab_set_site_permission is.
BC_EXPORT void bc_tab_reset_site_permissions(bc_id tab);

// Lets the ad blocker run on the site of the tab's page or not, if bc_tab_site_controls is watching
// the tab and bc_site_controls' ad_blocking isn't BC_AD_BLOCKING_NONE, and reloads the page. The
// profile keeps it, for the site and its subdomains. Then tab_site_controls. The user's, as
// bc_set_ad_blocking is.
BC_EXPORT void bc_tab_set_ad_blocking(bc_id tab, bool on);

// Takes the tab's playing video into Picture in Picture, or brings it back, as
// bc_site_controls' picture_in_picture says it can. Then tab_site_controls.
BC_EXPORT void bc_tab_picture_in_picture(bc_id tab);

// Shows the certificate the tab's page came with in the system's viewer, a sheet on the tab's
// window on the Mac, as Chrome's page info does. Does nothing for a page with none, as http and
// the browser's own pages have.
BC_EXPORT void bc_tab_show_certificate(bc_id tab);

// Answers to the offer tab_offer_changed made: save or update the password; never save one for the
// site; or not now.
#define BC_OFFER_ACCEPT 0
#define BC_OFFER_NEVER 1
#define BC_OFFER_NOT_NOW 2

// Answers the tab's offer with |answer|, a BC_OFFER_ answer, with the username as the user left it,
// or null to keep the offer's, if the offer is still open. Then tab_offer_changed. The user's:
// agents get none, as saving a password is the user's to decide.
BC_EXPORT void bc_tab_answer_offer(bc_id tab, int answer, const char* username);

// Scrolls the tab's page to put its selection, or its caret, in the middle of the page's view, as
// Edit › Find › Jump to Selection does. Does nothing if it has neither.
BC_EXPORT void bc_tab_jump_to_selection(bc_id tab);

// Pins the tab in its Space at the page it's at, at the bottom of the pinned tabs, or unpins it to
// the top of the Today tabs, with the rest of its split view. Then space_changed and
// tab_state_changed.
BC_EXPORT void bc_tab_set_pinned(bc_id tab, bool pinned);

// Makes the tab the favorite at |index| among those its Space shows, counted without it, else the
// last of them; or moves a favorite there. A tab of a Space leaves it and its split view, and its
// favorite's URL is its pinned URL if it's pinned, else the page it's at. A window showing it keeps
// showing it. Under BC_FAVORITES_EVERY the other profiles' Spaces get the site too, in tabs of
// their own, and moving it moves it there. Under BC_FAVORITES_SPACE a tab of an agent's Space isn't
// made one. Then favorites_changed, and space_changed and tab_state_changed for a tab of a Space.
BC_EXPORT void bc_tab_move_to_favorites(bc_id tab, size_t index);

// Moves the tab, with the rest of its split view, to |index| in its Space's Today tabs, or in its
// pinned tabs, inside |folder| if it isn't 0, counted without them, else to the bottom, and below a
// split |index| is inside. In the pinned tabs or a folder, |index| counts the tabs and folders
// directly in it. A Today tab moved to the pinned tabs is pinned at the page it's at, and a pinned
// tab moved to Today is unpinned. A favorite leaves the favorites for the Space its window shows,
// unless that's an agent's or doesn't show it, pinned at its favorite's URL in the pinned tabs, and
// a window showing it keeps showing it. Under BC_FAVORITES_EVERY the site goes from every profile,
// and its other tabs close. Then space_changed and tab_state_changed, and favorites_changed for a
// favorite.
BC_EXPORT void bc_tab_move(bc_id tab, bool pinned, bc_id folder, size_t index);

// Loads a pinned tab's pinned URL, or a favorite's URL, unless it's there already. Then
// tab_state_changed.
BC_EXPORT void bc_tab_back_to_pinned_url(bc_id tab);

// Copies the URL a pinned tab or a favorite goes back to into |url| as bc_space_name does, or an
// empty string for any other tab.
BC_EXPORT size_t bc_tab_pinned_url(bc_id tab, char* url, size_t capacity);

// Changes the URL a pinned tab or a favorite goes back to, which unpinning forgets, so an undo can
// give a tab back the one it had. Under BC_FAVORITES_EVERY a favorite's site gets it in every
// profile. Does nothing for another tab or an invalid |url|. Then
// space_changed or favorites_changed, and tab_state_changed. The user's: agents get none.
BC_EXPORT void bc_tab_set_pinned_url(bc_id tab, const char* url);

// Opens a copy of the tab, with its history, under it in Today, or at the top of Today for a
// pinned tab, or of the Space its window shows for a favorite. Shows it in its window, where its
// page loads. Returns its id, or 0. Then space_changed and window_selected.
BC_EXPORT bc_id bc_tab_duplicate(bc_id tab);

// Copies the name the user gave the tab into |name| as bc_space_name does, or an empty string if
// it shows its page's title.
BC_EXPORT size_t bc_tab_name(bc_id tab, char* name, size_t capacity);

// Names the tab, which shows |name| for its title from then on, kept across a quit, a reopen and a
// duplicate; an empty name gives it back its page's title. Then tab_title_changed.
BC_EXPORT void bc_tab_rename(bc_id tab, const char* name);

// Gives the tab |icon|, an emoji, UTF-8, to show in place of its page's favicon, kept as its name
// is; empty gives it back its favicon. Then tab_state_changed.
BC_EXPORT void bc_tab_set_icon(bc_id tab, const char* icon);

// Mutes or unmutes the tab, whatever page it's at. Duplicating it and ⇧⌘T keep it muted; a quit
// doesn't. Then tab_state_changed.
BC_EXPORT void bc_tab_set_muted(bc_id tab, bool muted);

// Closes the Today tabs below the tab, each as bc_tab_close does, bottom up. A window showing one
// shows the tab first. Then tab_closed for each, and space_changed.
BC_EXPORT void bc_tab_close_below(bc_id tab);

// Moves the tab to |space|: a pinned tab to the bottom of its pinned tabs, still pinned, a Today
// tab to the top of its Today tabs. A window showing the tab shows |space| with it. Then
// space_changed for both Spaces, and window_space_shown and window_selected for a window that went
// with it. A page can't take its cookies and logins to another profile, so into another profile's
// Space the tab goes as a new one, with its history, name and mute, which loads once it's shown.
// The old one closes, kept out of the archive and ⇧⌘T's tabs, as it lives on in the new one; then
// tab_closed. Nothing moves into an agent's Space: a tab leaves one only.
BC_EXPORT void bc_tab_move_to_space(bc_id tab, bc_id space);

// Zooms the tab's page in (|step| 1) or out (-1) to Chrome's next level, or back to 100 % (0), for
// its whole site, as Chrome does, and the profile keeps it. Returns the zoom in percent after.
BC_EXPORT int bc_tab_zoom(bc_id tab, int step);

// What bc_tab_devtools does: opens or closes the DevTools, opens or closes them at the console, or
// opens them picking an element in the page.
#define BC_DEVTOOLS_TOGGLE 0
#define BC_DEVTOOLS_CONSOLE 1
#define BC_DEVTOOLS_INSPECT 2

// Opens Chrome's DevTools for the tab shown, docked in its window, as BC_DEVTOOLS_ |what| says.
// They're the user's: agents get none (decision 8).
BC_EXPORT void bc_tab_devtools(bc_id tab, int what);

// Opens the source of the tab's page in a new tab under it, at its view-source: URL.
BC_EXPORT void bc_tab_view_source(bc_id tab);

// Shows Chrome's print preview for the page of the tab shown, over the page, as ⌘P does in Chrome.
// Nothing while the tab shows another dialog, or its page has crashed. The user's: agents get none.
BC_EXPORT void bc_tab_print(bc_id tab);

// Asks where to save the page of the tab shown, as Chrome's Save Page As, and saves it there. The
// user's: agents get none.
BC_EXPORT void bc_tab_save_page(bc_id tab);

// Opens Chrome's Task Manager, or brings it to the front, with the page |window| shows selected in
// it. The user's: agents get none.
BC_EXPORT void bc_window_task_manager(bc_id window);

// Closes the tab: its page gets its unload handlers, which may keep it open, then tab_closed, and
// a Today tab goes in the archive. The window stays open without it. A pinned tab stays in its
// Space instead, and a favorite in the favorites, and only its page goes: a window showing it
// shows the top Today tab of the Space it shows, else the nearest pinned tab still loaded, else no
// page. A tab in a split view leaves it, and a window
// showing it shows the tab beside it, unless |split| closes the rest of the split with it, as
// the ✕ on its row does: a pinned split stays a split. Then tab_state_changed, and
// window_selected.
BC_EXPORT void bc_tab_close(bc_id tab, bool split);

// Windows. Every window shows the same Spaces, and a tab's page is live in one window at a time:
// the one that showed it last. The reads are for the UI and agents alike.

// Copies up to |capacity| of the open windows into |windows|, the most recently active first, and
// returns how many there are. Private windows aren't among them.
BC_EXPORT size_t bc_windows(bc_id* windows, size_t capacity);
// The Space the window shows, and the tab it shows there, as window_space_shown and
// window_selected last said. 0 for none.
BC_EXPORT bc_id bc_window_space(bc_id window);
BC_EXPORT bc_id bc_window_selected(bc_id window);
// The window that shows |tab|'s page, focused or in its split view, else 0. A window whose page
// another window took doesn't count.
BC_EXPORT bc_id bc_tab_window(bc_id tab);
// Shows |space| with no page, as a window ⌘N opens does. The user's: agents get none.
BC_EXPORT void bc_window_show_space_empty(bc_id window, bc_id space);

// Private windows are the user's alone: agents get no path into them or their tabs, not even reads.
// bc_windows leaves them out, and the agent server is to refuse a private window's id, its Space's
// and its tabs'. The UI uses the rest of the API on them, as on any window, but they never show
// another window's Spaces, and closing one closes its tabs.

// Opens a private window at |bounds|, as bc_window_open does, for |profile|'s off-the-record
// profile: Chrome's incognito. Its private windows share one session, cookies and cache, that the
// last one's closing ends (private_session_ended), and nothing of it is written: no history, and
// its Space, which holds its tabs as Today tabs, is never saved, archived or listed by bc_spaces.
// It shows that Space alone, reported by window_space_shown, and no page until one opens. Its
// tabs never move to a regular window, nor regular tabs to it. Closing it closes its tabs. Returns
// 0 if |profile| isn't loaded or Chromium is quitting. The user's alone: agents get none.
BC_EXPORT bc_id bc_window_open_private(bc_rect bounds, void* ui_view, bc_id profile);

// Extensions' side panels. Each profile's tabs have their own, as each Chrome window does: one for
// every tab, or one for a tab that its extension gave it, which shows only while that tab does. An
// extension opens its panel with sidePanel.open() or from its action, and the user does with
// bc_window_toggle_side_panel. The user's: agents get none, as its page isn't a web page, and a
// private window has none.

// Where the window shows the side panel's page, in the window, its bottom corners as round as the
// page's. The page keeps its distance from the window's edges as the window resizes, as the tab's
// does. An empty frame, as for a page in full screen, hides it.
BC_EXPORT void bc_window_set_side_panel_frame(bc_id window, bc_rect frame);

// Closes the side panel the window shows, as its ✕ in Chrome does. Then window_side_panel.
BC_EXPORT void bc_window_close_side_panel(bc_id window);

// Whether the extension has a side panel for the tab the window shows, as Chrome's extension menu
// asks before it offers Open Side Panel.
BC_EXPORT bool bc_window_has_side_panel(bc_id window, const char* id);

// Opens the extension's side panel for the tab the window shows, or closes it if it shows, as
// Chrome's extension menu does. Then window_side_panel.
BC_EXPORT void bc_window_toggle_side_panel(bc_id window, const char* id);

// Saves the width the user gave the side panel, in the profile of the tab the window shows, for
// window_side_panel to give from now on.
BC_EXPORT void bc_window_set_side_panel_width(bc_id window, double width);

#ifdef __cplusplus
}
#endif

#endif  // BROWSER_CORE_PUBLIC_BROWSER_CORE_H_
