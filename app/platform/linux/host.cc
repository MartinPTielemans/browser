// The Linux host: what platform/mac/host.mm does for the UI, on Wayland and X11. So far only the
// preview's part, since the browser needs a Linux engine first (AGENTS.md, Dev).

extern "C" {

// Wayland gives a client no say in where its window goes, nor sight of the displays' layout: the
// compositor places it, Hyprland on the display with the focus. X11 window managers mostly place
// a new window themselves too. So the origin is left for them, and BROWSER_DISPLAY has nothing
// to pick.
void host_centered(double width, double height, double* x, double* y) {
  (void)width;
  (void)height;
  *x = 0;
  *y = 0;
}

}  // extern "C"
