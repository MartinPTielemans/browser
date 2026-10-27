# A page's own dialogs ask with the browser's dialog over the tab's card, in Chrome's words, each
# captured while it's up: alert(), confirm() and prompt() from the served page's keys. ↩ answers
# OK, with what was typed in a prompt, and Esc Cancel, and the page puts the answer in its title.
# An alert goes with its tab and comes back with it. Leaving a page whose beforeunload handler
# asks says "Leave site?": Esc stays and ↩ leaves. A site's HTTP sign-in asks for a username and
# password: Esc shows the page the site sends without them, and ↩ signs in, with the password in
# no log.

serve
log="$out/dialogs.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="${site}dialogs.html" "$app/Contents/MacOS/Browser" \
  < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the dialogs test page didn't draw"
page=$(grep '^browser: title [0-9]* ready$' "$log" | tail -1 | cut -d' ' -f3)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: page dialogs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The page's key $1 asks with its $2 (Alert, Confirm or Prompt), which settles over the card and is
# captured as dialog-$3.png.
ask() {
  send key "$1"
  expect "^browser: page $page dialog $2 .* says\$" "\"$1\" in the page didn't ask with its $2"
  expect '^browser: dialog settled$' "the page's $2 didn't settle"
  ../tools/window-screenshot.sh "$pid" "$out/dialog-$3.png" || fail "no window to capture"
}
# The key $1 in the dialog answers it $2 (accept or cancel), and the page's title becomes $3.
answer() {
  send key "$1"
  expect "^browser: page $page dialog answered $2\$" "$1 didn't answer the page's dialog $2"
  expect '^browser: dialog hidden$' "the page's dialog didn't leave once answered"
  expect "^browser: title $page $3\$" "the page didn't hear \"$3\" from its dialog"
}

ask a Alert alert
answer return accept alerted
ask c Confirm confirm
answer escape cancel "confirm false"
ask c Confirm confirm-again
answer return accept "confirm true"
# The prompt's field holds the page's default, selected, so typing takes its place.
ask p Prompt prompt
send type Grace
answer return accept "prompt Grace"

# Another tab takes the alert down, and its own brings it back.
open_from_bar example.com
other=$tab
send click $(row 1)
expect "^browser: selected $page\( in .*\)\{0,1\}\$" "a click on the dialogs page's row didn't show it"
send click $page_point
ask a Alert alert-back
send click $(row 0)
expect "^browser: selected $other\( in .*\)\{0,1\}\$" "a click on another tab's row didn't show it"
expect '^browser: dialog withdrawn$' "the alert stayed up over another tab"
send click $(row 1)
expect "^browser: selected $page\( in .*\)\{0,1\}\$" "a click on the dialogs page's row didn't show it"
expect '^browser: dialog shown .* says$' "the alert didn't come back with its tab"
expect '^browser: dialog settled$' "the alert didn't settle again"
send key return
expect "^browser: page $page dialog answered accept\$" "↩ didn't answer the alert that came back"

send click $page_point
send key u
expect "^browser: title $page armed\$" "the page didn't ask to be asked before it's left"
# Going from the page asks first, with Chrome's title.
leave() {
  go_from_bar example.org
  expect "^browser: page $page dialog Leave Leave site?\$" "going from the page didn't ask first"
  expect '^browser: dialog settled$' "Leave site? didn't settle"
}
leave
../tools/window-screenshot.sh "$pid" "$out/dialog-leave.png" || fail "no window to capture"
send key escape
expect "^browser: page $page dialog answered cancel\$" "Esc didn't stay on the page"
leave
send key return
expect "^browser: page $page dialog answered accept\$" "↩ didn't leave the page"
expect "^browser: loaded $page https://example.org/\$" "the tab didn't go to example.org once left"

# Each visit asks again, as a sign-in that was cancelled isn't asked for the page it shows.
sign_in() {
  go_from_bar "${site}sign-in?$1"
  expect "^browser: page $page dialog SignIn Sign in\$" "the site's sign-in didn't ask"
  expect '^browser: dialog settled$' "the sign-in didn't settle"
}
sign_in cancel
../tools/window-screenshot.sh "$pid" "$out/dialog-sign-in.png" || fail "no window to capture"
send key escape
expect "^browser: page $page dialog answered cancel\$" "Esc didn't cancel the sign-in"
expect "^browser: title $page signed out\$" "a cancelled sign-in didn't show what the site sent"
sign_in again
send type ada
send key tab
send type lovelace
send key return
expect "^browser: page $page dialog answered accept\$" "↩ didn't sign in"
expect "^browser: title $page signed in ada\$" "the site didn't take the username and password"
# The driver's own lines say what it typed.
grep -v '^browser: drive ' "$log" | grep -q lovelace && fail "the password reached the log"
exec 3>&-
quit
echo "ok: a page's alert, confirm and prompt ask over its card, answer with ↩ and Esc, and an alert"
echo "  comes back with its tab. Leave site? stays with Esc and leaves with ↩. A site's sign-in"
echo "  cancels with Esc and signs in with ↩, and the password reaches no log."
