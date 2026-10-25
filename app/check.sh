#!/bin/sh
# Runs the app twenty-three times and checks:
#   1. The start page loads in the browser window, over the GPUI sidebar drawn in it, and
#      quitting through AppKit closes the page and ends every Chromium process.
#   2. Clicks move keyboard focus between the page and the UI, and typing goes where the focus
#      is. With the page focused, the browser's reserved shortcuts reach the UI and never the
#      page, and the others reach the page first, which can keep them. ⌘Q quits. The command
#      bar opens over the page with the keyboard, never showing the last query, gives the
#      keyboard back when it closes, and loads what's typed in it, over https for a host typed
#      without one, or searches DuckDuckGo for words. While ⌘T's is open, "New Tab" has the
#      selected plate. Japanese input composes and commits in the page and in the command bar,
#      where return and escape go to the input method while it composes. The Edit menu's Undo,
#      Redo and Select All reach the focused page.
#   3. A tab opened from the command bar goes at the top of the Space's Today tabs and shows. The
#      command bar suggests pages from history as it's typed in, growing to fit them, with the
#      tabs already at them, never the one shown, and well-known sites under them. ↑, ↓ and ⌃N
#      choose a row, wrapping at the ends, and ↩ or a click on a tab's row shows that tab without
#      opening one. Typing at the end completes the text to the first row's page, which ⌫ takes
#      back, → keeps and ↩ goes to. ⌘1–⌘9, ⌥⌘↓, ⌥⌘↑ and clicks on rows show tabs. ⌃Tab, with the
#      page or the UI focused, shows the tab shown before, and with ⌃ held goes on back through the
#      tabs in the order they were shown, and ⌃⇧Tab the other way, until ⌃ goes up or a click. A
#      tab a page opens goes under it and closing it goes back there, and ⌘W otherwise shows the
#      tab below, or above at the bottom. Closing the last tab leaves the Space empty, where ⌘T
#      still works.
#      Reports how long switches take.
#   4. The page view keeps its margins and GPUI's view fills the window through a resize and
#      full screen, and the window's buttons stay in the header row. A page that goes full
#      screen takes the window with it and escape brings both back, the command bar opens in
#      full screen and keeps the keyboard when a page commits under it, then gives the keyboard
#      to the new page, and scrolling a long page doesn't stutter. Dragging the window's
#      background moves the window, and the buttons, URL field and rows keep their presses. A
#      double-click there zooms it, and one in the sidebar below the rows opens a new tab. ⌘S,
#      with the page or the UI focused, and the sidebar button hide the sidebar and the window's
#      buttons, and the card takes its place, through a resize; ⌘S brings them back. The
#      pointer kept at the window's edge brings the hidden sidebar out, and one passing it
#      doesn't. The sidebar's edge drags its width between 160 and 400, rubber-banding past
#      them, and a double-click on it puts the width back.
#   5. A page's popup opens as a tab in the window, under the page. The page's context menu is
#      Chrome's without the items the window can't do, its Open Link in New Tab opens a tab under
#      the page without showing it, and its Print and ⌘P from the page show print preview
#      centred over the page, at its top, which escape closes. Switching or closing a full-screen
#      page's tab ends its full screen and the window's.
#   6. Back and forward go through the tab's history from keys, with the page or the UI focused,
#      and from the header's buttons, which dim with nowhere to go. Reload loads the page again,
#      and while a page loads, reload is stop, which ⌘., escape and the button do. A tab's row
#      shows its page's favicon, and keeps it until the next page has loaded without one.
#   7. New Space in the Spaces menu makes a Space and shows it, empty and without a page, with a
#      form that takes the keyboard from the page. Japanese input composes in its field, return
#      while composing only commits, and return names the Space; ⌘T then opens a tab in it.
#      ⌃1–⌃9, ⌥⌘← and ⌥⌘→, with the page or the UI focused, and clicks on the switcher's dots show
#      another Space, which shows the tab it showed last. Two fingers across the sidebar page to
#      the Space a flick or the pages' place points at, never past the last, fingers along it
#      don't, and a wheel's tick across pages once. A tab's menu moves the tab to another Space,
#      which the window shows with it. Return with no name does nothing, New Space shows a form
#      already open, and Cancel deletes its Space and goes back. A right-click on the Space's name
#      or a dot shows Rename Space, its colours and Delete Space. Rename, and a
#      double-click on the name, turn it into a field, where return keeps the name and escape
#      cancels. Delete asks first for a Space with tabs, which close with it and reopen with ⇧⌘T
#      in the Space shown, and the last Space can't be deleted.
#   8. Launched again after ⌘Q, and again after its window's close button, the app brings back
#      the Spaces: their names and colours, their tabs with each tab's history, the tab each showed
#      and the Space it showed, and the sidebar's width. Only a tab shown loads; the others show
#      their titles and favicons. A tab named with a double-click on its row, where escape cancels,
#      keeps its name over its page's title, and Rename Tab in its menu with no name gives the
#      title back. A tab closed before the quit stays closed, in the archive, from which the
#      Library reopens it at its page. A page's downloads go in the check's folder, and ⇧⌘J opens
#      the Library on Downloads, which lists them. With one running, ⌘Q asks first: Continue
#      Downloading shows the Library's Downloads, and Quit quits, cancelling it. After the
#      relaunch, Remove from List and Clear List take them off the list and keep their files. A
#      SIGTERM, which macOS sends at logout and restart, keeps a tab opened just before it.
#      Reports how long putting them back takes.
#      With the auto-archive's 12 hours made 8 s, a Today tab no window has shown for that long
#      goes in the archive, counting the time across a quit, and the tab shown stays.
#   9. ⌘F opens the find bar over the page with the keyboard. Typing finds in the page, with "n/m",
#      once Japanese input has committed, and return, shift-return, ⌘G and ⇧⌘G and the bar's
#      arrows go through the matches, from the bar and from the page. A text the page doesn't have
#      shows NOT FOUND. A click on the page leaves the bar open, and the bar keeps its place
#      through a resize and full screen. Each tab has its own find, which a new tab starts from the
#      text found last; escape and the bar's close button close it, and so does another page.
#      ⌘0 and ⌘- from the page and the View menu's Zoom In zoom the page, and the zoom HUD shows
#      the percent under the find bar for a moment, and goes at once when another tab shows.
#      ⌥⌘I from the page docks DevTools in the card, filling it, with the page on its left, through
#      a resize, and ⌥⌘I again closes them and gives the page the keyboard. View › Developer's
#      JavaScript Console opens them, a tab without them shows its page across the card, and View
#      Source opens the page's source in a tab under it.
#  10. ⌘D pins the tab shown, from the page and from the UI, at the bottom of the pinned tabs, and
#      unpins it to the top of Today. ⌘1–⌘9 count the pinned tabs first. ⌘W, or the ✕ under the
#      pointer, unloads a pinned tab and keeps its row, which has no ✕ then, and shows the top
#      Today tab, else the nearest pinned tab still loaded, else no page, as closing the last
#      Today tab does. Shown again, the tab loads. Away from its pinned URL, ⌘↩ from the page and
#      the UI, and a click on its favicon, go back there. Launched again, the app brings back the
#      pinned tab pinned and away from its pinned URL, without loading it. Dragging a row moves its
#      tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back.
#      New Folder makes a folder under the pinned tabs, which takes the name typed. A row dragged
#      right under it goes in and dragged left comes out, a folder drags as a row, and a click
#      closes and opens it. Launched again, the app brings it back closed, a double-click renames
#      it, and Delete Folder in its menu leaves its tab where it was.
#  11. View › Clear Cache and Refresh and the command bar's Clear Cookies and Refresh delete the
#      cache and the cookies of the page's site, and load the page again. ⇧⌘C and ⌥⇧⌘C from the
#      page and the UI copy the tab's URL and a Markdown link to it, to a pasteboard of the run's
#      own. The sidebar shows Clear and the URL field's button only while
#      the pointer is over it, and the button copies the URL. ⇧⌘K from the page and the UI, and
#      Clear on the divider, close the Space's Today tabs but the one shown, with no toast, and ⌘Z
#      from the page and the UI brings them back in place, unloaded, unless the page's text field
#      has the keyboard, when ⌘Z is its own. ⇧⌘T from the page and the UI reopens the tab closed
#      last, where it was and with its history, and after a Clear, ⌘Z brings back only the tabs ⇧⌘T
#      didn't. A right-click on a row shows its tab's menu, whose Duplicate Tab, Close Tabs Below
#      and Unpin Tab work. ⌘W archives a Today tab. ⇧⌘L, View Library and the footer's button open
#      the Library, which pushes the page right at its width, and its back button closes it. A
#      click on an archived tab reopens it at the top of Today with its history, and Clear Archive
#      asks, then empties the archive.
#  12. Add Split View, from the menu and ⌃+ from the page, narrows the page at once for a pane on
#      its right, with the command bar over it, which escape takes back. A URL typed there opens in
#      the pane, focused. The panes share the card 10 apart, the focused one ringed, and a click
#      focuses a pane and gives its page the keyboard. The split is one row in the sidebar, which
#      ⌘1–⌘9 count once and show with the pane focused last, and a click on a segment focuses its
#      pane. Launched again, the app brings the split back with both pages. Separate Page from
#      Split View shows the page alone, ⌥↩ in the command bar opens a URL beside the page, and ⌘W
#      closes the focused pane, leaving the page beside it alone. The ✕ on a split's row closes
#      all its panes, and the window shows the tab above them. A row dragged onto the page brings
#      a target to each of the card's sides, which go when it's back over the sidebar, and let go
#      on the left one, the tab joins the page shown on its left.
#  13. Add to Favorites in a Today tab's menu takes the tab out of Today into a tile above the
#      Space's rows, without showing it. A click on the tile shows it, and ⌘1–⌘9 count favorites
#      first. Away from its URL, a double-click on the tile goes back there. ⌘W unloads it and
#      keeps the tile, showing the top Today tab. The tile's menu duplicates it to the top of
#      Today. Launched again, the app brings the favorite back, shown, and Remove from Favorites
#      in its menu puts it at the top of Today. Today rows dragged onto the tiles become favorites
#      where they're dropped, a tile dragged across the other goes before it, one dragged into the
#      list goes there, and escape puts a dragged tile back.
# Runs 2 to 13 need Browser to be the active app, and runs 4 and 9 take it full screen for a
# moment.
# BROWSER_DISPLAY=builtin opens it on the Mac's own screen.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
set -e
cd "$(dirname "$0")"
app="$PWD/build/Browser.app"
out=build/check
# Another Browser holds the profile directory, so this one couldn't start.
if pgrep -f "$app/Contents" > /dev/null; then
  echo "FAIL: Browser is already running. Quit it first." >&2
  exit 1
fi
rm -rf "$out"
mkdir -p "$out"
# The Spaces the app saves for its next launch. Each run starts without them, but the relaunches.
saved="$HOME/Library/Caches/browser-app/Spaces.json"
# DevTools open where the profile last had them, so an undocked try would leave them in a window.
# Run 9 docks them, so they start on the right. Run 8's download goes in the check's own folder,
# never the user's Downloads.
prefs="$HOME/Library/Caches/browser-app/Default/Preferences"
mkdir -p "$(dirname "$prefs")" "$out/downloads"
python3 -I -c '
import json, os, sys
prefs = {}
if os.path.exists(sys.argv[1]):
    with open(sys.argv[1]) as f:
        prefs = json.load(f)
prefs.setdefault("devtools", {}).setdefault("preferences", {})["currentDockState"] = "\"right\""
prefs.setdefault("download", {}).update(default_directory=sys.argv[2], prompt_for_download=False)
with open(sys.argv[1], "w") as f:
    json.dump(prefs, f)
' "$prefs" "$PWD/$out/downloads"
# A Space's colours in its menu, for a Space without one.
colors="Color › System Accent (checked), Color › Gray, Color › Red, Color › Orange, \
Color › Yellow, Color › Green, Color › Blue, Color › Pink, Color › Purple"
# After a failure, nothing the check started keeps running.
cleanup() {
  # The server may be gone already, and under set -e a failed kill would end the trap here.
  [ -z "${server:-}" ] || kill "$server" 2>/dev/null || true
  kill "$pid" 2>/dev/null || return 0
  for _ in $(seq 10); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -9 "$pid" 2>/dev/null
  pkill -9 -f "$app/Contents" || true
}
trap cleanup EXIT
fail() {
  echo "FAIL: $1" >&2
  grep -v '^$' "$log" | tail -5 >&2
  exit 1
}

# Waits up to 30 s for a line matching $1 in the log of the current run.
await() {
  for _ in $(seq 300); do
    grep -q "$1" "$log" && return 0
    kill -0 "$pid" 2>/dev/null || fail "the app exited"
    sleep 0.1
  done
  return 1
}

# The app logs "browser: loaded" when the main frame has loaded.
await_load() {
  await '^browser: load' || fail "the page didn't finish loading"
  grep -q '^browser: loaded' "$log" || fail "the page didn't load"
}

# Waits up to $1 seconds (10) for the app to exit and checks that no Chromium process outlives it.
exited() {
  for _ in $(seq $((${1:-10} * 2))); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  if kill -0 "$pid" 2>/dev/null; then
    sample "$pid" 2 -file "$out/hang.txt" > /dev/null 2>&1
    fail "the app didn't quit. Where it was: $out/hang.txt"
  fi
  wait "$pid" || fail "the app exited with status $?"
  for _ in $(seq 10); do
    pgrep -f "$app/Contents" > /dev/null || break
    sleep 0.5
  done
  pgrep -f "$app/Contents" > /dev/null && fail "Chromium helper processes were left running"
  return 0
}

# A quit Apple Event goes through -[NSApplication terminate:], the way macOS quits apps when the
# user logs out.
quit() {
  osascript -e 'tell application id "dev.browser.app" to quit'
  exited "$@"
}

log="$out/load.log"
rm -f "$saved"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await_load
../tools/window-screenshot.sh "$pid" "$out/load.png" || fail "no window to capture"
quit
echo "ok: $(grep '^browser: loaded' "$log" | cut -d' ' -f4-) loaded, quit cleanly. Window: $out/load.png"

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
mkfifo "$out/drive"
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the test page didn't draw"

# Sends a command to the app's driver (platform/mac/drive.mm) and waits until AppKit has handled it.
step=0
send() {
  step=$((step + 1))
  echo "$*" >&3
  await "^browser: drive $step done" || fail "the driver didn't finish \"$*\""
  grep -q "^browser: drive $step: can't do that" "$log" && fail "the driver can't do \"$*\""
  done=$(grep "^browser: drive $step done" "$log")
  key=$(echo "$done" | sed 's/.*key \([^,]*\),.*/\1/')
  focus=$(echo "$done" | sed 's/.*focus \([^,]*\),.*/\1/')
  active=$(echo "$done" | sed 's/.*active \([01]\).*/\1/')
  marked=$(echo "$done" | sed 's/.*marked //')
}
# Sends a command without waiting for it: a menu or an alert it opens keeps the driver's events
# until it closes.
post() {
  step=$((step + 1))
  echo "$*" >&3
}
# Waits for a line matching $1 logged after the last command was sent.
expect() {
  for _ in $(seq 100); do
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    sleep 0.1
  done
  fail "$2"
}

# Points from the window's top-left corner. The sidebar is 228 wide.
page_point="600 400"
sidebar_point="100 400"
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

# Chrome's browser window has the keyboard, with focus on $1.
in_browser() {
  [ "$key" = BrowserCoreNSWindow ] && [ "$focus" = "$1" ] ||
    fail "$2: the keyboard is in $key, on $focus"
}
# ⌘$1 with the page focused reaches the UI as command $2.
page_shortcut() {
  send key "$1" cmd
  expect "^browser: command $2 via page$" "⌘$(echo "$1" | tr a-z A-Z) with the page focused didn't reach the UI"
}
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

# Nothing logged since the last command was sent matches $1.
none_since() {
  ! sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1"
}
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
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via page$' || fail "⌘Q with the page focused didn't reach the UI"
exited
echo "ok: focus follows clicks, typing reaches the focused side, reserved shortcuts skip the page and the others reach it first, ⌘Q quits, the command bar takes the keyboard and gives it back, with New Tab's plate while ⌘T's is open, goes to a host over https and searches for words, Japanese input composes in both, and the Edit menu undoes, redoes and selects in the page. Windows: $out/command-bar.png, $out/search.png, $out/ime.png"

# A Space's Today tabs. The page opens a tab of its own from its button, the way a link to a new
# tab does. The app asks Chrome to trace its tab switches, timed as Chrome times its own: from the
# tab strip's change to the first frame shown. The trace is written as the app quits. In a handler
# attribute a bare open() is document.open(), which would blank the page instead.
page='data:text/html,<title>loading</title>
<button onclick=window.open() style="font:24px system-ui">Open a tab</button>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/tabs.log"
trace="$PWD/$out/trace.json"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" --enable-tracing=latency \
  --enable-tracing-format=json --trace-startup-file="$trace" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the tabs test page didn't draw"

# The Today tabs the app last showed, top to bottom.
today() {
  grep '^browser: today' "$log" | tail -1 | sed 's/^browser: today *//'
}
# Waits for the app to show Today as $1, with $2 shown. $3 says what went wrong if it doesn't.
expect_tabs() {
  expect "^browser: today $1\$" "$3: Today is \"$(today)\", not \"$1\""
  expect "^browser: selected $2\( in .*\)\{0,1\}\$" "$3: tab $2 isn't shown"
}
# Opens a tab from the command bar, to https://$1.
open_from_bar() {
  before=$(today)
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  expect '^browser: new tab plate 1$' "⌘T didn't give New Tab the selected plate"
  send type "$1"
  send key return
  expect "^browser: today [0-9][0-9]*${before:+ $before}\$" "the tab for $1 didn't open at the top of Today"
  tab=$(today | cut -d' ' -f1)
  expect "^browser: selected $tab\$" "the tab for $1 wasn't shown"
  expect '^browser: new tab plate 0$' "the tab for $1 didn't take the selected plate"
  expect "^browser: loaded $tab https://$1/\$" "$1 didn't load in its tab"
}
# The driver's key $1 shows tab $2, as a switch the app times.
switch() {
  send key $1
  expect "^browser: selected $2 in " "key $1 didn't show tab $2 of \"$(today)\""
}
# The center of row $1 of Today, counting from 0: rows are 41 apart under "+ New Tab", which is
# under the divider, 15 tall, and the Space's name, 44 tall and 90 below the window's top.
row() {
  echo "100 $((90 + 44 + 15 + 41 * ($1 + 1) + 20))"
}

