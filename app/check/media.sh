# A tab in a call says so on its row, as Chrome's tab strip does: the camera while its page uses
# the camera, the microphone alone, or the screen while it shares a screen, a window or a tab, and
# the tab it shares shows the screen too. The marks go once the call ends. Chromium's fake camera
# and microphone stand in for real ones, and the share takes the tab titled Shared without its
# picker; the question for the camera is answered with Allow, as the user would, since the fake
# question Chromium can put in its place never tells the browser what's captured.

serve
# "c" calls with the camera and microphone, "s" shares the tab titled Shared, "e" ends both, and
# "o" opens that tab, the same page with ?Shared, whose title is what follows the ?.
printf '%s' '<title>loading</title><body style="background:#333">
<script>
const name = decodeURIComponent(location.search.slice(1));
requestAnimationFrame(() => setTimeout(() => document.title = name || "ready"));
let streams = [];
onkeydown = async e => {
  try {
    if (e.key == "c") {
      streams.push(await navigator.mediaDevices.getUserMedia({video: true, audio: true}));
      document.title = "in a call";
    } else if (e.key == "s") {
      streams.push(await navigator.mediaDevices.getDisplayMedia({video: true}));
      document.title = "sharing";
    } else if (e.key == "e") {
      streams.forEach(stream => stream.getTracks().forEach(track => track.stop()));
      streams = [];
      document.title = "ended";
    } else if (e.key == "o") {
      window.open("call.html?Shared");
    }
  } catch (error) {
    document.title = "refused " + error.name;
  }
};
</script>' > "$out/site/call.html"
log="$out/media.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="${site}call.html" "$browser" --use-fake-device-for-media-stream \
  --auto-select-tab-capture-source-by-title=Shared < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the call page didn't draw"
a=$(today | cut -d' ' -f1)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: media. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi

# The tab the call shares, opened under it, then the call's tab shown again.
send key o
expect "^browser: today \($a [0-9][0-9]*\|[0-9][0-9]* $a\)\$" "the page didn't open the tab to share"
b=$(today | tr ' ' '\n' | grep -vx "$a")
expect "^browser: title $b Shared\$" "the tab to share didn't draw"
n=$(today | tr ' ' '\n' | grep -nx "$a" | cut -d: -f1)
switch "$((n + $(pinned | wc -w))) cmd" "$a"
send click $page_point

send key c
expect "^browser: page $a asks ${site%/} to " "the call didn't ask for the camera and microphone"
expect "^browser: question $a at " "the call's question wasn't drawn"
expect "^browser: question $a takes input\$" "the call's question never took input"
send layout
read -r qx qy qw qh <<EOF
$(grep "^browser: question $a at " "$log" | tail -1 | cut -d' ' -f5-)
EOF
send click $((qx + qw * 3 / 4)) $((qy + qh - 26))
expect "^browser: title $a in a call\$" "Allow didn't give the call the camera and microphone"
expect "^browser: page $a camera 1 microphone 1 sharing 0 shared 0\$" \
  "the call's tab didn't show it uses the camera"
../tools/window-screenshot.sh "$pid" "$out/media-call.png" || fail "no window to capture"

send click $page_point
send key s
expect "^browser: title $a sharing\$" "the call didn't share the tab titled Shared"
expect "^browser: page $a camera 1 microphone 1 sharing 1 shared 0\$" \
  "the call's tab didn't show it shares"
expect "^browser: page $b camera 0 microphone 0 sharing 0 shared 1\$" \
  "the tab shared didn't show it's shared"
../tools/window-screenshot.sh "$pid" "$out/media-sharing.png" || fail "no window to capture"

send key e
expect "^browser: page $a camera 0 microphone 0 sharing 0 shared 0\$" \
  "the call's tab kept its mark once the call ended"
expect "^browser: page $b camera 0 microphone 0 sharing 0 shared 0\$" \
  "the tab shared kept its mark once the share ended"
exec 3>&-
quit

echo "ok: a tab in a call marks its row with the camera, then the screen while it shares a tab, the tab it shares shows the screen too, and the marks go when the call ends."
