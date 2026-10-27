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
# the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu, Arc's for a Today tab, whose
# Duplicate, Archive Tabs Below and Unpin Tab work. ⌘W archives a Today tab. ⇧⌘L, Window › Open
# Library… and the footer's button open the Library, which pushes the page right at its width, and
# its back button closes it. A right-click on an archived tab shows its menu, whose Copy URL copies
# its URL and Delete from Archive takes it out. A click on one reopens it at the top of Today with
# its history, and Clear Archive asks, then empties the archive. A page's question about a
# permission shows under the URL field while its tab is shown, bringing out a hidden sidebar, and
# its ×, Block and Allow answer it. The URL field's Site Controls button and the command bar's Site
# Controls, from the page, open a panel under the field that lists the site's permissions as
# Chrome's page info does. A value changed there sticks, and its Reset permissions sets them back.
# Escape, a click outside and the button close it. A page that asks for Widevine brings a note under
# the field, which ClearKey doesn't. From macOS 14 its button shows the page in Safari's engine, in
# the tab, and the profile keeps the site for it: the site's cookies, HttpOnly too, and its
# localStorage go there with it before the page's first request and script, another site goes back
# to Chromium, a link whose redirects pass through another site and come back stays, the site comes
# back to Safari's engine, keys, clicks and Japanese input work with its page focused, as they do in
# Chromium's, ⌘T to the site opens a new tab there, a private window's tab there has a session of
# its own, which the regular window and the next private window don't see, and the Site Controls
# list it and turn it off, which brings back to Chromium the cookies and the item its page stored.
# Before 14 the button opens the page in Safari, and the note goes once the tab is at another site.
# The pointer on a link shows its address in a plate in the card's bottom left corner, which goes
# when it leaves.

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
# The pointer on a link reports its address, which shows in a plate in the card's bottom left
# corner after a moment, and the plate goes when the pointer leaves the link. The page's link is
# 200 by 40 points, 100 in from the card's top left, which is 228 and 10 in from the window's.
link_point="430 130"
send move $link_point
expect "^browser: link target http://127.0.0.1:$port/notes.txt\$" \
  "the pointer on the link reported no address"
expect '^browser: link target plate ' "the link's address didn't show in a plate"
expect '^browser: link target window shown$' "the plate's window didn't come on screen"
../tools/window-screenshot.sh "$pid" "$out/link-target.png" || fail "no window to capture"
send move $page_point
expect '^browser: link target none$' "the pointer leaving the link didn't report it"
expect '^browser: link target hidden$' "the plate stayed after the pointer left the link"
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
# ⇧⌘Z closes them again, still without a toast, and ⌘Z brings them back once more.
send key z cmd shift
expect '^browser: command redo via page$' "⇧⌘Z with the page focused didn't redo Clear"
expect_lists "$a" "$c" "after redoing Clear"
none_since '^browser: toast' || fail "redoing Clear showed a toast"
send key z cmd
expect '^browser: command undo via page$' "⌘Z after redoing Clear didn't run"
expect "^browser: today [0-9][0-9]* $c [0-9][0-9]*\$" \
  "Undo after redoing Clear didn't put the tabs back around the one shown: Today is \"$(today)\""
b=$(today | cut -d' ' -f3)
expect "^browser: page $b unloaded 1\$" "the tab Undo brought back the second time came back loaded"

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
expect '^browser: menu shown .*, Change Icon…, Rename…, ' \
  "a right-click on the middle Today row didn't offer Change Icon…"
send choose Change Icon…
expect "^browser: icon picker for tab $e on emoji\$" \
  "Change Icon… didn't open the icon picker for the middle tab"
expect '^browser: icon picker shown$' "Change Icon… didn't show the icon picker under the middle tab"
send insert glowing star
send key return
expect "^browser: page $e icon 🌟\$" "the picker's pick didn't become the middle tab's icon"
expect '^browser: icon picker hidden$' "picking an emoji didn't close the icon picker"
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
expect "^browser: page $e icon 🌟\$" "⇧⌘T didn't keep the tab's icon"
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

