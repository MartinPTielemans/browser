# Add Split View, from the menu, ⌃+ from the page and ⌥-click on "+ New Tab", narrows the page at
# once for a pane on its right, with the command bar over it, which escape takes back. ⌥⇧- and
# ⌥⌘-click on a link, and the link menu's Open Link in Split View, open it in a pane on the right.
# The link menu is Arc's, and its Open Link in New Window and in Incognito Window open the link in
# a window as ⌘N's and in a private one as ⇧⌘N's. A URL typed in the command bar opens in the pane,
# focused. The panes share the card 10 apart, the
# focused one ringed, and a click focuses a pane and gives its page the keyboard. The split is one
# row in the sidebar, which ⌘1–⌘9 count once and show with the pane focused last, and a click on a
# segment focuses its pane. The gap between the panes drags their widths, or stacked their heights,
# down to Chrome's least and snapping to even, and a double-click on it evens them. Expand Current
# Split gives the focused pane the card and the other 220 wide, and again does nothing. Change
# Orientation stacks the panes, with the find bar at the focused one's corner. Launched again, the
# app brings the split back stacked with both pages at the heights they were dragged to, and Change
# Orientation puts them side by side again. Separate Page from Split View shows the page alone, ⌥↩
# in the command bar opens a URL beside the page, and ⌘W closes the focused pane, leaving the page
# beside it alone, which ⇧⌘T puts back. The ✕ on a split's row closes all its panes, and the window
# shows the tab above them. ⇧⌘T brings them back together. A row dragged onto the page brings a
# target to each of the card's sides, which go when it's back over the sidebar, and let go on the
# left one, the tab joins the page shown on its left. The command bar's Flip Split View, Move Pane
# Right and Separate All Pages reorder and end a split.

# Split view, from the served page with example.org beside it.
serve
log="$out/split.log"
step=0
rm -f "$saved" "$launch_frame"
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
# ⌥-click on "+ New Tab" adds one too, as Dia's does.
send click 100 154 alt
expect '^browser: command add-split via sidebar$' \
  "⌥-click on \"+ New Tab\" didn't run Add Split View"
expect '^browser: command bar shown$' "⌥-click on \"+ New Tab\" didn't open the command bar"
read_pages
[ "$pages" = "WebContentsViewCocoa $left" ] ||
  fail "⌥-click on \"+ New Tab\" didn't narrow the page to $left but to $pages"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send click $page_point
# ⌥⇧-click on a link opens it in a pane on the right, focused, as Dia's does, and so does ⌥⌘-click,
# as Chrome's does.
for modifiers in "alt shift" "alt cmd"; do
  send click $((x + 20)) $((y + 20)) $modifiers
  expect "^browser: splits $a+[0-9][0-9]*\$" \
    "a click on a link with $modifiers didn't open it beside the page: \"$(splits)\""
  n=$(splits | cut -d+ -f2)
  expect "^browser: selected $n\( in .*\)\{0,1\}\$" "the link's pane isn't focused"
  expect "^browser: panes $a $n\$" "the window doesn't show the link beside the page"
  expect "^browser: loaded $n ${site}notes.txt\$" "the link didn't load in its pane"
  send key w cmd
  expect "^browser: tab $n closed\$" "⌘W didn't close the link's pane"
  expect "^browser: panes $a\$" "⌘W on the link's pane didn't leave the page alone"
  send click $page_point
