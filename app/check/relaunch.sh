# Launched again after ⌘Q, the app brings back every regular window that was open, as Arc and
# Chrome do: each at the frame it had, in the Space it showed, stacked as they were, with the one
# in front key. A window closed before the quit, and a private window, don't come back. Reports
# the launch's times with one window brought back and with two.

page='data:text/html,<title>relaunch</title>'
# Milliseconds from the process's start to the line "browser: $1 at <ms> ms".
launch_at() {
  grep -m1 "^browser: $1 at " "$log" | sed 's/.* at \([0-9.]*\) ms$/\1/'
}
# The frame logged last for the window with the id $1, as "<x> <y> <width> <height>".
frame_of() {
  grep "^browser: window $1 frame " "$log" | tail -1 | cut -d' ' -f5-
}
# The name of the Space the window with the id $1 shows.
space_of() {
  grep "^browser: window $1 in space " "$log" | tail -1 | cut -d' ' -f6-
}
# The ids of the windows opened, in the order they opened.
windows_opened() {
  grep '^browser: window opened ' "$log" | cut -d' ' -f4
}
# Starts the app with the log $1, and the page $2 if it's given.
launch_logged() {
  log="$1"
  step=0
  if [ -n "${2:-}" ]; then
    BROWSER_DRIVE=1 BROWSER_URL="$2" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
  else
    BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
  fi
  pid=$!
  exec 3> "$out/drive"
}

# A first launch, with nothing saved, opens one window as it always has. Moved and resized, it
# comes back there.
rm -f "$saved" "$launch_frame"
launch_logged "$out/relaunch-a.log" "$page"
await_load
await '^browser: window opened [0-9]' || fail "the first window didn't open"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: relaunch. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
send place 60 80 1000 700
quit_by_key
exited

launch_logged "$out/relaunch-b.log"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: first frame at ' || fail "the restored window didn't draw"
await '^browser: window opened [0-9]' || fail "the restored window didn't open"
first=$(windows_opened | head -1)
[ "$(frame_of "$first")" = "60 80 1000 700" ] ||
  fail "the window came back at $(frame_of "$first"), not where it was at 60 80 1000 700"
one="window on screen $(launch_at 'window on screen') ms, first frame $(launch_at 'first frame') ms"

# Two windows in two Spaces, of different sizes and places, with the second key. A third closes
# before the quit, and a private window stays open over the first.
send key n cmd
expect '^browser: command new-window via ' "⌘N didn't run New Window"
expect '^browser: window opened [0-9]' "⌘N didn't open a window"
second=$(windows_opened | tail -1)
send window 2
send place 320 140 900 640
send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run in the second window"
send type reading
send key return
expect '^browser: create space [0-9]* reading$' "return didn't name the second window's Space"
await "^browser: window $second in space reading\$" || fail "the second window didn't show its Space"
send key n cmd
expect '^browser: window opened [0-9]' "⌘N didn't open a third window"
third=$(windows_opened | tail -1)
send window 3
send key w cmd shift
expect "^browser: window closed $third\$" "⇧⌘W didn't close the third window"
send window 1
send key n cmd shift
expect '^browser: private window opened [0-9]' "⇧⌘N didn't open a private window"
send window 2
send order
expect '^browser: order window 2 window 4 window 1, key window 2$' \
  "before the quit, the windows weren't the second, the private one and the first, front to back"
quit_by_key
exited

# Back, each at its frame, in its Space, the second key and in front of the first, and nothing
# else.
launch_logged "$out/relaunch-c.log"
await '^browser: restored in ' || fail "the relaunch with two windows didn't restore the Spaces"
await '^browser: first frame at ' || fail "the first restored window didn't draw"
# The windows after the first open once it has drawn; the one behind shows Space 1.
await '^browser: window [0-9]* in space Space 1$' || fail "the window behind didn't come back"
[ "$(windows_opened | wc -l)" -eq 2 ] || fail "$(windows_opened | wc -l | tr -d ' ') windows came back, not 2"
grep -q '^browser: private window opened ' "$log" && fail "the private window came back"
front=$(windows_opened | head -1)
back=$(windows_opened | tail -1)
[ "$(frame_of "$front")" = "320 140 900 640" ] ||
  fail "the window in front came back at $(frame_of "$front"), not at 320 140 900 640"
[ "$(frame_of "$back")" = "60 80 1000 700" ] ||
  fail "the window behind came back at $(frame_of "$back"), not at 60 80 1000 700"
await "^browser: window $front in space reading\$" ||
  fail "the window in front came back in $(space_of "$front"), not reading"
await "^browser: window $back in space Space 1\$" ||
  fail "the window behind came back in $(space_of "$back"), not Space 1"
await "^browser: window $back shows [1-9][0-9]*\$" || fail "the window behind came back without its page"
send order
expect '^browser: order window 1 window 2, key window 1$' \
  "the windows came back out of order, or the one in front wasn't key"
../tools/window-screenshot.sh "$pid" "$out/relaunch-two.png" all || fail "no window to capture"
two="window on screen $(launch_at 'window on screen') ms, first frame $(launch_at 'first frame') ms"
exec 3>&-
quit

echo "ok: after ⌘Q, every regular window comes back at its frame, in its Space, stacked as it was with the one in front key; a window closed before the quit and a private window don't. Window: $out/relaunch-two.png"
echo "  Launch with one window: $one; with two: $two"
