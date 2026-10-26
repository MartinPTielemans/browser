# A site on the list of those whose video needs FairPlay opens in a WebKit tab, a WKWebView where
# Chromium's page would be (decision 9). The check's server at 127.0.0.1 stands in for them, added
# to the list with BROWSER_WEBKIT_SITES, and at localhost for a site off it. Opened from ⌘T, typed
# with ⌘L or brought back by a relaunch, its page shows in WebKit, its title and URL reach the
# sidebar, and agents may not act on it. A link from it to a site off the list loads in Chromium,
# ⌘← with no page of its own to go back to goes back to the Chromium page it left, ⌘W from inside
# it closes it, and a Space switch hides it. With BROWSER_CHECK_FAIRPLAY=1, on a Mac whose WebKit
# has FairPlay, EZDRM's test stream plays in it.

# The WebKit page: its title says it drew, and its top-left 40 points are a link off the list.
serve
chromium_url="http://localhost:$port/"
webkit_url="${site}webkit.html"
printf '%s' "<title>loading</title>
<a href=\"$chromium_url\" style=\"position:fixed;left:0;top:0;width:40px;height:40px\"></a>
<script>requestAnimationFrame(() => setTimeout(() => document.title = 'webkit ready'));</script>" \
  > "$out/site/webkit.html"
# The link's point in the window: the page starts past the sidebar's 228 points and 10 down.
link_point="248 30"

# Waits for tab $1 to show the WebKit page in WebKit. $2 says how it got there.
expect_webkit() {
  expect "^browser: webkit $1 $webkit_url\$" "$2: the page didn't open in WebKit"
  expect "^browser: title $1 webkit ready\$" "$2: the WebKit page's title didn't reach the sidebar"
  expect "^browser: page $1 back [01] forward 0 loading 0 $webkit_url\$" \
    "$2: the WebKit page's URL didn't reach the sidebar: $(grep "^browser: page $1" "$log" | tail -1)"
}
# How many WebKit pages the window shows, and that the one it shows is where the page goes.
webkit_pages() {
  send layout
  layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content')
  echo "$layout" | awk -F', ' '{ for (i = 2; i <= NF; i++) print $i }' |
    grep '^IloWebKitPage ' | grep -vc ' hidden$' || true
}
expect_webkit_shown() {
  [ "$(webkit_pages)" = 1 ] || fail "$1: the window doesn't show one WebKit page: $layout"
  content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*\)x\([0-9]*\),.*/\1 \2/')
  set -- "$1" $content
  echo "$layout" | grep -q "IloWebKitPage 228,10 $(($2 - 238))x$(($3 - 20))" ||
    fail "$1: the WebKit page isn't where the page goes: $layout"
}

log="$out/webkit.log"
step=0
rm -f "$saved"
BROWSER_WEBKIT_SITES=127.0.0.1 BROWSER_DRIVE=1 BROWSER_URL="$chromium_url" \
  "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
grep -q '^browser: webkit ' "$log" && fail "a page off the list opened in WebKit"
first=$(space)
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: WebKit tabs. Browser wasn't the active app, so the page ignored the click."
  return 0
fi

# Opened from ⌘T, the page shows in WebKit where Chromium's would.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type "$webkit_url"
send key return
expect "^browser: today [0-9]* $a\$" "the tab for the WebKit page didn't open at the top of Today"
w=$(today | cut -d' ' -f1)
expect_webkit "$w" "opened from ⌘T"
expect_webkit_shown "opened from ⌘T"
../tools/window-screenshot.sh "$pid" "$out/webkit.png" || fail "no window to capture"
# A click puts the keyboard in it, and a link off the list loads in Chromium, in the same tab.
send click $page_point
in_browser IloWebView "after a click in the WebKit page"
send click $link_point
expect "^browser: webkit $w leaves for $chromium_url\$" "the link off the list stayed in WebKit"
expect "^browser: loaded $w $chromium_url\$" "the link off the list didn't load in Chromium"
[ "$(webkit_pages)" = 0 ] || fail "the WebKit page stayed after its link left it: $layout"

# Typed with ⌘L, it goes back to WebKit, and ⌘← there goes back to the Chromium page it left.
go_from_bar "$webkit_url"
expect_webkit "$w" "typed with ⌘L"
send click $page_point
history_key left back page
expect "^browser: webkit $w back to Chromium\$" "⌘← in WebKit didn't go back to the Chromium page"
expect "^browser: page $w back . forward . loading 0 $chromium_url\$" \
  "⌘← in WebKit didn't show the Chromium page: $(grep "^browser: page $w" "$log" | tail -1)"
go_from_bar "$webkit_url"
expect_webkit "$w" "typed with ⌘L again"

# A Space switch hides it, and an agent may not act on the WebKit tab of its own Space.
send agentspace claude-code Claude Code
expect '^browser: made space [1-9][0-9]* for agent claude-code$' "the driver didn't make the agent's Space"
agent=$(grep '^browser: made space' "$log" | tail -1 | cut -d' ' -f4)
send key 2 ctrl
expect "^browser: space $agent\( in .*\)\{0,1\}\$" "⌃2 didn't show the agent's Space"
[ "$(webkit_pages)" = 0 ] || fail "the WebKit page stayed over another Space: $layout"
send key t cmd
expect '^browser: command bar shown$' "⌘T in the agent's Space didn't open the command bar"
send type "$webkit_url"
send key return
expect '^browser: today [0-9][0-9]*$' "the WebKit tab didn't open in the agent's Space"
g=$(today)
expect_webkit "$g" "opened in the agent's Space"
send agentmayact claude-code "$g"
expect "^browser: agent claude-code may act on $g: 0\$" "the agent may act on a WebKit tab"
send key 1 ctrl
expect "^browser: space $first\( in .*\)\{0,1\}\$" "⌃1 didn't show the user's Space again"
expect_webkit_shown "back in the user's Space"

