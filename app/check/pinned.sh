# ⌘D pins the tab shown, from the page and from the UI, at the bottom of the pinned tabs, and unpins
# it to the top of Today. ⌘1–⌘9 count the pinned tabs first. ⌘W, the ✕ under the pointer, which is
# − on the tab shown, or a middle-click on its row, unloads a pinned tab and keeps its row, which
# has no ✕ then, and shows the top Today tab, else the nearest pinned tab still loaded, else no
# page, as closing the last Today tab does. Shown again, the tab loads. A link in its page takes it
# away from its pinned URL, and ⌘↩ from the page and the UI, and a click on its favicon, go back
# there. An address typed into it opens in a Today tab of its own, as in Arc, and closing that shows
# the pinned tab again. Launched again, the app brings back the pinned tab pinned and
# away from its pinned URL, without loading it. Dragging a row moves its tab in the pinned tabs,
# pins it and unpins it, without showing it, and escape puts it back. New Folder makes a folder
# under the pinned tabs, which takes the name typed. A row dragged right under it goes in and
# dragged left comes out, a folder drags as a row, and a click closes and opens it. Launched again,
# the app brings it back closed, a double-click renames it, and Delete Folder in its menu leaves its
# tab where it was. Links and files dragged onto the list from the page or another app open as tabs
# where the pointer lets go, and dragged text that isn't a link opens a search for it there, but a
# link that isn't a web link doesn't open. The chevron after the Space's name collapses the pinned
# rows but the tab shown's, which the tabs' order then skips, and the command bar and the View menu
# expand and collapse them. Launched again, the app brings them back collapsed.

# Pinned tabs, from the served page, whose favicon fades once its tab is unloaded.
serve
log="$out/pinned.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$browser" < "$out/drive" > "$log" 2>&1 &
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

# Dragging a row moves its tab, in the pinned tabs and between them and Today, and its press shows
# it, as Arc's does. Today's top row is under the pinned rows, the divider and "+ New Tab".
today_top="100 $((90 + 44 + 41 * 2 + 15 + 41 + 20))"
clicks=$(detents)
send drag $(pinned_row 1) 100 150
expect "^browser: dragged $a to pinned 0\$" "a drag of the second pinned tab up didn't move it"
# It passed one row, so the trackpad clicked once, and letting go added none.
passed=$(($(detents) - clicks))
[ "$passed" = 1 ] || fail "a row dragged past one gave $passed clicks"
expect_lists "$a $c" "$b" "after dragging a pinned tab up"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "pressing the row dragged didn't show its tab"
# In steps, as the pointer goes: Today's tab comes away, and over the pinned rows, which stay put,
# a line marks where it would go.
clicks=$(detents)
send press $today_top
none_since '^browser: detent$' || fail "pressing a row clicked the trackpad"
send dragto 100 270
send dragto 100 230
send dragto 100 200
../tools/window-screenshot.sh "$pid" "$out/drag.png" || fail "no window to capture"
none_since '^browser: dragged' || fail "a row dropped before the pointer let go"
[ "$(detents)" -gt "$clicks" ] || fail "a row dragged up past the pinned rows didn't click"
# Holding the row at its place doesn't click again, nor does letting go.
send dragto 100 201
none_since '^browser: detent$' || fail "a row held at its place clicked again"
send release 100 200
none_since '^browser: detent$' || fail "letting go of a row clicked the trackpad"
expect "^browser: dragged $b to pinned 1\$" "a drag of the Today tab up didn't pin it"
expect_lists "$a $b $c" " *" "after dragging the Today tab into the pinned tabs"
# The rows make room only once it's let go, as Arc's do, so the next drag waits for them.
expect '^browser: rows settled$' "the pinned rows didn't settle after the drop"
send drag $(pinned_row 1) 100 300
expect "^browser: dragged $b to today 0\$" "a drag of a pinned tab down didn't unpin it"
expect_lists "$a $c" "$b" "after dragging a pinned tab into Today"
send drag $(pinned_row 0) 100 200
expect_lists "$c $a" "$b" "after dragging the first pinned tab down"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "pressing the row dragged didn't show its tab"
# The press now shows its row's tab, so it waits for the rows to stop under the pointer.
expect '^browser: rows settled$' "the pinned rows didn't settle after the drag down"
# Escape puts a dragged row back.
send press $(pinned_row 0)
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "pressing a row didn't show its tab"
send dragto 100 250
send key escape
expect '^browser: command stop via ' "escape during a drag didn't reach the UI"
clicks=$(detents)
send release 100 250
[ "$(detents)" = "$clicks" ] || fail "letting go after escape clicked the trackpad"
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
expect "^browser: hover tab $c close 1 favicon none\$" "the pinned tab's row didn't draw its ✕"
send click $(pinned_close 0)
expect '^browser: command close-tab via sidebar$' "a click on a pinned tab's ✕ didn't run Close Tab"
expect "^browser: page $c unloaded 1\$" "the ✕ didn't unload the pinned tab"
none_since '^browser: selected' || fail "unloading a pinned tab not shown changed the tab shown"
expect "^browser: hover tab $c close 0 favicon none\$" "the unloaded pinned tab's row kept its ✕"
send click $(pinned_close 0)
expect "^browser: selected $c in " "a click where an unloaded pinned tab's ✕ was didn't show the tab"
none_since '^browser: command close-tab' || fail "an unloaded pinned tab showed its ✕"
expect "^browser: loaded $c https://example.org/\$" "the pinned tab didn't load once shown"
# Shown, its ✕ is −, which unloads it too and shows the top Today tab.
expect "^browser: hover tab $c close − favicon none\$" "the pinned tab shown didn't draw −"
send click $(pinned_close 0)
expect '^browser: command close-tab via sidebar$' "a click on a pinned tab's − didn't run Close Tab"
expect "^browser: page $c unloaded 1\$" "the − didn't unload the pinned tab"
expect "^browser: selected $b\$" "the − on a pinned tab didn't show the top Today tab"
[ "$(pinned)" = "$c $a" ] || fail "the − took the pinned tab's row: pinned tabs are \"$(pinned)\""
send click $(pinned_row 0)
expect "^browser: selected $c in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $c https://example.org/\$" "the pinned tab didn't load once shown"
# A middle-click on a row closes its tab as the ✕ does, which unloads a pinned one.
send middleclick $(pinned_row 1)
expect '^browser: command close-tab via sidebar$' "a middle-click on a pinned tab didn't run Close Tab"
expect "^browser: page $a unloaded 1\$" "a middle-click didn't unload the pinned tab"
none_since '^browser: selected' || fail "a middle-click on a pinned tab not shown changed the tab shown"
send click $(pinned_row 1)
expect "^browser: selected $a in " "a click on the pinned tab a middle-click unloaded didn't show it"
expect "^browser: loaded $a $site\$" "the pinned tab a middle-click unloaded didn't load once shown"

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

