# Clicks move keyboard focus between the page and the UI, and typing goes where the focus is. With
# the page focused, the browser's reserved shortcuts reach the UI and never the page, and the others
# reach the page first, which can keep them. ⌘N, from the page and the UI, opens a window whose UI
# has the keyboard, and ⇧⌘W, from either, closes it, giving back the page it took, live, as ⌘W does
# in a window with no page. ⌘Q asks
# first: Cancel keeps the browser, and Quit, and don't ask again quits and turns the warning off.
# The command bar opens over the page with the keyboard, never showing the last query, gives the
# keyboard back when it closes, a held escape that closes it doesn't repeat into the page, and it
# loads what's typed in it, over https for a host typed without one, or searches DuckDuckGo for
# words. A page it opens in a new tab has the keyboard: its autofocused field gets what's typed, and
# ⌘T opens the bar again. While ⌘T's is open, "New Tab" has the selected plate. Keys
# typed right after ⌘T, ⌘L or ⌘F, from the page or the UI, wait for the bar it opens and land in its
# field, a composition too, and a page that keeps ⌘F gets them after it. Japanese input composes
# and commits in the page and in the command bar, where return and escape go to the input method
# while it composes. The Edit menu's Undo, Redo and Select All reach the focused page. ⌘Y opens the
# history page from the page and the UI. A page whose renderer crashes leaves the browser running,
# and ⌘R loads it again. Settings › Shortcuts gives New Tab ⌃⌘T, which stays reserved and reaches
# the UI from both sides, while ⌘T no longer runs it, and its ↺ gives ⌘T back. About ilo shows
# Settings' About tab, with Settings key and with About already showing. ⌘T as the window comes on
# screen, before the overlays' windows are made, makes them and opens the bar.

# The test page puts what reached it in its title, which the app logs. Chromium drops input
# to a page that hasn't drawn yet, so the page says when it has.
page='data:text/html,<title>loading</title>
<textarea id=text autofocus style="font:24px system-ui;width:95%25;height:90vh"></textarea>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
text.oninput = () => document.title = "value " + text.value;
// Like Google Docs, the page keeps ⌘F.
addEventListener("keydown", e => {
  if (!e.'"$cmd_key"') return;
  document.title = "page saw cmd-" + e.key;
  if (e.key == "f") e.preventDefault();
}, true);
addEventListener("keydown", e => {
  if (e.key == "Escape") document.title = "page saw escape " + (e.repeat ? "repeat" : "press");
});
</script>'
log="$out/keys.log"
step=0
# The user's keys, which a failed run may have left changed.
rm -f "$saved" "$launch_frame" "$(dirname "$saved")/Shortcuts"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
# At once, as the overlays' windows are made a frame or two after the window comes on screen.
for _ in $(seq 3000); do
  grep -q '^browser: first frame at ' "$log" && break
  kill -0 "$pid" 2>/dev/null || fail "the app exited"
  sleep 0.005
done
send key t cmd
if [ "$active" = 1 ]; then
  expect '^browser: command bar shown$' "⌘T as the window came on screen didn't open the command bar"
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the bar ⌘T opened at launch"
  launch_bar="ok: ⌘T as the window comes on screen opens the bar, its window made then."
  grep -q '^browser: overlays opened for a shortcut$' "$log" ||
    launch_bar="skip: ⌘T at launch came after the overlays' windows were made."
else
  launch_bar="skip: ⌘T at launch. Browser wasn't the active app."
fi
await_load
await '^browser: title [0-9]* ready$' || fail "the test page didn't draw"

send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: keyboard, focus, window states and scrolling. Browser wasn't the active app, so the"
  echo "  page ignored the click. macOS decides which app is active; a background process can't."
  exit 0
fi
[ "$focus" = RenderWidgetHostViewCocoa ] || fail "a click on the page left focus on $focus"
send type abc
expect '^browser: title [0-9]* value abc$' "typing didn't reach the focused page"
# In a text field, ⌘← and ⌘→ move the caret, so they don't come back to go back and forward.
send key left cmd
send key right cmd

