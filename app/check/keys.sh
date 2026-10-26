# Clicks move keyboard focus between the page and the UI, and typing goes where the focus is. With
# the page focused, the browser's reserved shortcuts reach the UI and never the page, and the others
# reach the page first, which can keep them. ⌘Q asks first: Cancel keeps the browser, and Quit, and
# don't ask again quits and turns the warning off. The command bar opens over the page with the
# keyboard, never showing the last query, gives the keyboard back when it closes, and loads what's
# typed in it, over https for a host typed without one, or searches DuckDuckGo for words. While ⌘T's
# is open, "New Tab" has the selected plate. Japanese input composes and commits in the page and in
# the command bar, where return and escape go to the input method while it composes. The Edit menu's
# Undo, Redo and Select All reach the focused page. ⌘Y opens the history page from the page and the
# UI. Settings › Shortcuts gives New Tab ⌃⌘T, which stays reserved and reaches the UI from both
# sides, while ⌘T no longer runs it, and its ↺ gives ⌘T back.

# The test page puts what reached it in its title, which the app logs. Chromium drops input
# to a page that hasn't drawn yet, so the page says when it has.
page='data:text/html,<title>loading</title>
<textarea id=text autofocus style="font:24px system-ui;width:95%25;height:90vh"></textarea>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
text.oninput = () => document.title = "value " + text.value;
// Like Google Docs, the page keeps ⌘F.
addEventListener("keydown", e => {
  if (!e.metaKey) return;
  document.title = "page saw cmd-" + e.key;
  if (e.key == "f") e.preventDefault();
}, true);
</script>'
log="$out/keys.log"
step=0
# The user's keys, which a failed run may have left changed.
rm -f "$saved" "$(dirname "$saved")/Shortcuts"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
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
send key f cmd
expect '^browser: title [0-9]* page saw cmd-f$' "⌘F with the page focused didn't reach the page"

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
grep -q '^browser: title [0-9]* page saw cmd-[tw]$' "$log" && fail "the page saw a reserved shortcut"
grep -q '^browser: title [0-9]* page saw cmd-l$' "$log" || fail "⌘L with the page focused didn't reach the page first"
grep -q '^browser: command find via page$' "$log" && fail "the page kept ⌘F, but the UI ran it too"
grep -q '^browser: command \(back\|forward\) via page$' "$log" && fail "⌘← or ⌘→ in the page's text field went back or forward"

# The driver's scripted input method stands in for Japanese input, which is system-wide.
send ime on
send type nihongo
expect '^browser: title [0-9]* value abcdnihongo$' "the page didn't show the composition"
send key space
expect '^browser: title [0-9]* value abcd日本語$' "the page didn't show the conversion"
send key return
send ime off
send type x
expect '^browser: title [0-9]* value abcd日本語x$' "return in the page did more than commit: $(grep '^browser: title' "$log" | tail -1)"
# The Edit menu's Undo, Redo and Select All reach the focused page, which does them itself.
send key z cmd
expect '^browser: title [0-9]* value abcd日本語$' "⌘Z in the page's text field didn't undo"
send key z cmd shift
expect '^browser: title [0-9]* value abcd日本語x$' "⇧⌘Z in the page's text field didn't redo"
send key a cmd
send type y
expect '^browser: title [0-9]* value y$' "⌘A in the page didn't select all"

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

# Settings › Shortcuts lists New Tab first, its keys in a chip at the row's right, 56 wide and 10
# in from the box, and ↺, once it has other keys, 6 before the chip. A click on the chip records
# the next keys pressed.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey 365 60
expect '^browser: settings tab Shortcuts$' "a click on Shortcuts didn't show its tab"
send clickkey 581 159
expect '^browser: shortcut recording new-tab$' "a click on New Tab's keys didn't record them"
send key t cmd ctrl
expect '^browser: shortcut new-tab ctrl-cmd-t$' "⌃⌘T didn't become New Tab's key"
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
expect '^browser: title [0-9]* value yz$' "typing didn't reach the page after ⌃⌘T"
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
send clickkey 537 159
expect '^browser: shortcut new-tab cmd-t$' "↺ didn't give New Tab ⌘T back"
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
# The bar draws itself before it goes on screen, so it never shows the last query: empty for a
# new tab, and on ⌘L with the page's URL.
awk '/^browser: command (new-tab|edit-url) via/ { command = $3 }
  /^browser: command bar drew/ { drew = $0 }
  /^browser: command bar shown$/ && (command == "new-tab") != (drew == "browser: command bar drew \"\"") { exit 1 }' "$log" ||
  fail "the command bar went on screen still showing what it drew last"
# ⌘Q asks first. Cancel keeps the browser running, and Quit, and Don't Ask Again quits.
post key q cmd
expect '^browser: command quit via page$' "⌘Q with the page focused didn't reach the UI"
expect '^browser: alert shown Quit Browser?$' "⌘Q didn't ask before quitting"
send choose Cancel
post key q cmd
expect '^browser: alert shown Quit Browser?$' "⌘Q didn't ask again after Cancel"
# It's the driver's last command: the app doesn't report it done.
post choose Quit, and don’t ask again
exec 3>&-
await '^browser: warn before quitting off$' ||
  fail "Quit, and don't ask again didn't turn the warning off"
exited
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q asks first, and Cancel keeps the browser and Quit, and don't ask again quits, the command bar takes the keyboard and gives it back, with New Tab's plate while ⌘T's is open, goes to a host over https and searches for words, Japanese input composes in both, and the Edit menu undoes, redoes and selects in the page, ⌘Y opens the history page from both sides, and New Tab rebound to ⌃⌘T in Settings › Shortcuts stays reserved from both sides while ⌘T runs nothing, until ↺ gives it back. Windows: $out/command-bar.png, $out/search.png, $out/ime.png, $out/shortcuts.png"
