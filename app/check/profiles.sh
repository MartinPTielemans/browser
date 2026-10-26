# The New Space form's Profile row makes a profile and puts the Space in it, and its Choose a Theme
# row colours the Space. The Space's tabs open in the new profile, without the first profile's
# cookies. While it's empty the Space's menu moves it between profiles, and with a tab it keeps its
# own. The first Space keeps its cookie and its favorite, which shows only in the Spaces of its
# profile, and a new Space made from a Space is in that Space's profile. After a relaunch each Space
# is in its profile still. Deleting the Space leaves the profile unused, and Delete in the menu
# deletes it once the user confirms.

# The cookie page's title shows its site's cookies, and "c" sets one. It's on localhost, as
# clear.sh's cookie is on 127.0.0.1.
serve
printf '%s' '<title>loading</title>
<script>
document.title = "cookie " + (document.cookie || "none");
onkeydown = e => {
  if (e.key == "c") {
    document.cookie = "p=1; max-age=3600";
    document.title = "cookie set";
  }
};
</script>' > "$out/site/profile.html"
cookie_page="$(echo "$site" | sed 's/127\.0\.0\.1/localhost/')profile.html"
log="$out/profiles.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$cookie_page" "$app/Contents/MacOS/Browser" \
  < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
first=$(space)
a=$(today)
await "^browser: title $a cookie " || fail "the cookie page didn't draw"
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
expect "^browser: title $a cookie p=1\$" "the cookie page lost its cookie on ⌘R"
# The tab becomes the first profile's favorite, which leaves the first Space empty.
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, ' \
  "the tab's menu didn't offer Add to Favorites"
send choose Add to Favorites
expect "^browser: favorites $a\$" "Add to Favorites didn't make the tab a favorite"

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
post click 100 299
expect '^browser: menu shown System Accent (checked), Gray, ' \
  "the form's Choose a Theme row didn't show the colours"
send choose Green
expect "^browser: colored space $second 55bf81\$" "Green in the form didn't colour the Space"
../tools/window-screenshot.sh "$pid" "$out/profile-form.png" || fail "no window to capture"
green=$(echo "$colors" | sed 's/ (checked)//; s/Green/Green (checked)/')
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
expect "^browser: menu shown Rename Space, $green, Profile › Default, \
Profile › Profile 2 (checked), Profile › New Profile, Delete Space\$" \
  "the empty Space's menu didn't offer its profiles"
../tools/window-screenshot.sh "$pid" "$out/profile-menu.png" || fail "no window to capture"
send choose Default
expect "^browser: profiled space $second $default\$" "Default didn't take the Space back"
expect "^browser: favorites $a\$" "the first profile's favorite didn't show in its Space again"
post rightclick $tiled_name
expect "^browser: menu shown Rename Space, $green, Profile › Default (checked), \
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
expect "^browser: menu shown Rename Space, $green, Profile › Default (disabled), \
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
expect "^browser: title $a cookie p=1\$" "the first profile's cookie went"
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
await "^browser: title $a cookie p=1\$" ||
  fail "the favorite came back without its profile's cookie"
send click $page_point
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect '^browser: favorites $' "the second Space showed the first profile's favorite"
b=$(today)
expect "^browser: title $b cookie none\$" "the second Space's tab came back in the first profile"
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
expect "^browser: menu shown Rename Space, $colors, Profile › Default (checked), \
Profile › Profile 2, Profile › New Profile, Profile › Delete “Profile 2”, \
Delete Space (disabled)\$" \
  "the menu didn't offer to delete the unused profile"
post choose Delete “Profile 2”
expect '^browser: dialog shown Delete “Profile 2”?$' "Delete didn't ask before deleting the profile"
expect '^browser: dialog settled$' "the dialog didn't settle"
../tools/window-screenshot.sh "$pid" "$out/profile-delete.png" || fail "no window to capture"
send key return
expect "^browser: removed profile $work Profile 2\$" "the profile wasn't deleted"
exec 3>&-
quit
echo "ok: the New Space form's Profile row makes a profile and puts the Space in it, where its tabs open without the first profile's cookie, and its Choose a Theme row colours it; an empty Space's menu moves it between profiles, and a Space with tabs keeps its profile; a new Space is in the profile of the Space it's made from; favorites show only in their profile's Spaces; after a relaunch each Space is in its profile and each tab loads in it; a profile nothing is in is deleted from the menu once the user confirms. Windows: $out/profile-popup.png, $out/profile-form.png, $out/profile-made.png, $out/profile-menu.png, $out/profile-space.png, $out/profile-delete.png"
