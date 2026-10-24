// The AppKit side of the app, called from Rust: what the core's API (core/public/browser_core.h)
// leaves to the app. The core makes the browser window and puts the page in it; GPUI draws in a
// view of the app's, under the page.
#pragma once

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

#ifdef __cplusplus
}
#endif
