# The page view keeps its margins and GPUI's view fills the window through a resize and full screen,
# and the window's buttons stay in the header row. A page that goes full screen takes the window
# with it and escape brings both back, the command bar opens in full screen and keeps the keyboard
# when a page commits under it, then gives the keyboard to the new page, and scrolling a long page
# doesn't stutter. Dragging the window's background moves the window, and the buttons, URL field and
# rows keep their presses. A double-click there zooms it, and one in the sidebar below the rows
# opens a new tab. ⌘S, with the page or the UI focused, and the sidebar button hide the sidebar and
# the window's buttons, and the card takes its place, through a resize; ⌘S brings them back. The
# pointer kept at the window's edge brings the hidden sidebar out, and one passing it doesn't. The
# sidebar's edge drags its width between 160 and 400, rubber-banding past them, and a double-click
# on it puts the width back. A page that hasn't drawn yet shows the card's colour, not white.

# A long page that times its frames while it scrolls and puts the numbers in its title.
page=$(base64 << 'HTML' | tr -d '\n'
<!doctype html>
<title>loading</title>
<style>
  body { font: 16px/1.5 system-ui; margin: 0 auto; max-width: 720px; padding: 0 16px }
  .card { margin: 16px 0; padding: 16px; border-radius: 12px; box-shadow: 0 2px 8px rgb(0 0 0 / 15%) }
</style>
<script>
  const text = "The quick brown fox jumps over the lazy dog. ".repeat(12);
  for (let i = 0; i < 300; i++) {
    document.write(`<div class=card style="background: hsl(${(i * 37) % 360} 70% 92%)"><h2>Section ${i}</h2><p>${text}</p></div>`);
  }
  // requestAnimationFrame runs once per frame the page draws, so the gaps between its calls are
  // frame times. Frames count while scroll events arrive. A frame slower than 60 Hz shows as a
  // stutter on any display.
  const intervals = [];
  let last = 0, lastScroll = 0, measuring = false;
  addEventListener("scroll", () => {
    lastScroll = performance.now();
    if (!measuring) {
      measuring = true;
      requestAnimationFrame(frame);
    }
  }, { passive: true });
  function frame(time) {
    if (last && time - lastScroll < 20) intervals.push(time - last);
    last = time;
    if (performance.now() - lastScroll < 300) return requestAnimationFrame(frame);
    const sorted = [...intervals].sort((a, b) => a - b);
    const at = q => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))].toFixed(1);
    const fps = Math.round(1000 * intervals.length / intervals.reduce((a, b) => a + b, 0));
    const slow = intervals.filter(i => i > 1000 / 60).length;
    document.title = `frames ${intervals.length} fps ${fps} median ${at(0.5)} p99 ${at(0.99)} max ${at(1)} slow ${slow} scrolled ${Math.round(scrollY)}`;
  }
  // An f takes the page full screen, as the full-screen button of a video does.
  addEventListener("keydown", e => e.key == "f" && !e.metaKey && document.documentElement.requestFullscreen());
  // After an n, the page goes to another site when its window loses the keyboard, so a new page
  // commits while the command bar is open.
  let leave = false;
  addEventListener("keydown", e => leave ||= e.key == "n");
  addEventListener("blur", () => leave && (location = "https://example.com/"));
  // A w opens the page the fragment names in a tab.
  addEventListener("keydown", e => e.key == "w" && window.open(location.hash.slice(1)));
  requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
</script>
HTML
)
log="$out/window.log"
step=0
rm -f "$saved"
serve
url="data:text/html;base64,$page#${site}wait"
BROWSER_DRIVE=1 BROWSER_URL="$url" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the scrolling page didn't draw"

# The window's buttons sit in the header row, where Arc's do. In full screen they come down with
# the menu bar instead. They go with the sidebar ("hidden").
check_buttons() {
  buttons=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout buttons' |
    cut -d' ' -f4-)
  expected="20.0,24.0 43.0,24.0 66.0,24.0"
  [ "${2:-}" = hidden ] && expected="hidden hidden hidden"
  [ "$buttons" = "$expected" ] || fail "$1: the window's buttons are at $buttons, not $expected"
}
check_layout "at launch"
check_buttons "at launch"

