# Launched again after ⌘Q, and again after its window's close button, the app brings back the
# Spaces: their names, colours and icons, their tabs with each tab's history, the tab each showed
# and the Space it showed, and the sidebar's width. Only a tab shown loads; the others show their
# titles and favicons. A tab named with a double-click on its row, where escape cancels, keeps its
# name over its page's title, and Rename Tab in its menu with no name gives the title back. An
# emoji from the character palette, after Change Icon…, takes its favicon's place until Remove
# Icon, as a Japanese conversion's first character does once it's committed. A Space's icon, from
# the same palette, comes back too. A tab closed before the quit stays closed,
# in the archive, from which the Library reopens it at its page. A page's downloads go in the
# check's folder, a new one's icon flies into the Library button, none of those a relaunch reads
# does, and ⇧⌘J opens the Library on Downloads, which lists them. With one running, ⌘Q
# asks first: Continue Downloading shows the Library's Downloads, and Quit quits, cancelling it.
# After the relaunch, Remove from List and Clear List take them off the list and keep their files.
# A new download's row dragged hands its file to the system's drag, the Library closing until it's
# let go. Resting on the Library button shows the newest finished downloads, and the menu of one's
# row there has Move to Trash, which puts its file in the Trash. A SIGTERM, which macOS sends at
# logout and restart, keeps a tab opened just before it. Reports how long putting them back takes.
# With the auto-archive's 12 hours made 8 s, a Today tab no window has shown for that long goes in
# the archive, counting the time across a quit, and the tab shown stays.

# The Spaces come back after a quit and after the window closes: their names, their tabs with each
# tab's history, the tab each showed and the Space the window showed. Only a tab shown loads, and
# one that doesn't shows its title and its favicon from the profile's history, which keeps none
# for data: pages, so the first page is served. A tab the user closed stays closed.
serve
log="$out/restore.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the restore test page didn't draw"
first=$(space)
a=$(today)
tab=$a
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "the served page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restoring Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The page's downloads go in the check's folder, and ⇧⌘J from the page opens the Library on
# Downloads, which lists them. The slow one runs until the quit.
send type d
expect '^browser: download [0-9]* done notes.txt$' "the page's download didn't finish"
[ -f "$out/downloads/notes.txt" ] || fail "the download isn't in the check's folder"
send type s
expect '^browser: download [0-9]* running slow.bin$' "the page's slow download didn't start"
# Its icon flies into the Library button.
d=$(grep -m1 '^browser: download [0-9]* running slow.bin$' "$log" | cut -d' ' -f3)
expect "^browser: download flying $d\$" "the new download's icon didn't fly"
../tools/window-screenshot.sh "$pid" "$out/download-flying.png" || fail "no window to capture"
expect "^browser: download landed $d\$" "the new download's icon didn't land"
send key j cmd shift
expect '^browser: command view-downloads via page$' "⇧⌘J with the page focused didn't reach the UI"
expect '^browser: library section downloads$' "⇧⌘J didn't open the Library on Downloads"
expect '^browser: library shown$' "⇧⌘J didn't open the Library"
../tools/window-screenshot.sh "$pid" "$out/downloads.png" || fail "no window to capture"
send key j cmd shift
expect '^browser: library hidden$' "⇧⌘J again didn't close the Library"
# Tab a goes to two pages and back to the first, so it has pages to go forward to.
go_from_bar example.com
expect_page example.com 1 0 "after loading a page"
go_from_bar example.org
expect_page example.org 1 0 "after loading another"
history_key left back page
expect_page example.com 1 1 "after going back"
history_key left back page
expect "^browser: page $a back 0 forward 1 loading 0 $site\$" \
  "tab a didn't go back to the first page"
open_from_bar example.net
c=$tab
open_from_bar example.org
send key w cmd
expect_tabs "$c $a" "$c" "after ⌘W closed the tab for example.org"
send menu New Space…
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
send type reading list
send key return
expect "^browser: create space $second reading list\$" "return didn't name the Space"
send key t cmd
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
e=$(today)
expect "^browser: loaded $e https://example.com/\$" "example.com didn't load in the new Space's tab"
post rightclick 100 112
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "the Space's menu didn't offer its colours"
send choose Pink
expect "^browser: colored space $second bf556a\$" "Pink didn't give the Space its colour"
../tools/window-screenshot.sh "$pid" "$out/space-color.png" || fail "no window to capture"
# Change Icon… in the Space's menu gives it an emoji from the icon picker's Emoji tab and the
# character palette, which a relaunch brings back.
pink=${colors/ (checked)/}
post rightclick 100 112
expect "^browser: menu shown Rename Space, Change Icon…, ${pink/Pink/Pink (checked)}, $profile, \
Delete Space\$" "the Space's menu didn't offer Change Icon…"
send choose Change Icon…
expect "^browser: icon picker for $second on icon\$" "Change Icon… didn't open the icon picker"
expect '^browser: icon picker shown$' "Change Icon… didn't show the icon picker"
# Emoji, the left half of Emoji | Icon at the picker's top.
send clickkey 92.5 39.5
expect '^browser: character palette shown$' "the Emoji tab didn't show the character palette"
send insert 🌱
expect "^browser: iconed space $second 🌱\$" \
  "an emoji from the palette didn't become the Space's icon"
