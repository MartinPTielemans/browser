# ⌘N opens another window on the same Spaces, showing the key window's Space with no page, whose
# card shows its one line, and a click on the card with ⌘T's bar open only closes the bar. It never
# shows another window's page: a Space it switches to and back shows the first tab no window shows.
# The Spaces menu acts on the window in front. A click on a row whose page another window shows
# moves the page there live, with no second load, and the window it left keeps its rows and draws a
# picture of the page in its card, which a click on takes the page back from, live.
# ⇧⌘W closes a window and gives its tabs to the one that was in front before it, live, which shows
# its page again; in the last window it quits. ⇧↩ in the command bar opens its page in a new
# window, as ⌘N makes one, and a Switch to Tab row for that window's page brings the window
# forward rather than taking the page.

# The page counts in its title from its first frame, so a page that reloaded would count from 1
# again. "o" in it opens a tab under it, as a link to a new tab does.
page='data:text/html,<title>loading</title>
<script>
let count = 0;
requestAnimationFrame(() => setTimeout(() =>
  setInterval(() => document.title = "count " + ++count, 200)));
onkeydown = e => { if (e.key == "o") window.open(); };
</script>'
log="$out/windows.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* count 1$' || fail "the counting page didn't draw"
await '^browser: window opened [0-9]' || fail "the app didn't say which window opened"
first=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
a=$(today)
# The page's count, last logged.
count() {
  grep "^browser: title $a count " "$log" | tail -1 | cut -d' ' -f5
}
# Waits for the page to count three more, as a page that's live and not loaded again does.
counting() {
  from=$(count)
  await "^browser: title $a count $((from + 3))\$" || fail "$1: the page stopped counting at $from"
  [ "$(grep -c "^browser: loaded $a " "$log")" = 1 ] || fail "$1: the page loaded again"
}
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: windows. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
send type o
expect "^browser: today $a [0-9][0-9]*\$" "o in the page didn't open a tab under it"
b=$(today | cut -d' ' -f2)
expect "^browser: window $first shows $b\$" "the tab the page opened didn't show"
send click $(row 0)
expect "^browser: window $first shows $a\$" "a click on the counting page's row didn't show it"

empties=$(grep -c '^browser: empty card shown$' "$log" || true)
send key n cmd
expect '^browser: command new-window via ' "⌘N didn't run New Window"
expect '^browser: window opened [0-9]' "⌘N didn't open a window"
second=$(grep '^browser: window opened ' "$log" | tail -1 | cut -d' ' -f4)
[ "$second" != "$first" ] || fail "⌘N's window has the first one's id"
expect "^browser: window $second shows 0\$" "the new window showed a page"
none_since "^browser: window $first shows" || fail "⌘N changed what the first window shows"
shown_first=$(grep -c "^browser: window $first shows" "$log")
send window 2
../tools/window-screenshot.sh "$pid" "$out/windows-new.png" all || fail "no window to capture"
# The new window's card shows its line once the fade has ended, as the shell says in the log. The
# first window shows a page, so the line is the new window's.
for _ in $(seq 100); do
  [ "$(grep -c '^browser: empty card shown$' "$log")" -gt "$empties" ] && break
  sleep 0.1
done
[ "$(grep -c '^browser: empty card shown$' "$log")" -gt "$empties" ] ||
  fail "the new window's empty card didn't show its line"
../tools/window-screenshot.sh "$pid" "$out/empty-window.png" all || fail "no window to capture"
# A click on the card with ⌘T's bar open only closes it: the host closes the bar at the press, and
# the release mustn't open it again. Another driver step after the bar's report is in gives the
# release time to have run.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar in the new window"
expect '^browser: new tab plate 1$' "⌘T didn't give New Tab the selected plate"
opened=$(grep -c '^browser: command new-tab via card$' "$log" || true)
# Below the bar, which reaches y 410 and covers $page_point.
send click 600 600
expect '^browser: command bar hidden$' "a click on the card didn't close the command bar"
expect '^browser: new tab plate 0$' "a click on the card left New Tab's plate"
send window 2
[ "$(grep -c '^browser: command new-tab via card$' "$log" || true)" = "$opened" ] ||
  fail "the click that closed the command bar opened it again"

# A Space the new window makes and leaves: back in the first Space it shows the first tab no other
# window shows, the one the page opened, and the first window keeps the page.
send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run in the new window"
expect "^browser: window $second shows 0\$" "the new Space showed a page"
send type reading
send key return
expect '^browser: create space [0-9]* reading$' "return didn't name the new window's Space"
send key 1 ctrl
expect "^browser: window $second shows $b\$" \
  "back in the first Space, the new window didn't show the tab no window shows"
none_since "^browser: window $second shows $a\$" || fail "the new window took the first one's page"
[ "$(grep -c "^browser: window $first shows" "$log")" = "$shown_first" ] ||
  fail "the new window's Space switches changed what the first window shows"
counting "with two windows"

# A click on the page's row in the new window moves the page there, live.
send click $(row 0)
expect "^browser: window $second shows $a\$" \
  "a click on the page's row didn't show it in the new window"
counting "in the new window"
[ "$(today)" = "$a $b" ] || fail "the move changed Today to \"$(today)\""
../tools/window-screenshot.sh "$pid" "$out/windows-moved.png" all || fail "no window to capture"