# A page shows the card's colour until its first frame, never white, and its tab is loading from the
# start. The page's w opens one whose server never answers, so it never commits, and its URL, which
# a page asked for, stays hidden until it does.
first=$(today)
send click $page_point
send key w
expect "^browser: tab [0-9]* opened by $first in the foreground\$" "the page didn't open a tab"
tab=$(grep '^browser: tab [0-9]* opened by' "$log" | tail -1 | cut -d' ' -f3)
expect "^browser: page $tab back 0 forward 0 loading 1 \$" \
  "a tab opened to a page that hasn't come doesn't say it's loading"
for _ in $(seq 20); do
  ../tools/window-screenshot.sh "$pid" "$out/before-paint.png" || fail "no window to capture"
  color=$(pixel "$out/before-paint.png" 0.6 0.5)
  dark "$color" && break
done
dark "$color" || fail "a page that hasn't drawn shows $color, not the card's colour"
send key w cmd
expect_tabs "$first" "$first" "after closing the tab of the page that hasn't come"
send click $page_point

# 300 trackpad events of 10 points, 8 ms apart.
send scroll $page_point 10 300
expect '^browser: title [0-9]* frames' "scrolling didn't move the page"
timing=$(grep -m1 '^browser: title [0-9]* frames' "$log" | sed 's/^browser: title [0-9]* //')
echo "$timing" | grep -q ' scrolled 3000$' || fail "the page didn't scroll by exactly 3000 points: $timing"
frames=$(echo "$timing" | sed 's/^frames \([0-9]*\) .*/\1/')
slow=$(echo "$timing" | sed 's/.* slow \([0-9]*\) .*/\1/')
[ $((slow * 100)) -le "$frames" ] || fail "more than 1% of frames took longer than 1/60 s: $timing"

send resize 1000 700
check_layout "after a resize"
check_buttons "after a resize"
send fullscreen
expect '^browser: window entered full screen$' "the window didn't go full screen"
check_layout "in full screen"
send key l cmd
expect '^browser: command bar shown$' "⌘L in full screen didn't open the command bar"
send state
[ "$key" = GPUIPanel ] || fail "the command bar in full screen didn't get the keyboard, $key did"
../tools/window-screenshot.sh "$pid" "$out/fullscreen.png" || fail "no window to capture"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar in full screen"
send fullscreen
expect '^browser: window left full screen$' "the window didn't leave full screen"
check_layout "after full screen"
check_buttons "after full screen"

# A page that goes full screen takes the window with it, and escape brings both back.
send key f
expect '^browser: page entered full screen$' "the page didn't go full screen"
expect '^browser: window entered full screen$' "the window didn't go full screen with the page"
check_layout "with the page full screen" fills
../tools/window-screenshot.sh "$pid" "$out/page-fullscreen.png" || fail "no window to capture"
send key escape
expect '^browser: page left full screen$' "escape didn't take the page out of full screen"
expect '^browser: window left full screen$' "the window didn't leave full screen with the page"
check_layout "after the page's full screen"
check_buttons "after the page's full screen"

# Chromium makes its window key when a new page's view replaces the focused one.
send key n
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
expect '^browser: loaded [0-9]* https://example.com/$' "the page didn't leave when the command bar opened"
send state
[ "$key" = GPUIPanel ] || fail "a page that committed under the command bar took the keyboard, $key has it"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar after a page committed under it"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar closed over a page that committed under it"