# ⌘W with the keyboard in a WebKit page closes its tab.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type "$webkit_url"
send key return
expect "^browser: today [0-9]* $w $a\$" "a second WebKit tab didn't open at the top of Today"
x=$(today | cut -d' ' -f1)
expect_webkit "$x" "a second one"
send click $page_point
in_browser IloWebView "after a click in the second WebKit page"
send key w cmd
expect '^browser: command close-tab via page$' "⌘W in the WebKit page didn't reach the UI"
expect "^browser: tab $x closed\$" "⌘W in the WebKit page didn't close its tab"
send click $(row 0)
expect "^browser: selected $w\$" "the first WebKit tab wasn't shown"
quit

# Launched again, the tab comes back, and shown, its page opens in WebKit again.
log="$out/webkit-restored.log"
step=0
BROWSER_WEBKIT_SITES=127.0.0.1 BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" \
  > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await "^browser: webkit [0-9]* $webkit_url\$" || fail "the relaunch didn't bring the WebKit tab back in WebKit"
w=$(grep -m1 "^browser: webkit [0-9]* $webkit_url\$" "$log" | cut -d' ' -f3)
await "^browser: title $w webkit ready\$" || fail "the restored WebKit tab didn't draw"
expect_webkit_shown "after the relaunch"
fairplay=
if [ -n "${BROWSER_CHECK_FAIRPLAY:-}" ]; then
  # EZDRM's public FairPlay demo, through Apple's older WebKit key API, which its TS stream needs,
  # as the spike in decision 9 played it.
  sed 's/^    //' > "$out/site/fairplay.html" << 'PAGE'
    <title>fairplay loading</title>
    <video muted playsinline style="width:100%"></video>
    <script>
    const HLS = 'https://fps.ezdrm.com/demo/video/ezdrm.m3u8';
    const CERT = 'https://fps.ezdrm.com/demo/video/eleisure.cer';
    const LICENSE = 'https://fps.ezdrm.com/api/licenses/09cc0377-6dd4-40cb-b09d-b582236e70fe';
    const video = document.querySelector('video');
    const say = s => document.title = 'fairplay ' + s;
    const toUtf16 = s => Uint16Array.from(s, c => c.charCodeAt(0));
    function initData(skd, id, cert) {
      const buf = new ArrayBuffer(skd.byteLength + 4 + id.byteLength + 4 + cert.byteLength);
      const view = new DataView(buf);
      let at = 0;
      new Uint8Array(buf, at, skd.byteLength).set(new Uint8Array(skd.buffer, skd.byteOffset, skd.byteLength));
      at += skd.byteLength;
      view.setUint32(at, id.byteLength, true);
      at += 4;
      new Uint16Array(buf, at, id.length).set(id);
      at += id.byteLength;
      view.setUint32(at, cert.byteLength, true);
      at += 4;
      new Uint8Array(buf, at, cert.byteLength).set(cert);
      return new Uint8Array(buf);
    }
    (async () => {
      try {
        const cert = new Uint8Array(await (await fetch(CERT)).arrayBuffer());
        video.addEventListener('webkitneedkey', e => {
          const uri = String.fromCharCode(...new Uint16Array(e.initData.buffer));
          const id = toUtf16(uri.split('skd://')[1].split(/[/;]/)[0]);
          if (!video.webkitKeys) video.webkitSetMediaKeys(new WebKitMediaKeys('com.apple.fps.1_0'));
          const session = video.webkitKeys.createSession('video/mp4', initData(e.initData, id, cert));
          session.addEventListener('webkitkeymessage', async m => {
            const r = await fetch(LICENSE, {method: 'POST',
              headers: {'Content-Type': 'application/octet-stream'}, body: m.message});
            session.update(new Uint8Array(await r.arrayBuffer()));
          });
          session.addEventListener('webkitkeyerror', () => say('key error ' + session.error.systemCode));
        });
        video.addEventListener('timeupdate', () => video.currentTime > 2 && say('playing'));
        video.addEventListener('error', () => say('error ' + video.error.code));
        video.src = HLS;
        await video.play();
      } catch (e) {
        say('failed ' + e.name);
      }
    })();
    </script>
PAGE
  send click $page_point
  go_from_bar "${site}fairplay.html"
  expect "^browser: webkit $w ${site}fairplay.html\$" "the FairPlay page didn't open in WebKit"
  await "^browser: title $w fairplay \(playing\|error\|key error\|failed\)" ||
    fail "the FairPlay stream didn't start or fail in 30 s"
  grep -q "^browser: title $w fairplay playing\$" "$log" ||
    fail "the FairPlay stream didn't play: $(grep "^browser: title $w fairplay" "$log" | tail -1)"
  ../tools/window-screenshot.sh "$pid" "$out/fairplay.png" || fail "no window to capture"
  fairplay=", and EZDRM's FairPlay stream plays. Window: $out/fairplay.png"
fi
quit
echo "ok: a listed site opens in WebKit from ⌘T, ⌘L and a relaunch, with its title and URL in the sidebar, a link off the list goes to Chromium, ⌘← goes back to the Chromium page it left, ⌘W from inside closes it, a Space switch hides it, and agents may not act on it${fairplay:-. skip: FairPlay playback, which needs BROWSER_CHECK_FAIRPLAY=1 on real hardware}. Window: $out/webkit.png"
