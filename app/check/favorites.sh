# Add to Favorites in a Today tab's menu takes the tab out of Today into a tile above the Space's
# rows, without showing it. A click on the tile shows it, and ⌘1–⌘9 count favorites first. Away from
# its URL, a double-click on the tile goes back there. ⌘W unloads it and keeps the tile, showing the
# top Today tab. The tile's menu duplicates it to the top of Today. Launched again, the app brings
# the favorite back, shown, and Remove from Favorites in its menu puts it at the top of Today. Today
# rows dragged onto the tiles become favorites where they're dropped, a tile dragged across the
# other goes before it, one dragged into the list goes there, and escape puts a dragged tile back.
# ⌘, from the page and the UI opens Settings, where switches turn the warning before quitting and
# the haptics off and the menu of a pop-up button sets how long Today tabs wait for the archive, and
# all three stay after the quit, when ⌘Q quits without asking. ⌘W and the window's close button hide
# it, and More Settings… opens Chromium's settings in a tab.

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
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below (disabled)$' \
  "a right-click on a Today row didn't offer Add to Favorites"
send choose Add to Favorites
expect '^browser: command add-to-favorites via tab menu$' "Add to Favorites didn't run"
expect "^browser: favorites $a\$" \
  "Add to Favorites didn't make the tab a favorite: \"$(favorites)\""
expect "^browser: today $b\$" "the favorite is still in Today: Today is \"$(today)\""
none_since '^browser: selected' || fail "Add to Favorites showed another tab"
# A click on the tile shows the favorite, and ⌘1–⌘9 count favorites first.
send click $tile
expect '^browser: command select via favorite$' "a click on the tile didn't run"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
../tools/window-screenshot.sh "$pid" "$out/favorites.png" || fail "no window to capture"
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
send clickkey 398 60
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
../tools/window-screenshot.sh "$pid" "$out/favorite-drag.png" || fail "no window to capture"
none_since '^browser: dragged' ||
  fail "a row dragged onto the favorites dropped before the pointer let go"
send release 114 112
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
# A tile dragged left of the other goes before it.
send drag 168 110 40 110
expect "^browser: dragged $a to favorites 0\$" \
  "a tile dragged left of the other didn't go before it"
expect "^browser: favorites $a $x\$" "the tiles didn't swap: the favorites are \"$(favorites)\""
# A tile dragged into the list goes there, unpinned in Today.
send drag 168 110 100 240
expect "^browser: dragged $x to today 0\$" "a tile dragged into Today didn't go there"
expect "^browser: today $x $y\$" "the tile dropped in Today isn't at its top: Today is \"$(today)\""
expect "^browser: favorites $a\$" "the tile dropped in Today is still a favorite"
none_since '^browser: selected' || fail "dragging a tile showed it"
# Escape puts a dragged tile back.
send press 114 110
send dragto 100 200
send dragto 100 300
send key escape
expect '^browser: command stop via ' "escape during a tile's drag didn't reach the UI"
send release 100 300
none_since '^browser: dragged' || fail "a tile's drag escape put back dropped it"
[ "$(favorites)" = "$a" ] || fail "a tile's drag escape put back moved it: \"$(favorites)\""
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
send clickkey 398 60
expect '^browser: settings tab Advanced$' "a click on Advanced didn't show its tab"
send clickkey 592 126
expect '^browser: haptics on$' "Settings didn't keep the haptics off across the quit"
send clickkey 16 16
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
grep -q '^browser: alert shown Quit' "$log" && fail "⌘Q asked first with the warning off"
echo "ok: Add to Favorites in a tab's menu makes the tab a tile above the Space's rows, which a click shows and ⌘1–⌘9 count first. A double-click on the tile takes it back to its URL, and ⌘W unloads it and keeps the tile. Its menu duplicates it into Today, and after a quit the favorite comes back, shown, and Remove from Favorites puts it at the top of Today. Rows dragged onto the tiles become favorites where they're dropped, tiles reorder and go into Today, and Escape puts a dragged tile back. ⌘, from the page and the UI opens Settings, whose switches set the warning before quitting and the haptics and whose pop-up sets the archive's time, all kept across a quit, when ⌘Q quits without asking; ⌘W and its close button hide it, and More Settings… opens Chromium's settings. Windows: $out/favorites.png, $out/favorite-unloaded.png, $out/favorites-restored.png, $out/favorite-drag.png, $out/settings.png"
