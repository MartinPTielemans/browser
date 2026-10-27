# Add to Favorites in a Today tab's menu takes the tab out of Today into a tile above the Space's
# rows, without showing it. Resting on the tile names it in a tooltip, which other apps don't take
# for the window in front, and the footer's Library button and the header's sidebar button name
# themselves with their shortcuts' keys on chips. A click on the tile shows it, and ⌘1–⌘9 count favorites first. Past its top, the list shows an
# edge under the tile, and showing a tab under its bottom scrolls to the tab's row. Away from its
# URL, a double-click on the tile goes back there. ⌘W unloads it and keeps the tile, showing the
# top Today tab. The tile's menu duplicates it to the top of Today. Launched
# again, the app brings the favorite back, shown, and Remove from Favorites in its menu puts it at
# the top of Today. A row dragged onto the tiles turns into a tile, and a tile dragged into the
# list into a row. Today rows dragged onto the tiles become favorites where they're dropped, a tile
# dragged across the other goes before it, one dragged into the list goes there, and escape puts a
# dragged tile back. Tiles taller than 40 % of the window scroll with the list. ⌘, from the page and
# the UI opens Settings, where switches turn the warning before quitting and the haptics off and the
# menus of two pop-up buttons set how long Today tabs wait for the archive and who shares the
# favorites (Each Space, kept across the quit and its favorite with it, then Each Profile's Spaces
# again), and all of them stay after the quit, when ⌘Q quits without asking. ⌘W and the window's close button hide it, and More
# Settings… opens Chromium's settings in a tab.

# Favorites, from the served page with example.org above it.
serve
log="$out/favorites.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the favorites test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: favorites. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The favorites the app last showed, left to right.
favorites() {
  grep '^browser: favorites' "$log" | tail -1 | sed 's/^browser: favorites *//'
}
# One favorite is one tile across the sidebar, 41 tall and 90 below the window's top, under the URL
# field, and its row of tiles takes the Space's rows 47 down.
tile="114 110"
tiled_row() {
  echo "100 $(($(row "$1" | cut -d' ' -f2) + 47))"
}
open_from_bar example.org
b=$tab
expect_tabs "$b $a" "$b" "after opening a tab"
# Add to Favorites in a Today tab's menu takes the tab out of Today into a tile, without showing it.
post rightclick $(row 1)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below (disabled)$' \
  "a right-click on a Today row didn't offer Add to Favorites"
send choose Add to Favorites
expect '^browser: command add-to-favorites via tab menu$' "Add to Favorites didn't run"
expect "^browser: favorites $a\$" \
  "Add to Favorites didn't make the tab a favorite: \"$(favorites)\""
expect "^browser: today $b\$" "the favorite is still in Today: Today is \"$(today)\""
none_since '^browser: selected' || fail "Add to Favorites showed another tab"
# The tile has only an icon, so after a moment over it a tooltip names the favorite, and it goes
# when the pointer leaves and when the tile is clicked.
send move $tile
expect '^browser: tooltip shown ready$' "resting on the tile didn't show its tooltip"
send move 100 400
expect '^browser: tooltip hidden$' "the pointer leaving the tile didn't hide its tooltip"
send move $tile
expect '^browser: tooltip shown ready$' "resting on the tile again didn't show its tooltip"
expect '^browser: tooltip window shown$' "the tile's tooltip didn't fade in"
# It hangs from the tile it names: its square corner, the top left, is under the tile's middle, 114
# across, and the window the host put over the page is where the UI asked.
since_move() {
  sed -n "/^browser: drive $step: /,\$p" "$log" | grep "$1" | tail -1
}
asked=$(since_move '^browser: tooltip at ')
placed=$(since_move '^browser: tooltip window at ')
echo "$asked $placed" | awk '
  { middle = $4 - 114; across = $4 - $12; down = $5 - $13 }
  END { exit !(NR == 1 && NF == 13 && middle > -1 && middle < 1 && across > -1 && across < 1 \
    && down > -1 && down < 1) }' \
  || fail "the tooltip isn't under the tile it names: asked \"$asked\", placed \"$placed\""
../tools/window-screenshot.sh "$pid" "$out/favorite-tooltip.png" || fail "no window to capture"
# Another app asking which window is in front, as T3 Code's window capture does, gets the browser
# window, not the tooltip over it or the command bar's shadow.
send front
expect '^browser: front driven, main driven, key driven, ax focused driven, ax main driven$' \
  "with the tooltip showing, other apps take the wrong window: $(grep '^browser: front ' "$log" | tail -1)"
