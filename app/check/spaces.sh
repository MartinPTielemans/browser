# New Space in the Spaces menu makes a Space and shows it, empty and without a page, with a form
# that takes the keyboard from the page. Japanese input composes in its field, return while
# composing only commits, and return names the Space and sends the form away; ⌘T then opens a tab in
# it. ⌃1–⌃9, ⌥⌘← and ⌥⌘→, with the page or the UI focused, and clicks on the switcher's dots show
# another Space, which shows the tab it showed last. Two fingers across the sidebar page to the
# Space a flick or the pages' place points at, never past the last, fingers along it don't, and a
# wheel's tick across pages once. A tab's menu moves the tab to another Space, which the window
# shows with it. Return with no name does nothing, New Space shows a form already open, and Cancel
# deletes its Space and goes back. A right-click on the Space's name or a dot shows Rename Space,
# its colours and Delete Space. Rename, and a double-click on the name, turn it into a field, where
# return keeps the name and escape cancels. Delete asks first for a Space with tabs, in a dialog
# that a click around cancels and return confirms, and the Space goes once the dialog has. Its tabs
# close with it and reopen with ⇧⌘T in the Space shown, and the last Space can't be deleted.

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
library_to "swipe $swipe_point -12 0 10" swipe hidden
library_to "wheel $swipe_point 1" wheel shown
library_to "wheel $swipe_point -1" wheel hidden

# A tab's menu moves it to another Space, and the window goes with the tab it shows.
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space › reading 日本語, Close Tab, Close Tabs Below (disabled)$' "the tab's menu didn't offer the other Space"
send choose reading 日本語
expect '^browser: command move-to-space via tab menu$' "Move to Space didn't run"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "Move to Space didn't show the tab's new Space"
expect_tabs "$a $b" "$a" "after Move to Space"
expect "^browser: settled space $second\$" "the second Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space › Space 1, Close Tab, Close Tabs Below$' "the moved tab's menu didn't offer the first Space"
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
send menu New Space…
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

# A right-click on the Space's name or its dot shows the Space's menu, as in Arc.
name_point="100 112"
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
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
send type gone
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
none_since '^browser: renamed space' || fail "escape in the name's field renamed the Space"
# Deleting a Space with tabs asks first, in a dialog over the window, and a click around it cancels.
# Return deletes the Space once the dialog has gone. Its tabs close with it, and ⇧⌘T reopens them
# here.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
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
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
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
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T didn't reach the UI"
expect "^browser: today [0-9]* $a\$" "⇧⌘T didn't reopen the deleted Space's tab here: Today is \"$(today)\""
expect "^browser: loaded $(today | cut -d' ' -f1) https://example.com/\$" "the reopened tab didn't load its page"
# The last Space can't go.
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space (disabled)\$" \
  "the last Space's menu offered Delete"
send choose Rename Space
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
exec 3>&-
quit
spaces_ms=$(sed -n 's/^browser: space [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: New Space makes an empty Space whose form takes the keyboard and Japanese input, and names it, and ⌘T opens a tab in it; ⌃1 ⌃2 ⌥⌘← ⌥⌘→ with the page or the UI focused, and the switcher's dots, show each Space with the tab it showed last; two fingers across the sidebar page to the Space a flick or the pages' place points at, never past the last, and a wheel's tick across pages once; past the first Space, fingers and a wheel's tick bring the Library out and put it back, and fingers let go short of halfway put it back; a tab's menu moves it to another Space, which shows with it; fingers held past the last Space ready a new Space and let go make it, with its form, and let go short of that don't; Cancel deletes a new Space and goes back to the Space it was made from. A right-click on a Space's name or dot shows Rename, its colours and Delete; a double-click on the name renames it too, return keeps the name and escape cancels. Delete asks first for a Space with tabs, in a dialog a click around cancels and return confirms, closes them once it has gone, and ⇧⌘T reopens them in the Space shown; the last Space can't go. Windows: $out/swipe.png, $out/library-swipe.png, $out/new-space-pull.png, $out/space-menu.png, $out/space-delete.png"
echo "  Space switch, key to the core's report: $spaces_ms. Windows: $out/space-form.png, $out/space-form-ime.png, $out/new-space.png, $out/spaces.png"
