# Spike (#359), on spike/live-mirror-359 only. Two windows side by side, and a page that paints
# every frame moved from the first to the second: the first draws its picture, and with
# MIRROR=host or MIRROR=portal, a live mirror of the page over it (BROWSER_MIRROR in the core). The
# run checks that the first window's card changes between two captures while the page paints only
# with a mirror, then measures for MIRROR_SECONDS (20) while the page paints and again once it's
# still: the CPU time of the app, its GPU process, its renderers and the window server, their idle
# wakeups, and with sudo that needs no password, the GPU's active residency from powermetrics.
# Without MIRROR it measures the picture alone, the baseline. Lines go to build/check/mirror.txt.
# ../spikes/live-mirror/measure.sh runs it for each mode, in turns.

mode="${MIRROR:-picture}"
seconds="${MIRROR_SECONDS:-20}"
# The page fills itself with a new colour and moves a square every frame, from the main thread,
# until a click stops it. No percent signs: the URL would read them as escapes.
page='data:text/html,<title>moving</title>
<body style="margin:0;overflow:hidden"><canvas id="c"></canvas>
<script>
const c = document.getElementById("c"), g = c.getContext("2d");
let on = true, t = 0;
function size() { c.width = innerWidth; c.height = innerHeight; }
size();
onresize = size;
function frame() {
  if (!on) return;
  t++;
  g.fillStyle = "rgb(" + (t * 3 & 255) + ",80,160)";
  g.fillRect(0, 0, c.width, c.height);
  g.fillStyle = "white";
  g.fillRect(t * 4 - c.width * Math.floor(t * 4 / c.width), 100, 80, 80);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
onclick = () => { on = false; document.title = "still"; };
</script>'
log="$out/mirror.log"
results="$out/mirror.txt"
step=0
rm -f "$saved" "$launch_frame"
if [ -n "${MIRROR:-}" ]; then
  export BROWSER_MIRROR="$MIRROR"
else
  unset BROWSER_MIRROR
fi
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: window opened [0-9]' || fail "the app didn't say which window opened"
first=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
a=$(today)
send click $sidebar_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: mirror. Browser wasn't the active app."
  exit 0
fi
# Side by side, so the window server draws both whole.
send place 0 40 600 600
send key n cmd
expect '^browser: window opened [0-9]' "⌘N didn't open a window"
second=$(grep '^browser: window opened ' "$log" | tail -1 | cut -d' ' -f4)
send window 2
send place 620 40 600 600
send click $(row 0)
expect "^browser: window $second shows $a\$" "a click on the page's row didn't show it in the new window"
expect "^browser: window $first pictures tab $a\$" "the first window got no picture of its page"
expect "^browser: window $first picture rests\$" "the first window's picture didn't step back"
if [ -n "${MIRROR:-}" ]; then
  await '^browser: mirror \(hosts context\|portals\)' ||
    fail "the first window didn't mirror the page: $(grep '^browser: mirror' "$log" | tail -1)"
else
  grep -q '^browser: mirror' "$log" && fail "the first window mirrored the page without MIRROR"
fi

# The first window's card, the left 45 percent of a capture of both windows.
card() {
  ../tools/window-screenshot.sh "$pid" "$out/mirror-both.png" all || fail "no window to capture"
  width=$(sips -g pixelWidth "$out/mirror-both.png" | awk '/pixelWidth/ { print $2 }')
  height=$(sips -g pixelHeight "$out/mirror-both.png" | awk '/pixelHeight/ { print $2 }')
  sips -c "$height" $((width * 45 / 100)) --cropOffset 0 0 "$out/mirror-both.png" \
    --out "$1" > /dev/null
}
card "$out/mirror-$mode-1.png"
card "$out/mirror-$mode-2.png"
if cmp -s "$out/mirror-$mode-1.png" "$out/mirror-$mode-2.png"; then
  [ -z "${MIRROR:-}" ] || fail "the mirror didn't change while the page painted"
else
  [ -n "${MIRROR:-}" ] || fail "the first window's picture changed while the page painted"
fi

# CPU seconds a process has used, from ps's [hh:]mm:ss.cc.
cpu_of() {
  ps -o time= -p "$1" 2> /dev/null |
    awk '{ n = split($1, part, ":"); s = 0; for (i = 1; i <= n; i++) s = s * 60 + part[i]; print s }'
}
# The app's helper processes of one type, such as gpu-process or renderer.
helpers_of() {
  pgrep -f "$app/Contents/.*--type=$1" | tr '\n' ' '
}
# Measures for $seconds and writes a line to mirror.txt for phase $1.
measure() {
  gpu=$(helpers_of gpu-process)
  renderers=$(helpers_of renderer)
  server=$(pgrep -x WindowServer | head -1)
  groups="browser:$pid gpu-process:$(echo $gpu | tr ' ' ,) renderers:$(echo $renderers | tr ' ' ,)"
  groups="$groups windowserver:$server"
  pids="$pid $gpu $renderers $server"
  before=""
  for each in $pids; do
    before="$before $each:$(cpu_of "$each")"
  done
  if sudo -n true 2> /dev/null; then
    sudo -n powermetrics --samplers gpu_power -i 1000 -n "$seconds" \
      > "$out/mirror-$mode-$1-powermetrics.txt" 2>&1 &
    metrics=$!
  else
    metrics=""
  fi
  # Two samples $seconds apart: their idle wakeups, which top counts from each process's start.
  args=""
  for each in $pids; do
    args="$args -pid $each"
  done
  top -l 2 -s "$seconds" -stats pid,idlew $args > "$out/mirror-$mode-$1-top.txt" 2>&1
  [ -z "$metrics" ] || wait "$metrics" || true
  after=""
  for each in $pids; do
    after="$after $each:$(cpu_of "$each")"
  done
  gpu_active=n/a
  if [ -n "$metrics" ]; then
    gpu_active=$(awk '/GPU HW active residency/ { gsub("%", "", $5); s += $5; n++ }
      END { if (n) printf "%.1f percent", s / n; else print "n/a" }' \
      "$out/mirror-$mode-$1-powermetrics.txt")
  fi
  line=$(awk -v groups="$groups" -v before="$before" -v after="$after" -v seconds="$seconds" '
    /^Processes:/ { sample++ }
    $1 ~ /^[0-9]+$/ && NF >= 2 { v = $2; gsub("[^0-9]", "", v); wakes[sample, $1] = v }
    END {
      split(before, b, " "); for (i in b) { split(b[i], kv, ":"); cpu0[kv[1]] = kv[2] }
      split(after, a, " "); for (i in a) { split(a[i], kv, ":"); cpu1[kv[1]] = kv[2] }
      n = split(groups, g, " ")
      for (i = 1; i <= n; i++) {
        split(g[i], nv, ":"); m = split(nv[2], ids, ",")
        cpu = 0; w = 0; known = 1
        for (j = 1; j <= m; j++) {
          cpu += cpu1[ids[j]] - cpu0[ids[j]]
          if (wakes[1, ids[j]] == "" || wakes[2, ids[j]] == "") known = 0
          else w += wakes[2, ids[j]] - wakes[1, ids[j]]
        }
        out = out sprintf("%s%s cpu %.2f s, %s idle wakeups/s", i > 1 ? "; " : "", nv[1], cpu,
          known ? sprintf("%.1f", w / seconds) : "n/a")
      }
      print out
    }' "$out/mirror-$mode-$1-top.txt")
  echo "mirror: $mode $1 over $seconds s: $line; GPU active $gpu_active" | tee -a "$results"
}

measure painting
send window 2
send click $page_point
expect "^browser: title $a still\$" "a click on the page didn't stop it"
measure still
exec 3>&-
quit
echo "ok: mirror, $mode: $(grep -c "^mirror: $mode " "$results") measures in $results"
