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

// Puts a window on screen where the first browser window will open, |width| by |height|, with
// the window's material and the sidebar as it was when the app last quit, from |profile_dir|.
// For the moment between NSApp being made and the browser starting.
void host_show_launch_window(const char* profile_dir, double width, double height);

// Takes the launch window's sidebar picture off the browser window, once the UI has drawn under
// it, and the launch window off screen if the browser window never came.
void host_close_launch_window(void);

// Calls |finished| once AppKit has finished launching the app, and so has handed over the links
// or files the launch carried (application:openURLs:). Call before the run loop starts.
void host_watch_finished_launching(void (*finished)(void));

// Keeps the first |width| points of |ui_view|'s window, its sidebar, in |profile_dir| for the next
// launch's window, unless the window is hidden or full screen, and says whether it took it. The
// file is written in the background.
bool host_save_launch_picture(void* ui_view, const char* profile_dir, double width);

// Makes the view the UI draws in, for bc_window_open, which sizes it to fill the window. The
// functions below that take a |ui_view| act on the browser window it's in.
void* host_ui_view(void);

// Shows the window holding |ui_view| in front, with its title bar over the UI, and makes the
// app the active one.
void host_show_window(void* ui_view);

// Where the next browser window goes after the one of |ui_view|, in host_centered's terms: as
// big, down and to the right as AppKit cascades windows, kept on its screen.
void host_cascaded(void* ui_view, double* x, double* y, double* width, double* height);

// Names the browser window of |ui_view| in the Window menu's list of windows and in Mission
// Control. The window doesn't draw it.
void host_set_window_title(void* ui_view, const char* title);

// Makes the browser window of |ui_view| key and puts it in front, out of the Dock if it's
// minimized, as a Switch to Tab row for a page it shows does. The user's: agents get none, as it
// takes the user's focus (decision 8).
void host_activate(void* ui_view);

// Calls |changed| with the view of each browser window that becomes the key window.
void host_watch_key_window(void (*changed)(void* ui_view));

// Calls |changed| with the view of each browser window that has gone full screen, or left it, once
// macOS's transition is over.
void host_watch_full_screen(void (*changed)(void* ui_view, bool entered));

// Calls |resigning| as the app is about to stop being the active one, its windows still shown.
void host_watch_resigning_active(void (*resigning)(void));

// Calls |became_active| each time the app becomes the active one, such as back from System
// Settings.
void host_watch_active(void (*became_active)(void));

// Whether the system opens http and https links with this app: whether it's the default browser.
bool host_is_default_browser(void);

// Asks the system to open http and https links with this app, which asks the user to confirm
// first, then calls |answered| once they have, either way. The way back is the system's own
// setting, in System Settings › Desktop & Dock. The user's alone: agents get none, as it's the
// system's setting, not the browser's.
void host_make_default_browser(void (*answered)(void));

// Milliseconds since the system started the app's process, for its launch times.
double host_since_launch(void);

// The browser window of |ui_view| has closed: what showed over it hides, and its windows go.
void host_forget_window(void* ui_view);

// The overlays: the command bar, and the Space's icon picker. One shows at a time.
#define HOST_COMMAND_BAR 0
#define HOST_ICON_PICKER 1

// Makes the window of |overlay_view| the |overlay| (HOST_COMMAND_BAR or HOST_ICON_PICKER), which
// floats over a browser window when shown, for UI that has to draw over the page, translucent over
// a blur of what's behind it, with a shadow, in a rounded shape of |radius|. It starts hidden.
// |dismissed| runs, after the event, when the user leaves the overlay without choosing anything in
// it.
void host_add_overlay(int overlay, void* overlay_view, double radius, void (*dismissed)(void));

// Gives the overlay's blur and shadow the shape the UI draws when that isn't a rounded rectangle:
// the polygon through the |count| corners, each three numbers in |corners|: x and y in points from
// the window's top left, and the radius it's rounded by there.
void host_set_overlay_shape(int overlay, const double* corners, size_t count);

