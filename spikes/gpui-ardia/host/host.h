// The browser-process side of the spike, called from Rust. Chromium (Ardia's build of Chrome's
// framework) owns the NSApplication, its delegate and the run loop, and makes the browser window,
// as Chrome's framework will in the real browser. GPUI draws inside that window.
#pragma once

#ifdef __cplusplus
extern "C" {
#endif

// Browser shortcuts the page hands back to the UI.
enum spike_command {
  SPIKE_NEW_TAB = 1,
  SPIKE_EDIT_URL = 2,
  SPIKE_CLOSE_TAB = 3,
  SPIKE_FIND = 4,
  SPIKE_QUIT = 5,
};
typedef void (*spike_command_fn)(int command);
typedef void (*spike_started_fn)(void);

// Starts Chromium and runs its run loop until it shuts down, then returns the exit code.
// |on_started| runs inside the run loop once Chromium is up; the UI starts there.
int spike_host_run(int argc, char** argv, spike_started_fn on_started, spike_command_fn on_command);

// Opens the browser window with a page of |url|, inset by |left| on the left and |gap| on the
// other sides. Returns the view the UI draws in: it fills the window, under the page.
void* spike_host_open_window(const char* url, double left, double gap);

// Loads |url| in the page.
void spike_host_load_url(const char* url);

// Makes the window of |overlay_view| float over the browser window when shown, for UI that has
// to draw over the page. It starts hidden.
void spike_host_add_overlay(void* overlay_view);

// Shows the overlay and gives it the keyboard.
void spike_host_show_overlay(void);

// Hides the overlay. The browser window gets the keyboard back, with the focus where it was.
void spike_host_hide_overlay(void);

// Closes the page and shuts Chromium down.
void spike_host_quit(void);

#ifdef __cplusplus
}
#endif