../tools/window-screenshot.sh "$pid" "$out/space-icon.png" || fail "no window to capture"
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
expect_tabs "$c $a" "$c" "back in the first Space"
send drag 226 400 298 400
expect '^browser: sidebar width 300$' "dragging the sidebar's edge didn't make it 300 wide"
# A double-click on a tab's row turns its title into a field, all selected. Escape leaves the
# title, and return keeps the name typed, which the tab shows over its page's title.
send doubleclick $(row 0)
expect "^browser: rename tab $c\$" \
  "a double-click on the tab's row didn't turn its title into a field"
send type nope
send key escape
expect '^browser: rename cancelled$' "escape didn't cancel the tab's rename"
none_since "^browser: title $c nope" || fail "escape renamed the tab"
send doubleclick $(row 0)
expect "^browser: rename tab $c\$" "a second double-click on the row didn't rename the tab"
send type news
../tools/window-screenshot.sh "$pid" "$out/rename-tab.png" || fail "no window to capture"
send key return
expect "^browser: title $c news\$" "return didn't name the tab"
# Change Icon… in its menu puts a field in its favicon's place and asks for the character palette,
# and an emoji the palette commits there is the tab's icon at once.
post rightclick $(row 0)
expect '^browser: menu shown .*Rename Tab, Change Icon…, Duplicate Tab' \
  "a right-click on the tab's row didn't offer Change Icon…"
send choose Change Icon…
expect "^browser: change icon $c\$" "Change Icon… didn't put a field in the tab's icon's place"
expect '^browser: character palette shown$' "Change Icon… didn't show the character palette"
../tools/window-screenshot.sh "$pid" "$out/tab-icon-field.png" || fail "no window to capture"
send insert 🦊
expect "^browser: page $c icon 🦊\$" "an emoji from the palette didn't become the tab's icon"
../tools/window-screenshot.sh "$pid" "$out/tab-icon.png" || fail "no window to capture"
# With a download running, Quit in the alert ⌘Q shows asks again, in Chrome's. Continue Downloading
# keeps the browser running and shows the Library's Downloads, and Quit quits, cancelling it.
post key q cmd
expect '^browser: command quit via ' "⌘Q didn't reach the UI"
expect '^browser: alert shown Quit Browser?$' "⌘Q didn't ask before quitting"
post choose Quit
expect '^browser: modal shown Download is in progress, .*, Continue Downloading, Quit$' \
  "⌘Q with a download running didn't ask first"
send choose Continue Downloading
expect '^browser: command view-downloads via chrome$' \
  "Continue Downloading didn't show the downloads"
expect '^browser: library section downloads$' \
  "Continue Downloading didn't show the Library's Downloads"
../tools/window-screenshot.sh "$pid" "$out/quit-downloads.png" || fail "no window to capture"
post key q cmd
expect '^browser: alert shown Quit Browser?$' "⌘Q didn't ask before quitting"
post choose Quit
expect '^browser: modal shown Download is in progress, ' "⌘Q didn't ask again"
# Quit is the driver's last command: the app doesn't report it done.
post choose Quit
exec 3>&-
exited

# Launched again, the app shows the first Space with the tab for example.net, and tab a under it
# at the served page, unloaded.
log="$out/restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "the relaunch didn't bring back two Spaces"
first=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f3)
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
grep -q "^browser: added space $first Space 1\$" "$log" || fail "the first Space came back without its name"
grep -q "^browser: added space $second reading list\$" "$log" || fail "the second Space came back without its name"
grep -q "^browser: colored space $second bf556a\$" "$log" ||
  fail "the second Space came back without its colour"
grep -q "^browser: iconed space $second 🌱\$" "$log" ||
  fail "the second Space came back without its icon"
