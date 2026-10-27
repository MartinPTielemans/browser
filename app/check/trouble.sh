# Pages that break show ilo's own state. A page that stops answering for Chrome's hang timeout gets
# the toast's Not responding with Wait and Reload, and a mark on its row: the note goes by itself
# once the page answers, Wait gives it longer, and Reload ends its renderer and loads it again, with
# no card. A page whose renderer dies, killed as macOS kills one under memory pressure, leaves its
# pane to the card's This page stopped, whose Reload loads it again, while a tab behind whose
# renderer dies is unloaded instead and loads once it's shown, with no card. A load that fails
# shows the core's error page, with the error's code, and one refused for want of a server loads
# again by itself, through Chromium's auto-reload, once the server is there.

serve
# Typing w in it keeps its renderer busy for 20 s, past Chrome's 15 s hang timeout, then the title
# says so, counted, since a title that doesn't change logs nothing.
printf '%s' '<title>loading</title><body style="background:#333">
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
let wakes = 0;
onkeydown = e => {
  if (e.key != "w") {
    return;
  }
  const end = Date.now() + 20000;
  while (Date.now() < end) {}
  document.title = "woke " + ++wakes;
};
</script>' > "$out/site/busy.html"
log="$out/trouble.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="${site}busy.html" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the busy page didn't draw"
tab=$(today | cut -d' ' -f1)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: trouble. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi

# expect, for up to 30 s: Chrome calls a page hung only 15 s after the input it hasn't answered.
expect_late() {
  for _ in $(seq 300); do
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    kill -0 "$pid" 2>/dev/null || fail "the app exited"
    sleep 0.1
  done
  fail "$2"
}
# The toast's buttons, from the right of the window's content, $width wide: it ends 18 in from the
# window's edge, its last button 13.5 in from its own, each button 9 + its label + 9 wide, 8 apart,
# all 41 down. Reload's middle is about 45 in from the toast's edge and Wait's about 106.
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
width=${content%x*}
toast_reload="$((width - 18 - 45)) 41"
toast_wait="$((width - 18 - 106)) 41"
hang() {
  send click $page_point
  send key w
  expect_late "browser: tab $tab page hung" "$1: the busy page wasn't called hung"
  expect "^browser: page $tab trouble hung\$" "$1: the app didn't hear the page hung"
  expect '^browser: toast shown Not responding \[wait, reload\]$' "$1: the toast didn't say Not responding"
  expect '^browser: toast settled$' "$1: the toast didn't settle"
}

# Hung, the note shows with the row's mark, and goes by itself once the page answers.
hang "hung"
../tools/window-screenshot.sh "$pid" "$out/hung.png" || fail "no window to capture"
expect_late "^browser: title $tab woke 1\$" "the busy page didn't wake"
expect "^browser: page $tab trouble fine\$" "the page answering didn't make it fine"
expect '^browser: toast hidden$' "the note stayed once the page answered"

# Wait gives it Chrome's timeout again, which the page answers within.
hang "wait"
send click $toast_wait
expect '^browser: toast button wait$' "Wait wasn't pressed at $toast_wait"
expect "^browser: page $tab trouble fine\$" "Wait didn't make the page fine"
expect_late "^browser: title $tab woke 2\$" "the busy page didn't wake after Wait"
none_since "page $tab trouble hung" || fail "the page was called hung again within Wait's time"

# Reload ends the renderer and loads the page again, without the crash card.
hang "reload"
send click $toast_reload
expect '^browser: toast button reload$' "Reload wasn't pressed at $toast_reload"
expect "browser: tab $tab page gone" "Reload didn't end the hung renderer"
expect "^browser: loaded $tab ${site}busy.html\$" "Reload didn't load the hung page again"
none_since "page $tab trouble crashed" || fail "a page Reload ended showed the crash card"

# A second tab, from the command bar. The first is behind it.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send insert "${site}index.html"
send key return
expect "^browser: today [0-9]* $tab\$" "the second tab didn't open over the first"
second=$(today | cut -d' ' -f1)
expect "^browser: loaded $second " "the second tab didn't load"

# Every renderer killed: the tab shown shows the card, and the one behind is unloaded, with none.
pkill -KILL -f "$app/Contents/.*--type=renderer" || fail "no renderer to kill"
expect "browser: tab $second page gone" "a killed renderer didn't report its page gone"
expect "^browser: page $second trouble crashed\$" "the tab shown wasn't called crashed"
expect "^browser: crash card shown $second\$" "the crash card didn't show"
expect "^browser: page $tab unloaded 1\$" "the tab behind wasn't unloaded"
none_since "page $tab trouble crashed" || fail "the tab behind was called crashed"
../tools/window-screenshot.sh "$pid" "$out/crashed.png" || fail "no window to capture"
# Its Reload, where the card says it is.
at=$(sed -n "s/^browser: crash reload $second at \([0-9]*\) \([0-9]*\) \([0-9]*\) \([0-9]*\)$/\1 \2 \3 \4/p" \
  "$log" | tail -1)
[ -n "$at" ] || fail "the crash card's Reload wasn't drawn"
set -- $at
send click $(($1 + $3 / 2)) $(($2 + $4 / 2))
expect "^browser: crash card reload $second\$" "the crash card's Reload wasn't pressed"
expect "^browser: loaded $second ${site}index.html\$" "Reload didn't load the dead page again"
expect "^browser: page $second trouble fine\$" "the page loaded again wasn't fine"

# The tab behind loads once it's shown, with no card.
send click $(row 1)
expect "^browser: selected $tab in " "the first tab wasn't shown"
expect "^browser: loaded $tab ${site}busy.html\$" "the tab behind didn't load once shown"
none_since "trouble crashed" || fail "the tab behind showed the crash card"

# A host that doesn't resolve shows the core's error page, with no card and no mark. The host has
# no address under a real domain, since the bar's first suggestion for a .invalid one is a search.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send insert "http://nothing.example.com/"
send key return
expect "^browser: page $tab trouble network -105\$" "a host that doesn't resolve wasn't a network error"
await '^browser: title [0-9]* nothing.example.com$' || fail "the error page didn't take the host for its title"
../tools/window-screenshot.sh "$pid" "$out/network-error.png" || fail "no window to capture"

# A port with no server is refused, and loads by itself once a server takes it.
free=$(python3 -I -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send insert "http://127.0.0.1:$free/"
send key return
expect "^browser: page $tab trouble network -102\$" "a port with no server wasn't refused"
python3 -I -m http.server "$free" --bind 127.0.0.1 --directory "$out/site" > "$out/late-server.log" 2>&1 &
stuck=$!
expect_late "^browser: loaded $tab http://127.0.0.1:$free/\$" "the refused page didn't load once the server was there"
expect "^browser: page $tab trouble fine\$" "the page that came wasn't fine"
kill "$stuck"
stuck=

quit_by_key
exited
echo "ok: a page that stops answering gets Not responding with Wait and Reload and goes by itself once it answers, Wait gives it longer and Reload ends its renderer and loads it again without the crash card, a dead page's pane shows This page stopped, whose Reload loads it again, a tab behind whose renderer dies is unloaded and loads once shown, a host that doesn't resolve shows the core's error page and a refused page loads by itself once its server comes. Windows: $out/hung.png, $out/crashed.png, $out/network-error.png"
