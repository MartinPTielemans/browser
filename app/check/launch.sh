# A second launch while the browser runs hands its command line to the running browser over
# Chromium's ProcessSingleton and exits (core/main_delegate.cc, LaunchedAgain). On Linux, with ilo
# the default browser in its desktop file (platform/linux/flatpak), `xdg-open` with a link, as
# another app sends one, opens it as a new tab in the running window's Space. The desktop file's
# New Window action opens a window as ⌘N's and New Incognito Window a private one as ⇧⌘N's. A
# launch with nothing brings the window in front forward. Each second process exits, so it never
# became a browser of its own. The Mac's links come through AppKit instead (links.sh), and
# Windows' check doesn't drive a second launch yet.

launch_check() {
  if [ "$(uname)" != Linux ]; then
    echo "skip: launch. Only Linux's check makes a second launch so far."
    return 0
  fi
  serve
  log="$out/launch.log"
  step=0
  rm -f "$saved" "$launch_frame"
  BROWSER_DRIVE=1 BROWSER_URL="$site" "$browser" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await_load
  first=$(space)
  window=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
  # xdg-open finds ilo as the default browser in the desktop file, with this build's chrome for
  # `ilo`, in folders of the run's own, so a desktop's own default is left alone.
  scratch=$(mktemp -d)
  mkdir -p "$scratch/data/applications" "$scratch/config"
  desktop="$scratch/data/applications/app.ilo.browser.desktop"
  sed "s|^Exec=ilo|Exec=$browser|" platform/linux/flatpak/app.ilo.browser.desktop > "$desktop"
  XDG_DATA_HOME="$scratch/data" XDG_CONFIG_HOME="$scratch/config" \
    xdg-mime default app.ilo.browser.desktop x-scheme-handler/http
  # The Exec line of the desktop file's action $1, as a desktop runs it.
  action() {
    awk -v group="[Desktop Action $1]" '
      /^\[/ { on = $0 == group }
      on && /^Exec=/ { sub(/^Exec=/, ""); print; exit }' "$desktop"
  }
  # Waits up to 30 s for the log to hold $2 lines matching $1.
  await_count() {
    for _ in $(seq 300); do
      [ "$(grep -c "$1" "$log")" -ge "$2" ] && return 0
      sleep 0.1
    done
    return 1
  }
  # Launches the browser again with $@, and waits up to 30 s for that process to exit, as it does
  # once the running browser has its command line.
  again=0
  launch_again() {
    again=$((again + 1))
    XDG_DATA_HOME="$scratch/data" XDG_CONFIG_HOME="$scratch/config" "$@" \
      > "$out/launch-again-$again.log" 2>&1 &
    helper=$!
    for _ in $(seq 300); do
      kill -0 "$helper" 2>/dev/null || break
      sleep 0.1
    done
    kill -0 "$helper" 2>/dev/null &&
      fail "the second launch $* didn't exit: $out/launch-again-$again.log"
    wait "$helper" || true
    helper=
  }

  launch_again xdg-open "${site}?again"
  line="^browser: link from another app ${site}?again: tab [1-9][0-9]* in space $first of window $window\$"
  await "$line" ||
    fail "xdg-open's link didn't open in Space $first of window $window: $(grep '^browser: link' "$log" | tail -1)"
  tab=$(grep "$line" "$log" | tail -1 | sed 's/.*: tab \([0-9]*\) .*/\1/')
  await "^browser: loaded $tab ${site}?again\$" || fail "xdg-open's link didn't load in tab $tab"
  await "^browser: launched again: 0$" || fail "the running browser didn't hear of xdg-open's launch"
  ../tools/window-screenshot.sh "$pid" "$out/launch-link.png" || fail "no window to capture"

  # shellcheck disable=SC2046 # the action's Exec line, split into words as a desktop splits it
  launch_again $(action new-window)
  await "^browser: launched again: 1$" || fail "the running browser didn't hear of New Window"
  await_count '^browser: window opened ' 2 || fail "the desktop file's New Window didn't open a window"
  ../tools/window-screenshot.sh "$pid" "$out/launch-window.png" || fail "no window to capture"

  # shellcheck disable=SC2046
  launch_again $(action new-private-window)
  await "^browser: launched again: 2$" || fail "the running browser didn't hear of New Incognito Window"
  await '^browser: private window opened ' ||
    fail "the desktop file's New Incognito Window didn't open a private window"
  ../tools/window-screenshot.sh "$pid" "$out/launch-private.png" || fail "no window to capture"

  launch_again "$browser"
  await_count '^browser: launched again: 0$' 2 ||
    fail "the running browser didn't hear of a launch with nothing"
  [ "$(grep -c '^browser: window opened ' "$log")" = 3 ] ||
    fail "a launch with nothing opened a window, where it brings the one in front forward"
  rm -rf "$scratch"
  scratch=
  quit
  echo "ok: with the browser running, xdg-open's link opens as a tab in its window's Space, the desktop file's New Window and New Incognito Window open a window and a private one, a launch with nothing opens none, and each second launch exits. Windows: $out/launch-link.png, $out/launch-window.png, $out/launch-private.png"
}
launch_check
