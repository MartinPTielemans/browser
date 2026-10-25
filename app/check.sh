#!/bin/sh
# Runs the app fifteen times and checks:
#   1. The start page loads in the browser window, over the GPUI sidebar drawn in it, and
#      quitting through AppKit closes the page and ends every Chromium process.
#   2. Clicks move keyboard focus between the page and the UI, and typing goes where the focus
#      is. With the page focused, the browser's reserved shortcuts reach the UI and never the
#      page, and the others reach the page first, which can keep them. ⌘Q quits. The command
#      bar opens over the page with the keyboard, never showing the last query, gives the
#      keyboard back when it closes, and loads what's typed in it, over https for a host typed
#      without one, or searches DuckDuckGo for words. Japanese input composes and commits in the
#      page and in the command bar, where return and escape go to the input method while it
#      composes.
#   3. A tab opened from the command bar goes at the top of the Space's Today tabs and shows. The
#      command bar suggests pages from history as it's typed in, growing to fit them, with the
#      tabs already at them, never the one shown, and well-known sites under them. ↑, ↓ and ⌃N
#      choose a row, wrapping at the ends, and ↩ or a click on a tab's row shows that tab without
#      opening one. Typing at the end completes the text to the first row's page, which ⌫ takes
#      back, → keeps and ↩ goes to. ⌘1–⌘9, ⌥⌘↓, ⌥⌘↑ and clicks on rows show tabs, a tab a page
#      opens goes under it and closing it goes back there, and ⌘W otherwise shows the tab below,
#      or above at the bottom. Closing the last tab leaves the Space empty, where ⌘T still works.
#      Reports how long switches take.
#   4. The page view keeps its margins and GPUI's view fills the window through a resize and
#      full screen, and the window's buttons stay in the header row. A page that goes full
#      screen takes the window with it and escape brings both back, the command bar opens in
#      full screen and keeps the keyboard when a page commits under it, then gives the keyboard
#      to the new page, and scrolling a long page doesn't stutter. Dragging the window's
#      background moves the window, and the buttons, URL field and rows keep their presses. A
#      double-click there zooms it, and one in the sidebar below the rows opens a new tab. ⌘S,
#      with the page or the UI focused, and the sidebar button hide the sidebar and the window's
#      buttons, and the card takes its place, through a resize; ⌘S brings them back. The
#      sidebar's edge drags its width between 160 and 400, rubber-banding past them, and a
#      double-click on it puts the width back.
#   5. A page's popup opens as a tab in the window, under the page.
#   6. Back and forward go through the tab's history from keys, with the page or the UI focused,
#      and from the header's buttons, which dim with nowhere to go. Reload loads the page again,
#      and while a page loads, reload is stop, which ⌘., escape and the button do. A tab's row
#      shows its page's favicon, and keeps it until the next page has loaded without one.
#   7. New Space in the Spaces menu makes a Space and shows it, empty and without a page, with a
#      form that takes the keyboard from the page. Japanese input composes in its field, return
#      while composing only commits, and return names the Space; ⌘T then opens a tab in it.
#      ⌃1–⌃9, ⌥⌘← and ⌥⌘→, with the page or the UI focused, and clicks on the switcher's dots show
#      another Space, which shows the tab it showed last. Two fingers across the sidebar page to
#      the Space a flick or the pages' place points at, never past the last, fingers along it
#      don't, and a wheel's tick across pages once. A tab's menu moves the tab to another Space,
#      which the window shows with it. Return with no name does nothing, New Space shows a form
#      already open, and Cancel deletes its Space and goes back. A right-click on the Space's name
#      or a dot shows Rename Space, its colours and Delete Space. Rename, and a
#      double-click on the name, turn it into a field, where return keeps the name and escape
#      cancels. Delete asks first for a Space with tabs, which close with it and reopen with ⇧⌘T
#      in the Space shown, and the last Space can't be deleted.
#   8. Launched again after ⌘Q, and again after its window's close button, the app brings back
#      the Spaces: their names and colours, their tabs with each tab's history, the tab each showed
#      and the Space it showed, and the sidebar's width. Only a tab shown loads; the others show
#      their titles and favicons. A tab closed before the quit stays closed. A SIGTERM, which macOS
#      sends at logout and restart, keeps a tab opened just before it. Reports how long putting
#      them back takes.
#   9. ⌘F opens the find bar over the page with the keyboard. Typing finds in the page, with "n of
#      m", once Japanese input has committed, and return, shift-return, ⌘G and ⇧⌘G go through the
#      matches, from the bar and from the page. A click on the page leaves the bar open, and the
#      bar keeps its place through a resize and full screen. Each tab has its own find, which a
#      new tab starts from the text found last; escape closes it, and so does another page.
#  10. ⌘D pins the tab shown, from the page and from the UI, at the bottom of the pinned tabs, and
#      unpins it to the top of Today. ⌘1–⌘9 count the pinned tabs first. ⌘W, or the ✕ under the
#      pointer, unloads a pinned tab and keeps its row, which has no ✕ then, and shows the top
#      Today tab, else the nearest pinned tab still loaded, else no page, as closing the last
#      Today tab does. Shown again, the tab loads. Away from its pinned URL, ⌘↩ from the page and
#      the UI, and a click on its favicon, go back there. Launched again, the app brings back the
#      pinned tab pinned and away from its pinned URL, without loading it. Dragging a row moves its
#      tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back.
#  11. ⇧⌘K from the page and the UI, and Clear on the divider, close the Space's Today tabs and
#      show the lowest pinned tab still loaded, with a toast that hides by itself. Its Undo brings
#      the tabs back in place, showing the one shown and leaving the other unloaded. ⇧⌘T from the
#      page and the UI reopens the tab closed last, where it was and with its history, and after a
#      Clear, Undo brings back only the tabs ⇧⌘T didn't. A right-click on a row shows its tab's
#      menu, whose Duplicate Tab, Close Tabs Below and Unpin Tab work.
# Runs 2 to 11 need Browser to be the active app, and runs 4 and 9 take it full screen for a
# moment.
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
# A Space's colours in its menu, for a Space without one.
colors="Color › System Accent (checked), Color › Gray, Color › Red, Color › Orange, \
Color › Yellow, Color › Green, Color › Blue, Color › Pink, Color › Purple"
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
# Sends a command without waiting for it: a menu or an alert it opens keeps the driver's events
# until it closes.
post() {
  step=$((step + 1))
  echo "$*" >&3
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
none_since '^browser: command bar typed' || fail "return while composing went somewhere"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send type .invalid
send key return
expect '^browser: command bar typed 日本語.invalid$' "the command bar didn't keep the committed text: $(grep '^browser: command bar typed' "$log" | tail -1)"
# ⌘L goes to the page first, and a key the page being replaced hasn't answered leaves with it.
expect '^browser: loaded [0-9]* https://duckduckgo.com/' "the search for the committed text didn't load"

send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.com
../tools/window-screenshot.sh "$pid" "$out/command-bar.png" || fail "no window to capture"
send key return
expect '^browser: command bar go https*://example.com/$' "return in the command bar didn't go to what was typed: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "going somewhere didn't close the command bar"
expect '^browser: loaded [0-9]* https://example.com/$' "the page didn't load what was typed in the command bar"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar loaded a page"
# Words that name no page are a search, with DuckDuckGo until the user picks an engine.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type arc browser
send key return
expect '^browser: command bar go https://duckduckgo.com/?q=arc+browser$' "words in the command bar didn't search: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: loaded [0-9]* https://duckduckgo.com/?q=arc+browser' "the search didn't load"
../tools/window-screenshot.sh "$pid" "$out/search.png" || fail "no window to capture"
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
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q quits, the command bar takes the keyboard and gives it back, goes to a host over https and searches for words, Japanese input composes in both. Windows: $out/command-bar.png, $out/search.png, $out/ime.png"

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
  expect "^browser: today [0-9][0-9]*${before:+ $before}\$" "the tab for $1 didn't open at the top of Today"
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
# under the divider, 15 tall, and the Space's name, 44 tall and 90 below the window's top.
row() {
  echo "100 $((90 + 44 + 15 + 41 * ($1 + 1) + 20))"
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

# The command bar's last suggestions, as "<url>[ tab <id>]" a line.
suggestions() {
  grep '^browser: command bar suggestions' "$log" | tail -1 | sed 's/^[^:]*: [^:]* ms: //' |
    tr -s ', ' '\n ' | sed 's/^ //'
}
# The command bar's height logged last.
bar_height() {
  grep '^browser: command bar height' "$log" | tail -1 | sed 's/.* //'
}
# Opens the command bar and types "example", which completes to a page and suggests example.com
# and example.org. ⌫ takes the completion back, so the first row is the search for what was typed.
# Waits for $1's tab among the rows, and checks that $2's isn't, since $2 is shown.
suggest_example() {
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  send type example
  expect '^browser: command bar completes "example" with "\.' "the command bar didn't complete \"example\""
  send key backspace
  expect "^browser: command bar suggestions \"example\" in [0-9.]* ms: https://duckduckgo.com/?q=example, .*https://$1/ tab " \
    "the command bar didn't suggest $1's tab: $(suggestions | tr '\n' ' ')"
  rows=$(suggestions | wc -l | tr -d ' ')
  # The bar may have grown to as many rows before.
  for _ in $(seq 50); do
    [ "$(bar_height)" = $((54 + 8 + 36 * rows)) ] && break
    sleep 0.1
  done
  [ "$(bar_height)" = $((54 + 8 + 36 * rows)) ] || fail "the command bar is $(bar_height) tall, not fitting $rows rows"
  none_since "https://$2/ tab " || fail "the command bar suggested $2's tab, which is shown"
}
suggest_example example.com example.org
send key up
last=$(suggestions | tail -1 | cut -d' ' -f1)
expect "^browser: command bar selected $last\$" "↑ on the first row didn't select the last"
send key n ctrl
expect '^browser: command bar selected https://duckduckgo.com/?q=example$' "⌃N on the last row didn't select the first"
for _ in 1 2 3 4 5 6 7 8; do
  send key down
  grep '^browser: command bar selected' "$log" | tail -1 | grep -q ' https://example.com/$' && break
done
../tools/window-screenshot.sh "$pid" "$out/suggestions.png" || fail "no window to capture"
send key return
expect "^browser: command bar switch to tab $b\$" "↩ on example.com's row didn't switch to its tab"
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "↩ on example.com's row didn't show its tab"
[ "$(today)" = "$c $b $a" ] || fail "switching to a tab changed Today to \"$(today)\""
suggest_example example.org example.com
# Rows are 36 tall, under the 54 of the input and an inset of 4.
row=$(suggestions | grep -n "^https://example.org/ tab $c\$" | cut -d: -f1)
send clickkey 320 $((54 + 4 + 36 * (row - 1) + 18))
expect "^browser: command bar switch to tab $c\$" "a click on example.org's row didn't switch to its tab"
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "a click on example.org's row didn't show its tab"
[ "$(today)" = "$c $b $a" ] || fail "switching to a tab changed Today to \"$(today)\""

# Typing at the end completes the text to the first row's page, as Chrome's address bar does:
# example.com, typed before, completes "example.c". ⌫ takes the completion back until the next key
# typed at the end, → keeps it as typed, and ↩ goes where it goes.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.c
expect '^browser: command bar completes "example.c" with "om"$' "the command bar didn't complete \"example.c\" to example.com"
expect '^browser: command bar drew "example.com"$' "the command bar didn't show the completion"
../tools/window-screenshot.sh "$pid" "$out/completion.png" || fail "no window to capture"
send key backspace
expect '^browser: command bar suggestions "example.c" in ' "the command bar didn't suggest again after ⌫"
expect '^browser: command bar drew "example.c"$' "⌫ didn't take the completion back"
none_since '^browser: command bar completes' || fail "the command bar completed the text again after ⌫"
send type o
expect '^browser: command bar completes "example.co" with "m"$' "typing at the end after ⌫ didn't complete again"
send key right
expect '^browser: command bar suggestions "example.com" in ' "→ didn't keep the completion as typed"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.c
expect '^browser: command bar completes "example.c" with "om"$' "the command bar didn't complete \"example.c\" to example.com"
send key return
expect '^browser: command bar go https://example.com/$' "↩ didn't go where the completion goes: $(grep '^browser: command bar go' "$log" | tail -1)"
expect "^browser: loaded $c https://example.com/\$" "↩ didn't load the completion's page in the tab"
[ "$(today)" = "$c $b $a" ] || fail "going to a page in a tab changed Today to \"$(today)\""
# A click on the URL field opens the bar on the tab's URL, out of the field: at its left edge and
# with the input centred on it. ⌘T's is centred.
send click 100 64
expect '^browser: command edit-url via sidebar$' \
  "a click on the URL field didn't open the command bar"
expect '^browser: command bar shown$' "a click on the URL field didn't open the command bar"
expect '^browser: command bar drew "https://example.com/"$' \
  "the command bar from the URL field didn't show the tab's URL"
at=$(grep '^browser: command bar at' "$log" | tail -1 | cut -d' ' -f5-)
[ "$at" = "8 37 640 54" ] || fail "the command bar from the URL field opened at $at, not out of it"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

# Well-known sites fill the rows under the profile's own, never first, so ↩ still searches.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type g
expect '^browser: command bar suggestions "g" in [0-9.]* ms: https://duckduckgo.com/?q=g, .*https://www.google.com/, https://mail.google.com/' \
  "the command bar didn't suggest well-known sites under the search: $(suggestions | tr '\n' ' ')"
../tools/window-screenshot.sh "$pid" "$out/popular.png" || fail "no window to capture"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

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
echo "ok: new tabs open at the top and show, the command bar suggests pages, their tabs and well-known sites and completes what's typed, keys and clicks switch, a page's tab goes under it and closing it goes back, ⌘W shows the tab below or above, an empty Space still opens tabs."
echo "  Switch to a loaded tab, tab strip to frame shown (Chrome's measure; gate 16 ms): $chrome_ms."
echo "  Switch, key to the core's report: $app_ms. Windows: $out/tabs.png, $out/suggestions.png, $out/completion.png, $out/popular.png, $out/empty.png"

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
# view, which fills the window. A full-screen page ("fills") covers the whole window, and with the
# sidebar hidden ("inset") it's 10 in from every side.
check_layout() {
  send layout
  layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout')
  content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
  view=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
  radius=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa [0-9,]* [0-9x]* radius \([0-9]*\).*/\1/p')
  ui=$(echo "$layout" | sed -n 's/.*GPUIView \([0-9,]* [0-9x]*\).*/\1/p')
  width=${content%x*}
  height=${content#*x}
  # Past the sidebar, 228 wide unless $2 gives its width.
  left=228
  case "${2:-}" in [0-9]*) left=$2 ;; esac
  expected="$left,10 $((width - left - 10))x$((height - 20))"
  # Rounded as the card's corners, square when it fills the window.
  expected_radius=6
  [ "${2:-}" = fills ] && expected="0,0 $content" && expected_radius=
  [ "${2:-}" = inset ] && expected="10,10 $((width - 20))x$((height - 20))"
  [ "$view" = "$expected" ] || fail "$1: the page view is at $view in a $content window"
  [ "$radius" = "$expected_radius" ] ||
    fail "$1: the page's corners have a radius of ${radius:-0}, not ${expected_radius:-0}"
  [ "$ui" = "0,0 $content" ] || fail "$1: GPUI's view is at $ui in a $content window"
}
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
drag 80 24 moves "the header"
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
  send doubleclick 80 24
  expect '^browser: window zoomed 1$' "a double-click on the header didn't zoom the window"
  send doubleclick 80 24
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
send click 104 24
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
send move 3 300
expect '^browser: reveal shown$' "the pointer at the window's edge didn't bring the sidebar out"
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
send click 104 24
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
echo "ok: the page view keeps its margins and the window's buttons stay at Arc's through a resize and full screen, a full-screen page takes the window with it and escape brings both back, the command bar opens in full screen and keeps the keyboard when a page commits under it, then gives it to the new page. Drags on the window's background move it and drags on its controls don't. A double-click on the header $zoom, and one on the empty sidebar opens a new tab. ⌘S from the page and the UI, and the sidebar button, hide the sidebar and the window's buttons, the card takes its place through a resize, and ⌘S brings them back. The pointer at the window's edge brings the hidden sidebar out over the page and the pointer past it sends it away, and the sidebar button and ⌘S dock it from there. The sidebar's edge drags its width, rubber-bands past 160 and 400 and settles there, and a double-click puts it back. Scrolling: $timing. Windows: $out/fullscreen.png, $out/page-fullscreen.png, $out/sidebar-hidden.png, $out/sidebar-reveal.png, $out/sidebar-band.png, $out/sidebar-narrow.png"

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
  # Chrome tries https first only for public hosts.
  expect "^browser: page $tab back 1 forward 0 loading 1 http://stuck.test/\$" "stuck.test didn't show as loading"
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

# Two fingers across the sidebar page between Spaces. A flick goes on to the next Space, a drag let
# go still goes to the Space nearest its place, and past the last Space the pages come back. Fingers
# along the list scroll it, and a wheel's tick across pages once. Held partway, the next Space's
# page shows beside the first's.
swipe_point="110 500"
post swipe $swipe_point -10 0 10 1500
await '^browser: swipe across$' || fail "two fingers across the sidebar didn't page"
../tools/window-screenshot.sh "$pid" "$out/swipe.png" || fail "no window to capture"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
none_since '^browser: space [0-9]' || fail "a swipe let go short of halfway showed another Space"
# $1 is the driver's swipe, $2 the Space it shows and $3 that Space's tab.
swipe_to() {
  send swipe $swipe_point $1
  expect '^browser: command select-space via swipe$' "a swipe of $1 didn't page"
  expect "^browser: space $2 in " "a swipe of $1 didn't show Space $2"
  expect_tabs "$3" "$3" "after a swipe of $1 showed Space $2"
}
swipe_to "-12 0 10" "$second" "$b"
swipe_to "12 0 10" "$first" "$a"
swipe_to "-30 0 20 200" "$second" "$b"
send swipe $swipe_point -12 0 10
expect '^browser: swipe across$' "two fingers across the sidebar didn't page"
none_since '^browser: space [0-9]' || fail "a flick past the last Space showed another"
send swipe $swipe_point 1 -10 10
expect '^browser: swipe along$' "two fingers along the list didn't read as scrolling"
none_since '^browser: space [0-9]' || fail "two fingers along the list showed another Space"
send wheel $swipe_point 1
expect '^browser: command select-space via wheel$' "a wheel's tick across the sidebar didn't page"
expect "^browser: space $first in " "a wheel's tick across the sidebar didn't show the first Space"
expect_tabs "$a" "$a" "after a wheel's tick showed the first Space"

# A tab's menu moves it to another Space, and the window goes with the tab it shows.
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Duplicate Tab, Move to Space › reading 日本語, Close Tab, Close Tabs Below (disabled)$' "the tab's menu didn't offer the other Space"
send choose reading 日本語
expect '^browser: command move-to-space via tab menu$' "Move to Space didn't run"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "Move to Space didn't show the tab's new Space"
expect_tabs "$a $b" "$a" "after Move to Space"
expect "^browser: settled space $second\$" "the second Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Duplicate Tab, Move to Space › Space 1, Close Tab, Close Tabs Below$' "the moved tab's menu didn't offer the first Space"
send choose Space 1
expect "^browser: space $first\( in .*\)\{0,1\}\$" "moving the tab back didn't show the first Space"
expect_tabs "$a" "$a" "after moving the tab back"
show_space "key 2 ctrl" select-space ui "$second" "$b"
show_space "key 1 ctrl" select-space ui "$first" "$a"

send menu New Space…
expect "^browser: spaces $first $second [0-9]*\$" "New Space didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: space $third in " "the window didn't show the third Space"
send key return
none_since '^browser: create space' || fail "return with no name named the Space"
show_space "key 1 ctrl" select-space ui "$first" "$a"
send menu New Space…
expect "^browser: space $third in " "New Space didn't show the form already open"
# Clicks wait for the Space's page to slide into place.
expect "^browser: settled space $third\$" "the form's page didn't settle"
none_since '^browser: spaces ' || fail "New Space with a form open made another Space"
send state
in_browser AccessKitSubclassOfGPUIView "back in the third Space's form"
send click 114 $((${content#*x} - 74))
expect "^browser: cancel space $third\$" "a click on Cancel didn't cancel"
expect "^browser: space $first in " "Cancel didn't show the first Space again"
expect "^browser: spaces $first $second\$" "Cancel didn't delete the third Space"
expect_tabs "$a" "$a" "after Cancel"
expect "^browser: settled space $first\$" "the first Space's page didn't settle after Cancel"

# A right-click on the Space's name or its dot shows the Space's menu, as in Arc.
name_point="100 112"
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "a right-click on the Space's name didn't show its menu"
../tools/window-screenshot.sh "$pid" "$out/space-menu.png" || fail "no window to capture"
send choose Rename Space
expect "^browser: rename space $first\$" "Rename Space didn't open the name's field"
send state
in_browser AccessKitSubclassOfGPUIView "in the name's field"
send type work
send key return
expect "^browser: renamed space $first work\$" "return in the name's field didn't rename the Space"
send doubleclick $name_point
expect "^browser: rename space $first\$" "a double-click on the Space's name didn't open its field"
send type gone
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
none_since '^browser: renamed space' || fail "escape in the name's field renamed the Space"
# Deleting a Space with tabs asks first. Its tabs close with it, and ⇧⌘T reopens them here.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
post choose Delete Space
expect '^browser: alert shown Delete “reading 日本語”?$' "Delete Space didn't ask first"
../tools/window-screenshot.sh "$pid" "$out/space-delete.png" || fail "no window to capture"
send choose Delete
expect "^browser: spaces $first\$" "Delete didn't delete the Space"
[ "$(today)" = "$a" ] || fail "deleting another Space left this one's Today as \"$(today)\""
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T didn't reach the UI"
expect "^browser: today [0-9]* $a\$" "⇧⌘T didn't reopen the deleted Space's tab here: Today is \"$(today)\""
expect "^browser: loaded $(today | cut -d' ' -f1) https://example.com/\$" "the reopened tab didn't load its page"
# The last Space can't go.
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space (disabled)\$" \
  "the last Space's menu offered Delete"
send choose Rename Space
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
exec 3>&-
quit
spaces_ms=$(sed -n 's/^browser: space [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: New Space makes an empty Space whose form takes the keyboard and Japanese input, and names it, and ⌘T opens a tab in it; ⌃1 ⌃2 ⌥⌘← ⌥⌘→ with the page or the UI focused, and the switcher's dots, show each Space with the tab it showed last; two fingers across the sidebar page to the Space a flick or the pages' place points at, never past the last, and a wheel's tick across pages once; a tab's menu moves it to another Space, which shows with it; Cancel deletes a new Space and goes back. A right-click on a Space's name or dot shows Rename, its colours and Delete; a double-click on the name renames it too, return keeps the name and escape cancels. Delete asks first for a Space with tabs, closes them, and ⇧⌘T reopens them in the Space shown; the last Space can't go. Windows: $out/swipe.png, $out/space-menu.png, $out/space-delete.png"
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
post rightclick 100 112
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "the Space's menu didn't offer its colours"
send choose Pink
expect "^browser: colored space $second bf556a\$" "Pink didn't give the Space its colour"
../tools/window-screenshot.sh "$pid" "$out/space-color.png" || fail "no window to capture"
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
expect_tabs "$c $a" "$c" "back in the first Space"
send drag 226 400 298 400
expect '^browser: sidebar width 300$' "dragging the sidebar's edge didn't make it 300 wide"
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
grep -q "^browser: colored space $second bf556a\$" "$log" ||
  fail "the second Space came back without its colour"
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
check_layout "after the relaunch, with the sidebar it had" 300
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

# A SIGTERM ends the session at once, before the save the new tab scheduled, which the core writes
# as Chrome exits.
log="$out/session-ending.log"
BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch before the SIGTERM didn't restore the Spaces"
await "^browser: loaded [0-9]* $site\$" || fail "the page opened over the Spaces didn't load"
kill -TERM "$pid"
exited
[ "$(python3 -I -c 'import json, sys
print(*[len(space["today"]) for space in json.load(open(sys.argv[1]))["spaces"]])' "$saved")" = "2 2" ] ||
  fail "the tab opened before the SIGTERM wasn't kept"
restore_ms=$(sed -n 's/^browser: restored in \([0-9.]*\) ms$/\1/p' "$out/restored.log" "$out/restored-again.log" | tr '\n' ' ')
echo "ok: after ⌘Q and after closing the window, the Spaces come back with their names, colours, tabs, history and the tab each showed, in the Space shown last, with the sidebar's width; only tabs shown load, the others show their titles and favicons, and a closed tab stays closed. A SIGTERM keeps a tab opened just before it."
echo "  Restore, 3 tabs in 2 Spaces: ${restore_ms}ms. Window: $out/restored.png"

# Find in page. The test page has three apples and one 日本語.
page='data:text/html;charset=utf-8,<title>loading</title>
<p style="font:24px system-ui">apple banana apple cherry apple 日本語</p>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/find.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the find test page didn't draw"
first=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: find in page. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The find bar, a window of GPUI's over the browser window, has the keyboard.
in_find_bar() {
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "$1: the keyboard is in $key, on $focus"
}
# Waits for tab $1's find to be $2, as the core reports it: "closed", or "open", the selected
# match, "of", how many there are and the text.
expect_find() {
  expect "^browser: find $1 $2\$" "$3: $(grep "^browser: find $1 " "$log" | tail -1)"
}
# The find bar is $1 points wide and 44 high, $2 from the right of a window $3 wide and 20 from its
# top.
check_find_bar() {
  send layout
  child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
    sed 's/^browser: layout child //')
  content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    sed 's/^browser: layout content \([0-9]*\)x.*/\1/')
  [ "$child" = "$((content - 320)),20 300x44" ] ||
    fail "$1: the find bar is at ${child:-nowhere} in a window $content wide"
}

send key f cmd
expect '^browser: command find via page$' "⌘F with the page focused didn't reach the UI"
expect '^browser: find bar shown$' "⌘F didn't open the find bar"
send state
in_find_bar "after ⌘F"
send type apple
expect_find "$first" "open 1 of 3 apple" "typing in the find bar didn't find in the page"
send key return
expect_find "$first" "open 2 of 3 apple" "return didn't go to the next match"
send key return shift
expect_find "$first" "open 1 of 3 apple" "shift-return didn't go back a match"
send key g cmd shift
expect_find "$first" "open 3 of 3 apple" "⇧⌘G in the find bar didn't go back, round to the last"
send key g cmd
expect_find "$first" "open 1 of 3 apple" "⌘G in the find bar didn't go on, round to the first"
check_find_bar "after ⌘F"
../tools/window-screenshot.sh "$pid" "$out/find.png" || fail "no window to capture"

# A click on the page takes the keyboard and leaves the bar open, and ⌘G from the page goes on.
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the page under the find bar"
send key g cmd
expect '^browser: command find-next via page$' "⌘G with the page focused didn't reach the UI"
expect_find "$first" "open [1-3] of 3 apple" "⌘G with the page focused didn't find"
grep -q '^browser: find bar hidden$' "$log" && fail "a click on the page closed the find bar"

# ⌘F gives the open bar the keyboard. Japanese input composes in it, and it finds once committed.
send key f cmd
expect '^browser: find bar drew "apple" ' "⌘F with the bar open didn't keep its text"
send state
in_find_bar "after ⌘F with the find bar open"
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the find bar didn't take the composition: marked $marked"
send key space
[ "$marked" = 0,3 ] || fail "the find bar didn't take the conversion: marked $marked"
send key return
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
expect_find "$first" "open 1 of 1 日本語" "the find bar didn't find the committed text"
grep -q '^browser: find [0-9]* open [0-9-]* of [0-9-]* n' "$log" &&
  fail "the find bar searched for the composition"

# A resize and full screen keep the bar by the window's top right corner.
send resize 1000 700
check_find_bar "after a resize"
send fullscreen
expect '^browser: window entered full screen$' "the window didn't go full screen"
check_find_bar "in full screen"
../tools/window-screenshot.sh "$pid" "$out/find-fullscreen.png" || fail "no window to capture"
send fullscreen
expect '^browser: window left full screen$' "the window didn't leave full screen"

# Each tab has its own find. A new tab's starts from the text found last, and switching tabs shows
# each one's in the bar, or none.
send key t cmd
expect '^browser: command bar shown$' "⌘T in the find bar didn't open the command bar"
send type example.com
send key return
expect '^browser: find bar hidden$' "the find bar stayed over a new tab"
expect '^browser: loaded [0-9]* https://example.com/$' "the new tab didn't load"
second=$(today | cut -d' ' -f1)
send key f cmd
expect_find "$second" "open 0 of 0 日本語" "a new tab's find didn't start from the text found last"
send type example
expect_find "$second" "open 1 of [1-9][0-9]* example" "the new tab's find didn't find"
send key 2 cmd
expect "^browser: selected $first" "⌘2 in the find bar didn't show the first tab"
expect '^browser: find bar drew "日本語" 1 of 1$' "the first tab didn't show its own find"
# Escape closes the tab's find and gives the keyboard back to its page.
send key escape
expect_find "$first" "closed" "escape didn't close the find"
expect '^browser: find bar hidden$' "escape didn't hide the find bar"
send state
in_browser RenderWidgetHostViewCocoa "after escape closed the find bar"
send key 1 cmd
expect '^browser: find bar shown$' "the second tab didn't show its find"
expect '^browser: find bar drew "example" 1 of ' "the second tab didn't show its own find"
send state
[ "$key" = BrowserCoreNSWindow ] || fail "switching to a tab with its find open moved the keyboard to $key"
send key 2 cmd
expect '^browser: find bar hidden$' "the first tab, its find closed, showed the find bar"
# ⌘F opens it again with the tab's last text, highlighting its match without selecting it.
send key f cmd
expect '^browser: find bar drew "日本語" 0 of 1$' "⌘F didn't open the find with the tab's last text"

# Another page closes it.
send key l cmd
expect '^browser: command bar shown$' "⌘L in the find bar didn't open the command bar"
send type example.org
send key return
expect_find "$first" "closed" "going to another page didn't close the find"
expect '^browser: find bar hidden$' "going to another page didn't hide the find bar"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't quit"
exited
echo "ok: ⌘F opens the find bar from the page with the keyboard, typing finds with \"n of m\", return, shift-return, ⌘G and ⇧⌘G go through the matches, from the bar and the page, Japanese input finds once committed, the bar keeps its place through a resize and full screen, each tab keeps its own find, escape closes it and another page closes it. Windows: $out/find.png, $out/find-fullscreen.png"

# Pinned tabs, from the served page, whose favicon fades once its tab is unloaded.
log="$out/pinned.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the pinned tabs test page didn't draw"
a=$(today)
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "the served page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: pinned tabs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The pinned tabs the app last showed, top to bottom.
pinned() {
  grep '^browser: pinned' "$log" | tail -1 | sed 's/^browser: pinned *//'
}
# Waits for the app to show the pinned tabs $1 over Today $2. $3 says what went wrong if it doesn't.
expect_lists() {
  expect "^browser: pinned $1\$" "$3: the pinned tabs are \"$(pinned)\", not \"$1\""
  expect "^browser: today $2\$" "$3: Today is \"$(today)\", not \"$2\""
}
# The centre of pinned row $1, counting from 0, under the Space's name, and of the ✕ it shows
# under the pointer.
pinned_row() {
  echo "100 $((90 + 44 + 41 * $1 + 20))"
}
pinned_close() {
  echo "202 $((90 + 44 + 41 * $1 + 20))"
}
# Its favicon's.
pinned_favicon() {
  echo "26 $((90 + 44 + 41 * $1 + 20))"
}
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
expect_tabs "$c $b $a" "$c" "after opening two tabs"
send key d cmd
expect '^browser: command toggle-pin via page$' "⌘D with the page focused didn't reach the UI"
expect_lists "$c" "$b $a" "after ⌘D from the page"
switch "3 cmd" "$a"
send click $sidebar_point
send key d cmd
expect '^browser: command toggle-pin via ui$' "⌘D with the UI focused didn't run"
expect_lists "$c $a" "$b" "after ⌘D from the UI"
switch "1 cmd" "$c"
switch "3 cmd" "$b"

# Dragging a row moves its tab, in the pinned tabs and between them and Today, without showing it.
# Today's top row is under the pinned rows, the divider and "+ New Tab".
today_top="100 $((90 + 44 + 41 * 2 + 15 + 41 + 20))"
send drag $(pinned_row 1) 100 150
expect "^browser: dragged $a to pinned 0\$" "a drag of the second pinned tab up didn't move it"
expect_lists "$a $c" "$b" "after dragging a pinned tab up"
none_since '^browser: selected' || fail "dragging a tab showed it"
# In steps, as the pointer goes: Today's tab comes away and the pinned rows make room for it.
send press $today_top
send dragto 100 270
send dragto 100 230
send dragto 100 200
../tools/window-screenshot.sh "$pid" "$out/drag.png" || fail "no window to capture"
none_since '^browser: dragged' || fail "a row dropped before the pointer let go"
send release 100 200
expect "^browser: dragged $b to pinned 1\$" "a drag of the Today tab up didn't pin it"
expect_lists "$a $b $c" " *" "after dragging the Today tab into the pinned tabs"
send drag $(pinned_row 1) 100 300
expect "^browser: dragged $b to today 0\$" "a drag of a pinned tab down didn't unpin it"
expect_lists "$a $c" "$b" "after dragging a pinned tab into Today"
send drag $(pinned_row 0) 100 200
expect_lists "$c $a" "$b" "after dragging the first pinned tab down"
none_since '^browser: selected' || fail "dragging a tab showed it"
# Escape puts a dragged row back.
send press $(pinned_row 0)
send dragto 100 250
send key escape
expect '^browser: command stop via ' "escape during a drag didn't reach the UI"
send release 100 250
none_since '^browser: dragged' || fail "a drag escape put back dropped its row"
none_since '^browser: selected' || fail "letting go of a drag escape put back showed its tab"
[ "$(pinned)" = "$c $a" ] || fail "a drag escape put back moved a tab: the pinned tabs are \"$(pinned)\""

# ⌘W on a pinned tab unloads it and shows the top Today tab, and showing it again loads it.
switch "2 cmd" "$a"
send key w cmd
expect "^browser: selected $b\$" "⌘W on a pinned tab didn't show the top Today tab"
expect "^browser: page $a unloaded 1\$" "⌘W on a pinned tab didn't unload it"
none_since "^browser: tab $a closed\$" || fail "⌘W closed a pinned tab"
[ "$(pinned)" = "$c $a" ] || fail "⌘W on a pinned tab took its row: the pinned tabs are \"$(pinned)\""
../tools/window-screenshot.sh "$pid" "$out/pinned.png" || fail "no window to capture"
send click $(pinned_row 1)
expect "^browser: selected $a in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $a $site\$" "the unloaded pinned tab didn't load once shown"
expect "^browser: page $a unloaded 0\$" "the pinned tab still read as unloaded once it loaded"

# The ✕ under the pointer unloads a pinned tab, and an unloaded one has none.
send move $(pinned_close 0)
send click $(pinned_close 0)
expect '^browser: command close-tab via sidebar$' "a click on a pinned tab's ✕ didn't run Close Tab"
expect "^browser: page $c unloaded 1\$" "the ✕ didn't unload the pinned tab"
none_since '^browser: selected' || fail "unloading a pinned tab not shown changed the tab shown"
send click $(pinned_close 0)
expect "^browser: selected $c in " "a click where an unloaded pinned tab's ✕ was didn't show the tab"
none_since '^browser: command close-tab' || fail "an unloaded pinned tab showed its ✕"
expect "^browser: loaded $c https://example.org/\$" "the pinned tab didn't load once shown"

# Without Today tabs, closing shows the nearest pinned tab still loaded, else no page.
switch "3 cmd" "$b"
send key w cmd
expect "^browser: tab $b closed\$" "⌘W didn't close the last Today tab"
expect_lists "$c $a" " *" "after closing the last Today tab"
expect "^browser: selected $a\$" "closing the last Today tab didn't show the lowest pinned tab still loaded"
send key w cmd
expect "^browser: selected $c\$" "⌘W on a pinned tab without Today tabs didn't show the other one loaded"
expect "^browser: page $a unloaded 1\$" "⌘W didn't unload the pinned tab"
send key w cmd
expect '^browser: selected 0$' "⌘W on the last pinned tab loaded didn't leave the window without a page"
expect "^browser: page $c unloaded 1\$" "⌘W didn't unload the last pinned tab loaded"
none_since '^browser: loaded' || fail "closing a pinned tab loaded another"
../tools/window-screenshot.sh "$pid" "$out/pinned-unloaded.png" || fail "no window to capture"

# ⌘D unpins a tab to the top of Today.
send click $(pinned_row 1)
expect "^browser: selected $a in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $a $site\$" "the unloaded pinned tab didn't load once shown"
send key d cmd
expect_lists "$c" "$a" "after ⌘D on a pinned tab"

# Away from its pinned URL, ⌘↩ goes back there, from the page and the UI.
send click $(pinned_row 0)
expect "^browser: selected $c in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $c https://example.org/\$" "the unloaded pinned tab didn't load once shown"
# Away from it, then back with ⌘↩ from side $1, which a click gives the keyboard.
away_and_back() {
  go_from_bar example.com
  expect "^browser: page $c away 1\$" "a pinned tab at another page didn't read as away from its pinned URL"
  if [ "$1" = ui ]; then
    send click $sidebar_point
  else
    send click $page_point
  fi
  send key return cmd
  expect "^browser: command back-to-pinned-url via $1\$" "⌘↩ with the $1 focused didn't run Back to Pinned URL"
  expect "^browser: loaded $c https://example.org/\$" "⌘↩ with the $1 focused didn't load the pinned URL"
  expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
}
away_and_back page
away_and_back ui
go_from_bar example.com
expect "^browser: page $c away 1\$" "a pinned tab at another page didn't read as away from its pinned URL"
# The quit leaves Today's tab shown, so the pinned tab comes back unloaded.
switch "2 cmd" "$a"
# Its favicon under the pointer offers the way back.
send move $(pinned_favicon 0)
../tools/window-screenshot.sh "$pid" "$out/pinned-away.png" || fail "no window to capture"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the pinned tab comes back pinned, at its page, unloaded, and away from its pinned
# URL. A click on its favicon goes back there and shows it.
log="$out/pinned-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: pinned [0-9][0-9]*$' || fail "the pinned tab didn't come back pinned"
await '^browser: today [0-9][0-9]*$' || fail "the Today tab didn't come back"
c=$(pinned)
a=$(today)
await "^browser: selected $a\$" || fail "the relaunch didn't show the tab shown last"
await "^browser: loaded $a $site\$" || fail "the tab shown didn't load"
await "^browser: page $c back 1 forward 0 loading 0 https://example.com/\$" ||
  fail "the pinned tab didn't come back at its page: $(grep "^browser: page $c" "$log" | tail -1)"
await "^browser: page $c unloaded 1\$" || fail "the pinned tab didn't come back unloaded"
await "^browser: page $c away 1\$" || fail "the pinned tab didn't come back away from its pinned URL"
grep -q "^browser: loaded $c " "$log" && fail "the pinned tab loaded without being shown"
../tools/window-screenshot.sh "$pid" "$out/pinned-restored.png" || fail "no window to capture"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored pinned tab's way back. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
send move $(pinned_favicon 0)
send click $(pinned_favicon 0)
expect '^browser: command back-to-pinned-url via sidebar$' "a click on an away pinned tab's favicon didn't run Back to Pinned URL"
expect "^browser: selected $c in " "a click on the pinned tab's favicon didn't show it"
expect "^browser: loaded $c https://example.org/\$" "a click on the pinned tab's favicon didn't load its pinned URL"
expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
none_since "^browser: loaded $c https://example.com/" || fail "the unloaded tab loaded its old page on the way back"
exec 3>&-
quit
echo "ok: ⌘D pins and unpins from the page and the UI, and ⌘1–⌘9 count pinned tabs first. Dragging a row moves its tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back. ⌘W and ✕ unload a pinned tab and keep its row, showing the top Today tab, else a pinned tab still loaded, else no page, and an unloaded pinned tab loads once shown. Away from its pinned URL, ⌘↩ from the page and the UI, and a click on its favicon, go back there. Pinned tabs come back pinned, unloaded and at their page, still away. Windows: $out/drag.png, $out/pinned.png, $out/pinned-unloaded.png, $out/pinned-away.png, $out/pinned-restored.png"

# Clear, from the served page and two more, with the served page pinned for it to leave.
log="$out/clear.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Clear test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: Clear. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Clear on the divider, which sits under the one pinned tab, and the toast's Undo, over the bottom
# of the list in the 800 pt tall window.
clear_point="200 185"
undo_point="192 732"
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
switch "3 cmd" "$a"
send key d cmd
expect_lists "$a" "$c $b" "after pinning the served page's tab"
switch "2 cmd" "$c"

# ⇧⌘K with the page focused closes the Today tabs and shows the pinned tab, without loading it.
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: toast cleared 2$' "Clear didn't show its toast for two tabs"
expect "^browser: tab $c closed\$" "Clear didn't close the tab shown"
expect "^browser: tab $b closed\$" "Clear didn't close the other Today tab"
expect_lists "$a" " *" "after Clear"
expect "^browser: selected $a\$" "Clear didn't show the pinned tab still loaded"
none_since '^browser: loaded' || fail "Clear loaded a page"
../tools/window-screenshot.sh "$pid" "$out/clear.png" || fail "no window to capture"

# Undo brings them back in place, showing the one shown and leaving the other unloaded.
send click $undo_point
expect '^browser: command undo-clear via sidebar$' "a click on the toast's Undo didn't run it"
expect '^browser: toast hidden$' "Undo left the toast"
expect "^browser: today [0-9][0-9]* [0-9][0-9]*\$" "Undo didn't bring back two Today tabs: Today is \"$(today)\""
c=$(today | cut -d' ' -f1)
b=$(today | cut -d' ' -f2)
expect "^browser: selected $c\$" "Undo didn't show the tab Clear closed shown"
expect "^browser: loaded $c https://example.org/\$" "the tab Undo showed didn't load at its page"
expect "^browser: page $b back 0 forward 0 loading 0 https://example.com/\$" "the other tab didn't come back at its page"
expect "^browser: page $b unloaded 1\$" "the tab Undo didn't show came back loaded"
none_since "^browser: loaded $b " || fail "the tab Undo didn't show loaded"

# The divider's Clear, and the toast hides by itself.
send click $clear_point
expect '^browser: command clear via sidebar$' "a click on Clear didn't run it"
expect '^browser: toast cleared 2$' "Clear on the divider didn't show its toast"
expect_lists "$a" " *" "after Clear on the divider"
expect '^browser: toast hidden$' "the toast didn't hide by itself"

# ⇧⌘K with the UI focused, then with no Today tab left to clear.
open_from_bar example.com
d=$tab
send click $sidebar_point
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K with the UI focused didn't run"
expect "^browser: tab $d closed\$" "⇧⌘K with the UI focused didn't close the Today tab"
expect "^browser: selected $a\$" "⇧⌘K with the UI focused didn't show the pinned tab"
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K without Today tabs didn't run"
none_since '^browser: toast cleared' || fail "Clear without Today tabs showed a toast"

# ⇧⌘T with the UI focused reopens the tab closed last and shows it.
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T with the UI focused didn't reach the UI"
expect "^browser: today [0-9][0-9]*\$" "⇧⌘T didn't reopen the tab Clear closed: Today is \"$(today)\""
d=$(today)
expect "^browser: selected $d\$" "⇧⌘T didn't show the tab it reopened"
expect "^browser: loaded $d https://example.com/\$" "the reopened tab didn't load its page"

# ⇧⌘T with the page focused puts a tab back where it was, with its history.
open_from_bar example.net
e=$tab
go_from_bar example.org
expect_page example.org 1 0 "after going on from example.net"
open_from_bar example.com
f=$tab
switch "3 cmd" "$e"
send click $page_point
send key w cmd
expect "^browser: tab $e closed\$" "⌘W didn't close the middle Today tab"
expect_lists "$a" "$f $d" "after closing the middle Today tab"
send click $page_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via page$' "⇧⌘T with the page focused didn't reach the UI"
expect_lists "$a" "$f [0-9][0-9]* $d" "after ⇧⌘T, the tab isn't back between the two it was between"
e=$(today | cut -d' ' -f2)
tab=$e
expect "^browser: selected $e\$" "⇧⌘T didn't show the tab it reopened"
expect_page example.org 1 0 "after ⇧⌘T"
../tools/window-screenshot.sh "$pid" "$out/reopen.png" || fail "no window to capture"

# After Clear, ⇧⌘T reopens the tab it closed last, and Undo brings back only the others.
send click $page_point
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: toast cleared 3$' "Clear didn't show its toast for three tabs"
expect_lists "$a" " *" "after clearing three tabs"
send click $sidebar_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T after Clear didn't reach the UI"
expect "^browser: today [0-9][0-9]*\$" "⇧⌘T after Clear didn't reopen one tab: Today is \"$(today)\""
d=$(today)
send click $undo_point
expect '^browser: command undo-clear via sidebar$' "a click on the toast's Undo didn't run it"
expect "^browser: today [0-9][0-9]* [0-9][0-9]* $d\$" "Undo after ⇧⌘T didn't bring back just the other two: Today is \"$(today)\""
e=$(today | cut -d' ' -f2)
expect "^browser: selected $e\$" "Undo didn't show the tab Clear closed shown"

# A right-click on a row shows its tab's menu, with the shortcuts. Duplicate Tab opens a copy under
# it, with its history, and shows it. Rows count from the one pinned tab.
d=$(today | cut -d' ' -f3)
t=$(today | cut -d' ' -f1)
post rightclick $(row 2)
expect '^browser: menu shown Pin Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on a Today row didn't show its tab's menu"
../tools/window-screenshot.sh "$pid" "$out/tab-menu.png" || fail "no window to capture"
send choose Duplicate Tab
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect_lists "$a" "$t $e [0-9][0-9]* $d" "after Duplicate Tab"
tab=$(today | cut -d' ' -f3)
expect "^browser: selected $tab\$" "Duplicate Tab didn't show the copy"
expect_page example.org 1 0 "the copy Duplicate Tab made"
# Close Tabs Below shows the tab right-clicked first, so none below it loads on the way out.
post rightclick $(row 1)
expect '^browser: menu shown Pin Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on the top Today row didn't show its tab's menu"
send choose Close Tabs Below
expect '^browser: command close-tabs-below via tab menu$' "Close Tabs Below didn't run"
expect_lists "$a" "$t" "after Close Tabs Below"
expect "^browser: tab $e closed\$" "Close Tabs Below didn't close the tab under the one right-clicked"
expect "^browser: tab $d closed\$" "Close Tabs Below didn't close the bottom tab"
expect "^browser: selected $t\$" "Close Tabs Below didn't show the tab right-clicked"
[ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: selected ')" = 1 ] ||
  fail "Close Tabs Below showed a tab on the way out"
# ⇧⌘T brings them back one at a time, each at its place.
send key t cmd shift
expect "^browser: today $t [0-9][0-9]*\$" "⇧⌘T after Close Tabs Below didn't reopen the tab under the one right-clicked: Today is \"$(today)\""
# A pinned tab's menu unpins it, and goes back to its pinned URL only when it's away from it.
post rightclick $(pinned_row 0)
expect '^browser: menu shown Unpin Tab, Back to Pinned URL (disabled), Duplicate Tab, Move to Space (disabled), Close Tab$' "a right-click on a pinned row didn't show its tab's menu"
send choose Unpin Tab
expect '^browser: command toggle-pin via tab menu$' "Unpin Tab didn't run"
expect "^browser: today $a $t [0-9][0-9]*\$" "Unpin Tab didn't move the tab to the top of Today: Today is \"$(today)\""
exec 3>&-
quit
kill "$server"
echo "ok: ⇧⌘K from the page and the UI, and Clear on the divider, close the Today tabs and show the pinned tab still loaded, with a toast that hides by itself. Undo brings the tabs back in place, showing the one shown and leaving the other unloaded. ⇧⌘T from the page and the UI reopens the tab closed last, where it was and with its history, and Undo brings back only the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu: Duplicate Tab copies the tab with its history under it, Close Tabs Below closes the Today tabs under it without showing them, and Unpin Tab unpins. Windows: $out/clear.png, $out/reopen.png, $out/tab-menu.png"