# A click on the tile shows the favorite, and ⌘1–⌘9 count favorites first.
send click $tile
expect '^browser: command select via favorite$' "a click on the tile didn't run"
expect '^browser: tooltip hidden$' "a click on the tile didn't hide its tooltip"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
../tools/window-screenshot.sh "$pid" "$out/favorites.png" || fail "no window to capture"
# The footer's Library button names itself beside it, with its shortcut's keys on chips (Arc), and
# the header's sidebar button under it; each goes when the pointer leaves.
check_layout "before the buttons' tooltips"
send move 24 $((height - 25))
expect '^browser: tooltip shown View Library ⇧⌘L$' \
  "resting on the Library button didn't show View Library and ⇧⌘L"
expect '^browser: tooltip window shown$' "the Library button's tooltip didn't fade in"
../tools/window-screenshot.sh "$pid" "$out/library-tooltip.png" || fail "no window to capture"
send move 100 400
expect '^browser: tooltip hidden$' "the pointer leaving the Library button didn't hide its tooltip"
send move 93 23
expect '^browser: tooltip shown Hide Sidebar ⌘S$' \
  "resting on the sidebar button didn't show Hide Sidebar and ⌘S"
expect '^browser: tooltip window shown$' "the sidebar button's tooltip didn't fade in"
../tools/window-screenshot.sh "$pid" "$out/header-tooltip.png" || fail "no window to capture"
send move 100 400
expect '^browser: tooltip hidden$' "the pointer leaving the sidebar button didn't hide its tooltip"
send click $(tiled_row 0)
expect "^browser: selected $b\( in .*\)\{0,1\}\$" \
  "a click on the Today row under the tile didn't show its tab"
switch "1 cmd" "$a"
# Away from its URL, a double-click on the tile goes back there.
tab=$a
go_from_bar example.com
expect "^browser: page $a away 1\$" "the favorite at another page didn't read as away from its URL"
send doubleclick $tile
expect '^browser: command back-to-pinned-url via favorite$' \
  "a double-click on the tile didn't run Back to Favorite URL"
expect "^browser: loaded $a $site\$" "a double-click on the tile didn't load the favorite's URL"
expect "^browser: page $a away 0\$" "the favorite still read as away at its URL"
# ⌘W unloads a favorite and keeps its tile, showing the top Today tab, and ⌘1 loads it again.
send click $page_point
send key w cmd
expect "^browser: selected $b\$" "⌘W on a favorite didn't show the top Today tab"
expect "^browser: page $a unloaded 1\$" "⌘W on a favorite didn't unload it"
none_since "^browser: tab $a closed\$" || fail "⌘W closed a favorite"
[ "$(favorites)" = "$a" ] ||
  fail "⌘W on a favorite took its tile: the favorites are \"$(favorites)\""
../tools/window-screenshot.sh "$pid" "$out/favorite-unloaded.png" || fail "no window to capture"
switch "1 cmd" "$a"
expect "^browser: loaded $a $site\$" "the unloaded favorite didn't load once shown"
# The tile's menu. Duplicate Tab opens a copy at the top of Today and shows it.
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL (disabled), Duplicate Tab, Remove from Favorites, Close Tab$' \
  "a right-click on the tile didn't show the favorite's menu"
send choose Duplicate Tab
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect "^browser: today [0-9][0-9]* $b\$" \
  "Duplicate Tab didn't open a copy at the top of Today: Today is \"$(today)\""
c=$(today | cut -d' ' -f1)
expect "^browser: selected $c\$" "Duplicate Tab didn't show the copy"
expect "^browser: loaded $c $site\$" "the copy didn't load the favorite's page"
# The row under the copy slides down to make room, and until it's there the list is only as long as
# where the row is drawn, too short to scroll in the window below.
expect '^browser: rows settled$' "the row under the copy didn't settle"
# Past its top in a window too short for its rows, the list shows a line across the sidebar 4 under
# the tile, and back at the top it hides it.
send layout
size=$(grep '^browser: layout content' "$log" | tail -1 |
  sed 's/^browser: layout content \([0-9]*\)x\([0-9]*\),.*/\1 \2/')
