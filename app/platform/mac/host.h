// The AppKit side of the app, called from Rust: what the core's API (core/public/browser_core.h)
// leaves to the app. The core makes the browser window and puts the page in it; GPUI draws in a
// view of the app's, under the page.
#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Starts the test driver (drive.mm), which takes commands on stdin.
void drive_start(void);

// Where a window of |width| by |height| points goes to be centered on the main display, or on the
// Mac's own with BROWSER_DISPLAY=builtin, in bc_window_open's terms: from the main display's
// top-left corner.
void host_centered(double width, double height, double* x, double* y);

// Makes the view the UI draws in, for bc_window_open, which sizes it to fill the window.
void* host_ui_view(void);

// Shows the window holding |ui_view| in front, with its title bar over the UI, and makes the
// app the active one.
void host_show_window(void* ui_view);

// Makes the window of |overlay_view| float over the browser window when shown, for UI that has
// to draw over the page. It starts hidden. |dismissed| runs, after the event, when the user leaves
// the overlay without choosing anything in it.
void host_add_overlay(void* overlay_view, void (*dismissed)(void));

// Shows the overlay and gives it the keyboard, |width| points wide: with its top left at |left|,
// |top| in the browser window's content and ending at least 8 points inside it, or if |centered|,
// centred a fifth of the way down.
void host_show_overlay(bool centered, double left, double top, double width);

// Makes the overlay |height| points tall, keeping its top edge, and draws it at that size.
void host_set_overlay_height(double height);

// Gives the overlay's shadow the shape the overlay draws now, after it changed.
void host_reshape_overlay(void);

// Hides the overlay. The browser window gets the keyboard back, with the focus where it was.
void host_hide_overlay(void);

// Hides the overlay as the user leaving it, as escape does.
void host_dismiss_overlay(void);

// Makes the window of |find_bar_view| float over the browser window when shown, |right| and |top|
// points from the top right corner of the window's content, for the find bar. It starts hidden.
void host_add_find_bar(void* find_bar_view, double right, double top);

// Shows the find bar, and gives it the keyboard if |keyboard|. Unlike the command bar, it stays
// when the user clicks the page, which takes the keyboard.
void host_show_find_bar(bool keyboard);

// Hides the find bar. If it had the keyboard, the browser window gets it back.
void host_hide_find_bar(void);

// Moves the find bar and the HUD |right| points further in from the window's right edge, to the
// focused page's corner when a split view's focused page isn't the rightmost.
void host_set_page_right(double right);

// Makes the window of |hud_view| float over the browser window when shown, |right| and |top| points
// from the top right corner of the window's content, for the zoom HUD. It starts hidden, and the
// mouse goes through it.
void host_add_hud(void* hud_view, double right, double top);

// Shows the HUD as the UI draws it now, again for a change, until host_hide_hud.
void host_show_hud(void);

// Fades the HUD out over |seconds|.
void host_hide_hud(double seconds);

// Gives |view| the keyboard in its window, from the page if the page has it.
void host_focus(void* view);

typedef struct {
  // Null for a line between items.
  const char* title;
  bool enabled;
  // The shortcut it shows: its key as AppKit's key equivalent, "" for none, and its modifiers.
  const char* key;
  bool command;
  bool shift;
  bool option;
  bool control;
  // The index of the item whose submenu it's in, or -1.
  int parent;
  // Whether it shows a check mark.
  bool checked;
  // A round swatch of this colour, 0xRRGGBB in sRGB, before its title, or -1 for none.
  int32_t swatch;
} host_menu_item;

// Shows a menu of the |count| |items| at |x|, |y|, in points from the top-left of the browser
// window's content, or of the reveal panel's if |in_reveal|, as a right-click there does. Returns
// the index of the one chosen once it closes, or -1.
int host_menu(const host_menu_item* items, size_t count, double x, double y, bool in_reveal);

// Asks in an alert whether to do what |message| asks, with |detail| under it, and returns whether
// the user chose |action| over Cancel once it closes.
bool host_confirm(const char* message, const char* detail, const char* action);

// Shows the browser window's close, minimize and zoom buttons, or hides them, as Arc does while its
// sidebar is hidden.
void host_show_window_buttons(bool shown);

// Does what a click on the browser window's close (0), minimize (1) or zoom (2) button does.
void host_press_window_button(int button);

// Makes the window of |reveal_view| the sidebar reveal's, which shows over the browser window's
// left edge without taking the keyboard. It starts hidden. While it shows, |pointer_moved| hears
// where the pointer is across the browser window's content, in points from its left edge, as it
// moves anywhere in the app's windows, and an x past them all when the app stops being the
// active one.
void host_add_reveal(void* reveal_view, void (*pointer_moved)(double x));

// Shows the reveal's window, |width| points wide from the browser window's left edge and as tall
// as its content, or makes it that wide if it shows.
void host_show_reveal(double width);

// Hides the reveal's window.
void host_hide_reveal(void);

// A detent on the trackpad, as a swipe's nearest Space changes.
void host_detent(void);

// The system's accent colour in sRGB, as 0xRRGGBB.
uint32_t host_accent_color(void);

// Calls |changed| each time the user changes the system's accent colour.
void host_watch_accent_color(void (*changed)(void));

// Calls |released| once, after the event that shows ⌃ is no longer held: ⌃ going up, a key or a
// button pressed without it, or the app going to the background. Watches the whole app, whichever
// view has the keyboard.
void host_watch_control(void (*released)(void));

// Puts |text| on the clipboard. A driven run puts it on a pasteboard of its own instead, so checks
// never touch the user's clipboard.
void host_copy(const char* text);

// The starts of today, yesterday, this week and last week in the user's calendar and time zone, in
// milliseconds since the Unix epoch.
void host_day_starts(double starts[4]);

// For the test driver: chooses the item titled |title| in the menu shown, or the button in the
// alert shown. False if neither shows one.
bool host_choose(const char* title);

#ifdef __cplusplus
}
#endif
