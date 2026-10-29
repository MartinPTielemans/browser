# An address form's submitted address brings our offer to save it, in Chrome's words, where the
# password's offer is, and return saves it. On the form again, Edit › AutoFill › Contact… opens
# Chrome's dropdown at the field that has the keyboard, where down previews the saved address. A
# card form's submitted card brings the offer to save the card, which escape declines; another card
# brings it again, and return saves that one. Edit › AutoFill › Credit Card… opens Chrome's
# dropdown at the card's field, which says filling isn't secure on http. Every address and card is
# made up, in a profile of the run's own, which no other run or the owner's copy reaches.

serve
log="$out/autofill.log"
step=0
BROWSER_DRIVE=1 BROWSER_URL="${site}address.html" BROWSER_PROFILE_DIR="$PWD/$out/autofill-profile" \
  "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* address$' || fail "the address form didn't draw"
tab=$(today)
# The first field, across the top of the page.
send click 600 30
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: autofill. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Fills the form shown with the values $@, a field each from the first, which has the keyboard,
# and submits it.
submit() {
  for value in "$@"; do
    send insert "$value"
    send key tab
  done
  send key return
}
# Waits for an offer of kind $1 with two lines of what it saves, and for its card to take input.
offered() {
  expect "^browser: offer $tab $1 2\$" "the form brought no offer to save its $1"
  expect '^browser: offer shown$' "the offer to save the $1 wasn't shown"
  expect "^browser: offer $tab takes input\$" "the offer to save the $1 never took input"
}

# The address, which return saves.
submit "Ada Lovelace" "1 Main St" Springfield IL 62701
offered address
../tools/window-screenshot.sh "$pid" "$out/address-offer.png" || fail "no window to capture"
send key return
expect "^browser: offer $tab answered Accept \$" "return didn't save the address"
expect '^browser: offer hidden$' "saving didn't hide the address's offer"
expect "^browser: offer $tab gone\$" "saving didn't close Chrome's offer to save the address"

# Edit › AutoFill › Contact… opens Chrome's dropdown at the name field, once escape has closed the
# one the click opened, and down previews the saved address.
go_from_bar "${site#http://}address.html"
expect "^browser: title $tab address\$" "the address form didn't load again"
send click 600 30
send key escape
send menu "AutoFill › Contact…"
expect "^browser: autofill $tab 0 opened\$" \
  "Edit › AutoFill › Contact… didn't open the dropdown at the name field"
send key down
expect "^browser: title $tab filled\$" "down in the dropdown didn't preview the saved address"
../tools/window-screenshot.sh "$pid" "$out/autofill-menu-contact.png" || fail "no window to capture"
send key escape

# A card, which escape declines, then another, which return saves.
go_from_bar "${site#http://}card.html"
expect "^browser: title $tab card\$" "the card form didn't load"
send click 600 30
submit "Ada Lovelace" 4111111111111111 12 2030
offered card
../tools/window-screenshot.sh "$pid" "$out/card-offer.png" || fail "no window to capture"
send key escape
expect "^browser: offer $tab answered NotNow \$" "escape didn't decline the card"
expect '^browser: offer hidden$' "declining didn't hide the card's offer"
go_from_bar "${site#http://}card.html"
expect "^browser: title $tab card\$" "the card form didn't load again"
send click 600 30
submit "Ada Lovelace" 5555555555554444 11 2031
offered card
send key return
expect "^browser: offer $tab answered Accept \$" "return didn't save the card"
expect '^browser: offer hidden$' "saving didn't hide the card's offer"

# Edit › AutoFill › Credit Card… opens Chrome's dropdown at the card's number field. On http it
# fills no card in, and says why.
go_from_bar "${site#http://}card.html"
expect "^browser: title $tab card\$" "the card form didn't load a third time"
send click 600 90
send key escape
send menu "AutoFill › Credit Card…"
expect "^browser: autofill $tab 2 opened\$" \
  "Edit › AutoFill › Credit Card… didn't open the dropdown at the number field"
../tools/window-screenshot.sh "$pid" "$out/autofill-menu-card.png" || fail "no window to capture"
send key escape
exec 3>&-
quit
echo "ok: a submitted address offers to save it where the password's offer is, and return saves it; Edit › AutoFill › Contact… opens Chrome's dropdown at the focused field, which previews the saved address; a submitted card offers to save it, escape declines and return saves another; Edit › AutoFill › Credit Card… opens the dropdown at the card's field. Windows: $out/address-offer.png, $out/autofill-menu-contact.png, $out/card-offer.png, $out/autofill-menu-card.png"
