# A tab opened from the command bar goes at the top of the Space's Today tabs and shows. The command
# bar lists the Space's other tabs before anything is typed, those shown last first, and suggests
# pages from history as it's typed in, growing to fit them, with the tabs already at them, never
# the one shown, and well-known sites under them. ↩ or a click on a tab's row shows that
# tab without opening one. Typing at the end completes the text to the first row's page, and ↩ goes
# there. ⌘1–⌘9, ⌥⌘↓, ⌥⌘↑ and clicks on rows show tabs. ⌃Tab, with the page or the UI focused, shows
# the tab shown before, and with ⌃ held goes on back through the tabs in the order they were shown,
# and ⌃⇧Tab the other way, until ⌃ goes up or a click. A tab a page opens goes under it and closing
# it goes back there, and ⌘W otherwise shows the tab below, or above at the bottom. Closing the last
# tab leaves the Space empty, where ⌘T still works. Mute/Unmute Tab mutes a tab, its row shows a
# speaker while it plays, crossed out while it's muted, and a click on it unmutes the tab. There ⇧⌦
# takes a page out of history, ⇧⌘↩ searches for what was typed, ⌘↩ opens it in a tab behind that
# loads once it's first shown, Tab on the row that hints a search engine, or a space after its
# keyword, searches with that engine alone until ⌫ leaves, ↩ on a sum's row copies its result, and a
# right-click in its field shows the field's menu, with Paste and Go. The command bar's tests, in
# src/command_bar/tests.rs, have the rest of its keys and what the menu's items do. Reports how long
# switches take.

# A Space's Today tabs. The page opens a tab of its own from its button, the way a link to a new tab
# does, and p in it plays a quiet tone until x. The app asks Chrome to trace its tab switches, timed
# as Chrome times its own: from the tab strip's change to the first frame shown. The trace is
# written as the app quits. In a handler attribute a bare open() is document.open(), which would
# blank the page instead.
page='data:text/html,<title>loading</title>
<button onclick=window.open() style="font:24px system-ui">Open a tab</button>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
let sound;
onkeydown = e => {
  if (e.key == "p") {
    sound = new AudioContext();
    const tone = sound.createOscillator();
    const gain = sound.createGain();
    gain.gain.value = 0.002;
    tone.connect(gain).connect(sound.destination);
    tone.start();
    document.title = "playing";
  }
  if (e.key == "x") sound.close();
};
</script>'
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

await '^browser: today [0-9][0-9]*$' || fail "the first tab isn't in Today"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: tabs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Mute/Unmute Tab mutes the tab, and its row shows the speaker crossed out. The page plays only
# once it's muted, so the run makes no sound, and Chrome counts its sound for 2 s after it stops.
# A click on the speaker unmutes the tab, and the speaker goes.
bar_command toggle-mute "Mute/Unmute Tab" mute
expect "^browser: page $a audible 0 muted 1\$" "Mute/Unmute Tab didn't mute the tab"
send click $page_point
send key p
expect "^browser: title $a playing\$" "p in the page didn't play its sound"
expect "^browser: page $a audible 1 muted 1\$" "the muted tab's sound didn't show"
../tools/window-screenshot.sh "$pid" "$out/muted.png" || fail "no window to capture"
send key x
expect "^browser: page $a audible 0 muted 1\$" "the tab's sound didn't stop"
# The speaker is after the favicon, at the title's x, 43.
send click 52 $(row 0 | cut -d' ' -f2)
expect '^browser: command toggle-mute via sidebar$' "a click on the speaker didn't reach the UI"
expect "^browser: page $a audible 0 muted 0\$" "a click on the speaker didn't unmute the tab"
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
expect_tabs "$c $b $a" "$c" "after opening two tabs"
../tools/window-screenshot.sh "$pid" "$out/tabs.png" || fail "no window to capture"