done
# The link menu is Arc's: Open Link in New Tab, in New Window, in Split View and in Incognito
# Window, then Save Link As… and Copy Link Address, before the menu's other items.
link_items='Open Link in New Tab, Open Link in New Window, Open Link in Split View, '
link_items="${link_items}Open Link in Incognito Window, Save Link As…, Copy Link Address"
link_menu() {
  post rightclick $((x + 20)) $((y + 20))
  expect "^browser: menu shown $link_items\(, .*\)\{0,1\}\$" \
    "a right-click on the page's link didn't show Arc's link items in order: \
$(grep '^browser: menu shown' "$log" | tail -1)"
}
# The tab that loaded the link last.
link_tab() {
  sed -n "s|^browser: loaded \([0-9]*\) ${site}notes.txt\$|\1|p" "$log" | tail -1
}
# Open Link in New Window opens it in a window as ⌘N's, showing it.
link_menu
send choose Open Link in New Window
expect '^browser: window opened [0-9]' "Open Link in New Window didn't open a window"
link_window=$(grep '^browser: window opened ' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: loaded [0-9]* ${site}notes.txt\$" "the link didn't load in its window"
n=$(link_tab)
expect "^browser: window $link_window shows $n\$" "the link's window didn't show it"
send window 2
send key w cmd
expect "^browser: tab $n closed\$" "⌘W didn't close the link's tab in its window"
send key w cmd shift
expect "^browser: window closed $link_window\$" "⇧⌘W didn't close the link's window"
send window 1
send click $page_point
# Open Link in Incognito Window opens it in a private window as ⇧⌘N's, whose session ends with it.
link_menu
send choose Open Link in Incognito Window
expect '^browser: private window opened [0-9]* profile [0-9]*$' \
  "Open Link in Incognito Window didn't open a private window"
link_window=$(grep '^browser: private window opened ' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: loaded [0-9]* ${site}notes.txt\$" "the link didn't load in the private window"
n=$(link_tab)
expect "^browser: window $link_window shows $n\$" "the private window didn't show the link"
# The driver counts windows in the order they opened, closed ones too.
send window 3
send key w cmd shift
expect "^browser: window closed $link_window\$" "⇧⌘W didn't close the private window"
expect '^browser: private session ended [0-9]*$' "closing the private window didn't end its session"
send window 1
send click $page_point
# Open Link in Split View opens it beside the page, as ⌥⇧-click does.
link_menu
send choose Open Link in Split View
expect "^browser: splits $a+[0-9][0-9]*\$" \
  "Open Link in Split View didn't open the link beside the page: \"$(splits)\""
n=$(splits | cut -d+ -f2)
expect "^browser: selected $n\( in .*\)\{0,1\}\$" "the link's pane isn't focused"
expect "^browser: loaded $n ${site}notes.txt\$" "the link didn't load in its pane"
send key w cmd
expect "^browser: panes $a\$" "⌘W on the link's pane didn't leave the page alone"
send click $page_point
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
# The gap between the panes drags their widths, as Chrome's does, each kept a tenth of the card
# wide, at least 50 and at most 200, and the gap snaps to even within 15. A double-click on it
# evens them.
gap=$((x + half - 5))
least=$(((w + 5) / 10))
[ "$least" -lt 50 ] && least=50
[ "$least" -gt 200 ] && least=200
wider="WebContentsViewCocoa $x,$y $((half + 90))x$h"
narrower="$((x + half + 100)),$y $((w - half - 100))x$h"
send drag $gap 400 $((gap + 100)) 400
read_pages
[ "$pages" = "$wider WebContentsViewCocoa $narrower BrowserCoreFocusRing $narrower" ] ||
  fail "dragging the gap 100 right didn't widen the left pane by 100: $pages"
../tools/window-screenshot.sh "$pid" "$out/split-resized.png" || fail "no window to capture"
send drag $((gap + 100)) 400 $((x + w - 20)) 400
most="WebContentsViewCocoa $x,$y $((w - least - 10))x$h"
fewest="$((x + w - least)),$y ${least}x$h"
read_pages
[ "$pages" = "$most WebContentsViewCocoa $fewest BrowserCoreFocusRing $fewest" ] ||
  fail "dragged far right, the right pane isn't $least wide: $pages"
send drag $((x + w - least - 5)) 400 $((gap + 8)) 400
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "dragged to 8 from even, the gap didn't snap there: $pages"
send drag $gap 400 $((gap + 100)) 400
read_pages
[ "$pages" = "$wider WebContentsViewCocoa $narrower BrowserCoreFocusRing $narrower" ] ||
  fail "dragging the gap 100 right again didn't widen the left pane: $pages"
send doubleclick $((gap + 100)) 400
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "a double-click on the gap didn't even the panes: $pages"
# Expand Current Split gives the focused pane, the right one, the card in one frame and leaves the
# left 220 wide, and doing it again changes nothing. A double-click on the gap, now 220 in, evens
# them again.
expanded="WebContentsViewCocoa $x,$y 220x$h WebContentsViewCocoa $((x + 230)),$y $((w - 230))x$h"
expanded="$expanded BrowserCoreFocusRing $((x + 230)),$y $((w - 230))x$h"
send menu Expand Current Split
expect '^browser: command expand-split via ui$' "Expand Current Split didn't run from the menu"
read_pages
[ "$pages" = "$expanded" ] || fail "Expand Current Split didn't leave the left pane 220 wide: $pages"
../tools/window-screenshot.sh "$pid" "$out/split-expanded.png" || fail "no window to capture"
send menu Expand Current Split
read_pages
[ "$pages" = "$expanded" ] || fail "Expand Current Split again changed the panes: $pages"
send doubleclick $((x + 225)) 400
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "a double-click on the expanded split's gap didn't even the panes: $pages"
# Change Orientation stacks the panes top to bottom, 10 apart, each as tall as the other, the ring
# on the focused one, and the find bar goes to its top right corner. The driver's frames count
# from the bottom.
tall=$(((h + 10) / 2))
top="$x,$((y + h - tall + 10)) ${w}x$((tall - 10))"
bottom="$x,$y ${w}x$((h - tall))"
stacked="WebContentsViewCocoa $top WebContentsViewCocoa $bottom"
bar_command change-orientation "Change Orientation" "change orientation"
expect "^browser: splits $a/$b\$" "Change Orientation didn't stack the split: \"$(splits)\""
read_pages
[ "$pages" = "$stacked BrowserCoreFocusRing $bottom" ] ||
  fail "the panes aren't stacked with the ring on the bottom one: $pages"
send key f cmd
expect '^browser: find bar shown$' "⌘F in the bottom pane didn't show the find bar"
send layout
child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
  sed 's/^browser: layout child //')
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\).*/\1/')
height=${content#*x}
content=${content%x*}
[ "$child" = "$((content - 320)),$((20 + tall)) 300x32" ] ||
  fail "the find bar is at ${child:-nowhere}, not at the bottom pane's corner"
../tools/window-screenshot.sh "$pid" "$out/split-stacked.png" || fail "no window to capture"
send key escape
expect '^browser: find bar hidden$' "escape didn't close the find bar"
# Stacked, the gap drags their heights, the top one taller by 50 here, which the next launch keeps.
gap=$((height - (y + h - tall + 5)))
taller="$x,$((y + h - tall - 40)) ${w}x$((tall + 40))"
shorter="$x,$y ${w}x$((h - tall - 50))"
resized="WebContentsViewCocoa $taller WebContentsViewCocoa $shorter BrowserCoreFocusRing $shorter"
send drag 600 $gap 600 $((gap + 50))
read_pages
[ "$pages" = "$resized" ] ||
  fail "dragging the stacked gap 50 down didn't make the top pane taller: $pages"
exec 3>&-
quit

# Launched again, the split comes back stacked with both its pages, and the pane focused last
# focused. Change Orientation puts them side by side again.
log="$out/split-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the split"
await '^browser: splits [0-9][0-9]*/[0-9][0-9]*$' || fail "the split didn't come back stacked"
a=$(splits | cut -d/ -f1)
b=$(splits | cut -d/ -f2)
# The tab above the split, whose ID the relaunch may have changed too.
c=$(today | cut -d' ' -f1)
await "^browser: selected $b\$" || fail "the relaunch didn't focus the pane focused last"
await "^browser: panes $a $b\$" || fail "the relaunch didn't show both panes"
await "^browser: loaded $a $site\$" || fail "the split's top page didn't load"
await "^browser: loaded $b https://example.org/\$" || fail "the split's bottom page didn't load"
read_pages
[ "$pages" = "$resized" ] ||
  fail "the relaunch didn't show the panes stacked at the heights they were dragged to: $pages"
send doubleclick 600 $((gap + 50))
read_pages
[ "$pages" = "$stacked BrowserCoreFocusRing $bottom" ] ||
  fail "a double-click on the stacked gap didn't even the panes: $pages"
bar_command change-orientation "Change Orientation" "change orientation"
expect "^browser: splits $a+$b\$" "Change Orientation didn't put the panes side by side again"
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "the panes aren't side by side again with the ring on the right one: $pages"
send click $((x + w - 100)) 400
# Separate Page from Split View shows the focused pane alone.
send menu Separate Page from Split View
expect '^browser: splits $' "Separate Page from Split View didn't end the split"
expect "^browser: panes $b\$" "the separated page isn't shown alone"
read_pages
[ "$pages" = "$whole" ] || fail "the separated page doesn't fill the card: $pages"
# ⌘Z puts the page back in the split beside the one it left, and ⇧⌘Z takes it out again.
send key z cmd
expect '^browser: command undo via \(ui\|page\)$' "⌘Z didn't undo Separate Page from Split View"
expect "^browser: splits $a+$b\$" "⌘Z didn't put the page back in its split: \"$(splits)\""
send key z cmd shift
expect '^browser: command redo via \(ui\|page\)$' "⇧⌘Z didn't redo Separate Page from Split View"
expect '^browser: splits $' "⇧⌘Z didn't end the split again"
expect "^browser: panes $b\$" "⇧⌘Z didn't show the separated page alone"
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
# ⇧⌘T brings the closed pane back into its split, at its place, focused.
send key t cmd shift
expect "^browser: splits $b+[0-9][0-9]*\$" \
  "⇧⌘T didn't bring the pane back into its split: \"$(splits)\""
e=$(splits | cut -d+ -f2)
expect "^browser: selected $e\( in .*\)\{0,1\}\$" "the pane ⇧⌘T brought back isn't focused"
expect "^browser: panes $b $e\$" "the window doesn't show the pane ⇧⌘T brought back"
expect "^browser: loaded $e https://example.net/\$" "the pane ⇧⌘T brought back didn't load"
# Flip Split View swaps the panes, and Move Pane Right moves the focused one back to the right.
bar_command flip-split "Flip Split View" "flip split"
expect "^browser: splits $e+$b\$" "Flip Split View didn't swap the split's pages: \"$(splits)\""
expect "^browser: panes $e $b\$" "Flip Split View didn't swap the pages shown"
bar_command move-pane-right "Move Pane Right" "move pane right"
expect "^browser: splits $b+$e\$" "Move Pane Right didn't move the focused page: \"$(splits)\""
expect "^browser: panes $b $e\$" "Move Pane Right didn't move the page shown"
# The ✕ on a split's row closes all its panes, and with the left one focused the window shows the
# tab its Space picks after them.
send click $((x + 100)) 400
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "a click on the left pane didn't focus it"
send move 202 $(row 2 | cut -d' ' -f2)
expect '^browser: hover tab [0-9][0-9]* close 1 favicon none$' "the split's row didn't draw its ✕"
send click 202 $(row 2 | cut -d' ' -f2)
expect '^browser: command close-tab via sidebar$' "a click on the split's ✕ didn't run Close Tab"
expect "^browser: tab $b closed\$" "the split's ✕ didn't close its left pane"
expect "^browser: tab $e closed\$" "the split's ✕ didn't close its right pane"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "closing the split didn't show the tab above it"
# ⇧⌘T twice brings them back, the second into a split with the first, and the ✕ closes them again.
send key t cmd shift
send key t cmd shift
expect "^browser: splits [0-9][0-9]*+[0-9][0-9]*\$" \
  "⇧⌘T twice didn't bring the split's panes back together: \"$(splits)\""
send click $((x + 100)) 400
send move 202 $(row 2 | cut -d' ' -f2)
expect '^browser: hover tab [0-9][0-9]* close 1 favicon none$' "the split's row didn't draw its ✕"
send click 202 $(row 2 | cut -d' ' -f2)
expect '^browser: command close-tab via sidebar$' "a click on the split's ✕ didn't run Close Tab"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "closing the split again didn't show $a"
# A row dragged onto the page becomes a picture of a window, and a target slides in at each of the
# card's sides. Back over the sidebar it's a row again, and let go over the left target it joins
# the page shown on its left, focused.
target="$((x + 140)) $((y + h / 2))"
send press $(row 0)
send dragto 110 $(row 0 | cut -d' ' -f2)
send dragto $target
none_since '^browser: detent$' || fail "a row dragged onto the page clicked the trackpad"
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
# Separate All Pages ends the split, with its pages in their order, and the focused one alone.
bar_command separate-split "Separate All Pages" "separate all"
expect '^browser: splits $' "Separate All Pages didn't end the split: \"$(splits)\""
expect "^browser: panes $c\$" "Separate All Pages didn't show the focused page alone"
expect "^browser: today.* $c $a\( \|\$\)" "Separate All Pages lost the pages' order: $(today)"
exec 3>&-
quit
echo "ok: Add Split View narrows the page at once for a pane on the right, with the command bar over it, which escape takes back, and ⌃+ from the page and ⌥-click on \"+ New Tab\" do it too. ⌥⇧- and ⌥⌘-click on a link and the link menu's Open Link in Split View open it in a pane on the right. The link menu is Arc's, and its Open Link in New Window and in Incognito Window open the link in a window and a private window. A URL there opens in the pane, focused. The panes share the card 10 apart, the focused one ringed, and a click focuses a pane and gives its page the keyboard. The split is one row in the sidebar, which ⌘1–⌘9 count once and show with the pane focused last, and a click on a segment focuses that pane. The gap between the panes drags their widths, or stacked their heights, down to Chrome's least and snapping to even, and a double-click evens them. Expand Current Split gives the focused pane the card and the other 220 wide, and again changes nothing. Change Orientation stacks the panes, with the find bar at the focused one's corner, and the split comes back stacked after a quit with both pages at the heights they were dragged to, until Change Orientation puts them side by side again. Separate Page from Split View shows the page alone, ⌥↩ in the command bar opens a URL beside the page, ⌘W closes the focused pane, which ⇧⌘T puts back in its split, and the ✕ on a split's row closes all its panes, which ⇧⌘T brings back together. A row dragged onto the page brings a target to each of the card's sides, which go when it's back over the sidebar, and let go on the left one the tab joins the page on its left. Flip Split View and Move Pane Right in the command bar reorder a split's pages, and Separate All Pages ends it. ⌘Z after Separate Page from Split View puts the page back in its split, and ⇧⌘Z takes it out again. Windows: $out/split.png, $out/split-left.png, $out/split-resized.png, $out/split-expanded.png, $out/split-stacked.png, $out/split-targets.png"