await "^browser: space $first\$" || fail "the relaunch didn't show the first Space"
# Without the tab closed before the quit.
await '^browser: today [0-9]* [0-9]*$' || fail "the first Space didn't come back with its two tabs"
c=$(today | cut -d' ' -f1)
a=$(today | cut -d' ' -f2)
await "^browser: selected $c\$" || fail "the first Space didn't show the tab it showed last"
await "^browser: loaded $c https://example.net/\$" || fail "the tab shown didn't load"
await "^browser: title $c news\$" || fail "the tab came back without its name"
grep -q "^browser: title $c Example Domain\$" "$log" &&
  fail "the tab's page's title took its name's place"
await "^browser: page $c icon 🦊\$" || fail "the tab came back without its icon"
await "^browser: page $a back 0 forward 1 loading 0 $site\$" ||
  fail "tab a didn't come back at its page with history: $(grep "^browser: page $a" "$log" | tail -1)"
await "^browser: title $a ready\$" || fail "tab a didn't come back with its title"
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "tab a didn't come back with its favicon"
grep -q "^browser: loaded $a " "$log" && fail "tab a loaded without being shown"
await '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' ||
  fail "the second Space's tab didn't come back at example.com"
e=$(grep -m1 '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' "$log" | cut -d' ' -f3)
grep -q "^browser: loaded $e " "$log" && fail "the second Space's tab loaded without being shown"
../tools/window-screenshot.sh "$pid" "$out/restored.png" || fail "no window to capture"
await '^browser: archive 1 tabs$' || fail "the relaunch didn't bring back the archived tab"
await '^browser: download [0-9]* done notes.txt$' || fail "the relaunch didn't list the download"
await '^browser: download [0-9]* cancelled slow.bin$' ||
  fail "the download the quit cancelled didn't come back cancelled"
# Newest first, so the first is this run's.
s=$(grep -m1 '^browser: download [0-9]* cancelled slow.bin$' "$log" | cut -d' ' -f3)
check_layout "after the relaunch, with the sidebar it had" 300
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restored history. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Rename Tab in its menu, with the name taken out, gives the tab back its page's title.
post rightclick $(row 0)
menu="Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Remove Icon, Duplicate Tab, Move to Space › reading list, Close Tab, Close Tabs Below"
expect "^browser: menu shown $menu\$" \
  "a right-click on the renamed tab's row didn't offer Rename Tab"
send choose Rename Tab
expect "^browser: rename tab $c\$" "Rename Tab didn't turn the tab's title into a field"
send key backspace
send key return
expect "^browser: title $c Example Domain\$" \
  "taking the name out didn't give the tab back its page's title"
# A composition isn't the icon until it's committed, and then its first character is.
post rightclick $(row 0)
expect "^browser: menu shown $menu\$" "a right-click on the tab's row didn't offer Remove Icon"
send choose Change Icon…
expect "^browser: change icon $c\$" "Change Icon… didn't put a field in the tab's icon's place"
send ime on
send type nihongo
send key space
[ "$marked" != none ] || fail "the icon's field didn't take the conversion"
none_since "^browser: page $c icon " || fail "a composition became the tab's icon"
send key return
expect "^browser: page $c icon 日\$" "the committed conversion's first character isn't the icon"
send ime off
# Remove Icon gives it back its favicon.
post rightclick $(row 0)
expect "^browser: menu shown $menu\$" "a right-click on the tab's row didn't offer Remove Icon"
send choose Remove Icon
expect "^browser: page $c icon none\$" "Remove Icon didn't give the tab back its favicon"
send click $page_point
# The tab closed before the quit reopens from the Library at the top of Today, at its page. ⌘W
# archives it again.
send key l cmd shift
expect '^browser: command view-library via page$' "⇧⌘L with the page focused didn't reach the UI"
expect '^browser: library shown$' "⇧⌘L didn't open the Library"
send click 300 111
expect '^browser: restore archived [0-9][0-9]*$' "a click on the archived tab didn't restore it"
expect "^browser: today [0-9][0-9]* $c $a\$" \
  "the archived tab didn't come back at the top of Today: Today is \"$(today)\""
r=$(today | cut -d' ' -f1)
expect "^browser: selected $r\$" "the tab from the archive isn't shown"
expect "^browser: loaded $r https://example.org/\$" "the tab from the archive didn't load its page"
expect '^browser: archive 0 tabs$' "the tab reopened stayed in the archive"
expect '^browser: library hidden$' "reopening a tab didn't close the Library"
send click $page_point
send key w cmd
expect_tabs "$c $a" "$c" "after closing the tab from the archive"
# The downloads the relaunch read didn't fly.
grep -q '^browser: download flying ' "$log" && fail "a download the relaunch read flew in"
# Resting on the footer's Library button shows the newest finished downloads over the tab list,
# here notes.txt alone, as the cancelled one isn't finished. They stay as the pointer comes onto
# them, and go once it's on neither.
send move 24 $((height - 25))
expect '^browser: recent downloads shown 1$' \
  "resting on the Library button didn't show the recent download"
