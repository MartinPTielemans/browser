# A page's submitted login brings our offer to save its password, a card 18 in from the page
# card's top right corner, which keeps its place through a resize. Showing another tab closes it,
# and showing the tab again doesn't bring it back, but the key after the host in the URL field
# does. Japanese input composes in its username field, where return commits it without saving;
# return saves the username as left in the field, and the login page fills the saved password in
# after, with a key that opens the Site Controls. Chrome's dropdown under the username field fills
# the login in again once the password is cleared. A changed password brings the offer to update it,
# and escape there declines. On another site, Never answers, and that site's Site Controls list it
# in a Passwords row whose menu takes it off Never. On the first site, the row names the login,
# All Passwords… lists it in Settings, whose Remove asks first, and the row's Remove asks in the
# browser window's dialog and removes it, which Settings hears. A form's address brings Chrome's
# offer to save it, which return saves, and a card's, which escape declines. Each run has a
# profile of its own, which no other run or the owner's copy reaches.

# The login page, in a new profile, whose passwords start empty.
serve
log="$out/passwords.log"
step=0
passwords_profile="$PWD/$out/passwords-profile"
BROWSER_DRIVE=1 BROWSER_URL="${site}login.html" BROWSER_PROFILE_DIR="$passwords_profile" \
  "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* login$' || fail "the login page didn't draw"
tab=$(today)
# The username field, across the top of the page, and the password field under it.
send click 600 60
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: passwords. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Logs in on the page shown with password $1: the username field has the keyboard.
log_in() {
  send key a cmd
  send type check@example.test
  send key tab
  send key a cmd
  send type "$1"
  send key return
}
# Waits for the page's login to bring an offer of kind $1, and for its card to take input. Sets $ox
# and $oy to where the card is in the browser window.
offered() {
  expect "^browser: offer $tab $1 check@example.test " "the login brought no offer to $1"
  expect '^browser: offer shown$' "the offer to $1 wasn't shown"
  expect "^browser: offer $tab takes input\$" "the offer to $1 never took input"
  send layout
  child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep '^browser: layout child' |
    grep ' 320x180$' | head -1 | sed 's/^browser: layout child //')
  [ -n "$child" ] || fail "the offer's card isn't over the browser window"
  ox=${child%%,*}
  oy=${child#*,}
  oy=${oy%% *}
}
# The offer's window has the keyboard, in its username field.
in_offer() {
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "$1: the keyboard is in $key, on $focus"
}
# Where to click in the offer's username field, and on its Never, at the left of its buttons.
offer_field() {
  echo "$((ox + 56)) $((oy + 70))"
}
offer_never() {
  echo "$((ox + 24)) $((oy + 150))"
}
# The card is 18 from the top right corner of the window's content, 8 in from the page card's.
check_offer() {
  content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    sed 's/^browser: layout content \([0-9]*\)x.*/\1/')
  [ "$child" = "$((content - 338)),18 320x180" ] ||
    fail "$1: the offer is at $child in a window $content wide"
}

log_in hunter2.check
offered save
check_offer "after the login"
../tools/window-screenshot.sh "$pid" "$out/password-offer.png" || fail "no window to capture"
send resize 1000 700
send layout
child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep '^browser: layout child' |
  grep ' 320x180$' | head -1 | sed 's/^browser: layout child //')
check_offer "after a resize"

# Another tab closes the offer, and coming back doesn't bring it back.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type "${site#http://}welcome.html"
send key return
expect '^browser: offer hidden$' "showing another tab didn't hide the offer"
expect "^browser: offer $tab gone\$" "showing another tab didn't close Chrome's offer"
switch "$(($(pinned | wc -w) + 2)) cmd" "$tab"
back=$step
# The command bar opening and closing takes longer than the core reporting an offer.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
sed -n "/^browser: drive $back: /,\$p" "$log" | grep -q '^browser: offer shown$' &&
  fail "showing the tab again brought its closed offer back"
