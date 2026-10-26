# New Space in the Spaces menu makes a Space and shows it, empty and without a page, with a form
# that takes the keyboard from the page. Japanese input composes in its field, return while
# composing only commits, and return names the Space and sends the form away; ⌘T then opens a tab in
# it. ⌃1–⌃9, ⌥⌘← and ⌥⌘→, with the page or the UI focused, and clicks on the switcher's dots show
# another Space, which shows the tab it showed last. Two fingers across the sidebar page to the
# Space a flick or the pages' place points at, never past the last, fingers along it don't, and a
# wheel's tick across pages once. A tab's menu moves the tab to another Space, which the window
# shows with it. Return with no name does nothing, New Space from the footer's + shows a form
# already open, and Cancel, or two fingers back, deletes its Space and goes back to the Space it
# was made from; fingers going on rubber-band the form. A right-click on the Space's name or a
# dot shows Rename Space, Change Icon…, its colours and Delete Space; an emoji committed after
# Change Icon… is the Space's icon, in its dot's place, until Remove Icon. Rename, the Spaces
# menu's too, and a double-click on the name, turn it into a field, where a right-click shows the
# field's menu, return keeps the name and escape cancels. Delete asks first for a Space with tabs,
# in a dialog that a click around cancels and return confirms, and the Space goes once the dialog
# has, which a toast says. ⌘Z brings it back where it was, with its name and tabs. Its tabs close
# with it and reopen with ⇧⌘T in the Space shown, which ⌘Z then leaves out, and the last Space
# can't be deleted. The Spaces menu lists the Spaces, and its names show them.

