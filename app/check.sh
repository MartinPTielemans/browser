#!/bin/sh
# Runs the app ten times and checks:
#   1. The start page loads in the browser window, over the GPUI sidebar drawn in it, and
#      quitting through AppKit closes the page and ends every Chromium process.
#   2. Clicks move keyboard focus between the page and the UI, and typing goes where the focus
#      is. With the page focused, the browser's reserved shortcuts reach the UI and never the
#      page, and the others reach the page first, which can keep them. ⌘Q quits. The command
#      bar opens over the page with the keyboard, never showing the last query, gives the
#      keyboard back when it closes, and loads what's typed in it. Japanese input composes and
#      commits in the page and in the command bar, where return and escape go to the input
#      method while it composes.
#   3. A tab opened from the command bar goes at the top of the Space's Today tabs and shows.
#      ⌘1–⌘9, ⌥⌘↓, ⌥⌘↑ and clicks on rows show tabs, a tab a page opens goes under it and closing
#      it goes back there, and ⌘W otherwise shows the tab below, or above at the bottom. Closing
#      the last tab leaves the Space empty, where ⌘T still works. Reports how long switches take.
#   4. The page view keeps its margins and GPUI's view fills the window through a resize and
#      full screen, and the window's buttons stay in the header row. A page that goes full
#      screen takes the window with it and escape brings both back, the command bar opens in
#      full screen and keeps the keyboard when a page commits under it, then gives the keyboard
#      to the new page, and scrolling a long page doesn't stutter. Dragging the window's
#      background moves the window, and the buttons, URL field and rows keep their presses. A
#      double-click there zooms it, and one in the sidebar below the rows opens a new tab.
#   5. A page's popup opens as a tab in the window, under the page.
#   6. Back and forward go through the tab's history from keys, with the page or the UI focused,
#      and from the header's buttons, which dim with nowhere to go. Reload loads the page again,
#      and while a page loads, reload is stop, which ⌘., escape and the button do. A tab's row
#      shows its page's favicon, and keeps it until the next page has loaded without one.
#   7. New Space in the Spaces menu makes a Space and shows it, empty and without a page, with a
#      form that takes the keyboard from the page. Japanese input composes in its field, return
#      while composing only commits, and return names the Space; ⌘T then opens a tab in it.
#      ⌃1–⌃9, ⌥⌘← and ⌥⌘→, with the page or the UI focused, and clicks on the switcher's dots show
#      another Space, which shows the tab it showed last. Return with no name does nothing, New
#      Space shows a form already open, and Cancel deletes its Space and goes back.
#   8. Launched again after ⌘Q, and again after its window's close button, the app brings back
#      the Spaces: their names, their tabs with each tab's history, the tab each showed and the
#      Space it showed. Only a tab shown loads; the others show their titles and favicons. A tab
#      closed before the quit stays closed. Reports how long putting them back takes.
# Runs 2 to 8 need Browser to be the active app, and run 4 takes it full screen for a moment.
# BROWSER_DISPLAY=builtin opens it on the Mac's own screen.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
set -e
cd "$(dirname "$0")"
app="$PWD/build/Browser.app"
out=build/check
# Another Browser holds the profile directory, so this one couldn't start.
if pgrep -f "$app/Contents" > /dev/null; then
  echo "FAIL: Browser is already running. Quit it first." >&2
  exit 1
fi
rm -rf "$out"
mkdir -p "$out"
# The Spaces the app saves for its next launch. Each run starts without them, but the relaunches.
saved="$HOME/Library/Caches/browser-app/Spaces.json"
# After a failure, nothing the check started keeps running.
cleanup() {
  # The server may be gone already, and under set -e a failed kill would end the trap here.
  [ -z "${server:-}" ] || kill "$server" 2>/dev/null || true
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
    kill -0 "$pid" 2>/dev/null || fail "the app exited"
    sleep 0.1
  done
  return 1
}

# The app logs "browser: loaded" when the main frame has loaded.
await_load() {
  await '^browser: load' || fail "the page didn't finish loading"
  grep -q '^browser: loaded' "$log" || fail "the page didn't load"
}

# Waits up to $1 seconds (10) for the app to exit and checks that no Chromium process outlives it.
exited() {
  for _ in $(seq $((${1:-10} * 2))); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  if kill -0 "$pid" 2>/dev/null; then
    sample "$pid" 2 -file "$out/hang.txt" > /dev/null 2>&1
    fail "the app didn't quit. Where it was: $out/hang.txt"
  fi
  wait "$pid" || fail "the app exited with status $?"
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
  osascript -e 'tell application id "dev.browser.app" to quit'
  exited "$@"
}

