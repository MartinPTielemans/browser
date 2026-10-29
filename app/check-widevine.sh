#!/bin/bash
# Linux only, and on its own, as check.sh's runs are the Mac's: Google's Widevine module, offered
# by the protected-media note, downloaded at its yes, and playing after the restart (#236). It
# opens Shaka Player's public Widevine demo stream, with its no-account license server, from a page
# served here, in a profile of its own; nothing signs in anywhere. It needs the network, as the
# module comes from Google, and build/Browser (./build.sh) with an engine built with Widevine.
# The first run clicks the note's Download from Google and waits for the module; the second, on the
# same profile as a restart would be, expects the page to play. Leaves captures and logs in
# build/check/widevine/.
# Usage: ./check-widevine.sh
set -euo pipefail
cd "$(dirname "$0")"
[ "$(uname -s)" = Linux ] || { echo "FAIL: Linux only." >&2; exit 1; }
[ -x build/Browser/chrome ] || { echo "FAIL: no build/Browser. Run ./build.sh." >&2; exit 1; }

out=$PWD/build/check/widevine
work=$(mktemp -d)
server=
cleanup() {
  [ -z "$server" ] || kill "$server" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
rm -rf "$out"
mkdir -p "$out" "$work/site" "$work/profile"

# The page says what happened in its title, which the app logs. Playing, it pauses after a second,
# as headless.sh captures once the picture stands still.
cat >"$work/site/widevine.html" <<'EOF'
<!doctype html>
<title>widevine loading</title>
<video id="video" muted width="640"></video>
<script src="https://cdn.jsdelivr.net/npm/shaka-player@4.11.7/dist/shaka-player.compiled.js"></script>
<script>
  const video = document.getElementById("video");
  const said = (title) => { document.title = title; };
  video.addEventListener("timeupdate", () => {
    if (video.currentTime > 1) { video.pause(); said("widevine playing"); }
  });
  shaka.polyfill.installAll();
  const player = new shaka.Player();
  player.attach(video)
    .then(() => {
      player.configure({ drm: { servers: { "com.widevine.alpha": "https://cwip-shaka-proxy.appspot.com/no_auth" } } });
      return player.load("https://storage.googleapis.com/shaka-demo-assets/angel-one-widevine/dash.mpd");
    })
    .then(() => video.play())
    .catch((error) => said("widevine error " + (error.code || error)));
</script>
EOF
# 127.0.0.1 is a secure context, which EME needs, as https would be.
python3 -I -c '
import functools, http.server, sys
handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=sys.argv[1])
server = http.server.HTTPServer(("127.0.0.1", 0), handler)
print(server.server_address[1], flush=True)
server.serve_forever()
' "$work/site" >"$work/port" 2>/dev/null &
server=$!
for _ in $(seq 50); do
  [ -s "$work/port" ] && break
  sleep 0.1
done
[ -s "$work/port" ] || { echo "FAIL: the page's server didn't start." >&2; exit 1; }
url="http://127.0.0.1:$(cat "$work/port")/widevine.html"

export BROWSER_PROFILE_DIR=$work/profile BROWSER_URL=$url

# The note offers the module: a click on its button, where clear.sh clicks the Mac's, is the yes.
# The window fills the session's one output, so its points are the output's.
yes='
set -euo pipefail
fail() { echo "FAIL: $*" >&2; exit 1; }
tab=$(grep -oE "^browser: drm note [0-9]+ at " "$LOG" | tail -1 | cut -d" " -f4)
grep -q "^browser: page $tab needs com.widevine.alpha\$" "$LOG" || fail "the page asked for no Widevine"
read -r nx ny nw nh <<<"$(grep "^browser: drm note $tab at " "$LOG" | tail -1 | cut -d" " -f6-)"
swaymsg -q seat seat0 cursor set $((nx + nw / 2)) $((ny + nh - 26))
swaymsg -q seat seat0 cursor press button1
swaymsg -q seat seat0 cursor release button1
for _ in $(seq 100); do
  grep -q "^browser: widevine download\$" "$LOG" && break
  sleep 0.1
done
grep -q "^browser: widevine download\$" "$LOG" || fail "the note'"'"'s button downloaded nothing"
# Google'"'"'s module is about 10 MB.
for _ in $(seq 1200); do
  grep -qE "^browser: widevine state (restart|failed)\$" "$LOG" && break
  sleep 0.1
done
grep -q "^browser: widevine state restart\$" "$LOG" || fail "the module didn'"'"'t download"
[ -n "$(ls "$BROWSER_PROFILE_DIR/WidevineCdm" 2>/dev/null)" ] || fail "no module in the profile"
'
../tools/headless.sh --x11 --then "$yes" "$out/note.png" '^browser: drm note [0-9]+ at ' \
  build/Browser/chrome
echo "ok: the note downloaded Google's Widevine module at its yes"

# Launched again, as the note's Restart would, Chromium loads the module and the page plays.
played='
set -euo pipefail
fail() { echo "FAIL: $*" >&2; exit 1; }
grep -q "^browser: widevine state ready\$" "$LOG" || fail "the core didn'"'"'t say the module loaded"
if grep -qE "^browser: drm note [0-9]+ at " "$LOG"; then
  fail "the note showed with the module loaded"
fi
'
../tools/headless.sh --x11 --page --then "$played" "$out/playing.png" \
  '^browser: title [0-9]+ widevine (playing|error .*)$' build/Browser/chrome
grep -qE '^browser: title [0-9]+ widevine playing$' "$out/playing.log" \
  || { grep -E '^browser: title [0-9]+ widevine' "$out/playing.log" >&2; echo "FAIL: the page didn't play" >&2; exit 1; }
echo "ok: the page played with Google's Widevine module"