send resize 1000 340
# The list scrolls as far as the last frame drawn has it.
expect '^browser: drawn at 1000x340$' "the window didn't draw at its new size"
send scroll 100 200 10 5
expect '^browser: scroll edge shown$' "the list scrolled past its top didn't show its edge"
../tools/window-screenshot.sh "$pid" "$out/scroll-edge.png" || fail "no window to capture"
edge=$(pixel "$out/scroll-edge.png" 0.1 $(echo "135.5 / 340" | bc -l))
gap=$(pixel "$out/scroll-edge.png" 0.1 $(echo "133.5 / 340" | bc -l))
brighter "$edge" "$gap" || fail "the edge is $edge over the sidebar's $gap"
send scroll 100 200 -10 5
expect '^browser: scroll edge hidden$' "the list back at its top didn't hide its edge"
# There, showing the last tab scrolls the list the least that shows its row whole, and showing a
# tab whose row is in view leaves the list where it is.
switch "9 cmd" "$b"
expect "^browser: list scrolled to [1-9][0-9.]* for $b\$" \
  "showing a tab under the list's bottom didn't scroll to its row"
expect '^browser: scroll edge shown$' "the list scrolled to a row didn't show its edge"
../tools/window-screenshot.sh "$pid" "$out/list-scrolled.png" || fail "no window to capture"
switch "2 cmd" "$c"
none_since '^browser: list scrolled' || fail "showing a tab in view scrolled the list"
send resize $size
expect "^browser: drawn at ${size/ /x}\$" "the window didn't draw back at its size"
# ⌘, from the page opens Settings on General. Its switch turns the warning before quitting off, the
# menu of its pop-up button sets how long Today tabs wait for the archive, the switch on its
# Advanced tab turns the haptics off, and ⌘W hides it.
send click $page_point
send key , cmd
expect '^browser: command settings via page$' "⌘, from the page didn't run Settings"
expect '^browser: settings shown General$' "⌘, didn't show Settings on its General tab"
../tools/window-screenshot.sh "$pid" "$out/settings.png" || fail "no window to capture"
send clickkey 592 126
expect '^browser: warn before quitting off$' "a click on Warn before quitting didn't turn it off"
post clickkey 500 168
expect '^browser: menu shown 12 hours (default) (checked), 24 hours, 7 days, 30 days$' \
  "a click on Archive tabs after didn't show the times, with 12 hours chosen"
send choose 24 hours
expect '^browser: archive tabs after 24 hours$' "choosing 24 hours in Settings didn't set it"
post clickkey 500 213
expect "^browser: menu shown In Every Space, In Each Profile's Spaces (checked), In Each Space\$" \
  "a click on Favorites didn't show the choices, with Each Profile's Spaces chosen"
send choose In Each Space
expect "^browser: favorites scope In Each Space\$" "choosing In Each Space in Settings didn't set it"
send clickkey 430 60
expect '^browser: settings tab Advanced$' "a click on Advanced didn't show its tab"
send clickkey 592 126
expect '^browser: haptics off$' "a click on the haptics switch didn't turn them off"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
none_since '^browser: command close-tab' || fail "⌘W in Settings closed a tab"
# The quit leaves the favorite shown.
send click $tile
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
exec 3>&-
quit

# Launched again, the favorite comes back shown, and Remove from Favorites in its menu puts it at
# the top of Today.
log="$out/favorites-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the favorites"
await '^browser: favorites [0-9][0-9]*$' || fail "the favorite didn't come back"
a=$(favorites)
await '^browser: today [0-9][0-9]* [0-9][0-9]*$' || fail "the Today tabs didn't come back"
left=$(today)
await "^browser: selected $a\$" || fail "the relaunch didn't show the favorite shown last"
await "^browser: loaded $a $site\$" || fail "the favorite didn't load"
../tools/window-screenshot.sh "$pid" "$out/favorites-restored.png" || fail "no window to capture"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored favorite's menu. Browser wasn't the active app, so the page ignored"
  echo "  the click."
  exit 0
fi
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL (disabled), Duplicate Tab, Remove from Favorites, Close Tab$' \
  "a right-click on the restored tile didn't show the favorite's menu"
send choose Remove from Favorites
expect '^browser: command remove-from-favorites via tab menu$' "Remove from Favorites didn't run"
expect '^browser: favorites $' "Remove from Favorites left the tile"
expect "^browser: today $a $left\$" \
  "Remove from Favorites didn't put the tab at the top of Today: Today is \"$(today)\""
