# What the runs share. check.sh sources it, then each run's file.

# A Space's colours in its menu, for a Space without one.
colors="Color › System Accent (checked), Color › Gray, Color › Red, Color › Orange, \
Color › Yellow, Color › Green, Color › Blue, Color › Pink, Color › Purple"
# Its profiles, in the one the check starts with, for a Space with tabs and an empty one.
profile="Profile › Default (checked), Profile › New Profile (disabled)"
empty_profile="Profile › Default (checked), Profile › New Profile"
# The product's name, which the app's menus and alerts use.
app_name=$(cat branding/name)
# Points from the window's top-left corner. The sidebar is 228 wide.
page_point="600 400"
sidebar_point="100 400"

# After a run, nothing it started keeps running. check.sh stops the check's server once every run
# is done.
cleanup() {
  # Under set -e a failed kill would end the trap here.
  [ -z "${stuck:-}" ] || kill "$stuck" 2>/dev/null || true
  [ -n "${pid:-}" ] && kill "$pid" 2>/dev/null || return 0
  for _ in $(seq 10); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -9 "$pid" 2>/dev/null
  pkill -9 -f "$app/Contents" || true
}
# Ends the run, saying what failed and on which of its lines, with the log's last lines and, while
# the app runs, a capture of its windows as they were, which takes the place of the captures that
# only CHECK_CAPTURES=1 takes.
fail() {
  # The run's line, as bash reports it: the first caller outside lib.sh.
  where=
  n=0
  while frame=$(caller $n 2>/dev/null); do
    case $frame in
      *" check/lib.sh") n=$((n + 1)) ;;
      *) where=" (${frame##* }:${frame%% *})" && break ;;
    esac
  done
  echo "FAIL: $1$where" >&2
  [ -z "${log:-}" ] || grep -v '^$' "$log" | tail -5 >&2
  [ -z "${pid:-}" ] || ! kill -0 "$pid" 2>/dev/null ||
    ../tools/window-screenshot.sh "$pid" "${log%.log}-failed.png" all 2>/dev/null || true
  exit 1
}
# Waits up to 30 s for a line matching $1 in the log of the current run. Every wait reads the log
# through check/logwait.c, which looks again each time the app writes to it, not on a timer.
await() {
  "$out/logwait" -p "${pid:-0}" 30000 "$log" "$1" > /dev/null && return 0
  [ $? -ne 2 ] || fail "the app exited"
  return 1
}
# The app logs "browser: loaded" when the main frame has loaded.
await_load() {
  await '^browser: load' || fail "the page didn't finish loading"
  grep -q '^browser: loaded' "$log" || fail "the page didn't load"
}
# Waits up to $1 seconds (10) for the app to exit and checks that no Chromium process outlives it.
exited() {
  for _ in $(seq $((${1:-10} * 10))); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.1
  done
  if kill -0 "$pid" 2>/dev/null; then
    sample "$pid" 2 -file "$out/hang.txt" > /dev/null 2>&1
    fail "the app didn't quit. Where it was: $out/hang.txt"
  fi
  wait "$pid" || fail "the app exited with status $?"
  for _ in $(seq 50); do
    pgrep -f "$app/Contents" > /dev/null || break
    sleep 0.1
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

# Sends a command to the app's driver (platform/mac/drive.mm) and waits until AppKit has handled it.
send() {
  step=$((step + 1))
  echo "$*" >&3
  done=$("$out/logwait" -f "browser: drive $step: " -p "${pid:-0}" 30000 "$log" \
    "^browser: drive $step\( done,\|: can't do that\$\)") || {
    [ $? -ne 2 ] || fail "the app exited"
    fail "the driver didn't finish \"$*\""
  }
  case $done in *": can't do that") fail "the driver can't do \"$*\"" ;; esac
  key=${done#*, key } key=${key%%,*}
  focus=${done#*, focus } focus=${focus%%,*}
  active=${done#*, active } active=${active%%,*}
  marked=${done#*, marked }
}
# Sends a command without waiting for it: a menu or an alert it opens keeps the driver's events
# until it closes.
post() {
  step=$((step + 1))
  echo "$*" >&3
}
# A Today tab's Share as its menu logs it: this Mac's share services, or dimmed with none. A check
# reads past them and never chooses one.
share='\(Share (disabled)\|Share › [^,]*\(, Share › [^,]*\)*\)'
# Waits up to 10 s for a line matching $1 logged after the last command was sent.
expect() {
  "$out/logwait" -f "browser: drive $step: " -p "${pid:-0}" 10000 "$log" "$1" > /dev/null ||
    fail "$2"
}
# expect, for a press that has to land within the first frames of a motion, which expect, waking
# as the line comes, now does too.
expect_soon() {
  expect "$@"
}
# Nothing logged since the last command was sent matches $1.
none_since() {
  ! sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1"
}
# How many detents the app has asked the trackpad for: one for each place a dragged row passes.
detents() {
  grep -c '^browser: detent$' "$log" || true
}
# ⌘Q, and return for Quit in the dialog that asks first, once it has the keyboard. Quit is the
# driver's last command: the app doesn't report it done.
quit_by_key() {
  post key q cmd
  expect "^browser: dialog shown Quit $app_name?\$" "⌘Q didn't ask before quitting"
  expect '^browser: dialog settled$' "the dialog asking to quit didn't settle"
  post key return
  exec 3>&-
}
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
# Switch times in milliseconds, one a line, as "median <m> ms, max <x> ms, <n> switches".
summary() {
  sort -n | awk '{ v[NR] = $1 } END { if (NR) printf "median %.1f ms, max %.1f ms, %d switches", v[int((NR + 1) / 2)], v[NR], NR }'
}

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
# Runs command $1, named $2, from the command bar, found by typing $3.
bar_command() {
  send key l cmd
  expect '^browser: command bar shown$' "⌘L didn't open the command bar"
  send type "$3"
  expect "^browser: command bar commands \"$3\": $2" "the command bar didn't find $2 first"
  send key return
  expect "^browser: command $1 via command bar\$" "$2 didn't run from the command bar"
}
# The driver's key $1, with ⌘, runs command $2, which the UI hears from side $3. Chromium and GPUI
# name keys through the Mac's keyboard layout, where the keys for ⌘[ and ⌘] on a US one can be
# others (å and a dead key on a Danish one), so this drives ⌘← and ⌘→, the same on every layout.
history_key() {
  send key "$1" cmd
  expect "^browser: command $2 via $3\$" "⌘$1 with the $3 focused didn't run $2"
}

# The Space the app last showed.
space() {
  grep '^browser: space [0-9]' "$log" | tail -1 | cut -d' ' -f3
}

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

# Starts the check's web server, in site.sh, unless it's running. check.sh starts it before the
# runs. Its pages are at $site.
serve() {
  [ -n "${site:-}" ] || . check/site.sh
}
# Waits for the check's server to log a request matching $1, as site.sh writes them.
served() {
  "$out/logwait" 10000 "$out/site.log" "$1" > /dev/null || fail "$2"
}
# Captures the app's main window, or with "all" every one of its windows, in $out/$1, as proof: a
# step compares it, or the run's ok line names it for a PR or the owner to look at.
proof() {
  ../tools/window-screenshot.sh "$pid" "$out/$1" ${2:+"$2"} || fail "no window to capture"
}
# A capture only for a look, which no step compares and no ok line names: CHECK_CAPTURES=1 takes
# it. A failing run captures its windows as it fails.
look() {
  [ "${CHECK_CAPTURES:-}" != 1 ] || proof "$@"
}
# The colour at ($2, $3), as fractions of the width and height, in the capture $1: "r g b".
pixel() {
  sips -s format bmp "$1" --out "$out/pixel.bmp" > /dev/null || fail "sips couldn't read $1"
  python3 -I - "$out/pixel.bmp" "$2" "$3" << 'PY'
import struct, sys
data = open(sys.argv[1], "rb").read()
offset, = struct.unpack_from("<I", data, 10)
width, height, _, bits = struct.unpack_from("<iiHH", data, 18)
x = int(float(sys.argv[2]) * width)
y = int(float(sys.argv[3]) * abs(height))
row = y if height < 0 else abs(height) - 1 - y
stride = (bits * width + 31) // 32 * 4
b, g, r = data[offset + row * stride + x * bits // 8:][:3]
print(r, g, b)
PY
}
# Whether the colour "r g b" is as dark as the card is over any Space's colour.
dark() {
  set -- $1
  [ "$1" -lt 64 ] && [ "$2" -lt 64 ] && [ "$3" -lt 64 ]
}
# Whether the colour "r g b" is lighter than "r2 g2 b2" by 8 or more in each channel.
brighter() {
  set -- $1 $2
  [ "$1" -ge $(($4 + 8)) ] && [ "$2" -ge $(($5 + 8)) ] && [ "$3" -ge $(($6 + 8)) ]
}
