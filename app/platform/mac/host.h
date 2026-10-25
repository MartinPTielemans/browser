// The AppKit side of the app, called from Rust: what the core's API (core/public/browser_core.h)
// leaves to the app. The core makes the browser window and puts the page in it; GPUI draws in a
// view of the app's, under the page.
#pragma once

#include <stdbool.h>
#include <stddef.h>

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
// to draw over the page. It starts hidden.
void host_add_overlay(void* overlay_view);

// Shows the overlay and gives it the keyboard.
void host_show_overlay(void);

// Makes the overlay |height| points tall, keeping its top edge, and draws it at that size.
void host_set_overlay_height(double height);

// Hides the overlay. The browser window gets the keyboard back, with the focus where it was.
void host_hide_overlay(void);

// Makes the window of |find_bar_view| float over the browser window when shown, |right| and |top|
// points from the top right corner of the window's content, for the find bar. It starts hidden.
void host_add_find_bar(void* find_bar_view, double right, double top);

// Shows the find bar, and gives it the keyboard if |keyboard|. Unlike the command bar, it stays
// when the user clicks the page, which takes the keyboard.
void host_show_find_bar(bool keyboard);

// Hides the find bar. If it had the keyboard, the browser window gets it back.
void host_hide_find_bar(void);

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
} host_menu_item;

// Shows a menu of the |count| |items| at |x|, |y|, in points from the top-left of the browser
// window's content, as a right-click there does. Returns the index of the one chosen once it closes,
// or -1.
int host_menu(const host_menu_item* items, size_t count, double x, double y);

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

// For the test driver: chooses the item titled |title| in the menu shown, or the button in the
// alert shown. False if neither shows one.
bool host_choose(const char* title);

#ifdef __cplusplus
}
#endif