// Rounds the overlay's blur and shadow by |radius| from its next show on, as the command bar's
// panel over the URL field is rounded less than its centred one.
void host_set_overlay_radius(int overlay, double radius);

// Makes the overlay grow in as it shows, as a system popover does, from |x| and |y| points from
// its window's top left, such as the tip of its arrow. It goes in one frame.
void host_set_overlay_growth(int overlay, double x, double y);

// Shows the overlay over |ui_view|'s window and gives it the keyboard, |width| points wide: with
// its top left at |left|, |top| in the window's content and ending at least 8 points inside it, or
// if |centered|, centred across it with its top |top| points above its middle. The other overlay
// closes, as the user leaving it. Shown there already, it moves to the new place. Keys typed
// before it has the keyboard go to it once it does.
void host_show_overlay(int overlay,
                       void* ui_view,
                       bool centered,
                       double left,
                       double top,
                       double width);

// Makes the overlay |height| points tall, keeping its top edge, and draws it at that size.
void host_set_overlay_height(int overlay, double height);

// Whether this system's emoji font draws |emoji|, UTF-8, as the one emoji it is. A newer emoji than
// the system knows draws as a missing glyph, and a newer sequence as the emoji it is made of. For
// the icon picker's list. emoji.mm.
bool host_can_draw_emoji(const char* emoji);

// Hides the overlay. The browser window gets the keyboard back, with the focus where it was.
void host_hide_overlay(int overlay);

// Hides the overlay as the user leaving it, as escape does.
void host_dismiss_overlay(int overlay);

// Holds the keys typed from now until the command bar or the find bar takes the keyboard, and then
// gives them to it, or for 400 ms at most: for a shortcut that opens one but reaches the page
// first, which may keep it. Showing either bar with the keyboard holds them by itself.
void host_hold_keys(void);

// Makes the window of |find_bar_view| the find bar of |ui_view|'s window, floating over it when
// shown, |right| and |top| points from the top right corner of the window's content. It starts
// hidden.
void host_add_find_bar(void* ui_view, void* find_bar_view, double right, double top);

// Shows |ui_view|'s find bar, and gives it the keyboard if |keyboard|, with the keys typed until
// then. Unlike the command bar, it stays when the user clicks the page, which takes the keyboard.
void host_show_find_bar(void* ui_view, bool keyboard);

// Hides |ui_view|'s find bar. If it had the keyboard, its browser window gets it back.
void host_hide_find_bar(void* ui_view);

// Makes the window of |offer_view| the offer to save a password of |ui_view|'s window, floating
// over it when shown, |right| and |top| points from the top right corner of the window's content,
// or under the find bar while it shows. It starts hidden.
void host_add_offer(void* ui_view, void* offer_view, double right, double top);

// Shows |ui_view|'s offer, without the keyboard: as the find bar, it takes it when clicked, and the
// page takes it back when clicked.
void host_show_offer(void* ui_view);

// Hides |ui_view|'s offer. If it had the keyboard, its browser window gets it back.
void host_hide_offer(void* ui_view);

// Moves |ui_view|'s find bar, its offer and the HUD over it |right| points further in from the
// window's right edge and |top| points further down, to the focused page's top right corner when a
// split view's focused page isn't the top right one.
void host_set_pane_corner(void* ui_view, double right, double top);

// Makes the window of |hud_view| float over a browser window when shown, |right| and |top| points
// from the top right corner of the window's content, for the zoom HUD, which draws its pill
// |margin| inside the window. It starts hidden, and the mouse goes through it.
void host_add_hud(void* hud_view, double right, double top, double margin);

// Shows the HUD over |ui_view|'s window as the UI draws it now, again for a change, until
// host_hide_hud.
void host_show_hud(void* ui_view);

// Hides the HUD, once it has drawn its way out.
void host_hide_hud(void);