# ⌘D unpins a tab to the top of Today. The served page stays pinned, for its link, and the names
# swap with it: from here $c is the pinned tab, the served page, and $a the Today tab.
send click $(pinned_row 0)
expect "^browser: selected $c in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $c https://example.org/\$" "the unloaded pinned tab didn't load once shown"
send key d cmd
expect_lists "$a" "$c" "after ⌘D on a pinned tab"
unpinned=$c
c=$a
a=$unpinned

# Away from its pinned URL, ⌘↩ goes back there, from the page and the UI. The click comes under
# the pointer that pressed the row ⌘D unpinned, as a trackpad tap does, while that row may still be
# drawn there, and shows the tab that rests there.
send click $(pinned_row 0)
expect "^browser: selected $c in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $c $site\$" "the unloaded pinned tab didn't load once shown"
# The served page's link, 200 by 40 points, 100 in from the card's top left, which is 228 and 10 in
# from the window's (site.sh).
link_point="430 130"
# Away from it by its page's link.
link_away() {
  send click $link_point
  expect "^browser: loaded $c ${site}notes.txt\$" "the page's link didn't load in the pinned tab"
  expect "^browser: page $c away 1\$" "a pinned tab at another page didn't read as away from its pinned URL"
}
# Away from it, then back with ⌘↩ from side $1, which a click gives the keyboard.
link_away_and_back() {
  link_away
  if [ "$1" = ui ]; then
    send click $sidebar_point
  else
    send click $page_point
  fi
  send key return cmd
  expect "^browser: command back-to-pinned-url via $1\$" "⌘↩ with the $1 focused didn't run Back to Pinned URL"
  expect "^browser: loaded $c $site\$" "⌘↩ with the $1 focused didn't load the pinned URL"
  expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
}
link_away_and_back page
link_away_and_back ui
# An address typed into it opens in a tab it opened, at the top of Today, shown, and the pinned tab
# keeps its page. Closing that tab shows the pinned one again, its opener.
go_from_bar example.com
expect "^browser: tab [0-9]* opened by $c in the foreground\$" \
  "an address typed into the pinned tab didn't open a tab of its own"