none_since '^browser: selected' || fail "Remove from Favorites showed another tab"
# Dragging moves tabs between the rows and the tiles. With no favorites, a Today row dragged over
# the Space's name is drawn as a tile there, and dropped it's the first favorite.
set -- $left
x=$1
y=$2
send press $(row 1)
send dragto 100 200
send dragto 114 112
expect '^browser: drag shape tile$' "a row dragged onto the favorites didn't turn into a tile"
expect '^browser: detent$' "a row dragged onto the favorites didn't click the trackpad"
../tools/window-screenshot.sh "$pid" "$out/favorite-drag.png" || fail "no window to capture"
none_since '^browser: dragged' ||
  fail "a row dragged onto the favorites dropped before the pointer let go"
send release 114 112
none_since '^browser: detent$' || fail "letting go on the favorites clicked the trackpad"
expect "^browser: dragged $x to favorites 0\$" \
  "a Today row dragged onto the favorites didn't go there"
expect "^browser: favorites $x\$" \
  "the row dropped on the favorites isn't one: the favorites are \"$(favorites)\""
expect "^browser: today $a $y\$" "the row dropped on the favorites is still in Today: \"$(today)\""
# Dropped right of the tile, a row is the second favorite, and the two share the tiles' row.
send drag $(tiled_row 0) 180 112
expect "^browser: dragged $a to favorites 1\$" "a row dragged right of the tile didn't go there"
expect "^browser: favorites $x $a\$" \
  "the second row dropped on the favorites isn't the second: the favorites are \"$(favorites)\""
expect '^browser: tiles settled$' "the tiles didn't settle after the drop"
# A tile dragged left of the other goes before it.
send drag 168 110 40 110
expect "^browser: dragged $a to favorites 0\$" \
  "a tile dragged left of the other didn't go before it"
expect "^browser: favorites $a $x\$" "the tiles didn't swap: the favorites are \"$(favorites)\""
expect '^browser: tiles settled$' "the tiles didn't settle after the swap"
# A tile dragged into the list goes there, unpinned in Today.
send drag 168 110 100 240
expect "^browser: dragged $x to today 0\$" "a tile dragged into Today didn't go there"
expect "^browser: today $x $y\$" "the tile dropped in Today isn't at its top: Today is \"$(today)\""
expect "^browser: favorites $a\$" "the tile dropped in Today is still a favorite"
none_since '^browser: selected' || fail "dragging a tile showed it"
# Escape puts a dragged tile back.
send press 114 110
none_since '^browser: detent$' || fail "pressing a tile clicked the trackpad"
send dragto 100 200
expect '^browser: drag shape row$' "a tile dragged into the list didn't turn into a row"
expect '^browser: detent$' "a tile dragged into the list didn't click the trackpad"
../tools/window-screenshot.sh "$pid" "$out/favorite-drag-out.png" || fail "no window to capture"
send dragto 100 300
send key escape
expect '^browser: command stop via ' "escape during a tile's drag didn't reach the UI"
send release 100 300
none_since '^browser: dragged' || fail "a tile's drag escape put back dropped it"
[ "$(favorites)" = "$a" ] || fail "a tile's drag escape put back moved it: \"$(favorites)\""
# Three tiles in the sidebar at its narrowest take two rows, 94 points, more than 40 % of a window
# 220 tall, so they scroll with the list, under the URL field (Ardia).
send drag $(tiled_row 0) 200 112
expect "^browser: dragged $x to favorites 1\$" "a row dragged right of the tile didn't go there"
expect '^browser: tiles settled$' "the tiles didn't settle after the second drop"
send drag $(tiled_row 0) 200 112
expect "^browser: dragged $y to favorites 2\$" "a row dragged right of the tiles didn't go last"
expect "^browser: favorites $a $x $y\$" "the tiles aren't three: the favorites are \"$(favorites)\""
expect '^browser: tiles settled$' "the tiles didn't settle after the third drop"
# The drag's press showed $y, so the click on its tile below has a tab to change from.
switch "1 cmd" "$a"
send drag 225 400 120 400
expect '^browser: sidebar width 164$' "dragging the sidebar's edge didn't narrow it to 164"
send layout
size=$(grep '^browser: layout content' "$log" | tail -1 |
  sed 's/^browser: layout content \([0-9]*\)x\([0-9]*\),.*/\1 \2/')
send resize 1000 220
expect '^browser: tiles scroll with the list$' \
  "two rows of tiles in a window 220 tall didn't scroll with the list"