# The command bar's last suggestions, as "<url>[ tab <id>][ site <keyword>]" a line.
suggestions() {
  grep '^browser: command bar suggestions' "$log" | tail -1 | sed 's/^[^:]*: [^:]* ms: //' |
    tr -s ', ' '\n ' | sed 's/^ //'
}
# The command bar's height logged last.
bar_height() {
  grep '^browser: command bar height' "$log" | tail -1 | sed 's/.* //'
}
# Waits for the bar to take its last suggestions' height, which the host sets after the UI logs
# them, so a capture then shows the rows drawn. It may have been as many rows before. Arc's panel:
# the input 64, its line and the gap under it 4.5, 50 a row up to five, 10 below.
await_bar_height() {
  rows=$(suggestions | wc -l | tr -d ' ')
  want=$(awk -v rows="$rows" 'BEGIN { if (rows > 5) rows = 5; print 78.5 + 50 * rows }')
  for _ in $(seq 50); do
    [ "$(bar_height)" = "$want" ] && return
    sleep 0.1
  done
  fail "the command bar is $(bar_height) tall, not $want for $rows rows"
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
  await_bar_height
  none_since "https://$2/ tab " || fail "the command bar suggested $2's tab, which is shown"
}
suggest_example example.com example.org
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
# Rows are 50 tall, under the 64 of the input, its line and a gap of 4. The bar shows five.
row=$(suggestions | grep -n "^https://example.org/ tab $c\$" | cut -d: -f1)
[ "$row" -le 5 ] || fail "example.org's row is the ${row}th, below the five the bar shows"
send clickkey 320 $((68 + 50 * (row - 1) + 25))
expect "^browser: command bar switch to tab $c\$" "a click on example.org's row didn't switch to its tab"
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "a click on example.org's row didn't show its tab"
[ "$(today)" = "$c $b $a" ] || fail "switching to a tab changed Today to \"$(today)\""

# Typing at the end completes the text to the first row's page, as Chrome's address bar does:
# example.com, typed before, completes "example.c", and ↩ goes where it goes.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.c
expect '^browser: command bar completes "example.c" with "om"$' "the command bar didn't complete \"example.c\" to example.com"
expect '^browser: command bar drew "example.com"$' "the command bar didn't show the completion"
../tools/window-screenshot.sh "$pid" "$out/completion.png" || fail "no window to capture"
send key return
expect '^browser: command bar go https://example.com/$' "↩ didn't go where the completion goes: $(grep '^browser: command bar go' "$log" | tail -1)"
expect "^browser: loaded $c https://example.com/\$" "↩ didn't load the completion's page in the tab"
[ "$(today)" = "$c $b $a" ] || fail "going to a page in a tab changed Today to \"$(today)\""
# A click on the URL field opens the bar on the tab's URL, as Arc's ⌘L does: the same panel as ⌘T's,
# 766 wide and centred.
send click 100 64
expect '^browser: command edit-url via sidebar$' \
  "a click on the URL field didn't open the command bar"
expect '^browser: command bar shown$' "a click on the URL field didn't open the command bar"
expect '^browser: command bar drew "https://example.com/"$' \
  "the command bar from the URL field didn't show the tab's URL"
at=$(grep '^browser: command bar at' "$log" | tail -1 | cut -d' ' -f5-)
# The input alone is 64 tall, centred on the window.
[ "$(echo "$at" | awk '{ print $3, $4 }')" = "766 64" ] ||
  fail "the command bar from the URL field opened at $at, not 766 by 64"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

# Before anything is typed, the bar lists the Space's other tabs, as Arc's does: those shown most
# recently first, then the rest, such as favorites never shown.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
listed=$(grep '^browser: command bar tabs' "$log" | tail -1 | sed 's/^[^:]*: command bar tabs *//')
case "$listed" in
  "$b $a" | "$b $a "*) ;;
  *) fail "the command bar listed tabs \"$listed\" before anything was typed, not $b then $a" ;;
