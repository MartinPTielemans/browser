# The link preview. A link to another site in a pinned tab's page opens in a card over the window,
# its page in a tab of no Space, and the pinned tab stays on its page. Esc closes it and its page.
# ⌘↩ keeps it as the top Today tab, shown, without loading it again. A click on the dim closes it,
# and the page behind doesn't get the click. ⌘W with its page focused closes it, and not the tab
# behind. A favorite's link opens it too. With the Settings switch off, the link goes in the pinned
# tab. Reports how long the first preview took to draw its first frame.

serve
# The test page's top 40% is a link to the same server under another name, localhost, which is
# another site to the browser. Its bottom 60% counts the presses that reach it in its title. Each
# has text, as Chromium ignores a click on a page until it has drawn, and an empty page draws
# nothing. The title says "ready" once a frame is drawn, which the run waits for before a click.
target="http://localhost:$port/preview-target.html"
printf '%s' "<title>loading</title>
<a href=\"$target\" style=\"position:fixed;left:0;top:0;width:100%;height:40%;font:24px system-ui\">
A link to another site</a>
<div id=press style=\"position:fixed;left:0;top:40%;width:100%;height:60%;font:24px system-ui\">
Presses count here</div>
<script>
let presses = 0;
press.onmousedown = () => document.title = \"pressed \" + ++presses;
requestAnimationFrame(() => requestAnimationFrame(() => document.title = \"ready\"));
</script>" > "$out/site/preview.html"
printf '%s' '<title>target</title><p style="font:24px system-ui">Another site</p>' \
  > "$out/site/preview-target.html"
log="$out/preview.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="${site}preview.html" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the preview test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: link preview. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# That click made the window key, which a page ignores, so the next one is the first it counts.
send click $page_point
expect "^browser: title $a pressed [0-9]*\$" "a click on the test page's lower part didn't reach it"
# The link, in the page's top 40%, and a point of the dim over the page's lower part, left of the
# card, which is 5% of the page's height in from its edges.
link_point="600 200"
dim_point="240 600"
# Clicks the link, and waits for its preview to settle, setting $p to the preview's tab.
open_preview() {
  send click $link_point
  expect "^browser: link preview [1-9][0-9]* $target\$" "$1: a click on the link to another site didn't open the preview"
  p=$(grep "^browser: link preview [1-9][0-9]* $target\$" "$log" | tail -1 | cut -d' ' -f4)
  expect '^browser: link preview shown$' "$1: the preview's window didn't show"
  expect '^browser: link preview settled$' "$1: the preview didn't settle"
  expect "^browser: loaded $p $target\$" "$1: the preview's page didn't load"
}
# The lines matching $2, $3… came in that order since the last command was sent, once the last has.
# $1 says when.
in_order() {
  when=$1
  shift
  for line in "$@"; do
    last_line=$line
  done
  expect "$last_line" "$when: \"$last_line\" didn't come"
  at=0
  for line in "$@"; do
    n=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -n -m1 "$line" | cut -d: -f1)
    [ -n "$n" ] && [ "$n" -gt "$at" ] || fail "$when: \"$line\" didn't come after the line before it"
    at=$n
  done
}
# The pinned tab's page is still the test page.
still_home() {
  grep -q "^browser: loaded $a $target\$" "$log" && fail "$1: the link loaded in the pinned tab"
  [ "$(grep "^browser: loaded $a " "$log" | tail -1)" = "browser: loaded $a ${site}preview.html" ] ||
    fail "$1: the pinned tab isn't at its page"
}

# Pinned, the tab's links to other sites open in the preview, which grows from the link.
send key d cmd
expect '^browser: command toggle-pin via page$' "⌘D with the page focused didn't reach the UI"
expect "^browser: pinned $a\$" "⌘D didn't pin the test page's tab: the pinned tabs are \"$(pinned)\""
open_preview "from the pinned tab"
first_frame=$(grep -m1 '^browser: link preview first frame in ' "$log" | sed 's/.* in //')
[ -n "$first_frame" ] || fail "the preview didn't log its first frame"
../tools/window-screenshot.sh "$pid" "$out/preview-open.png" || fail "no window to capture"

# Esc in its page closes it, and its page once it has left.
send key escape
in_order "after Esc" '^browser: link preview closing (escape)$' '^browser: link preview released$' \
  '^browser: link preview hidden$' "^browser: tab $p closed\$"
still_home "after Esc"

# ⌘↩ keeps it as the top Today tab, shown, as it is: its page doesn't load again.
open_preview "for ⌘↩"
kept=$p
send key return cmd
in_order "after ⌘↩" '^browser: link preview keeping (cmd-enter)$' "^browser: link preview kept $p\$"
expect "^browser: today $p\$" "⌘↩ didn't put the preview's tab at the top of Today: Today is \"$(today)\""
expect "^browser: selected $p\( in .*\)\{0,1\}\$" "⌘↩ didn't show the preview's tab"
expect '^browser: link preview hidden$' "⌘↩ didn't hide the preview"
check_layout "the kept tab"
../tools/window-screenshot.sh "$pid" "$out/preview-kept.png" || fail "no window to capture"
[ "$(grep -c "^browser: loaded $p " "$log")" = 1 ] || fail "the kept tab's page loaded again"
still_home "after ⌘↩"
away
send click $(pinned_row 0)
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the pinned row didn't show its tab"

# A click on the dim closes it, and the page behind doesn't get the click.
open_preview "for the dim"
send click $dim_point
in_order "after a click on the dim" '^browser: link preview closing (click outside)$' \
  "^browser: tab $p closed\$"
none_since "^browser: title $a pressed" || fail "the click on the dim reached the page behind"

# ⌘W in its page closes it, not the pinned tab behind.
open_preview "for ⌘W"
send key w cmd
in_order "after ⌘W" '^browser: link preview closing (cmd-w)$' "^browser: tab $p closed\$"
none_since '^browser: command close-tab' || fail "⌘W in the preview closed the tab behind"
none_since "^browser: page $a unloaded 1\$" || fail "⌘W in the preview unloaded the pinned tab"

# A favorite's links open it too: the pinned tab's copy, which Duplicate Tab in its menu puts at the
# top of Today, under the pinned row, the divider and "+ New Tab", dragged onto the Space's name.
away
post rightclick $(pinned_row 0)
expect "^browser: menu shown $(cased '.*Duplicate Tab')" "a right-click on the pinned row didn't show its menu"
send choose "$(cased 'Duplicate Tab')"
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect "^browser: today [1-9][0-9]* $kept\$" "Duplicate Tab didn't put a copy at the top of Today"
b=$(today | cut -d' ' -f1)
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "Duplicate Tab didn't show the copy"
expect "^browser: title $b ready\$" "the copy of the test page didn't draw"
send press 100 $((90 + 44 + 41 + 15 + 41 + 20))
send dragto 100 200
send dragto 114 112
expect '^browser: drag shape tile$' "the copy's row dragged onto the Space's name didn't turn into a tile"
send release 114 112
expect "^browser: favorites $b\$" "the copy's row didn't become a favorite: \"$(grep '^browser: favorites' "$log" | tail -1)\""
open_preview "from the favorite"
send key escape
expect "^browser: tab $p closed\$" "Esc didn't close the favorite's preview"
# The favorite's row of tiles takes the Space's rows 47 down.
away
send click 100 $(($(pinned_row 0 | cut -d' ' -f2) + 47))
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the pinned row didn't show its tab"

# With the switch in Settings › Advanced off, the link goes in the pinned tab.
send click $page_point
expect "^browser: title $a pressed [0-9]*\$" "a click on the test page's lower part didn't reach it"
send key , cmd
expect '^browser: settings shown General$' "⌘, didn't show Settings on its General tab"
send clickkey 491 60
expect '^browser: settings tab Advanced$' "a click on Advanced didn't show its tab"
# Its switch is the row under the haptics', as General's rows are 42 apart.
send clickkey 592 168
expect '^browser: link previews off$' "a click on the link previews switch didn't turn them off"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
send click $link_point
expect "^browser: loaded $a $target\$" "with previews off, the link didn't load in the pinned tab"
none_since '^browser: link preview ' || fail "with previews off, the link opened in a preview"

exec 3>&-
quit
echo "ok: a link to another site in a pinned tab or a favorite opens in a preview over the window, which Esc, a click on the dim and ⌘W close, with its page, leaving the tab and the page behind as they were, and ⌘↩ keeps as the top Today tab, shown, without loading it again. With the Settings switch off, the link goes in the pinned tab. Windows: $out/preview-open.png, $out/preview-kept.png"
echo "  The first preview's first frame came $first_frame after the click reached the UI."