log="$out/load.log"
rm -f "$saved"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await_load
../tools/window-screenshot.sh "$pid" "$out/load.png" || fail "no window to capture"
quit
echo "ok: $(grep '^browser: loaded' "$log" | cut -d' ' -f4-) loaded, quit cleanly. Window: $out/load.png"

# The test page puts what reached it in its title, which the app logs. Chromium drops input
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
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the test page didn't draw"

# Sends a command to the app's driver (platform/mac/drive.mm) and waits until AppKit has handled it.
step=0
send() {
  step=$((step + 1))
  echo "$*" >&3
  await "^browser: drive $step done" || fail "the driver didn't finish \"$*\""
  grep -q "^browser: drive $step: can't do that" "$log" && fail "the driver can't do \"$*\""
  done=$(grep "^browser: drive $step done" "$log")
  key=$(echo "$done" | sed 's/.*key \([^,]*\),.*/\1/')
  focus=$(echo "$done" | sed 's/.*focus \([^,]*\),.*/\1/')
  active=$(echo "$done" | sed 's/.*active \([01]\).*/\1/')
  marked=$(echo "$done" | sed 's/.*marked //')
}
# Waits for a line matching $1 logged after the last command was sent.
expect() {
  for _ in $(seq 100); do
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    sleep 0.1
  done
  fail "$2"
}

# Points from the window's top-left corner. The sidebar is 228 wide.
page_point="600 400"
sidebar_point="100 400"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: keyboard, focus, window states and scrolling. Browser wasn't the active app, so the"
  echo "  page ignored the click. macOS decides which app is active; a background process can't."
  exit 0
fi
[ "$focus" = RenderWidgetHostViewCocoa ] || fail "a click on the page left focus on $focus"
send type abc
expect '^browser: title [0-9]* value abc$' "typing didn't reach the focused page"
# In a text field, ⌘← and ⌘→ move the caret, so they don't come back to go back and forward.
send key left cmd
send key right cmd

# Chrome's browser window has the keyboard, with focus on $1.
in_browser() {
  [ "$key" = BrowserCoreNSWindow ] && [ "$focus" = "$1" ] ||
    fail "$2: the keyboard is in $key, on $focus"
}
# ⌘$1 with the page focused reaches the UI as command $2.
page_shortcut() {
  send key "$1" cmd
  expect "^browser: command $2 via page$" "⌘$(echo "$1" | tr a-z A-Z) with the page focused didn't reach the UI"
}
for shortcut in "t new-tab" "l edit-url"; do
  page_shortcut $shortcut
  expect '^browser: command bar shown$' "$shortcut didn't open the command bar"
  # A shortcut the page doesn't keep comes back after the page has had it, so after the
  # driver's done.
  send state
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "the command bar opened without the keyboard, which is in $key, on $focus"
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the command bar"
  in_browser RenderWidgetHostViewCocoa "after the command bar closed"
done
send key f cmd
expect '^browser: title [0-9]* page saw cmd-f$' "⌘F with the page focused didn't reach the page"

send click $sidebar_point
# GPUI's view in a window it doesn't own, which AccessKit subclasses.
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
send type x
send key f cmd
expect '^browser: command find via ui$' "⌘F with the UI focused didn't reach the UI"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T with the UI focused didn't reach the UI"
expect '^browser: command bar shown$' "⌘T with the UI focused didn't open the command bar"
send click $page_point
expect '^browser: command bar hidden$' "a click on the page didn't close the command bar"
in_browser RenderWidgetHostViewCocoa "after a click on the page closed the command bar"
send type d
# Input reaches the page in order, so by now it has seen the x and the shortcuts, if they got there.
expect '^browser: title [0-9]* value abcd$' "the page got keys meant for the UI: $(grep '^browser: title' "$log" | tail -1)"
grep -q '^browser: title [0-9]* page saw cmd-[tw]$' "$log" && fail "the page saw a reserved shortcut"
grep -q '^browser: title [0-9]* page saw cmd-l$' "$log" || fail "⌘L with the page focused didn't reach the page first"
grep -q '^browser: command find via page$' "$log" && fail "the page kept ⌘F, but the UI ran it too"
grep -q '^browser: command \(back\|forward\) via page$' "$log" && fail "⌘← or ⌘→ in the page's text field went back or forward"