esac
../tools/window-screenshot.sh "$pid" "$out/listed.png" || fail "no window to capture"
# Well-known sites fill the rows under the profile's own, never first, so ↩ still searches.
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
# ⌃Tab goes back through the tabs in the order they were shown, c, a and b by now, as far as ⌃
# stays down. Then the tab it showed counts as shown last.
send click $page_point
switch "tab ctrl" "$a"
expect '^browser: command recent-tab via page$' "⌃Tab with the page focused didn't reach the UI"
switch "tab ctrl" "$b"
send flags
switch "tab ctrl" "$c"
send flags
switch "tab ctrl shift" "$a"
send flags
send click $sidebar_point
switch "tab ctrl" "$c"
expect '^browser: command recent-tab via ui$' "⌃Tab with the UI focused didn't reach the UI"
send flags
# A click ends the walk too.
switch "tab ctrl" "$a"
send click $(row 1)
expect "^browser: selected $b in " "a click on the second row didn't show its tab"
switch "tab ctrl shift" "$c"
send flags
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
# ⇧⌦ takes a page from history out of the rows.
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in an empty Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in an empty Space didn't open the command bar"
send type example
expect '^browser: command bar completes "example" with "\.' \
  "the command bar didn't complete \"example\""
send key backspace
expect '^browser: command bar suggestions "example" in .*?q=example, .*https://example.org/' \
  "the command bar didn't suggest example.org: $(suggestions | tr '\n' ' ')"
for _ in 1 2 3 4 5 6 7 8; do
  send key down
  grep '^browser: command bar selected' "$log" | tail -1 | grep -q ' https://example.org/$' && break
done
send key delete shift
expect '^browser: command bar removes https://example.org/$' "⇧⌦ didn't take example.org's row out"
expect '^browser: command bar suggestions "example" in ' \
  "the command bar didn't list its rows again after ⇧⌦"
suggestions | grep -q '^https://example.org/' && fail "example.org's row stayed after ⇧⌦"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
# ⇧⌘↩ searches for what was typed even when it names a page, as Chrome's does.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.com
send key return shift cmd
expect '^browser: command bar go https://duckduckgo.com/?q=example.com$' \
  "⇧⌘↩ didn't search for what was typed: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "⇧⌘↩ didn't close the command bar"
# Site search, as Chrome's keyword mode: Tab on the row that hints the default engine searches
# with it alone, ⌫ in its empty field leaves it, keeping the keyword and a space, and a space typed
# after the keyword enters it again.
listed='^browser: command bar suggestions "duckduckgo.com'
hint="[0-9.]* ms: [^,]* site duckduckgo.com"
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type duckduckgo.com
expect "$listed\" in $hint" \
  "the command bar's first row didn't hint DuckDuckGo's site search: $(suggestions | tr '\n' ' ')"
await_bar_height
../tools/window-screenshot.sh "$pid" "$out/site-hint.png" || fail "no window to capture"
send key tab
expect '^browser: command bar site search duckduckgo.com by tab$' \
  "Tab on the row that hints DuckDuckGo didn't enter its site search"
send type cats
expect "$listed cats\" in [0-9.]* ms: https://duckduckgo.com/?q=cats" \
  "the site search didn't list DuckDuckGo's search first: $(suggestions | tr '\n' ' ')"
../tools/window-screenshot.sh "$pid" "$out/site-search.png" || fail "no window to capture"
for _ in 1 2 3 4 5; do send key backspace; done
expect '^browser: command bar leaves site search$' \
  "⌫ in the site search's empty field didn't leave it"
expect '^browser: command bar drew "duckduckgo.com "$' \
  "leaving the site search didn't put the keyword and the space back"
send key backspace
expect "$listed\" in $hint" \
  "the command bar didn't hint DuckDuckGo's site search again: $(suggestions | tr '\n' ' ')"
send type ' dogs'
expect '^browser: command bar site search duckduckgo.com by space$' \
  "a space after DuckDuckGo's keyword didn't enter its site search"
expect "$listed dogs\" in " "the site search didn't list DuckDuckGo's searches for what was typed"
send key return
went=$(grep '^browser: command bar go' "$log" | tail -1)
expect '^browser: command bar go https://duckduckgo.com/?q=dogs' \
  "↩ in the site search didn't search DuckDuckGo: $went"
