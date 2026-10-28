# The New Space form's Profile row makes a profile and puts the Space in it, and its Choose a Theme
# row colours the Space. The Space's tabs open in the new profile, without the first profile's
# cookies. While it's empty the Space's menu moves it between profiles, and with a tab it keeps its
# own. The first Space keeps its cookie and its favorite, which shows only in the Spaces of its
# profile, and a new Space made from a Space is in that Space's profile. After a relaunch each Space
# is in its profile still. A tab moved to the other profile's Space loads there with that profile's
# cookie, after a warning the first time. Settings › Profiles renames a profile, which the Space
# menu then shows, and makes and deletes one. Deleting the Space leaves its profile unused, and
# Delete in the menu deletes it once the user confirms.

# The cookie page's title says whether it finds its cookie, p, never a value, since the app logs
# every title, and "c" sets it. The cookie outlives the run, so its value is this run's start and an
# earlier run's doesn't count. It's on localhost, as clear.sh's cookie is on 127.0.0.1.
serve
printf '%s' '<title>loading</title>
<script>
const mine = "p='"$(date +%s)"'";
document.title = "cookie " + (document.cookie.split("; ").includes(mine) ? "p" : "none");
onkeydown = e => {
  if (e.key == "c") {
    document.cookie = mine + "; max-age=3600";
    document.title = "cookie set";
  }
};
</script>' > "$out/site/profile.html"
cookie_page="$(echo "$site" | sed 's/127\.0\.0\.1/localhost/')profile.html"
log="$out/profiles.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$cookie_page" "$app/Contents/MacOS/Browser" \
  < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
first=$(space)
a=$(today)
await "^browser: title $a cookie none\$" ||
  fail "the cookie page didn't draw, or found an earlier run's cookie"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: profiles. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
default=$(sed -n "s/^browser: profiled space $first \([0-9]*\)\$/\1/p" "$log" | tail -1)
grep -q "^browser: added profile $default Default\$" "$log" ||
  fail "the first Space isn't in the profile named Default"
send key c
expect "^browser: title $a cookie set\$" "c in the cookie page didn't set a cookie"
send key r cmd
expect "^browser: title $a cookie p\$" "the cookie page lost its cookie on ⌘R"
# Dragged onto the Space's name, the tab becomes the first profile's favorite, which leaves the
# first Space empty.
send press $(row 0)
send dragto 100 160
send dragto 114 112
expect '^browser: drag shape tile$' "the row dragged onto the Space's name didn't turn into a tile"
send release 114 112
expect "^browser: favorites $a\$" "the row dragged onto the Space's name didn't make a favorite"

# A new Space is in the profile of the Space it's made from. Its form's Profile row puts it in a
# new profile, whose name Chrome's numbering gives, and its favorites aren't the first profile's.
send menu New Space…
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: space $second in " "the window didn't show the new Space"
expect "^browser: profiled space $second $default\$" \
  "the new Space isn't in the profile of the Space it was made from"
expect "^browser: settled space $second\$" "the new Space's page didn't settle"
# The form's Profile and Choose a Theme rows, under its name field.
post click 100 255
expect '^browser: menu shown Default (checked), New Profile$' \
  "the form's Profile row didn't show the profiles"