# A right-click on a row shows its tab's menu, Arc's for a Today tab: Copy Link and Share, its
# look, Mute (Unmute while muted, dimmed while silent), its place, then Archive Tab and Archive Tabs
# Below. Duplicate opens a copy under it, with its history and icon, and shows it. Rows count from
# the one pinned tab. No check chooses Copy Link or Share.
d=$(today | cut -d' ' -f3)
t=$(today | cut -d' ' -f1)
post rightclick $(row 2)
expect "^browser: menu shown Copy Link, $share, Change Icon…, Remove Icon, Rename…, Unmute, Add Split View, Duplicate, Move to › [^,]* (checked), Move to › New Space…, Archive Tab, Archive Tabs Below\$" "a right-click on a Today row didn't show Arc's tab menu"
../tools/window-screenshot.sh "$pid" "$out/tab-menu.png" || fail "no window to capture"
send choose Duplicate
expect '^browser: command duplicate-tab via tab menu$' "Duplicate didn't run"
expect_lists "$a" "$t $e [0-9][0-9]* $d" "after Duplicate"
tab=$(today | cut -d' ' -f3)
expect "^browser: selected $tab\$" "Duplicate didn't show the copy"
expect "^browser: page $tab icon 🌟\$" "Duplicate didn't copy the tab's icon"
expect_page example.org 1 0 "the copy Duplicate made"
# The window is titled by the tab shown, which the Window menu lists it by. The menu bar names its
# items as Arc's, by what they'd do now. Edit has Paste and Match Style, which no check chooses.
send title
expect '^browser: key window titled Example Domain$' "the window isn't titled by the tab shown"
maybe='\( (disabled)\)\{0,1\}'
send menubar Edit
expect "^browser: menu bar Edit: .*, Paste$maybe, Paste and Match Style$maybe, Select All" \
  "the Edit menu has no Paste and Match Style after Paste"
send menubar View
expect "^browser: menu bar View: Hide Sidebar$maybe, " "View doesn't name ⌘S Hide Sidebar"
send menubar Tabs
expect "^browser: menu bar Tabs: .*, Pin Tab$maybe, .*, Reveal Tab in Sidebar$maybe, Clear Today" \
  "Tabs doesn't name ⌘D Pin Tab for a Today tab, or lacks Reveal Tab in Sidebar or Clear Today"
send menubar Spaces
expect "^browser: menu bar Spaces: New Space…$maybe, Rename Space$maybe, Change Profile › Default (checked)$maybe, Change Profile › New Profile (disabled), Next Space" \
  "Spaces has no Change Profile after Rename Space, with the Space's profile checked"
send menubar Archive
expect "^browser: menu bar Archive: .*, View Archive…$maybe, " "Archive has no View Archive…"
send menubar Window
expect "^browser: menu bar Window: .*, Open Library…$maybe, View Downloads…" \
  "Window has no Open Library… and View Downloads…"
