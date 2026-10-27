#!/bin/sh
# Runs the app and checks what it does, in twenty runs that launch it thirty-five times. Each run's
# file in check/ says what it checks. ./check.sh tabs spaces does only those runs, in that order,
# and ./check.sh all of them:
#   load keys tabs window popup history spaces agents profiles restore find pinned clear split
#   favorites extensions adblock windows private capture
# The store run installs from Google's Web Store and adblock-live blocks Google's ad servers with
# uBlock Origin's own lists, so they need the network and run only when named: ./check.sh store.
# Every run but load needs Browser to be the active app, and window and find take it full screen
# for a moment.
# It opens on the Mac's own screen, where there is one, so it doesn't cover what's playing on
# another; BROWSER_DISPLAY=main opens it on the main display.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
set -e
export BROWSER_DISPLAY="${BROWSER_DISPLAY:-builtin}"
cd "$(dirname "$0")"
runs="load keys tabs window popup history spaces agents profiles restore find pinned clear split"
runs="$runs favorites extensions adblock windows private capture"
named="store adblock-live"
for run in "$@"; do
  case " $runs $named " in
    *" $run "*) ;;
    *) echo "FAIL: there's no run $run. The runs: $runs $named" >&2; exit 1 ;;
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
# check's own folder, never the user's Downloads. A failed extensions run may have left one of its
# extensions pinned. The profile keeps the clear run's answers to its sites' questions, and how
# often it dismissed them, for sites whose ports a later run may get.
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
prefs.setdefault("extensions", {})["pinned_extensions"] = []
exceptions = prefs.get("profile", {}).get("content_settings", {}).get("exceptions", {})
for kind in ("window_placement", "permission_autoblocking_data"):
    exceptions.pop(kind, None)
with open(sys.argv[1], "w") as f:
    json.dump(prefs, f)
' "$prefs" "$PWD/$out/downloads"
# Each run starts in the one profile, Default: Chrome deletes the others a failed run left at its
# next launch.
python3 -I -c '
import json, os, sys
if not os.path.exists(sys.argv[1]):
    sys.exit()
with open(sys.argv[1]) as f:
    local = json.load(f)
profile = local.setdefault("profile", {})
left = [name for name in profile.get("info_cache", {}) if name != "Default"]
for name in left:
    del profile["info_cache"][name]
profile["last_used"] = "Default"
profile["last_active_profiles"] = profile["profiles_order"] = ["Default"]
deleted = local.setdefault("profiles", {}).setdefault("profile_basenames_deleted", [])
deleted.extend(name for name in left if name not in deleted)
with open(sys.argv[1], "w") as f:
    json.dump(local, f)
' "$(dirname "$saved")/Local State"
. check/lib.sh
trap cleanup EXIT
# The runs send the app's driver its commands through it.
mkfifo "$out/drive"
for run in ${*:-$runs}; do
  . "check/$run.sh"
done