# The window's background moves the window, as a title bar does: the header row outside its
# buttons, the sidebar below the rows and the card's margins. The buttons, the URL field and the
# rows and the Space's name keep their presses. Each drag moves 5 points, about a pointer's first step. Posted events
# don't move the window itself: the window server follows the real pointer.
moves=0
drag() {
  send drag "$1" "$2" $(($1 + 5)) "$2"
  count=$(grep -c '^browser: window move$' "$log" || true)
  if [ "$3" = moves ]; then
    [ "$count" -gt "$moves" ] || fail "a drag on $4 didn't move the window"
  else
    [ "$count" = "$moves" ] || fail "a drag on $4 moved the window"
  fi
  moves=$count
}
drag 115 24 moves "the header"
drag 138 24 stays "the back button"
drag 110 64 stays "the URL field"
drag 110 112 stays "the Space's name"
drag 110 195 stays "a tab's row"
drag 110 400 moves "the empty sidebar"
drag 600 5 moves "the card's top margin"
drag $((width - 8)) 300 moves "the card's right margin"
# A double-click there does what System Settings › Desktop & Dock says a title bar's does: zoom,
# unless it's set to something else.
action=$(defaults read -g AppleActionOnDoubleClick 2>/dev/null || echo Maximize)
if [ "$action" = Maximize ]; then
  send doubleclick 115 24
  expect '^browser: window zoomed 1$' "a double-click on the header didn't zoom the window"
  send doubleclick 115 24
  expect '^browser: window zoomed 0$' "a second double-click on the header didn't zoom it back"
  zoom="zooms it"
else
  zoom="isn't checked, since a title bar's double-click is set to $action here"
fi
send doubleclick 110 400
expect '^browser: command new-tab via sidebar$' "a double-click on the empty sidebar didn't open a new tab"
expect '^browser: command bar shown$' "a double-click on the empty sidebar didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

# ⌘S hides the sidebar, with the page focused or the UI, and the card takes its place, 10 in from
# every side, as the window's buttons go. ⌘S and the sidebar button bring both back.
# toggle <from> <hidden|shown> <what>: waits for the sidebar to finish moving after the command.
toggle() {
  expect "^browser: command toggle-sidebar via $1\$" "$3 didn't reach the UI"
  expect "^browser: sidebar $2\$" "$3 didn't leave the sidebar $2"
  if [ "$2" = hidden ]; then
    check_layout "after $3" inset
    check_buttons "after $3" hidden
  else
    check_layout "after $3"
    check_buttons "after $3"
  fi
}
send click $page_point
send key s cmd
toggle page hidden "⌘S with the page focused"
../tools/window-screenshot.sh "$pid" "$out/sidebar-hidden.png" || fail "no window to capture"
send key s cmd
toggle page shown "⌘S again with the page focused"
send click 110 400
send key s cmd
toggle ui hidden "⌘S with the UI focused"
send key s cmd
toggle ui shown "⌘S again with the UI focused"
send click 93 24
toggle sidebar hidden "a click on the sidebar button"
send key s cmd
toggle ui shown "⌘S after the sidebar button"
# The page stays inset through a resize while the sidebar is hidden.
send resize 900 600
send key s cmd
toggle ui hidden "⌘S in a smaller window"
send resize 1000 700
check_layout "after a resize with the sidebar hidden" inset
send key s cmd
toggle ui shown "⌘S after a resize"

# With the sidebar hidden, the pointer at the window's left edge brings it out over the page in a
# window of its own, which takes clicks and leaves the keyboard to the browser window, and the
# pointer well past it sends it away. A click on its sidebar button, or ⌘S, docks the sidebar.
# check_reveal <what> [shown]: the reveal's window covers the left edge, or there's none.
check_reveal() {
  send layout
  child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
    cut -d' ' -f4-)
  height=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    sed 's/^browser: layout content [0-9]*x\([0-9]*\),.*/\1/')
  expected=
  [ "${2:-}" = shown ] && expected="0,0 233x$height"
  [ "$child" = "$expected" ] ||
    fail "$1: the reveal's window is at ${child:-none}, not ${expected:-none}"
}
send key s cmd
toggle ui hidden "⌘S before the reveal"
send move 400 300
# The pointer has to stay at the edge a moment: one passing it doesn't bring the sidebar out.
step=$((step + 1))
passed=$step
echo "move 3 300" >&3
send move 400 300
send move 3 300
expect '^browser: reveal shown$' "the pointer at the window's edge didn't bring the sidebar out"
[ "$(sed -n "/^browser: drive $passed: /,\$p" "$log" | grep -c '^browser: reveal shown$')" = 1 ] ||
  fail "the pointer passing the window's edge brought the sidebar out"
