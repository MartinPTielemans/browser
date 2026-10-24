#!/bin/sh
# Runs the spike four times and checks what it has proved so far:
#   1. The start page loads in the browser window, over the GPUI sidebar drawn in it, and
#      quitting through AppKit closes the page and ends every Chromium process.
#   2. Clicks move keyboard focus between the page and the UI, and typing goes where the focus
#      is. With the page focused, the browser's reserved shortcuts reach the UI and never the
#      page, and the others reach the page first, which can keep them. ⌘Q quits. The command
#      bar opens over the page with the keyboard, never showing the last query, gives the
#      keyboard back when it closes, and loads what's typed in it. Japanese input composes and
#      commits in the page and in the command bar, where return and escape go to the input
#      method while it composes.
#   3. The page view keeps its margins and GPUI's view fills the window through a resize and
#      full screen, a page that goes full screen takes the window with it and escape brings
#      both back, the command bar opens in full screen and keeps the keyboard when a page
#      commits under it, then gives the keyboard to the new page, and scrolling a long page
#      doesn't stutter.
#   4. A page's popup opens as a tab in the window.
# Runs 2 to 4 need Spike to be the active app, and run 3 takes it full screen for a moment.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
set -e
cd "$(dirname "$0")"
app="$PWD/build/Spike.app"
out=build/check
# Another Spike holds the profile directory, so this one couldn't start.
if pgrep -f "$app/Contents" > /dev/null; then
  echo "FAIL: Spike is already running. Quit it first." >&2
  exit 1
fi
rm -rf "$out"
mkdir -p "$out"
# After a failure, nothing the check started keeps running.
cleanup() {
  kill "$pid" 2>/dev/null || return 0
  for _ in $(seq 10); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -9 "$pid" 2>/dev/null
  pkill -9 -f "$app/Contents" || true
}
trap cleanup EXIT
fail() {
  echo "FAIL: $1" >&2
  grep -v '^$' "$log" | tail -5 >&2
  exit 1
}

# Waits up to 30 s for a line matching $1 in the log of the current run.
await() {
  for _ in $(seq 300); do
    grep -q "$1" "$log" && return 0
    kill -0 "$pid" 2>/dev/null || fail "the spike exited"
    sleep 0.1
  done
  return 1
}

# The spike logs "spike: loaded" or "spike: load error" for the main frame.
await_load() {
  await '^spike: load' || fail "the page didn't finish loading"
  grep -q '^spike: loaded' "$log" || fail "the page didn't load"
}

# Waits for the spike to exit and checks that no Chromium process outlives it.
exited() {
  for _ in $(seq 20); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -0 "$pid" 2>/dev/null && fail "the spike didn't quit"
  wait "$pid" || fail "the spike exited with status $?"
  for _ in $(seq 10); do
    pgrep -f "$app/Contents" > /dev/null || break
    sleep 0.5
  done
  pgrep -f "$app/Contents" > /dev/null && fail "Chromium helper processes were left running"
  return 0
}

# A quit Apple Event goes through -[NSApplication terminate:], the way macOS quits apps when the
# user logs out.
quit() {
  osascript -e 'tell application id "dev.browser.spike-core" to quit'
  exited
}

log="$out/load.log"
"$app/Contents/MacOS/Spike" < /dev/null > "$log" 2>&1 &
pid=$!
await_load
../../tools/window-screenshot.sh "$pid" "$out/load.png" || fail "no window to capture"
quit
echo "ok: $(grep '^spike: loaded' "$log" | cut -d' ' -f3-) loaded, quit cleanly. Window: $out/load.png"

