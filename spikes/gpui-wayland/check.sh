#!/bin/bash
# Runs the spike on a virtual Hyprland output nobody sees, as tools/hypr-preview.sh does the preview,
# and checks that GPUI draws in the host's window, that the pointer reaches GPUI, that keys the host
# forwards reach it, and how fast it draws with the host's loop done right and the two ways wrong.
# The pointer check moves the seat's cursor onto the virtual output for a moment and puts it back.
# Leaves the capture and the logs in build/check/.
set -euo pipefail
cd "$(dirname "$0")"
spike=$PWD/target/release/spike
[ -x "$spike" ] || { echo "No spike. Build it: cargo build --release" >&2; exit 1; }
out=$PWD/build/check
mkdir -p "$out"

output=ilo-spike-$$
pid=
stop() {
  [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
  pid=
}
cleanup() {
  stop
  hyprctl output remove "$output" >/dev/null || true
}
trap cleanup EXIT

monitor() {
  hyprctl monitors -j | python3 -I -c \
    "import json,sys; m=next((m for m in json.load(sys.stdin) if m['name']=='$output'), None); print(m and '%d %d %d' % (m['activeWorkspace']['id'], m['x'], m['y']) or '')"
}
hyprctl output create headless "$output" >/dev/null
for _ in $(seq 50); do
  read -r workspace ox oy <<<"$(monitor)" || true
  [ -n "${workspace:-}" ] && break
  sleep 0.1
done
[ -n "${workspace:-}" ] || { echo "The virtual output didn't come up." >&2; exit 1; }

# Waits for a line the spike prints, up to five seconds.
wait_for() {
  local log=$1 line=$2
  for _ in $(seq 50); do
    grep -q -- "$line" "$log" && return 0
    sleep 0.1
  done
  echo "FAIL: no \"$line\" in $log" >&2
  return 1
}

start() {
  local log=$1 env=$2
  : >"$log"
  hyprctl dispatch exec "[workspace $workspace silent] env $env $spike >$log 2>&1" >/dev/null
  wait_for "$log" "spike: started"
  for _ in $(seq 50); do
    pid=$(hyprctl clients -j | python3 -I -c \
      "import json,sys; print(next((c['pid'] for c in json.load(sys.stdin) if c['class']=='ilo-gpui-wayland-spike'), ''))")
    [ -n "$pid" ] && break
    sleep 0.1
  done
}

# The frames GPUI drew in each of the last three seconds.
frames() {
  sleep 3.5
  grep -o 'frames [0-9]*' "$1" | tail -3 | cut -d' ' -f2 | paste -sd' '
}

log=$out/spike.log
start "$log" SPIKE=1
height=$(sed -n 's/^spike: host configured at [0-9]*x//p' "$log" | tail -1)
wait_for "$log" "spike: ui drawn at 280x$height"
sleep 0.5
grim -o "$output" "$out/spike.png"
echo "drawn: 280x$height, captured in build/check/spike.png"

cursor=$(hyprctl cursorpos | tr -d ' ')
hyprctl dispatch movecursor $((ox + 140)) $((oy + 200)) >/dev/null
wait_for "$log" "spike: ui pointer entered"
hyprctl dispatch movecursor $((ox + 700)) $((oy + 200)) >/dev/null
wait_for "$log" "spike: host pointer entered the page"
hyprctl dispatch movecursor "${cursor%,*}" "${cursor#*,}" >/dev/null
echo "pointer: GPUI's subsurface took it, then the host's page"

hyprctl dispatch sendshortcut ", a, pid:$pid" >/dev/null
wait_for "$log" "spike: ui key a"
hyprctl dispatch sendshortcut ", TAB, pid:$pid" >/dev/null
wait_for "$log" "spike: keyboard to the page"
hyprctl dispatch sendshortcut ", b, pid:$pid" >/dev/null
wait_for "$log" "spike: page key"
grep -q "spike: ui key b" "$log" && { echo "FAIL: the sidebar got the page's key" >&2; exit 1; }
echo "keys: the host forwarded a to GPUI, then kept b for the page"

echo "frames a second, the host dispatching GPUI after its reads: $(frames "$log")"
stop

start "$out/naive.log" SPIKE_NAIVE=1
echo "frames a second, GPUI dispatched only on its own descriptor: $(frames "$out/naive.log")"
stop

start "$out/standalone.log" SPIKE_STANDALONE=1
echo "frames a second, GPUI alone in a window of its own: $(frames "$out/standalone.log")"
stop

start "$out/roundtrip.log" SPIKE_ROUNDTRIP=1
echo "frames a second, with a host roundtrip each turn: $(frames "$out/roundtrip.log")"
stop

start "$out/naive-roundtrip.log" "SPIKE_NAIVE=1 SPIKE_ROUNDTRIP=1"
echo "frames a second, with a host roundtrip each turn and GPUI dispatched only on its own descriptor: $(frames "$out/naive-roundtrip.log")"
stop

: >"$out/hold.log"
hyprctl dispatch exec "[workspace $workspace silent] env SPIKE_HOLD_READ=1 $spike >$out/hold.log 2>&1" >/dev/null
sleep 6
# GPUI reads the socket only when there's data on it, so the deadlock waits for that moment.
returned=$(grep -c "GPUI returned while a read was held" "$out/hold.log" || true)
if [ "$(grep -c "calling GPUI while holding" "$out/hold.log" || true)" -gt "$returned" ]; then
  echo "hold read: GPUI hung on call $((returned + 1)) while the host held a prepared read"
else
  echo "hold read: GPUI returned all $returned times while the host held a prepared read"
fi
pkill -f "SPIKE_HOLD_READ=1 $spike" 2>/dev/null || true
pkill -xf "$spike" 2>/dev/null || true