# The first window, which the page left, asks the core for a picture of it once the move is over,
# and draws it in its card once it has stepped back.
expect "^browser: window $first page elsewhere $a\$" "the first window wasn't told its page went"
expect "^browser: window $first pictures tab $a\$" "the first window got no picture of its page"
expect "^browser: window $first picture rests\$" "the first window's picture didn't step back"
# Brought forward, it takes a new picture, and still leaves the page where it is.
send window 1
expect "^browser: window $first pictures tab $a\$" "the first window took no new picture as it came forward"
../tools/window-screenshot.sh "$pid" "$out/windows-pictured.png" || fail "no window to capture"

# A click on the picture takes the page back, live, from the new window, which draws its picture.
send click $page_point
expect '^browser: command select via picture$' "a click on the picture didn't take the page"
expect "^browser: window $first shows $a\$" "a click on the picture didn't show the page"
counting "back in the first window from its picture"
expect "^browser: window $second pictures tab $a\$" "the new window got no picture of the page"
expect "^browser: window $second picture rests\$" "the new window's picture didn't step back"
send window 2
expect "^browser: window $second pictures tab $a\$" "the new window took no new picture as it came forward"
../tools/window-screenshot.sh "$pid" "$out/windows-pictured-back.png" || fail "no window to capture"

# The row of the tab the new window has selected takes the page again, as before.
send click $(row 0)
expect "^browser: window $second shows $a\$" "a click on the row didn't take the page from its picture"
counting "in the new window again"

# ⇧⌘W closes the new window. The first keeps every row and shows its page again, live.
send key w cmd shift
expect '^browser: command close-window via ' "⇧⌘W didn't run Close Window"
expect "^browser: window closed $second\$" "⇧⌘W didn't close the window"
expect "^browser: window $first shows $a\$" "the first window didn't get its page back"
counting "back in the first window"
grep -q '^browser: tab [0-9]* closed$' "$log" && fail "closing a window closed a tab"
[ "$(today)" = "$a $b" ] || fail "closing the window changed Today to \"$(today)\""
send window 1
check_layout "back in the first window"
../tools/window-screenshot.sh "$pid" "$out/windows-closed.png" || fail "no window to capture"

# ⇧↩ in the command bar opens example.com in a new window, and the first keeps its page.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.com
send key return shift
# The bar sends what it suggested first, http:// while the https:// probe is out; the page upgrades.
expect '^browser: command bar window https*://example.com/' "⇧↩ didn't open example.com in a window"
expect '^browser: window opened [0-9]' "⇧↩ didn't open a window"
third=$(grep '^browser: window opened ' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: window $third shows [1-9][0-9]*\$" "⇧↩'s window didn't show a tab"
c=$(grep "^browser: window $third shows [1-9]" "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: loaded $c https://example.com/\$" "example.com didn't load in ⇧↩'s window"
none_since "^browser: window $first shows" || fail "⇧↩ changed what the first window shows"
../tools/window-screenshot.sh "$pid" "$out/windows-shift-return.png" all ||
  fail "no window to capture"

# From the first window, the Switch to Tab row for that page brings its window forward, and the
# first window keeps its own page. "example" is the start of the page's title and its URL.
send window 1
send key t cmd
expect '^browser: command bar shown$' "⌘T in the first window didn't open the command bar"
send type example
expect '^browser: command bar completes "example" with "\.' \
  "the command bar didn't complete \"example\""
send key backspace
suggested="^browser: command bar suggestions \"example\" in .* ms: .*https://example.com/ tab $c"
expect "$suggested" "the command bar didn't suggest the tab ⇧↩'s window shows"
for _ in 1 2 3 4 5 6 7 8; do
  send key down
  grep '^browser: command bar selected' "$log" | tail -1 | grep -q ' https://example.com/$' && break
done
send key return
expect "^browser: command bar switch to tab $c in window $third\$" \
  "↩ on the tab's row didn't go to the window that shows it"
expect "^browser: key window $third\$" "Switch to Tab didn't make the window showing the tab key"
none_since "^browser: window $first shows" ||
  fail "Switch to Tab changed what the first window shows"
counting "after Switch to Tab"
../tools/window-screenshot.sh "$pid" "$out/windows-switch.png" all || fail "no window to capture"

# ⇧↩'s window closes, so the next run doesn't bring back two.
send window 3
send key w cmd shift
expect "^browser: window closed $third\$" "⇧⌘W didn't close ⇧↩'s window"
send window 1

# ⇧⌘W in the last window quits. It's the driver's last command: the app doesn't report it done.
post key w cmd shift
exec 3>&-
await "^browser: window closed $first\$" || fail "⇧⌘W didn't close the last window"
exited
echo "ok: ⌘N opens a window on the same Spaces with no page, whose card shows its one line, and a click on it with the command bar open only closes the bar, which never takes another window's page on a Space switch, the Spaces menu acts on it, a click on a row moves its page there live and the window it left draws a picture of the page, a click on which takes the page back, ⇧⌘W gives the closed window's tabs back to the other, live, and quits from the last, ⇧↩ in the command bar opens a new window, and Switch to Tab brings forward the window showing the tab. Windows: $out/windows-new.png, $out/empty-window.png, $out/windows-moved.png, $out/windows-pictured.png, $out/windows-pictured-back.png, $out/windows-closed.png, $out/windows-shift-return.png, $out/windows-switch.png"
