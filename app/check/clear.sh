# View › Clear Cache and Refresh and the command bar's Clear Cookies and Refresh delete the cache
# and the cookies of the page's site, and load the page again. ⇧⌘C and ⌥⇧⌘C from the page and the UI
# copy the tab's URL and a Markdown link to it, to a pasteboard of the run's own, without its
# tracking parameters. The sidebar shows Clear and the URL field's button only while the pointer is
# over it, and the button copies the URL. ⇧⌘K from the page and the UI, and Clear on the divider,
# close the Space's Today tabs but the one shown, with no toast, and ⌘Z from the page and the UI
# brings them back in place, unloaded, unless the page's text field has the keyboard, when ⌘Z is its
# own. With only the tab shown left, Clear has an arrow, lights up under the pointer and closes that
# tab too, and ⌘Z shows it again. ⇧⌘T from the page and the UI reopens the tab closed last, where it
# was, with its history, muted if it was and with its icon, and after a Clear, ⌘Z brings back only
# the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu, whose Duplicate Tab, Close Tabs
# Below and Unpin Tab work. ⌘W archives a Today tab. ⇧⌘L, View Library and the footer's button open
# the Library, which pushes the page right at its width, and its back button closes it. A
# right-click on an archived tab shows its menu, whose Copy URL copies its URL and Delete from
# Archive takes it out. A click on one reopens it at the top of Today with its history, and Clear
# Archive asks, then empties the archive. A page's question about a permission shows under the URL
# field while its tab is shown, bringing out a hidden sidebar, and its ×, Block and Allow answer it.
# The URL field's Site Controls button and the command bar's Site Controls, from the page, open a
# panel under the field that lists the site's permissions as Chrome's page info does. A value
# changed there sticks, and its Reset permissions sets them back. Escape, a click outside and the
# button close it.

# Clear, from the served page and two more, with the served page pinned for it to leave.
serve
log="$out/clear.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Clear test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: Clear. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Clear Cache and Refresh, from the View menu, and Clear Cookies and Refresh, from the command bar,
# delete the page's site's cache and cookies and load the page again.
send key k
expect "^browser: title $a cookie [^ ]* cache 1\$" "k in the served page didn't open the cookie page"
send key c
expect "^browser: title $a cookie set\$" "c in the cookie page didn't set a cookie"
send key r cmd
expect "^browser: title $a cookie c=1 cache 1\$" \
  "⌘R lost the cookie or fetched the cached script again: $(grep '^browser: title' "$log" | tail -1)"
send menu Clear Cache and Refresh
expect '^browser: command clear-cache-and-refresh via ui$' "Clear Cache and Refresh didn't run"
expect "^browser: title $a cookie c=1 cache 2\$" \
  "Clear Cache and Refresh didn't fetch the script again, or lost the cookie: $(grep '^browser: title' "$log" | tail -1)"
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type clear cookies
expect '^browser: command bar commands "clear cookies": Clear Cookies and Refresh$' \
  "the command bar didn't find Clear Cookies and Refresh"
send key return
expect '^browser: command clear-cookies-and-refresh via command bar$' \
  "Clear Cookies and Refresh didn't run from the command bar"
expect "^browser: title $a cookie none cache 2\$" \
  "Clear Cookies and Refresh didn't delete the cookie, or fetched the script again: $(grep '^browser: title' "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/clear-site.png" || fail "no window to capture"
send key left cmd
expect "^browser: page $a back 0 forward 1 loading 0 $site\$" "⌘← didn't go back to the served page"
# Clear on the divider, which sits under the one pinned tab, the URL field's Copy URL and Site
# Controls buttons, and the served page's text field, along the bottom of the 800 pt tall window.
clear_point="200 185"
copy_point="178 64"
site_controls_point="206 64"
field_point="600 760"
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
switch "3 cmd" "$a"
send key d cmd
expect_lists "$a" "$c $b" "after pinning the served page's tab"
switch "2 cmd" "$c"