// Makes the window of |view| float over a browser window's content when shown, covering it, for
// the split view's drop targets over the page. It starts hidden, and the mouse goes through it.
void host_add_drop_targets(void* view);

// Shows the drop targets' window as the UI draws it now, at the content of |ui_view|'s window.
void host_show_drop_targets(void* ui_view);

// Hides the drop targets' window.
void host_hide_drop_targets(void);

// Makes the window of |dialog_view| the dialog's, which covers a browser window when shown, with
// the keyboard until it's hidden. It starts hidden.
void host_add_dialog(void* dialog_view);

// Shows the dialog's window over |ui_view|'s window, as the UI draws it now, with the keyboard.
void host_show_dialog(void* ui_view);

// Hides the dialog's window. The browser window gets the keyboard back, with the focus where it
// was.
void host_hide_dialog(void);

// Makes the window of |view| the Site Controls', which float over a browser window as its child
// when shown, translucent over a blur in their shape, with a shadow. It starts hidden. |dismissed|
// runs, after the event, when the user leaves them: a click outside them, another window taking
// the keyboard, or switching apps.
void host_add_site_controls(void* view, void (*dismissed)(void));

// The Site Controls' shape from their window's top left, |count| corners clockwise, each as x, y
// and the radius it's rounded by, which their blur and shadow take at their next show or height.
void host_set_site_controls_shape(const double* corners, size_t count);

// Shows the Site Controls over |ui_view|'s window, as the UI draws them now, with the keyboard:
// |width| by |height| points, their top left at |left|, |top| in the window's content.
void host_show_site_controls(void* ui_view, double left, double top, double width, double height);

// Makes the Site Controls |height| points tall, keeping their top edge, and draws them at that
// size.
void host_set_site_controls_height(double height);

// The UI drew a frame of the Site Controls' motion, at |scale| about |x|, |y| from their window's
// top left and at |opacity|, which their blur and shadow follow.
void host_site_controls_drew(double scale, double opacity, double x, double y);

// The Site Controls take no more clicks or keys as they leave: the mouse goes through them, and the
// browser window gets the keyboard back, with the focus where it was, if they had it.
void host_release_site_controls(void);

// Hides the Site Controls once they've left.
void host_hide_site_controls(void);

// Makes the window of |view| Capture's, which covers a browser window's page when shown, as its
// child, with the keyboard and a camera for the pointer. It starts hidden. |dismissed| runs, after
// the event, when the user leaves it: a click outside it, another window taking the keyboard,
// switching apps, or the browser window changing size, which the picture of the page no longer
// fits.
void host_add_capture(void* view, void (*dismissed)(void));

// Shows Capture's window over |ui_view|'s window, as the UI draws it now, with the keyboard:
// |width| by |height| points, its top left at |left|, |top| in the window's content.
void host_show_capture(void* ui_view, double left, double top, double width, double height);

// The pointer over Capture's window is the camera, or with |camera| false the arrow, as over its
// menu.
void host_capture_camera(bool camera);

// Hides Capture's window. The browser window gets the keyboard back, with the focus where it was.
void host_hide_capture(void);

// Puts the PNG |png|, |size| bytes long, on the clipboard, or on host_copy's pasteboard of its own
// in a driven run.
void host_copy_png(const uint8_t* png, size_t size);

// Asks where to save the PNG |png|, |size| bytes long, in a save panel on |ui_view|'s window, named
// for when it was taken, and saves it there.
void host_save_png(void* ui_view, const uint8_t* png, size_t size);

// Shares the PNG |png|, |size| bytes long, as a file named for when it was taken: in the system's
// share menu at |x|, |y|, |width| by |height| points from the top left of |view|, a window's view,
// or with |messages| in a new message in Messages.
void host_share_png(const uint8_t* png,
                    size_t size,
                    void* view,
                    double x,
                    double y,
                    double width,
                    double height,
                    bool messages);

// Makes the window of |toast_view| the toast's, which floats over a browser window's top right
// corner when shown, as its child, and lets the mouse through. It starts hidden.
void host_add_toast(void* toast_view);

