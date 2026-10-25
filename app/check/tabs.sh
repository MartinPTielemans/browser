# A tab opened from the command bar goes at the top of the Space's Today tabs and shows. The command
# bar suggests pages from history as it's typed in, growing to fit them, with the tabs already at
# them, never the one shown, and well-known sites under them. ↑, ↓ and ⌃N choose a row, wrapping at
# the ends, and ↩ or a click on a tab's row shows that tab without opening one. Typing at the end
# completes the text to the first row's page, which ⌫ takes back, → keeps and ↩ goes to. ⌘1–⌘9, ⌥⌘↓,
# ⌥⌘↑ and clicks on rows show tabs. ⌃Tab, with the page or the UI focused, shows the tab shown
# before, and with ⌃ held goes on back through the tabs in the order they were shown, and ⌃⇧Tab the
# other way, until ⌃ goes up or a click. A tab a page opens goes under it and closing it goes back
# there, and ⌘W otherwise shows the tab below, or above at the bottom. Closing the last tab leaves
# the Space empty, where ⌘T still works. There ⌘↩ opens a row's page behind and the bar stays open,
# ⇧⌦ takes a page out of the rows, ⇧⌘↩ searches for what was typed and ⌃↩ goes to www.<word>.com.
# Reports how long switches take.

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
# ⌘↩ opens a row's page in a tab behind, at the top of Today, and the bar stays open, as Arc's
# does. ⇧⌦ takes a page from history out of the rows.
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
send key return cmd
expect '^browser: command bar behind https://example.org/$' \
  "⌘↩ didn't open example.org's row behind"
expect '^browser: today [0-9][0-9]*$' "⌘↩ didn't open a tab in Today: Today is \"$(today)\""
none_since '^browser: command bar hidden$' || fail "⌘↩ closed the command bar"
none_since '^browser: selected [1-9]' || fail "⌘↩ showed the tab it opened"
send key delete shift
expect '^browser: command bar removes https://example.org/$' "⇧⌦ didn't take example.org's row out"
expect '^browser: command bar suggestions "example" in ' \
  "the command bar didn't list its rows again after ⇧⌦"
suggestions | grep -q '^https://example.org/' && fail "example.org's row stayed after ⇧⌦"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
# ⇧⌘↩ searches for what was typed even when it names a page, and ⌃↩ goes to www.<word>.com, as
# Chrome's does.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.com
send key return shift cmd
expect '^browser: command bar go https://duckduckgo.com/?q=example.com$' \
  "⇧⌘↩ didn't search for what was typed: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "⇧⌘↩ didn't close the command bar"
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example
expect '^browser: command bar completes "example" with "\.' \
  "the command bar didn't complete \"example\""
send key return ctrl
expect '^browser: command bar go https*://www.example.com/$' \
  "⌃↩ didn't go to www.example.com: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "⌃↩ didn't close the command bar"
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
echo "ok: new tabs open at the top and show, the command bar suggests pages, their tabs and well-known sites and completes what's typed, keys and clicks switch, ⌘↩ opens a row behind and stays open, ⇧⌦ removes one, ⇧⌘↩ searches and ⌃↩ adds www. and .com, ⌃Tab goes back through the tabs shown while ⌃ is held, a page's tab goes under it and closing it goes back, ⌘W shows the tab below or above, an empty Space still opens tabs."
echo "  Switch to a loaded tab, tab strip to frame shown (Chrome's measure; gate 16 ms): $chrome_ms."
echo "  Switch, key to the core's report: $app_ms. Windows: $out/tabs.png, $out/suggestions.png, $out/completion.png, $out/popular.png, $out/empty.png"