# The key after the host stays for the offer, and a click on it brings the offer back, which Not
# Now answers.
grep -q "^browser: key $tab offer\$" "$log" || fail "the offer gave its tab no key"
# Where the key was drawn last, its centre in the window.
key_at() { grep '^browser: key drawn at ' "$log" | tail -1 | cut -d' ' -f5-6; }
[ -n "$(key_at)" ] || fail "the key wasn't drawn"
../tools/window-screenshot.sh "$pid" "$out/password-key.png" || fail "no window to capture"
send click $(key_at)
expect "^browser: key $tab clicked\$" "a click on the key didn't reach it"
offered save
send click $(offer_field)
send key escape
expect "^browser: offer $tab answered NotNow check@example.test\$" \
  "escape didn't decline the offer the key brought back"
expect '^browser: offer hidden$' "declining didn't hide the offer the key brought back"
go_from_bar "${site#http://}login.html"
expect "^browser: title $tab login\$" "the login page didn't load again"

# Japanese input composes in the offer's username field, and return commits it without saving.
send click 600 60
log_in hunter2.check
offered save
send click $(offer_field)
in_offer "after a click in the offer"
send key a cmd
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the offer's field didn't take the composition: marked $marked"
send key space
[ "$marked" = 0,3 ] || fail "the offer's field didn't take the conversion: marked $marked"
send key return
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
none_since "^browser: offer $tab answered" || fail "return committing the composition answered"
../tools/window-screenshot.sh "$pid" "$out/password-offer-ime.png" || fail "no window to capture"
# Return saves the username as the field has it.
send key a cmd
send type check@example.test
send key return
expect "^browser: offer $tab answered Accept check@example.test\$" "return didn't save"
expect '^browser: offer hidden$' "saving didn't hide the offer"
expect "^browser: offer $tab gone\$" "saving didn't close Chrome's offer"

# The login page fills the saved password in. A changed one brings the offer to update it, and
# escape declines.
go_from_bar "${site#http://}login.html"
expect "^browser: title $tab filled\$" "the login page didn't fill the saved password in"
# The saved login gives the page a key, and a click on it opens the Site Controls, which list it.
await "^browser: key $tab saved\$" || fail "the saved login gave the page no key"
send click $(key_at)
expect "^browser: key $tab clicked\$" "a click on the saved login's key didn't reach it"
expect '^browser: command site-controls via key$' \
  "the saved login's key didn't open the Site Controls"
expect '^browser: site controls settled$' "the Site Controls the key opened didn't settle"
send key escape
expect '^browser: site controls hidden$' "escape didn't close the Site Controls the key opened"
# With the password cleared, a click in the username field shows Chrome's dropdown under it, where
# down previews the saved login and return fills it in again.
send click 600 60
send key tab
send key a cmd
send key backspace
expect "^browser: title $tab cleared\$" "clearing the password didn't undo the fill"
send click 600 60
send key down
expect "^browser: title $tab filled\$" "down didn't preview the saved login"
../tools/window-screenshot.sh "$pid" "$out/password-fill.png" || fail "no window to capture"
# Chrome ignores a suggestion accepted in the first 500 ms its dropdown is drawn
# (kIgnoreEarlyClicksOnSuggestionsDuration), and nothing marks when that ends.
sleep 0.6
send key return
expect "^browser: title $tab chosen\$" "Chrome's dropdown didn't fill the saved login in"
send click 600 60
send key tab
send key a cmd
send type hunter3.check
send key return
offered update
../tools/window-screenshot.sh "$pid" "$out/password-update.png" || fail "no window to capture"
send click $(offer_field)
in_offer "after a click in the update offer"
send key escape
expect "^browser: offer $tab answered NotNow check@example.test\$" "escape didn't decline"
expect '^browser: offer hidden$' "declining didn't hide the offer"