for shortcut in "t new-tab" "l edit-url"; do
  page_shortcut $shortcut
  expect '^browser: command bar shown$' "$shortcut didn't open the command bar"
  # ⌘T's bar gives "New Tab" the selected plate, and the tab gets it back when the bar closes.
  if [ "$shortcut" = "t new-tab" ]; then
    expect '^browser: new tab plate 1$' "⌘T didn't give New Tab the selected plate"
    ../tools/window-screenshot.sh "$pid" "$out/new-tab-plate.png" || fail "no window to capture"
  fi
  # A shortcut the page doesn't keep comes back after the page has had it, so after the
  # driver's done.
  send state
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "the command bar opened without the keyboard, which is in $key, on $focus"
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the command bar"
  if [ "$shortcut" = "t new-tab" ]; then
    expect '^browser: new tab plate 0$' "escape didn't give the tab back its plate"
  fi
  in_browser RenderWidgetHostViewCocoa "after the command bar closed"
done
# A held escape that closes the command bar ends there: its repeats don't go on to the page, which
# could take them to leave its full screen, and the next press reaches it.
page_shortcut l edit-url
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send keydown escape
expect '^browser: command bar hidden$' "escape going down didn't close the command bar"
in_browser RenderWidgetHostViewCocoa "after a held escape closed the command bar"
held=$step
send keydown escape repeat
send keydown escape repeat
send keyup escape
send key escape
expect '^browser: title [0-9]* page saw escape press$' "escape pressed again didn't reach the page"
sed -n "/^browser: drive $held: /,\$p" "$log" | grep -q '^browser: title [0-9]* page saw escape repeat$' &&
  fail "a held escape's repeats went on to the page after it closed the command bar"
send key f cmd
expect '^browser: title [0-9]* page saw cmd-f$' "⌘F with the page focused didn't reach the page"
# ⌘N from the page opens a window with no page, so its UI has the keyboard, and ⇧⌘W there closes
# it.
page_shortcut n new-window
expect '^browser: window opened [0-9]' "⌘N with the page focused didn't open a window"
send state
in_browser AccessKitSubclassOfGPUIView "in the window ⌘N opened"
send key w cmd shift
expect '^browser: command close-window via ui$' "⇧⌘W with the UI focused didn't reach the UI"
expect '^browser: window closed [0-9]' "⇧⌘W with the UI focused didn't close the window"
send window 1
# ⌘W in a window with no page closes it too, as ⇧⌘W does.
send click $page_point
in_browser RenderWidgetHostViewCocoa "before ⌘N opened a window for ⌘W"
page_shortcut n new-window
expect '^browser: window opened [0-9]' "⌘N with the page focused didn't open a window"
send key w cmd
expect '^browser: command close-tab via ui$' "⌘W with the UI focused didn't reach the UI"
expect '^browser: window closed [0-9]' "⌘W in a window with no page didn't close it"
send window 1

send click $sidebar_point
# GPUI's view in a window it doesn't own, which AccessKit subclasses.
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
send type x
send key f cmd
expect '^browser: command find via ui$' "⌘F with the UI focused didn't reach the UI"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T with the UI focused didn't reach the UI"
expect '^browser: command bar shown$' "⌘T with the UI focused didn't open the command bar"
send click $page_point
expect '^browser: command bar hidden$' "a click on the page didn't close the command bar"
expect '^browser: new tab plate 0$' "a click on the page didn't give the tab back its plate"
in_browser RenderWidgetHostViewCocoa "after a click on the page closed the command bar"
send type d
# Input reaches the page in order, so by now it has seen the x and the shortcuts, if they got there.
expect '^browser: title [0-9]* value abcd$' "the page got keys meant for the UI: $(grep '^browser: title' "$log" | tail -1)"
# ⌘N from the UI, and ⇧⌘W from the page in the window it opened: the page moves there on a click
# on its row, and back, live, as the window closes.
tab=$(today)
send click $sidebar_point
send key n cmd
expect '^browser: command new-window via ui$' "⌘N with the UI focused didn't reach the UI"
expect '^browser: window opened [0-9]' "⌘N with the UI focused didn't open a window"
first=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
second=$(grep '^browser: window opened ' "$log" | tail -1 | cut -d' ' -f4)
# The driver counts windows in the order they opened, closed ones too.
send window "$(grep -c '^browser: window opened ' "$log")"
send click $(row 0)
expect "^browser: window $second shows $tab\$" \
  "a click on the page's row didn't move it to the new window"
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the page in the new window"
send key w cmd shift
expect '^browser: command close-window via page$' "⇧⌘W with the page focused didn't reach the UI"
expect "^browser: window closed $second\$" \
  "⇧⌘W with the page focused didn't close the window"
