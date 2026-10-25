# A page's popup opens as a tab in the window, under the page. The page's context menu is Chrome's
# without the items the window can't do, its Open Link in New Tab opens a tab under the page without
# showing it, and its Print and ⌘P from the page show print preview centred over the page, at its
# top, which escape closes. Switching or closing a full-screen page's tab ends its full screen and
# the window's.

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