../tools/window-screenshot.sh "$pid" "$out/profile-popup.png" || fail "no window to capture"
send choose New Profile
expect '^browser: create profile [0-9]* Profile 2$' "New Profile didn't make a profile"
work=$(sed -n 's/^browser: create profile \([0-9]*\) .*/\1/p' "$log" | tail -1)
expect "^browser: added profile $work Profile 2\$" "the new profile wasn't listed"
expect "^browser: profiled space $second $work\$" "the form's Space didn't go into the new profile"
expect '^browser: favorites $' "the new profile's Space showed the first profile's favorite"
send click 100 299
expect "^browser: theme picker for $second\$" "the form's Choose a Theme row didn't open the picker"
expect '^browser: theme picker shown$' "the form's Choose a Theme row didn't show the picker"
expect '^browser: theme picker drawn$' "the theme picker didn't draw itself on screen"
send clickkey "$(swatch_x 6)" $swatch_y
expect "^browser: colored space $second [0-9a-f]*\$" "the green swatch didn't colour the Space"
../tools/window-screenshot.sh "$pid" "$out/profile-form.png" || fail "no window to capture"
send key escape
expect '^browser: theme picker hidden$' "escape didn't close the theme picker"
send type work
send key return
expect "^browser: create space $second work\$" "return didn't name the Space"
expect '^browser: space form gone$' "the form didn't leave after return"
../tools/window-screenshot.sh "$pid" "$out/profile-made.png" || fail "no window to capture"
# Empty, its menu takes it back to the first profile, whose favorite shows again, and to the new
# one again. The Space's name is under the URL field, and 47 lower under a row of tiles.
name_point="100 112"
tiled_name="100 159"
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $themed, Profile › Default, \
Profile › Profile 2 (checked), Profile › New Profile, Delete Space\$" \
  "the empty Space's menu didn't offer its profiles"
../tools/window-screenshot.sh "$pid" "$out/profile-menu.png" || fail "no window to capture"
send choose Default
expect "^browser: profiled space $second $default\$" "Default didn't take the Space back"
expect "^browser: favorites $a\$" "the first profile's favorite didn't show in its Space again"
post rightclick $tiled_name
expect "^browser: menu shown Rename Space, Change Icon…, $themed, Profile › Default (checked), \
Profile › Profile 2, Profile › New Profile, Profile › Delete “Profile 2”, Delete Space\$" \
  "the menu didn't offer the unused profile back"
send choose Profile 2
expect "^browser: profiled space $second $work\$" "Profile 2 didn't take the Space again"
expect '^browser: favorites $' "the first profile's favorite stayed in the other profile's Space"
send key t cmd
expect '^browser: command bar shown$' "⌘T in the new profile's Space didn't open the command bar"
send type "$cookie_page"
send key return
expect '^browser: today [0-9][0-9]*$' "the cookie page didn't open in the new profile's Space"
b=$(today)
expect "^browser: title $b cookie none\$" \
  "the new profile's tab saw the first profile's cookie: $(grep "^browser: title $b" "$log" |
  tail -1)"
../tools/window-screenshot.sh "$pid" "$out/profile-space.png" || fail "no window to capture"
# Its menu, with a tab, can't change its profile.
post rightclick $name_point
expect "^browser: menu shown Rename Space, Change Icon…, $themed, Profile › Default (disabled), \
Profile › Profile 2 (checked), Profile › New Profile (disabled), Delete Space\$" \
  "the menu of a Space with a tab offered another profile"
send choose Profile 2
none_since '^browser: profiled space' || fail "choosing its own profile changed the Space's"
# A Space made from this one is in its profile. Cancel takes it away again.
send menu New Space…
expect "^browser: spaces $first $second [0-9]*\$" "New Space didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: profiled space $third $work\$" \
  "a Space made from the new profile's Space isn't in it"