# ⇧⌘C and ⌥⇧⌘C copy the tab's URL and a Markdown link to it, from the page and the UI. A driven
# run copies to a pasteboard of its own, never the clipboard.
send key c cmd shift
expect '^browser: command copy-url via page$' "⇧⌘C with the page focused didn't reach the UI"
expect '^browser: copied https://example.org/$' "⇧⌘C didn't copy the tab's URL"
# The toast says so, 8 inside the card's top right corner.
expect '^browser: toast shown Copied Current URL$' "⇧⌘C's toast didn't say it copied the URL"
expect '^browser: toast settled$' "⇧⌘C's toast didn't settle"
../tools/window-screenshot.sh "$pid" "$out/url-toast.png" || fail "no window to capture"
send click $sidebar_point
send key c cmd shift alt
expect '^browser: command copy-markdown-link via ui$' "⌥⇧⌘C with the UI focused didn't run"
expect '^browser: copied \[Example Domain](https://example.org/)$' \
  "⌥⇧⌘C didn't copy a Markdown link to the tab"

# Clear and the URL field's button fade in while the pointer is over the sidebar, and out when it
# leaves. The button copies the URL, and doesn't open the command bar as the rest of the field does.
send move $sidebar_point
expect '^browser: sidebar hovered 1$' "the sidebar didn't see the pointer come over it"
expect '^browser: sidebar tools shown$' "Clear and the URL field's button didn't fade in"
../tools/window-screenshot.sh "$pid" "$out/sidebar-hovered.png" || fail "no window to capture"
send click $copy_point
expect '^browser: command copy-url via sidebar$' \
  "a click on the URL field's button didn't run Copy URL"
expect '^browser: copied https://example.org/$' "the URL field's button didn't copy the tab's URL"
none_since '^browser: command edit-url' ||
  fail "a click on the URL field's button opened the command bar"
# It says so with a checkmark in its glyph's place for 2 s, which draws itself in as the link
# leaves, and no toast.
expect '^browser: copy url draws check$' "the URL field's button didn't start drawing a checkmark"
expect '^browser: copy url shows check$' "the URL field's checkmark didn't finish drawing"
../tools/window-screenshot.sh "$pid" "$out/url-copied.png" || fail "no window to capture"
expect '^browser: copy url brings back link$' "the URL field's button kept its checkmark"
expect '^browser: copy url shows link$' "the URL field's link didn't come all the way back"
none_since '^browser: toast shown' || fail "the URL field's button showed a toast"
send move $page_point
expect '^browser: sidebar hovered 0$' "the sidebar didn't see the pointer leave"
expect '^browser: sidebar tools hidden$' "Clear and the URL field's button didn't fade out"
../tools/window-screenshot.sh "$pid" "$out/sidebar-away.png" || fail "no window to capture"

# ⇧⌘K with the page focused closes the Today tabs but the one shown, here the middle of three,
# without showing or loading another.
open_from_bar example.net
x=$tab
switch "3 cmd" "$c"
send click $page_point
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: cleared 2$' "Clear didn't close the two tabs not shown"
expect "^browser: tab $b closed\$" "Clear didn't close the bottom tab"
expect "^browser: tab $x closed\$" "Clear didn't close the top tab"
expect_lists "$a" "$c" "after Clear"
none_since '^browser: selected' || fail "Clear showed another tab"
none_since '^browser: loaded' || fail "Clear loaded a page"
none_since '^browser: toast' || fail "Clear showed a toast"
../tools/window-screenshot.sh "$pid" "$out/clear.png" || fail "no window to capture"

# ⌘Z with the page focused puts them back in place, unloaded, and the tab shown stays.
send key z cmd
expect '^browser: command undo via page$' "⌘Z with the page focused didn't undo Clear"
expect "^browser: today [0-9][0-9]* $c [0-9][0-9]*\$" \
  "Undo didn't put the tabs back around the one shown: Today is \"$(today)\""
b=$(today | cut -d' ' -f3)
expect "^browser: page $b back 0 forward 0 loading 0 https://example.com/\$" \
  "the bottom tab didn't come back at its page"
expect "^browser: page $b unloaded 1\$" "the tab Undo brought back came back loaded"
none_since '^browser: selected' || fail "Undo showed another tab"
none_since '^browser: loaded' || fail "Undo loaded a page"