# The driver's scripted input method stands in for Japanese input, which is system-wide.
send ime on
send type nihongo
expect '^browser: title [0-9]* value abcdnihongo$' "the page didn't show the composition"
send key space
expect '^browser: title [0-9]* value abcd日本語$' "the page didn't show the conversion"
send key return
send ime off
send type x
expect '^browser: title [0-9]* value abcd日本語x$' "return in the page did more than commit: $(grep '^browser: title' "$log" | tail -1)"

# Nothing logged since the last command was sent matches $1.
none_since() {
  ! sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1"
}
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the command bar didn't take the composition: marked $marked"
send key escape
none_since '^browser: command bar hidden$' || fail "escape while composing closed the command bar"
[ "$marked" = none ] || fail "escape didn't cancel the composition: marked $marked"
send type nihongo
send key space
[ "$marked" = 0,3 ] || fail "the command bar didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/ime.png" || fail "no window to capture"
send key return
none_since '^browser: command bar go' || fail "return while composing went somewhere"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send type .invalid
send key return
expect '^browser: command bar go https://日本語.invalid$' "the command bar didn't keep the committed text: $(grep '^browser: command bar go' "$log" | tail -1)"

send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.com
../tools/window-screenshot.sh "$pid" "$out/command-bar.png" || fail "no window to capture"
send key return
expect '^browser: command bar go https://example.com$' "return in the command bar didn't go to what was typed"
expect '^browser: command bar hidden$' "going somewhere didn't close the command bar"
expect '^browser: loaded [0-9]* https://example.com/$' "the page didn't load what was typed in the command bar"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar loaded a page"
# The bar draws itself before it goes on screen, so it never shows the last query: empty for a
# new tab, and on ⌘L with the page's URL.
awk '/^browser: command (new-tab|edit-url) via/ { command = $3 }
  /^browser: command bar drew/ { drew = $0 }
  /^browser: command bar shown$/ && (command == "new-tab") != (drew == "browser: command bar drew \"\"") { exit 1 }' "$log" ||
  fail "the command bar went on screen still showing what it drew last"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via page$' || fail "⌘Q with the page focused didn't reach the UI"
exited
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q quits, the command bar takes the keyboard and gives it back, Japanese input composes in both. Windows: $out/command-bar.png, $out/ime.png"

# A Space's Today tabs. The page opens a tab of its own from its button, the way a link to a new
# tab does. The app asks Chrome to trace its tab switches, timed as Chrome times its own: from the
# tab strip's change to the first frame shown. The trace is written as the app quits. In a handler
# attribute a bare open() is document.open(), which would blank the page instead.
page='data:text/html,<title>loading</title>
<button onclick=window.open() style="font:24px system-ui">Open a tab</button>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/tabs.log"
trace="$PWD/$out/trace.json"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" --enable-tracing=latency \
  --enable-tracing-format=json --trace-startup-file="$trace" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the tabs test page didn't draw"

# The Today tabs the app last showed, top to bottom.
today() {
  grep '^browser: today' "$log" | tail -1 | sed 's/^browser: today *//'
}
# Waits for the app to show Today as $1, with $2 shown. $3 says what went wrong if it doesn't.
expect_tabs() {
  expect "^browser: today $1\$" "$3: Today is \"$(today)\", not \"$1\""
  expect "^browser: selected $2\( in .*\)\{0,1\}\$" "$3: tab $2 isn't shown"
}
# Opens a tab from the command bar, to https://$1.
open_from_bar() {
  before=$(today)
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  send type "$1"
  send key return
  expect "^browser: today [0-9]* $before\$" "the tab for $1 didn't open at the top of Today"
  tab=$(today | cut -d' ' -f1)
  expect "^browser: selected $tab\$" "the tab for $1 wasn't shown"
  expect "^browser: loaded $tab https://$1/\$" "$1 didn't load in its tab"
}
# The driver's key $1 shows tab $2, as a switch the app times.
switch() {
  send key $1
  expect "^browser: selected $2 in " "key $1 didn't show tab $2 of \"$(today)\""
}
# The center of row $1 of Today, counting from 0: rows are 41 apart under "+ New Tab", which is
# under the Space's name, 44 tall and 90 below the window's top.
row() {
  echo "100 $((90 + 44 + 41 * ($1 + 1) + 20))"
}

