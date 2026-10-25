# Add Split View, from the menu and ⌃+ from the page, narrows the page at once for a pane on its
# right, with the command bar over it, which escape takes back. A URL typed there opens in the pane,
# focused. The panes share the card 10 apart, the focused one ringed, and a click focuses a pane and
# gives its page the keyboard. The split is one row in the sidebar, which ⌘1–⌘9 count once and show
# with the pane focused last, and a click on a segment focuses its pane. Launched again, the app
# brings the split back with both pages. Separate Page from Split View shows the page alone, ⌥↩ in
# the command bar opens a URL beside the page, and ⌘W closes the focused pane, leaving the page
# beside it alone. The ✕ on a split's row closes all its panes, and the window shows the tab above
# them. A row dragged onto the page brings a target to each of the card's sides, which go when it's
# back over the sidebar, and let go on the left one, the tab joins the page shown on its left.

# Split view, from the served page with example.org beside it.
serve
log="$out/split.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the split view test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: split view. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The Space's splits the app last showed, each as its tabs joined by "+".
splits() {
  grep '^browser: splits' "$log" | tail -1 | sed 's/^browser: splits *//'
}
# Sets $pages to the frames of the pages shown, left to right, then the focused one's ring. Not in
# a subshell, where send's count of the commands sent would be lost.
read_pages() {
  send layout
  pages=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
    grep -o '\(WebContentsViewCocoa\|BrowserCoreFocusRing\) [0-9,]* [0-9x]*' | tr '\n' ' ' |
    sed 's/ $//')
}
# Two panes share the page's frame, 10 apart, as Arc's do.
read_pages
whole=$pages
frame=${whole#WebContentsViewCocoa }
x=${frame%%,*}
y=${frame#*,}
y=${y%% *}
w=${frame#* }
h=${w#*x}
w=${w%x*}
half=$(((w + 10) / 2))
left="$x,$y $((half - 10))x$h"
right="$((x + half)),$y $((w - half))x$h"
# Add Split View narrows the page at once to make room on the right, where the command bar opens,
# and escape gives the page its frame back.
send menu Add Split View
expect '^browser: command add-split via ui$' "Add Split View didn't run"
expect '^browser: command bar shown$' "Add Split View didn't open the command bar"
read_pages
[ "$pages" = "WebContentsViewCocoa $left" ] ||
  fail "while a pane is added, the page isn't at $left but at $pages"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
read_pages
[ "$pages" = "$whole" ] || fail "escape didn't give the page its frame back: $pages"
# ⌃+ from the page adds one too, and a URL typed there opens in the new pane, focused.
send key + ctrl
expect '^browser: command add-split via page$' "⌃+ with the page focused didn't reach the UI"
expect '^browser: command bar shown$' "⌃+ didn't open the command bar"
send type example.org
send key return
expect "^browser: splits $a+[0-9][0-9]*\$" "example.org didn't open beside the page: \"$(splits)\""
b=$(splits | cut -d+ -f2)
expect "^browser: selected $b\$" "the new pane isn't focused"
expect "^browser: panes $a $b\$" "the window doesn't show both panes"
expect "^browser: loaded $b https://example.org/\$" "example.org didn't load in the new pane"
two="WebContentsViewCocoa $left WebContentsViewCocoa $right"
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "the panes aren't side by side with the ring on the right one: $pages"
../tools/window-screenshot.sh "$pid" "$out/split.png" || fail "no window to capture"
# A click in a pane focuses it, its page taking the keyboard, and the ring goes with it.
send click $((x + 100)) 400
expect "^browser: selected $a\$" "a click in the left pane didn't focus it"
expect "^browser: panes $a $b\$" "focusing a pane changed the panes shown"
[ "$focus" = RenderWidgetHostViewCocoa ] || fail "the left pane's page doesn't have the keyboard"
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $left" ] ||
  fail "the ring didn't go to the left pane: $pages"
../tools/window-screenshot.sh "$pid" "$out/split-left.png" || fail "no window to capture"
# In the sidebar the split is one row, which ⌘2 shows under a tab opened above it, with the pane
# focused last, and a click on its right segment shows that pane focused.
open_from_bar example.net
c=$tab
expect "^browser: today $c $a $b\$" "the new tab isn't above the split: Today is \"$(today)\""
send key 2 cmd
expect "^browser: selected $a\( in .*\)\{0,1\}\$" \
  "⌘2 didn't show the split with its pane focused last"
expect "^browser: panes $a $b\$" "⌘2 didn't show both panes"
send click 170 $(row 1 | cut -d' ' -f2)
expect "^browser: selected $b\( in .*\)\{0,1\}\$" \
  "a click on the split's right segment didn't focus it"
exec 3>&-
quit

# Launched again, the split comes back with both its pages, and the pane focused last focused.
log="$out/split-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the split"
await '^browser: splits [0-9][0-9]*+[0-9][0-9]*$' || fail "the split didn't come back"
a=$(splits | cut -d+ -f1)
b=$(splits | cut -d+ -f2)
# The tab above the split, whose ID the relaunch may have changed too.
c=$(today | cut -d' ' -f1)
await "^browser: selected $b\$" || fail "the relaunch didn't focus the pane focused last"
await "^browser: panes $a $b\$" || fail "the relaunch didn't show both panes"
await "^browser: loaded $a $site\$" || fail "the split's left page didn't load"
await "^browser: loaded $b https://example.org/\$" || fail "the split's right page didn't load"
send click $((x + w - 100)) 400
# Separate Page from Split View shows the focused pane alone.
send menu Separate Page from Split View
expect '^browser: splits $' "Separate Page from Split View didn't end the split"
expect "^browser: panes $b\$" "the separated page isn't shown alone"
read_pages
[ "$pages" = "$whole" ] || fail "the separated page doesn't fill the card: $pages"
# ⌥↩ in the command bar opens its URL in a pane beside the page shown, and ⌘W closes the focused
# pane, leaving the card to the page beside it.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.net
send key return alt
expect "^browser: splits $b+[0-9][0-9]*\$" \
  "⌥↩ didn't open example.net beside the page: \"$(splits)\""
d=$(splits | cut -d+ -f2)
expect "^browser: panes $b $d\$" "⌥↩ didn't show the page and its new pane"
expect "^browser: loaded $d https://example.net/\$" "example.net didn't load in the new pane"
send key w cmd
expect "^browser: tab $d closed\$" "⌘W didn't close the focused pane"
expect "^browser: panes $b\$" "⌘W on a pane didn't leave the page beside it alone"
expect '^browser: splits $' "the split outlived its pane"
read_pages
[ "$pages" = "$whole" ] || fail "the page left doesn't fill the card: $pages"
# The ✕ on a split's row closes all its panes, and with the left one focused the window shows the
# tab its Space picks after them.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.net
send key return alt
expect "^browser: splits $b+[0-9][0-9]*\$" "⌥↩ didn't open example.net beside the page again"
e=$(splits | cut -d+ -f2)
expect "^browser: loaded $e https://example.net/\$" "example.net didn't load in the new pane"
send click $((x + 100)) 400
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "a click on the left pane didn't focus it"
send move 202 $(row 2 | cut -d' ' -f2)
send click 202 $(row 2 | cut -d' ' -f2)
expect '^browser: command close-tab via sidebar$' "a click on the split's ✕ didn't run Close Tab"
expect "^browser: tab $b closed\$" "the split's ✕ didn't close its left pane"
expect "^browser: tab $e closed\$" "the split's ✕ didn't close its right pane"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "closing the split didn't show the tab above it"
# A row dragged onto the page becomes a picture of a window, and a target slides in at each of the
# card's sides. Back over the sidebar it's a row again, and let go over the left target it joins
# the page shown on its left, focused.
target="$((x + 140)) $((y + h / 2))"
send press $(row 0)
send dragto 110 $(row 0 | cut -d' ' -f2)
send dragto $target
expect "^browser: drop targets for $a\$" "a row dragged onto the page didn't bring the targets"
expect '^browser: drop targets shown$' "the targets' window didn't show"
expect '^browser: drop target left$' "the left target isn't under the tab"
../tools/window-screenshot.sh "$pid" "$out/split-targets.png" || fail "no window to capture"
send dragto 100 300
expect '^browser: drop targets gone$' "the targets stayed with the tab back over the sidebar"
expect '^browser: drop targets hidden$' "the targets' window stayed"
send dragto $target
expect '^browser: drop target left$' "the left target isn't under the tab again"
send release $target
expect "^browser: splits $c+$a\$" "the tab let go on the left target didn't join the page: \"$(splits)\""
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "the tab let go on a target isn't focused"
expect "^browser: panes $c $a\$" "the window doesn't show the tab let go left of the page"
exec 3>&-
quit
echo "ok: Add Split View narrows the page at once for a pane on the right, with the command bar over it, which escape takes back, and ⌃+ from the page does it too. A URL there opens in the pane, focused. The panes share the card 10 apart, the focused one ringed, and a click focuses a pane and gives its page the keyboard. The split is one row in the sidebar, which ⌘1–⌘9 count once and show with the pane focused last, and a click on a segment focuses that pane. The split comes back after a quit with both pages. Separate Page from Split View shows the page alone, ⌥↩ in the command bar opens a URL beside the page, ⌘W closes the focused pane, and the ✕ on a split's row closes all its panes. A row dragged onto the page brings a target to each of the card's sides, which go when it's back over the sidebar, and let go on the left one the tab joins the page on its left. Windows: $out/split.png, $out/split-left.png, $out/split-targets.png"