expect "^browser: settled space $third\$" "the third Space's page didn't settle"
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
send click 114 $((${content#*x} - 74))
expect "^browser: cancel space $third\$" "a click on Cancel didn't cancel"
expect "^browser: spaces $first $second\$" "Cancel didn't delete the third Space"
# The first profile's Space has its favorite and its cookie still.
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
expect "^browser: favorites $a\$" "the first Space didn't show its profile's favorite"
send click $page_point
send key r cmd
expect "^browser: title $a cookie p\$" "the first profile's cookie went"
exec 3>&-
quit

# Launched again, each Space is in its profile, and each tab loads in its own.
log="$out/profiles-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "the relaunch didn't bring back two Spaces"
first=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f3)
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
default=$(sed -n 's/^browser: added profile \([0-9]*\) Default$/\1/p' "$log")
work=$(sed -n 's/^browser: added profile \([0-9]*\) Profile 2$/\1/p' "$log")
[ -n "$work" ] || fail "the new profile wasn't listed after the relaunch"
grep -q "^browser: profiled space $first $default\$" "$log" ||
  fail "the first Space came back in another profile"
grep -q "^browser: profiled space $second $work\$" "$log" ||
  fail "the second Space came back in another profile"
await "^browser: favorites [0-9][0-9]*\$" || fail "the first profile's favorite didn't come back"
a=$(grep '^browser: favorites' "$log" | tail -1 | cut -d' ' -f3)
await "^browser: title $a cookie p\$" ||
  fail "the favorite came back without its profile's cookie"
send click $page_point
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect '^browser: favorites $' "the second Space showed the first profile's favorite"
b=$(today)
expect "^browser: title $b cookie none\$" "the second Space's tab came back in the first profile"
# Its menu moves it to the first profile's Space, after a warning the first time, as a tab
# whose page loads there with that profile's cookie. Moved back, from under the favorite's tile,
# it asks nothing.
post rightclick $(row 0)
expect '^browser: menu shown .*, Move to › Space 1, Move to › work (checked), ' \
  "the tab's menu didn't offer the other profile's Space"
post choose Space 1
expect '^browser: dialog shown Move this tab to the “Default” profile?$' \
  "moving the tab to another profile's Space didn't warn first"
expect '^browser: dialog settled$' "the dialog didn't settle"
../tools/window-screenshot.sh "$pid" "$out/profile-move.png" || fail "no window to capture"
send key return
expect "^browser: tab $b closed\$" "the tab moved to another profile kept its old page"
expect "^browser: space $first\$" "the window didn't go with the moved tab"
expect '^browser: today [0-9][0-9]*$' "the moved tab isn't in the first Space"
c=$(today)
expect "^browser: title $c cookie p\$" "the moved tab didn't load with the first profile's cookie"
post rightclick 100 $(($(row 0 | cut -d' ' -f2) + 47))
expect '^browser: menu shown .*, Move to › Space 1 (checked), Move to › work, ' \
  "the moved tab's menu didn't offer its old Space"
send choose work
none_since '^browser: dialog shown' || fail "the second move to another profile warned again"
expect "^browser: tab $c closed\$" "the tab moved back kept its page in the first profile"
expect "^browser: space $second\$" "the window didn't go with the tab moved back"
expect '^browser: today [0-9][0-9]*$' "the tab moved back isn't in the second Space"
d=$(today)
expect "^browser: title $d cookie none\$" "the tab moved back still saw the first profile's cookie"
# In Every Space, the favorite has a twin in the other profile's Space. Taken away from its URL and
# removed from the favorites, its twin goes too, and ⌘Z brings back both, each going back to the
# favorite's URL in its own profile.
printf '%s' '<title>away</title>' > "$out/site/away.html"
printf '%s' '<title>elsewhere</title>' > "$out/site/elsewhere.html"
away_page="$(echo "$site" | sed 's/127\.0\.0\.1/localhost/')away.html"
elsewhere_page="$(echo "$site" | sed 's/127\.0\.0\.1/localhost/')elsewhere.html"
tile="114 110"
send menu Settings…
expect '^browser: settings shown General$' "Settings… didn't show Settings on its General tab"
post clickkey 500 250
expect "^browser: menu shown In Every Space, In Each Profile's Spaces (checked), In Each Space\$" \
  "a click on Favorites didn't show the choices"
send choose In Every Space
expect "^browser: favorites scope In Every Space\$" "choosing In Every Space in Settings didn't set it"
expect '^browser: favorites [0-9][0-9]*$' "the favorite has no twin in the other profile's Space"
twin=$(grep '^browser: favorites' "$log" | tail -1 | cut -d' ' -f3)
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
[ "$twin" != "$a" ] || fail "the other profile's Space shows the first profile's favorite itself"
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
send click $tile
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
go_from_bar "$away_page"
expect "^browser: title $a away\$" "the favorite didn't go to another page"
expect "^browser: page $a away 1\$" "the favorite away from its URL didn't say so"
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL, Duplicate Tab, Remove from Favorites, ' \
  "the favorite's menu didn't offer Remove from Favorites"
send choose Remove from Favorites
expect '^browser: favorites $' "Remove from Favorites left the tile"
send key z cmd
expect '^browser: command undo via \(ui\|page\)$' "⌘Z didn't undo Remove from Favorites"
expect "^browser: favorites $a\$" "⌘Z didn't put the favorite back"
# Back to Favorite URL takes each twin to the URL the favorite had, in its own profile.
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL, ' "⌘Z put the favorite back without its URL"
send choose Back to Favorite URL
expect "^browser: title $a cookie p\$" "Back to Favorite URL didn't load the favorite's URL"
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect '^browser: favorites [0-9][0-9]*$' "⌘Z didn't bring back the favorite's twin"
twin=$(grep '^browser: favorites' "$log" | tail -1 | cut -d' ' -f3)
[ "$twin" != "$a" ] || fail "the other profile's Space shows the first profile's favorite itself"
send click $tile
expect "^browser: selected $twin\( in .*\)\{0,1\}\$" "a click on the twin's tile didn't show it"
# The twin opens where the favorite is, so it goes to a page of its own.
go_from_bar "$elsewhere_page"
expect "^browser: title $twin elsewhere\$" "the twin didn't go to another page"
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL, ' "the twin away from its URL didn't offer it back"
send choose Back to Favorite URL
expect "^browser: title $twin cookie none\$" \
  "the twin's Back to Favorite URL didn't load the favorite's URL in its own profile"
# Back to In Each Profile's Spaces, each profile keeps the site as its own favorite. Removed and
# closed, the second Space has only its tab again.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
post clickkey 500 250
expect "^browser: menu shown In Every Space (checked), In Each Profile's Spaces, In Each Space\$" \
  "a click on Favorites didn't show In Every Space chosen"
send choose "In Each Profile's Spaces"
expect "^browser: favorites scope In Each Profile's Spaces\$" \
  "choosing In Each Profile's Spaces in Settings didn't set it"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
[ "$(grep '^browser: favorites' "$log" | tail -1)" = "browser: favorites $twin" ] ||
  fail "the second profile didn't keep the twin as its own favorite: $(grep '^browser: favorites' "$log" | tail -1)"
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL (disabled), Duplicate Tab, Remove from Favorites, ' \
  "the second profile's favorite's menu didn't offer Remove from Favorites"
send choose Remove from Favorites
expect '^browser: favorites $' "Remove from Favorites left the second profile's tile"
expect "^browser: today $twin $d\$" "the removed favorite isn't at the top of Today: Today is \"$(today)\""
send click $page_point
send key w cmd
expect "^browser: today $d\$" "⌘W didn't close the removed favorite: Today is \"$(today)\""
expect "^browser: selected $d\( in .*\)\{0,1\}\$" "⌘W didn't show the second Space's tab"
# Settings › Profiles lists the profiles, a row each from 129, 41 apart, and New Profile under
# them. A double-click on a name puts it in a field, where escape leaves it and return keeps a new
# one.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey 246 60
expect '^browser: settings tab Profiles$' "a click on Profiles didn't show its tab"
send doubleclickkey 130 170
expect "^browser: rename profile $work\$" "a double-click on Profile 2 didn't put it in a field"
send type personal
send key escape
expect '^browser: rename cancelled$' "escape didn't cancel the renaming"
none_since '^browser: renamed profile' || fail "escape renamed the profile"
send doubleclickkey 130 170
expect "^browser: rename profile $work\$" "the second double-click didn't put it in a field"
send type personal
send key return
expect "^browser: renamed profile $work personal\$" "return didn't rename the profile"
# New Profile makes one, with its name in a field, which takes Japanese input. Its Delete asks
# first, and Cancel keeps it.
send clickkey 100 214
expect '^browser: create profile [0-9]* Profile 3$' "New Profile didn't make a profile"
spare=$(sed -n 's/^browser: create profile \([0-9]*\) .*/\1/p' "$log" | tail -1)
expect "^browser: rename profile $spare\$" "the new profile's name didn't take the keyboard"
send type spare
send key space
send ime on
send type nihongo
send key space
[ "$marked" = 6,3 ] || fail "the profile's field didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/settings-profile-name.png" || fail "no window to capture"
send key return
send ime off
send key return
expect "^browser: renamed profile $spare spare 日本語\$" \
  "the new profile wasn't named: $(grep '^browser: renamed profile' "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/settings-profiles.png" || fail "no window to capture"
post clickkey 578 211
expect '^browser: alert shown Delete “spare 日本語”?$' "Delete in Settings didn't ask first"
send choose Cancel
none_since '^browser: removed profile' || fail "Cancel deleted the profile"
post clickkey 578 211
expect '^browser: alert shown Delete “spare 日本語”?$' "Delete in Settings didn't ask again"
send choose Delete
expect "^browser: removed profile $spare spare 日本語\$" "Delete in Settings didn't delete it"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
# Deleted, the Space leaves its profile unused, which the first Space's menu then deletes, once the
# user confirms.
post rightclick $name_point
expect "^browser: menu shown Rename Space, " \
  "a right-click on the Space's name didn't show its menu"
post choose Delete Space
expect '^browser: dialog settled$' "Delete Space didn't ask first"
send key return
expect "^browser: spaces $first\$" "Delete didn't delete the second Space"
expect "^browser: space $first\( in .*\)\{0,1\}\$" "deleting the second Space didn't show the first"
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
post rightclick $tiled_name
expect "^browser: menu shown Rename Space, Change Icon…, $colors, Profile › Default (checked), \
Profile › personal, Profile › New Profile, Profile › Delete “personal”, \
Delete Space (disabled)\$" \
  "the menu didn't offer to delete the unused profile by the name Settings gave it"
post choose Delete “personal”
expect '^browser: dialog shown Delete “personal”?$' "Delete didn't ask before deleting the profile"
expect '^browser: dialog settled$' "the dialog didn't settle"
../tools/window-screenshot.sh "$pid" "$out/profile-delete.png" || fail "no window to capture"
send key return
expect "^browser: removed profile $work personal\$" "the profile wasn't deleted"
exec 3>&-
quit
echo "ok: the New Space form's Profile row makes a profile and puts the Space in it, where its tabs open without the first profile's cookie, and its Choose a Theme row colours it; an empty Space's menu moves it between profiles, and a Space with tabs keeps its profile; a new Space is in the profile of the Space it's made from; favorites show only in their profile's Spaces; after a relaunch each Space is in its profile and each tab loads in it; a tab moved to another profile's Space loads there with its cookies, after a warning only the first time; in Every Space a favorite has a twin in the other profile, which goes with it when it’s removed, comes back with ⌘Z, and goes back to the favorite’s URL in its own profile, and stays the other profile’s own favorite after; Settings › Profiles renames a profile on a double-click, which the Space menu shows, and makes one, named with Japanese input, and deletes it once the user confirms; a profile nothing is in is deleted from the menu once the user confirms. Windows: $out/profile-popup.png, $out/profile-form.png, $out/profile-made.png, $out/profile-menu.png, $out/profile-space.png, $out/profile-move.png, $out/settings-profile-name.png, $out/settings-profiles.png, $out/profile-delete.png"