// The toast takes clicks, while it has buttons, or lets the mouse through to the page.
void host_toast_takes_clicks(bool takes);

// Shows the toast's window over |ui_view|'s window, as the UI draws it now.
void host_show_toast(void* ui_view);

// Hides the toast's window.
void host_hide_toast(void);

// Makes the window of |tooltip_view| the tooltip's, which floats over a browser window as its
// child when shown, and lets the mouse through and never takes the keyboard. It starts hidden.
// |away| is called when that window stops being key or the app stops being active, which hide it.
void host_add_tooltip(void* tooltip_view, void (*away)(void));

// Shows the tooltip's window as the UI draws it now, fading in over |fade| seconds from where it
// is, or moves it, with its top left at (|left|, |top|) points in the content of |ui_view|'s
// window. The UI has placed it inside the window, so the host doesn't clamp it.
void host_show_tooltip(void* ui_view, double left, double top, double fade);

// Fades the tooltip's window out over |fade| seconds, then hides it.
void host_hide_tooltip(double fade);

// Makes the window of |link_view| the link target's, which floats over a browser window as its
// child when shown, and lets the mouse through and never takes the keyboard. It starts hidden.
// |pointer| is called with the pointer's place in the content of the browser window, in points
// from its top left, at every move of it while that window shows.
void host_add_link_target(void* link_view, void (*pointer)(double x, double y));

// Shows the link target's window as the UI draws it now, fading in over |fade| seconds from where
// it is, or moves it, with its top left at (|left|, |top|) points in the content of |ui_view|'s
// window. The UI has placed it inside the window, so the host doesn't clamp it.
void host_show_link_target(void* ui_view, double left, double top, double fade);

// Fades the link target's window out over |fade| seconds, then hides it.
void host_hide_link_target(double fade);

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

// Where host_menu shows a menu.
#define HOST_IN_WINDOW 0
#define HOST_IN_REVEAL 1
#define HOST_IN_SETTINGS 2

// Shows a menu of the |count| |items| at |x|, |y|, in points from the top-left of the content of
// |ui_view|'s window, its reveal panel's or the Settings window's (|in|), as a right-click there
// does. With |over| an item's index, that item's title goes at |x|, |y| instead, as a pop-up
// button shows its menu over its own title. Returns the index of the one chosen once it closes,
// or -1.
int host_menu(void* ui_view,
              const host_menu_item* items,
              size_t count,
              double x,
              double y,
              int in,
              int over);

// Shows a menu as host_menu does, at |x|, |y| in points from the top-left of |view|, a window's
// content, such as a text field's context menu in whichever window has the field.
int host_menu_in(const host_menu_item* items, size_t count, double x, double y, void* view);

// Asks |message| in an alert, with |detail| under it and the |count| |buttons|, the first the
// default and a "Cancel" one taking escape, and returns the index of the one chosen once it
// closes.
int host_ask(const char* message, const char* detail, const char* const* buttons, size_t count);

// Shows the close, minimize and zoom buttons of |ui_view|'s window, or hides them, as Arc does
// while its sidebar is hidden.
void host_show_window_buttons(void* ui_view, bool shown);

// Does what a click on the close (0), minimize (1) or zoom (2) button of |ui_view|'s window does.
void host_press_window_button(void* ui_view, int button);

// Makes the window of |reveal_view| the sidebar reveal of |ui_view|'s window, which shows over its
// left edge without taking the keyboard. It starts hidden. While it shows, |pointer_moved| hears,
// with |ui_view|, where the pointer is across the window's content, in points from its left edge,
// as it moves anywhere in the app's windows, and an x past them all when the app stops being the
// active one.
void host_add_reveal(void* ui_view,
                     void* reveal_view,
                     void (*pointer_moved)(void* ui_view, double x));

// Shows |ui_view|'s reveal window, |width| points wide from its browser window's left edge and as
// tall as its content, or makes it that wide if it shows.
void host_show_reveal(void* ui_view, double width);