await '^browser: today [0-9][0-9]*$' || fail "the first tab isn't in Today"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: tabs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
expect_tabs "$c $b $a" "$c" "after opening two tabs"
../tools/window-screenshot.sh "$pid" "$out/tabs.png" || fail "no window to capture"

# The command bar's last suggestions, as "<url>[ tab <id>]" a line.
suggestions() {
  grep '^browser: command bar suggestions' "$log" | tail -1 | sed 's/^[^:]*: [^:]* ms: //' |
    tr -s ', ' '\n ' | sed 's/^ //'
}
# The command bar's height logged last.
bar_height() {
  grep '^browser: command bar height' "$log" | tail -1 | sed 's/.* //'
}
# Opens the command bar and types "example", which completes to a page and suggests example.com
# and example.org. ⌫ takes the completion back, so the first row is the search for what was typed.
# Waits for $1's tab among the rows, and checks that $2's isn't, since $2 is shown.
suggest_example() {
  send key t cmd
  expect '^browser: command bar shown$' "⌘T didn't open the command bar"
  send type example
  expect '^browser: command bar completes "example" with "\.' "the command bar didn't complete \"example\""
  send key backspace
  expect "^browser: command bar suggestions \"example\" in [0-9.]* ms: https://duckduckgo.com/?q=example, .*https://$1/ tab " \
    "the command bar didn't suggest $1's tab: $(suggestions | tr '\n' ' ')"
  rows=$(suggestions | wc -l | tr -d ' ')
  # The bar may have grown to as many rows before.
  for _ in $(seq 50); do
    [ "$(bar_height)" = $((54 + 8 + 36 * rows)) ] && break
    sleep 0.1
  done
  [ "$(bar_height)" = $((54 + 8 + 36 * rows)) ] || fail "the command bar is $(bar_height) tall, not fitting $rows rows"
  none_since "https://$2/ tab " || fail "the command bar suggested $2's tab, which is shown"
}
suggest_example example.com example.org
send key up
last=$(suggestions | tail -1 | cut -d' ' -f1)
expect "^browser: command bar selected $last\$" "↑ on the first row didn't select the last"
send key n ctrl
expect '^browser: command bar selected https://duckduckgo.com/?q=example$' "⌃N on the last row didn't select the first"
for _ in 1 2 3 4 5 6 7 8; do
  send key down
  grep '^browser: command bar selected' "$log" | tail -1 | grep -q ' https://example.com/$' && break
done
../tools/window-screenshot.sh "$pid" "$out/suggestions.png" || fail "no window to capture"
send key return
expect "^browser: command bar switch to tab $b\$" "↩ on example.com's row didn't switch to its tab"
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "↩ on example.com's row didn't show its tab"
[ "$(today)" = "$c $b $a" ] || fail "switching to a tab changed Today to \"$(today)\""
suggest_example example.org example.com
# Rows are 36 tall, under the 54 of the input and an inset of 4.
row=$(suggestions | grep -n "^https://example.org/ tab $c\$" | cut -d: -f1)
send clickkey 320 $((54 + 4 + 36 * (row - 1) + 18))
expect "^browser: command bar switch to tab $c\$" "a click on example.org's row didn't switch to its tab"
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "a click on example.org's row didn't show its tab"
[ "$(today)" = "$c $b $a" ] || fail "switching to a tab changed Today to \"$(today)\""

# Typing at the end completes the text to the first row's page, as Chrome's address bar does:
# example.com, typed before, completes "example.c". ⌫ takes the completion back until the next key
# typed at the end, → keeps it as typed, and ↩ goes where it goes.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.c
expect '^browser: command bar completes "example.c" with "om"$' "the command bar didn't complete \"example.c\" to example.com"
expect '^browser: command bar drew "example.com"$' "the command bar didn't show the completion"
../tools/window-screenshot.sh "$pid" "$out/completion.png" || fail "no window to capture"
send key backspace
expect '^browser: command bar suggestions "example.c" in ' "the command bar didn't suggest again after ⌫"
expect '^browser: command bar drew "example.c"$' "⌫ didn't take the completion back"
none_since '^browser: command bar completes' || fail "the command bar completed the text again after ⌫"
send type o
expect '^browser: command bar completes "example.co" with "m"$' "typing at the end after ⌫ didn't complete again"
send key right
expect '^browser: command bar suggestions "example.com" in ' "→ didn't keep the completion as typed"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type example.c
expect '^browser: command bar completes "example.c" with "om"$' "the command bar didn't complete \"example.c\" to example.com"
send key return
expect '^browser: command bar go https://example.com/$' "↩ didn't go where the completion goes: $(grep '^browser: command bar go' "$log" | tail -1)"
expect "^browser: loaded $c https://example.com/\$" "↩ didn't load the completion's page in the tab"
[ "$(today)" = "$c $b $a" ] || fail "going to a page in a tab changed Today to \"$(today)\""
# A click on the URL field opens the bar on the tab's URL, out of the field: at its left edge and
# with the input centred on it. ⌘T's is centred.
send click 100 64
expect '^browser: command edit-url via sidebar$' \
  "a click on the URL field didn't open the command bar"
expect '^browser: command bar shown$' "a click on the URL field didn't open the command bar"
expect '^browser: command bar drew "https://example.com/"$' \
  "the command bar from the URL field didn't show the tab's URL"
at=$(grep '^browser: command bar at' "$log" | tail -1 | cut -d' ' -f5-)
[ "$at" = "8 37 640 54" ] || fail "the command bar from the URL field opened at $at, not out of it"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

# Well-known sites fill the rows under the profile's own, never first, so ↩ still searches.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type g
expect '^browser: command bar suggestions "g" in [0-9.]* ms: https://duckduckgo.com/?q=g, .*https://www.google.com/, https://mail.google.com/' \
  "the command bar didn't suggest well-known sites under the search: $(suggestions | tr '\n' ' ')"
../tools/window-screenshot.sh "$pid" "$out/popular.png" || fail "no window to capture"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

switch "3 cmd" "$a"
switch "1 cmd" "$c"
switch "9 cmd" "$a"
switch "up alt cmd" "$b"
switch "down alt cmd" "$a"
# Next wraps around, as in Chrome.
switch "down alt cmd" "$c"
# ⌃Tab goes back through the tabs in the order they were shown, c, a and b by now, as far as ⌃
# stays down. Then the tab it showed counts as shown last.
send click $page_point
switch "tab ctrl" "$a"
expect '^browser: command recent-tab via page$' "⌃Tab with the page focused didn't reach the UI"
switch "tab ctrl" "$b"
send flags
switch "tab ctrl" "$c"
send flags
switch "tab ctrl shift" "$a"
send flags
send click $sidebar_point
switch "tab ctrl" "$c"
expect '^browser: command recent-tab via ui$' "⌃Tab with the UI focused didn't reach the UI"
send flags
# A click ends the walk too.
switch "tab ctrl" "$a"
send click $(row 1)
expect "^browser: selected $b in " "a click on the second row didn't show its tab"
switch "tab ctrl shift" "$c"
send flags
send click 100 154
expect '^browser: command new-tab via sidebar$' "a click on \"+ New Tab\" didn't run New Tab"
expect '^browser: command bar shown$' "a click on \"+ New Tab\" didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

switch "3 cmd" "$a"
send click 290 34
expect "^browser: tab [0-9]* opened by $a in the foreground\$" "the page's button didn't open a tab"
d=$(grep '^browser: tab [0-9]* opened by' "$log" | tail -1 | cut -d' ' -f3)
expect_tabs "$c $b $a $d" "$d" "a tab the page opened"
send key w cmd
expect_tabs "$c $b $a" "$a" "after closing the page's tab"

switch "1 cmd" "$c"
send key w cmd
expect_tabs "$b $a" "$b" "after closing the top tab"
switch "9 cmd" "$a"
send key w cmd
expect_tabs "$b" "$b" "after closing the bottom tab"
send key w cmd
expect_tabs " *" 0 "after closing the last tab"
../tools/window-screenshot.sh "$pid" "$out/empty.png" || fail "no window to capture"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in an empty Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in an empty Space didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
exec 3>&-
# Chrome writes the trace before it quits, once every process has started tracing or 10 s have
# passed, so a renderer that closed early can hold the quit that long.
quit 30

# Switch times in milliseconds, one a line, as "median <m> ms, max <x> ms, <n> switches".
summary() {
  sort -n | awk '{ v[NR] = $1 } END { if (NR) printf "median %.1f ms, max %.1f ms, %d switches", v[int((NR + 1) / 2)], v[NR], NR }'
}
app_ms=$(sed -n 's/^browser: selected [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
[ -s "$trace" ] || fail "the app didn't write its trace"
chrome_ms=$(python3 -I - "$trace" << 'PY' | summary
import json, sys
# Chrome's tab switch spans, begin and end events paired by id, in microseconds. A switch to a tab
# whose page hasn't loaded yet, as when a new tab shows, times the page load; Chrome reports those
# apart from switches, and so does this.
events = json.load(open(sys.argv[1]))
events = events["traceEvents"] if isinstance(events, dict) else events
begins = {}
for event in events:
    if event.get("name") != "TabSwitching::Latency":
        continue
    if event.get("ph") == "b":
        state = event.get("args", {}).get("tab_switch_measurement", {}).get("tab_state", "")
        begins[event.get("id")] = (event["ts"], "NOT_LOADED" in state)
    elif event.get("ph") == "e" and event.get("id") in begins:
        begin, not_loaded = begins.pop(event.get("id"))
        if not not_loaded:
            print((event["ts"] - begin) / 1000)
PY
)
[ -n "$chrome_ms" ] || fail "the trace has no tab switches"
echo "ok: new tabs open at the top and show, the command bar suggests pages, their tabs and well-known sites and completes what's typed, keys and clicks switch, ⌃Tab goes back through the tabs shown while ⌃ is held, a page's tab goes under it and closing it goes back, ⌘W shows the tab below or above, an empty Space still opens tabs."
echo "  Switch to a loaded tab, tab strip to frame shown (Chrome's measure; gate 16 ms): $chrome_ms."
echo "  Switch, key to the core's report: $app_ms. Windows: $out/tabs.png, $out/suggestions.png, $out/completion.png, $out/popular.png, $out/empty.png"

# A long page that times its frames while it scrolls and puts the numbers in its title.
page=$(base64 << 'HTML' | tr -d '\n'
<!doctype html>
<title>loading</title>
<style>
  body { font: 16px/1.5 system-ui; margin: 0 auto; max-width: 720px; padding: 0 16px }
  .card { margin: 16px 0; padding: 16px; border-radius: 12px; box-shadow: 0 2px 8px rgb(0 0 0 / 15%) }
</style>
<script>
  const text = "The quick brown fox jumps over the lazy dog. ".repeat(12);
  for (let i = 0; i < 300; i++) {
    document.write(`<div class=card style="background: hsl(${(i * 37) % 360} 70% 92%)"><h2>Section ${i}</h2><p>${text}</p></div>`);
  }
  // requestAnimationFrame runs once per frame the page draws, so the gaps between its calls are
  // frame times. Frames count while scroll events arrive. A frame slower than 60 Hz shows as a
  // stutter on any display.
  const intervals = [];
  let last = 0, lastScroll = 0, measuring = false;
  addEventListener("scroll", () => {
    lastScroll = performance.now();
    if (!measuring) {
      measuring = true;
      requestAnimationFrame(frame);
    }
  }, { passive: true });
  function frame(time) {
    if (last && time - lastScroll < 20) intervals.push(time - last);
    last = time;
    if (performance.now() - lastScroll < 300) return requestAnimationFrame(frame);
    const sorted = [...intervals].sort((a, b) => a - b);
    const at = q => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))].toFixed(1);
    const fps = Math.round(1000 * intervals.length / intervals.reduce((a, b) => a + b, 0));
    const slow = intervals.filter(i => i > 1000 / 60).length;
    document.title = `frames ${intervals.length} fps ${fps} median ${at(0.5)} p99 ${at(0.99)} max ${at(1)} slow ${slow} scrolled ${Math.round(scrollY)}`;
  }
  // An f takes the page full screen, as the full-screen button of a video does.
  addEventListener("keydown", e => e.key == "f" && !e.metaKey && document.documentElement.requestFullscreen());
  // After an n, the page goes to another site when its window loses the keyboard, so a new page
  // commits while the command bar is open.
  let leave = false;
  addEventListener("keydown", e => leave ||= e.key == "n");
  addEventListener("blur", () => leave && (location = "https://example.com/"));
  requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
</script>
HTML
)
log="$out/window.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="data:text/html;base64,$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the scrolling page didn't draw"

# The page view keeps the sidebar's 228 points on its left and 10 on its other sides, over GPUI's
# view, which fills the window. A full-screen page ("fills") covers the whole window, and with the
# sidebar hidden ("inset") it's 10 in from every side.
check_layout() {
  send layout
  layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout')
  content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
  view=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
  radius=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa [0-9,]* [0-9x]* radius \([0-9]*\).*/\1/p')
  ui=$(echo "$layout" | sed -n 's/.*GPUIView \([0-9,]* [0-9x]*\).*/\1/p')
  width=${content%x*}
  height=${content#*x}
  # Past the sidebar, 228 wide unless $2 gives its width.
  left=228
  case "${2:-}" in [0-9]*) left=$2 ;; esac
  expected="$left,10 $((width - left - 10))x$((height - 20))"
  # Rounded as the card's corners, square when it fills the window.
  expected_radius=6
  [ "${2:-}" = fills ] && expected="0,0 $content" && expected_radius=
  [ "${2:-}" = inset ] && expected="10,10 $((width - 20))x$((height - 20))"
  [ "$view" = "$expected" ] || fail "$1: the page view is at $view in a $content window"
  [ "$radius" = "$expected_radius" ] ||
    fail "$1: the page's corners have a radius of ${radius:-0}, not ${expected_radius:-0}"
  [ "$ui" = "0,0 $content" ] || fail "$1: GPUI's view is at $ui in a $content window"
}
# The window's buttons sit in the header row, where Arc's do. In full screen they come down with
# the menu bar instead. They go with the sidebar ("hidden").
check_buttons() {
  buttons=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout buttons' |
    cut -d' ' -f4-)
  expected="20.0,24.0 43.0,24.0 66.0,24.0"
  [ "${2:-}" = hidden ] && expected="hidden hidden hidden"
  [ "$buttons" = "$expected" ] || fail "$1: the window's buttons are at $buttons, not $expected"
}
check_layout "at launch"
check_buttons "at launch"

# 300 trackpad events of 10 points, 8 ms apart.
send scroll $page_point 10 300
expect '^browser: title [0-9]* frames' "scrolling didn't move the page"
timing=$(grep -m1 '^browser: title [0-9]* frames' "$log" | sed 's/^browser: title [0-9]* //')
echo "$timing" | grep -q ' scrolled 3000$' || fail "the page didn't scroll by exactly 3000 points: $timing"
frames=$(echo "$timing" | sed 's/^frames \([0-9]*\) .*/\1/')
slow=$(echo "$timing" | sed 's/.* slow \([0-9]*\) .*/\1/')
[ $((slow * 100)) -le "$frames" ] || fail "more than 1% of frames took longer than 1/60 s: $timing"

send resize 1000 700
check_layout "after a resize"
check_buttons "after a resize"
send fullscreen
expect '^browser: window entered full screen$' "the window didn't go full screen"
check_layout "in full screen"
send key l cmd
expect '^browser: command bar shown$' "⌘L in full screen didn't open the command bar"
send state
[ "$key" = GPUIPanel ] || fail "the command bar in full screen didn't get the keyboard, $key did"
../tools/window-screenshot.sh "$pid" "$out/fullscreen.png" || fail "no window to capture"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar in full screen"
send fullscreen
expect '^browser: window left full screen$' "the window didn't leave full screen"
check_layout "after full screen"
check_buttons "after full screen"

# A page that goes full screen takes the window with it, and escape brings both back.
send key f
expect '^browser: page entered full screen$' "the page didn't go full screen"
expect '^browser: window entered full screen$' "the window didn't go full screen with the page"
check_layout "with the page full screen" fills
../tools/window-screenshot.sh "$pid" "$out/page-fullscreen.png" || fail "no window to capture"
send key escape
expect '^browser: page left full screen$' "escape didn't take the page out of full screen"
expect '^browser: window left full screen$' "the window didn't leave full screen with the page"
check_layout "after the page's full screen"
check_buttons "after the page's full screen"

# Chromium makes its window key when a new page's view replaces the focused one.
send key n
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
expect '^browser: loaded [0-9]* https://example.com/$' "the page didn't leave when the command bar opened"
send state
[ "$key" = GPUIPanel ] || fail "a page that committed under the command bar took the keyboard, $key has it"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar after a page committed under it"
send state
in_browser RenderWidgetHostViewCocoa "after the command bar closed over a page that committed under it"

# The window's background moves the window, as a title bar does: the header row outside its
# buttons, the sidebar below the rows and the card's margins. The buttons, the URL field and the
# rows and the Space's name keep their presses. Each drag moves 5 points, about a pointer's first step. Posted events
# don't move the window itself: the window server follows the real pointer.
moves=0
drag() {
  send drag "$1" "$2" $(($1 + 5)) "$2"
  count=$(grep -c '^browser: window move$' "$log" || true)
  if [ "$3" = moves ]; then
    [ "$count" -gt "$moves" ] || fail "a drag on $4 didn't move the window"
  else
    [ "$count" = "$moves" ] || fail "a drag on $4 moved the window"
  fi
  moves=$count
}
drag 80 24 moves "the header"
drag 138 24 stays "the back button"
drag 110 64 stays "the URL field"
drag 110 112 stays "the Space's name"
drag 110 195 stays "a tab's row"
drag 110 400 moves "the empty sidebar"
drag 600 5 moves "the card's top margin"
drag $((width - 8)) 300 moves "the card's right margin"
# A double-click there does what System Settings › Desktop & Dock says a title bar's does: zoom,
# unless it's set to something else.
action=$(defaults read -g AppleActionOnDoubleClick 2>/dev/null || echo Maximize)
if [ "$action" = Maximize ]; then
  send doubleclick 80 24
  expect '^browser: window zoomed 1$' "a double-click on the header didn't zoom the window"
  send doubleclick 80 24
  expect '^browser: window zoomed 0$' "a second double-click on the header didn't zoom it back"
  zoom="zooms it"