check_reveal "with the sidebar out" shown
check_layout "with the sidebar out" inset
check_buttons "with the sidebar out" hidden
../tools/window-screenshot.sh "$pid" "$out/sidebar-reveal.png" || fail "no window to capture"
send move 200 300
send move 250 300
check_reveal "with the pointer over the sidebar out" shown
send move 400 300
expect '^browser: reveal hidden$' "the pointer past the sidebar out didn't send it away"
check_reveal "after the sidebar went away"
send move 3 300
expect '^browser: reveal shown$' "the pointer at the edge didn't bring the sidebar back out"
send click 93 24
toggle sidebar shown "a click on the sidebar button of the sidebar out"
check_reveal "with the sidebar docked from its sidebar button"
send key s cmd
toggle ui hidden "⌘S after docking the sidebar out"
send move 3 300
expect '^browser: reveal shown$' "the pointer at the edge didn't bring the sidebar out again"
send key s cmd
toggle ui shown "⌘S with the sidebar out"
check_reveal "with the sidebar docked by ⌘S"
# The sidebar's edge, the 6 pt right of its rows, drags its width between 160 and 400. Past them it
# rubber-bands, and let go it settles at the one it passed. A double-click on it puts back 228.
send drag 225 400 325 400
expect '^browser: sidebar width 328$' "dragging the sidebar's edge didn't widen the sidebar"
check_layout "with the sidebar dragged wider" 328
send press 325 400
send dragto 700 400
for _ in $(seq 20); do
  send layout
  band=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
    sed -n 's/.*WebContentsViewCocoa \([0-9]*\),.*/\1/p')
  [ "${band:-0}" -gt 400 ] && break
done
[ "${band:-0}" -gt 400 ] && [ "$band" -lt 460 ] ||
  fail "dragged far past its widest, the sidebar is ${band:-not} wide, not a little past 400"
../tools/window-screenshot.sh "$pid" "$out/sidebar-band.png" || fail "no window to capture"
send release 700 400
expect '^browser: sidebar width 400$' "let go past its widest, the sidebar didn't settle at 400"
check_layout "with the sidebar at its widest" 400
send drag 397 400 20 400
expect '^browser: sidebar width 160$' "let go past its narrowest, the sidebar didn't settle at 160"
check_layout "with the sidebar at its narrowest" 160
../tools/window-screenshot.sh "$pid" "$out/sidebar-narrow.png" || fail "no window to capture"
send doubleclick 157 400
expect '^browser: sidebar width 228$' "a double-click on the sidebar's edge didn't put its width back"
none_since '^browser: command new-tab' || fail "a double-click on the sidebar's edge opened a tab"
check_layout "after a double-click on the sidebar's edge"
exec 3>&-
quit
echo "ok: the page view keeps its margins and the window's buttons stay at Arc's through a resize and full screen, a full-screen page takes the window with it and escape brings both back, the command bar opens in full screen and keeps the keyboard when a page commits under it, then gives it to the new page. Drags on the window's background move it and drags on its controls don't. A double-click on the header $zoom, and one on the empty sidebar opens a new tab. ⌘S from the page and the UI, and the sidebar button, hide the sidebar and the window's buttons, the card takes its place through a resize, and ⌘S brings them back. The pointer kept at the window's edge brings the hidden sidebar out over the page, one passing it doesn't, and the pointer past it sends it away, and the sidebar button and ⌘S dock it from there. The sidebar's edge drags its width, rubber-bands past 160 and 400 and settles there, and a double-click puts it back. A page that hasn't drawn shows the card's colour ($color). Scrolling: $timing. Windows: $out/fullscreen.png, $out/page-fullscreen.png, $out/sidebar-hidden.png, $out/sidebar-reveal.png, $out/sidebar-band.png, $out/sidebar-narrow.png, $out/before-paint.png"