# Spaces. A new one is empty, and each keeps its tabs and the one it showed last.
page='data:text/html,<title>loading</title>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/spaces.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Spaces test page didn't draw"
first=$(space)
await "^browser: spaces $first\$" || fail "the app didn't start with one Space"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The switcher's dots are centred under the sidebar, 25 above the window's bottom.
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
dots_y=$((${content#*x} - 25))

send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run new-space"
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: space $second in " "the window didn't show the new Space"
expect_tabs " *" 0 "in the new Space"
send layout
sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' | grep -q WebContentsViewCocoa &&
  fail "the new Space shows the first Space's page"
# The form's field has the keyboard, which the page had.
in_browser AccessKitSubclassOfGPUIView "in the new Space's form"
../tools/window-screenshot.sh "$pid" "$out/space-form.png" || fail "no window to capture"
send type reading
send key space
send ime on
send type nihongo
send key space
[ "$marked" = 8,3 ] || fail "the form's field didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/space-form-ime.png" || fail "no window to capture"
send key return
none_since '^browser: create space' || fail "return while composing named the Space"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send key return
expect "^browser: create space $second reading 日本語\$" "return didn't name the Space: $(grep '^browser: create space' "$log" | tail -1)"
expect "^browser: renamed space $second reading 日本語\$" "the core didn't rename the Space"
expect '^browser: space form gone$' "the form didn't leave after Create"
../tools/window-screenshot.sh "$pid" "$out/new-space.png" || fail "no window to capture"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in the new Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
b=$(today)
expect "^browser: selected $b\$" "the new Space's tab wasn't shown"
expect "^browser: loaded $b https://example.com/\$" "example.com didn't load in the new Space's tab"

# $1 is the driver's key or click, $2 the command it runs, $3 where the UI hears it from, and $4
# the Space it shows, with its tab $5.
show_space() {
  send $1
  expect "^browser: command $2 via $3\$" "$1 with the $3 focused didn't run $2"
  expect "^browser: space $4 in " "$1 with the $3 focused didn't show Space $4"
  # The menu shows the shortcut too, and a key the shell ran never reaches it.
  [ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: command ')" = 1 ] ||
    fail "$1 with the $3 focused ran more than one command"
  expect_tabs "$5" "$5" "after $1 showed Space $4"
}
send click $page_point
show_space "key 1 ctrl" select-space page "$first" "$a"
send click $page_point
show_space "key 2 ctrl" select-space page "$second" "$b"
send click $page_point
show_space "key left alt cmd" previous-space page "$first" "$a"
send click $page_point
show_space "key right alt cmd" next-space page "$second" "$b"
# Spaces don't wrap around.
send key right alt cmd
expect '^browser: command next-space via page$' "⌥⌘→ at the last Space didn't reach the UI"
send state
none_since '^browser: space [0-9]' || fail "⌥⌘→ at the last Space showed another"
send click $sidebar_point
show_space "key 1 ctrl" select-space ui "$first" "$a"
show_space "key right alt cmd" next-space ui "$second" "$b"
show_space "key left alt cmd" previous-space ui "$first" "$a"
show_space "click 131 $dots_y" select-space sidebar "$second" "$b"
show_space "click 97 $dots_y" select-space sidebar "$first" "$a"
../tools/window-screenshot.sh "$pid" "$out/spaces.png" || fail "no window to capture"

# Two fingers across the sidebar page between Spaces. A flick goes on to the next Space, a drag let
# go still goes to the Space nearest its place, and past the last Space the pages come back. Fingers
# along the list scroll it, and a wheel's tick across pages once. Held partway, the next Space's
# page shows beside the first's.
swipe_point="110 500"
post swipe $swipe_point -10 0 10 1500
await '^browser: swipe across$' || fail "two fingers across the sidebar didn't page"
../tools/window-screenshot.sh "$pid" "$out/swipe.png" || fail "no window to capture"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
none_since '^browser: space [0-9]' || fail "a swipe let go short of halfway showed another Space"
# $1 is the driver's swipe, $2 the Space it shows and $3 that Space's tab.
swipe_to() {
  send swipe $swipe_point $1
  expect '^browser: command select-space via swipe$' "a swipe of $1 didn't page"
  expect "^browser: space $2 in " "a swipe of $1 didn't show Space $2"
  expect_tabs "$3" "$3" "after a swipe of $1 showed Space $2"
}
swipe_to "-12 0 10" "$second" "$b"
swipe_to "12 0 10" "$first" "$a"
swipe_to "-30 0 20 200" "$second" "$b"
send swipe $swipe_point -12 0 10
expect '^browser: swipe across$' "two fingers across the sidebar didn't page"
none_since '^browser: space [0-9]' || fail "a flick past the last Space showed another"
send swipe $swipe_point 1 -10 10
expect '^browser: swipe along$' "two fingers along the list didn't read as scrolling"
none_since '^browser: space [0-9]' || fail "two fingers along the list showed another Space"
send wheel $swipe_point 1
expect '^browser: command select-space via wheel$' "a wheel's tick across the sidebar didn't page"
expect "^browser: space $first in " "a wheel's tick across the sidebar didn't show the first Space"
expect_tabs "$a" "$a" "after a wheel's tick showed the first Space"
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
# Past the first Space is the Library. Held partway it shows beside the page, and let go short of
# halfway it goes back. A flick brings it out and one the other way puts it back, and so do a
# wheel's ticks.
post swipe $swipe_point 30 0 10 1500
expect '^browser: library section ' "two fingers past the first Space didn't bring the Library out"
../tools/window-screenshot.sh "$pid" "$out/library-swipe.png" || fail "no window to capture"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
expect '^browser: library hidden$' "a swipe let go short of halfway didn't put the Library back"
none_since '^browser: command view-library' ||
  fail "a swipe let go short of halfway opened the Library"
# $1 is how the driver moves, $2 its source and $3 whether the Library shows after it.
library_to() {
  send $1
  expect "^browser: command view-library via $2\$" "a $1 didn't reach the Library"
  expect "^browser: library $3\$" "a $1 didn't leave the Library $3"
}
library_to "swipe $swipe_point 12 0 10" swipe shown
# Fingers going on past where it opens take the Library on a little with the card, its column's
# fill still reaching the window's left edge, lighter than the window right of it.
post swipe $swipe_point 60 0 10 1500
expect '^browser: library past its end$' "fingers past the Library's end didn't reach it"
../tools/window-screenshot.sh "$pid" "$out/library-end.png" || fail "no window to capture"
column=$(pixel "$out/library-end.png" $(echo "6 / ${content%x*}" | bc -l) 0.85)
beside=$(pixel "$out/library-end.png" $(echo "200 / ${content%x*}" | bc -l) 0.85)
brighter "$column" "$beside" ||
  fail "past its end the Library's column left the window's edge: $column at it, $beside beside"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
none_since '^browser: command view-library' || fail "fingers past the Library's end closed it"
expect '^browser: library shown$' "the Library didn't settle back where it opens"
library_to "swipe $swipe_point -12 0 10" swipe hidden
library_to "wheel $swipe_point 1" wheel shown
library_to "wheel $swipe_point -1" wheel hidden

# A tab's menu moves it to another Space, and the window goes with the tab it shows.
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Duplicate Tab, Move to Space › reading 日本語, Close Tab, Close Tabs Below (disabled)$' "the tab's menu didn't offer the other Space"
send choose reading 日本語
expect '^browser: command move-to-space via tab menu$' "Move to Space didn't run"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "Move to Space didn't show the tab's new Space"
expect_tabs "$a $b" "$a" "after Move to Space"
expect "^browser: settled space $second\$" "the second Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Duplicate Tab, Move to Space › Space 1, Close Tab, Close Tabs Below$' "the moved tab's menu didn't offer the first Space"
send choose Space 1
expect "^browser: space $first\( in .*\)\{0,1\}\$" "moving the tab back didn't show the first Space"
expect_tabs "$a" "$a" "after moving the tab back"
show_space "key 2 ctrl" select-space ui "$second" "$b"

# Fingers pull the last Space's page past its place and bring in a circle, which readies a new
# Space once its ring closes. Let go short of that, the page goes back; let go readied, they make
# the Space with its form, as New Space… does.
expect "^browser: settled space $second\$" "the second Space's page didn't settle"
send swipe $swipe_point -10 0 10
expect '^browser: swipe across$' "two fingers across the sidebar didn't page"
none_since '^browser: new space ready$' || fail "a short pull past the last Space readied a Space"
post swipe $swipe_point -40 0 10 1500
expect '^browser: new space ready$' "a long pull past the last Space didn't ready a Space"
../tools/window-screenshot.sh "$pid" "$out/new-space-pull.png" || fail "no window to capture"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
expect '^browser: command new-space via swipe$' "letting go of a readied pull didn't make a Space"
expect "^browser: spaces $first $second [0-9]*\$" "the pull didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: space $third in " "the window didn't show the third Space"
send key return
none_since '^browser: create space' || fail "return with no name named the Space"
show_space "key 1 ctrl" select-space ui "$first" "$a"
# The footer's + offers New Space… and New Folder…, 8 in from the sidebar's edge.
post click 204 $((${content#*x} - 25))
expect '^browser: menu shown New Space…, New Folder…$' \
  "the footer's + didn't offer New Space… and New Folder…"
send choose New Space…
expect '^browser: command new-space via footer$' "New Space… in the +'s menu didn't run"
expect "^browser: space $third in " "New Space didn't show the form already open"
# Clicks wait for the Space's page to slide into place.
expect "^browser: settled space $third\$" "the form's page didn't settle"
none_since '^browser: spaces ' || fail "New Space with a form open made another Space"
send state
in_browser AccessKitSubclassOfGPUIView "back in the third Space's form"
send click 114 $((${content#*x} - 74))
expect "^browser: cancel space $third\$" "a click on Cancel didn't cancel"
# The pull opened the form from the second Space.
expect "^browser: space $second in " "Cancel didn't show the second Space again"
expect "^browser: spaces $first $second\$" "Cancel didn't delete the third Space"
expect_tabs "$b" "$b" "after Cancel"
expect "^browser: settled space $second\$" "the second Space's page didn't settle after Cancel"
show_space "key 1 ctrl" select-space ui "$first" "$a"
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
# Fingers going on past the form rubber-band it, ready no new Space and leave its field the
# keyboard. Going back, they page to the Space it was made from, here the first, not the Space
# before it, and its Space goes once its page is out.
send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run new-space"
expect "^browser: spaces $first $second [0-9]*\$" "New Space didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: settled space $third\$" "the form's page didn't settle"
send swipe $swipe_point -40 0 10
expect '^browser: swipe across$' "two fingers across the form didn't page"
expect "^browser: settled space $third\$" "the form's page didn't settle back"
none_since '^browser: new space ready$' || fail "fingers past the form readied a Space"
none_since '^browser: cancel space' || fail "fingers going on past the form cancelled it"
send state
in_browser AccessKitSubclassOfGPUIView "in the form after a swipe"
send swipe $swipe_point 12 0 10
expect "^browser: cancel space $third\$" "two fingers back off the form didn't cancel it"
expect "^browser: space $first in " "a swipe back off the form didn't show the Space it came from"
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
expect "^browser: spaces $first $second\$" "a swipe back off the form didn't delete its Space"
expect_tabs "$a" "$a" "after a swipe back off the form"
# The Spaces menu lists the Spaces by name, and a name shows its Space.
show_space "menu reading 日本語" select-space ui "$second" "$b"
expect "^browser: settled space $second\$" "the Space the menu showed didn't settle"
show_space "key 1 ctrl" select-space ui "$first" "$a"
expect "^browser: settled space $first\$" "the first Space's page didn't settle"

# A right-click on the Space's name or its dot shows the Space's menu, as in Arc.
name_point="100 112"
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on the Space's name didn't show its menu"
../tools/window-screenshot.sh "$pid" "$out/space-menu.png" || fail "no window to capture"
send choose Rename Space
expect "^browser: rename space $first\$" "Rename Space didn't open the name's field"
send state
in_browser AccessKitSubclassOfGPUIView "in the name's field"
send type work
send key return
expect "^browser: renamed space $first work\$" "return in the name's field didn't rename the Space"
send doubleclick $name_point
expect "^browser: rename space $first\$" "a double-click on the Space's name didn't open its field"
# A right-click in the field shows the field's menu, not the Space's, and the field stays.
post rightclick $name_point
expect '^browser: menu shown Cut, Copy, Paste, Select All$' \
  "a right-click in the name's field didn't show the field's menu"
none_since '^browser: menu shown Rename Space' || fail "the right-click showed the Space's menu too"
send choose Select All
send type gone
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
none_since '^browser: renamed space' || fail "escape in the name's field renamed the Space"
# Rename Space in the Spaces menu turns the name of the Space shown into a field too.
send menu Rename Space
expect '^browser: command rename-space via ui$' "Rename Space in the menu didn't run rename-space"
expect "^browser: rename space $first\$" "Rename Space in the menu didn't open the name's field"
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
# Change Icon… in the Space's menu puts a field before its name and asks for the character palette,
# and an emoji committed there is the Space's icon at once, in its dot's place in the switcher.
# Escape leaves the Space as it was, and the menu offers Remove Icon only once there is an icon.
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on the Space's name didn't offer Change Icon…"
send choose Change Icon…
expect "^browser: change space icon $first\$" \
  "Change Icon… didn't put a field before the Space's name"
expect '^browser: character palette shown$' "Change Icon… didn't show the character palette"
../tools/window-screenshot.sh "$pid" "$out/space-icon-field.png" || fail "no window to capture"
send key escape
expect '^browser: rename cancelled$' "escape in the icon's field didn't cancel"
none_since '^browser: iconed space' || fail "escape in the icon's field gave the Space an icon"
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on the Space's name offered Remove Icon without an icon"
send choose Change Icon…
expect "^browser: change space icon $first\$" \
  "Change Icon… didn't put a field before the Space's name"
send insert 🦊
expect "^browser: iconed space $first 🦊\$" \
  "an emoji from the palette didn't become the Space's icon"
../tools/window-screenshot.sh "$pid" "$out/space-icon.png" || fail "no window to capture"
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, Remove Icon, $colors, $profile, \
Delete Space\$" "a right-click on the Space's name didn't offer Remove Icon"
send choose Remove Icon
expect "^browser: iconed space $first none\$" "Remove Icon didn't take the Space's icon away"
# Deleting a Space with tabs asks first, in a dialog over the window, and a click around it cancels.
# Return deletes the Space once the dialog has gone. Its tabs close with it, and ⇧⌘T reopens them
# here.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
post choose Delete Space
expect '^browser: dialog shown Delete “reading 日本語”?$' "Delete Space didn't ask first"
expect '^browser: dialog settled$' "the dialog didn't settle"
send click 300 40
expect '^browser: dialog cancelled$' "a click around the dialog didn't cancel it"
expect '^browser: dialog hidden$' "the dialog didn't go"
send state
[ "$key" = BrowserCoreNSWindow ] || fail "the window didn't get the keyboard back: $key has it"
none_since '^browser: spaces' || fail "a click around the dialog deleted the Space"
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
post choose Delete Space
expect '^browser: dialog settled$' "Delete Space didn't ask again"
../tools/window-screenshot.sh "$pid" "$out/space-delete.png" || fail "no window to capture"
send key return
expect '^browser: dialog confirmed$' "return didn't choose Delete"
expect "^browser: spaces $first\$" "Delete didn't delete the Space"
sed -n "/^browser: drive $step: /,\$p" "$log" | grep -A 999 '^browser: dialog hidden$' |
  grep -q "^browser: spaces $first\$" || fail "the Space went before the dialog did"
[ "$(today)" = "$a" ] || fail "deleting another Space left this one's Today as \"$(today)\""
# The toast says so, and ⌘Z brings the Space back where it was, with its name and its tab, and
# shows it.
deleted='^browser: toast shown Space deleted! Use ⌘Z to undo\.$'
expect "$deleted" "the toast didn't say the Space went"
expect '^browser: toast settled$' "the toast didn't settle"
../tools/window-screenshot.sh "$pid" "$out/space-deleted.png" || fail "no window to capture"
send key z cmd
expect '^browser: command undo via \(ui\|page\)$' "⌘Z didn't undo the delete"
expect "^browser: spaces $first $second\$" "⌘Z didn't bring the Space back where it was"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "⌘Z didn't show the Space it brought back"
expect "^browser: loaded [0-9]* https://example.com/\$" "the Space came back without its tab"
expect "^browser: settled space $second\$" "the Space ⌘Z brought back didn't settle"
# Deleted again, its tab reopens here with ⇧⌘T, and ⌘Z brings back the Space without it.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
post choose Delete Space
expect '^browser: dialog shown Delete “reading 日本語”?$' "the Space came back without its name"
expect '^browser: dialog settled$' "the dialog didn't settle"
send key return
expect "^browser: spaces $first\$" "Delete didn't delete the Space it brought back"
expect "^browser: today $a\$" "the Space shown didn't come back: Today is \"$(today)\""
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T didn't reach the UI"
expect "^browser: today [0-9]* $a\$" "⇧⌘T didn't reopen the deleted Space's tab here: Today is \"$(today)\""
expect "^browser: loaded $(today | cut -d' ' -f1) https://example.com/\$" \
  "the reopened tab didn't load its page"
send key z cmd
expect "^browser: spaces $first $second\$" "⌘Z after ⇧⌘T didn't bring the Space back"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "⌘Z after ⇧⌘T didn't show the Space"
expect '^browser: today $' "⌘Z brought back the tab ⇧⌘T had reopened: Today is \"$(today)\""
# Empty, it goes without asking, and says so too.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $empty_profile, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
send choose Delete Space
expect "^browser: spaces $first\$" "Delete didn't delete the empty Space"
none_since '^browser: dialog shown' || fail "deleting an empty Space asked first"
expect "$deleted" "the toast didn't say the empty Space went"
# The last Space can't go.
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $colors, $profile, \
Delete Space (disabled)\$" \
  "the last Space's menu offered Delete"
send choose Rename Space
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
exec 3>&-
quit
spaces_ms=$(sed -n 's/^browser: space [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: New Space makes an empty Space whose form takes the keyboard and Japanese input, and names it, and ⌘T opens a tab in it; ⌃1 ⌃2 ⌥⌘← ⌥⌘→ with the page or the UI focused, and the switcher's dots, show each Space with the tab it showed last; two fingers across the sidebar page to the Space a flick or the pages' place points at, never past the last, and a wheel's tick across pages once; past the first Space, fingers and a wheel's tick bring the Library out and put it back, and fingers let go short of halfway put it back, and past where it opens take it on a little, its column's fill at the window's edge; a tab's menu moves it to another Space, which shows with it; fingers held past the last Space ready a new Space and let go make it, with its form, and let go short of that don't; Cancel, and two fingers back, delete a new Space and go back to the Space it was made from, and fingers going on past its form rubber-band it. A right-click on a Space's name or dot shows Rename, Change Icon…, its colours and Delete; an emoji from the character palette is its icon, in its dot's place in the switcher, until Remove Icon, and escape leaves it as it was; a double-click on the name renames it too, a right-click in its field shows the field's menu, return keeps the name and escape cancels. Delete asks first for a Space with tabs, in a dialog a click around cancels and return confirms, closes them once it has gone, and the toast says so; ⌘Z brings the Space back where it was with its name and tabs, ⇧⌘T reopens them in the Space shown, and ⌘Z then leaves those out; the last Space can't go. The Spaces menu names each Space and shows it, and its Rename Space opens the name's field. Windows: $out/swipe.png, $out/library-swipe.png, $out/library-end.png, $out/new-space-pull.png, $out/space-menu.png, $out/space-delete.png, $out/space-deleted.png"
echo "  Space switch, key to the core's report: $spaces_ms. Windows: $out/space-form.png, $out/space-form-ime.png, $out/new-space.png, $out/spaces.png"
