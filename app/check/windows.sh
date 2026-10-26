# ⌘N opens another window on the same Spaces, showing the key window's Space with no page, and
# never another window's page: a Space it switches to and back shows the first tab no window
# shows. The Spaces menu acts on the window in front. A click on a row whose page another window
# shows moves the page there live, with no second load, and the window it left keeps its rows.
# ⇧⌘W closes a window and gives its tabs to the one that was in front before it, live, which shows
# its page again; in the last window it quits.

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

# ⇧⌘W in the last window quits. It's the driver's last command: the app doesn't report it done.
post key w cmd shift
exec 3>&-
await "^browser: window closed $first\$" || fail "⇧⌘W didn't close the last window"
exited
echo "ok: ⌘N opens a window on the same Spaces with no page, which never takes another window's page on a Space switch, the Spaces menu acts on it, a click on a row moves its page there live, ⇧⌘W gives the closed window's tabs back to the other, live, and quits from the last. Windows: $out/windows-new.png, $out/windows-moved.png, $out/windows-closed.png"
