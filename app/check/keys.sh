# Clicks move keyboard focus between the page and the UI, and typing goes where the focus is. With
# the page focused, the browser's reserved shortcuts reach the UI and never the page, and the others
# reach the page first, which can keep them. ⌘Q asks first: Cancel keeps the browser, and Quit, and
# don't ask again quits and turns the warning off. The command bar opens over the page with the
# keyboard, never showing the last query, gives the keyboard back when it closes, and loads what's
# typed in it, over https for a host typed without one, or searches DuckDuckGo for words. While ⌘T's
# is open, "New Tab" has the selected plate and the card shows the New Tab page in place of the
# page, with the bar as its box where Dia's is, and a click there leaves it for the page, which
# comes back without the keyboard. Japanese input composes and commits in the page and in
# the command bar, where return and escape go to the input method while it composes. The Edit menu's
# Undo, Redo and Select All reach the focused page.

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
rm -f "$saved"
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
    send layout
    layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -A9 -m1 '^browser: layout')
    echo "$layout" | grep -q WebContentsViewCocoa && fail "the page stayed over the New Tab page"
    # In the 1280 × 800 window, docs/ux-spec.md › New tab's sums put the box 424, 281 in.
    echo "$layout" | grep -q '^browser: layout child 424,281 652x112$' ||
      fail "the New Tab box isn't where Dia's is: $(echo "$layout" | grep child)"
    ../tools/window-screenshot.sh "$pid" "$out/new-tab-plate.png" || fail "no window to capture"
    # Words are a search, and the box's Go pill names the engine.
    send type arc browser
    expect '^browser: command bar drew "arc browser"$' "the New Tab box didn't take the words"
    ../tools/window-screenshot.sh "$pid" "$out/new-tab-search.png" || fail "no window to capture"
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
    # The page is back with the keyboard.
    send state
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
# The click lands on the New Tab page, which goes, and the page comes back without the keyboard.
send click $page_point
expect '^browser: command bar hidden$' "a click on the New Tab page didn't close the command bar"
expect '^browser: new tab plate 0$' "a click on the New Tab page didn't give the tab back its plate"
send state
in_browser AccessKitSubclassOfGPUIView "after a click on the New Tab page closed the command bar"
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the page that came back"
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
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q asks first, and Cancel keeps the browser and Quit, and don't ask again quits, the command bar takes the keyboard and gives it back, with New Tab's plate while ⌘T's is open, goes to a host over https and searches for words, Japanese input composes in both, and the Edit menu undoes, redoes and selects in the page. Windows: $out/command-bar.png, $out/search.png, $out/ime.png"
