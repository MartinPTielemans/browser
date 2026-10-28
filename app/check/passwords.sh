# A page's submitted login brings our offer to save its password, a card 18 in from the page
# card's top right corner, which keeps its place through a resize. Showing another tab closes it,
# and showing the tab again doesn't bring it back. Japanese input composes in its username field,
# where return commits it without saving; return saves the username as left in the field, and the
# login page fills the saved password in after. A changed password brings the offer to update it,
# and escape there declines. On another site, Never answers. Each run has a profile of its own,
# which no other run or the owner's copy reaches.

# The login page, in a new profile, whose passwords start empty.
serve
log="$out/passwords.log"
step=0
passwords_profile="$PWD/$out/passwords-profile"
BROWSER_DRIVE=1 BROWSER_URL="${site}login.html" BROWSER_PROFILE_DIR="$passwords_profile" \
  "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
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
# The page goes back to the login, which takes longer than the core reporting an offer.
go_from_bar "${site#http://}login.html"
expect "^browser: title $tab login\$" "the login page didn't load again"
sed -n "/^browser: drive $back: /,\$p" "$log" | grep -q '^browser: offer shown$' &&
  fail "showing the tab again brought its closed offer back"

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
exec 3>&-
quit
echo "ok: a submitted login offers to save its password in a card 18 from the page's top right, through a resize; another tab closes it for good; Japanese input composes in its username field and return commits it without saving; return saves the username as left, and the login page fills the saved password in; a changed password offers to update it, which escape declines; and Never answers on another site. Windows: $out/password-offer.png, $out/password-offer-ime.png, $out/password-update.png"