// Hides |ui_view|'s reveal window.
void host_hide_reveal(void* ui_view);

// Makes the window of |settings_view| the Settings window: a standard one, dark, that can show
// over the browser window in full screen. It starts hidden.
void host_add_settings(void* settings_view);

// Shows the Settings window, as the UI draws it now, in front and with the keyboard. The first
// time, it's centred over the browser window in front, on its display.
void host_show_settings(void);

// Hides the Settings window, which keeps its place for the next host_show_settings.
void host_hide_settings(void);

// Closes the window with the keyboard if it's one of Chrome's with a close button, such as its
// Task Manager or DevTools undocked, as ⌘W does in Chrome, and says whether it did.
bool host_close_other_window(void);

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

// A link or a file that a drag from the page or another app carries.
typedef struct {
  // Its URL, UTF-8.
  const char* url;
  // Whether it's a file's, dragged as a file, rather than a link or text.
  bool file;
} host_dragged;

// What host_take_drops hears of a drag.
enum { HOST_DRAG_OVER, HOST_DRAG_LEFT, HOST_DRAG_DROP };

// Hands drags over the UI's views, from the page or another app, to |dragged| with the view:
// HOST_DRAG_OVER as the pointer comes or moves to (x, y), in points from the view's top-left
// corner, HOST_DRAG_LEFT once it leaves or the drag ends elsewhere, and HOST_DRAG_DROP as it's let
// go there. |items| are what the drag carries. |dragged| returns whether a drop there opens any of
// them, or opened them.
void host_take_drops(bool (*dragged)(void* ui_view, int phase, double x, double y,
                                     const host_dragged* items, size_t count));

// Moves the file at |path|, a full path in UTF-8, to the system's Trash, and says whether it's gone
// from there, as it is if it was gone already.
bool host_trash(const char* path);

// Starts the system's drag of the file at |path|, a full path in UTF-8, from the pointer, during
// the event of the left button held and moved. Anywhere it's let go may take a copy of it, the
// UI's view as host_take_drops says. Calls |ended| once it's let go, with whether something took
// it. Returns false, and never calls |ended|, if it didn't start, as when there's no file there.
bool host_drag_file(const char* path, void (*ended)(bool taken));

// Looks up a preview of the file at |path|, a full path in UTF-8, off the main thread: for an
// image, a thumbnail at most 96 pixels on its longer side, its aspect kept, else the system's icon
// for the file, 52 pixels square. Then calls |done| on the main thread with |id|, the preview as a
// PNG |size| bytes long, |width| by |height| pixels, and whether it's an image's thumbnail, or with
// a null |png| if there's no file there.
void host_file_preview(const char* path, uint64_t id,
                       void (*done)(uint64_t id, const uint8_t* png, size_t size, double width,
                                    double height, bool image));

// Puts |text| on the clipboard. A driven run puts it on a pasteboard of its own instead, so checks
// never touch the user's clipboard.
void host_copy(const char* text);

// Opens |url| in Safari, which plays the protected video the engine can't. A driven run only logs it,
// so checks never launch Safari.
void host_open_in_safari(const char* url);

// Shows the system's share menu for the page at |url| titled |title|, hanging from the frame at
// |x|, |y|, |width| by |height| points in |ui_view|'s window's content.
void host_share(void* ui_view,
                double x,
                double y,
                double width,
                double height,
                const char* url,
                const char* title);

// The starts of today, yesterday, this week and last week in the user's calendar and time zone, in
// milliseconds since the Unix epoch.
void host_day_starts(double starts[4]);

// For the test driver: chooses the item titled |title| in the menu shown, or the button in the
// alert shown. False if neither shows one.
bool host_choose(const char* title);

// For the test driver: the NSWindow of the browser window whose view host_ui_view made |index|th,
// from 0, once it's shown, else null.
void* host_browser_window(size_t index);

#ifdef __cplusplus
}
#endif