typed=$(grep '^browser: tab [0-9]* opened by' "$log" | tail -1 | cut -d' ' -f3)
expect_lists "$c" "$typed $a" "after an address typed into the pinned tab"
expect "^browser: selected $typed\( in .*\)\{0,1\}\$" "the tab an address typed into the pinned tab opened wasn't shown"
expect "^browser: loaded $typed https://example.com/\$" "the address typed into the pinned tab didn't load in its tab"
none_since "^browser: loaded $c " || fail "the address typed into the pinned tab loaded in it"
send key w cmd
expect "^browser: tab $typed closed\$" "⌘W didn't close the tab the pinned tab opened"
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "closing the tab the pinned tab opened didn't show the pinned tab"
link_away
# The quit leaves Today's tab shown, so the pinned tab comes back unloaded.
switch "2 cmd" "$a"
# Its favicon under the pointer offers the way back.
send move $(pinned_favicon 0)
expect "^browser: hover tab $c close [01] favicon $c\$" \
  "the pointer on the pinned tab's favicon wasn't drawn"
../tools/window-screenshot.sh "$pid" "$out/pinned-away.png" || fail "no window to capture"
quit_by_key
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the pinned tab comes back pinned, at its page, unloaded, and away from its pinned
# URL. A click on its favicon goes back there and shows it.
log="$out/pinned-restored.log"
step=0
BROWSER_DRIVE=1 "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: pinned [0-9][0-9]*$' || fail "the pinned tab didn't come back pinned"
await '^browser: today [0-9][0-9]*$' || fail "the Today tab didn't come back"
c=$(pinned)
a=$(today)
await "^browser: selected $a\$" || fail "the relaunch didn't show the tab shown last"
await "^browser: loaded $a https://example.org/\$" || fail "the tab shown didn't load"
await "^browser: page $c back 1 forward 0 loading 0 ${site}notes.txt\$" ||
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
expect "^browser: hover tab $c close [01] favicon $c\$" \
  "the pointer on the pinned tab's favicon wasn't drawn"
send click $(pinned_favicon 0)
expect '^browser: command back-to-pinned-url via sidebar$' "a click on an away pinned tab's favicon didn't run Back to Pinned URL"
expect "^browser: selected $c in " "a click on the pinned tab's favicon didn't show it"
expect "^browser: loaded $c $site\$" "a click on the pinned tab's favicon didn't load its pinned URL"
expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
none_since "^browser: loaded $c ${site}notes.txt" || fail "the unloaded tab loaded its old page on the way back"

# New Folder makes a folder under the pinned tabs with its name's field, which takes what's typed.
# The pinned tabs and folders the app last drew: each folder with + if it's open, else -, and
# what's in it in brackets.
outline() {
  grep '^browser: outline' "$log" | tail -1 | sed 's/^browser: outline *//'
}
# Waits for the rows moving since the last outline to come to rest, so a press lands on the row
# drawn there.
settled() {
  for _ in $(seq 300); do
    awk '/^browser: outline/ { s = 0 } /^browser: rows settled/ { s = 1 } END { exit !s }' \
      "$log" && return 0
    sleep 0.1
  done
  fail "the rows didn't come to rest"
}
send menu New Folder…
expect '^browser: command new-folder via ui$' "New Folder in the Tabs menu didn't run new-folder"
expect '^browser: rename folder [0-9]*$' "New Folder didn't give the folder's name field the keyboard"
f=$(grep '^browser: rename folder' "$log" | tail -1 | cut -d' ' -f4)
[ "$(outline)" = "$c $f+[]" ] || fail "New Folder didn't make an open folder under the pinned tab: $(outline)"
send type work
send key return
expect "^browser: renamed folder $f work\$" "the new folder didn't take the name typed"
# Dragged right of where it was, under the folder, a pinned tab goes in, and dragged left of it,
# out. A folder drags as a row does.
send press $(pinned_row 0)
send dragto 110 180
expect '^browser: detent$' "a row dragged under the folder and in didn't click the trackpad"
send dragto 120 200
none_since '^browser: detent$' || fail "a row held in the folder clicked again"
send release 120 200
expect "^browser: dragged $c to folder $f 0\$" "a pinned tab dragged right under the folder didn't go in"
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged in isn't in the folder: $(outline)"
settled
send press $(pinned_row 1)
send dragto 90 197
expect '^browser: detent$' "a row dragged left out of the folder didn't click the trackpad"
send dragto 76 199
none_since '^browser: detent$' || fail "a row held out of the folder clicked again"
send release 76 199
expect "^browser: dragged $c to pinned 1\$" "a tab dragged left out of the folder didn't come out"
expect "^browser: outline $f+\\[\\] $c\$" "the tab dragged out is still in the folder: $(outline)"
settled
send drag $(pinned_row 0) 100 200
expect "^browser: dragged $f to pinned 1\$" "a drag of the folder down didn't move it"
# Its press closed it, as a press on a folder does, and a click opens it again.
expect "^browser: outline $c $f-\\[\\]\$" \
  "the folder dragged down isn't under the tab, closed: $(outline)"