# Archive Tabs Below shows the tab right-clicked first, so none below it loads on the way out.
post rightclick $(row 1)
expect "^browser: menu shown Copy Link, $share, Change Icon…, Rename…, Mute (disabled), Add Split View, Duplicate, Move to › [^,]* (checked), Move to › New Space…, Archive Tab, Archive Tabs Below\$" "a right-click on the top Today row didn't show Arc's tab menu"
send choose Archive Tabs Below
expect '^browser: command close-tabs-below via tab menu$' "Archive Tabs Below didn't run"
expect_lists "$a" "$t" "after Archive Tabs Below"
expect "^browser: tab $e closed\$" "Archive Tabs Below didn't close the tab under the one right-clicked"
expect "^browser: tab $d closed\$" "Archive Tabs Below didn't close the bottom tab"
expect "^browser: selected $t\$" "Archive Tabs Below didn't show the tab right-clicked"
[ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: selected ')" = 1 ] ||
  fail "Archive Tabs Below showed a tab on the way out"
# It says so with an Undo, and one ⌘Z brings back the three tabs under the one right-clicked, the
# copy Duplicate made among them, in place, and shows the copy again, as it was shown when they
# closed; ⇧⌘Z closes them again.
expect '^browser: toast shown Tabs Closed \[undo\]$' "Archive Tabs Below didn't offer Undo"
send key z cmd
expect '^browser: command undo via \(ui\|page\)$' "⌘Z didn't undo Archive Tabs Below"
expect "^browser: today $t [0-9][0-9]* [0-9][0-9]* [0-9][0-9]*\$" \
  "⌘Z didn't bring back the three tabs under the one right-clicked: Today is \"$(today)\""
copy=$(today | cut -d' ' -f3)
expect "^browser: selected $copy\( in .*\)\{0,1\}\$" \
  "⌘Z after Archive Tabs Below didn't show the copy that was shown when they closed"
[ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: selected ')" = 1 ] ||
  fail "⌘Z after Archive Tabs Below showed the tabs it brought back on the way"
send key z cmd shift
expect '^browser: command redo via \(ui\|page\)$' "⇧⌘Z didn't redo Archive Tabs Below"
expect_lists "$a" "$t" "after redoing Archive Tabs Below"
# ⇧⌘T brings them back one at a time, each at its place.
send key t cmd shift
expect "^browser: today $t [0-9][0-9]*\$" "⇧⌘T after Archive Tabs Below didn't reopen the tab under the one right-clicked: Today is \"$(today)\""
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
# The Window menu's Open Library… opens it too, and the Archive menu's Clear Archive asks first,
# and return in the dialog empties it. The Library's back button closes it, and the footer's
# button opens it.
send menu Open Library…
expect '^browser: command view-library via ui$' "Open Library… didn't run"
expect '^browser: library shown$' "Open Library… didn't open the Library"
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

# A page that asks for a key system Chromium can't play, such as Widevine, brings a note under the
# URL field. ClearKey plays, so a page asking for it brings none. The tab is the one shown, which
# All Site Settings… opened.
tab=$(grep '^browser: selected ' "$log" | tail -1 | cut -d' ' -f3)
go_from_bar "127.0.0.1:$port/clearkey.html"
expect "^browser: loaded $tab http://127.0.0.1:$port/clearkey.html\$" "the ClearKey page didn't load"
expect "^browser: title $tab clearkey asked\$" "the ClearKey page's request wasn't answered"
# The driver's round trip comes after the report the page's request would have made.
send layout
grep -q "^browser: page $tab needs " "$log" && fail "a page asking for ClearKey brought the note"
# The session page stores in Chromium a cookie from its server, HttpOnly, one from its script and
# an item, which go with the tab to Safari's engine below.
go_from_bar "127.0.0.1:$port/session?set"
expect "^browser: title $tab session chromium sees n item\$" \
  "the session page didn't store its cookie and item in Chromium"
go_from_bar "127.0.0.1:$port/drm.html"
expect "^browser: loaded $tab http://127.0.0.1:$port/drm.html\$" "the Widevine page didn't load"
expect "^browser: page $tab needs com.widevine.alpha\$" "the page asking for Widevine brought no note"
expect "^browser: drm note $tab at " "the note wasn't drawn"
send layout
read -r nx ny nw nh <<EOF
$(grep "^browser: drm note $tab at " "$log" | tail -1 | cut -d' ' -f6-)
EOF
../tools/window-screenshot.sh "$pid" "$out/drm-note.png" || fail "no window to capture"
# WebKit loads with the first page in it, or earlier only to clear a store a page made in an
# earlier run, as Clear Cookies and Refresh above does once this run has played. Read before the
# click, whose page can say it loaded WebKit before the next line runs.
early=$(grep '^browser: system engine loaded for ' "$log" | grep -v ' for clearing a site' || true)
send click $((nx + nw / 2)) $((ny + nh - 26))
# Safari's engine keeps a store for each profile from macOS 14; before it, the note offers Safari.
if [ "$(sw_vers -productVersion | cut -d. -f1)" -ge 14 ]; then
  drm_done="Safari's engine"
  [ -z "$early" ] || fail "WebKit loaded before its first page: $early"
  # Play in This Tab shows the page in Safari's engine, in the tab, and the note goes.
  expect "^browser: system engine $tab on\$" "the note's button didn't show the page in Safari's engine"
  expect "^browser: page $tab needs nothing\$" "the note stayed in Safari's engine"
  grep -q '^browser: system engine loaded for ' "$log" || fail "Safari's engine's page didn't say WebKit loaded"
  expect "^browser: loaded $tab http://127.0.0.1:$port/drm.html\$" "Safari's engine didn't load the page"
  expect "^browser: title $tab drm asked\$" "Safari's engine didn't report the page's title"
  # The site's session came first: the page's first request carried both cookies, and its script
  # found the item.
  copied="^browser: copied [0-9]* cookies and [0-9]* storage items for 127.0.0.1 into the system's engine\$"
  expect "$copied" "the switch didn't copy the site's session into Safari's engine"
  cookies=$(grep "$copied" "$log" | tail -1 | cut -d' ' -f3)
  items=$(grep "$copied" "$log" | tail -1 | cut -d' ' -f6)
  [ "$cookies" -ge 2 ] && [ "$items" -ge 1 ] ||
    fail "the switch copied $cookies cookies and $items storage items, not the session page's"
  served "^GET safari 127.0.0.1:$port/drm.html cookie .*h=$port; n=$port" \
    "Safari's engine's first request didn't carry the site's cookies from Chromium"
  served "^GET safari 127.0.0.1:$port/seen?item=chromium " \
    "Safari's engine's page didn't find the item from Chromium as its script ran"
  # Chromium's page let go for the switch isn't a renderer lost, which a crash card would show.
  grep -q "^.*browser: tab $tab page gone" "$log" && fail "the switch to Safari's engine read as the page gone"
  ../tools/window-screenshot.sh "$pid" "$out/drm-engine.png" || fail "no window to capture"
  # The site's other pages stay there; another site goes back to Chromium, in the same tab.
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine safari\$" "the site's next page left Safari's engine"
  go_from_bar "localhost:$port/engine.html"
  expect "^browser: tab $tab back to Chromium for http://localhost:$port/engine.html\$" \
    "another site didn't take the tab back to Chromium"
  expect "^browser: title $tab engine chromium\$" "another site stayed in Safari's engine"
  # The profile kept the site, which opens in Safari's engine with no note.
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine safari\$" "the site kept for Safari's engine opened in Chromium"
  # A link on the site whose redirects pass through another site and come back, as a sign-in's
  # through an identity site do, stays in Safari's engine. One whose redirects end at the other site
  # goes back to Chromium, as a link there does.
  go_from_bar "127.0.0.1:$port/hop.html"
  expect "^browser: title $tab hop safari\$" "the site's hop page left Safari's engine"
  send click $page_point
  expect "^browser: title $tab engine safari\$" \
    "a link's redirects through another site didn't come back in Safari's engine"
  none_since "^browser: tab $tab back to Chromium" ||
    fail "a link's redirects through another site took the tab back to Chromium"
  served "^GET safari localhost:$port/bounce" "the redirects didn't pass through the other site in Safari's engine"
  go_from_bar "127.0.0.1:$port/hop.html?away"
  expect "^browser: title $tab hop safari\$" "the site's hop page left Safari's engine"
  send click $page_point
  expect "^browser: tab $tab back to Chromium for http://localhost:$port/engine.html\$" \
    "a link whose redirects ended at another site didn't take the tab back to Chromium"
  expect "^browser: title $tab engine chromium\$" "a link's redirects to another site stayed in Safari's engine"
  # Chromium's view shows the page it kept, with its title, until the handback's load replaces it,
  # which would count a click from zero again.
  expect "^browser: loaded $tab http://localhost:$port/engine.html\$" "Chromium didn't load the handed-back page"
  # The command bar closes at a click outside it before the click is dispatched, so the click lands
  # on the page as it would with the bar closed (AGENTS.md › Over the page). Chromium's page counts
  # the click that closed the bar and the next, as Safari's engine's does below.
  send click $page_point
  expect "^browser: title $tab pressed 1\$" "a click didn't reach Chromium's page"
  send key l cmd
  expect '^browser: command bar shown$' "⌘L over Chromium's page didn't open the command bar"
  send click $page_point
  expect '^browser: command bar hidden$' "a click on Chromium's page didn't close the command bar"
  expect "^browser: title $tab pressed 2\$" \
    "the click that closed the bar didn't reach Chromium's page: $(grep "^browser: title $tab " "$log" | tail -1)"
  in_browser RenderWidgetHostViewCocoa "after a click on Chromium's page closed the command bar"
  send click $page_point
  expect "^browser: title $tab pressed 3\$" \
    "the click after the bar closed didn't reach Chromium's page: $(grep "^browser: title $tab " "$log" | tail -1)"
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine safari\$" "the site kept for Safari's engine opened in Chromium"
  # Keys with Safari's engine's page focused, as keys.sh has them with Chromium's. A click gives the
  # page the keyboard, which the note's button left with the UI, and the reserved shortcuts still
  # reach the UI.
  send click $page_point
  expect "^browser: title $tab pressed 1\$" "a click didn't reach Safari's engine's page"
  send state
  in_browser WKWebView "after a click on Safari's engine's page"
  for shortcut in "t new-tab" "l edit-url"; do
    page_shortcut $shortcut
    expect '^browser: command bar shown$' "$shortcut over Safari's engine didn't open the command bar"
    send key escape
    expect '^browser: command bar hidden$' "escape didn't close the command bar over Safari's engine"
    in_browser WKWebView "after the command bar over Safari's engine closed"
  done
  # A click in the page closes the bar before it's dispatched, so the page counts it and the next,
  # as Chromium's does above.
  send key l cmd
  expect '^browser: command bar shown$' "⌘L over Safari's engine didn't open the command bar"
  send click $page_point
  expect '^browser: command bar hidden$' "a click on Safari's engine's page didn't close the command bar"
  expect "^browser: title $tab pressed 2\$" \
    "the click that closed the bar didn't reach Safari's engine's page: $(grep "^browser: title $tab " "$log" | tail -1)"
  in_browser WKWebView "after a click on Safari's engine's page closed the command bar"
  send click $page_point
  expect "^browser: title $tab pressed 3\$" \
    "the click after the bar closed didn't reach Safari's engine's page: $(grep "^browser: title $tab " "$log" | tail -1)"
  # In the page's text field, ⌘A and ⌘C stay with the page.
  send type abc
  expect "^browser: title $tab value abc\$" "typing didn't reach the text field in Safari's engine"
  for shortcut in "a select" "c copy"; do
    read -r letter did <<<"$shortcut"
    send key "$letter" cmd
    expect "^browser: title $tab $did abc\$" \
      "⌘$(echo "$letter" | tr a-z A-Z) in Safari's engine's text field didn't $did its text"
    none_since '^browser: command [a-z-]* via page$' ||
      fail "⌘$(echo "$letter" | tr a-z A-Z) in Safari's engine's text field reached the UI"
  done
  # Japanese input composes in the field and commits there, replacing what ⌘A selected.
  send ime on
  send type nihongo
  send key space
  send key return
  expect "^browser: title $tab committed 日本語\$" \
    "a Japanese composition didn't commit into Safari's engine's text field"
  send ime off
  send type x
  expect "^browser: title $tab value 日本語x\$" \
    "return in Safari's engine's text field did more than commit: $(grep "^browser: title $tab " "$log" | tail -1)"
  ../tools/window-screenshot.sh "$pid" "$out/engine-ime.png" || fail "no window to capture"
  # ⌘W from the page closes the tab, and the site kept opens in Safari's engine in the next one shown.
  page_shortcut w close-tab
  expect '^browser: selected [1-9]' "⌘W over Safari's engine showed no tab"
  closed=$tab
  tab=$(grep '^browser: selected ' "$log" | tail -1 | cut -d' ' -f3)
  [ "$tab" != "$closed" ] || fail "⌘W over Safari's engine left its tab shown"
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine safari\$" "the site kept for Safari's engine opened in Chromium in another tab"
  # Safari's engine's page stores a cookie from its server, HttpOnly, one from its script and an
  # item. Its script sees Chromium's cookie and item, and neither HttpOnly cookie.
  go_from_bar "127.0.0.1:$port/session?webkit"
  expect "^browser: title $tab session safari sees.* n w item back\$" \
    "Safari's engine's session page didn't see Chromium's cookie and item, or keep its own"
  grep "^browser: title $tab session safari sees " "$log" | tail -1 | grep -qwE "h|hw" &&
    fail "an HttpOnly cookie reached the script in Safari's engine"
  # ⌘T to the site opens a new tab in Safari's engine, though Chrome starts a new tab's first load
  # before the core has the tab. The new tab is the one shown from here.
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  send insert "127.0.0.1:$port/engine.html"
  send key return
  expect "^browser: loaded [0-9]* http://127.0.0.1:$port/engine.html\$" "⌘T to the site kept for Safari's engine loaded nothing"
  opened=$(sed -n "s|^browser: loaded \([0-9]*\) http://127.0.0.1:$port/engine.html\$|\1|p" "$log" | tail -1)
  [ "$opened" != "$tab" ] || fail "⌘T to the site loaded it in the tab shown"
  expect "^browser: title $opened engine safari\$" \
    "⌘T to the site kept for Safari's engine opened it in Chromium: $(grep "^browser: title $opened " "$log" | tail -1)"
  expect "$copied" "the new tab didn't take the site's session into Safari's engine"
  tab=$opened
  # A private window's tab at the site opens in Safari's engine too, in a session of its own that
  # starts from the private window's and ends with it: the regular window doesn't see what its page
  # stored, and nor does the next private window. The page names what it finds, never a value.
  # Opens the session page at query $1 from ⌘T's bar in the key window, the private window $2, and
  # sets $private_tab to its tab.
  private_session_page() {
    send key t cmd
    expect '^browser: command bar shown$' "⌘T didn't open the command bar in the private window"
    send insert "127.0.0.1:$port/session$1"
    send key return
    expect "^browser: loaded [0-9]* http://127.0.0.1:$port/session$1\$" \
      "the private window didn't load the session page"
    private_tab=$(sed -n "s|^browser: loaded \([0-9]*\) http://127.0.0.1:$port/session$1\$|\1|p" \
      "$log" | tail -1)
    expect "^browser: window $2 shows $private_tab\$" "the private window didn't show the session page"
  }
  send key n cmd shift
  expect '^browser: private window opened [0-9]* profile [0-9]*$' "⇧⌘N didn't open a private window"
  private=$(grep '^browser: private window opened ' "$log" | tail -1 | cut -d' ' -f5)
  send window 2
  private_session_page "?private" "$private"
  expect "^browser: copied 0 cookies and 0 storage items for 127.0.0.1 into the system's engine\$" \
    "the private tab didn't start Safari's engine from the private session, which has none of the site's"
  expect "^browser: title $private_tab session safari sees p mine\$" \
    "the private window's page didn't store its cookie and item in Safari's engine: $(grep "^browser: title $private_tab " "$log" | tail -1)"
  send window 1
  go_from_bar "127.0.0.1:$port/session"
  expect "^browser: title $tab session safari sees " "the regular window's session page didn't load in Safari's engine"
  grep "^browser: title $tab session safari sees " "$log" | tail -1 | grep -qwE "p|mine" &&
    fail "the regular window's Safari's engine saw what the private window's page stored"
  send window 2
  send key w cmd shift
  expect "^browser: window closed $private\$" "⇧⌘W didn't close the private window"
  # await, which says so if the app has exited. Its line can only be this close's: none came before.
  await '^browser: private session ended [0-9]*$' ||
    fail "the private window's closing didn't end its session"
  send window 1
  send key n cmd shift
  expect '^browser: private window opened [0-9]* profile [0-9]*$' "⇧⌘N didn't open a private window again"
  private=$(grep '^browser: private window opened ' "$log" | tail -1 | cut -d' ' -f5)
  send window 2
  private_session_page "" "$private"
  expect "^browser: title $private_tab session safari sees none\$" \
    "the next private window's Safari's engine saw the ended session's values, or the regular window's: $(grep "^browser: title $private_tab " "$log" | tail -1)"
  send key w cmd shift
  expect "^browser: window closed $private\$" "⇧⌘W didn't close the next private window"
  send window 1
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine safari\$" "the site's engine page left Safari's engine"
  # The Site Controls list it in the first row, and Off forgets it, brings its session back to
  # Chromium and shows the page there.
  send move $page_point
  send move $sidebar_point
  expect '^browser: sidebar tools shown$' "the URL field's buttons didn't fade in"
  send click $site_controls_point
  expect "^browser: site controls http://127.0.0.1:$port: .*; Safari's engine on\$" \
    "the Site Controls didn't list Safari's engine"
  expect '^browser: site controls settled$' "the Site Controls didn't open in Safari's engine"
  ../tools/window-screenshot.sh "$pid" "$out/site-controls-engine.png" || fail "no window to capture"
  post click 207 $(row_y 0)
  expect "^browser: menu shown On (checked), Off\$" "Safari's engine's row didn't offer On and Off"
  send choose Off
  expect "^browser: site controls set Safari's engine Off\$" "Off in the menu didn't run"
  expect "^browser: copied [0-9]* cookies and [0-9]* storage items for 127.0.0.1 back to Chromium\$" \
    "Off didn't copy the site's session back to Chromium"
  expect "^browser: title $tab engine chromium\$" "Off didn't show the page in Chromium"
  # Chromium's page, taking the pane, may have closed them already.
  if none_since '^browser: site controls hidden$'; then
    send key escape
    expect '^browser: site controls hidden$' "escape didn't close the Site Controls"
  fi
  go_from_bar "127.0.0.1:$port/engine.html"
  expect "^browser: title $tab engine chromium\$" "the site turned off went to Safari's engine again"
  # Chromium has what Safari's engine's page stored: its request carries both of that page's
  # cookies, and its script sees the one it may and the item.
  go_from_bar "127.0.0.1:$port/session"
  expect "^browser: title $tab session chromium sees.* w item back\$" \
    "Chromium didn't get the cookie and the item Safari's engine's page stored"
  grep "^browser: title $tab session chromium sees " "$log" | tail -1 | grep -qwE "h|hw" &&
    fail "an HttpOnly cookie reached the script in Chromium"
  served "^GET chromium 127.0.0.1:$port/session cookie .*hw=$port.*w=$port" \
    "Chromium's request didn't carry the HttpOnly cookie Safari's engine's server set"
  go_from_bar "localhost:$port/index.html"
  expect "^browser: loaded $tab http://localhost:$port/index.html\$" "localhost didn't load again"
else
  drm_done="Safari"
  expect "^browser: open in safari http://127.0.0.1:$port/drm.html\$" \
    "the note's button didn't open the page in Safari"
  none_since "^browser: page $tab needs nothing\$" || fail "the button took the note away"
  go_from_bar "localhost:$port/index.html"
  expect "^browser: loaded $tab http://localhost:$port/index.html\$" "localhost didn't load again"
  expect "^browser: page $tab needs nothing\$" "the note stayed at another site"
fi
exec 3>&-
quit
echo "ok: the pointer on a link shows its address in a plate that goes when the pointer leaves it, Clear Cache and Refresh from the View menu fetches the page's script again, keeping its cookie, and Clear Cookies and Refresh from the command bar deletes its cookie, keeping the cached script. ⇧⌘C and ⌥⇧⌘C from the page and the UI copy the tab's URL and a Markdown link to it, without its tracking parameters, and ⇧⌘C's toast says Copied Current URL. The sidebar shows Clear and the URL field's button only while the pointer is over it, and the button copies the URL. ⇧⌘K from the page and the UI, and Clear on the divider, close the Today tabs but the one shown, with no toast, and ⌘Z from the page and the UI brings them back in place, unloaded, but not from the page's text field, where it undoes typing. With only the tab shown left, Clear has an arrow before it and closes that tab too, and ⌘Z brings it back, shown. ⇧⌘T from the page and the UI reopens the tab closed last, where it was, with its history, muted if it was and with its icon, and ⌘Z brings back only the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu, Arc's for a Today tab: Duplicate copies the tab with its history and icon under it, Archive Tabs Below closes the Today tabs under it without showing them, and Unpin Tab unpins. ⌘W archives a Today tab, and ⇧⌘L, Window › Open Library… and the footer's button open the Library, which pushes the page right at its width; a click on an archived tab reopens it at the top of Today with its history and closes the Library, a right-click on one shows its menu, whose Copy URL copies its URL and Delete from Archive takes it out; and Clear Archive asks, then empties it. A page's question about a permission shows under the URL field while its tab is shown, and brings a hidden sidebar out until it's answered: × dismisses it, Block refuses the site now and later, and Allow grants. The URL field's Site Controls button and the command bar's Site Controls list the site's permissions as Chrome does; Block there sticks and refuses the site, Reset permissions has it ask again, Share shares the page, the lock does nothing without a certificate, the … menu lists Arc's items and opens the site's settings, and the button, escape and a click outside close them. A page that asks for Widevine brings a note under the URL field, while one that asks for ClearKey brings none. Its button used $drm_done: Safari's engine shows the page in the tab with the site's cookies, HttpOnly too, and localStorage from Chromium there before its first request and script, keeps the site for it until the Site Controls turn it off, which brings the cookies and the item its page stored back to Chromium, gives another site back to Chromium, keeps a link whose redirects pass through another site and come back, and gives back one whose redirects end there, opens a new tab from ⌘T at the site there, gives a private window's tab there a session of its own, which the regular window doesn't see and which ends with the window, and with its page focused ⌘T, ⌘L and ⌘W reach the UI, the click that closes the command bar reaches the page, as it does Chromium's, and in its text field ⌘A and ⌘C stay with the page and a Japanese composition commits; Safari opens the page there, leaving the note until the tab is at another site. ⇧⌘Z redoes Clear, and Archive Tabs Below offers Undo, which brings back its tabs in place and shows the one that was shown, and ⇧⌘Z closes them again. Windows: $out/link-target.png, $out/clear-site.png, $out/url-toast.png, $out/sidebar-hovered.png, $out/sidebar-away.png, $out/clear.png, $out/clear-arrow.png, $out/clear-arrow-undone.png, $out/reopen.png, $out/tab-menu.png, $out/library.png, $out/archived-menu.png, $out/library-empty.png, $out/question.png, $out/question-reveal.png, $out/site-controls.png, $out/site-controls-blocked.png, $out/drm-note.png, and from macOS 14 $out/drm-engine.png, $out/engine-ime.png and $out/site-controls-engine.png"