else
  zoom="isn't checked, since a title bar's double-click is set to $action here"
fi
send doubleclick 110 400
expect '^browser: command new-tab via sidebar$' "a double-click on the empty sidebar didn't open a new tab"
expect '^browser: command bar shown$' "a double-click on the empty sidebar didn't open the command bar"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"

# ⌘S hides the sidebar, with the page focused or the UI, and the card takes its place, 10 in from
# every side, as the window's buttons go. ⌘S and the sidebar button bring both back.
# toggle <from> <hidden|shown> <what>: waits for the sidebar to finish moving after the command.
toggle() {
  expect "^browser: command toggle-sidebar via $1\$" "$3 didn't reach the UI"
  expect "^browser: sidebar $2\$" "$3 didn't leave the sidebar $2"
  if [ "$2" = hidden ]; then
    check_layout "after $3" inset
    check_buttons "after $3" hidden
  else
    check_layout "after $3"
    check_buttons "after $3"
  fi
}
send click $page_point
send key s cmd
toggle page hidden "⌘S with the page focused"
../tools/window-screenshot.sh "$pid" "$out/sidebar-hidden.png" || fail "no window to capture"
send key s cmd
toggle page shown "⌘S again with the page focused"
send click 110 400
send key s cmd
toggle ui hidden "⌘S with the UI focused"
send key s cmd
toggle ui shown "⌘S again with the UI focused"
send click 104 24
toggle sidebar hidden "a click on the sidebar button"
send key s cmd
toggle ui shown "⌘S after the sidebar button"
# The page stays inset through a resize while the sidebar is hidden.
send resize 900 600
send key s cmd
toggle ui hidden "⌘S in a smaller window"
send resize 1000 700
check_layout "after a resize with the sidebar hidden" inset
send key s cmd
toggle ui shown "⌘S after a resize"

# With the sidebar hidden, the pointer at the window's left edge brings it out over the page in a
# window of its own, which takes clicks and leaves the keyboard to the browser window, and the
# pointer well past it sends it away. A click on its sidebar button, or ⌘S, docks the sidebar.
# check_reveal <what> [shown]: the reveal's window covers the left edge, or there's none.
check_reveal() {
  send layout
  child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
    cut -d' ' -f4-)
  height=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    sed 's/^browser: layout content [0-9]*x\([0-9]*\),.*/\1/')
  expected=
  [ "${2:-}" = shown ] && expected="0,0 233x$height"
  [ "$child" = "$expected" ] ||
    fail "$1: the reveal's window is at ${child:-none}, not ${expected:-none}"
}
send key s cmd
toggle ui hidden "⌘S before the reveal"
send move 400 300
# The pointer has to stay at the edge a moment: one passing it doesn't bring the sidebar out.
step=$((step + 1))
passed=$step
echo "move 3 300" >&3
send move 400 300
send move 3 300
expect '^browser: reveal shown$' "the pointer at the window's edge didn't bring the sidebar out"
[ "$(sed -n "/^browser: drive $passed: /,\$p" "$log" | grep -c '^browser: reveal shown$')" = 1 ] ||
  fail "the pointer passing the window's edge brought the sidebar out"
check_reveal "with the sidebar out" shown
check_layout "with the sidebar out" inset
check_buttons "with the sidebar out" hidden
../tools/window-screenshot.sh "$pid" "$out/sidebar-reveal.png" || fail "no window to capture"
send move 200 300
send move 250 300
check_reveal "with the pointer over the sidebar out" shown
send move 400 300
expect '^browser: reveal hidden$' "the pointer past the sidebar out didn't send it away"
check_reveal "after the sidebar went away"
send move 3 300
expect '^browser: reveal shown$' "the pointer at the edge didn't bring the sidebar back out"
send click 104 24
toggle sidebar shown "a click on the sidebar button of the sidebar out"
check_reveal "with the sidebar docked from its sidebar button"
send key s cmd
toggle ui hidden "⌘S after docking the sidebar out"
send move 3 300
expect '^browser: reveal shown$' "the pointer at the edge didn't bring the sidebar out again"
send key s cmd
toggle ui shown "⌘S with the sidebar out"
check_reveal "with the sidebar docked by ⌘S"
# The sidebar's edge, the 6 pt right of its rows, drags its width between 160 and 400. Past them it
# rubber-bands, and let go it settles at the one it passed. A double-click on it puts back 228.
send drag 225 400 325 400
expect '^browser: sidebar width 328$' "dragging the sidebar's edge didn't widen the sidebar"
check_layout "with the sidebar dragged wider" 328
send press 325 400
send dragto 700 400
for _ in $(seq 20); do
  send layout
  band=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
    sed -n 's/.*WebContentsViewCocoa \([0-9]*\),.*/\1/p')
  [ "${band:-0}" -gt 400 ] && break
done
[ "${band:-0}" -gt 400 ] && [ "$band" -lt 460 ] ||
  fail "dragged far past its widest, the sidebar is ${band:-not} wide, not a little past 400"
../tools/window-screenshot.sh "$pid" "$out/sidebar-band.png" || fail "no window to capture"
send release 700 400
expect '^browser: sidebar width 400$' "let go past its widest, the sidebar didn't settle at 400"
check_layout "with the sidebar at its widest" 400
send drag 397 400 20 400
expect '^browser: sidebar width 160$' "let go past its narrowest, the sidebar didn't settle at 160"
check_layout "with the sidebar at its narrowest" 160
../tools/window-screenshot.sh "$pid" "$out/sidebar-narrow.png" || fail "no window to capture"
send doubleclick 157 400
expect '^browser: sidebar width 228$' "a double-click on the sidebar's edge didn't put its width back"
none_since '^browser: command new-tab' || fail "a double-click on the sidebar's edge opened a tab"
check_layout "after a double-click on the sidebar's edge"
exec 3>&-
quit
echo "ok: the page view keeps its margins and the window's buttons stay at Arc's through a resize and full screen, a full-screen page takes the window with it and escape brings both back, the command bar opens in full screen and keeps the keyboard when a page commits under it, then gives it to the new page. Drags on the window's background move it and drags on its controls don't. A double-click on the header $zoom, and one on the empty sidebar opens a new tab. ⌘S from the page and the UI, and the sidebar button, hide the sidebar and the window's buttons, the card takes its place through a resize, and ⌘S brings them back. The pointer kept at the window's edge brings the hidden sidebar out over the page, one passing it doesn't, and the pointer past it sends it away, and the sidebar button and ⌘S dock it from there. The sidebar's edge drags its width, rubber-bands past 160 and 400 and settles there, and a double-click puts it back. Scrolling: $timing. Windows: $out/fullscreen.png, $out/page-fullscreen.png, $out/sidebar-hidden.png, $out/sidebar-reveal.png, $out/sidebar-band.png, $out/sidebar-narrow.png"

# A page's popup opens as a tab in the window, as links to new tabs do, instead of a window of
# Chrome's own.
page='data:text/html,<title>loading</title>
<a href="https://example.org/" style="font:24px system-ui">a link</a>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
// A popup needs a user gesture, which a key press is.
addEventListener("keydown", e =>
  e.key == "p" && !e.metaKey && open("https://example.com/", "", "popup,width=400,height=300"));
addEventListener("keydown", e =>
  e.key == "f" && !e.metaKey && document.documentElement.requestFullscreen());
</script>'
log="$out/popup.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the popup test page didn't draw"
send click $page_point
send key p
expect '^browser: tab [0-9]* opened by [0-9]* in the foreground$' "the page's popup didn't open as a tab"
opener=$(grep -m1 '^browser: today [0-9]' "$log" | cut -d' ' -f3)
popup=$(grep '^browser: tab [0-9]* opened by' "$log" | cut -d' ' -f3)
expect "^browser: today $opener $popup\$" "the popup's tab didn't go under the page: $(grep '^browser: today' "$log" | tail -1)"
expect '^browser: loaded [0-9]* https://example.com/$' "the popup's page didn't load in its tab"
check_layout "with the popup's tab"
../tools/window-screenshot.sh "$pid" "$out/popup.png" || fail "no window to capture"
# The page's context menu is Chrome's, without the items for what the browser doesn't have yet
# (other windows, split view, incognito) or whose UI is in Chrome's own window (Cast, Reading Mode).
send key 1 cmd
expect "^browser: selected $opener in " "⌘1 didn't show the page again"
post rightclick 250 30
expect '^browser: menu tracked Open Link in New Tab, Save Link As…, Copy Link Address, ' \
  "a right-click on the page's link didn't show the link's menu"
menu=$(grep '^browser: menu tracked' "$log" | tail -1 | cut -d' ' -f4-)
case "$menu" in
*Window* | *"Split View"*) fail "the link's menu offers what the window can't do: $menu" ;;
esac
../tools/window-screenshot.sh "$pid" "$out/link-menu.png" || fail "no window to capture"
send choose Open Link in New Tab
expect "^browser: tab [0-9]* opened by $opener in the background\$" \
  "Open Link in New Tab didn't open a tab under the page"
