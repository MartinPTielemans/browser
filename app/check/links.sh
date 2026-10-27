# Links from other apps, as `open` sends them, the way a link clicked in Mail does. With the app
# running, one opens as a new tab in the Space the window in front shows, and loads there. With an
# agent's Space shown, it goes in the user's Space the window showed before, which comes back; with
# a private window in front, in the regular window, which comes to the front. Settings › General
# says the browser isn't the default, as a dev build never is: it declares no links (build.sh), and
# making it the default asks in a system dialog the driver can't answer. A link that launches the
# browser opens over the Spaces it brings back, before its window's first frame. Reports the launch
# times from the process's start, without a link and with one.

serve
log="$out/links.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
first=$(space)
window=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
# The line the app logs for a link from another app, $1 being what follows the site's address.
link_line() {
  echo "^browser: link from another app ${site}$1: tab [1-9][0-9]* in space $2 of window $3\$"
}
# Sends https://…/$1 as another app does, and waits for its tab in Space $2 of window $3 to load it.
open_link() {
  open -a "$app" "${site}$1"
  await "$(link_line "$1" "$2" "$3")" ||
    fail "the link ${site}$1 didn't open in Space $2 of window $3: $(grep '^browser: link' "$log" | tail -1)"
  tab=$(grep "$(link_line "$1" "$2" "$3")" "$log" | tail -1 | sed 's/.*: tab \([0-9]*\) .*/\1/')
  await "^browser: loaded $tab ${site}$1\$" || fail "the link ${site}$1 didn't load in tab $tab"
}
open_link "?ext" "$first" "$window"
await "^browser: selected $tab\( in .*\)\{0,1\}\$" || fail "the link's tab isn't shown"
send click $page_point
driven=$active
if [ "$driven" = 1 ]; then
  # Settings says the browser isn't the default and offers to make it so.
  send key , cmd
  expect '^browser: settings shown General$' "⌘, didn't show Settings on its General tab"
  expect '^browser: default browser no$' "Settings didn't read whether the browser is the default"
  proof links-settings.png
  send key w cmd
  expect '^browser: settings hidden$' "⌘W didn't hide Settings"
  # With an agent's Space shown, the link goes in the user's, which the window shows again.
  send agentspace claude-code Claude Code
  expect '^browser: made space [1-9][0-9]* for agent claude-code$' "the driver didn't make the agent's Space"
  agent=$(grep '^browser: made space' "$log" | tail -1 | cut -d' ' -f4)
  send key 2 ctrl
  expect "^browser: space $agent\( in .*\)\{0,1\}\$" "⌃2 didn't show the agent's Space"
  open_link "?agent" "$first" "$window"
  [ "$(space)" = "$first" ] || fail "the window shows Space $(space), not the user's $first"
  # With a private window in front, the link goes to the regular window, which comes to the front.
  send key n cmd shift
  expect '^browser: private window opened [0-9]* profile [0-9]*$' "⇧⌘N didn't open a private window"
  private=$(grep '^browser: private window opened ' "$log" | tail -1 | cut -d' ' -f5)
  open_link "?private" "$first" "$window"
  await "^browser: key window $window\$" || fail "the regular window didn't come to the front"
  grep -q "^browser: link from another app .* of window $private\$" "$log" &&
    fail "a link from another app opened in the private window"
fi
exec 3>&-
quit

# Launched by `open`, as macOS launches the default browser, the app logs to $1 and is $pid once
# it runs. Then $2, if given, is the link that launched it.
launch() {
  log=$1
  rm -f "$log"
  open -a "$app" --env BROWSER_DISPLAY="$BROWSER_DISPLAY" --stdout "$log" --stderr "$log" ${2:+"$2"}
  pid=
  for _ in $(seq 100); do
    pid=$(pgrep -nf "$app/Contents/MacOS/Browser" || true)
    [ -n "$pid" ] && break
    sleep 0.1
  done
  [ -n "$pid" ] || fail "open didn't launch the app"
  await '^browser: first frame at ' || fail "the window didn't draw"
}
# Quits the app `open` launched, which isn't this shell's child to wait for.
quit_launched() {
  osascript -e 'tell application id "dev.browser.app" to quit'
  for _ in $(seq 20); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -0 "$pid" 2>/dev/null && fail "the app didn't quit"
  for _ in $(seq 10); do
    pgrep -f "$app/Contents" > /dev/null || break
    sleep 0.5
  done
  pgrep -f "$app/Contents" > /dev/null && fail "Chromium helper processes were left running"
  return 0
}
# Milliseconds from the process's start to the line "browser: $1 at <ms> ms".
at() {
  grep -m1 "^browser: $1 at " "$log" | sed 's/.* at \([0-9.]*\) ms$/\1/'
}

launch "$out/links-plain.log"
await '^browser: restored in ' || fail "the Spaces didn't come back"
plain="launch window $(at 'launch window') ms, first frame $(at 'first frame') ms"
quit_launched

# The link that launches the browser opens over the Spaces it brings back, before the window has
# drawn, so the restored page never shows first.
launch "$out/links-cold.log" "${site}?cold"
await '^browser: restored in ' || fail "the Spaces didn't come back under the link"
await "$(link_line "?cold" "[0-9]*" "[0-9]*")" || fail "the link that launched the browser didn't open"
tab=$(grep "$(link_line "?cold" "[0-9]*" "[0-9]*")" "$log" | sed 's/.*: tab \([0-9]*\) .*/\1/')
await "^browser: loaded $tab ${site}?cold\$" || fail "the link that launched the browser didn't load"
opened=$(grep -n -m1 '^browser: link from another app ' "$log" | cut -d: -f1)
drawn=$(grep -n -m1 '^browser: first frame at ' "$log" | cut -d: -f1)
[ "$opened" -lt "$drawn" ] || fail "the link opened after the window's first frame"
proof links-cold.png
cold="launch window $(at 'launch window') ms, first frame $(at 'first frame') ms"
quit_launched

if [ "$driven" = 1 ]; then
  echo "ok: links from another app open as new tabs in the front window's Space and load there, in the user's Space when an agent's is shown, and in the regular window, brought to the front, over a private one; Settings says the browser isn't the default. Window: $out/links-settings.png"
else
  echo "ok: a link from another app opens as a new tab in the front window's Space and loads there."
  echo "skip: the agent's Space, the private window and Settings. Browser wasn't the active app, so the page ignored the click."
fi
echo "ok: a link that launches the browser opens over the Spaces it brings back, before the first frame, and loads. Window: $out/links-cold.png"
echo "  Launch without a link: $plain; with one: $cold"
