#!/bin/sh
# Runs the app and checks what it does, in thirteen runs that launch it twenty-three times. Each
# run's file in check/ says what it checks. ./check.sh tabs spaces does only those runs, in that
# order, and ./check.sh all of them:
#   load keys tabs window popup history spaces restore find pinned clear split favorites
# Every run but load needs Browser to be the active app, and window and find take it full screen
# for a moment.
# It opens on the Mac's own screen, where there is one, so it doesn't cover what's playing on
# another; BROWSER_DISPLAY=main opens it on the main display.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
set -e
export BROWSER_DISPLAY="${BROWSER_DISPLAY:-builtin}"
cd "$(dirname "$0")"
runs="load keys tabs window popup history spaces restore find pinned clear split favorites"
for run in "$@"; do
  case " $runs " in
    *" $run "*) ;;
    *) echo "FAIL: there's no run $run. The runs: $runs" >&2; exit 1 ;;
  esac
done
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
# The find run docks them, so they start on the right. The restore run's download goes in the
# check's own folder, never the user's Downloads.
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
. check/lib.sh
trap cleanup EXIT
# The runs send the app's driver its commands through it.
mkfifo "$out/drive"
for run in ${*:-$runs}; do
  . "check/$run.sh"
done