# The test page puts what reached it in its title, which the spike logs. Chromium drops input
# to a page that hasn't drawn yet, so the page says when it has.
page='data:text/html,<title>loading</title>
<textarea id=text autofocus style="font:24px system-ui;width:95%25;height:90vh"></textarea>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
text.oninput = () => document.title = "value " + text.value;
// Like Google Docs, the page keeps ⌘F.
addEventListener("keydown", e => {
  if (!e.metaKey) return;
  document.title = "page saw cmd-" + e.key;
  if (e.key == "f") e.preventDefault();
}, true);
</script>'
log="$out/keys.log"
mkfifo "$out/drive"
SPIKE_DRIVE=1 SPIKE_URL="$page" "$app/Contents/MacOS/Spike" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^spike: title ready$' || fail "the test page didn't draw"

# Sends a command to the spike's driver (host/drive.mm) and waits until AppKit has handled it.
step=0
send() {
  step=$((step + 1))
  echo "$*" >&3
  await "^spike: drive $step done" || fail "the driver didn't finish \"$*\""
  grep -q "^spike: drive $step: can't do that" "$log" && fail "the driver can't do \"$*\""
  done=$(grep "^spike: drive $step done" "$log")
  key=$(echo "$done" | sed 's/.*key \([^,]*\),.*/\1/')
  focus=$(echo "$done" | sed 's/.*focus \([^,]*\),.*/\1/')
  active=$(echo "$done" | sed 's/.*active \([01]\).*/\1/')
  marked=$(echo "$done" | sed 's/.*marked //')
}
# Waits for a line matching $1 logged after the last command was sent.
expect() {
  for _ in $(seq 100); do
    sed -n "/^spike: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    sleep 0.1
  done
  fail "$2"
}

# Points from the window's top-left corner. The sidebar is 240 wide.
page_point="600 400"
sidebar_point="100 400"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: keyboard, focus, window states and scrolling. Spike wasn't the active app, so the"
  echo "  page ignored the click. macOS decides which app is active; a background process can't."
  exit 0
fi
[ "$focus" = RenderWidgetHostViewCocoa ] || fail "a click on the page left focus on $focus"
send type abc
expect '^spike: title value abc$' "typing didn't reach the focused page"

# Chrome's browser window has the keyboard, with focus on $1.
in_browser() {
  [ "$key" = BrowserCoreNSWindow ] && [ "$focus" = "$1" ] ||
    fail "$2: the keyboard is in $key, on $focus"
}
# ⌘$1 with the page focused reaches the UI as command $2.
page_shortcut() {
  send key "$1" cmd
  expect "^spike: command $2 via page$" "⌘$(echo "$1" | tr a-z A-Z) with the page focused didn't reach the UI"
}
page_shortcut w close-tab
in_browser RenderWidgetHostViewCocoa "after ⌘W"
for shortcut in "t new-tab" "l edit-url"; do
  page_shortcut $shortcut
  expect '^spike: command bar shown$' "$shortcut didn't open the command bar"
  # A shortcut the page doesn't keep comes back after the page has had it, so after the
  # driver's done.
  send state
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "the command bar opened without the keyboard, which is in $key, on $focus"
  send key escape
  expect '^spike: command bar hidden$' "escape didn't close the command bar"
  in_browser RenderWidgetHostViewCocoa "after the command bar closed"
done
send key f cmd
expect '^spike: title page saw cmd-f$' "⌘F with the page focused didn't reach the page"

send click $sidebar_point
# GPUI's view in a window it doesn't own, which AccessKit subclasses.
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
send type x
send key f cmd
expect '^spike: command find via ui$' "⌘F with the UI focused didn't reach the UI"
send key t cmd
expect '^spike: command new-tab via ui$' "⌘T with the UI focused didn't reach the UI"
expect '^spike: command bar shown$' "⌘T with the UI focused didn't open the command bar"
send click $page_point
expect '^spike: command bar hidden$' "a click on the page didn't close the command bar"
in_browser RenderWidgetHostViewCocoa "after a click on the page closed the command bar"
send type d
# Input reaches the page in order, so by now it has seen the x and the shortcuts, if they got there.
expect '^spike: title value abcd$' "the page got keys meant for the UI: $(grep '^spike: title' "$log" | tail -1)"
grep -q '^spike: title page saw cmd-[tw]$' "$log" && fail "the page saw a reserved shortcut"
grep -q '^spike: title page saw cmd-l$' "$log" || fail "⌘L with the page focused didn't reach the page first"
grep -q '^spike: command find via page$' "$log" && fail "the page kept ⌘F, but the UI ran it too"