../tools/window-screenshot.sh "$pid" "$out/tiles-tall.png" || fail "no window to capture"
# Scrolled 40, the tiles went up 40 with the rows, and the first tile's place holds the third's.
send scroll 100 150 20 2
expect '^browser: scroll edge shown$' "the list scrolled with the tiles didn't show its edge"
../tools/window-screenshot.sh "$pid" "$out/tiles-scrolled.png" || fail "no window to capture"
send click 40 117
expect '^browser: command select via favorite$' "a click on the scrolled tiles didn't reach one"
expect "^browser: selected $y\( in .*\)\{0,1\}\$" \
  "the third tile isn't 40 up with the list: a click there didn't show its favorite"
send scroll 100 150 -20 2
expect '^browser: scroll edge hidden$' "the list back at its top didn't hide its edge"
send resize $size
expect '^browser: tiles stay put$' "the tiles in a window back at its height still scroll"
send doubleclick 161 400
expect '^browser: sidebar width 228$' \
  "a double-click on the sidebar's edge didn't put its width back"
# Three tiles across the sidebar with titles of very different widths, long, medium and short as
# the owner's are: each tooltip hangs from its own tile, going along them and back, not placed from
# the width of the one before. It's placed where the UI asked, drawn as wide as it was placed for,
# and its square corner, the top left, is under the tile's middle, with the tooltip going right
# from there over the page. Where that would cross the window's right edge it goes left instead, its
# top right corner under the tile's middle.
long="Creating A World That Doesn't Exist - YouTube"
medium="Elon Musk on X: a post / X"
short="Proton Mail: Sign-in"
for page in long medium short; do
  eval "printf '<title>%s</title>' \"\$$page\"" > "$out/site/$page.html"
done
for titled in "1 $a long" "2 $x medium" "3 $y short"; do
  set -- $titled
  switch "$1 cmd" "$2"
  tab=$2
  go_from_bar "${site#http://}$3.html"
  eval "title=\$$3"
  expect "^browser: title $2 $title\$" "the $3 page didn't give its title to tab $2"
done
# The width the window was last drawn at.
drawn_width() {
  grep '^browser: drawn at' "$log" | tail -1 | sed 's/^browser: drawn at \([0-9.]*\)x.*/\1/'
}
# Resting on the tile whose middle is at x $1, the tooltip saying $2 hangs from it. $3 is "mirrored"
# when it must go left of the tile for want of room at its right, else it goes right.
hangs() {
  send move $1 110
  expect "^browser: tooltip shown $2\$" "resting on the tile at $1 didn't show \"$2\""
  expect '^browser: tooltip window shown$' "the tooltip of \"$2\" didn't fade in"
  expect '^browser: tooltip drawn ' "the tooltip of \"$2\" wasn't drawn"
  asked=$(since_move '^browser: tooltip at ')
  placed=$(since_move '^browser: tooltip window at ')
  drawn=$(since_move '^browser: tooltip drawn ')
  echo "$asked $placed $drawn" | awk -v tile=$1 -v window=$(drawn_width) -v mirrored="${3:-}" '
    { wide = $6
      left = tile
      if (tile + wide > window - 8 && tile - wide >= 8) left = tile - wide
      if (left > window - wide - 8) left = window - wide - 8
      if (left < 8) left = 8
      along = $4 - left; across = $4 - $12; down = $5 - $13; same = $6 - $17
      corner = (mirrored == "mirrored") ? $4 + wide - tile : $4 - tile }
    END { exit !(NR == 1 && NF == 17 && along > -1 && along < 1 && across > -1 && across < 1 \
      && down > -1 && down < 1 && same > -1 && same < 1 && corner > -1 && corner < 1) }' \
    || fail "the tooltip of \"$2\" isn't hung from the tile at $1: asked \"$asked\", placed \"$placed\", drawn \"$drawn\", window $(drawn_width)"
  width=$(echo "$asked" | cut -d' ' -f6)
}
hangs 41 "$long"
long_width=$width
hangs 114 "$medium"
medium_width=$width
hangs 187 "$short"
short_width=$width
[ "$(echo "$long_width > $medium_width && $medium_width > $short_width" | bc -l)" = 1 ] ||
  fail "the tooltips aren't long, medium and short: $long_width, $medium_width, $short_width"
