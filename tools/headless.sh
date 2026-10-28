#!/bin/bash
# Runs a program in a sway session of its own, headless, and captures it: nothing reaches the
# owner's screens or input, and the owner's session locking or turning its outputs off doesn't
# stop the frames. sway uses the real GPU and runs Xwayland, so --x11 runs the program on X11 (the
# browser, for now: decision 3), and without it on the session's Wayland socket. Once the program
# has logged a line matching <ready> and the picture has stopped changing, the capture is the
# session's one output; the program's log goes beside it. sway and the program go when it ends.
# --page waits only for the right half, the page's, to stand still, as the preview's sidebar has a
# loading tab that never stops. --scale gives the output that scale, 1 by default, at 1280 by 800
# points. --then runs a
# command once the capture is taken, before the session ends, with the session's WAYLAND_DISPLAY,
# SWAYSOCK and Xwayland's DISPLAY, and the program's log as $LOG: to click (swaymsg seat seat0 cursor ...), type or
# capture again.
# --run runs <command> in the session instead, with no capture, and exits with its status: for
# app/check.sh, which launches the browser itself. It has Xwayland's DISPLAY with --x11, and the
# session's Wayland socket's name in BROWSER_HEADLESS, which tools/window-screenshot.sh captures.
# Usage: tools/headless.sh [--x11] [--page] [--scale <n>] [--then <command>] <output.png> <ready>
#   <program> [arguments...]
#        tools/headless.sh [--x11] [--scale <n>] --run <command> [arguments...]
set -euo pipefail
x11=
still=()
scale=1
then=
run=
while [ $# -gt 0 ]; do
  case $1 in
    --x11) x11=1 ;;
    --page) still=(-gravity east -crop 50%x100%+0+0) ;;
    --scale) scale=$2; shift ;;
    --then) then=$2; shift ;;
    --run) run=1; shift; break ;;
    *) break ;;
  esac
  shift
done
if [ -z "$run" ]; then
  [ $# -ge 3 ] || { echo "usage: $0 [--x11] [--page] [--scale <n>] [--then <command>] <output.png> <ready> <program> [arguments...]" >&2; exit 2; }
  out=$(realpath -m "$1")
  ready=$2
  shift 2
  log=${out%.*}.log
fi
for tool in sway grim magick; do
  command -v "$tool" >/dev/null || { echo "Needs sway, grim and ImageMagick: no $tool." >&2; exit 1; }
done

dir=$(mktemp -d)
sway_pid=
pid=
cleanup() {
  [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
  # A browser's helpers outlive it briefly; they're its children, in its process group.
  [ -z "$pid" ] || kill -- "-$pid" 2>/dev/null || true
  [ -z "$sway_pid" ] || kill "$sway_pid" 2>/dev/null || true
  rm -rf "$dir"
}
trap cleanup EXIT

# The session tells its children's environment, where the Wayland socket and Xwayland's display
# are, through a file once it's up.
cat >"$dir/config" <<EOF
output HEADLESS-1 resolution $(awk "BEGIN { printf \"%dx%d\", 1280 * $scale, 800 * $scale }") scale $scale
xwayland enable
default_border none
exec sh -c 'echo "\$WAYLAND_DISPLAY \$SWAYSOCK \${DISPLAY:-}" > $dir/env'
EOF
# The integrated GPU's, not NVIDIA's: Vulkan, which GPUI draws with, finds no adapter that can
# present to a compositor on NVIDIA's render node.
for render in /dev/dri/renderD*; do
  [ "$(basename "$(readlink -f "/sys/class/drm/${render##*/}/device/driver")")" = nvidia ] || break
done
[ -n "$run" ] || mkdir -p "$(dirname "$out")"
env -u WAYLAND_DISPLAY -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u SWAYSOCK \
  WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDER_DRM_DEVICE=$render \
  sway -c "$dir/config" >"$dir/sway.log" 2>&1 &
sway_pid=$!
for _ in $(seq 100); do
  [ -s "$dir/env" ] && break
  sleep 0.1
done
[ -s "$dir/env" ] || { cat "$dir/sway.log" >&2; echo "The session didn't come up." >&2; exit 1; }
read -r wayland swaysock display <"$dir/env"
[ -z "$x11" ] || [ -n "$display" ] || { echo "The session has no Xwayland." >&2; exit 1; }

if [ -n "$run" ]; then
  status=0
  if [ -n "$x11" ]; then
    env -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u GDK_SCALE DISPLAY="$display" \
      SWAYSOCK="$swaysock" BROWSER_HEADLESS="$wayland" "$@" || status=$?
  else
    env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u GDK_SCALE WAYLAND_DISPLAY="$wayland" \
      SWAYSOCK="$swaysock" BROWSER_HEADLESS="$wayland" "$@" || status=$?
  fi
  exit $status
fi

# The owner's session's scale (Chromium reads GDK_SCALE on X11) isn't this session's, which is 1.
if [ -n "$x11" ]; then
  setsid env -u WAYLAND_DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u GDK_SCALE DISPLAY="$display" "$@" >"$log" 2>&1 &
else
  setsid env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u GDK_SCALE WAYLAND_DISPLAY="$wayland" "$@" >"$log" 2>&1 &
fi
pid=$!
for _ in $(seq 300); do
  grep -qE "$ready" "$log" && break
  kill -0 "$pid" 2>/dev/null || break
  sleep 0.1
done
grep -qE "$ready" "$log" || { tail -30 "$log" >&2; echo "No \"$ready\" in the program's log." >&2; exit 1; }
last=
for _ in $(seq 100); do
  WAYLAND_DISPLAY=$wayland grim "$out"
  now=$(magick "$out" "${still[@]}" rgb:- | sha256sum)
  if [ "$now" = "$last" ]; then
    echo "$out"
    [ -z "$then" ] || WAYLAND_DISPLAY=$wayland SWAYSOCK=$swaysock DISPLAY=$display LOG=$log bash -c "$then"
    exit 0
  fi
  last=$now
  sleep 0.2
done
echo "The picture never stood still." >&2
exit 1