expect "^browser: window $first shows $tab\$" "the first window didn't get its page back"
[ "$(grep -c "^browser: loaded $tab " "$log")" = 1 ] ||
  fail "the page loaded again as it moved"
send window 1
send click $page_point
in_browser RenderWidgetHostViewCocoa "after the window ⌘N opened closed"
grep -q '^browser: title [0-9]* page saw cmd-[twnW]$' "$log" && fail "the page saw a reserved shortcut"
grep -q '^browser: title [0-9]* page saw cmd-l$' "$log" || fail "⌘L with the page focused didn't reach the page first"
grep -q '^browser: command find via page$' "$log" && fail "the page kept ⌘F, but the UI ran it too"
grep -q '^browser: command \(back\|forward\) via page$' "$log" && fail "⌘← or ⌘→ in the page's text field went back or forward"

# The driver's scripted input method stands in for Japanese input, which is system-wide.
composed=日本語
if [ "$(uname)" = Linux ]; then
  echo "skip: composing in a page: the Linux driver types plain keys there; check/ime.sh covers it with a real input method."
  composed=
else
  send ime on
  send type nihongo
  expect '^browser: title [0-9]* value abcdnihongo$' "the page didn't show the composition"
  send key space
  expect '^browser: title [0-9]* value abcd日本語$' "the page didn't show the conversion"
  send key return
  send ime off
fi
send type x
expect "^browser: title [0-9]* value abcd${composed}x\$" "return in the page did more than commit: $(grep '^browser: title' "$log" | tail -1)"
# The Edit menu's Undo, Redo and Select All reach the focused page, which does them itself.
send key z cmd
expect "^browser: title [0-9]* value abcd$composed\$" "⌘Z in the page's text field didn't undo"
send key z cmd shift
expect "^browser: title [0-9]* value abcd${composed}x\$" "⇧⌘Z in the page's text field didn't redo"
send key a cmd
send type y
expect '^browser: title [0-9]* value y$' "⌘A in the page didn't select all"

# Keys typed right after ⌘T, ⌘L or ⌘F land in the field it opens, in order and a composition
# whole, never in the page (Ardia P§7). They go with no wait after the shortcut, so they're queued
# before the bar has the keyboard, and the app says it held them for it.
typed_after() {
  post key "$1" cmd
  send type "$2"
  # Linux holds them only until the bar takes the keyboard, and X11 sends on those that come before
  # it has the input focus, so there where they land, which every caller checks, is the proof.
  [ "$(uname)" = Linux ] && return
  expect '^browser: held [0-9]* key events for GPUIPanel$' \
    "keys typed right after ⌘$(echo "$1" | tr a-z A-Z) $3 didn't wait for its bar"
}
drew() {
  grep "^browser: $1 drew" "$log" | tail -1
}
for shortcut in t l; do
  typed_after $shortcut abc "with the page focused"
  expect '^browser: command bar drew "abc"$' \
    "keys typed right after ⌘$shortcut with the page focused went elsewhere: $(drew 'command bar')"
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the command bar"
  in_browser RenderWidgetHostViewCocoa "after the command bar closed"
done
if [ "$(uname)" = Linux ]; then
  echo "skip: composing right after ⌘L: the Linux driver types plain keys in a page."
else
  send ime on
  typed_after l nihongo "with the page focused, composing,"
  expect '^browser: command bar drew "nihongo"$' \
    "a composition typed right after ⌘L didn't go to the command bar: $(drew 'command bar')"
  send state
  [ "$marked" = 0,7 ] || fail "the command bar didn't take all of the composition: marked $marked"
  send key escape
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the command bar"
  send ime off
  in_browser RenderWidgetHostViewCocoa "after the command bar closed"
fi
send click $sidebar_point
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
for shortcut in t l; do
  typed_after $shortcut abc "with the UI focused"
  expect '^browser: command bar drew "abc"$' \
    "keys typed right after ⌘$shortcut with the UI focused went elsewhere: $(drew 'command bar')"
  send key escape
  expect '^browser: command bar hidden$' "escape didn't close the command bar"
