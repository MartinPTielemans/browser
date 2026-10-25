# ⌘D pins the tab shown, from the page and from the UI, at the bottom of the pinned tabs, and unpins
# it to the top of Today. ⌘1–⌘9 count the pinned tabs first. ⌘W, or the ✕ under the pointer, unloads
# a pinned tab and keeps its row, which has no ✕ then, and shows the top Today tab, else the nearest
# pinned tab still loaded, else no page, as closing the last Today tab does. Shown again, the tab
# loads. Away from its pinned URL, ⌘↩ from the page and the UI, and a click on its favicon, go back
# there. Launched again, the app brings back the pinned tab pinned and away from its pinned URL,
# without loading it. Dragging a row moves its tab in the pinned tabs, pins it and unpins it,
# without showing it, and escape puts it back. New Folder makes a folder under the pinned tabs,
# which takes the name typed. A row dragged right under it goes in and dragged left comes out, a
# folder drags as a row, a click closes and opens it, and its tooltip is its name. Launched again,
# the app brings it back closed, a double-click renames it, and Delete Folder in its menu leaves its
# tab where it was. Links and files dragged onto the list from the page or another app open as tabs
# where the pointer lets go, and dragged text that isn't a web link doesn't.

# Pinned tabs, from the served page, whose favicon fades once its tab is unloaded.
serve
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
quit_by_key
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
send dragto 120 200
send release 120 200
expect "^browser: dragged $c to folder $f 0\$" "a pinned tab dragged right under the folder didn't go in"
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged in isn't in the folder: $(outline)"
settled
send press $(pinned_row 1)
send dragto 90 197
send dragto 76 199
send release 76 199
expect "^browser: dragged $c to pinned 1\$" "a tab dragged left out of the folder didn't come out"
expect "^browser: outline $f+\\[\\] $c\$" "the tab dragged out is still in the folder: $(outline)"
settled
send drag $(pinned_row 0) 100 200
expect "^browser: dragged $f to pinned 1\$" "a drag of the folder down didn't move it"
expect "^browser: outline $c $f+\\[\\]\$" "the folder dragged down isn't under the tab: $(outline)"
settled
send press $(pinned_row 0)
send dragto 110 180
send dragto 120 200
send release 120 200
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged under the folder didn't go in: $(outline)"
settled
none_since '^browser: selected' || fail "dragging into and out of a folder showed a tab"
../tools/window-screenshot.sh "$pid" "$out/folder.png" || fail "no window to capture"
# A click closes the folder and another opens it.
send click $(pinned_row 0)
expect '^browser: command toggle-folder via sidebar$' "a click on the folder didn't run toggle-folder"
expect "^browser: outline $f-\\[$c\\]\$" "a click on the open folder didn't close it: $(outline)"
send click $(pinned_row 0)
expect "^browser: outline $f+\\[$c\\]\$" "a click on the closed folder didn't open it: $(outline)"
# The folder's tooltip is its name once the pointer has left the row it clicked and come back, and
# it goes when the pointer leaves.
send move $(pinned_row 1)
send move $(pinned_row 0)
expect '^browser: tooltip work$' "the folder's tooltip isn't its name"
send move $(pinned_row 1)
expect '^browser: tooltip none$' "the folder's tooltip stayed after the pointer left it"
# The command bar finds commands by their names' letters in order, those that start with what's
# typed first, four at most, and one found by its letters alone after the search row. ↩ runs the
# command selected.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type clear
expect '^browser: command bar commands "clear": Clear Today, Clear Cache and Refresh, Clear Archive, Clear Cookies and Refresh$' "the command bar didn't find the four commands that start with \"clear\": $(grep '^browser: command bar commands' "$log" | tail -1)"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar again"
send type cllps
expect '^browser: command bar commands "cllps": Collapse All Folders$' "the command bar didn't find Collapse All Folders for \"cllps\""
send key down
expect '^browser: command bar selected Collapse All Folders$' "↓ didn't select Collapse All Folders, under the search row"
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
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
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
expect '^browser: menu shown Rename Folder, Delete Folder$' "the folder's menu isn't Rename Folder, Delete Folder"
send choose Delete Folder
expect '^browser: command delete-folder via tab menu$' "Delete Folder didn't run delete-folder"
expect "^browser: outline $c\$" "Delete Folder didn't leave its tab in its place: $(outline)"

# A file dragged over the pinned tab makes room for it there, and let go, it opens there pinned and
# shows, titled by its name.
t=$(today)
../tools/window-screenshot.sh "$pid" "$out/drop-before.png" || fail "no window to capture"
send dragover 100 150 file "$(cd "$out" && pwd)/site/notes.txt"
expect '^browser: drag over HostUIView taken$' "the list didn't take a file dragged over it"
expect '^browser: rows settled$' "the pinned tab didn't make room for the file"
../tools/window-screenshot.sh "$pid" "$out/drop-room.png" || fail "no window to capture"
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
exec 3>&-
quit
echo "ok: ⌘D pins and unpins from the page and the UI, and ⌘1–⌘9 count pinned tabs first. Dragging a row moves its tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back. ⌘W and ✕ unload a pinned tab and keep its row, showing the top Today tab, else a pinned tab still loaded, else no page, and an unloaded pinned tab loads once shown. Away from its pinned URL, ⌘↩ from the page and the UI, and a click on its favicon, go back there. Pinned tabs come back pinned, unloaded and at their page, still away. New Folder makes a folder its field names, a drag right and left takes a tab in and out of it, a folder drags as a row, a click closes and opens it, and the command bar finds Collapse All Folders, which closes it. It comes back closed, a double-click renames it, and Delete Folder leaves its tab in place. A link and a file dragged onto the list open as tabs where they're let go, Today or pinned, and show, and text that isn't a web link doesn't. Windows: $out/drag.png, $out/pinned.png, $out/pinned-unloaded.png, $out/pinned-away.png, $out/pinned-restored.png, $out/folder.png, $out/folder-restored.png, $out/drop-before.png, $out/drop-room.png, $out/dropped.png"