expect '^browser: command bar hidden$' "↩ in the site search didn't close the command bar"
# A sum's result is the row after the first, and ↩ on it copies the result, to a pasteboard of the
# run's own.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type 12+7
expect '^browser: command bar calculates "12+7": 19$' "the command bar didn't work out 12+7"
send key down
expect '^browser: command bar selected = 19$' "↓ didn't select the sum's row"
../tools/window-screenshot.sh "$pid" "$out/sum.png" || fail "no window to capture"
send key return
expect '^browser: command bar copies 19$' "↩ on the sum's row didn't copy its result"
expect '^browser: copied 19$' "the sum's result wasn't copied"
expect '^browser: command bar hidden$' "copying the sum's result didn't close the command bar"
# A right-click in the command bar's field shows its menu, with Paste and Go and Paste and Search.
# The check chooses only Select All: Cut, Copy and the Paste items use the owner's pasteboard.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type 8-3
expect '^browser: command bar calculates "8-3": 5$' "the command bar didn't work out 8-3"
post rightclickkey 200 27
pastes='Paste, Paste and Go, Paste and Search'
expect "^browser: menu shown Cut (disabled), Copy (disabled), $pastes, Select All\$" \
  "a right-click in the command bar's field didn't show its menu"
../tools/window-screenshot.sh "$pid" "$out/field-menu.png" || fail "no window to capture"
send choose Select All
send type 2+2
expect '^browser: command bar calculates "2+2": 4$' \
  "Select All in the field's menu didn't select its text for typing to replace"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar after its menu"
# ⌘↩ opens what was typed in a tab behind, at the top of Today, which loads only once it's first
# shown (Dia), and the bar stays open.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
before=$(today)
send type example.org
send key return cmd
# The bar may go over http, from history, as the https page was taken out of it above.
expect '^browser: command bar behind https\{0,1\}://example.org/$' \
  "⌘↩ didn't open example.org behind"
expect "^browser: today [0-9][0-9]* $before\$" "⌘↩'s tab didn't open at the top of Today"
waiting=$(today | cut -d' ' -f1)
expect "^browser: page $waiting unloaded 1\$" "⌘↩'s tab didn't wait to load"
none_since '^browser: command bar hidden$' || fail "⌘↩ closed the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar after ⌘↩"
grep -q "^browser: loaded $waiting " "$log" && fail "⌘↩'s tab loaded before it was shown"
switch "1 cmd" "$waiting"
expect "^browser: loaded $waiting https\{0,1\}://example.org/\$" "⌘↩'s tab didn't load once shown"
exec 3>&-
# Chrome writes the trace before it quits, once every process has started tracing or 10 s have
# passed, so a renderer that closed early can hold the quit that long.
quit 30

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
echo "ok: new tabs open at the top and show, the command bar lists the Space's other tabs before anything is typed, suggests pages, their tabs and well-known sites and completes what's typed, keys and clicks switch, ⇧⌦ takes a page out of history, ⇧⌘↩ searches and ⌘↩ opens a tab behind that loads once shown, Tab on a hinted row and a space after an engine's keyword search with it alone until ⌫ leaves, ↩ on a sum's row copies its result, and a right-click in its field shows Paste and Go, ⌃Tab goes back through the tabs shown while ⌃ is held, a page's tab goes under it and closing it goes back, ⌘W shows the tab below or above, an empty Space still opens tabs, a tab mutes and its speaker unmutes it."
echo "  Switch to a loaded tab, tab strip to frame shown (Chrome's measure; gate 16 ms): $chrome_ms."
echo "  Switch, key to the core's report: $app_ms. Windows: $out/muted.png, $out/tabs.png, $out/suggestions.png, $out/completion.png, $out/listed.png, $out/popular.png, $out/site-hint.png, $out/site-search.png, $out/field-menu.png, $out/empty.png"