# The divider's Clear, under the pointer.
send move $clear_point
expect '^browser: sidebar hovered 1$' "the sidebar didn't see the pointer come over Clear"
expect '^browser: sidebar tools shown$' "Clear didn't fade in under the pointer"
send click $clear_point
expect '^browser: command clear via sidebar$' "a click on Clear didn't run it"
expect '^browser: cleared 2$' "Clear on the divider didn't close the two tabs not shown"
expect_lists "$a" "$c" "after Clear on the divider"

# With a Clear to undo, ⌘Z in the page's text field undoes typing there.
switch "1 cmd" "$a"
send click $field_point
send type ab
expect '^browser: title [0-9]* value ab$' "typing didn't reach the served page's text field"
send key z cmd
expect '^browser: title [0-9]* value a$' "⌘Z in the page's text field didn't undo typing there"
none_since '^browser: command undo' || fail "⌘Z in the page's text field undid Clear"

# ⇧⌘K with the UI focused and the pinned tab shown closes every Today tab, then finds none.
send click $sidebar_point
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K with the UI focused didn't run"
expect '^browser: cleared 1$' "⇧⌘K with the pinned tab shown didn't close the Today tab"
expect "^browser: tab $c closed\$" "⇧⌘K with the UI focused didn't close the Today tab"
expect_lists "$a" " *" "after Clear with the pinned tab shown"
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K without Today tabs didn't run"
none_since '^browser: cleared' || fail "Clear without Today tabs closed some"

# ⇧⌘T with the UI focused reopens the tab closed last and shows it.
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T with the UI focused didn't reach the UI"
expect "^browser: today [0-9][0-9]*\$" "⇧⌘T didn't reopen the tab Clear closed: Today is \"$(today)\""
d=$(today)
expect "^browser: selected $d\$" "⇧⌘T didn't show the tab it reopened"
expect "^browser: loaded $d https://example.org/\$" "the reopened tab didn't load its page"

# With the one Today tab shown, Clear has an arrow before it, lights up under the pointer, and
# closes that tab too, showing the pinned one. ⌘Z brings it back, shown, with the arrow.
expect '^browser: clear arrow 1$' "Clear had no arrow with the one Today tab shown"
send move $clear_point
expect '^browser: clear hovered 1$' "Clear didn't see the pointer come over it"
../tools/window-screenshot.sh "$pid" "$out/clear-arrow.png" || fail "no window to capture"
send click $clear_point
expect '^browser: command clear via sidebar$' "a click on Clear with its arrow didn't run it"
expect '^browser: cleared 1$' "Clear with its arrow didn't close the last Today tab"
expect "^browser: tab $d closed\$" "Clear with its arrow didn't close the tab shown"
expect_lists "$a" " *" "after clearing the last Today tab"
expect "^browser: selected $a\$" "clearing the last Today tab didn't show the pinned tab"
expect '^browser: clear arrow 0$' "Clear's arrow stayed with no Today tabs"
send key z cmd
expect '^browser: command undo via ui$' "⌘Z after clearing the last Today tab didn't run"
expect "^browser: today [0-9][0-9]*\$" "⌘Z didn't bring back the last Today tab: Today is \"$(today)\""
d=$(today)
expect "^browser: selected $d\$" "⌘Z didn't show the last Today tab again"
expect "^browser: loaded $d https://example.org/\$" "the last Today tab didn't load once shown"
expect '^browser: clear arrow 1$' "Clear's arrow didn't come back with the last Today tab"
../tools/window-screenshot.sh "$pid" "$out/clear-arrow-undone.png" || fail "no window to capture"

# ⇧⌘T with the page focused puts a tab back where it was, with its history.
open_from_bar example.net
e=$tab
go_from_bar example.org
expect_page example.org 1 0 "after going on from example.net"
open_from_bar example.com
f=$tab
switch "3 cmd" "$e"
# Muted, it comes back muted.
bar_command toggle-mute "Mute/Unmute Tab" mute
expect "^browser: page $e audible 0 muted 1\$" "Mute/Unmute Tab didn't mute the middle tab"
# With an icon, it comes back with it.
post rightclick $(row 2)
expect '^browser: menu shown .*Rename Tab, Change Icon…, ' \
  "a right-click on the middle Today row didn't offer Change Icon…"