await '^browser: today [0-9][0-9]*$' || fail "the first tab isn't in Today"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: tabs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
expect_tabs "$c $b $a" "$c" "after opening two tabs"
../tools/window-screenshot.sh "$pid" "$out/tabs.png" || fail "no window to capture"

switch "3 cmd" "$a"
switch "1 cmd" "$c"
switch "9 cmd" "$a"
switch "up alt cmd" "$b"
switch "down alt cmd" "$a"
# Next wraps around, as in Chrome.
switch "down alt cmd" "$c"
send click $(row 1)
expect "^browser: selected $b in " "a click on the second row didn't show its tab"
send click 100 154
expect '^browser: command new-tab via sidebar$' "a click on \"+ New Tab\" didn't run New Tab"
expect '^browser: command bar shown$' "a click on \"+ New Tab\" didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

switch "3 cmd" "$a"
send click 290 34
expect "^browser: tab [0-9]* opened by $a in the foreground\$" "the page's button didn't open a tab"
d=$(grep '^browser: tab [0-9]* opened by' "$log" | tail -1 | cut -d' ' -f3)
expect_tabs "$c $b $a $d" "$d" "a tab the page opened"
send key w cmd
expect_tabs "$c $b $a" "$a" "after closing the page's tab"

switch "1 cmd" "$c"
send key w cmd
expect_tabs "$b $a" "$b" "after closing the top tab"
switch "9 cmd" "$a"
send key w cmd
expect_tabs "$b" "$b" "after closing the bottom tab"
send key w cmd
expect_tabs " *" 0 "after closing the last tab"
../tools/window-screenshot.sh "$pid" "$out/empty.png" || fail "no window to capture"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in an empty Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in an empty Space didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
exec 3>&-
# Chrome writes the trace before it quits, once every process has started tracing or 10 s have
# passed, so a renderer that closed early can hold the quit that long.
quit 30