# The driver's scripted input method stands in for Japanese input, which is system-wide.
send ime on
send type nihongo
expect '^spike: title value abcdnihongo$' "the page didn't show the composition"
send key space
expect '^spike: title value abcd日本語$' "the page didn't show the conversion"
send key return
send ime off
send type x
expect '^spike: title value abcd日本語x$' "return in the page did more than commit: $(grep '^spike: title' "$log" | tail -1)"

# Nothing logged since the last command was sent matches $1.
none_since() {
  ! sed -n "/^spike: drive $step: /,\$p" "$log" | grep -q "$1"
}
send key l cmd
expect '^spike: command bar shown$' "⌘L didn't open the command bar"
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the command bar didn't take the composition: marked $marked"
send key escape
none_since '^spike: command bar hidden$' || fail "escape while composing closed the command bar"
[ "$marked" = none ] || fail "escape didn't cancel the composition: marked $marked"
send type nihongo
send key space
[ "$marked" = 0,3 ] || fail "the command bar didn't take the conversion: marked $marked"
../../tools/window-screenshot.sh "$pid" "$out/ime.png" || fail "no window to capture"
send key return
none_since '^spike: command bar go' || fail "return while composing went somewhere"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send type .invalid
send key return
expect '^spike: command bar go https://日本語.invalid$' "the command bar didn't keep the committed text: $(grep '^spike: command bar go' "$log" | tail -1)"

send key l cmd
expect '^spike: command bar shown$' "⌘L didn't open the command bar"
send type example.com
../../tools/window-screenshot.sh "$pid" "$out/command-bar.png" || fail "no window to capture"
send key return
expect '^spike: command bar go https://example.com$' "return in the command bar didn't go to what was typed"
expect '^spike: command bar hidden$' "going somewhere didn't close the command bar"
expect '^spike: loaded https://example.com/$' "the page didn't load what was typed in the command bar"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar loaded a page"
# The bar draws itself empty before it goes on screen, so it never shows the last query.
awk '/^spike: command bar drew/ { drew = $0 }
  /^spike: command bar shown$/ && drew != "spike: command bar drew \"\"" { exit 1 }' "$log" ||
  fail "the command bar went on screen still showing what it drew last"
# ⌘Q is the driver's last command: the spike doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^spike: command quit via page$' || fail "⌘Q with the page focused didn't reach the UI"
exited
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q quits, the command bar takes the keyboard and gives it back, Japanese input composes in both. Windows: $out/command-bar.png, $out/ime.png"

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
  requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
</script>
HTML
)
log="$out/window.log"
step=0
SPIKE_DRIVE=1 SPIKE_URL="data:text/html;base64,$page" "$app/Contents/MacOS/Spike" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^spike: title ready$' || fail "the scrolling page didn't draw"