send choose Change Icon…
expect "^browser: change icon $e\$" \
  "Change Icon… didn't put a field in the middle tab's icon's place"
send insert ★
expect "^browser: page $e icon ★\$" "the palette's pick didn't become the middle tab's icon"
send click $page_point
send key w cmd
expect "^browser: tab $e closed\$" "⌘W didn't close the middle Today tab"
expect_lists "$a" "$f $d" "after closing the middle Today tab"
send click $page_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via page$' "⇧⌘T with the page focused didn't reach the UI"
expect_lists "$a" "$f [0-9][0-9]* $d" "after ⇧⌘T, the tab isn't back between the two it was between"
e=$(today | cut -d' ' -f2)
tab=$e
expect "^browser: selected $e\$" "⇧⌘T didn't show the tab it reopened"
expect "^browser: page $e audible 0 muted 1\$" "⇧⌘T didn't keep the tab muted"
expect "^browser: page $e icon ★\$" "⇧⌘T didn't keep the tab's icon"
expect_page example.org 1 0 "after ⇧⌘T"
../tools/window-screenshot.sh "$pid" "$out/reopen.png" || fail "no window to capture"

# After Clear, ⇧⌘T reopens the tab it closed last, the top one, and ⌘Z with the UI focused brings
# back only the other.
send click $page_point
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: cleared 2$' "Clear didn't close the two tabs not shown"
expect_lists "$a" "$e" "after clearing around the tab shown"
send click $sidebar_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T after Clear didn't reach the UI"
expect "^browser: today [0-9][0-9]* $e\$" \
  "⇧⌘T after Clear didn't reopen the top tab: Today is \"$(today)\""
t=$(today | cut -d' ' -f1)
send key z cmd
expect '^browser: command undo via ui$' "⌘Z with the UI focused didn't run"
expect "^browser: today $t $e [0-9][0-9]*\$" \
  "Undo after ⇧⌘T didn't bring back just the bottom tab: Today is \"$(today)\""
none_since "^browser: selected" || fail "Undo after ⇧⌘T showed another tab"