# On another site, Never answers.
go_from_bar "localhost:$port/login.html"
expect "^browser: title $tab login\$" "the login page didn't load on localhost"
send click 600 60
log_in hunter2.check
offered save
send click $(offer_never)
expect "^browser: offer $tab answered Never check@example.test\$" "Never didn't answer"
expect '^browser: offer hidden$' "Never didn't hide the offer"

# The Site Controls list the site set to Never in a Passwords row, whose menu takes it off Never at
# once. With no extension pinned and no permission set, it's the row under ad blocking's, whose
# middle is at y 262 (clear.sh measures the panel), its own 43 under it at y 305.
last_controls() { grep '^browser: site controls http' "$log" | tail -1; }
controls="^browser: site controls http://localhost:$port: "
bar_command site-controls "Site Controls" "site controls"
expect "${controls}.*; passwords never localhost:$port\$" \
  "the Site Controls didn't list localhost as set to Never: $(last_controls)"
expect '^browser: site controls settled$' "the Site Controls didn't open"
../tools/window-screenshot.sh "$pid" "$out/site-controls-never.png" || fail "no window to capture"
post click 207 305
expect "^browser: menu shown $(cased 'Offer to Save Passwords, All Passwords…')\$" \
  "the Passwords row didn't offer to take localhost off Never"
send choose "$(cased 'Offer to Save Passwords')"
expect "^browser: site controls allow saving localhost:$port\$" "Offer to Save Passwords didn't run"
expect "^browser: remove login [0-9]* never localhost:$port\$" \
  "Offer to Save Passwords didn't take localhost off Never"
expect "${controls}[^;]*; [^;]*\$" \
  "the Site Controls kept the Passwords row once localhost was off Never: $(last_controls)"
send key escape
expect '^browser: site controls hidden$' "escape didn't close the Site Controls"

# On the first site the row names the saved login, and All Passwords… lists it in Settings, whose
# Remove asks in an alert, where Cancel keeps it. With one profile there's no pop-up, so the first
# login's row is the Shortcuts tab's first row, its Remove at the box's right.
go_from_bar "${site#http://}login.html"
expect "^browser: title $tab filled\$" "the login page didn't fill the saved password in again"
controls="^browser: site controls http://127.0.0.1:$port: "
bar_command site-controls "Site Controls" "site controls"
expect "${controls}.*; passwords 127.0.0.1:$port check@example.test\$" \
  "the Site Controls didn't list the saved login: $(last_controls)"
expect '^browser: site controls settled$' "the Site Controls didn't open on the first site"
../tools/window-screenshot.sh "$pid" "$out/site-controls-passwords.png" ||
  fail "no window to capture"
post click 207 305
expect "^browser: menu shown $(cased 'Remove “check@example.test”…, All Passwords…')\$" \
  "the Passwords row didn't offer to remove the saved login"
send choose "$(cased 'All Passwords…')"
expect '^browser: site controls run passwords$' "All Passwords… didn't run"
expect '^browser: settings tab Passwords$' "All Passwords… didn't show Settings' Passwords tab"
expect "^browser: passwords [0-9]*: 127.0.0.1:$port check@example.test\$" \
  "Settings didn't list the saved login alone: $(grep '^browser: passwords ' "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/settings-passwords.png" || fail "no window to capture"
post clickkey 574 159
expect "^browser: alert shown Remove the password for “check@example.test” on 127.0.0.1:$port?\$" \
  "Remove in Settings didn't ask first"
send choose Cancel
none_since '^browser: remove login' || fail "Cancel removed the login"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"

# The row's Remove closes the panel and asks in the browser window's dialog, where return removes
# the login, and Settings, which asked for the logins, hears that it's gone. A click on the page
# gives the browser window the keyboard back.
send click $page_point
bar_command site-controls "Site Controls" "site controls"
expect '^browser: site controls settled$' "the Site Controls didn't open again"
post click 207 305
expect "^browser: menu shown $(cased 'Remove “check@example.test”…, All Passwords…')\$" \
  "the Passwords row didn't offer to remove the saved login again"