done
# Closed first: the find bar opened from the UI before stays open, and ⌘F would make it key at once.
send key f cmd
send key escape
expect '^browser: find bar hidden$' "escape didn't close the find bar"
send click $sidebar_point
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
typed_after f abc "with the UI focused"
expect '^browser: find bar drew "abc" ' \
  "keys typed right after ⌘F with the UI focused went elsewhere: $(drew 'find bar')"
send key escape
expect '^browser: find bar hidden$' "escape didn't close the find bar"
# A page that keeps the shortcut, as this one keeps ⌘F, gets the keys typed after it once it has,
# in order. Input reaches the page in order, so it has seen none of the keys the bars got.
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the page"
post key f cmd
send type abc
title=$(grep '^browser: title' "$log" | tail -1)
expect '^browser: title [0-9]* value yabc$' \
  "the page that keeps ⌘F didn't get the keys after it, or got a bar's: $title"

# View History (⌘Y) opens Chrome's history page in a tab, from the page, which doesn't keep the
# key, and from the UI. ⌘W closes the tab, which shows the test page again.
before=$(today)
page_shortcut y view-history
expect "^browser: today [0-9][0-9]*${before:+ $before}\$" \
  "⌘Y with the page focused didn't open a tab"
tab=$(today | cut -d' ' -f1)
expect "^browser: loaded $tab chrome://history/\$" \
  "⌘Y with the page focused didn't open the history page"
send key w cmd
expect "^browser: today $before\$" "⌘W didn't close the history tab"
send click $sidebar_point
send key y cmd
expect '^browser: command view-history via ui$' "⌘Y with the UI focused didn't reach the UI"
expect "^browser: today [0-9][0-9]*${before:+ $before}\$" \
  "⌘Y with the UI focused didn't open a tab"
tab=$(today | cut -d' ' -f1)
expect "^browser: loaded $tab chrome://history/\$" \
  "⌘Y with the UI focused didn't open the history page"
send key w cmd
expect "^browser: today $before\$" "⌘W didn't close the history tab"
send click $page_point
in_browser RenderWidgetHostViewCocoa "after the history tab closed"

# Settings › Shortcuts' search, over its boxes, finds a command by the words of its name or by
# its keys (#682). Under it, New Tab comes first, its keys in a chip at the row's right, 56 wide
# and 10 in from the box, and ↺, once it has other keys, 6 before the chip. A click on the chip
# records the next keys pressed.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey 373 60
expect '^browser: settings tab Shortcuts$' "a click on Shortcuts didn't show its tab"
send clickkey 300 120
send type new tab
expect '^browser: shortcuts search "new tab" finds New Tab$' "the search didn't find New Tab by name"
send key a cmd
send type cmd t
expect '^browser: shortcuts search "cmd t" finds New Tab, Restore Last Closed Tab$' \
  "the search didn't find ⌘T and ⇧⌘T by their keys"
../tools/window-screenshot.sh "$pid" "$out/shortcuts-search.png" || fail "no window to capture"
send key a cmd
send key backspace
expect '^browser: shortcuts search cleared$' "⌫ didn't clear the search"
send clickkey 581 203
expect '^browser: shortcut recording new-tab$' "a click on New Tab's keys didn't record them"
send key t cmd ctrl
# Off the Mac ⌃ presses Alt, as ⌥ does (docs/ux-spec.md › Keyboard).
new_tab_key=ctrl-cmd-t
if [ "$(uname)" = Linux ]; then
  new_tab_key=alt-cmd-t
fi
expect "^browser: shortcut new-tab $new_tab_key\$" "⌃⌘T didn't become New Tab's key"
../tools/window-screenshot.sh "$pid" "$out/shortcuts.png" || fail "no window to capture"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
# New Tab's new key is reserved, as ⌘T was: the page never sees it.
send click $page_point
send key t cmd ctrl
expect '^browser: command new-tab via page$' "⌃⌘T with the page focused didn't reach the UI"
expect '^browser: command bar shown$' "⌃⌘T with the page focused didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
in_browser RenderWidgetHostViewCocoa "after the command bar closed"
send type z
expect '^browser: title [0-9]* value yabcz$' "typing didn't reach the page after ⌃⌘T"
grep -q '^browser: title [0-9]* page saw cmd-t$' "$log" &&
  fail "the page saw ⌃⌘T, New Tab's reserved key"