../tools/window-screenshot.sh "$pid" "$out/favorite-tooltips.png" || fail "no window to capture"
hangs 114 "$medium"
hangs 41 "$long"
# In a window narrow enough that the short tooltip would cross its right edge from the last tile,
# its top right corner is under the tile's middle instead. The window may not go so narrow.
send resize 320 ${size#* }
for _ in $(seq 30); do
  [ "$(drawn_width | cut -d. -f1)" -le 322 ] && break
  sleep 0.1
done
if [ "$(drawn_width | cut -d. -f1)" -le 322 ]; then
  hangs 187 "$short" mirrored
  ../tools/window-screenshot.sh "$pid" "$out/favorite-tooltip-mirrored.png" || fail "no window to capture"
else
  echo "note: the window didn't narrow to 320, so the mirrored tooltip wasn't checked."
fi
send resize $size
expect "^browser: drawn at ${size/ /x}\$" "the window didn't draw back at its size"
# Settings kept the time, the haptics and the warning across the quit. ⌘, from the UI opens it,
# its close button hides it, and More Settings… opens Chromium's settings in a tab.
send click $sidebar_point
send key , cmd
expect '^browser: command settings via ui$' "⌘, from the UI didn't run Settings"
expect '^browser: settings shown General$' "⌘, from the UI didn't show Settings"
post clickkey 500 168
expect '^browser: menu shown 12 hours (default), 24 hours (checked), 7 days, 30 days$' \
  "Settings didn't keep 24 hours across the quit"
send choose "12 hours (default)"
expect '^browser: archive tabs after 12 hours (default)$' "choosing 12 hours didn't set it"
post clickkey 500 213
expect "^browser: menu shown In Every Space, In Each Profile's Spaces, In Each Space (checked)\$" \
  "Settings didn't keep Each Space across the quit"
send choose "In Each Profile's Spaces"
expect "^browser: favorites scope In Each Profile's Spaces\$" "choosing Each Profile's Spaces didn't set it"
send clickkey 430 60
expect '^browser: settings tab Advanced$' "a click on Advanced didn't show its tab"
send clickkey 592 126
expect '^browser: haptics on$' "Settings didn't keep the haptics off across the quit"
# The driver's done for a click that closes its window can be lost with the window, as it always
# was in a VM, so the click isn't waited for; Settings going is.
post clickkey 16 16
expect '^browser: settings hidden$' "the close button didn't hide Settings"
send menu Settings…
expect '^browser: settings shown Advanced$' "Settings… didn't show Settings on the tab it showed"
send clickkey 120 169
expect '^browser: more settings$' "a click on More Settings… didn't run it"
expect '^browser: settings hidden$' "More Settings… didn't hide Settings"
expect '^browser: loaded [0-9]* chrome://settings/$' \
  "More Settings… didn't open Chromium's settings"
# The warning stayed off, so ⌘Q quits without asking. It's the driver's last command.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited
grep -q '^browser: dialog shown Quit' "$log" && fail "⌘Q asked first with the warning off"
echo "ok: Add to Favorites in a tab's menu makes the tab a tile above the Space's rows, which names itself in a tooltip, which other apps don't take for the window in front, as the Library and sidebar buttons do with their keys, which a click shows and ⌘1–⌘9 count first, with an edge under them while the list is scrolled, which scrolls to the row of a tab shown under its bottom. A double-click on the tile takes it back to its URL, and ⌘W unloads it and keeps the tile. Its menu duplicates it into Today, and after a quit the favorite comes back, shown, and Remove from Favorites puts it at the top of Today. Rows dragged onto the tiles turn into tiles, and tiles dragged into the list into rows. Rows dragged onto the tiles become favorites where they're dropped, tiles reorder and go into Today, and Escape puts a dragged tile back. Tiles taller than 40 % of the window scroll with the list, under the URL field. ⌘, from the page and the UI opens Settings, whose switches set the warning before quitting and the haptics and whose pop-ups set the archive's time and who shares the favorites (Each Space, with the favorite kept in it across the quit, and back), all kept across a quit, when ⌘Q quits without asking; ⌘W and its close button hide it, and More Settings… opens Chromium's settings. Three tiles with a long, a medium and a short title each hang their tooltip's square corner from their own middle, along them and back, drawn as wide as it was placed for, and mirrored near the window's right edge. Windows: $out/favorite-tooltip.png, $out/favorite-tooltips.png, $out/favorite-tooltip-mirrored.png, $out/library-tooltip.png, $out/header-tooltip.png, $out/favorites.png, $out/scroll-edge.png, $out/list-scrolled.png, $out/favorite-unloaded.png, $out/favorites-restored.png, $out/favorite-drag.png, $out/favorite-drag-out.png, $out/tiles-tall.png, $out/tiles-scrolled.png, $out/settings.png"