# The page view keeps the sidebar's 240 points on its left and 8 on its other sides, over GPUI's
# view, which fills the window. A full-screen page ("fills") covers the whole window.
check_layout() {
  send layout
  layout=$(sed -n "/^spike: drive $step: /,\$p" "$log" | grep -m1 '^spike: layout')
  content=$(echo "$layout" | sed 's/^spike: layout content \([0-9]*x[0-9]*\),.*/\1/')
  view=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
  ui=$(echo "$layout" | sed -n 's/.*GPUIView \([0-9,]* [0-9x]*\).*/\1/p')
  width=${content%x*}
  height=${content#*x}
  expected="240,8 $((width - 248))x$((height - 16))"
  [ "${2:-}" = fills ] && expected="0,0 $content"
  [ "$view" = "$expected" ] || fail "$1: the page view is at $view in a $content window"
  [ "$ui" = "0,0 $content" ] || fail "$1: GPUI's view is at $ui in a $content window"
}
check_layout "at launch"

# 300 trackpad events of 10 points, 8 ms apart.
send scroll $page_point 10 300
expect '^spike: title frames' "scrolling didn't move the page"
timing=$(grep -m1 '^spike: title frames' "$log" | sed 's/^spike: title //')
echo "$timing" | grep -q ' scrolled 3000$' || fail "the page didn't scroll by exactly 3000 points: $timing"
frames=$(echo "$timing" | sed 's/^frames \([0-9]*\) .*/\1/')
slow=$(echo "$timing" | sed 's/.* slow \([0-9]*\) .*/\1/')
[ $((slow * 100)) -le "$frames" ] || fail "more than 1% of frames took longer than 1/60 s: $timing"

send resize 1000 700
check_layout "after a resize"
send fullscreen
expect '^spike: window entered full screen$' "the window didn't go full screen"
check_layout "in full screen"
send key l cmd
expect '^spike: command bar shown$' "⌘L in full screen didn't open the command bar"
send state
[ "$key" = GPUIPanel ] || fail "the command bar in full screen didn't get the keyboard, $key did"
../../tools/window-screenshot.sh "$pid" "$out/fullscreen.png" || fail "no window to capture"
send key escape
expect '^spike: command bar hidden$' "escape didn't close the command bar in full screen"
send fullscreen
expect '^spike: window left full screen$' "the window didn't leave full screen"
check_layout "after full screen"

# A page that goes full screen takes the window with it, and escape brings both back.
send key f
expect '^spike: page entered full screen$' "the page didn't go full screen"
expect '^spike: window entered full screen$' "the window didn't go full screen with the page"
check_layout "with the page full screen" fills
../../tools/window-screenshot.sh "$pid" "$out/page-fullscreen.png" || fail "no window to capture"
send key escape
expect '^spike: page left full screen$' "escape didn't take the page out of full screen"
expect '^spike: window left full screen$' "the window didn't leave full screen with the page"
check_layout "after the page's full screen"

# Chromium makes its window key when a new page's view replaces the focused one.
send key n
send key l cmd
expect '^spike: command bar shown$' "⌘L didn't open the command bar"
expect '^spike: loaded https://example.com/$' "the page didn't leave when the command bar opened"
send state
[ "$key" = GPUIPanel ] || fail "a page that committed under the command bar took the keyboard, $key has it"
send key escape
expect '^spike: command bar hidden$' "escape didn't close the command bar after a page committed under it"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar closed over a page that committed under it"
exec 3>&-
quit
echo "ok: the page view keeps its margins through a resize and full screen, a full-screen page takes the window with it and escape brings both back, the command bar opens in full screen and keeps the keyboard when a page commits under it, then gives it to the new page. Scrolling: $timing. Windows: $out/fullscreen.png, $out/page-fullscreen.png"

# A page's popup opens as a tab in the window, as links to new tabs do, instead of a window of
# Chrome's own.
page='data:text/html,<title>loading</title>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
// A popup needs a user gesture, which a key press is.
addEventListener("keydown", e => e.key == "p" && open("https://example.com/", "", "popup,width=400,height=300"));
</script>'
log="$out/popup.log"
step=0
SPIKE_DRIVE=1 SPIKE_URL="$page" "$app/Contents/MacOS/Spike" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^spike: title ready$' || fail "the popup test page didn't draw"
send click $page_point
send key p
expect '^spike: tab opened in the foreground$' "the page's popup didn't open as a tab"
expect '^spike: loaded https://example.com/$' "the popup's page didn't load in its tab"
check_layout "with the popup's tab"
../../tools/window-screenshot.sh "$pid" "$out/popup.png" || fail "no window to capture"
exec 3>&-
quit
echo "ok: a page's popup opens as a tab in the window. Window: $out/popup.png"