settled
send click $(pinned_row 1)
expect "^browser: outline $c $f+\\[\\]\$" "a click on the folder dragged didn't open it: $(outline)"
# It's empty, so no row moves to wait for.
send press $(pinned_row 0)
send dragto 110 180
expect '^browser: detent$' "a row dragged under the folder and in didn't click the trackpad"
send dragto 120 200
none_since '^browser: detent$' || fail "a row held in the folder clicked again"
send release 120 200
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged under the folder didn't go in: $(outline)"
settled
none_since '^browser: selected' || fail "dragging into and out of a folder showed a tab"
../tools/window-screenshot.sh "$pid" "$out/folder.png" || fail "no window to capture"
# A click closes the folder and another opens it, recorded for its motion: closing critically
# damped, opening on its ease-out, neither overshooting. With the Today tab shown, the folder's tab
# isn't, so it folds away and the rows under it slide up, then back down.
switch "9 cmd" "$a"
record folder-motion 3
send click $(pinned_row 0)
expect '^browser: command toggle-folder via sidebar$' "a click on the folder didn't run toggle-folder"
expect "^browser: outline $f-\\[$c\\]\$" "a click on the open folder didn't close it: $(outline)"
# Each half plays out in the film before the next click.
settled
send click $(pinned_row 0)
expect "^browser: outline $f+\\[$c\\]\$" "a click on the closed folder didn't open it: $(outline)"
settled
recorded
# The command bar finds commands by their names' letters in order, those that start with what's
# typed first, and one found by its letters alone last, after the suggestions. ↩ runs the command
# selected.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type clear
expect '^browser: command bar commands "clear": Clear Today, Clear Cache and Refresh, Clear Cookies and Refresh, Clear Archive, Collapse All Folders$' "the command bar didn't find the four commands that start with \"clear\", then Collapse All Folders: $(grep '^browser: command bar commands' "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/command-bar-commands.png" || fail "no window to capture"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar again"
send type cllps
expect '^browser: command bar commands "cllps": Collapse All Folders$' "the command bar didn't find Collapse All Folders for \"cllps\""
send key pagedown
expect '^browser: command bar selected Collapse All Folders$' "⇟ didn't select Collapse All Folders, the last row"
send key return
expect '^browser: command bar runs Collapse All Folders$' "↩ didn't run Collapse All Folders"
expect '^browser: command collapse-folders via command bar$' "Collapse All Folders didn't run collapse-folders"
expect "^browser: outline $f-\\[$c\\]\$" "Collapse All Folders didn't close the folder: $(outline)"
expect '^browser: new tab plate 0$' "running a command left New Tab's plate"
quit_by_key
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the folder comes back closed with its tab. A double-click renames it, and Delete
# Folder in its menu leaves its tab where it was.
log="$out/folder-restored.log"
step=0
BROWSER_DRIVE=1 "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: outline [0-9]*-\[[0-9]*\]$' || fail "the folder didn't come back closed with its tab: $(outline)"
f=$(outline | cut -d- -f1)
c=$(outline | sed 's/.*\[\(.*\)\]/\1/')
# Ids are new after a launch.
await '^browser: today [0-9][0-9]*$' || fail "the Today tab didn't come back"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored folder. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The first click opens it.
send doubleclick $(pinned_row 0)
expect "^browser: rename folder $f\$" "a double-click on the folder didn't open its name field"
expect "^browser: outline $f+\\[$c\\]\$" "the double-click's first click didn't open the folder: $(outline)"
send type later
send key return
expect "^browser: renamed folder $f later\$" "the folder didn't take the name typed over its old one"
../tools/window-screenshot.sh "$pid" "$out/folder-restored.png" || fail "no window to capture"
post rightclick $(pinned_row 0)
expect "^browser: menu shown $(cased 'Rename Folder, Delete Folder')\$" "the folder's menu isn't Rename Folder, Delete Folder"
send choose "$(cased 'Delete Folder')"
expect '^browser: command delete-folder via tab menu$' "Delete Folder didn't run delete-folder"
expect "^browser: outline $c\$" "Delete Folder didn't leave its tab in its place: $(outline)"
# The toast offers Undo; ⌘Z remakes the folder around its tab, under the name it had, and ⇧⌘Z
# deletes it again.
expect '^browser: toast shown Folder Deleted \[undo\]$' "Delete Folder didn't offer Undo"
send key z cmd
expect '^browser: command undo via \(ui\|page\)$' "⌘Z didn't undo Delete Folder"
expect "^browser: outline [0-9][0-9]*[-+]\\[$c\\]\$" "⌘Z didn't put the folder back around its tab: $(outline)"
send key z cmd shift
expect '^browser: command redo via \(ui\|page\)$' "⇧⌘Z didn't redo Delete Folder"
expect "^browser: outline $c\$" "⇧⌘Z didn't delete the folder again: $(outline)"