none_since '^browser: selected' || fail "Open Link in New Tab showed its tab"
# Print preview is the tab's dialog, a child window centred over the page, at its top. Once $1
# has asked for it: waits for it, captures it in $out/$2 once it has drawn over before-print.png,
# a capture of the window before, and closes it with escape.
print_preview() {
  child=""
  for _ in $(seq 50); do
    send layout
    child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
      cut -d' ' -f4-)
    [ -n "$child" ] && break
    sleep 0.1
  done
  [ -n "$child" ] || fail "$1 didn't show print preview"
  shown=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    grep -o 'WebContentsViewCocoa [0-9,]* [0-9x]*' | cut -d' ' -f2-)
  x=${child%%,*} y=${child#*,} y=${y%% *} width=${child#* } width=${width%x*}
  page_x=${shown%%,*} page_y=${shown#*,} page_y=${page_y%% *} page_width=${shown#* }
  page_width=${page_width%x*}
  off=$((2 * x + width - 2 * page_x - page_width))
  [ "$y" -eq "$page_y" ] && [ "$off" -ge -1 ] && [ "$off" -le 1 ] ||
    fail "$1 showed print preview at $child, over the page at $shown"
  # Its window is on screen before its page draws, which then takes escape once it has loaded.
  for _ in $(seq 50); do
    ../tools/window-screenshot.sh "$pid" "$out/$2" || fail "no window to capture"
    cmp -s "$out/before-print.png" "$out/$2" || break
    sleep 0.1
  done
  for _ in $(seq 50); do
    send key escape
    send layout
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q '^browser: layout child' || break
    sleep 0.1
  done
  sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q '^browser: layout child' &&
    fail "escape didn't close the print preview $1 showed"
  rm "${out:?}/before-print.png"
}
send click $page_point
../tools/window-screenshot.sh "$pid" "$out/before-print.png" || fail "no window to capture"
post rightclick $page_point
expect '^browser: menu tracked Back, ' "a right-click on the page didn't show the page's menu"
menu=$(grep '^browser: menu tracked' "$log" | tail -1 | cut -d' ' -f4-)
[ "$menu" = "Back, Forward, Reload, Save As…, Print…, View Page Source, Inspect" ] ||
  fail "the page's menu is $menu"
send choose Print…
print_preview "the page menu's Print…" print.png
# ⌘P, as File › Print, with the page focused.
send click $page_point
../tools/window-screenshot.sh "$pid" "$out/before-print.png" || fail "no window to capture"
page_shortcut p print
print_preview ⌘P print-key.png
# Switching or closing a full-screen page's tab ends its full screen, and the window's with it.
send click $page_point
send key f
expect '^browser: page entered full screen$' "the page didn't go full screen"
expect '^browser: window entered full screen$' "the window didn't go full screen with the page"
send key down cmd alt
expect '^browser: page left full screen$' "switching tabs didn't take the page out of full screen"
expect '^browser: window left full screen$' "the window stayed full screen as its page's tab left"
check_layout "after switching away from a full-screen page"
send key up cmd alt
expect "^browser: selected $opener in " "⌥⌘↑ didn't show the page again"
send key f
expect '^browser: window entered full screen$' "the page didn't go full screen again"
send key w cmd
expect '^browser: page left full screen$' "closing its tab didn't take the page out of full screen"
expect '^browser: window left full screen$' "the window didn't leave full screen as its page closed"
check_layout "after closing a full-screen page's tab"
exec 3>&-
quit
echo "ok: a page's popup opens as a tab in the window, under the page. The page's context menu leaves out what the window can't do, Open Link in New Tab opens a tab under the page, and Print and ⌘P show print preview centred over the page, at its top, which escape closes. Switching or closing a full-screen page's tab ends its full screen and the window's. Windows: $out/popup.png, $out/link-menu.png, $out/print.png, $out/print-key.png"

# Back, forward, reload and stop. stuck.test leads to a server that takes connections and never
# answers, so its page keeps loading until it's stopped.
python3 -I -c '
import socket, time
server = socket.socket()
server.bind(("127.0.0.1", 0))
server.listen()
print(server.getsockname()[1], flush=True)
time.sleep(600)' > "$out/port" &
server=$!
for _ in $(seq 50); do
  [ -s "$out/port" ] && break
  sleep 0.1
done
[ -s "$out/port" ] || fail "the server that never answers didn't start"
# The start page's favicon is a 16 × 16 orange square.
page='data:text/html,<title>loading</title>
<link rel="icon" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg==">
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/history.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" \
  --host-resolver-rules="MAP stuck.test 127.0.0.1:$(cat "$out/port")" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the history test page didn't draw"
tab=$(grep -m1 '^browser: today [0-9]' "$log" | cut -d' ' -f3)
await "^browser: page $tab back 0 forward 0 loading 0 data:" || fail "a fresh tab can go back or forward"
await "^browser: favicon $tab [1-9][0-9]* bytes\$" || fail "the start page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: history. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The header's buttons, centred 23.5 below the window's top.
back_button="138 24"
forward_button="172 24"
reload_button="206 24"

# Waits for the tab's page to be at https://$1/, done loading, with back and forward as $2 and $3
# (1 or 0). $4 says when.
expect_page() {
  expect "^browser: page $tab back $2 forward $3 loading 0 https://$1/\$" "$4: the tab isn't at $1 with back $2 and forward $3: $(grep "^browser: page $tab" "$log" | tail -1)"
}
# Loads https://$1 in the tab from the command bar.
go_from_bar() {
  send key l cmd
  expect '^browser: command bar shown$' "⌘L didn't open the command bar"
  send type "$1"
  send key return
  expect '^browser: command bar hidden$' "going to $1 didn't close the command bar"
}
# The driver's key $1, with ⌘, runs command $2, which the UI hears from side $3. Chromium and GPUI
# name keys through the Mac's keyboard layout, where the keys for ⌘[ and ⌘] on a US one can be
# others (å and a dead key on a Danish one), so this drives ⌘← and ⌘→, the same on every layout.
history_key() {
  send key "$1" cmd
  expect "^browser: command $2 via $3\$" "⌘$1 with the $3 focused didn't run $2"
}

go_from_bar example.com
expect_page example.com 1 0 "after loading a page"
# example.com has no favicon, so the start page's goes, but only once example.com has loaded.
expect "^browser: favicon $tab 0 bytes\$" "the start page's favicon stayed on example.com"
dropped=$(grep -n "^browser: favicon $tab 0 bytes\$" "$log" | head -1 | cut -d: -f1)
loaded=$(grep -n "^browser: page $tab back 1 forward 0 loading 0 https://example.com/\$" "$log" | head -1 | cut -d: -f1)
[ "$dropped" -gt "$loaded" ] || fail "the start page's favicon went before example.com had loaded"
go_from_bar example.org
expect_page example.org 1 0 "after loading another"
../tools/window-screenshot.sh "$pid" "$out/history.png" || fail "no window to capture"
history_key left back page
expect_page example.com 1 1 "after ⌘←"
history_key right forward page
expect_page example.org 1 0 "after ⌘→"
send click $back_button
expect '^browser: command back via sidebar$' "a click on back didn't go back"
expect_page example.com 1 1 "after a click on back"
send click $forward_button
expect '^browser: command forward via sidebar$' "a click on forward didn't go forward"
expect_page example.org 1 0 "after a click on forward"
# The click left the keyboard with the UI.
history_key left back ui
expect_page example.com 1 1 "after ⌘← with the UI focused"
history_key right forward ui
expect_page example.org 1 0 "after ⌘→ with the UI focused"

history_key r reload ui
expect "^browser: loaded $tab https://example.org/\$" "⌘R didn't load the page again"
send key r cmd shift
expect '^browser: command force-reload via ui$' "⇧⌘R didn't run force-reload"
expect "^browser: loaded $tab https://example.org/\$" "⇧⌘R didn't load the page again"
send click $reload_button
expect '^browser: command reload via sidebar$' "a click on reload didn't reload"
expect "^browser: loaded $tab https://example.org/\$" "a click on reload didn't load the page again"

# While a page loads, the URL field shows where it's going and reload is stop.
send click $page_point
for stop in "key . cmd|page|⌘." "key escape|page|escape" "click $reload_button|sidebar|a click on stop"; do
  IFS='|' read -r action from what << EOF
$stop
EOF
  go_from_bar stuck.test
  # Chrome tries https first only for public hosts.
  expect "^browser: page $tab back 1 forward 0 loading 1 http://stuck.test/\$" "stuck.test didn't show as loading"
  if [ "$from" = sidebar ]; then
    ../tools/window-screenshot.sh "$pid" "$out/loading.png" || fail "no window to capture"
  fi
  send $action
  expect "^browser: command stop via $from\$" "$what while loading didn't run stop"
  expect_page example.org 1 0 "after $what stopped the load"
done
exec 3>&-
quit
kill "$server"
echo "ok: ⌘← ⌘→ and the header's buttons go back and forward, with the page or the UI focused; reload and force reload load the page again; ⌘., escape and the stop button stop a load; the row keeps a page's favicon until the next page has loaded without one. Windows: $out/history.png, $out/loading.png"

# Spaces. A new one is empty, and each keeps its tabs and the one it showed last.
page='data:text/html,<title>loading</title>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/spaces.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Spaces test page didn't draw"
# The Space the app last showed.
space() {
  grep '^browser: space [0-9]' "$log" | tail -1 | cut -d' ' -f3
}
first=$(space)
await "^browser: spaces $first\$" || fail "the app didn't start with one Space"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The switcher's dots are centred under the sidebar, 25 above the window's bottom.
send layout
content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
dots_y=$((${content#*x} - 25))

send menu New Space…
expect '^browser: command new-space via ui$' "New Space in the menu didn't run new-space"
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: space $second in " "the window didn't show the new Space"
expect_tabs " *" 0 "in the new Space"
send layout
sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' | grep -q WebContentsViewCocoa &&
  fail "the new Space shows the first Space's page"
# The form's field has the keyboard, which the page had.
in_browser AccessKitSubclassOfGPUIView "in the new Space's form"
../tools/window-screenshot.sh "$pid" "$out/space-form.png" || fail "no window to capture"
send type reading
send key space
send ime on
send type nihongo
send key space
[ "$marked" = 8,3 ] || fail "the form's field didn't take the conversion: marked $marked"
../tools/window-screenshot.sh "$pid" "$out/space-form-ime.png" || fail "no window to capture"
send key return
none_since '^browser: create space' || fail "return while composing named the Space"
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
send key return
expect "^browser: create space $second reading 日本語\$" "return didn't name the Space: $(grep '^browser: create space' "$log" | tail -1)"
expect "^browser: renamed space $second reading 日本語\$" "the core didn't rename the Space"
../tools/window-screenshot.sh "$pid" "$out/new-space.png" || fail "no window to capture"
send key t cmd
expect '^browser: command new-tab via ui$' "⌘T in the new Space didn't reach the UI"
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
b=$(today)
expect "^browser: selected $b\$" "the new Space's tab wasn't shown"
expect "^browser: loaded $b https://example.com/\$" "example.com didn't load in the new Space's tab"

# $1 is the driver's key or click, $2 the command it runs, $3 where the UI hears it from, and $4
# the Space it shows, with its tab $5.
show_space() {
  send $1
  expect "^browser: command $2 via $3\$" "$1 with the $3 focused didn't run $2"
  expect "^browser: space $4 in " "$1 with the $3 focused didn't show Space $4"
  # The menu shows the shortcut too, and a key the shell ran never reaches it.
  [ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: command ')" = 1 ] ||
    fail "$1 with the $3 focused ran more than one command"
  expect_tabs "$5" "$5" "after $1 showed Space $4"
}
send click $page_point
show_space "key 1 ctrl" select-space page "$first" "$a"
send click $page_point
show_space "key 2 ctrl" select-space page "$second" "$b"
send click $page_point
show_space "key left alt cmd" previous-space page "$first" "$a"
send click $page_point
show_space "key right alt cmd" next-space page "$second" "$b"
# Spaces don't wrap around.
send key right alt cmd
expect '^browser: command next-space via page$' "⌥⌘→ at the last Space didn't reach the UI"
send state
none_since '^browser: space [0-9]' || fail "⌥⌘→ at the last Space showed another"
send click $sidebar_point
show_space "key 1 ctrl" select-space ui "$first" "$a"
show_space "key right alt cmd" next-space ui "$second" "$b"
show_space "key left alt cmd" previous-space ui "$first" "$a"
show_space "click 131 $dots_y" select-space sidebar "$second" "$b"
show_space "click 97 $dots_y" select-space sidebar "$first" "$a"
../tools/window-screenshot.sh "$pid" "$out/spaces.png" || fail "no window to capture"

# Two fingers across the sidebar page between Spaces. A flick goes on to the next Space, a drag let
# go still goes to the Space nearest its place, and past the last Space the pages come back. Fingers
# along the list scroll it, and a wheel's tick across pages once. Held partway, the next Space's
# page shows beside the first's.
swipe_point="110 500"
post swipe $swipe_point -10 0 10 1500
await '^browser: swipe across$' || fail "two fingers across the sidebar didn't page"
../tools/window-screenshot.sh "$pid" "$out/swipe.png" || fail "no window to capture"
await "^browser: drive $step done" || fail "the driver didn't finish the swipe"
none_since '^browser: space [0-9]' || fail "a swipe let go short of halfway showed another Space"
# $1 is the driver's swipe, $2 the Space it shows and $3 that Space's tab.
swipe_to() {
  send swipe $swipe_point $1
  expect '^browser: command select-space via swipe$' "a swipe of $1 didn't page"
  expect "^browser: space $2 in " "a swipe of $1 didn't show Space $2"
  expect_tabs "$3" "$3" "after a swipe of $1 showed Space $2"
}
swipe_to "-12 0 10" "$second" "$b"
swipe_to "12 0 10" "$first" "$a"
swipe_to "-30 0 20 200" "$second" "$b"
send swipe $swipe_point -12 0 10
expect '^browser: swipe across$' "two fingers across the sidebar didn't page"
none_since '^browser: space [0-9]' || fail "a flick past the last Space showed another"
send swipe $swipe_point 1 -10 10
expect '^browser: swipe along$' "two fingers along the list didn't read as scrolling"
none_since '^browser: space [0-9]' || fail "two fingers along the list showed another Space"
send wheel $swipe_point 1
expect '^browser: command select-space via wheel$' "a wheel's tick across the sidebar didn't page"
expect "^browser: space $first in " "a wheel's tick across the sidebar didn't show the first Space"
expect_tabs "$a" "$a" "after a wheel's tick showed the first Space"

# A tab's menu moves it to another Space, and the window goes with the tab it shows.
expect "^browser: settled space $first\$" "the first Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space › reading 日本語, Close Tab, Close Tabs Below (disabled)$' "the tab's menu didn't offer the other Space"
send choose reading 日本語
expect '^browser: command move-to-space via tab menu$' "Move to Space didn't run"
expect "^browser: space $second\( in .*\)\{0,1\}\$" "Move to Space didn't show the tab's new Space"
expect_tabs "$a $b" "$a" "after Move to Space"
expect "^browser: settled space $second\$" "the second Space's page didn't settle"
post rightclick $(row 0)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space › Space 1, Close Tab, Close Tabs Below$' "the moved tab's menu didn't offer the first Space"
send choose Space 1
expect "^browser: space $first\( in .*\)\{0,1\}\$" "moving the tab back didn't show the first Space"
expect_tabs "$a" "$a" "after moving the tab back"
show_space "key 2 ctrl" select-space ui "$second" "$b"
show_space "key 1 ctrl" select-space ui "$first" "$a"

send menu New Space…
expect "^browser: spaces $first $second [0-9]*\$" "New Space didn't make a third Space"
third=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f5)
expect "^browser: space $third in " "the window didn't show the third Space"
send key return
none_since '^browser: create space' || fail "return with no name named the Space"
show_space "key 1 ctrl" select-space ui "$first" "$a"
send menu New Space…
expect "^browser: space $third in " "New Space didn't show the form already open"
# Clicks wait for the Space's page to slide into place.
expect "^browser: settled space $third\$" "the form's page didn't settle"
none_since '^browser: spaces ' || fail "New Space with a form open made another Space"
send state
in_browser AccessKitSubclassOfGPUIView "back in the third Space's form"
send click 114 $((${content#*x} - 74))
expect "^browser: cancel space $third\$" "a click on Cancel didn't cancel"
expect "^browser: space $first in " "Cancel didn't show the first Space again"
expect "^browser: spaces $first $second\$" "Cancel didn't delete the third Space"
expect_tabs "$a" "$a" "after Cancel"
expect "^browser: settled space $first\$" "the first Space's page didn't settle after Cancel"

# A right-click on the Space's name or its dot shows the Space's menu, as in Arc.
name_point="100 112"
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "a right-click on the Space's name didn't show its menu"
../tools/window-screenshot.sh "$pid" "$out/space-menu.png" || fail "no window to capture"
send choose Rename Space
expect "^browser: rename space $first\$" "Rename Space didn't open the name's field"
send state
in_browser AccessKitSubclassOfGPUIView "in the name's field"
send type work
send key return
expect "^browser: renamed space $first work\$" "return in the name's field didn't rename the Space"
send doubleclick $name_point
expect "^browser: rename space $first\$" "a double-click on the Space's name didn't open its field"
send type gone
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
none_since '^browser: renamed space' || fail "escape in the name's field renamed the Space"
# Deleting a Space with tabs asks first. Its tabs close with it, and ⇧⌘T reopens them here.
post rightclick 131 $dots_y
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "a right-click on a dot didn't show its Space's menu"
post choose Delete Space
expect '^browser: alert shown Delete “reading 日本語”?$' "Delete Space didn't ask first"
../tools/window-screenshot.sh "$pid" "$out/space-delete.png" || fail "no window to capture"
send choose Delete
expect "^browser: spaces $first\$" "Delete didn't delete the Space"
[ "$(today)" = "$a" ] || fail "deleting another Space left this one's Today as \"$(today)\""
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T didn't reach the UI"
expect "^browser: today [0-9]* $a\$" "⇧⌘T didn't reopen the deleted Space's tab here: Today is \"$(today)\""
expect "^browser: loaded $(today | cut -d' ' -f1) https://example.com/\$" "the reopened tab didn't load its page"
# The last Space can't go.
post rightclick $name_point
expect "^browser: menu shown Rename Space, $colors, Delete Space (disabled)\$" \
  "the last Space's menu offered Delete"
send choose Rename Space
send key escape
expect '^browser: rename cancelled$' "escape in the name's field didn't cancel"
exec 3>&-
quit
spaces_ms=$(sed -n 's/^browser: space [0-9]* in \([0-9.]*\) ms$/\1/p' "$log" | summary)
echo "ok: New Space makes an empty Space whose form takes the keyboard and Japanese input, and names it, and ⌘T opens a tab in it; ⌃1 ⌃2 ⌥⌘← ⌥⌘→ with the page or the UI focused, and the switcher's dots, show each Space with the tab it showed last; two fingers across the sidebar page to the Space a flick or the pages' place points at, never past the last, and a wheel's tick across pages once; a tab's menu moves it to another Space, which shows with it; Cancel deletes a new Space and goes back. A right-click on a Space's name or dot shows Rename, its colours and Delete; a double-click on the name renames it too, return keeps the name and escape cancels. Delete asks first for a Space with tabs, closes them, and ⇧⌘T reopens them in the Space shown; the last Space can't go. Windows: $out/swipe.png, $out/space-menu.png, $out/space-delete.png"
echo "  Space switch, key to the core's report: $spaces_ms. Windows: $out/space-form.png, $out/space-form-ime.png, $out/new-space.png, $out/spaces.png"

# The Spaces come back after a quit and after the window closes: their names, their tabs with each
# tab's history, the tab each showed and the Space the window showed. Only a tab shown loads, and
# one that doesn't shows its title and its favicon from the profile's history, which keeps none
# for data: pages, so the first page is served. A tab the user closed stays closed. The page's text
# field, along its bottom, puts what's in it in the title. "d" typed in the page downloads a file,
# "s" one that takes longer than the run to come, and "k" opens the cookie page, once a check, as
# later runs type in it too.
mkdir "$out/site"
printf '%s' '<title>loading</title><link rel="icon" href="icon.png">
<textarea id=text style="position:fixed;left:0;bottom:0;width:100%;height:60px"></textarea>
<a id=d download href="notes.txt"></a><a id=s download href="slow.bin"></a>
<a id=k href="clear.html"></a>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
text.oninput = () => document.title = "value " + text.value;
onkeydown = e => {
  const link = e.target == document.body && document.getElementById(e.key);
  if (link && !localStorage[e.key]) {
    localStorage[e.key] = 1;
    link.click();
  }
};
</script>' > "$out/site/index.html"
printf 'notes\n' > "$out/site/notes.txt"
# The cookie page's title shows the site's cookie and how many times the server has sent its
# script, which the cache keeps for an hour. "c" in it sets a cookie.
printf '%s' '<title>loading</title><script src="count.js"></script>
<script>
document.title = "cookie " + (document.cookie || "none") + " cache " + count;
onkeydown = e => {
  if (e.key == "c") {
    document.cookie = "c=1";
    document.title = "cookie set";
  }
};
</script>' > "$out/site/clear.html"
printf '%s' iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAAGUlEQVR42mN44GDwnxLMMGrAqAGjBgwXAwCb4E8fp7RFqwAAAABJRU5ErkJggg== |
  base64 -D > "$out/site/icon.png"
python3 -I -u -c '
import functools, http.server, sys, time

class Handler(http.server.SimpleHTTPRequestHandler):
    sent = 0

    def do_GET(self):
        if self.path == "/count.js":
            Handler.sent += 1
            body = b"var count = %d;" % Handler.sent
            self.send_response(200)
            self.send_header("Content-Type", "text/javascript")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "max-age=3600")
            self.end_headers()
            self.wfile.write(body)
            return
        if self.path != "/slow.bin":
            return super().do_GET()
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", "100000000")
        self.end_headers()
        try:
            while True:
                self.wfile.write(bytes(1000))
                time.sleep(0.1)
        except OSError:
            pass

handler = functools.partial(Handler, directory=sys.argv[1])
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
print("Serving HTTP on 127.0.0.1 port", server.server_address[1], "...")
server.serve_forever()
' "$out/site" > "$out/site.log" 2>&1 &
server=$!
for _ in $(seq 50); do
  grep -q '^Serving HTTP' "$out/site.log" && break
  sleep 0.1
done
port=$(sed -n 's/^Serving HTTP on .* port \([0-9]*\) .*/\1/p' "$out/site.log")
[ -n "$port" ] || fail "the page server didn't start"
site="http://127.0.0.1:$port/"
log="$out/restore.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the restore test page didn't draw"
first=$(space)
a=$(today)
tab=$a
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "the served page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restoring Spaces. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The page's downloads go in the check's folder, and ⇧⌘J from the page opens the Library on
# Downloads, which lists them. The slow one runs until the quit.
send type d
expect '^browser: download [0-9]* done notes.txt$' "the page's download didn't finish"
[ -f "$out/downloads/notes.txt" ] || fail "the download isn't in the check's folder"
send type s
expect '^browser: download [0-9]* running slow.bin$' "the page's slow download didn't start"
send key j cmd shift
expect '^browser: command view-downloads via page$' "⇧⌘J with the page focused didn't reach the UI"
expect '^browser: library section downloads$' "⇧⌘J didn't open the Library on Downloads"
expect '^browser: library shown$' "⇧⌘J didn't open the Library"
../tools/window-screenshot.sh "$pid" "$out/downloads.png" || fail "no window to capture"
send key j cmd shift
expect '^browser: library hidden$' "⇧⌘J again didn't close the Library"
# Tab a goes to two pages and back to the first, so it has pages to go forward to.
go_from_bar example.com
expect_page example.com 1 0 "after loading a page"
go_from_bar example.org
expect_page example.org 1 0 "after loading another"
history_key left back page
expect_page example.com 1 1 "after going back"
history_key left back page
expect "^browser: page $a back 0 forward 1 loading 0 $site\$" "tab a didn't go back to the first page"
open_from_bar example.net
c=$tab
open_from_bar example.org
send key w cmd
expect_tabs "$c $a" "$c" "after ⌘W closed the tab for example.org"
send menu New Space…
expect "^browser: spaces $first [0-9]*\$" "New Space didn't make a second Space"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
send type reading list
send key return
expect "^browser: create space $second reading list\$" "return didn't name the Space"
send key t cmd
expect '^browser: command bar shown$' "⌘T in the new Space didn't open the command bar"
send type example.com
send key return
expect '^browser: today [0-9][0-9]*$' "the tab for example.com didn't open in the new Space"
e=$(today)
expect "^browser: loaded $e https://example.com/\$" "example.com didn't load in the new Space's tab"
post rightclick 100 112
expect "^browser: menu shown Rename Space, $colors, Delete Space\$" \
  "the Space's menu didn't offer its colours"
send choose Pink
expect "^browser: colored space $second bf556a\$" "Pink didn't give the Space its colour"
../tools/window-screenshot.sh "$pid" "$out/space-color.png" || fail "no window to capture"
send key 1 ctrl
expect "^browser: space $first in " "⌃1 didn't show the first Space"
expect_tabs "$c $a" "$c" "back in the first Space"
send drag 226 400 298 400
expect '^browser: sidebar width 300$' "dragging the sidebar's edge didn't make it 300 wide"
# A double-click on a tab's row turns its title into a field, all selected. Escape leaves the
# title, and return keeps the name typed, which the tab shows over its page's title.
send doubleclick $(row 0)
expect "^browser: rename tab $c\$" \
  "a double-click on the tab's row didn't turn its title into a field"
send type nope
send key escape
expect '^browser: rename cancelled$' "escape didn't cancel the tab's rename"
none_since "^browser: title $c nope" || fail "escape renamed the tab"
send doubleclick $(row 0)
expect "^browser: rename tab $c\$" "a second double-click on the row didn't rename the tab"
send type news
../tools/window-screenshot.sh "$pid" "$out/rename-tab.png" || fail "no window to capture"
send key return
expect "^browser: title $c news\$" "return didn't name the tab"
# With a download running, ⌘Q asks first, in Chrome's alert. Continue Downloading keeps the browser
# running and shows the Library's Downloads, and Quit quits, cancelling it.
post key q cmd
expect '^browser: command quit via ' "⌘Q didn't reach the UI"
expect '^browser: modal shown Download is in progress, .*, Continue Downloading, Quit$' \
  "⌘Q with a download running didn't ask first"
send choose Continue Downloading
expect '^browser: command view-downloads via chrome$' \
  "Continue Downloading didn't show the downloads"
expect '^browser: library section downloads$' \
  "Continue Downloading didn't show the Library's Downloads"
../tools/window-screenshot.sh "$pid" "$out/quit-downloads.png" || fail "no window to capture"
post key q cmd
expect '^browser: modal shown Download is in progress, ' "⌘Q didn't ask again"
# Quit is the driver's last command: the app doesn't report it done.
post choose Quit
exec 3>&-
exited

# Launched again, the app shows the first Space with the tab for example.net, and tab a under it
# at the served page, unloaded.
log="$out/restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "the relaunch didn't bring back two Spaces"
first=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f3)
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
grep -q "^browser: added space $first Space 1\$" "$log" || fail "the first Space came back without its name"
grep -q "^browser: added space $second reading list\$" "$log" || fail "the second Space came back without its name"
grep -q "^browser: colored space $second bf556a\$" "$log" ||
  fail "the second Space came back without its colour"
await "^browser: space $first\$" || fail "the relaunch didn't show the first Space"
# Without the tab closed before the quit.
await '^browser: today [0-9]* [0-9]*$' || fail "the first Space didn't come back with its two tabs"
c=$(today | cut -d' ' -f1)
a=$(today | cut -d' ' -f2)
await "^browser: selected $c\$" || fail "the first Space didn't show the tab it showed last"
await "^browser: loaded $c https://example.net/\$" || fail "the tab shown didn't load"
await "^browser: title $c news\$" || fail "the tab came back without its name"
grep -q "^browser: title $c Example Domain\$" "$log" &&
  fail "the tab's page's title took its name's place"
await "^browser: page $a back 0 forward 1 loading 0 $site\$" ||
  fail "tab a didn't come back at its page with history: $(grep "^browser: page $a" "$log" | tail -1)"
await "^browser: title $a ready\$" || fail "tab a didn't come back with its title"
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "tab a didn't come back with its favicon"
grep -q "^browser: loaded $a " "$log" && fail "tab a loaded without being shown"
await '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' ||
  fail "the second Space's tab didn't come back at example.com"
e=$(grep -m1 '^browser: page [0-9]* back 0 forward 0 loading 0 https://example.com/$' "$log" | cut -d' ' -f3)
grep -q "^browser: loaded $e " "$log" && fail "the second Space's tab loaded without being shown"
../tools/window-screenshot.sh "$pid" "$out/restored.png" || fail "no window to capture"
await '^browser: archive 1 tabs$' || fail "the relaunch didn't bring back the archived tab"
await '^browser: download [0-9]* done notes.txt$' || fail "the relaunch didn't list the download"
await '^browser: download [0-9]* cancelled slow.bin$' ||
  fail "the download the quit cancelled didn't come back cancelled"
# Newest first, so the first is this run's.
s=$(grep -m1 '^browser: download [0-9]* cancelled slow.bin$' "$log" | cut -d' ' -f3)
check_layout "after the relaunch, with the sidebar it had" 300
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: restored history. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Rename Tab in its menu, with the name taken out, gives the tab back its page's title.
post rightclick $(row 0)
menu="Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space › reading list, Close Tab, Close Tabs Below"
expect "^browser: menu shown $menu\$" \
  "a right-click on the renamed tab's row didn't offer Rename Tab"
send choose Rename Tab
expect "^browser: rename tab $c\$" "Rename Tab didn't turn the tab's title into a field"
send key backspace
send key return
expect "^browser: title $c Example Domain\$" \
  "taking the name out didn't give the tab back its page's title"
send click $page_point
# The tab closed before the quit reopens from the Library at the top of Today, at its page. ⌘W
# archives it again.
send key l cmd shift
expect '^browser: command view-library via page$' "⇧⌘L with the page focused didn't reach the UI"
expect '^browser: library shown$' "⇧⌘L didn't open the Library"
send click 300 111
expect '^browser: restore archived [0-9][0-9]*$' "a click on the archived tab didn't restore it"
expect "^browser: today [0-9][0-9]* $c $a\$" \
  "the archived tab didn't come back at the top of Today: Today is \"$(today)\""
r=$(today | cut -d' ' -f1)
expect "^browser: selected $r\$" "the tab from the archive isn't shown"
expect "^browser: loaded $r https://example.org/\$" "the tab from the archive didn't load its page"
expect '^browser: archive 0 tabs$' "the tab reopened stayed in the archive"
expect '^browser: library hidden$' "reopening a tab didn't close the Library"
send click $page_point
send key w cmd
expect_tabs "$c $a" "$c" "after closing the tab from the archive"
# Remove from List in the cancelled download's menu takes it off the list, and Clear List in the
# finished one's takes off every download that isn't running and keeps their files.
send key j cmd shift
expect '^browser: library section downloads$' "⇧⌘J didn't open the Library on Downloads"
expect '^browser: library shown$' "⇧⌘J didn't open the Library after the relaunch"
listed=$(grep '^browser: downloads ' "$log" | tail -1 | cut -d' ' -f3)
post rightclick 300 111
expect '^browser: menu shown Remove from List, Clear List$' \
  "a right-click on the cancelled download's row didn't show its menu"
send choose Remove from List
expect "^browser: download $s Remove via download menu\$" "Remove from List didn't reach it"
expect "^browser: downloads $((listed - 1))\$" "Remove from List didn't take the download off"
post rightclick 300 111
expect '^browser: menu shown Open, Show in Finder, Remove from List, Clear List$' \
  "a right-click on the finished download's row didn't show its menu"
send choose Clear List
expect '^browser: command clear-downloads via download menu$' "Clear List didn't run"
expect '^browser: downloads 0$' "Clear List didn't empty the list"
[ -f "$out/downloads/notes.txt" ] || fail "Clear List deleted the download's file"
send key j cmd shift
expect '^browser: library hidden$' "⇧⌘J didn't close the Library"
switch "2 cmd" "$a"
expect "^browser: loaded $a $site\$" "tab a didn't load once shown"
tab=$a
history_key right forward page
expect_page example.com 1 1 "going forward in tab a's restored history"
send key 2 ctrl
expect "^browser: space $second in " "⌃2 didn't show the second Space"
expect_tabs "$e" "$e" "in the second Space"
expect "^browser: loaded $e https://example.com/\$" "the second Space's tab didn't load once shown"
# The window's close button, which ends the app too: the driver doesn't report it done.
echo "click 20 24" >&3
exec 3>&-
await '^browser: window closed$' || fail "a click on the close button didn't close the window"
exited

# Closing the window kept them too, and the second Space shows.
log="$out/restored-again.log"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch after closing the window didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "closing the window didn't keep both Spaces"
second=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
await "^browser: space $second\$" || fail "the launch after closing the window didn't show the second Space"
await '^browser: today [0-9]*$' || fail "the second Space didn't come back with its tab"
await "^browser: loaded $(today) https://example.com/\$" || fail "the second Space's tab didn't load"
await '^browser: page [0-9]* back 1 forward 1 loading 0 https://example.com/$' ||
  fail "tab a didn't keep the page it went forward to"
quit

# A SIGTERM ends the session at once, before the save the new tab scheduled, which the core writes
# as Chrome exits.
log="$out/session-ending.log"
BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: restored in ' || fail "the launch before the SIGTERM didn't restore the Spaces"
await "^browser: loaded [0-9]* $site\$" || fail "the page opened over the Spaces didn't load"
kill -TERM "$pid"
exited
[ "$(python3 -I -c 'import json, sys
print(*[len(space["today"]) for space in json.load(open(sys.argv[1]))["spaces"]])' "$saved")" = "2 2" ] ||
  fail "the tab opened before the SIGTERM wasn't kept"
restore_ms=$(sed -n 's/^browser: restored in \([0-9.]*\) ms$/\1/p' "$out/restored.log" "$out/restored-again.log" | tr '\n' ' ')

# Auto-archive, 8 s instead of 12 hours. Tab a, then b, opens over the Spaces, and a isn't seen
# after b shows. Past a's 8 s, the next launch archives a at once and b 8 s after c shows.
log="$out/archive-a.log"
rm -f "$saved"
BROWSER_URL="${site}?a" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await "^browser: loaded [0-9]* ${site}?a\$" || fail "tab a didn't load"
quit
log="$out/archive-b.log"
BROWSER_URL="${site}?b" "$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await "^browser: loaded [0-9]* ${site}?b\$" || fail "tab b didn't load over tab a"
seen=$(date +%s)
quit
wait=$((seen + 9 - $(date +%s)))
[ "$wait" -le 0 ] || sleep "$wait"
log="$out/archive-c.log"
start=$(date +%s)
BROWSER_URL="${site}?c" "$app/Contents/MacOS/Browser" --archive-tabs-after=8 < /dev/null \
  > "$log" 2>&1 &
pid=$!
await '^browser: archive 1 tabs$' || fail "tab a, unseen for 8 s across the quit, wasn't archived"
[ $(($(date +%s) - start)) -lt 6 ] || fail "tab a was archived 8 s after the launch, not at once"
await "^browser: loaded [0-9]* ${site}?c\$" || fail "tab c didn't load"
c=$(sed -n "s|^browser: loaded \([0-9]*\) ${site}?c\$|\1|p" "$log" | head -1)
await '^browser: archive 2 tabs$' || fail "tab b, unseen for 8 s after c showed, wasn't archived"
[ $(($(date +%s) - start)) -ge 8 ] || fail "tab b was archived before it was unseen for 8 s"
await "^browser: today $c\$" || fail "Today is \"$(today)\" after the auto-archive, not tab c alone"
quit
echo "ok: after ⌘Q and after closing the window, the Spaces come back with their names, colours, tabs, history and the tab each showed, in the Space shown last, with the sidebar's width; only tabs shown load, the others show their titles and favicons, a tab renamed with a double-click keeps its name over its page's title until Rename Tab takes it out, and a closed tab stays closed, in the archive, from which it reopens at its page. A page's downloads are listed in the Library's Downloads, which ⇧⌘J opens. With one running, ⌘Q asks first, Continue Downloading shows Downloads and Quit cancels it, and after the relaunch Remove from List and Clear List take them off and keep their files. A SIGTERM keeps a tab opened just before it. Today tabs no window shows go in the archive after the interval, counted across a quit, and the tab shown stays."
echo "  Restore, 3 tabs in 2 Spaces: ${restore_ms}ms. Window: $out/restored.png"

# Find in page, and zoom. The test page has three apples and one 日本語, and puts its zoom in its
# title as it changes.
page='data:text/html;charset=utf-8,<title>loading</title>
<p style="font:24px system-ui">apple banana apple cherry apple 日本語</p>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
const unzoomed = devicePixelRatio;
onresize = () => document.title = "zoom " + Math.round(100 * devicePixelRatio / unzoomed);
</script>'
log="$out/find.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the find test page didn't draw"
first=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: find in page. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The find bar, a window of GPUI's over the browser window, has the keyboard.
in_find_bar() {
  [ "$key" = GPUIPanel ] && [ "$focus" = GPUIView ] ||
    fail "$1: the keyboard is in $key, on $focus"
}
# Waits for tab $1's find to be $2, as the core reports it: "closed", or "open", the selected
# match, "of", how many there are and the text.
expect_find() {
  expect "^browser: find $1 $2\$" "$3: $(grep "^browser: find $1 " "$log" | tail -1)"
}
# The find bar is 300 points wide and 32 high, 20 from the window's top right corner. Sets $content
# to the window's width.
check_find_bar() {
  send layout
  child=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child' |
    sed 's/^browser: layout child //')
  content=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content' |
    sed 's/^browser: layout content \([0-9]*\)x.*/\1/')
  [ "$child" = "$((content - 320)),20 300x32" ] ||
    fail "$1: the find bar is at ${child:-nowhere} in a window $content wide"
}

send key f cmd
expect '^browser: command find via page$' "⌘F with the page focused didn't reach the UI"
expect '^browser: find bar shown$' "⌘F didn't open the find bar"
send state
in_find_bar "after ⌘F"
send type apple
expect_find "$first" "open 1 of 3 apple" "typing in the find bar didn't find in the page"
send key return
expect_find "$first" "open 2 of 3 apple" "return didn't go to the next match"
send key return shift
expect_find "$first" "open 1 of 3 apple" "shift-return didn't go back a match"
send key g cmd shift
expect_find "$first" "open 3 of 3 apple" "⇧⌘G in the find bar didn't go back, round to the last"
send key g cmd
expect_find "$first" "open 1 of 3 apple" "⌘G in the find bar didn't go on, round to the first"
check_find_bar "after ⌘F"
../tools/window-screenshot.sh "$pid" "$out/find.png" || fail "no window to capture"

# The bar's next and previous go through the matches. Arc's are 28 pt at 212 and 238 in the bar.
send click $((content - 320 + 252)) 36
expect_find "$first" "open 2 of 3 apple" \
  "a click on the find bar's next didn't go to the next match"
send click $((content - 320 + 226)) 36
expect_find "$first" "open 1 of 3 apple" "a click on the find bar's previous didn't go back a match"
# A text the page doesn't have shows NOT FOUND in place of the count and the arrows.
send type z
expect '^browser: find bar drew "applez" NOT FOUND$' \
  "a text the page doesn't have didn't show NOT FOUND"
../tools/window-screenshot.sh "$pid" "$out/find-not-found.png" || fail "no window to capture"
send key backspace
expect_find "$first" "open [1-3] of 3 apple" "the find didn't find the text again"

# A click on the page takes the keyboard and leaves the bar open, and ⌘G from the page goes on.
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the page under the find bar"
send key g cmd
expect '^browser: command find-next via page$' "⌘G with the page focused didn't reach the UI"
expect_find "$first" "open [1-3] of 3 apple" "⌘G with the page focused didn't find"
grep -q '^browser: find bar hidden$' "$log" && fail "a click on the page closed the find bar"

# ⌘F gives the open bar the keyboard. Japanese input composes in it, and it finds once committed.
send key f cmd
expect '^browser: find bar drew "apple" ' "⌘F with the bar open didn't keep its text"
send state
in_find_bar "after ⌘F with the find bar open"
send ime on
send type nihongo
[ "$marked" = 0,7 ] || fail "the find bar didn't take the composition: marked $marked"
send key space
[ "$marked" = 0,3 ] || fail "the find bar didn't take the conversion: marked $marked"
send key return
[ "$marked" = none ] || fail "return didn't commit the composition: marked $marked"
send ime off
expect_find "$first" "open 1 of 1 日本語" "the find bar didn't find the committed text"
grep -q '^browser: find [0-9]* open [0-9-]* of [0-9-]* n' "$log" &&
  fail "the find bar searched for the composition"

# A resize and full screen keep the bar by the window's top right corner.
send resize 1000 700
check_find_bar "after a resize"
send fullscreen
expect '^browser: window entered full screen$' "the window didn't go full screen"
check_find_bar "in full screen"
../tools/window-screenshot.sh "$pid" "$out/find-fullscreen.png" || fail "no window to capture"
send fullscreen
expect '^browser: window left full screen$' "the window didn't leave full screen"

# Each tab has its own find. A new tab's starts from the text found last, and switching tabs shows
# each one's in the bar, or none.
send key t cmd
expect '^browser: command bar shown$' "⌘T in the find bar didn't open the command bar"
send type example.com
send key return
expect '^browser: find bar hidden$' "the find bar stayed over a new tab"
expect '^browser: loaded [0-9]* https://example.com/$' "the new tab didn't load"
second=$(today | cut -d' ' -f1)
send key f cmd
expect_find "$second" "open 0 of 0 日本語" "a new tab's find didn't start from the text found last"
send type example
expect_find "$second" "open 1 of [1-9][0-9]* example" "the new tab's find didn't find"
send key 2 cmd
expect "^browser: selected $first" "⌘2 in the find bar didn't show the first tab"
expect '^browser: find bar drew "日本語" 1/1$' "the first tab didn't show its own find"
# Escape closes the tab's find and gives the keyboard back to its page.
send key escape
expect_find "$first" "closed" "escape didn't close the find"
expect '^browser: find bar hidden$' "escape didn't hide the find bar"
send state
in_browser RenderWidgetHostViewCocoa "after escape closed the find bar"
send key 1 cmd
expect '^browser: find bar shown$' "the second tab didn't show its find"
expect '^browser: find bar drew "example" 1/' "the second tab didn't show its own find"
send state
[ "$key" = BrowserCoreNSWindow ] || fail "switching to a tab with its find open moved the keyboard to $key"
send key 2 cmd
expect '^browser: find bar hidden$' "the first tab, its find closed, showed the find bar"
# ⌘F opens it again with the tab's last text, highlighting its match without selecting it.
send key f cmd
expect '^browser: find bar drew "日本語" 0/1$' "⌘F didn't open the find with the tab's last text"
# The bar's close button closes it too, and gives the page the keyboard.
check_find_bar "before its close button"
send click $((content - 320 + 284)) 36
expect_find "$first" "closed" "a click on the find bar's close button didn't close the find"
expect '^browser: find bar hidden$' "a click on the find bar's close button didn't hide it"
send state
in_browser RenderWidgetHostViewCocoa "after a click on the find bar's close button"
send key f cmd
expect '^browser: find bar shown$' "⌘F after the close button didn't open the find bar again"

# ⌘0 and ⌘- from the page and the View menu's Zoom In zoom it, for its site, which the profile
# keeps, so ⌘0 first. The HUD shows the zoom over the card's top right, 8 points under the find
# bar. A tab shown in its place hides it at once.
send click $page_point
send key 0 cmd
expect '^browser: command actual-size via page$' "⌘0 with the page focused didn't reach the UI"
expect "^browser: zoom $first 100%\$" "⌘0 didn't zoom the page to 100 %"
send key - cmd
expect '^browser: command zoom-out via page$' "⌘- with the page focused didn't reach the UI"
expect "^browser: zoom $first 90%\$" "⌘- didn't zoom the page out to 90 %"
expect '^browser: title [0-9]* zoom 90$' "⌘- didn't zoom the page out"
expect '^browser: zoom hud shown$' "⌘- didn't show the zoom HUD"
send layout
hud=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout child .* 80x32$' |
  sed 's/^browser: layout child //')
[ "$hud" = "$((content - 100)),60 80x32" ] ||
  fail "the zoom HUD is at ${hud:-nowhere} in a window $content wide, under the find bar"
../tools/window-screenshot.sh "$pid" "$out/zoom.png" || fail "no window to capture"
send menu "Zoom In"
expect '^browser: command zoom-in via ui$' "the View menu's Zoom In didn't reach the UI"
expect "^browser: zoom $first 100%\$" "the View menu's Zoom In didn't zoom the page in to 100 %"
send menu "Zoom In"
expect '^browser: title [0-9]* zoom 110$' "the View menu's Zoom In didn't zoom the page in"
send key 1 cmd
expect "^browser: selected $second" "⌘1 didn't show the second tab"
send state
awk -v state="browser: drive $step: state" '
  /^browser: zoom hud shown$/ { hidden = 0 }
  /^browser: zoom hud hidden$/ { hidden = 1 }
  $0 == state { exit !hidden }' "$log" ||
  fail "the zoom HUD stayed over the tab shown in place of the one zoomed"
send key 2 cmd
expect "^browser: selected $first" "⌘2 didn't show the first tab again"
send key 0 cmd
expect '^browser: title [0-9]* zoom 100$' "⌘0 didn't zoom the page back to 100 %"
expect '^browser: zoom hud hidden$' "the zoom HUD didn't go after a moment"

# Another page closes it.
send key l cmd
expect '^browser: command bar shown$' "⌘L in the find bar didn't open the command bar"
send type example.org
send key return
expect_find "$first" "closed" "going to another page didn't close the find"
expect '^browser: find bar hidden$' "going to another page didn't hide the find bar"
expect '^browser: loaded [0-9]* https://example.org/$' "the first tab didn't load example.org"

# DevTools dock in the card on the right. They fill the page's frame, and the page keeps the rest
# of it, from the frame's left, through a resize. Each tab has its own. Sets $views to the frames
# of the views shown, DevTools' first, once there are $1 of them. DevTools fill the frame at once,
# and the page moves aside once they've loaded, as in Arc, so with two it waits for that too.
await_views() {
  for _ in $(seq 50); do
    send layout
    layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout content')
    views=$(echo "$layout" | grep -o 'WebContentsViewCocoa [0-9,]* [0-9x]*' | cut -d' ' -f2-)
    [ "$(echo "$views" | wc -l)" -eq "$1" ] &&
      [ "$1" -eq 1 -o "$(echo "$views" | sed -n 1p)" != "$(echo "$views" | sed -n 2p)" ] &&
      return 0
  done
  fail "$2: the window shows $(echo "$views" | wc -l | tr -d ' ') page views, not $1"
}
# DevTools fill the card, which is 228 from the left and 10 from the other edges, and the page
# keeps its left part.
check_docked() {
  content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
  width=${content%x*}
  height=${content#*x}
  devtools=$(echo "$views" | sed -n 1p)
  docked=$(echo "$views" | sed -n 2p)
  docked_width=$(echo "$docked" | sed 's/.* \([0-9]*\)x.*/\1/')
  [ "$devtools" = "228,10 $((width - 238))x$((height - 20))" ] ||
    fail "$1: DevTools are at $devtools in a $content window"
  [ "$docked" = "228,10 ${docked_width}x$((height - 20))" ] &&
    [ "$docked_width" -gt 0 ] && [ "$docked_width" -lt $((width - 238)) ] ||
    fail "$1: the page is at $docked, beside DevTools at $devtools"
}
send click $page_point
send key i cmd alt
expect '^browser: command devtools via page$' "⌥⌘I with the page focused didn't reach the UI"
await_views 2 "after ⌥⌘I"
check_docked "after ⌥⌘I"
../tools/window-screenshot.sh "$pid" "$out/devtools.png" || fail "no window to capture"
send resize 1100 700
await_views 2 "after a resize"
check_docked "after a resize"
# ⌥⌘I again closes them, and the page has the keyboard.
send key i cmd alt
expect '^browser: command devtools via page$' "⌥⌘I again didn't reach the UI"
await_views 1 "after ⌥⌘I again"
send state
in_browser RenderWidgetHostViewCocoa "after ⌥⌘I closed DevTools"
send menu "JavaScript Console"
expect '^browser: command javascript-console via ui$' \
  "the View menu's JavaScript Console didn't reach the UI"
await_views 2 "after JavaScript Console"
send key 1 cmd
expect "^browser: selected $second" "⌘1 didn't show the second tab"
await_views 1 "on a tab without DevTools"
send key 2 cmd
expect "^browser: selected $first" "⌘2 didn't show the first tab again"
await_views 2 "back on the tab with DevTools"
check_docked "back on the tab with DevTools"
# View Source opens the page's source in a tab under it.
send menu "View Source"
expect '^browser: command view-source via ui$' "the View menu's View Source didn't reach the UI"
expect '^browser: page [0-9]* .* view-source:https://example.org/$' \
  "View Source didn't open the page's source"
source=$(grep '^browser: page [0-9]* .* view-source:' "$log" | tail -1 | cut -d' ' -f3)
expect "^browser: today $second $first $source\$" "the source's tab isn't under the page's"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't quit"
exited
echo "ok: ⌘F opens the find bar from the page with the keyboard, typing finds with \"n/m\", return, shift-return, ⌘G and ⇧⌘G and the bar's arrows go through the matches, from the bar and the page, a text the page doesn't have shows NOT FOUND, Japanese input finds once committed, the bar keeps its place through a resize and full screen, each tab keeps its own find, escape and the close button close it and another page closes it. ⌘0, ⌘- and the View menu zoom the page, and the HUD shows the zoom for a moment, under the find bar, and goes when another tab shows. ⌥⌘I and the View menu dock DevTools in the card beside the page, through a resize and per tab, ⌥⌘I closes them and the page gets the keyboard back, and View Source opens a tab under the page. Windows: $out/find.png, $out/find-not-found.png, $out/find-fullscreen.png, $out/zoom.png, $out/devtools.png"

# Pinned tabs, from the served page, whose favicon fades once its tab is unloaded.
log="$out/pinned.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the pinned tabs test page didn't draw"
a=$(today)
await "^browser: favicon $a [1-9][0-9]* bytes\$" || fail "the served page's favicon didn't reach the UI"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: pinned tabs. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The pinned tabs the app last showed, top to bottom.
pinned() {
  grep '^browser: pinned' "$log" | tail -1 | sed 's/^browser: pinned *//'
}
# Waits for the app to show the pinned tabs $1 over Today $2. $3 says what went wrong if it doesn't.
expect_lists() {
  expect "^browser: pinned $1\$" "$3: the pinned tabs are \"$(pinned)\", not \"$1\""
  expect "^browser: today $2\$" "$3: Today is \"$(today)\", not \"$2\""
}
# The centre of pinned row $1, counting from 0, under the Space's name, and of the ✕ it shows
# under the pointer.
pinned_row() {
  echo "100 $((90 + 44 + 41 * $1 + 20))"
}
pinned_close() {
  echo "202 $((90 + 44 + 41 * $1 + 20))"
}
# Its favicon's.
pinned_favicon() {
  echo "26 $((90 + 44 + 41 * $1 + 20))"
}
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
expect_tabs "$c $b $a" "$c" "after opening two tabs"
send key d cmd
expect '^browser: command toggle-pin via page$' "⌘D with the page focused didn't reach the UI"
expect_lists "$c" "$b $a" "after ⌘D from the page"
switch "3 cmd" "$a"
send click $sidebar_point
send key d cmd
expect '^browser: command toggle-pin via ui$' "⌘D with the UI focused didn't run"
expect_lists "$c $a" "$b" "after ⌘D from the UI"
switch "1 cmd" "$c"
switch "3 cmd" "$b"

# Dragging a row moves its tab, in the pinned tabs and between them and Today, without showing it.
# Today's top row is under the pinned rows, the divider and "+ New Tab".
today_top="100 $((90 + 44 + 41 * 2 + 15 + 41 + 20))"
send drag $(pinned_row 1) 100 150
expect "^browser: dragged $a to pinned 0\$" "a drag of the second pinned tab up didn't move it"
expect_lists "$a $c" "$b" "after dragging a pinned tab up"
none_since '^browser: selected' || fail "dragging a tab showed it"
# In steps, as the pointer goes: Today's tab comes away and the pinned rows make room for it.
send press $today_top
send dragto 100 270
send dragto 100 230
send dragto 100 200
../tools/window-screenshot.sh "$pid" "$out/drag.png" || fail "no window to capture"
none_since '^browser: dragged' || fail "a row dropped before the pointer let go"
send release 100 200
expect "^browser: dragged $b to pinned 1\$" "a drag of the Today tab up didn't pin it"
expect_lists "$a $b $c" " *" "after dragging the Today tab into the pinned tabs"
send drag $(pinned_row 1) 100 300
expect "^browser: dragged $b to today 0\$" "a drag of a pinned tab down didn't unpin it"
expect_lists "$a $c" "$b" "after dragging a pinned tab into Today"
send drag $(pinned_row 0) 100 200
expect_lists "$c $a" "$b" "after dragging the first pinned tab down"
none_since '^browser: selected' || fail "dragging a tab showed it"
# Escape puts a dragged row back.
send press $(pinned_row 0)
send dragto 100 250
send key escape
expect '^browser: command stop via ' "escape during a drag didn't reach the UI"
send release 100 250
none_since '^browser: dragged' || fail "a drag escape put back dropped its row"
none_since '^browser: selected' || fail "letting go of a drag escape put back showed its tab"
[ "$(pinned)" = "$c $a" ] || fail "a drag escape put back moved a tab: the pinned tabs are \"$(pinned)\""

# ⌘W on a pinned tab unloads it and shows the top Today tab, and showing it again loads it.
switch "2 cmd" "$a"
send key w cmd
expect "^browser: selected $b\$" "⌘W on a pinned tab didn't show the top Today tab"
expect "^browser: page $a unloaded 1\$" "⌘W on a pinned tab didn't unload it"
none_since "^browser: tab $a closed\$" || fail "⌘W closed a pinned tab"
[ "$(pinned)" = "$c $a" ] || fail "⌘W on a pinned tab took its row: the pinned tabs are \"$(pinned)\""
../tools/window-screenshot.sh "$pid" "$out/pinned.png" || fail "no window to capture"
send click $(pinned_row 1)
expect "^browser: selected $a in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $a $site\$" "the unloaded pinned tab didn't load once shown"
expect "^browser: page $a unloaded 0\$" "the pinned tab still read as unloaded once it loaded"

# The ✕ under the pointer unloads a pinned tab, and an unloaded one has none.
send move $(pinned_close 0)
send click $(pinned_close 0)
expect '^browser: command close-tab via sidebar$' "a click on a pinned tab's ✕ didn't run Close Tab"
expect "^browser: page $c unloaded 1\$" "the ✕ didn't unload the pinned tab"
none_since '^browser: selected' || fail "unloading a pinned tab not shown changed the tab shown"
send click $(pinned_close 0)
expect "^browser: selected $c in " "a click where an unloaded pinned tab's ✕ was didn't show the tab"
none_since '^browser: command close-tab' || fail "an unloaded pinned tab showed its ✕"
expect "^browser: loaded $c https://example.org/\$" "the pinned tab didn't load once shown"

# Without Today tabs, closing shows the nearest pinned tab still loaded, else no page.
switch "3 cmd" "$b"
send key w cmd
expect "^browser: tab $b closed\$" "⌘W didn't close the last Today tab"
expect_lists "$c $a" " *" "after closing the last Today tab"
expect "^browser: selected $a\$" "closing the last Today tab didn't show the lowest pinned tab still loaded"
send key w cmd
expect "^browser: selected $c\$" "⌘W on a pinned tab without Today tabs didn't show the other one loaded"
expect "^browser: page $a unloaded 1\$" "⌘W didn't unload the pinned tab"
send key w cmd
expect '^browser: selected 0$' "⌘W on the last pinned tab loaded didn't leave the window without a page"
expect "^browser: page $c unloaded 1\$" "⌘W didn't unload the last pinned tab loaded"
none_since '^browser: loaded' || fail "closing a pinned tab loaded another"
../tools/window-screenshot.sh "$pid" "$out/pinned-unloaded.png" || fail "no window to capture"

# ⌘D unpins a tab to the top of Today.
send click $(pinned_row 1)
expect "^browser: selected $a in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $a $site\$" "the unloaded pinned tab didn't load once shown"
send key d cmd
expect_lists "$c" "$a" "after ⌘D on a pinned tab"

# Away from its pinned URL, ⌘↩ goes back there, from the page and the UI.
send click $(pinned_row 0)
expect "^browser: selected $c in " "a click on the unloaded pinned tab didn't show it"
expect "^browser: loaded $c https://example.org/\$" "the unloaded pinned tab didn't load once shown"
# Away from it, then back with ⌘↩ from side $1, which a click gives the keyboard.
away_and_back() {
  go_from_bar example.com
  expect "^browser: page $c away 1\$" "a pinned tab at another page didn't read as away from its pinned URL"
  if [ "$1" = ui ]; then
    send click $sidebar_point
  else
    send click $page_point
  fi
  send key return cmd
  expect "^browser: command back-to-pinned-url via $1\$" "⌘↩ with the $1 focused didn't run Back to Pinned URL"
  expect "^browser: loaded $c https://example.org/\$" "⌘↩ with the $1 focused didn't load the pinned URL"
  expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
}
away_and_back page
away_and_back ui
go_from_bar example.com
expect "^browser: page $c away 1\$" "a pinned tab at another page didn't read as away from its pinned URL"
# The quit leaves Today's tab shown, so the pinned tab comes back unloaded.
switch "2 cmd" "$a"
# Its favicon under the pointer offers the way back.
send move $(pinned_favicon 0)
../tools/window-screenshot.sh "$pid" "$out/pinned-away.png" || fail "no window to capture"
# ⌘Q is the driver's last command: the app doesn't report it done.
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the pinned tab comes back pinned, at its page, unloaded, and away from its pinned
# URL. A click on its favicon goes back there and shows it.
log="$out/pinned-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: pinned [0-9][0-9]*$' || fail "the pinned tab didn't come back pinned"
await '^browser: today [0-9][0-9]*$' || fail "the Today tab didn't come back"
c=$(pinned)
a=$(today)
await "^browser: selected $a\$" || fail "the relaunch didn't show the tab shown last"
await "^browser: loaded $a $site\$" || fail "the tab shown didn't load"
await "^browser: page $c back 1 forward 0 loading 0 https://example.com/\$" ||
  fail "the pinned tab didn't come back at its page: $(grep "^browser: page $c" "$log" | tail -1)"
await "^browser: page $c unloaded 1\$" || fail "the pinned tab didn't come back unloaded"
await "^browser: page $c away 1\$" || fail "the pinned tab didn't come back away from its pinned URL"
grep -q "^browser: loaded $c " "$log" && fail "the pinned tab loaded without being shown"
../tools/window-screenshot.sh "$pid" "$out/pinned-restored.png" || fail "no window to capture"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored pinned tab's way back. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
send move $(pinned_favicon 0)
send click $(pinned_favicon 0)
expect '^browser: command back-to-pinned-url via sidebar$' "a click on an away pinned tab's favicon didn't run Back to Pinned URL"
expect "^browser: selected $c in " "a click on the pinned tab's favicon didn't show it"
expect "^browser: loaded $c https://example.org/\$" "a click on the pinned tab's favicon didn't load its pinned URL"
expect "^browser: page $c away 0\$" "the pinned tab still read as away at its pinned URL"
none_since "^browser: loaded $c https://example.com/" || fail "the unloaded tab loaded its old page on the way back"

# New Folder makes a folder under the pinned tabs with its name's field, which takes what's typed.
# The pinned tabs and folders the app last drew: each folder with + if it's open, else -, and
# what's in it in brackets.
outline() {
  grep '^browser: outline' "$log" | tail -1 | sed 's/^browser: outline *//'
}
# Waits for the rows moving since the last outline to come to rest, so a press lands on the row
# drawn there.
settled() {
  for _ in $(seq 300); do
    awk '/^browser: outline/ { s = 0 } /^browser: rows settled/ { s = 1 } END { exit !s }' \
      "$log" && return 0
    sleep 0.1
  done
  fail "the rows didn't come to rest"
}
send menu New Folder…
expect '^browser: command new-folder via ui$' "New Folder in the Tabs menu didn't run new-folder"
expect '^browser: rename folder [0-9]*$' "New Folder didn't give the folder's name field the keyboard"
f=$(grep '^browser: rename folder' "$log" | tail -1 | cut -d' ' -f4)
[ "$(outline)" = "$c $f+[]" ] || fail "New Folder didn't make an open folder under the pinned tab: $(outline)"
send type work
send key return
expect "^browser: renamed folder $f work\$" "the new folder didn't take the name typed"
# Dragged right of where it was, under the folder, a pinned tab goes in, and dragged left of it,
# out. A folder drags as a row does.
send press $(pinned_row 0)
send dragto 110 180
send dragto 120 200
send release 120 200
expect "^browser: dragged $c to folder $f 0\$" "a pinned tab dragged right under the folder didn't go in"
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged in isn't in the folder: $(outline)"
settled
send press $(pinned_row 1)
send dragto 90 197
send dragto 76 199
send release 76 199
expect "^browser: dragged $c to pinned 1\$" "a tab dragged left out of the folder didn't come out"
expect "^browser: outline $f+\\[\\] $c\$" "the tab dragged out is still in the folder: $(outline)"
settled
send drag $(pinned_row 0) 100 200
expect "^browser: dragged $f to pinned 1\$" "a drag of the folder down didn't move it"
expect "^browser: outline $c $f+\\[\\]\$" "the folder dragged down isn't under the tab: $(outline)"
settled
send press $(pinned_row 0)
send dragto 110 180
send dragto 120 200
send release 120 200
expect "^browser: outline $f+\\[$c\\]\$" "the tab dragged under the folder didn't go in: $(outline)"
settled
none_since '^browser: selected' || fail "dragging into and out of a folder showed a tab"
../tools/window-screenshot.sh "$pid" "$out/folder.png" || fail "no window to capture"
# A click closes the folder and another opens it.
send click $(pinned_row 0)
expect '^browser: command toggle-folder via sidebar$' "a click on the folder didn't run toggle-folder"
expect "^browser: outline $f-\\[$c\\]\$" "a click on the open folder didn't close it: $(outline)"
send click $(pinned_row 0)
expect "^browser: outline $f+\\[$c\\]\$" "a click on the closed folder didn't open it: $(outline)"
# The command bar finds commands by their names' letters in order, those that start with what's
# typed first, four at most, and one found by its letters alone after the search row. ↩ runs the
# command selected.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type clear
expect '^browser: command bar commands "clear": Clear Today, Clear Cache and Refresh, Clear Archive, Clear Cookies and Refresh$' "the command bar didn't find the four commands that start with \"clear\": $(grep '^browser: command bar commands' "$log" | tail -1)"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar again"
send type cllps
expect '^browser: command bar commands "cllps": Collapse All Folders$' "the command bar didn't find Collapse All Folders for \"cllps\""
send key down
expect '^browser: command bar selected Collapse All Folders$' "↓ didn't select Collapse All Folders, under the search row"
send key return
expect '^browser: command bar runs Collapse All Folders$' "↩ didn't run Collapse All Folders"
expect '^browser: command collapse-folders via command bar$' "Collapse All Folders didn't run collapse-folders"
expect "^browser: outline $f-\\[$c\\]\$" "Collapse All Folders didn't close the folder: $(outline)"
expect '^browser: new tab plate 0$' "running a command left New Tab's plate"
echo "key q cmd" >&3
exec 3>&-
await '^browser: command quit via ' || fail "⌘Q didn't reach the UI"
exited

# Launched again, the folder comes back closed with its tab. A double-click renames it, and Delete
# Folder in its menu leaves its tab where it was.
log="$out/folder-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Space"
await '^browser: outline [0-9]*-\[[0-9]*\]$' || fail "the folder didn't come back closed with its tab: $(outline)"
f=$(outline | cut -d- -f1)
c=$(outline | sed 's/.*\[\(.*\)\]/\1/')
# Ids are new after a launch.
await '^browser: today [0-9][0-9]*$' || fail "the Today tab didn't come back"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored folder. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The first click opens it.
send doubleclick $(pinned_row 0)
expect "^browser: rename folder $f\$" "a double-click on the folder didn't open its name field"
expect "^browser: outline $f+\\[$c\\]\$" "the double-click's first click didn't open the folder: $(outline)"
send type later
send key return
expect "^browser: renamed folder $f later\$" "the folder didn't take the name typed over its old one"
../tools/window-screenshot.sh "$pid" "$out/folder-restored.png" || fail "no window to capture"
post rightclick $(pinned_row 0)
expect '^browser: menu shown Rename Folder, Delete Folder$' "the folder's menu isn't Rename Folder, Delete Folder"
send choose Delete Folder
expect '^browser: command delete-folder via tab menu$' "Delete Folder didn't run delete-folder"
expect "^browser: outline $c\$" "Delete Folder didn't leave its tab in its place: $(outline)"
exec 3>&-
quit
echo "ok: ⌘D pins and unpins from the page and the UI, and ⌘1–⌘9 count pinned tabs first. Dragging a row moves its tab in the pinned tabs, pins it and unpins it, without showing it, and escape puts it back. ⌘W and ✕ unload a pinned tab and keep its row, showing the top Today tab, else a pinned tab still loaded, else no page, and an unloaded pinned tab loads once shown. Away from its pinned URL, ⌘↩ from the page and the UI, and a click on its favicon, go back there. Pinned tabs come back pinned, unloaded and at their page, still away. New Folder makes a folder its field names, a drag right and left takes a tab in and out of it, a folder drags as a row, a click closes and opens it, and the command bar finds Collapse All Folders, which closes it. It comes back closed, a double-click renames it, and Delete Folder leaves its tab in place. Windows: $out/drag.png, $out/pinned.png, $out/pinned-unloaded.png, $out/pinned-away.png, $out/pinned-restored.png, $out/folder.png, $out/folder-restored.png"

# Clear, from the served page and two more, with the served page pinned for it to leave.
log="$out/clear.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the Clear test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: Clear. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# Clear Cache and Refresh, from the View menu, and Clear Cookies and Refresh, from the command bar,
# delete the page's site's cache and cookies and load the page again.
send key k
expect "^browser: title $a cookie [^ ]* cache 1\$" "k in the served page didn't open the cookie page"
send key c
expect "^browser: title $a cookie set\$" "c in the cookie page didn't set a cookie"
send key r cmd
expect "^browser: title $a cookie c=1 cache 1\$" \
  "⌘R lost the cookie or fetched the cached script again: $(grep '^browser: title' "$log" | tail -1)"
send menu Clear Cache and Refresh
expect '^browser: command clear-cache-and-refresh via ui$' "Clear Cache and Refresh didn't run"
expect "^browser: title $a cookie c=1 cache 2\$" \
  "Clear Cache and Refresh didn't fetch the script again, or lost the cookie: $(grep '^browser: title' "$log" | tail -1)"
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar"
send type clear cookies
expect '^browser: command bar commands "clear cookies": Clear Cookies and Refresh$' \
  "the command bar didn't find Clear Cookies and Refresh"
send key return
expect '^browser: command clear-cookies-and-refresh via command bar$' \
  "Clear Cookies and Refresh didn't run from the command bar"
expect "^browser: title $a cookie none cache 2\$" \
  "Clear Cookies and Refresh didn't delete the cookie, or fetched the script again: $(grep '^browser: title' "$log" | tail -1)"
../tools/window-screenshot.sh "$pid" "$out/clear-site.png" || fail "no window to capture"
send key left cmd
expect "^browser: page $a back 0 forward 1 loading 0 $site\$" "⌘← didn't go back to the served page"
# Clear on the divider, which sits under the one pinned tab, the URL field's button, and the
# served page's text field, along the bottom of the 800 pt tall window.
clear_point="200 185"
copy_point="202 64"
field_point="600 760"
open_from_bar example.com
b=$tab
open_from_bar example.org
c=$tab
switch "3 cmd" "$a"
send key d cmd
expect_lists "$a" "$c $b" "after pinning the served page's tab"
switch "2 cmd" "$c"

# ⇧⌘C and ⌥⇧⌘C copy the tab's URL and a Markdown link to it, from the page and the UI. A driven
# run copies to a pasteboard of its own, never the clipboard.
send key c cmd shift
expect '^browser: command copy-url via page$' "⇧⌘C with the page focused didn't reach the UI"
expect '^browser: copied https://example.org/$' "⇧⌘C didn't copy the tab's URL"
send click $sidebar_point
send key c cmd shift alt
expect '^browser: command copy-markdown-link via ui$' "⌥⇧⌘C with the UI focused didn't run"
expect '^browser: copied \[Example Domain](https://example.org/)$' \
  "⌥⇧⌘C didn't copy a Markdown link to the tab"

# Clear and the URL field's button show only while the pointer is over the sidebar. The button
# copies the URL, and doesn't open the command bar as the rest of the field does.
send move $sidebar_point
expect '^browser: sidebar hovered 1$' "the sidebar didn't see the pointer come over it"
../tools/window-screenshot.sh "$pid" "$out/sidebar-hovered.png" || fail "no window to capture"
send click $copy_point
expect '^browser: command copy-url via sidebar$' \
  "a click on the URL field's button didn't run Copy URL"
expect '^browser: copied https://example.org/$' "the URL field's button didn't copy the tab's URL"
none_since '^browser: command edit-url' ||
  fail "a click on the URL field's button opened the command bar"
send move $page_point
expect '^browser: sidebar hovered 0$' "the sidebar didn't see the pointer leave"
../tools/window-screenshot.sh "$pid" "$out/sidebar-away.png" || fail "no window to capture"

# ⇧⌘K with the page focused closes the Today tabs but the one shown, here the middle of three,
# without showing or loading another.
open_from_bar example.net
x=$tab
switch "3 cmd" "$c"
send click $page_point
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: cleared 2$' "Clear didn't close the two tabs not shown"
expect "^browser: tab $b closed\$" "Clear didn't close the bottom tab"
expect "^browser: tab $x closed\$" "Clear didn't close the top tab"
expect_lists "$a" "$c" "after Clear"
none_since '^browser: selected' || fail "Clear showed another tab"
none_since '^browser: loaded' || fail "Clear loaded a page"
none_since '^browser: toast' || fail "Clear showed a toast"
../tools/window-screenshot.sh "$pid" "$out/clear.png" || fail "no window to capture"

# ⌘Z with the page focused puts them back in place, unloaded, and the tab shown stays.
send key z cmd
expect '^browser: command undo-clear via page$' "⌘Z with the page focused didn't undo Clear"
expect "^browser: today [0-9][0-9]* $c [0-9][0-9]*\$" \
  "Undo didn't put the tabs back around the one shown: Today is \"$(today)\""
b=$(today | cut -d' ' -f3)
expect "^browser: page $b back 0 forward 0 loading 0 https://example.com/\$" \
  "the bottom tab didn't come back at its page"
expect "^browser: page $b unloaded 1\$" "the tab Undo brought back came back loaded"
none_since '^browser: selected' || fail "Undo showed another tab"
none_since '^browser: loaded' || fail "Undo loaded a page"

# The divider's Clear, under the pointer.
send move $clear_point
expect '^browser: sidebar hovered 1$' "the sidebar didn't see the pointer come over Clear"
send click $clear_point
expect '^browser: command clear via sidebar$' "a click on Clear didn't run it"
expect '^browser: cleared 2$' "Clear on the divider didn't close the two tabs not shown"
expect_lists "$a" "$c" "after Clear on the divider"

# With a Clear to undo, ⌘Z in the page's text field undoes typing there.
switch "1 cmd" "$a"
send click $field_point
send type ab
expect '^browser: title [0-9]* value ab$' "typing didn't reach the served page's text field"
send key z cmd
expect '^browser: title [0-9]* value a$' "⌘Z in the page's text field didn't undo typing there"
none_since '^browser: command undo-clear' || fail "⌘Z in the page's text field undid Clear"

# ⇧⌘K with the UI focused and the pinned tab shown closes every Today tab, then finds none.
send click $sidebar_point
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K with the UI focused didn't run"
expect '^browser: cleared 1$' "⇧⌘K with the pinned tab shown didn't close the Today tab"
expect "^browser: tab $c closed\$" "⇧⌘K with the UI focused didn't close the Today tab"
expect_lists "$a" " *" "after Clear with the pinned tab shown"
send key k cmd shift
expect '^browser: command clear via ui$' "⇧⌘K without Today tabs didn't run"
none_since '^browser: cleared' || fail "Clear without Today tabs closed some"

# ⇧⌘T with the UI focused reopens the tab closed last and shows it.
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T with the UI focused didn't reach the UI"
expect "^browser: today [0-9][0-9]*\$" "⇧⌘T didn't reopen the tab Clear closed: Today is \"$(today)\""
d=$(today)
expect "^browser: selected $d\$" "⇧⌘T didn't show the tab it reopened"
expect "^browser: loaded $d https://example.org/\$" "the reopened tab didn't load its page"

# ⇧⌘T with the page focused puts a tab back where it was, with its history.
open_from_bar example.net
e=$tab
go_from_bar example.org
expect_page example.org 1 0 "after going on from example.net"
open_from_bar example.com
f=$tab
switch "3 cmd" "$e"
send click $page_point
send key w cmd
expect "^browser: tab $e closed\$" "⌘W didn't close the middle Today tab"
expect_lists "$a" "$f $d" "after closing the middle Today tab"
send click $page_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via page$' "⇧⌘T with the page focused didn't reach the UI"
expect_lists "$a" "$f [0-9][0-9]* $d" "after ⇧⌘T, the tab isn't back between the two it was between"
e=$(today | cut -d' ' -f2)
tab=$e
expect "^browser: selected $e\$" "⇧⌘T didn't show the tab it reopened"
expect_page example.org 1 0 "after ⇧⌘T"
../tools/window-screenshot.sh "$pid" "$out/reopen.png" || fail "no window to capture"

# After Clear, ⇧⌘T reopens the tab it closed last, the top one, and ⌘Z with the UI focused brings
# back only the other.
send click $page_point
send key k cmd shift
expect '^browser: command clear via page$' "⇧⌘K with the page focused didn't reach the UI"
expect '^browser: cleared 2$' "Clear didn't close the two tabs not shown"
expect_lists "$a" "$e" "after clearing around the tab shown"
send click $sidebar_point
send key t cmd shift
expect '^browser: command reopen-closed-tab via ui$' "⇧⌘T after Clear didn't reach the UI"
expect "^browser: today [0-9][0-9]* $e\$" \
  "⇧⌘T after Clear didn't reopen the top tab: Today is \"$(today)\""
t=$(today | cut -d' ' -f1)
send key z cmd
expect '^browser: command undo-clear via ui$' "⌘Z with the UI focused didn't run"
expect "^browser: today $t $e [0-9][0-9]*\$" \
  "Undo after ⇧⌘T didn't bring back just the bottom tab: Today is \"$(today)\""
none_since "^browser: selected" || fail "Undo after ⇧⌘T showed another tab"

# A right-click on a row shows its tab's menu, with the shortcuts. Duplicate Tab opens a copy under
# it, with its history, and shows it. Rows count from the one pinned tab.
d=$(today | cut -d' ' -f3)
t=$(today | cut -d' ' -f1)
post rightclick $(row 2)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on a Today row didn't show its tab's menu"
../tools/window-screenshot.sh "$pid" "$out/tab-menu.png" || fail "no window to capture"
send choose Duplicate Tab
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect_lists "$a" "$t $e [0-9][0-9]* $d" "after Duplicate Tab"
tab=$(today | cut -d' ' -f3)
expect "^browser: selected $tab\$" "Duplicate Tab didn't show the copy"
expect_page example.org 1 0 "the copy Duplicate Tab made"
# Close Tabs Below shows the tab right-clicked first, so none below it loads on the way out.
post rightclick $(row 1)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below$' "a right-click on the top Today row didn't show its tab's menu"
send choose Close Tabs Below
expect '^browser: command close-tabs-below via tab menu$' "Close Tabs Below didn't run"
expect_lists "$a" "$t" "after Close Tabs Below"
expect "^browser: tab $e closed\$" "Close Tabs Below didn't close the tab under the one right-clicked"
expect "^browser: tab $d closed\$" "Close Tabs Below didn't close the bottom tab"
expect "^browser: selected $t\$" "Close Tabs Below didn't show the tab right-clicked"
[ "$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -c '^browser: selected ')" = 1 ] ||
  fail "Close Tabs Below showed a tab on the way out"
# ⇧⌘T brings them back one at a time, each at its place.
send key t cmd shift
expect "^browser: today $t [0-9][0-9]*\$" "⇧⌘T after Close Tabs Below didn't reopen the tab under the one right-clicked: Today is \"$(today)\""
# A pinned tab's menu unpins it, and goes back to its pinned URL only when it's away from it.
post rightclick $(pinned_row 0)
expect '^browser: menu shown Unpin Tab, Add to Favorites, Back to Pinned URL (disabled), Rename Tab, Duplicate Tab, Move to Space (disabled), Close Tab$' "a right-click on a pinned row didn't show its tab's menu"
send choose Unpin Tab
expect '^browser: command toggle-pin via tab menu$' "Unpin Tab didn't run"
expect "^browser: today $a $t [0-9][0-9]*\$" "Unpin Tab didn't move the tab to the top of Today: Today is \"$(today)\""

# ⌘W archives the tab shown. ⇧⌘L opens the Library, which pushes the page right at its width, and
# a click on the archived tab reopens it at the top of Today with its history, shows it and closes
# the Library.
shown=$(grep '^browser: selected' "$log" | tail -1 | cut -d' ' -f3)
history=$(grep "^browser: page $shown back " "$log" | tail -1 | sed 's/^browser: page [0-9]* //')
url=${history##* }
send click $sidebar_point
send key w cmd
expect "^browser: tab $shown closed\$" "⌘W didn't close the tab shown"
expect '^browser: archive [1-9][0-9]* tabs$' "⌘W didn't archive the tab"
expect "^browser: today [0-9 ]*\$" "⌘W didn't take the tab out of Today"
left=$(today)
send layout
layout=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout')
content=$(echo "$layout" | sed 's/^browser: layout content \([0-9]*x[0-9]*\),.*/\1/')
docked=$(echo "$layout" | sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
width=${content%x*}
height=${content#*x}
send key l cmd shift
expect '^browser: command view-library via ui$' "⇧⌘L with the UI focused didn't reach the UI"
expect '^browser: library shown$' "⇧⌘L didn't open the Library"
send layout
view=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
[ "$view" = "417,10 ${docked#* }" ] ||
  fail "the Library didn't push the page to 417 at its width: it's at $view, docked at $docked"
../tools/window-screenshot.sh "$pid" "$out/library.png" || fail "no window to capture"
send click 300 111
expect '^browser: restore archived [0-9][0-9]*$' "a click on the archived tab didn't restore it"
expect "^browser: today [0-9][0-9]* $left\$" \
  "the archived tab didn't come back at the top of Today: Today is \"$(today)\""
r=$(today | cut -d' ' -f1)
expect "^browser: selected $r\$" "the tab from the archive isn't shown"
expect "^browser: loaded $r $url\$" "the tab from the archive didn't load its page"
expect "^browser: page $r $history\$" "the tab from the archive came back without its history"
expect '^browser: library hidden$' "reopening a tab didn't close the Library"
send layout
view=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
  sed -n 's/.*WebContentsViewCocoa \([0-9,]* [0-9x]*\).*/\1/p')
[ "$view" = "$docked" ] || fail "the page is at $view after the Library, not back at $docked"
# The Window menu's View Library opens it too, and the Archive menu's Clear Archive asks first,
# then empties it. The Library's back button closes it, and the footer's button opens it.
send menu View Library
expect '^browser: command view-library via ui$' "View Library didn't run"
expect '^browser: library shown$' "View Library didn't open the Library"
post menu Clear Archive
expect '^browser: alert shown Clear the archive?$' "Clear Archive didn't ask first"
send choose Clear Archive
expect '^browser: archive 0 tabs$' "Clear Archive didn't empty the archive"
../tools/window-screenshot.sh "$pid" "$out/library-empty.png" || fail "no window to capture"
send click 24 $((height - 24))
expect '^browser: command view-library via library$' "the Library's back button didn't run"
expect '^browser: library hidden$' "the back button didn't close the Library"
send click 24 $((height - 25))
expect '^browser: command view-library via sidebar$' "the footer's Library button didn't run"
expect '^browser: library shown$' "the footer's Library button didn't open the Library"
send key l cmd shift
expect '^browser: library hidden$' "⇧⌘L didn't close the Library"
exec 3>&-
quit
echo "ok: Clear Cache and Refresh from the View menu fetches the page's script again, keeping its cookie, and Clear Cookies and Refresh from the command bar deletes its cookie, keeping the cached script. ⇧⌘C and ⌥⇧⌘C from the page and the UI copy the tab's URL and a Markdown link to it. The sidebar shows Clear and the URL field's button only while the pointer is over it, and the button copies the URL. ⇧⌘K from the page and the UI, and Clear on the divider, close the Today tabs but the one shown, with no toast, and ⌘Z from the page and the UI brings them back in place, unloaded, but not from the page's text field, where it undoes typing. ⇧⌘T from the page and the UI reopens the tab closed last, where it was and with its history, and ⌘Z brings back only the tabs ⇧⌘T didn't. A right-click on a row shows its tab's menu: Duplicate Tab copies the tab with its history under it, Close Tabs Below closes the Today tabs under it without showing them, and Unpin Tab unpins. ⌘W archives a Today tab, and ⇧⌘L, View Library and the footer's button open the Library, which pushes the page right at its width; a click on an archived tab reopens it at the top of Today with its history and closes the Library, and Clear Archive asks, then empties it. Windows: $out/clear-site.png, $out/sidebar-hovered.png, $out/sidebar-away.png, $out/clear.png, $out/reopen.png, $out/tab-menu.png, $out/library.png, $out/library-empty.png"

# Split view, from the served page with example.org beside it.
log="$out/split.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the split view test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: split view. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The Space's splits the app last showed, each as its tabs joined by "+".
splits() {
  grep '^browser: splits' "$log" | tail -1 | sed 's/^browser: splits *//'
}
# Sets $pages to the frames of the pages shown, left to right, then the focused one's ring. Not in
# a subshell, where send's count of the commands sent would be lost.
read_pages() {
  send layout
  pages=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: layout' |
    grep -o '\(WebContentsViewCocoa\|BrowserCoreFocusRing\) [0-9,]* [0-9x]*' | tr '\n' ' ' |
    sed 's/ $//')
}
# Two panes share the page's frame, 10 apart, as Arc's do.
read_pages
whole=$pages
frame=${whole#WebContentsViewCocoa }
x=${frame%%,*}
y=${frame#*,}
y=${y%% *}
w=${frame#* }
h=${w#*x}
w=${w%x*}
half=$(((w + 10) / 2))
left="$x,$y $((half - 10))x$h"
right="$((x + half)),$y $((w - half))x$h"
# Add Split View narrows the page at once to make room on the right, where the command bar opens,
# and escape gives the page its frame back.
send menu Add Split View
expect '^browser: command add-split via ui$' "Add Split View didn't run"
expect '^browser: command bar shown$' "Add Split View didn't open the command bar"
read_pages
[ "$pages" = "WebContentsViewCocoa $left" ] ||
  fail "while a pane is added, the page isn't at $left but at $pages"
send key escape
expect '^browser: command bar hidden$' "escape didn't close the command bar"
read_pages
[ "$pages" = "$whole" ] || fail "escape didn't give the page its frame back: $pages"
# ⌃+ from the page adds one too, and a URL typed there opens in the new pane, focused.
send key + ctrl
expect '^browser: command add-split via page$' "⌃+ with the page focused didn't reach the UI"
expect '^browser: command bar shown$' "⌃+ didn't open the command bar"
send type example.org
send key return
expect "^browser: splits $a+[0-9][0-9]*\$" "example.org didn't open beside the page: \"$(splits)\""
b=$(splits | cut -d+ -f2)
expect "^browser: selected $b\$" "the new pane isn't focused"
expect "^browser: panes $a $b\$" "the window doesn't show both panes"
expect "^browser: loaded $b https://example.org/\$" "example.org didn't load in the new pane"
two="WebContentsViewCocoa $left WebContentsViewCocoa $right"
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $right" ] ||
  fail "the panes aren't side by side with the ring on the right one: $pages"
../tools/window-screenshot.sh "$pid" "$out/split.png" || fail "no window to capture"
# A click in a pane focuses it, its page taking the keyboard, and the ring goes with it.
send click $((x + 100)) 400
expect "^browser: selected $a\$" "a click in the left pane didn't focus it"
expect "^browser: panes $a $b\$" "focusing a pane changed the panes shown"
[ "$focus" = RenderWidgetHostViewCocoa ] || fail "the left pane's page doesn't have the keyboard"
read_pages
[ "$pages" = "$two BrowserCoreFocusRing $left" ] ||
  fail "the ring didn't go to the left pane: $pages"
../tools/window-screenshot.sh "$pid" "$out/split-left.png" || fail "no window to capture"
# In the sidebar the split is one row, which ⌘2 shows under a tab opened above it, with the pane
# focused last, and a click on its right segment shows that pane focused.
open_from_bar example.net
c=$tab
expect "^browser: today $c $a $b\$" "the new tab isn't above the split: Today is \"$(today)\""
send key 2 cmd
expect "^browser: selected $a\( in .*\)\{0,1\}\$" \
  "⌘2 didn't show the split with its pane focused last"
expect "^browser: panes $a $b\$" "⌘2 didn't show both panes"
send click 170 $(row 1 | cut -d' ' -f2)
expect "^browser: selected $b\( in .*\)\{0,1\}\$" \
  "a click on the split's right segment didn't focus it"
exec 3>&-
quit

# Launched again, the split comes back with both its pages, and the pane focused last focused.
log="$out/split-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the split"
await '^browser: splits [0-9][0-9]*+[0-9][0-9]*$' || fail "the split didn't come back"
a=$(splits | cut -d+ -f1)
b=$(splits | cut -d+ -f2)
await "^browser: selected $b\$" || fail "the relaunch didn't focus the pane focused last"
await "^browser: panes $a $b\$" || fail "the relaunch didn't show both panes"
await "^browser: loaded $a $site\$" || fail "the split's left page didn't load"
await "^browser: loaded $b https://example.org/\$" || fail "the split's right page didn't load"
send click $((x + w - 100)) 400
# Separate Page from Split View shows the focused pane alone.
send menu Separate Page from Split View
expect '^browser: splits $' "Separate Page from Split View didn't end the split"
expect "^browser: panes $b\$" "the separated page isn't shown alone"
read_pages
[ "$pages" = "$whole" ] || fail "the separated page doesn't fill the card: $pages"
# ⌥↩ in the command bar opens its URL in a pane beside the page shown, and ⌘W closes the focused
# pane, leaving the card to the page beside it.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.net
send key return alt
expect "^browser: splits $b+[0-9][0-9]*\$" \
  "⌥↩ didn't open example.net beside the page: \"$(splits)\""
d=$(splits | cut -d+ -f2)
expect "^browser: panes $b $d\$" "⌥↩ didn't show the page and its new pane"
expect "^browser: loaded $d https://example.net/\$" "example.net didn't load in the new pane"
send key w cmd
expect "^browser: tab $d closed\$" "⌘W didn't close the focused pane"
expect "^browser: panes $b\$" "⌘W on a pane didn't leave the page beside it alone"
expect '^browser: splits $' "the split outlived its pane"
read_pages
[ "$pages" = "$whole" ] || fail "the page left doesn't fill the card: $pages"
# The ✕ on a split's row closes all its panes, and with the left one focused the window shows the
# tab its Space picks after them.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
send type example.net
send key return alt
expect "^browser: splits $b+[0-9][0-9]*\$" "⌥↩ didn't open example.net beside the page again"
e=$(splits | cut -d+ -f2)
expect "^browser: loaded $e https://example.net/\$" "example.net didn't load in the new pane"
send click $((x + 100)) 400
expect "^browser: selected $b\( in .*\)\{0,1\}\$" "a click on the left pane didn't focus it"
send move 202 $(row 2 | cut -d' ' -f2)
send click 202 $(row 2 | cut -d' ' -f2)
expect '^browser: command close-tab via sidebar$' "a click on the split's ✕ didn't run Close Tab"
expect "^browser: tab $b closed\$" "the split's ✕ didn't close its left pane"
expect "^browser: tab $e closed\$" "the split's ✕ didn't close its right pane"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "closing the split didn't show the tab above it"
# A row dragged onto the page becomes a picture of a window, and a target slides in at each of the
# card's sides. Back over the sidebar it's a row again, and let go over the left target it joins
# the page shown on its left, focused.
target="$((x + 140)) $((y + h / 2))"
send press $(row 0)
send dragto 110 $(row 0 | cut -d' ' -f2)
send dragto $target
expect "^browser: drop targets for $a\$" "a row dragged onto the page didn't bring the targets"
expect '^browser: drop targets shown$' "the targets' window didn't show"
expect '^browser: drop target left$' "the left target isn't under the tab"
../tools/window-screenshot.sh "$pid" "$out/split-targets.png" || fail "no window to capture"
send dragto 100 300
expect '^browser: drop targets gone$' "the targets stayed with the tab back over the sidebar"
expect '^browser: drop targets hidden$' "the targets' window stayed"
send dragto $target
expect '^browser: drop target left$' "the left target isn't under the tab again"
send release $target
expect "^browser: splits $c+$a\$" "the tab let go on the left target didn't join the page: \"$(splits)\""
expect "^browser: selected $c\( in .*\)\{0,1\}\$" "the tab let go on a target isn't focused"
expect "^browser: panes $c $a\$" "the window doesn't show the tab let go left of the page"
exec 3>&-
quit
echo "ok: Add Split View narrows the page at once for a pane on the right, with the command bar over it, which escape takes back, and ⌃+ from the page does it too. A URL there opens in the pane, focused. The panes share the card 10 apart, the focused one ringed, and a click focuses a pane and gives its page the keyboard. The split is one row in the sidebar, which ⌘1–⌘9 count once and show with the pane focused last, and a click on a segment focuses that pane. The split comes back after a quit with both pages. Separate Page from Split View shows the page alone, ⌥↩ in the command bar opens a URL beside the page, ⌘W closes the focused pane, and the ✕ on a split's row closes all its panes. A row dragged onto the page brings a target to each of the card's sides, which go when it's back over the sidebar, and let go on the left one the tab joins the page on its left. Windows: $out/split.png, $out/split-left.png, $out/split-targets.png"

# Favorites, from the served page with example.org above it.
log="$out/favorites.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$site" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the favorites test page didn't draw"
a=$(today)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: favorites. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi
# The favorites the app last showed, left to right.
favorites() {
  grep '^browser: favorites' "$log" | tail -1 | sed 's/^browser: favorites *//'
}
# One favorite is one tile across the sidebar, 41 tall and 90 below the window's top, under the URL
# field, and its row of tiles takes the Space's rows 47 down.
tile="114 110"
tiled_row() {
  echo "100 $(($(row "$1" | cut -d' ' -f2) + 47))"
}
open_from_bar example.org
b=$tab
expect_tabs "$b $a" "$b" "after opening a tab"
# Add to Favorites in a Today tab's menu takes the tab out of Today into a tile, without showing it.
post rightclick $(row 1)
expect '^browser: menu shown Pin Tab, Add to Favorites, Rename Tab, Duplicate Tab, Move to Space (disabled), Close Tab, Close Tabs Below (disabled)$' \
  "a right-click on a Today row didn't offer Add to Favorites"
send choose Add to Favorites
expect '^browser: command add-to-favorites via tab menu$' "Add to Favorites didn't run"
expect "^browser: favorites $a\$" \
  "Add to Favorites didn't make the tab a favorite: \"$(favorites)\""
expect "^browser: today $b\$" "the favorite is still in Today: Today is \"$(today)\""
none_since '^browser: selected' || fail "Add to Favorites showed another tab"
# A click on the tile shows the favorite, and ⌘1–⌘9 count favorites first.
send click $tile
expect '^browser: command select via favorite$' "a click on the tile didn't run"
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
../tools/window-screenshot.sh "$pid" "$out/favorites.png" || fail "no window to capture"
send click $(tiled_row 0)
expect "^browser: selected $b\( in .*\)\{0,1\}\$" \
  "a click on the Today row under the tile didn't show its tab"
switch "1 cmd" "$a"
# Away from its URL, a double-click on the tile goes back there.
tab=$a
go_from_bar example.com
expect "^browser: page $a away 1\$" "the favorite at another page didn't read as away from its URL"
send doubleclick $tile
expect '^browser: command back-to-pinned-url via favorite$' \
  "a double-click on the tile didn't run Back to Favorite URL"
expect "^browser: loaded $a $site\$" "a double-click on the tile didn't load the favorite's URL"
expect "^browser: page $a away 0\$" "the favorite still read as away at its URL"
# ⌘W unloads a favorite and keeps its tile, showing the top Today tab, and ⌘1 loads it again.
send click $page_point
send key w cmd
expect "^browser: selected $b\$" "⌘W on a favorite didn't show the top Today tab"
expect "^browser: page $a unloaded 1\$" "⌘W on a favorite didn't unload it"
none_since "^browser: tab $a closed\$" || fail "⌘W closed a favorite"
[ "$(favorites)" = "$a" ] ||
  fail "⌘W on a favorite took its tile: the favorites are \"$(favorites)\""
../tools/window-screenshot.sh "$pid" "$out/favorite-unloaded.png" || fail "no window to capture"
switch "1 cmd" "$a"
expect "^browser: loaded $a $site\$" "the unloaded favorite didn't load once shown"
# The tile's menu. Duplicate Tab opens a copy at the top of Today and shows it.
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL (disabled), Duplicate Tab, Remove from Favorites, Close Tab$' \
  "a right-click on the tile didn't show the favorite's menu"
send choose Duplicate Tab
expect '^browser: command duplicate-tab via tab menu$' "Duplicate Tab didn't run"
expect "^browser: today [0-9][0-9]* $b\$" \
  "Duplicate Tab didn't open a copy at the top of Today: Today is \"$(today)\""
c=$(today | cut -d' ' -f1)
expect "^browser: selected $c\$" "Duplicate Tab didn't show the copy"
expect "^browser: loaded $c $site\$" "the copy didn't load the favorite's page"
# The quit leaves the favorite shown.
send click $tile
expect "^browser: selected $a\( in .*\)\{0,1\}\$" "a click on the tile didn't show the favorite"
exec 3>&-
quit

# Launched again, the favorite comes back shown, and Remove from Favorites in its menu puts it at
# the top of Today.
log="$out/favorites-restored.log"
step=0
BROWSER_DRIVE=1 "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the favorites"
await '^browser: favorites [0-9][0-9]*$' || fail "the favorite didn't come back"
a=$(favorites)
await '^browser: today [0-9][0-9]* [0-9][0-9]*$' || fail "the Today tabs didn't come back"
left=$(today)
await "^browser: selected $a\$" || fail "the relaunch didn't show the favorite shown last"
await "^browser: loaded $a $site\$" || fail "the favorite didn't load"
../tools/window-screenshot.sh "$pid" "$out/favorites-restored.png" || fail "no window to capture"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: the restored favorite's menu. Browser wasn't the active app, so the page ignored"
  echo "  the click."
  exit 0
fi
post rightclick $tile
expect '^browser: menu shown Back to Favorite URL (disabled), Duplicate Tab, Remove from Favorites, Close Tab$' \
  "a right-click on the restored tile didn't show the favorite's menu"
send choose Remove from Favorites
expect '^browser: command remove-from-favorites via tab menu$' "Remove from Favorites didn't run"
expect '^browser: favorites $' "Remove from Favorites left the tile"
expect "^browser: today $a $left\$" \
  "Remove from Favorites didn't put the tab at the top of Today: Today is \"$(today)\""
none_since '^browser: selected' || fail "Remove from Favorites showed another tab"
# Dragging moves tabs between the rows and the tiles. With no favorites, a Today row dragged over
# the Space's name is drawn as a tile there, and dropped it's the first favorite.
set -- $left
x=$1
y=$2
send press $(row 1)
send dragto 100 200
send dragto 114 112
../tools/window-screenshot.sh "$pid" "$out/favorite-drag.png" || fail "no window to capture"
none_since '^browser: dragged' ||
  fail "a row dragged onto the favorites dropped before the pointer let go"
send release 114 112
expect "^browser: dragged $x to favorites 0\$" \
  "a Today row dragged onto the favorites didn't go there"
expect "^browser: favorites $x\$" \
  "the row dropped on the favorites isn't one: the favorites are \"$(favorites)\""
expect "^browser: today $a $y\$" "the row dropped on the favorites is still in Today: \"$(today)\""
# Dropped right of the tile, a row is the second favorite, and the two share the tiles' row.
send drag $(tiled_row 0) 180 112
expect "^browser: dragged $a to favorites 1\$" "a row dragged right of the tile didn't go there"
expect "^browser: favorites $x $a\$" \
  "the second row dropped on the favorites isn't the second: the favorites are \"$(favorites)\""
# A tile dragged left of the other goes before it.
send drag 168 110 40 110
expect "^browser: dragged $a to favorites 0\$" \
  "a tile dragged left of the other didn't go before it"
expect "^browser: favorites $a $x\$" "the tiles didn't swap: the favorites are \"$(favorites)\""
# A tile dragged into the list goes there, unpinned in Today.
send drag 168 110 100 240
expect "^browser: dragged $x to today 0\$" "a tile dragged into Today didn't go there"
expect "^browser: today $x $y\$" "the tile dropped in Today isn't at its top: Today is \"$(today)\""
expect "^browser: favorites $a\$" "the tile dropped in Today is still a favorite"
none_since '^browser: selected' || fail "dragging a tile showed it"
# Escape puts a dragged tile back.
send press 114 110
send dragto 100 200
send dragto 100 300
send key escape
expect '^browser: command stop via ' "escape during a tile's drag didn't reach the UI"
send release 100 300
none_since '^browser: dragged' || fail "a tile's drag escape put back dropped it"
[ "$(favorites)" = "$a" ] || fail "a tile's drag escape put back moved it: \"$(favorites)\""
exec 3>&-
quit
kill "$server"
echo "ok: Add to Favorites in a tab's menu makes the tab a tile above the Space's rows, which a click shows and ⌘1–⌘9 count first. A double-click on the tile takes it back to its URL, and ⌘W unloads it and keeps the tile. Its menu duplicates it into Today, and after a quit the favorite comes back, shown, and Remove from Favorites puts it at the top of Today. Rows dragged onto the tiles become favorites where they're dropped, tiles reorder and go into Today, and Escape puts a dragged tile back. Windows: $out/favorites.png, $out/favorite-unloaded.png, $out/favorites-restored.png, $out/favorite-drag.png"
