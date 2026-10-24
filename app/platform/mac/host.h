// The AppKit side of the app, called from Rust: what the core's API (core/public/browser_core.h)
// leaves to the app. The core makes the browser window and puts the page in it; GPUI draws in a
// view of the app's, under the page.
#pragma once

#include <stdbool.h>

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

#ifdef __cplusplus
}
#endif