# ⌘T is the page's now, and runs nothing from either side.
send key t cmd
expect '^browser: title [0-9]* page saw cmd-t$' "⌘T with the page focused didn't reach the page"
none_since '^browser: command new-tab' || fail "⌘T with the page focused still ran New Tab"
send click $sidebar_point
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
send key t cmd
none_since '^browser: command new-tab' || fail "⌘T with the UI focused still ran New Tab"
send key t cmd ctrl
expect '^browser: command new-tab via ui$' "⌃⌘T with the UI focused didn't reach the UI"
expect '^browser: command bar shown$' "⌃⌘T with the UI focused didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
# ↺ gives New Tab ⌘T back, so later runs have the defaults.
send menu Settings…
expect '^browser: settings shown Shortcuts$' \
  "Settings… didn't show the Shortcuts tab it showed last"
send clickkey 537 203
expect '^browser: shortcut new-tab cmd-t$' "↺ didn't give New Tab ⌘T back"
# The app menu's About opens Settings on its About tab: the version, the engine's and the credits.
# Settings has the keyboard from the click on ↺, so the menu's command runs inside its window's
# own update, which crashed the app when About read that window there.
[ "$key" = GPUIWindow ] || fail "Settings didn't have the keyboard after a click on ↺: $key does"
send menu About $app_name
expect '^browser: settings tab About$' "About $app_name didn't show Settings' About tab"
expect '^browser: settings drew About$' \
  "Settings didn't draw About after About $app_name with Settings key"
../tools/window-screenshot.sh "$pid" "$out/about.png" || fail "no window to capture"
# Again with About showing: Settings comes forward and draws it again, with no tab to change.
send menu About $app_name
expect '^browser: settings drew About$' \
  "Settings didn't draw About after About $app_name with About already showing"
none_since '^browser: settings tab ' || fail "About $app_name with About showing changed the tab"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
send click $page_point
page_shortcut t new-tab
expect '^browser: command bar shown$' "⌘T didn't open the command bar once it was New Tab's again"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
in_browser RenderWidgetHostViewCocoa "after the command bar closed"

send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the command bar didn't take the composition: marked $marked"
send key escape
none_since '^browser: command bar hidden$' || fail "escape while composing closed the command bar"
[ "$marked" = none ] || fail "escape didn't cancel the composition: marked $marked"
send type nihongo
send key space
[ "$marked" = 0,3 ] || fail "the command bar didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/ime.png" || fail "no window to capture"
send key return
none_since '^browser: command bar typed' || fail "return while composing went somewhere"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send type .invalid
send key return
expect '^browser: command bar typed 日本語.invalid$' "the command bar didn't keep the committed text: $(grep '^browser: command bar typed' "$log" | tail -1)"
# ⌘L goes to the page first, and a key the page being replaced hasn't answered leaves with it.
expect '^browser: loaded [0-9]* https://duckduckgo.com/' "the search for the committed text didn't load"

send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.com
../tools/window-screenshot.sh "$pid" "$out/command-bar.png" || fail "no window to capture"
send key return
expect '^browser: command bar go https*://example.com/$' "return in the command bar didn't go to what was typed: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "going somewhere didn't close the command bar"
expect '^browser: loaded [0-9]* https://example.com/$' "the page didn't load what was typed in the command bar"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar loaded a page"
# Words that name no page are a search, with DuckDuckGo until the user picks an engine.
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type arc browser
send key return
expect '^browser: command bar go https://duckduckgo.com/?q=arc+browser$' "words in the command bar didn't search: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: loaded [0-9]* https://duckduckgo.com/?q=arc+browser' "the search didn't load"
../tools/window-screenshot.sh "$pid" "$out/search.png" || fail "no window to capture"
# A page the bar opens in a new tab has the keyboard, so its field autofocused gets what's typed
# and the browser's shortcuts reach the UI, with no click in it first (#428, where X11 lost them).
serve
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type "${site}autofocus.html"
send key return
expect "^browser: command bar go ${site}autofocus.html\$" \
  "the command bar didn't go to the page typed: $(grep '^browser: command bar go' "$log" | tail -1)"
