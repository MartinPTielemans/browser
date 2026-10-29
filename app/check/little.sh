# Mini Windows, Arc's Little Arc. ⌥⌘N opens one in the Space the window shows, in front, under its
# bar, and ⌘L there opens the command bar, whose URL loads in it. An unused escape closes it. A link
# from another app opens in one, and the same link again brings that window back rather than
# opening another. ⌘O moves its page to its Space in the main window and closes it. Left in the
# background past the archive's time, here 3 s, it closes and its tab goes to the archive, marked
# as a Mini Window's. Its page goes full screen, as a video's button takes it, and back. Over the
# main window in full screen, ⌥⌘N's window opens in front of it. The Dock menu's Hide and Show All
# Mini Windows, the menu bar button and the macOS desktop it opens on are the Dock's, the menu
# bar's and Mission Control's, which the driver can't reach, so they aren't checked here.

serve
# An f takes the page full screen, as a video's full-screen button does, and an x leaves it.
printf '%s' '<title>ready</title>
<script>
addEventListener("keydown", e => {
  if (e.metaKey) return;
  if (e.key == "f") document.documentElement.requestFullscreen();
  if (e.key == "x" && document.fullscreenElement) document.exitFullscreen();
});
document.addEventListener("fullscreenchange", () => {
  document.title = `fullscreen ${document.fullscreenElement ? 1 : 0}`;
});
</script>' > "$out/site/little.html"
log="$out/little.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_LITTLE_ARCHIVE_AFTER=3 BROWSER_DRIVE=1 BROWSER_URL="$site" "$browser" < "$out/drive" \
  > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
main=$(grep -m1 '^browser: window opened ' "$log" | cut -d' ' -f4)
home=$(space)
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: little. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi

# The Mini Window that opened last, from its log line.
opened() { grep '^browser: little window opened ' "$log" | tail -1 | cut -d' ' -f5; }

# ⌥⌘N opens an empty one for the Space shown, in front, and ⌘L's URL loads in it.
send key n cmd alt
expect '^browser: command new-little-window via ' "⌥⌘N didn't run New Mini Window"
expect "^browser: little window opened [0-9]* profile [0-9]* home $home\$" \
  "⌥⌘N didn't open a Mini Window for Space $home"
little=$(opened)
expect "^browser: key window $little\$" "the Mini Window didn't come to the front"
send window 2
send key l cmd
expect '^browser: command bar shown$' "⌘L didn't open the command bar in the Mini Window"
send insert "${site}little.html"
send key return
expect "^browser: loaded [0-9]* ${site}little.html\$" "the command bar's URL didn't load in the Mini Window"
../tools/window-screenshot.sh "$pid" "$out/little.png" || fail "no window to capture"
# Its page goes full screen and back, as a video's does.
send click 450 340
send key f
expect '^browser: page entered full screen$' "the Mini Window's page didn't go full screen"
expect '^browser: title [0-9]* fullscreen 1$' "the Mini Window's page didn't say it's full screen"
send key x
expect '^browser: page left full screen$' "the Mini Window's page didn't leave full screen"
expect '^browser: window left full screen$' "the Mini Window didn't leave full screen with its page"
# Escape the page doesn't take closes it.
send key escape
expect '^browser: little window closed by escape$' "an unused escape didn't close the Mini Window"
expect "^browser: window closed $little\$" "the Mini Window didn't close"
send window 1

# A link from another app opens in a Mini Window for the Space the window shows.
link="${site}?little"
open -a "$app" "$link"
await "^browser: link from another app $link: little window [0-9]* in space $home\$" ||
  fail "the link from another app didn't open in a Mini Window: $(grep '^browser: link' "$log" | tail -1)"
little=$(opened)
await "^browser: loaded [0-9]* $link\$" || fail "the link didn't load in the Mini Window"
# Again, it brings that window back.
open -a "$app" "$link"
await "^browser: link from another app $link: little window $little again\$" ||
  fail "the same link again didn't bring its Mini Window back"
[ "$(grep -c '^browser: little window opened ' "$log")" = 2 ] ||
  fail "the same link again opened another Mini Window"

# ⌘O moves its page to the main window, in its Space, and closes it.
send window 2
send key o cmd
expect "^browser: little window $little put [0-9]* in window $main space $home\$" \
  "⌘O didn't move the page to the main window"
moved=$(grep "^browser: little window $little put " "$log" | tail -1 | cut -d' ' -f6)
expect "^browser: window closed $little\$" "⌘O didn't close the Mini Window"
expect "^browser: window $main shows $moved\$" "the main window doesn't show the page ⌘O moved"
send window 1

# One left in the background past the archive's time closes, its tab archived.
open -a "$app" "${site}?archived"
await "^browser: link from another app ${site}?archived: little window [0-9]* in space $home\$" ||
  fail "the second link didn't open in a Mini Window"
little=$(opened)
await "^browser: loaded [0-9]* ${site}?archived\$" || fail "the second link didn't load"
send window 1
await "^browser: little window $little archived after 3 s\$" ||
  fail "the Mini Window left in the background wasn't archived"
await "^browser: window closed $little\$" || fail "the archived Mini Window didn't close"
await '^browser: archive [1-9][0-9]* tabs$' || fail "the archive didn't get the Mini Window's tab"

# Over the main window in full screen, ⌥⌘N's window opens in front of it, on the same desktop.
send fullscreen
expect '^browser: window entered full screen$' "the main window didn't go full screen"
send key n cmd alt
expect "^browser: little window opened [0-9]* profile [0-9]* home [0-9]*\$" \
  "⌥⌘N over the full-screen window didn't open a Mini Window"
little=$(opened)
expect "^browser: key window $little\$" "the Mini Window over full screen didn't come to the front"
send order
expect '^browser: order window 2 window 1, key window 2$' \
  "the Mini Window isn't in front of the full-screen window: $(last order)"
../tools/window-screenshot.sh "$pid" "$out/little-over-full-screen.png" || fail "no window to capture"
send window 2
send key w cmd
expect "^browser: window closed $little\$" "⌘W didn't close the Mini Window over full screen"
send window 1
send fullscreen
expect '^browser: window left full screen$' "the main window didn't leave full screen"

exec 3>&-
quit
echo "ok: ⌥⌘N opens a Mini Window in front, ⌘L loads a URL in it, its page goes full screen and back, and an unused escape closes it; a link from another app opens in one and again brings it back; ⌘O moves its page to the main window; one left in the background is archived; over a full-screen window one opens in front. Windows: $out/little.png $out/little-over-full-screen.png"
