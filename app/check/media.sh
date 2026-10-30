# A tab in a call says so on its row, as Chrome's tab strip does: the camera while its page uses
# the camera, the microphone alone, or the screen while it shares a screen, a window or a tab, and
# the tab it shares shows the screen too. While it shares a tab, another tab's menu has Share This
# Tab Instead, which moves the share there. The marks go once the call ends. A call floats in
# Picture in Picture while its tab is hidden, and comes back with it. A tab playing music out of
# sight has a player at the sidebar's foot, whose button pauses it, whose click shows the tab, and
# whose × puts it away. View › Cast… and the page menu's Cast… open the tab's Cast card, and ×
# closes it. Chromium's fake camera and microphone stand in for real ones, and the share takes the
# tab titled Shared without its picker; the question for the camera is answered with Allow, as the
# user would, since the fake question Chromium can put in its place never tells the browser what's
# captured.

serve
# "c" calls with the camera and microphone, "s" shares the tab titled Shared, "e" ends both, "o"
# opens that tab, the same page with ?Shared, whose title is what follows the ?, "p" one titled
# Other, and "m" plays ten seconds of silence over and over, as music titled Quiet with a next
# track: long enough for Chrome to give it controls, and no sound.
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
    } else if (e.key == "m") {
      const rate = 8000, n = rate * 10, wav = new DataView(new ArrayBuffer(44 + n));
      const text = (at, s) => [...s].forEach((c, i) => wav.setUint8(at + i, c.charCodeAt(0)));
      text(0, "RIFF"); wav.setUint32(4, 36 + n, true); text(8, "WAVEfmt ");
      wav.setUint32(16, 16, true); wav.setUint16(20, 1, true); wav.setUint16(22, 1, true);
      wav.setUint32(24, rate, true); wav.setUint32(28, rate, true); wav.setUint16(32, 1, true);
      wav.setUint16(34, 8, true); text(36, "data"); wav.setUint32(40, n, true);
      for (let i = 0; i < n; i++) {
        wav.setUint8(44 + i, 128);
      }
      const music = new Audio(URL.createObjectURL(new Blob([wav], {type: "audio/wav"})));
      music.loop = true;
      music.onplaying = () => document.title = "playing";
      music.onpause = () => document.title = "paused";
      navigator.mediaSession.metadata = new MediaMetadata({title: "Quiet", artist: "Check"});
      navigator.mediaSession.setActionHandler("nexttrack", () => document.title = "next track");
      await music.play();
    } else if (e.key == "o" || e.key == "p") {
      window.open(e.key == "o" ? "call.html?Shared" : "call.html?Other");
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

# The tab the call shares and the one it shares instead, opened from it, then the call's tab
# shown again. Each opens under it, so the call's tab is found by its place in Today.
# shown <tab>: shows <tab> by its ⌘ number. shown_again <tab>: and gives its page the keyboard.
shown() {
  n=$(today | tr ' ' '\n' | grep -nx "$1" | cut -d: -f1)
  switch "$((n + $(pinned | wc -w))) cmd" "$1"
}
shown_again() {
  shown "$1"
  send click $page_point
}
send key o
expect "^browser: today \($a [0-9][0-9]*\|[0-9][0-9]* $a\)\$" \
  "the page didn't open the tab to share"
b=$(today | tr ' ' '\n' | grep -vx "$a")
expect "^browser: title $b Shared\$" "the tab to share didn't draw"
shown_again "$a"
send key p
expect "^browser: today .* .* .*\$" "the page didn't open the tab to share instead"
c=$(today | tr ' ' '\n' | grep -vx "$a" | grep -vx "$b")
expect "^browser: title $c Other\$" "the tab to share instead didn't draw"
shown_again "$a"

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
expect "^browser: page $c share instead 1\$" "the other tab wasn't offered to share instead"
../tools/window-screenshot.sh "$pid" "$out/media-sharing.png" || fail "no window to capture"

# The other tab's menu, from its row, moves the share there.
post rightclick $(row "$(($(today | tr ' ' '\n' | grep -nx "$c" | cut -d: -f1) - 1))")
expect "^browser: menu shown .*$(cased 'Share This Tab Instead')" \
  "the other tab's menu didn't offer Share This Tab Instead"
send choose "$(cased 'Share This Tab Instead')"
expect "^browser: page $c camera 0 microphone 0 sharing 0 shared 1\$" \
  "Share This Tab Instead didn't share the other tab"
expect "^browser: page $b camera 0 microphone 0 sharing 0 shared 0\$" \
  "Share This Tab Instead left the first tab shared"

# Chrome shows the tab a page starts sharing, unless the page asks it not to, so the call's tab is
# shown again for its "e".
[ "$(grep '^browser: selected ' "$log" | tail -1 | cut -d' ' -f3)" = "$a" ] || shown "$a"
send click $page_point
send key e
expect "^browser: page $a camera 0 microphone 0 sharing 0 shared 0\$" \
  "the call's tab kept its mark once the call ended"
expect "^browser: page $c camera 0 microphone 0 sharing 0 shared 0\$" \
  "the tab shared kept its mark once the share ended"

# A call floats once its tab is hidden, as Arc's does: Chrome's automatic Picture in Picture for a
# page using the camera that asks for it with its media session, on https or a file alone. So a
# file's page, from the command bar, whose "c" calls and which says when its window floats and
# lands. Showing its tab again brings the call back.
printf '%s' '<title>loading</title><body style="background:#333">
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
navigator.mediaSession.setActionHandler("enterpictureinpicture", async () => {
  const floating = await documentPictureInPicture.requestWindow({width: 320, height: 180});
  document.title = "floating";
  floating.addEventListener("pagehide", () => document.title = "landed");
});
onkeydown = async e => {
  if (e.key == "c") {
    window.call = await navigator.mediaDevices.getUserMedia({video: true, audio: true});
    document.title = "in a call";
  } else if (e.key == "e") {
    call.getTracks().forEach(track => track.stop());
    document.title = "ended";
  }
};
</script>' > "$out/site/float.html"
send key t cmd
expect '^browser: command bar shown$' "the command bar didn't open for the floating call"
send type "file://$(cd "$out/site" && pwd)/float.html"
send key return
expect "^browser: loaded [0-9]* file:///.*/float.html\$" "the floating call's page didn't load"
f=$(grep '^browser: loaded [0-9]* file:///.*/float.html$' "$log" | tail -1 | cut -d' ' -f3)
expect "^browser: title $f ready\$" "the floating call's page didn't draw"
send click $page_point
send key c
expect "^browser: question $f takes input\$" "the floating call didn't ask for the camera"
send layout
read -r qx qy qw qh <<EOF
$(grep "^browser: question $f at " "$log" | tail -1 | cut -d' ' -f5-)
EOF
send click $((qx + qw * 3 / 4)) $((qy + qh - 26))
expect "^browser: title $f in a call\$" "Allow didn't give the floating call the camera"
shown "$b"
expect "^browser: title $f floating\$" "the call didn't float with its tab hidden"
shown_again "$f"
expect "^browser: title $f landed\$" "the floating call didn't come back with its tab"
send key e
expect "^browser: title $f ended\$" "e didn't end the floating call"
shown_again "$a"

# Music in the call's tab, then another tab shown: the tab playing out of sight has a player.
send key m
expect "^browser: title $a playing\$" "m in the page didn't play its silence"
expect "^browser: page $a media 1 playing 1 next 1 Quiet\$" "the page's music had no controls"
none_since "^browser: media players [0-9]" || fail "the tab shown had a player"
shown "$b"
expect "^browser: media players $a\$" "the tab playing out of sight had no player"
expect "^browser: media player $a at " "the player wasn't drawn"
read -r mx my mw mh <<EOF
$(grep "^browser: media player $a at " "$log" | tail -1 | cut -d' ' -f6-)
EOF
../tools/window-screenshot.sh "$pid" "$out/media-player.png" || fail "no window to capture"
# Its buttons from the right, 8 in, 24 wide and 6 apart: ×, the next track, then pause.
button_x() { echo $((mx + mw - 8 - 12 - $1 * 30)); }
send click "$(button_x 2)" $((my + mh / 2))
expect '^browser: command media-pause via player$' "the player's pause didn't take the click"
expect "^browser: title $a paused\$" "the player's pause didn't pause the page"
expect "^browser: page $a media 1 playing 0 next 1 Quiet\$" "the player didn't show it paused"
send click $((mx + 60)) $((my + mh / 2))
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the player didn't show its tab"
expect '^browser: media players $' "the tab shown kept its player"
shown "$b"
expect "^browser: media players $a\$" "the paused tab out of sight had no player"
send click "$(button_x 0)" $((my + mh / 2))
expect '^browser: command media-dismiss via player$' "the player's × didn't take the click"
expect '^browser: media players $' "× didn't put the player away"

# The Cast card, from View › Cast… and from the page menu, with whatever devices the network has,
# none clicked. Linux alone: the Mac asks for the local network on the screen as Chrome looks with
# DIAL, and Windows' firewall may.
if [ "$(uname)" = Linux ]; then
  send menu "Cast…"
  expect "^browser: cast $b .* devices [0-9]*\$" "View › Cast… didn't open the tab's Cast dialog"
  expect "^browser: cast $b at " "the Cast card wasn't drawn"
  read -r kx ky kw kh <<EOF
$(grep "^browser: cast $b at " "$log" | tail -1 | cut -d' ' -f5-)
EOF
  ../tools/window-screenshot.sh "$pid" "$out/media-cast.png" || fail "no window to capture"
  # Its ×, 7 inside its top right corner and 22 square.
  send click $((kx + kw - 18)) $((ky + 18))
  expect '^browser: command cast-close via card$' "the Cast card's × didn't take the click"
  expect "^browser: cast $b closed\$" "× didn't close the Cast dialog"
  post rightclick $page_point
  expect '^browser: menu shown .*Cast…' "the page's menu had no Cast…"
  send choose "Cast…"
  expect "^browser: cast $b .* devices [0-9]*\$" "the page menu's Cast… didn't open the Cast dialog"
  send menu "Cast…"
  expect "^browser: cast $b closed\$" "View › Cast… again didn't close the Cast dialog"
else
  echo "skip: the Cast card on $(uname). Chrome looking for devices would ask for the network."
fi
exec 3>&-
quit

echo "ok: a tab in a call marks its row with the camera, then the screen while it shares a tab, the tab it shares shows the screen too, another tab's Share This Tab Instead moves the share there, and the marks go when the call ends; a call floats while its tab is hidden; music playing out of sight has a player that pauses it, shows its tab and goes on ×; the Cast card opens from View and the page menu and closes on ×."