# A right-click on a row shows its tab's menu, with the shortcuts. Duplicate Tab opens a copy under
# it, with its history and icon, and shows it. Rows count from the one pinned tab.
d=$(today | cut -d' ' -f3)
t=$(today | cut -d' ' -f1)
post rightclick $(row 2)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Remove Icon, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on a Today row didn't show its tab's menu"
../tools/window-screenshot.sh "$pid" "$out/tab-menu.png" || fail "no window to capture"
send choose Duplicate Tab
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect_lists "$a" "$t $e [0-9][0-9]* $d" "after Duplicate Tab"
tab=$(today | cut -d' ' -f3)
expect "^browser: selected $tab\$" "Duplicate Tab didn't show the copy"
expect "^browser: page $tab icon ★\$" "Duplicate Tab didn't copy the tab's icon"
expect_page example.org 1 0 "the copy Duplicate Tab made"
# Close Tabs Below shows the tab right-clicked first, so none below it loads on the way out.
post rightclick $(row 1)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Change Icon…, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on the top Today row didn't show its tab's menu"
send choose Close Tabs Below
expect '^browser: command close-tabs-below via tab menu$' "Close Tabs Below didn't run"
expect_lists "$a" "$t" "after Close Tabs Below"
expect "^browser: tab $e closed\$" "Close Tabs Below didn't close the tab under the one right-clicked"
expect "^browser: tab $d closed\$" "Close Tabs Below didn't close the bottom tab"
expect "^browser: selected $t\$" "Close Tabs Below didn't show the tab right-clicked"
[ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: selected ')" = 1 ] ||
  fail "Close Tabs Below showed a tab on the way out"
# ⇧⌘T brings them back one at a time, each at its place.
send key t cmd shift
expect "^browser: today $t [0-9][0-9]*\$" "⇧⌘T after Close Tabs Below didn't reopen the tab under the one right-clicked: Today is \"$(today)\""
# A pinned tab's menu unpins it, and goes back to its pinned URL only when it's away from it.
post rightclick $(pinned_row 0)
expect '^browser: menu shown Unpin Tab, Add to Favorites, Back to Pinned URL (disabled), Rename Tab, Change Icon…, Duplicate Tab, Move to Space (disabled), Close Tab$' "a right-click on a pinned row didn't show its tab's menu"
send choose Unpin Tab
expect '^browser: command toggle-pin via tab menu$' "Unpin Tab didn't run"
expect "^browser: today $a $t [0-9][0-9]*\$" "Unpin Tab didn't move the tab to the top of Today: Today is \"$(today)\""

# ⌘W archives the tab shown. ⇧⌘L opens the Library, which pushes the page right at its width, and
# a click on the archived tab reopens it at the top of Today with its history, shows it and closes
# the Library.
shown=$(grep '^browser: selected' "$log" | tail -1 | cut -d' ' -f3)
history=$(grep "^browser: page $shown back " "$log" | tail -1 | sed 's/^browser: page [0-9]* //')
url=${history##* }
send click $sidebar_point
send key w cmd
expect "^browser: tab $shown closed\$" "⌘W didn't close the tab shown"
expect '^browser: archive [1-9][0-9]* tabs$' "⌘W didn't archive the tab"
expect "^browser: today [0-9 ]*\$" "⌘W didn't take the tab out of Today"
left=$(today)
send layout
layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout')
content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
docked=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
width=${content%x*}
height=${content#*x}
send key l cmd shift
expect '^browser: command view-library via ui$' "⇧⌘L with the UI focused didn't reach the UI"
expect '^browser: library shown$' "⇧⌘L didn't open the Library"
send layout
view=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
[ "$view" = "417,10 ${docked#* }" ] ||
  fail "the Library didn't push the page to 417 at its width: it's at $view, docked at $docked"
../tools/window-screenshot.sh "$pid" "$out/library.png" || fail "no window to capture"
# A right-click on the archived tab shows its menu, whose Copy URL copies its URL.
post rightclick 300 111
expect '^browser: menu shown Restore Tab, Copy URL, Delete from Archive$' \
  "a right-click on the archived tab didn't show its menu"
../tools/window-screenshot.sh "$pid" "$out/archived-menu.png" || fail "no window to capture"
send choose Copy URL
expect "^browser: copied $url\$" "Copy URL in the archived tab's menu didn't copy its URL"
send click 300 111
expect '^browser: restore archived [0-9][0-9]*$' "a click on the archived tab didn't restore it"
expect "^browser: today [0-9][0-9]* $left\$" \
  "the archived tab didn't come back at the top of Today: Today is \"$(today)\""
r=$(today | cut -d' ' -f1)
expect "^browser: selected $r\$" "the tab from the archive isn't shown"
expect "^browser: loaded $r $url\$" "the tab from the archive didn't load its page"
expect "^browser: page $r $history\$" "the tab from the archive came back without its history"
expect '^browser: library hidden$' "reopening a tab didn't close the Library"
send layout
view=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
[ "$view" = "$docked" ] || fail "the page is at $view after the Library, not back at $docked"
# The Window menu's View Library opens it too, and the Archive menu's Clear Archive asks first,
# and return in the dialog empties it. The Library's back button closes it, and the footer's
# button opens it.
send menu View Library
expect '^browser: command view-library via ui$' "View Library didn't run"
expect '^browser: library shown$' "View Library didn't open the Library"
# Delete from Archive in a tab's menu takes it out, and the Library stays open.
archived=$(grep '^browser: archive [0-9]* tabs$' "$log" | tail -1 | cut -d' ' -f3)
post rightclick 300 111
expect '^browser: menu shown Restore Tab, Copy URL, Delete from Archive$' \
  "a right-click on an archived tab didn't show its menu"
send choose Delete from Archive
expect '^browser: remove archived [0-9][0-9]*$' "Delete from Archive didn't run"
expect "^browser: archive $((archived - 1)) tabs\$" "Delete from Archive didn't take the tab out"
none_since '^browser: library hidden$' || fail "Delete from Archive closed the Library"
send menu Clear Archive
expect '^browser: dialog shown Clear the archive?$' "Clear Archive didn't ask first"
send key return
expect '^browser: archive 0 tabs$' "Clear Archive didn't empty the archive"
../tools/window-screenshot.sh "$pid" "$out/library-empty.png" || fail "no window to capture"
send click 24 $((height - 24))
expect '^browser: command view-library via library$' "the Library's back button didn't run"
expect '^browser: library hidden$' "the back button didn't close the Library"
send click 24 $((height - 25))
expect '^browser: command view-library via sidebar$' "the footer's Library button didn't run"
expect '^browser: library shown$' "the footer's Library button didn't open the Library"
send key l cmd shift
expect '^browser: library hidden$' "⇧⌘L didn't close the Library"
# A copied URL leaves out the parameters that only tell a site where a click came from. On the
# check's own site: the tabs run types example.org, which this URL in history would complete to.
go_from_bar "${site#http://}?utm_source=check&id=1&fbclid=2"
expect "^browser: loaded $r ${site}.utm_source=check&id=1&fbclid=2\$" \
  "the tab didn't go to the URL with tracking parameters"
send key c cmd shift
expect "^browser: copied ${site}.id=1\$" \
  "⇧⌘C copied the URL with its tracking parameters: $(grep '^browser: copied' "$log" | tail -1)"

# A page's question about a permission shows under the URL field, as Chrome's hangs under its
# address bar. Another tab takes it down, and coming back brings it again. With the sidebar hidden,
# a question brings it out and keeps it out until it's answered. × dismisses it, and the site may
# ask again; Block refuses, and the site's next ask is refused without one; Allow grants.
# asks <site> <what>: the page asks, and the card is drawn at $qx $qy, $qw by $qh inside its edge.
asks() {
  expect "^browser: page $r asks $1 to Manage windows on all your displays\$" "$2"
  expect "^browser: question $r at " "the page's question wasn't drawn"
  send layout
  read -r qx qy qw qh <<EOF
$(grep "^browser: question $r at " "$log" | tail -1 | cut -d' ' -f5-)
EOF
}
send click $page_point
send key a
asks "${site%/}" "the page's question didn't come"
../tools/window-screenshot.sh "$pid" "$out/question.png" || fail "no window to capture"
open_from_bar example.net
expect "^browser: page $r asks nothing\$" "showing another tab didn't take the page's question down"
switch "$(($(pinned | wc -w) + 2)) cmd" "$r"
asks "${site%/}" "showing the asking tab again didn't bring its question back"
send click $((qx + qw - 18)) $((qy + 18))
expect "^browser: page $r asks nothing\$" "× didn't take the question down"
expect "^browser: title $r refused NotAllowedError\$" "× didn't refuse the page"
send key s cmd
expect '^browser: sidebar hidden$' "⌘S didn't hide the sidebar"
send click $page_point
send key a
expect '^browser: reveal shown$' "the page's question didn't bring the hidden sidebar out"
asks "${site%/}" "the site couldn't ask again after ×"
send move 600 400
none_since '^browser: reveal hidden$' || fail "the pointer leaving sent the question away"
../tools/window-screenshot.sh "$pid" "$out/question-reveal.png" || fail "no window to capture"
send click $((qx + qw / 4)) $((qy + qh - 26))
expect "^browser: page $r asks nothing\$" "Block didn't take the question down"
expect "^browser: title $r refused NotAllowedError\$" "Block didn't refuse the page"
send move 600 400
expect '^browser: reveal hidden$' "the pointer leaving didn't send the sidebar away once answered"
# A key that does nothing reports who has the keyboard, which the hidden panel must not.
send flags
in_browser RenderWidgetHostViewCocoa "the hidden sidebar's panel kept the keyboard"
send key s cmd
expect '^browser: sidebar shown$' "⌘S didn't bring the sidebar back"
send click $page_point
send key a
expect "^browser: title $r refused NotAllowedError\$" "the blocked site's next ask wasn't refused"
none_since "^browser: page $r asks http" || fail "the blocked site asked again"
# The page by its file's name, which no other run's history would complete it past.
go_from_bar "localhost:$port/index.html"
expect "^browser: loaded $r http://localhost:$port/index.html\$" "localhost didn't load in the tab"
send click $page_point
send key a
asks "http://localhost:$port" "another site's question didn't come"
send click $((qx + qw * 3 / 4)) $((qy + qh - 26))
expect "^browser: page $r asks nothing\$" "Allow didn't take the question down"
expect "^browser: title $r granted\$" "Allow didn't grant the page"

# The Site Controls list the site's permissions as Chrome's page info does, here the one Allow gave.
# The panel hangs from the button at 206 64: 238 wide from x 87, its beak's tip 9 pt above its body
# at y 91. With no extension pinned, its first row's middle is at y 262, the rows 43 apart, Reset
# permissions under them, and the footer's middle 74 below the last row's top.
controls="^browser: site controls http://localhost:$port: "
row_y() { echo $((262 + 43 * $1)); }
footer_y() { echo $((262 - 16 + 43 * ($(listed | wc -l)) + 74)); }
# The permissions the Site Controls listed last, one a line, in their order.
listed() {
  grep "$controls" "$log" | tail -1 | sed 's/^[^;]*; //' | tr ',' '\n' | sed 's/^ //'
}
send move $page_point
expect '^browser: sidebar tools hidden$' "Clear and the URL field's buttons didn't fade out"
send move $sidebar_point
expect '^browser: sidebar tools shown$' "the URL field's buttons didn't fade in"
send click $site_controls_point
expect '^browser: command site-controls via sidebar$' "a click on Site Controls didn't run it"
expect "${controls}[^;]*; .*Window management Allow" \
  "the Site Controls didn't list Window management as allowed"
expect '^browser: site controls shown$' "the Site Controls didn't show"
expect '^browser: site controls settled$' "the Site Controls didn't finish opening"
none_since '^browser: command edit-url' || fail "a click on Site Controls opened the command bar"
../tools/window-screenshot.sh "$pid" "$out/site-controls.png" || fail "no window to capture"
# Its value's menu offers Chrome's three, and Block blocks the site at once.
i=$(listed | grep -n '^Window management ' | cut -d: -f1)
post click 207 $(row_y $((i - 1)))
expect '^browser: menu shown Allow (checked), Block, Ask$' \
  "the value of Window management didn't offer Allow, Block and Ask"
send choose Block
expect '^browser: site controls set Window management Block$' "Block in the menu didn't run"
expect "${controls}[^;]*; .*Window management Block" \
  "the Site Controls didn't show Window management as blocked"
# http has no certificate to show, so the lock doesn't take the click.
send click 120 $(footer_y)
none_since '^browser: site controls run certificate' || fail "the lock ran without a certificate"
# Share closes them and hangs the system's share menu from its tile, at 122 118.
send click 122 118
expect '^browser: site controls run share$' "a click on Share didn't run it"
expect "^browser: share http://localhost:$port/index.html\$" "Share didn't share the page"
expect '^browser: site controls hidden$' "the Site Controls didn't leave for Share"
send click $site_controls_point
expect '^browser: site controls settled$' "the Site Controls didn't open after Share"
# The button again closes them.
send click $site_controls_point
expect '^browser: site controls closing (toggle)$' "Site Controls' second click didn't close them"
expect '^browser: site controls hidden$' "the Site Controls didn't leave"
# The page asks and is refused without a question.
send click $page_point
send key a
expect "^browser: title $r refused NotAllowedError\$" \
  "the site blocked in Site Controls wasn't refused"
none_since "^browser: page $r asks http" || fail "the site blocked in Site Controls asked"
# From the page, the command bar's Site Controls open them, which kept the change, and Escape closes
# them.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type site controls
expect '^browser: command bar commands "site controls": Site Controls$' \
  "the command bar didn't find Site Controls"
send key return
expect '^browser: command site-controls via command bar$' "Site Controls didn't run from the bar"
expect "${controls}[^;]*; .*Window management Block" "the Site Controls didn't keep Block"
expect '^browser: site controls settled$' "the Site Controls didn't open from the command bar"
../tools/window-screenshot.sh "$pid" "$out/site-controls-blocked.png" ||
  fail "no window to capture"
send key escape
expect '^browser: site controls closing (escape)$' "escape didn't close the Site Controls"
expect '^browser: site controls hidden$' "the Site Controls didn't leave after escape"
# Reset permissions sets the site back to asking, and a click outside closes them.
send click $site_controls_point
expect '^browser: site controls settled$' "the Site Controls didn't open again"
send click 207 $(row_y $(listed | wc -l))
expect '^browser: site controls reset$' "Reset permissions didn't run"
expect "${controls}" "the Site Controls didn't list the permissions again after the reset"
send click $page_point
expect '^browser: site controls closing (dismissed)$' \
  "a click outside didn't close the Site Controls"
expect '^browser: site controls hidden$' "the Site Controls didn't leave after a click outside"
send key a
asks "http://localhost:$port" "the site reset in Site Controls didn't ask again"
send click $((qx + qw * 3 / 4)) $((qy + qh - 26))
expect "^browser: title $r granted\$" "Allow didn't grant the page after the reset"
# The footer's … menu is Arc's without Boost's items, and All Site Settings… opens Chrome's for the
# site.
send click $site_controls_point
expect '^browser: site controls settled$' "the Site Controls didn't open for their … menu"
send click 298 $(footer_y)
more_menu='Clear Cache, Clear Cookies, Manage Extensions…, Add Extension…, All Site Settings…'
expect "^browser: menu shown $more_menu\$" "the … menu didn't list Arc's items"
send choose 'All Site Settings…'
expect '^browser: site controls run All Site Settings…$' "All Site Settings… didn't run"
site_settings="chrome://settings/content/siteDetails?site=http%3A%2F%2Flocalhost%3A$port"
expect "^browser: loaded [0-9]* $site_settings\$" "All Site Settings… didn't open them"
exec 3>&-
quit
echo "ok: Clear Cache and Refresh from the View menu fetches the page's script again, keeping its cookie, and Clear Cookies and Refresh from the command bar deletes its cookie, keeping the cached script. ⇧⌘C and ⌥⇧⌘C from the page and the UI copy the tab's URL and a Markdown link to it, without its tracking parameters, and ⇧⌘C's toast says Copied Current URL. The sidebar shows Clear and the URL field's button only while the pointer is over it, and the button copies the URL. ⇧⌘K from the page and the UI, and Clear on the divider, close the Today tabs but the one shown, with no toast, and ⌘Z from the page and the UI brings them back in place, unloaded, but not from the page's text field, where it undoes typing. With only the tab shown left, Clear has an arrow before it and closes that tab too, and ⌘Z brings it back, shown. ⇧⌘T from the page and the UI reopens the tab closed last, where it was, with its history, muted if it was and with its icon, and ⌘Z brings back only the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu: Duplicate Tab copies the tab with its history and icon under it, Close Tabs Below closes the Today tabs under it without showing them, and Unpin Tab unpins. ⌘W archives a Today tab, and ⇧⌘L, View Library and the footer's button open the Library, which pushes the page right at its width; a click on an archived tab reopens it at the top of Today with its history and closes the Library, a right-click on one shows its menu, whose Copy URL copies its URL and Delete from Archive takes it out; and Clear Archive asks, then empties it. A page's question about a permission shows under the URL field while its tab is shown, and brings a hidden sidebar out until it's answered: × dismisses it, Block refuses the site now and later, and Allow grants. The URL field's Site Controls button and the command bar's Site Controls list the site's permissions as Chrome does; Block there sticks and refuses the site, Reset permissions has it ask again, Share shares the page, the lock does nothing without a certificate, the … menu lists Arc's items and opens the site's settings, and the button, escape and a click outside close them. Windows: $out/clear-site.png, $out/url-toast.png, $out/sidebar-hovered.png, $out/sidebar-away.png, $out/clear.png, $out/clear-arrow.png, $out/clear-arrow-undone.png, $out/reopen.png, $out/tab-menu.png, $out/library.png, $out/archived-menu.png, $out/library-empty.png, $out/question.png, $out/question-reveal.png, $out/site-controls.png, $out/site-controls-blocked.png"
