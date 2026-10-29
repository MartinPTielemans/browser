# Resting on a sidebar tab's row shows its hover card beside the rows (#430): a picture of a tab
# not shown, taken once the switches that hid it are quiet and never on a switch, its title and
# host, and its buttons. The tab shown has no picture, as its page is beside the card. The next row
# swaps its card in at once, and leaving the rows or scrolling them lets it go. The card takes the
# Space's look, dark and light. Reports how long the switches take, beside the tabs run's.

# A page of colour, so its picture shows in the card.
page='data:text/html,<title>loading</title>
<body style="margin:0;height:100vh;background:linear-gradient(135deg,%23e1676f,%237eb6d5)">
<h1 style="font:bold 64px system-ui;color:white;margin:40px">Colours</h1>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "colours"));</script>'
log="$out/cards.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* colours$' || fail "the cards test page didn't draw"
await '^browser: today [0-9][0-9]*$' || fail "the first tab isn't in Today"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: cards. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
open_from_bar example.com
b=$tab
expect_tabs "$b $a" "$b" "after opening example.com"
# The page hidden is pictured once the switch is quiet, drawn unseen for it, never the one shown.
expect "^browser: hover card \(not \)\{0,1\}pictured $a\$" \
  "the core never answered for the tab hidden once the switch was quiet"
none_since "^browser: hover card not pictured $a\$" \
  || fail "the core's picture of the tab hidden came back empty once the switch was quiet"
none_since "^browser: hover card pictured $b\$" || fail "the tab shown was pictured"

# Resting on the hidden tab's row, its card shows with its picture beside the rows, where the UI
# put it, and is drawn before it's on screen.
send move $(row 1)
expect "^browser: hover card shown $a pin favorite\( split\)\{0,1\} picture\$" \
  "resting on the hidden tab's row didn't show its card with its picture"
expect "^browser: hover card drawn $a picture\$" "the hidden tab's card wasn't drawn with its picture"
expect '^browser: hover card window shown$' "the hidden tab's card didn't come on screen"
asked=$(grep '^browser: hover card at ' "$log" | tail -1)
placed=$(grep '^browser: hover card window at ' "$log" | tail -1)
echo "$asked $placed" | awk '
  { across = $5 - $14; down = $6 - $15 }
  END { exit !(NR == 1 && NF == 15 && across > -1 && across < 1 && down > -1 && down < 1 && $5 > 200) }' \
  || fail "the card isn't beside the rows where the UI put it: asked \"$asked\", placed \"$placed\""
../tools/window-screenshot.sh "$pid" "$out/card-dark.png" || fail "no window to capture"
# The next row, the tab shown, swaps its card in at once, with no picture. Its Split pairs it with
# the tab shown before it, as the sidebar's split does.
send move $(row 0)
expect "^browser: hover card shown $b pin favorite\( split\)\{0,1\} none\$" "the tab shown's card has a picture"
none_since '^browser: hover card hidden$' || fail "the card went between rows instead of swapping"
../tools/window-screenshot.sh "$pid" "$out/card-shown-tab.png" || fail "no window to capture"
# Leaving the rows lets it go, and so does a scroll over them.
send move $page_point
expect '^browser: hover card hidden$' "leaving the rows didn't hide the card"
send move $(row 1)
expect "^browser: hover card shown $a " "resting on the hidden tab's row again didn't show its card"
send scroll $(row 1) -5 1
expect '^browser: hover card hidden$' "a scroll over the rows didn't hide the card"
send move $page_point

# A light theme for the Space: the theme picker's first swatch, then the sun. The card takes the
# Space's look.
post rightclick 100 112
expect "^browser: menu shown $(cased "Rename Space, Change Icon…, $colors, ")" "the Space's menu didn't offer its theme"
send choose "$(cased 'Edit Theme Color…')"
expect '^browser: theme picker shown$' "Edit Theme Color… didn't show the theme picker"
expect '^browser: theme picker drawn$' "the theme picker didn't draw itself on screen"
send clickkey "$(swatch_x 0)" $swatch_y
expect '^browser: colored space [0-9]* [0-9a-f]*$' "the first swatch didn't colour the Space"
# The sun, the second appearance button, 13 right of the picker's arrow.
send clickkey 191.3 49
expect '^browser: themed space [0-9]* .*appearance=light' "the sun didn't make the Space's theme light"
send key escape
expect '^browser: theme picker hidden$' "escape didn't close the theme picker"
send move $(row 1)
expect "^browser: hover card shown $a .* picture\$" "the hidden tab's card didn't show in the light Space"
expect '^browser: hover card window shown$' "the hidden tab's card didn't come on screen in the light Space"
../tools/window-screenshot.sh "$pid" "$out/card-light.png" || fail "no window to capture"
send move $page_point
expect '^browser: hover card hidden$' "leaving the rows didn't hide the light card"

# Switches back and forth, each timed, then the one hidden last is pictured and the one shown
# isn't.
for _ in 1 2 3 4 5; do
  switch "2 cmd" "$a"
  switch "1 cmd" "$b"
done
expect "^browser: hover card pictured $a\$" "the tab hidden last wasn't pictured after the switches"
none_since "^browser: hover card pictured $b\$" || fail "the tab shown was pictured after the switches"
exec 3>&-
quit

app_ms=$(sed -n 's/^browser: selected [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: resting on a tab's row shows its card beside the rows, with a picture of its page once it's hidden and the switch quiet, and none for the tab shown; the next row swaps in at once, leaving the rows or a scroll lets it go, and the card takes a light Space's look."
echo "  Switch, key to the core's report, with cards pictured: $app_ms. Windows: $out/card-dark.png, $out/card-shown-tab.png, $out/card-light.png"