send move 24 $((height - 45))
send move 120 $((height - 78))
none_since '^browser: recent downloads hidden$' ||
  fail "the recent downloads went as the pointer came onto them"
../tools/window-screenshot.sh "$pid" "$out/recent-downloads.png" || fail "no window to capture"
send move $page_point
expect '^browser: recent downloads hidden$' "the recent downloads stayed after the pointer left"
# Remove from List in the cancelled download's menu takes it off the list, and Clear List in the
# finished one's takes off every download that isn't running and keeps their files.
send key j cmd shift
expect '^browser: library section downloads$' "⇧⌘J didn't open the Library on Downloads"
expect '^browser: library shown$' "⇧⌘J didn't open the Library after the relaunch"
listed=$(grep '^browser: downloads ' "$log" | tail -1 | cut -d' ' -f3)
post rightclick 300 111
expect '^browser: menu shown Remove from List, Clear List$' \
  "a right-click on the cancelled download's row didn't show its menu"
send choose Remove from List
expect "^browser: download $s Remove via download menu\$" "Remove from List didn't reach it"
expect "^browser: downloads $((listed - 1))\$" "Remove from List didn't take the download off"
post rightclick 300 111
expect '^browser: menu shown Open, Show in Finder, Move to Trash, Remove from List, Clear List$' \
  "a right-click on the finished download's row didn't show its menu"
send choose Clear List
expect '^browser: command clear-downloads via download menu$' "Clear List didn't run"
expect '^browser: downloads 0$' "Clear List didn't empty the list"
[ -f "$out/downloads/notes.txt" ] || fail "Clear List deleted the download's file"
send key j cmd shift
expect '^browser: library hidden$' "⇧⌘J didn't close the Library"
switch "2 cmd" "$a"
expect "^browser: loaded $a $site\$" "tab a didn't load once shown"
# A finished download's row dragged hands its file to the system's drag and closes the Library,
# which comes back once it's let go where nothing takes it. Move to Trash in its menu then puts the
# file in the Trash and takes it off the list.
send click $page_point
send type q
trashed='^browser: download [0-9]* done trash.txt$'
expect "$trashed" "the page's second download didn't finish"
t=$(grep "$trashed" "$log" | tail -1 | cut -d' ' -f3)
send key j cmd shift
expect '^browser: library shown$' "⇧⌘J didn't open the Library on the second download"
press=$((step + 1))
send press 300 111
send dragto 300 140
expect "^browser: download $t drag\$" "dragging the download's row didn't start the system's drag"
send release 300 140
# No real button holds the system's drag, so it may end before the release reaches it.
dragged() {
  for _ in $(seq 100); do
    sed -n "/^browser: drive $press: /,\$p" "$log" | sed -n "/$1/,\$p" | grep -q "$2" && return 0
    sleep 0.1
  done
  return 1
}
dragged "download $t drag" '^browser: library hidden$' ||
  fail "the Library didn't close for the download's drag"
dragged "download $t drag" '^browser: file drag ended nowhere$' ||
  fail "the download's drag didn't end where nothing took it"
dragged 'file drag ended' '^browser: library shown$' ||
  fail "the Library didn't come back after the download's drag"
send key j cmd shift
expect '^browser: library hidden$' "⇧⌘J didn't close the Library after the download's drag"
# Its row in the recent downloads shows the Library row's menu, whose Move to Trash takes the rows
# away with the last of them.
send move 24 $((height - 25))
expect '^browser: recent downloads shown 1$' \
  "resting on the Library button didn't show the second download"
post rightclick 120 $((height - 78))
expect '^browser: menu shown Open, Show in Finder, Move to Trash, Remove from List, Clear List$' \
  "a right-click on the recent download's row didn't show its menu"
send choose Move to Trash
expect "^browser: download $t Trash via download menu\$" "Move to Trash didn't reach the download"
expect '^browser: downloads 0$' "Move to Trash didn't take the download off the list"
expect '^browser: recent downloads hidden$' "the recent downloads stayed with none left"
[ -e "$out/downloads/trash.txt" ] && fail "Move to Trash left the file in the check's folder"
send click $page_point
tab=$a
history_key right forward page
expect_page example.com 1 1 "going forward in tab a's restored history"
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect_tabs "$e" "$e" "in the second Space"
expect "^browser: loaded $e https://example.com/\$" "the second Space's tab didn't load once shown"
# The window's close button, which ends the app too: the driver doesn't report it done.
echo "click 20 24" >&3
exec 3>&-
await '^browser: window closed ' || fail "a click on the close button didn't close the window"
exited

