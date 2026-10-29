# What the runs share. check.sh sources it, then each run's file.

# $1, ilo's menu items as the Mac's menus write them, or a pattern of them, as the app writes them
# here (shell/menus.rs): off the Mac in sentence case, keeping each item's first word, Space,
# Spaces, Library, acronyms and names in “”. Items are split at ", " and " › ". Not for names users
# give elsewhere.
cased() {
  if [ "$(uname)" = Darwin ]; then
    printf '%s\n' "$1"
    return
  fi
  printf '%s\n' "$1" | awk '{
    first = 1
    quoted = 0
    n = split($0, words, / /)
    for (i = 1; i <= n; i++) {
      word = words[i]
      noun = word
      sub(/,$/, "", noun)
      sub(/…$/, "", noun)
      if (word ~ /^“/) quoted = 1
      keep = first || quoted || noun ~ /^[A-Z][A-Z]+$/ || noun == "Space" || noun == "Spaces" || noun == "Library"
      printf "%s%s", (i > 1 ? " " : ""), (keep ? word : tolower(word))
      if (word ~ /”/) quoted = 0
      first = word ~ /,$/ || word == "›"
    }
    printf "\n"
  }'
}

# A Space's theme in its menu: the theme picker, and once it has a colour, the way back to none.
colors=$(cased "Edit Theme Color…")
themed=$(cased "$colors, Remove Theme Color")
# Where the theme picker's swatches are, from its top left: 13 for its arrow, then a 29.5 pitch.
swatch_y=380
swatch_x() { echo "$1" | awk '{ print 13 + 60.5 + 29.5 * $1 }'; }
# Its profiles, in the one the check starts with, for a Space with tabs and an empty one.
profile=$(cased "Profile › Default (checked), Profile › New Profile (disabled)")
empty_profile=$(cased "Profile › Default (checked), Profile › New Profile")
# The product's name, which the app's menus and alerts use, and the dev build's id.
app_name=$(cat branding/name)
app_id=dev.browser.app
# The KeyboardEvent property a page reads ⌘ from, which off the Mac is pressed with Ctrl (keys.rs).
cmd_key=metaKey
{ windows || [ "$(uname)" = Linux ]; } && cmd_key=ctrlKey
# Points from the window's top-left corner. The sidebar is 228 wide.
page_point="600 400"
sidebar_point="100 400"

# After a failure, nothing the check started keeps running.
cleanup() {
  # The servers may be gone already, and under set -e a failed kill would end the trap here.
  [ -z "${server:-}" ] || kill "$server" 2>/dev/null || true
  [ -z "${collector:-}" ] || kill "$collector" 2>/dev/null || true
  [ -z "${stuck:-}" ] || kill "$stuck" 2>/dev/null || true
  [ -z "${helper:-}" ] || kill "$helper" 2>/dev/null || true
  # A run's temp dir, such as the updates run's with its key.
  [ -z "${scratch:-}" ] || rm -rf "$scratch"
  kill "$pid" 2>/dev/null || return 0
  for _ in $(seq 10); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -9 "$pid" 2>/dev/null
  pkill -9 -f "$procs" || true
}
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
    windows || [ "$(uname)" = Linux ] || sample "$pid" 2 -file "$out/hang.txt" > /dev/null 2>&1
    fail "the app didn't quit. Where it was: $out/hang.txt"
  fi
  wait "$pid" || fail "the app exited with status $?"
  for _ in $(seq 10); do
    pgrep -f "$procs" > /dev/null || break
    sleep 0.5
  done
  # It exits once no process is left to send it a crash.
  pgrep -f "$app/Contents/.*/chrome_crashpad_handler" > /dev/null &&
    fail "Crashpad's handler outlived the app"
  pgrep -f "$procs" > /dev/null && fail "Chromium helper processes were left running"
  return 0
}
# A quit Apple Event goes through -[NSApplication terminate:], the way macOS quits apps when the
# user logs out. On Windows taskkill without /F closes the app's windows, as the user would, and
# Chromium quits with its last one. On Linux SIGINT is that way: Chromium quits as from its menu,
# where SIGTERM ends the session at once, without the startup trace tabs.sh reads.
quit_app() {
  if windows; then
    taskkill //PID "$(cat "/proc/$pid/winpid")" > /dev/null
  elif [ "$(uname)" = Linux ]; then
    kill -INT "$pid"
  else
    osascript -e "tell application id \"$app_id\" to quit"
  fi
}
quit() {
  quit_app
  exited "$@"
}