expect '^browser: command bar hidden$' "going somewhere didn't close the command bar"
expect "^browser: loaded [0-9]* ${site}autofocus.html\$" "the page typed in the bar didn't load"
tab=$(grep "^browser: loaded [0-9]* ${site}autofocus.html\$" "$log" | tail -1 | cut -d' ' -f3)
expect "^browser: title $tab ready\$" "the page the bar opened didn't draw"
send type new
expect "^browser: title $tab value new\$" \
  "typing didn't reach the field of the page the bar opened in a new tab"
page_shortcut t new-tab
expect '^browser: command bar shown$' "⌘T in the page the bar opened didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
# The bar draws itself before it goes on screen, so it never shows the last query: empty for a
# new tab, and on ⌘L with the page's URL.
awk '/^browser: command (new-tab|edit-url) via/ { command = $3 }
  /^browser: command bar drew/ { drew = $0 }
  /^browser: command bar shown$/ && (command == "new-tab") != (drew == "browser: command bar drew \"\"") { exit 1 }' "$log" ||
  fail "the command bar went on screen still showing what it drew last"
# A page whose renderer goes away leaves the browser running, and ⌘R loads it again. Killed, as
# macOS kills one under memory pressure, it takes Chrome's sad tab path a crash takes; a SIGSEGV
# sent from here only prints a stack, and the command bar completes chrome://crash to another page.
# The core's line comes through Chromium's log, after its prefix.
pkill -KILL -f "$procs.*--type=renderer" || fail "no renderer to kill"
expect 'browser: tab [0-9]* page gone' "a killed renderer didn't report its page gone"
send click $sidebar_point
send key r cmd
expect '^browser: loaded [0-9]* ' "⌘R didn't load the crashed page again"
send click $page_point

# ⌘Q asks first, in the dialog over the window. Escape keeps the browser running, and Quit, and
# Don't Ask Again quits. That button is at the card's bottom left, 29 in from its left and bottom
# edges; the card is centred, 450 wide and, with no detail, 172 tall.
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
width=${content%x*}
height=${content#*x}
post key q cmd
expect '^browser: command quit via page$' "⌘Q with the page focused didn't reach the UI"
expect "^browser: dialog shown Quit $app_name?\$" "⌘Q didn't ask before quitting"
expect '^browser: dialog settled$' "the dialog asking to quit didn't settle"
../tools/window-screenshot.sh "$pid" "$out/quit.png" || fail "no window to capture"
send key escape
expect '^browser: dialog cancelled$' "escape didn't cancel quitting"
expect '^browser: dialog hidden$' "the dialog asking to quit didn't go"
post key q cmd
expect "^browser: dialog shown Quit $app_name?\$" "⌘Q didn't ask again after Cancel"
expect '^browser: dialog settled$' "the dialog asking to quit didn't settle again"
# It's the driver's last command: the app doesn't report it done.
post click $((width / 2 - 225 + 29 + 40)) $((height / 2 - 86 + 1 + 38 + 22 + 46 + 18))
exec 3>&-
await '^browser: warn before quitting off$' ||
  fail "Quit, and don't ask again didn't turn the warning off"
exited
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q asks first in the dialog, and escape keeps the browser and Quit, and don't ask again quits, the command bar takes the keyboard and gives it back, a held escape that closes it doesn't repeat into the page, keys typed right after ⌘T, ⌘L and ⌘F land in the bar they open from both sides, a composition too, and reach a page that keeps ⌘F after it, with New Tab's plate while ⌘T's is open, goes to a host over https and searches for words, and a page it opens in a new tab has the keyboard, its autofocused field and the browser's shortcuts too, Japanese input composes in both, and the Edit menu undoes, redoes and selects in the page, ⌘Y opens the history page from both sides, Settings › Shortcuts' search finds New Tab by name and ⌘T's rows by their keys, and New Tab rebound to ⌃⌘T there stays reserved from both sides while ⌘T runs nothing, until ↺ gives it back, About ilo shows Settings' About tab with Settings key and again with About showing, and a killed page's renderer leaves the browser running and ⌘R loads the page again. Windows: $out/quit.png, $out/command-bar.png, $out/search.png, $out/ime.png, $out/shortcuts-search.png, $out/shortcuts.png, $out/about.png"
echo "$launch_bar"