send choose 'Remove “check@example.test”…'
expect '^browser: site controls run remove-login$' "Remove didn't run"
expect "^browser: dialog shown Remove the password for “check@example.test” on 127.0.0.1:$port?\$" \
  "Remove didn't ask first"
expect '^browser: dialog settled$' "the dialog asking to remove the login didn't settle"
../tools/window-screenshot.sh "$pid" "$out/password-remove.png" || fail "no window to capture"
send key return
expect "^browser: remove login [0-9]* 127.0.0.1:$port check@example.test\$" \
  "return didn't remove the login"
expect "^browser: passwords [0-9]*: none\$" "Settings didn't hear the login was removed"

# A form's address brings Chrome's offer to save it, in the same card with its lines to read and
# the keyboard on the card, where return saves it; the form then offers it in Chrome's dropdown.
# A card's offer names it by its last four digits, and escape declines it. Made-up data only.
# Types $1 in the form's first field, then each of the rest after a tab, and submits it.
submit_form() {
  send click 600 60
  send type "$1"
  shift
  for value in "$@"; do
    send key tab
    send type "$value"
  done
  send key return
}
# Waits for the page's form to bring an offer of kind $1, and for its card to take input.
offered_to_save() {
  expect "^browser: offer $tab $1 [0-9]*\$" "the $1 form brought no offer to save it"
  expect '^browser: offer shown$' "the offer to save the $1 wasn't shown"
  expect "^browser: offer $tab takes input\$" "the offer to save the $1 never took input"
}
go_from_bar "${site#http://}address.html"
expect "^browser: title $tab address\$" "the address page didn't load"
submit_form 'Ada Lovelace' '1 Check Street' Springfield IL 62701 'United States' ada@example.test
offered_to_save address
../tools/window-screenshot.sh "$pid" "$out/address-offer.png" || fail "no window to capture"
send key return
expect "^browser: offer $tab answered Accept \$" "return didn't save the address"
expect "^browser: offer $tab gone\$" "saving the address left Chrome's offer open"
go_from_bar "${site#http://}address.html"
expect "^browser: title $tab address\$" "the address page didn't load again"
send click 600 60
send key down
expect "^browser: title $tab filled\$" "Chrome's dropdown didn't offer the saved address"
../tools/window-screenshot.sh "$pid" "$out/address-fill.png" || fail "no window to capture"
send key escape
go_from_bar "${site#http://}card.html"
expect "^browser: title $tab card\$" "the card page didn't load"
submit_form 'Ada Lovelace' 4111111111111111 12 2030
offered_to_save card
../tools/window-screenshot.sh "$pid" "$out/card-offer.png" || fail "no window to capture"
send key escape
expect "^browser: offer $tab answered NotNow \$" "escape didn't decline the card"
expect "^browser: offer $tab gone\$" "declining the card left Chrome's offer open"
exec 3>&-
quit
echo "ok: a submitted login offers to save its password in a card 18 from the page's top right, through a resize; another tab closes it, and the key after the host brings it back; Japanese input composes in its username field and return commits it without saving; return saves the username as left, and the login page fills the saved password in, with a key that opens the Site Controls, and Chrome's dropdown fills it in again; a changed password offers to update it, which escape declines; and Never answers on another site, whose Site Controls take it off Never; the first site's Site Controls name its login, All Passwords… lists it in Settings, where Remove asks and Cancel keeps it, and the row's Remove asks in the dialog and removes it, which Settings hears; a form's address offers to save it in the card, return saves it and Chrome's dropdown then offers it, and a card's offer declines with escape. Windows: $out/password-offer.png, $out/password-key.png, $out/password-fill.png, $out/password-offer-ime.png, $out/password-update.png, $out/site-controls-never.png, $out/site-controls-passwords.png, $out/settings-passwords.png, $out/password-remove.png, $out/address-offer.png, $out/address-fill.png, $out/card-offer.png"
