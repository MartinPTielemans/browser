#!/bin/sh
# Runs the app and checks what it does.
# Each run's file in check/ says what it checks. ./check.sh tabs spaces does only those runs, in
# that order, and ./check.sh all of them:
#   load keys tabs window popup history spaces agents profiles restore relaunch find pinned clear
#   split favorites cards extensions adblock adblock-lists windows private capture passwords
#   links preview access trouble dialogs crash import quit onboarding
# The store run installs from Google's Web Store and adblock-live blocks Google's ad servers with
# uBlock Origin's own lists, so they need the network and run only when named: ./check.sh store.
# Updates and passkeys run only when named too, until they pass on the Mac.
# Every run but load and crash needs Browser to be the active app, and window, find and updates
# take it full screen for a moment. Updates and passkeys are the Mac's alone so far: updates makes
# build/Updates.app, a copy with Sparkle and a feed of its own (build.sh updates).
# In a check VM the check turns the Mac's appearance dark for the runs, which read the window's
# dark look, and puts back the one it had after.
# It opens on the Mac's own screen, where there is one, so it doesn't cover what's playing on
# another; BROWSER_DISPLAY=main opens it on the main display.
# Leaves window captures and logs in build/check/. Build first with ./build.sh.
# BROWSER_PROFILE_DIR gives the app and the runs another profile directory, such as a new one for
# each run, since one an earlier run left can lack what a run expects, such as its search engines.
# On Windows it runs under Git Bash, with python3 on the PATH, on the app the engine's chrome.exe
# runs from build/Browser. The runs need what the Windows driver (platform/windows/drive.cc) does
# so far: load, tabs and dialogs.
# On Linux it runs itself in a headless sway session of its own (tools/headless.sh --run), on its
# Xwayland, as the driver (platform/linux/drive.rs) can drive only X11 so far, and the app's
# windows and input then never reach the owner's session. Of the runs, load and history pass
# there so far; the others stop where they need what Linux hasn't yet, such as the held escape's
# rule (keys), ilo's menu over the command bar (tabs) and IME (spaces), which
# the driver says it can't do. Captures are the session's whole output.
set -e
export BROWSER_DISPLAY="${BROWSER_DISPLAY:-builtin}"
# Each run starts a new profile, which would play the onboarding first. Only its own run plays it.
export BROWSER_ONBOARDING=skip
cd "$(dirname "$0")"
if [ "$(uname)" = Linux ] && [ -z "${BROWSER_HEADLESS:-}" ]; then
  exec ../tools/headless.sh --x11 --run ./check.sh "$@"
fi
runs="load keys tabs window popup history spaces agents profiles restore relaunch find pinned clear"
runs="$runs split favorites cards extensions adblock adblock-lists windows private capture passwords"
runs="$runs links preview access trouble dialogs crash import quit onboarding"
named="store adblock-live updates passkeys"
for run in "$@"; do
  case " $runs $named " in
    *" $run "*) ;;
    *) echo "FAIL: there's no run $run. The runs: $runs $named" >&2; exit 1 ;;
  esac
done
out=build/check
windows() {
  case "$(uname)" in MINGW* | MSYS*) return 0 ;; esac
  return 1
}
# The app, what runs it, the pattern that finds its processes, Chromium's helpers too, and its dev
# profile.
if windows; then
  . check/git-bash.sh
  app="$PWD/build/Browser"
  browser=windows_browser
  procs="$(cygpath -m "$app/chrome.exe")"
  profile_dir="$(cygpath -u "$LOCALAPPDATA")/browser-app"
elif [ "$(uname)" = Linux ]; then
  app="$PWD/build/Browser"
  browser="$app/chrome"
  procs="$app/chrome"
  profile_dir="$HOME/.cache/browser-app"
else
  app="$PWD/build/Browser.app"
  browser="$app/Contents/MacOS/Browser"
  procs="$app/Contents"
  profile_dir="$HOME/Library/Caches/browser-app"
fi
if [ -n "${BROWSER_PROFILE_DIR:-}" ]; then
  profile_dir="$BROWSER_PROFILE_DIR"
  ! windows || profile_dir="$(cygpath -u "$profile_dir")"
fi
# Another Browser holds the profile directory, so this one couldn't start.
if pgrep -f "$procs" > /dev/null; then
  echo "FAIL: Browser is already running. Quit it first." >&2
  exit 1
fi
rm -rf "$out"
mkdir -p "$out"
# The Spaces the app saves for its next launch. Each run starts without them, but the relaunches.
saved="$profile_dir/Spaces.json"
# The frame the launch window opens at, which the app keeps next to them. A run that starts without
# the Spaces starts without it too, so its launch window isn't at the last run's frame.
launch_frame="$(dirname "$saved")/launch-frame"
# DevTools open where the profile last had them, so an undocked try would leave them in a window.
# The find run docks them, so they start on the right. The restore run's download goes in the
# check's own folder, never the user's Downloads. A failed extensions run may have left one of its
# extensions pinned. The profile keeps the clear run's answers to its sites' questions, and how
# often it dismissed them, for sites whose ports a later run may get, and a failed clear run the
# site it kept for Safari's engine, whose pages would then open there, and the clear run the site
# whose protected-video note it closed, which would never show it again, and a failed one the
# listed host it turned off.
prefs="$profile_dir/Default/Preferences"
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
engine = prefs.get("browser_core", {}).get("system_engine", {})
engine.pop("sites", None)
engine.pop("closed_notes", None)
engine.pop("listed_off", None)
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
# The window follows the Mac's appearance and the runs read its dark look, so a check VM runs dark,
# and cleanup puts back the appearance it had. The owner's Mac keeps its own.
if [ "$(uname)" = Darwin ] && [ "$(sysctl -n kern.hv_vmm_present)" = 1 ]; then
  dark_mode=$(osascript -e 'tell application "System Events" to tell appearance preferences to get dark mode') || {
    echo "FAIL: System Events didn't say the Mac's appearance: allow the check to control it" >&2
    exit 1
  }
  appearance true
fi
# The runs send the app's driver its commands through it.
mkfifo "$out/drive"
for run in ${*:-$runs}; do
  . "check/$run.sh"
done
