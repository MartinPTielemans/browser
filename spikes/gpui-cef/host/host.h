// The browser-process side of the spike, called from Rust. Chromium (through CEF) owns the
// NSApplication, its delegate and the run loop, the way Chrome's framework will in the real
// browser. GPUI attaches to them.
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Browser shortcuts the page view hands back to the UI.
enum spike_command {
  SPIKE_NEW_TAB = 1,
  SPIKE_EDIT_URL = 2,
  SPIKE_CLOSE_TAB = 3,
  SPIKE_FIND = 4,
  SPIKE_QUIT = 5,
};
typedef void (*spike_command_fn)(int command);

// Loads CEF, creates the application object and starts Chromium. Must run before GPUI starts.
// Returns false if Chromium couldn't start.
bool spike_host_init(int argc, char** argv, spike_command_fn on_command);

// Creates the page view over |parent_view| (an NSView), at |x|, |y|, |width| and |height| in
// its coordinates. The view keeps its margins when the window resizes.
void spike_host_create_browser(void* parent_view, double x, double y, double width, double height,
                               const char* url);

// Opens a plain AppKit window with the page in it at the size it has in the spike's window, and
// no GPUI. It's the baseline for measuring what GPUI costs the page.
void spike_host_open_plain_window(const char* url);

// Loads |url| in the page.
void spike_host_load_url(const char* url);

// Makes the window of |overlay_view| float over the window of |parent_view| when shown, for UI
// that has to draw over the page. It starts hidden.
void spike_host_add_overlay(void* parent_view, void* overlay_view);

// Shows the overlay and gives it the keyboard.
void spike_host_show_overlay(void);

// Hides the overlay. The browser window gets the keyboard back, with the focus where it was.
void spike_host_hide_overlay(void);

// Runs the application's run loop until spike_host_quit, then shuts Chromium down.
void spike_host_run(void);

// Closes the page and ends the run loop.
void spike_host_quit(void);

#ifdef __cplusplus
}
#endif
