# A new profile's first launch plays the onboarding over the window, captured at each step in a
# light and a dark appearance: the intro draws ilo's wordmark and its line and goes on by itself,
# or at any key; Esc or Skip goes on from a step, and the last step leaves the flow for the first
# Space's welcome. Help › Welcome to ilo plays it again, in the window while it shows no page and
# in a new window while it shows one. A launch that brings Spaces back never starts it. Every
# other run sets BROWSER_ONBOARDING=skip (check.sh); this one sets the appearance instead.

serve
# Captures the window as onboarding-$1.png.
capture_step() {
  ../tools/window-screenshot.sh "$pid" "$out/onboarding-$1.png" || fail "no window to capture"
}
# Launches a new profile, whose saved Spaces are gone, in appearance $1.
launch_new() {
  log="$out/onboarding-$1.log"
  step=0
  rm -f "$saved"
  BROWSER_DRIVE=1 BROWSER_ONBOARDING=$1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await '^browser: onboarding step intro settled$' || fail "$1: the intro didn't draw"
}

# Light: the intro holds once drawn and goes on by itself, and Esc leaves the last step.
launch_new light
grep -q '^browser: onboarding started$' "$log" || fail "a new profile didn't start the onboarding"
capture_step light-intro
await '^browser: onboarding left intro via time$' || fail "light: the intro didn't go on by itself"
await '^browser: onboarding step import settled$' || fail "light: the import step didn't come"
capture_step light-import
send key escape
expect '^browser: onboarding left import via escape$' "light: Esc didn't leave the import step"
expect '^browser: onboarding done$' "light: the flow didn't end after its last step"
capture_step light-welcome
exec 3>&-
quit

# A launch that brings the Spaces back, as the light one left them, never starts it, even with
# the run not skipping it.
log="$out/onboarding-relaunch.log"
step=0
BROWSER_DRIVE=1 BROWSER_ONBOARDING=light "$app/Contents/MacOS/Browser" < "$out/drive" \
  > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't bring the Spaces back"
await '^browser: UI shown at ' || fail "the relaunch didn't show its UI"
grep -q '^browser: onboarding' "$log" &&
  fail "a launch that brought Spaces back started the onboarding"
exec 3>&-
quit

# Dark: a key skips the intro, and Skip, at the window's bottom right, leaves the last step.
launch_new dark
capture_step dark-intro
send key a
expect '^browser: onboarding left intro via key$' "dark: a key didn't skip the intro"
expect '^browser: onboarding step import settled$' "dark: the import step didn't come"
capture_step dark-import
# Its words and chip end 24 in from the window's right and bottom, 20 tall.
frame=$(grep -m1 '^browser: window [0-9]* frame ' "$log" | cut -d' ' -f7-8)
width=${frame% *}
height=${frame#* }
[ -n "$height" ] || fail "the window didn't log its frame"
send click $((width - 24 - 15)) $((height - 24 - 10))
expect '^browser: onboarding left import via skip$' \
  "dark: a click on Skip didn't leave the import step"
expect '^browser: onboarding done$' "dark: Skip on the last step didn't end the flow"
capture_step dark-welcome

# Help › Welcome to ilo plays it again here, as the window shows no page.
send menu "Welcome to $app_name"
expect '^browser: command welcome via ui$' "Welcome to $app_name didn't run"
expect '^browser: onboarding step intro settled$' "Welcome to $app_name didn't play the intro again"
send key escape
expect '^browser: onboarding left intro via key$' "Esc didn't skip the replayed intro"
send key escape
expect '^browser: onboarding done$' "the replay didn't end after its last step"

# With a page shown it plays in a new window, which shows none, and the page stays where it was.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type "${site#http://}index.html"
send key return
expect "^browser: loaded [0-9]* ${site}index.html\$" "the page from the command bar didn't load"
windows=$(grep -c '^browser: window [0-9]* frame ' "$log")
send menu "Welcome to $app_name"
expect '^browser: onboarding step intro settled$' "Welcome to $app_name over a page didn't play it"
[ "$(grep -c '^browser: window [0-9]* frame ' "$log")" -gt "$windows" ] ||
  fail "Welcome to $app_name over a page didn't open a window for it"
capture_step replay
send key a
send key escape
expect '^browser: onboarding done$' "the replay in its own window didn't end"
exec 3>&-
quit

echo "ok: a new profile's first launch plays the intro, which goes on by itself or at a key, then a step that Esc and Skip leave for the welcome, in light and dark; Help › Welcome to $app_name plays it again, in a new window over a page; a launch that brings Spaces back doesn't. Captures: $out/onboarding-*.png"