# A file dragged over the pinned tab gets a line above it, and let go, it opens there pinned and
# shows, titled by its name.
t=$(today)
../tools/window-screenshot.sh "$pid" "$out/drop-before.png" || fail "no window to capture"
send dragover 100 150 file "$(cd "$out" && pwd)/site/notes.txt"
expect '^browser: drag over HostUIView taken$' "the list didn't take a file dragged over it"
expect '^browser: drop line in pinned 0$' "no line marked the file's place above the pinned tab"
../tools/window-screenshot.sh "$pid" "$out/drop-line.png" || fail "no window to capture"
send dragdrop 100 150
expect '^browser: drag dropped$' "the list didn't take the file let go over it"
expect '^browser: dropped 1 in pinned 0$' "the file didn't drop above the pinned tab"
expect_lists "[0-9][0-9]* $c" "$t" "after dropping a file"
e=$(pinned | cut -d' ' -f1)
expect "^browser: selected $e\$" "the dropped file's tab didn't show"
expect "^browser: loaded $e file:///.*/site/notes.txt\$" "the dropped file didn't load in its tab"
# A link let go under the Today tab, whose row is centred at 292 under the two pinned tabs', the
# divider and New Tab's, opens there.
send dragdrop 100 330 url "${site}notes.txt"
expect '^browser: dropped 1 in today 1$' "the link didn't drop under the Today tab"
expect_lists "$e $c" "$t [0-9][0-9]*" "after dropping a link"
d=$(today | cut -d' ' -f2)
expect "^browser: selected $d\$" "the dropped link's tab didn't show"
expect "^browser: loaded $d ${site}notes.txt\$" "the dropped link didn't load in its tab"
../tools/window-screenshot.sh "$pid" "$out/dropped.png" || fail "no window to capture"
# Text that isn't a web link isn't taken, and nothing opens.
send dragover 100 300 text "javascript:document.title='dropped'"
expect '^browser: drag over HostUIView refused$' "the list took a javascript: URL dragged as text"
send dragleave
none_since '^browser: dropped' || fail "dragged text that isn't a web link opened"
# Other text, let go under the dropped link's row, centred at 332, is searched for there with the
# default engine, its spaces run together.
send dragdrop 100 345 text "dogs  and   cats"
expect '^browser: dropped 1 in today 2$' "the dropped text wasn't taken under the dropped link"
expect_lists "$e $c" "$t $d [0-9][0-9]*" "after dropping text"
s=$(today | cut -d' ' -f3)
expect "^browser: selected $s\$" "the dropped text's search didn't show"
expect "^browser: loaded $s https://duckduckgo.com/?q=dogs+and+cats" \
  "the dropped text didn't load a search for it: $(grep "^browser: loaded $s " "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/dropped-text.png" || fail "no window to capture"