# Closing the window kept them too, and the second Space shows.
log="$out/restored-again.log"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch after closing the window didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "closing the window didn't keep both Spaces"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
await "^browser: space $second\$" || fail "the launch after closing the window didn't show the second Space"
await '^browser: today [0-9]*$' || fail "the second Space didn't come back with its tab"
await "^browser: loaded $(today) https://example.com/\$" || fail "the second Space's tab didn't load"
await '^browser: page [0-9]* back 1 forward 1 loading 0 https://example.com/$' ||
  fail "tab a didn't keep the page it went forward to"
quit

# A SIGTERM ends the session at once, before the save the new tab scheduled, which the core writes
# as Chrome exits.
log="$out/session-ending.log"
BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch before the SIGTERM didn't restore the Spaces"
await "^browser: loaded [0-9]* $site\$" || fail "the page opened over the Spaces didn't load"
kill -TERM "$pid"
exited
[ "$(python3 -I -c 'import json, sys
print(*[len(space["today"]) for space in json.load(open(sys.argv[1]))["spaces"]])' "$saved")" = "2 2" ] ||
  fail "the tab opened before the SIGTERM wasn't kept"
restore_ms=$(sed -n 's/^browser: restored in \([0-9.]*\) ms$/\1/p' "$out/restored.log" "$out/restored-again.log" | tr '\n' ' ')

# Auto-archive, 8 s instead of 12 hours. Tab a, then b, opens over the Spaces, and a isn't seen
# after b shows. Past a's 8 s, the next launch archives a at once and b 8 s after c shows.
log="$out/archive-a.log"
rm -f "$saved"
BROWSER_URL="${site}?a" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await "^browser: loaded [0-9]* ${site}?a\$" || fail "tab a didn't load"
quit
log="$out/archive-b.log"
BROWSER_URL="${site}?b" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await "^browser: loaded [0-9]* ${site}?b\$" || fail "tab b didn't load over tab a"
seen=$(date +%s)
quit
wait=$((seen + 9 - $(date +%s)))
[ "$wait" -le 0 ] || sleep "$wait"
log="$out/archive-c.log"
start=$(date +%s)
BROWSER_URL="${site}?c" "$app/Contents/MacOS/Browser" --archive-tabs-after=8 < /dev/null \
  > "$log" 2>&1 &
pid=$!
await '^browser: archive 1 tabs$' || fail "tab a, unseen for 8 s across the quit, wasn't archived"
[ $(($(date +%s) - start)) -lt 6 ] || fail "tab a was archived 8 s after the launch, not at once"
await "^browser: loaded [0-9]* ${site}?c\$" || fail "tab c didn't load"
c=$(sed -n "s|^browser: loaded \([0-9]*\) ${site}?c\$|\1|p" "$log" | head -1)
await '^browser: archive 2 tabs$' || fail "tab b, unseen for 8 s after c showed, wasn't archived"
[ $(($(date +%s) - start)) -ge 8 ] || fail "tab b was archived before it was unseen for 8 s"
await "^browser: today $c\$" || fail "Today is \"$(today)\" after the auto-archive, not tab c alone"
quit
echo "ok: after ⌘Q and after closing the window, the Spaces come back with their names, colours, icons, tabs, history and the tab each showed, in the Space shown last, with the sidebar's width; only tabs shown load, the others show their titles and favicons, a tab renamed with a double-click keeps its name over its page's title until Rename Tab takes it out, an emoji from the character palette keeps its favicon's place until Remove Icon, and a closed tab stays closed, in the archive, from which it reopens at its page. A page's downloads are listed in the Library's Downloads, which ⇧⌘J opens, and a new one's icon flies into the Library button, though none of those a relaunch reads does. With one running, ⌘Q asks first, Continue Downloading shows Downloads and Quit cancels it, and after the relaunch Remove from List and Clear List take them off and keep their files. Resting on the Library button shows the newest finished downloads, until the pointer leaves them, and a row's menu is the Library row's. A SIGTERM keeps a tab opened just before it. Today tabs no window shows go in the archive after the interval, counted across a quit, and the tab shown stays."
echo "  Restore, 3 tabs in 2 Spaces: ${restore_ms}ms. Windows: $out/restored.png,"
echo "  $out/recent-downloads.png"