# Sends a command to the app's driver (platform/mac/drive.mm, platform/windows/drive.cc,
# platform/linux/drive.rs) and waits
# until the app has handled it.
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
# A Today tab's Share as its menu logs it, with the comma before it: this Mac's share services, or
# with none its one Share… for the share sheet, dimmed for a page with no link, or nothing on Linux,
# which has neither. A check reads past them and never chooses one.
share='\(, Share…\( (disabled)\)\?\|, Share › [^,]*\(, Share › [^,]*\)*\)\?'
# Waits for a line matching $1 logged after the last command was sent.
expect() {
  for _ in $(seq 100); do
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    sleep 0.1
  done
  fail "$2"
}
# expect, looking every 10 ms, for a press that has to land within the first frames of a motion.
expect_soon() {
  for _ in $(seq 1000); do
    sed -n "/^browser: drive $step: /,\$p" "$log" | grep -q "$1" && return 0
    sleep 0.01
  done
  fail "$2"
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

# What the sidebar last logged of $1, "sidebar hovered" or "sidebar tools": 1 or shown, 0 or hidden,
# or nothing before its first change.
last() { grep "^browser: $1 " "$log" | tail -1 | sed 's/.* //'; }
# Records the window to $out/$1.mp4 for $2 seconds, for the parity rows whose target moves: it
# returns once frames are being written, so the motion driven next is in it, and recorded waits
# for the file.
record() {
  recording="$out/$1.mp4"
  ../tools/window-record.sh "$pid" "$2" "$recording" > "$out/$1-record.log" 2>&1 &
  recorder=$!
  for _ in $(seq 300); do
    grep -q '^recording$' "$out/$1-record.log" && return 0
    kill -0 "$recorder" 2> /dev/null ||
      fail "the window's recording didn't start: $(tail -1 "$out/$1-record.log")"
    sleep 0.1
  done
  fail "the window's recording didn't start in 30 s"
}
recorded() {
  wait "$recorder" || fail "the window's recording didn't finish: $(tail -1 "${recording%.mp4}-record.log")"
}
# The driver's clicks come with no move, and one in the sidebar tells it the pointer is there, so
# where the sidebar last saw the pointer depends on the steps before. This moves it to the page and
# waits until the sidebar has seen it go and its tools have faded out, whether or not this move was
# what changed them, so the next move over the sidebar shows them anew.
away() {
  send move $page_point
  for _ in $(seq 100); do
    [ "$(last 'sidebar hovered')" != 1 ] && [ "$(last 'sidebar tools')" != shown ] && return 0
    sleep 0.1
  done
  fail "the sidebar's tools stayed with the pointer over the page"
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
  grep '^browser: pinned' "$log" | grep -v '^browser: pinned extensions ' | tail -1 |
    sed 's/^browser: pinned *//'
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

# Forgets the sites the profile keeps for Safari's engine, the notes it closed and the listed hosts
# it turned off, which a run that turns them on leaves for the runs after. Only while the app isn't
# running, as it writes its preferences as it quits.
forget_engine_sites() {
  prefs="$profile_dir/Default/Preferences"
  [ -e "$prefs" ] || return 0
  python3 -I -c '
import json, sys
with open(sys.argv[1]) as f:
    prefs = json.load(f)
engine = prefs.get("browser_core", {}).get("system_engine", {})
for kind in ("sites", "closed_notes", "listed_off"):
    engine.pop(kind, None)
with open(sys.argv[1], "w") as f:
    json.dump(prefs, f)
' "$prefs" || fail "the profile's sites for Safari's engine couldn't be reset"
}

# Starts the check's web server, in site.sh, unless a run before has. Its pages are at $site.
serve() {
  [ -n "${site:-}" ] || . check/site.sh
}
# Waits for the check's server to log a request matching $1, as site.sh writes them.
served() {
  for _ in $(seq 100); do
    grep -q "$1" "$out/site.log" && return 0
    sleep 0.1
  done
  fail "$2"
}
# The colour at ($2, $3), as fractions of the width and height, in the capture $1: "r g b".
pixel() {
  if windows; then
    to_bmp "$1" "$out/pixel.bmp" || fail "PowerShell couldn't read $1"
  elif [ "$(uname)" = Linux ]; then
    magick "$1" BMP3:"$out/pixel.bmp" || fail "ImageMagick couldn't read $1"
  else
    sips -s format bmp "$1" --out "$out/pixel.bmp" > /dev/null || fail "sips couldn't read $1"
  fi
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