# The chevron after the Space's name, there while the pointer is over the sidebar, collapses the
# pinned rows as it's pressed, and the name turns bold: they fold away, but for the tab shown's, as
# a closed folder's do, and ⌘1–⌘9 count Today's tabs first. The command bar's Expand Pinned Tabs
# brings them back, and View › Collapse Pinned Tabs folds them again.
# The middle of the chevron the app last drew for pinned rows open or shut ($1).
caret() {
  grep "^browser: pinned caret $1 at " "$log" | tail -1 |
    awk '{ printf "%d %d", $6 + $8 / 2, $7 + $9 / 2 }'
}
[ -n "$(caret open)" ] || fail "the Space with pinned tabs drew no chevron after its name"
away
send move $(caret open)
expect '^browser: sidebar hovered 1$' "the sidebar didn't see the pointer come over the chevron"
expect '^browser: sidebar tools shown$' "the chevron didn't fade in with Clear"
../tools/window-screenshot.sh "$pid" "$out/pinned-caret.png" || fail "no window to capture"
send click $(caret open)
expect '^browser: command collapse-pinned via sidebar$' "a click on the chevron didn't run collapse-pinned"
expect "^browser: outline -\\[$e $c\\]\$" "the chevron didn't collapse the pinned rows: $(outline)"
expect '^browser: pinned caret shut at ' "the chevron didn't turn for collapsed rows"
settled
../tools/window-screenshot.sh "$pid" "$out/pinned-collapsed.png" || fail "no window to capture"
switch "1 cmd" "$t"
bar_command expand-pinned "Expand Pinned Tabs" "expand pinned"
expect "^browser: outline $e $c\$" "Expand Pinned Tabs didn't bring the pinned rows back: $(outline)"
expect '^browser: pinned caret open at ' "the chevron didn't turn back for open rows"
settled
send menu Collapse Pinned Tabs
expect '^browser: command collapse-pinned via ui$' "View › Collapse Pinned Tabs didn't run collapse-pinned"
expect "^browser: outline -\\[$e $c\\]\$" "Collapse Pinned Tabs didn't collapse the pinned rows: $(outline)"
quit_by_key
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the pinned rows come back collapsed, with the chevron, which expands them.
log="$out/pinned-collapsed.log"
step=0
BROWSER_DRIVE=1 "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: outline -\[[0-9]* [0-9]*\]$' || fail "the pinned rows didn't come back collapsed: $(outline)"
await '^browser: pinned caret shut at ' || fail "the collapsed rows came back without the chevron"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored pinned rows. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
send click $(caret shut)
expect '^browser: command expand-pinned via sidebar$' "a click on the chevron didn't run expand-pinned"
expect '^browser: outline [0-9]* [0-9]*$' "the chevron didn't expand the restored pinned rows: $(outline)"
settled
../tools/window-screenshot.sh "$pid" "$out/pinned-expanded.png" || fail "no window to capture"
exec 3>&-
quit
echo "ok: ⌘D pins and unpins from the page and the UI, and ⌘1–⌘9 count pinned tabs first. A middle-click on a row closes its tab as its ✕ does. Dragging a row moves its tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back. ⌘W and ✕ unload a pinned tab and keep its row, showing the top Today tab, else a pinned tab still loaded, else no page, and an unloaded pinned tab loads once shown. Away from its pinned URL, ⌘↩ from the page and the UI, and a click on its favicon, go back there. Pinned tabs come back pinned, unloaded and at their page, still away. New Folder makes a folder its field names, a drag right and left takes a tab in and out of it, a folder drags as a row, a click closes and opens it, and the command bar finds Collapse All Folders, which closes it. It comes back closed, a double-click renames it, and Delete Folder leaves its tab in place. A link and a file dragged onto the list open as tabs where they're let go, Today or pinned, and show, text that isn't a link opens a search for it there, and a link that isn't a web link doesn't open. Delete Folder offers Undo, ⌘Z remakes the folder around its tab and ⇧⌘Z deletes it again. The chevron after the Space's name collapses the pinned rows, which ⌘1 then skips, Expand Pinned Tabs and Collapse Pinned Tabs in the command bar and the View menu bring them back and fold them, and they come back collapsed. Windows: $out/drag.png, $out/pinned.png, $out/pinned-unloaded.png, $out/pinned-away.png, $out/pinned-restored.png, $out/folder.png, $out/folder-restored.png, $out/folder-motion.mp4, $out/drop-before.png, $out/drop-line.png, $out/dropped.png, $out/dropped-text.png, $out/command-bar-commands.png, $out/pinned-caret.png, $out/pinned-collapsed.png, $out/pinned-expanded.png"