# Switch times in milliseconds, one a line, as "median <m> ms, max <x> ms, <n> switches".
summary() {
  sort -n | awk '{ v[NR] = $1 } END { if (NR) printf "median %.1f ms, max %.1f ms, %d switches", v[int((NR + 1) / 2)], v[NR], NR }'
}
app_ms=$(sed -n 's/^browser: selected [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
[ -s "$trace" ] || fail "the app didn't write its trace"
chrome_ms=$(python3 -I - "$trace" << 'PY' | summary
import json, sys
# Chrome's tab switch spans, begin and end events paired by id, in microseconds. A switch to a tab
# whose page hasn't loaded yet, as when a new tab shows, times the page load; Chrome reports those
# apart from switches, and so does this.
events = json.load(open(sys.argv[1]))
events = events["traceEvents"] if isinstance(events, dict) else events
begins = {}
for event in events:
    if event.get("name") != "TabSwitching::Latency":
        continue
    if event.get("ph") == "b":
        state = event.get("args", {}).get("tab_switch_measurement", {}).get("tab_state", "")
        begins[event.get("id")] = (event["ts"], "NOT_LOADED" in state)
    elif event.get("ph") == "e" and event.get("id") in begins:
        begin, not_loaded = begins.pop(event.get("id"))
        if not not_loaded:
            print((event["ts"] - begin) / 1000)
PY
)
[ -n "$chrome_ms" ] || fail "the trace has no tab switches"
echo "ok: new tabs open at the top and show, keys and clicks switch, a page's tab goes under it and closing it goes back, ⌘W shows the tab below or above, an empty Space still opens tabs."
echo "  Switch to a loaded tab, tab strip to frame shown (Chrome's measure; gate 16 ms): $chrome_ms."
echo "  Switch, key to the core's report: $app_ms. Windows: $out/tabs.png, $out/empty.png"

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
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="data:text/html;base64,$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the scrolling page didn't draw"

# The page view keeps the sidebar's 228 points on its left and 10 on its other sides, over GPUI's
# view, which fills the window. A full-screen page ("fills") covers the whole window.
check_layout() {
  send layout
  layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout')
  content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
  view=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
  ui=$(echo "$layout" | sed -n 's/.*GPUIView \([0-9,]* [0-9x]*\).*/\1/p')
  width=${content%x*}
  height=${content#*x}
  expected="228,10 $((width - 238))x$((height - 20))"
  [ "${2:-}" = fills ] && expected="0,0 $content"
  [ "$view" = "$expected" ] || fail "$1: the page view is at $view in a $content window"
  [ "$ui" = "0,0 $content" ] || fail "$1: GPUI's view is at $ui in a $content window"
}
# The window's buttons sit in the header row, where Arc's do. In full screen they come down with
# the menu bar instead.
check_buttons() {
  buttons=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout buttons' |
    cut -d' ' -f4-)
  [ "$buttons" = "20.0,24.0 43.0,24.0 66.0,24.0" ] ||
    fail "$1: the window's buttons are at $buttons, not 20,24 43,24 66,24"
}
check_layout "at launch"
check_buttons "at launch"

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
drag 100 24 moves "the header"
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
  send doubleclick 100 24
  expect '^browser: window zoomed 1$' "a double-click on the header didn't zoom the window"
  send doubleclick 100 24
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
exec 3>&-
quit
echo "ok: the page view keeps its margins and the window's buttons stay at Arc's through a resize and full screen, a full-screen page takes the window with it and escape brings both back, the command bar opens in full screen and keeps the keyboard when a page commits under it, then gives it to the new page. Drags on the window's background move it and drags on its controls don't. A double-click on the header $zoom, and one on the empty sidebar opens a new tab. Scrolling: $timing. Windows: $out/fullscreen.png, $out/page-fullscreen.png"

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
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the popup test page didn't draw"
send click $page_point
send key p
expect '^browser: tab [0-9]* opened by [0-9]* in the foreground$' "the page's popup didn't open as a tab"
opener=$(grep -m1 '^browser: today [0-9]' "$log" | cut -d' ' -f3)
popup=$(grep '^browser: tab [0-9]* opened by' "$log" | cut -d' ' -f3)
expect "^browser: today $opener $popup\$" "the popup's tab didn't go under the page: $(grep '^browser: today' "$log" | tail -1)"
expect '^browser: loaded [0-9]* https://example.com/$' "the popup's page didn't load in its tab"
check_layout "with the popup's tab"
../tools/window-screenshot.sh "$pid" "$out/popup.png" || fail "no window to capture"
exec 3>&-
quit
echo "ok: a page's popup opens as a tab in the window, under the page. Window: $out/popup.png"

# Back, forward, reload and stop. stuck.test leads to a server that takes connections and never
# answers, so its page keeps loading until it's stopped.
python3 -I -c '
import socket, time
server = socket.socket()
server.bind(("127.0.0.1", 0))
server.listen()
print(server.getsockname()[1], flush=True)
time.sleep(600)' > "$out/port" &
server=$!
for _ in $(seq 50); do
  [ -s "$out/port" ] && break
  sleep 0.1
done
[ -s "$out/port" ] || fail "the server that never answers didn't start"
# The start page's favicon is a 16 × 16 orange square.
page='data:text/html,<title>loading</title>
<link rel="icon" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg==">
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/history.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" \
  --host-resolver-rules="MAP stuck.test 127.0.0.1:$(cat "$out/port")" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the history test page didn't draw"
tab=$(grep -m1 '^browser: today [0-9]' "$log" | cut -d' ' -f3)
await "^browser: page $tab back 0 forward 0 loading 0 data:" || fail "a fresh tab can go back or forward"
await "^browser: favicon $tab [1-9][0-9]* bytes\$" || fail "the start page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: history. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The header's buttons, centred 23.5 below the window's top.
back_button="138 24"
forward_button="172 24"
reload_button="206 24"

# Waits for the tab's page to be at https://$1/, done loading, with back and forward as $2 and $3
# (1 or 0). $4 says when.
expect_page() {
  expect "^browser: page $tab back $2 forward $3 loading 0 https://$1/\$" "$4: the tab isn't at $1 with back $2 and forward $3: $(grep "^browser: page $tab" "$log" | tail -1)"
}
# Loads https://$1 in the tab from the command bar.
go_from_bar() {
  send key l cmd
  expect '^browser: command bar shown$' "⌘L didn't open the command bar"
  send type "$1"
  send key return
  expect '^browser: command bar hidden$' "going to $1 didn't close the command bar"
}
# The driver's key $1, with ⌘, runs command $2, which the UI hears from side $3. Chromium and GPUI
# name keys through the Mac's keyboard layout, where the keys for ⌘[ and ⌘] on a US one can be
# others (å and a dead key on a Danish one), so this drives ⌘← and ⌘→, the same on every layout.
history_key() {
  send key "$1" cmd
  expect "^browser: command $2 via $3\$" "⌘$1 with the $3 focused didn't run $2"
}

go_from_bar example.com
expect_page example.com 1 0 "after loading a page"
# example.com has no favicon, so the start page's goes, but only once example.com has loaded.
expect "^browser: favicon $tab 0 bytes\$" "the start page's favicon stayed on example.com"
dropped=$(grep -n "^browser: favicon $tab 0 bytes\$" "$log" | head -1 | cut -d: -f1)
loaded=$(grep -n "^browser: page $tab back 1 forward 0 loading 0 https://example.com/\$" "$log" | head -1 | cut -d: -f1)
[ "$dropped" -gt "$loaded" ] || fail "the start page's favicon went before example.com had loaded"
go_from_bar example.org
expect_page example.org 1 0 "after loading another"
../tools/window-screenshot.sh "$pid" "$out/history.png" || fail "no window to capture"
history_key left back page
expect_page example.com 1 1 "after ⌘←"
history_key right forward page
expect_page example.org 1 0 "after ⌘→"
send click $back_button
expect '^browser: command back via sidebar$' "a click on back didn't go back"
expect_page example.com 1 1 "after a click on back"
send click $forward_button
expect '^browser: command forward via sidebar$' "a click on forward didn't go forward"
expect_page example.org 1 0 "after a click on forward"
# The click left the keyboard with the UI.
history_key left back ui
expect_page example.com 1 1 "after ⌘← with the UI focused"
history_key right forward ui
expect_page example.org 1 0 "after ⌘→ with the UI focused"

history_key r reload ui
expect "^browser: loaded $tab https://example.org/\$" "⌘R didn't load the page again"
send key r cmd shift
expect '^browser: command force-reload via ui$' "⇧⌘R didn't run force-reload"
expect "^browser: loaded $tab https://example.org/\$" "⇧⌘R didn't load the page again"
send click $reload_button
expect '^browser: command reload via sidebar$' "a click on reload didn't reload"
expect "^browser: loaded $tab https://example.org/\$" "a click on reload didn't load the page again"

# While a page loads, the URL field shows where it's going and reload is stop.
send click $page_point
for stop in "key . cmd|page|⌘." "key escape|page|escape" "click $reload_button|sidebar|a click on stop"; do
  IFS='|' read -r action from what << EOF
$stop
EOF
  go_from_bar stuck.test
  expect "^browser: page $tab back 1 forward 0 loading 1 https://stuck.test/\$" "stuck.test didn't show as loading"
  if [ "$from" = sidebar ]; then
    ../tools/window-screenshot.sh "$pid" "$out/loading.png" || fail "no window to capture"
  fi
  send $action
  expect "^browser: command stop via $from\$" "$what while loading didn't run stop"
  expect_page example.org 1 0 "after $what stopped the load"
done
exec 3>&-
quit
kill "$server"
echo "ok: ⌘← ⌘→ and the header's buttons go back and forward, with the page or the UI focused; reload and force reload load the page again; ⌘., escape and the stop button stop a load; the row keeps a page's favicon until the next page has loaded without one. Windows: $out/history.png, $out/loading.png"

# Spaces. A new one is empty, and each keeps its tabs and the one it showed last.
page='data:text/html,<title>loading</title>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/spaces.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Spaces test page didn't draw"
# The Space the app last showed.
space() {
  grep '^browser: space [0-9]' "$log" | tail -1 | cut -d' ' -f3
}
first=$(space)
await "^browser: spaces $first\$" || fail "the app didn't start with one Space"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The switcher's dots are centred under the sidebar, 25 above the window's bottom.
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
dots_y=$((${content#*x} - 25))

send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run new-space"
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: space $second in " "the window didn't show the new Space"
expect_tabs " *" 0 "in the new Space"
send layout
sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' | grep -q WebContentsViewCocoa &&
  fail "the new Space shows the first Space's page"
# The form's field has the keyboard, which the page had.
in_browser AccessKitSubclassOfGPUIView "in the new Space's form"
../tools/window-screenshot.sh "$pid" "$out/space-form.png" || fail "no window to capture"
send type reading
send key space
send ime on
send type nihongo
send key space
[ "$marked" = 8,3 ] || fail "the form's field didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/space-form-ime.png" || fail "no window to capture"
send key return
none_since '^browser: create space' || fail "return while composing named the Space"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send key return
expect "^browser: create space $second reading 日本語\$" "return didn't name the Space: $(grep '^browser: create space' "$log" | tail -1)"
expect "^browser: renamed space $second reading 日本語\$" "the core didn't rename the Space"
../tools/window-screenshot.sh "$pid" "$out/new-space.png" || fail "no window to capture"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in the new Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
b=$(today)
expect "^browser: selected $b\$" "the new Space's tab wasn't shown"
expect "^browser: loaded $b https://example.com/\$" "example.com didn't load in the new Space's tab"

# $1 is the driver's key or click, $2 the command it runs, $3 where the UI hears it from, and $4
# the Space it shows, with its tab $5.
show_space() {
  send $1
  expect "^browser: command $2 via $3\$" "$1 with the $3 focused didn't run $2"
  expect "^browser: space $4 in " "$1 with the $3 focused didn't show Space $4"
  # The menu shows the shortcut too, and a key the shell ran never reaches it.
  [ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: command ')" = 1 ] ||
    fail "$1 with the $3 focused ran more than one command"
  expect_tabs "$5" "$5" "after $1 showed Space $4"
}
send click $page_point
show_space "key 1 ctrl" select-space page "$first" "$a"
send click $page_point
show_space "key 2 ctrl" select-space page "$second" "$b"
send click $page_point
show_space "key left alt cmd" previous-space page "$first" "$a"
send click $page_point
show_space "key right alt cmd" next-space page "$second" "$b"
# Spaces don't wrap around.
send key right alt cmd
expect '^browser: command next-space via page$' "⌥⌘→ at the last Space didn't reach the UI"
send state
none_since '^browser: space [0-9]' || fail "⌥⌘→ at the last Space showed another"
send click $sidebar_point
show_space "key 1 ctrl" select-space ui "$first" "$a"
show_space "key right alt cmd" next-space ui "$second" "$b"
show_space "key left alt cmd" previous-space ui "$first" "$a"
show_space "click 131 $dots_y" select-space sidebar "$second" "$b"
show_space "click 97 $dots_y" select-space sidebar "$first" "$a"
../tools/window-screenshot.sh "$pid" "$out/spaces.png" || fail "no window to capture"

send menu New Space…
expect "^browser: spaces $first $second [0-9]*\$" "New Space didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: space $third in " "the window didn't show the third Space"
send key return
none_since '^browser: create space' || fail "return with no name named the Space"
show_space "key 1 ctrl" select-space ui "$first" "$a"
send menu New Space…
expect "^browser: space $third in " "New Space didn't show the form already open"
none_since '^browser: spaces ' || fail "New Space with a form open made another Space"
send state
in_browser AccessKitSubclassOfGPUIView "back in the third Space's form"
send click 114 $((${content#*x} - 74))
expect "^browser: cancel space $third\$" "a click on Cancel didn't cancel"
expect "^browser: space $first in " "Cancel didn't show the first Space again"
expect "^browser: spaces $first $second\$" "Cancel didn't delete the third Space"
expect_tabs "$a" "$a" "after Cancel"
exec 3>&-
quit
spaces_ms=$(sed -n 's/^browser: space [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: New Space makes an empty Space whose form takes the keyboard and Japanese input, and names it, and ⌘T opens a tab in it; ⌃1 ⌃2 ⌥⌘← ⌥⌘→ with the page or the UI focused, and the switcher's dots, show each Space with the tab it showed last; Cancel deletes a new Space and goes back."
echo "  Space switch, key to the core's report: $spaces_ms. Windows: $out/space-form.png, $out/space-form-ime.png, $out/new-space.png, $out/spaces.png"

# The Spaces come back after a quit and after the window closes: their names, their tabs with each
# tab's history, the tab each showed and the Space the window showed. Only a tab shown loads, and
# one that doesn't shows its title and its favicon from the profile's history, which keeps none
# for data: pages, so the first page is served. A tab the user closed stays closed.
mkdir "$out/site"
printf '%s' '<title>loading</title><link rel="icon" href="icon.png">
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>' > "$out/site/index.html"
printf '%s' iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg== |
  base64 -D > "$out/site/icon.png"
python3 -I -u -m http.server 0 --bind 127.0.0.1 --directory "$out/site" > "$out/site.log" 2>&1 &
server=$!
for _ in $(seq 50); do
  grep -q '^Serving HTTP' "$out/site.log" && break
  sleep 0.1
done
port=$(sed -n 's/^Serving HTTP on .* port \([0-9]*\) .*/\1/p' "$out/site.log")
[ -n "$port" ] || fail "the page server didn't start"
site="http://127.0.0.1:$port/"
log="$out/restore.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the restore test page didn't draw"
first=$(space)
a=$(today)
tab=$a
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "the served page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restoring Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Tab a goes to two pages and back to the first, so it has pages to go forward to.
go_from_bar example.com
expect_page example.com 1 0 "after loading a page"
go_from_bar example.org
expect_page example.org 1 0 "after loading another"
history_key left back page
expect_page example.com 1 1 "after going back"
history_key left back page
expect "^browser: page $a back 0 forward 1 loading 0 $site\$" "tab a didn't go back to the first page"
open_from_bar example.net
c=$tab
open_from_bar example.org
send key w cmd
expect_tabs "$c $a" "$c" "after ⌘W closed the tab for example.org"
send menu New Space…
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
send type reading list
send key return
expect "^browser: create space $second reading list\$" "return didn't name the Space"
send key t cmd
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
e=$(today)
expect "^browser: loaded $e https://example.com/\$" "example.com didn't load in the new Space's tab"
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
expect_tabs "$c $a" "$c" "back in the first Space"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the app shows the first Space with the tab for example.net, and tab a under it
# at the served page, unloaded.
log="$out/restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "the relaunch didn't bring back two Spaces"
first=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f3)
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
grep -q "^browser: added space $first Space 1\$" "$log" || fail "the first Space came back without its name"
grep -q "^browser: added space $second reading list\$" "$log" || fail "the second Space came back without its name"
await "^browser: space $first\$" || fail "the relaunch didn't show the first Space"
# Without the tab closed before the quit.
await '^browser: today [0-9]* [0-9]*$' || fail "the first Space didn't come back with its two tabs"
c=$(today | cut -d' ' -f1)
a=$(today | cut -d' ' -f2)
await "^browser: selected $c\$" || fail "the first Space didn't show the tab it showed last"
await "^browser: loaded $c https://example.net/\$" || fail "the tab shown didn't load"
await "^browser: page $a back 0 forward 1 loading 0 $site\$" ||
  fail "tab a didn't come back at its page with history: $(grep "^browser: page $a" "$log" | tail -1)"
await "^browser: title $a ready\$" || fail "tab a didn't come back with its title"
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "tab a didn't come back with its favicon"
grep -q "^browser: loaded $a " "$log" && fail "tab a loaded without being shown"
await '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' ||
  fail "the second Space's tab didn't come back at example.com"
e=$(grep -m1 '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' "$log" | cut -d' ' -f3)
grep -q "^browser: loaded $e " "$log" && fail "the second Space's tab loaded without being shown"
../tools/window-screenshot.sh "$pid" "$out/restored.png" || fail "no window to capture"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restored history. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
switch "2 cmd" "$a"
expect "^browser: loaded $a $site\$" "tab a didn't load once shown"
tab=$a
history_key right forward page
expect_page example.com 1 1 "going forward in tab a's restored history"
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect_tabs "$e" "$e" "in the second Space"
expect "^browser: loaded $e https://example.com/\$" "the second Space's tab didn't load once shown"
# The window's close button, which ends the app too: the driver doesn't report it done.
echo "click 20 24" >&3
exec 3>&-
await '^browser: window closed$' || fail "a click on the close button didn't close the window"
exited

# Closing the window kept them too, and the second Space shows.
log="$out/restored-again.log"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch after closing the window didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "closing the window didn't keep both Spaces"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
await "^browser: space $second\$" || fail "the launch after closing the window didn't show the second Space"
await '^browser: today [0-9]*$' || fail "the second Space didn't come back with its tab"
await "^browser: loaded $(today) https://example.com/\$" || fail "the second Space's tab didn't load"
await '^browser: page [0-9]* back 1 forward 1 loading 0 https://example.com/$' ||
  fail "tab a didn't keep the page it went forward to"
quit
kill "$server"
restore_ms=$(sed -n 's/^browser: restored in \([0-9.]*\) ms$/\1/p' "$out/restored.log" "$out/restored-again.log" | tr '\n' ' ')
echo "ok: after ⌘Q and after closing the window, the Spaces come back with their names, tabs, history and the tab each showed, in the Space shown last; only tabs shown load, the others show their titles and favicons, and a closed tab stays closed."
echo "  Restore, 3 tabs in 2 Spaces: ${restore_ms}ms. Window: $out/restored.png"
