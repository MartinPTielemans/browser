# A page loads in the browser window, over the GPUI sidebar drawn in it, and quitting through
# AppKit closes the page and ends every Chromium process. The first run shows the welcome and no
# page, so the page is given.

log="$out/load.log"
rm -f "$saved" "$launch_frame"
BROWSER_URL='https://en.wikipedia.org/wiki/Arc_(web_browser)' "$browser" \
  < /dev/null > "$log" 2>&1 &
pid=$!
await_load
../tools/window-screenshot.sh "$pid" "$out/load.png" || fail "no window to capture"
quit
echo "ok: $(grep '^browser: loaded' "$log" | cut -d' ' -f4-) loaded, quit cleanly. Window: $out/load.png"
