# A new user's first run on Windows, from the installer a release ships (BROWSER_INSTALLER, the
# ilo-windows-x64-setup.exe package.sh makes): it installs for the user alone, its Start menu
# shortcut opens the chrome.exe it put in %LOCALAPPDATA%\Programs\ilo, which plays the onboarding,
# ends it, opens a page, quits, and launched again brings back the window and its tab. Uninstalled,
# no program file is left. Windows only, and only where ilo isn't installed, so it never reaches a
# user's own copy; with a profile of the run's own, never the user's.

windows || { echo "skip: first-run. The installer is Windows'."; exit 0; }
installer=${BROWSER_INSTALLER:-}
[ -f "$installer" ] || fail "BROWSER_INSTALLER doesn't name the installer: '$installer'"
name=$(cat branding/name)
installed="$(cygpath -u "$LOCALAPPDATA")/Programs/$name"
shortcut="$(cygpath -u "$APPDATA")/Microsoft/Windows/Start Menu/Programs/$name.lnk"
uninstall_key="HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\app.ilo.browser_is1"
log="$out/first-run-install.log"
: > "$log"
[ -e "$installed" ] && fail "$name is installed here already, in $installed: the run installs its own"

# Inno Setup's switches start with /, which Git Bash would take for paths.
MSYS2_ARG_CONV_EXCL='*' "$installer" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART \
  "/LOG=$(cygpath -w "$out/first-run-setup.txt")" || fail "the installer failed with status $?"
[ -f "$installed/chrome.exe" ] || fail "the installer put no chrome.exe in $installed"
[ -f "$shortcut" ] || fail "the installer made no Start menu shortcut"
target=$(LNK="$(cygpath -w "$shortcut")" powershell.exe -NoProfile -NonInteractive -Command \
  '(New-Object -ComObject WScript.Shell).CreateShortcut($env:LNK).TargetPath' | tr -d '\r')
[ "$(cygpath -u "$target")" = "$installed/chrome.exe" ] ||
  fail "the Start menu shortcut opens $target, not the installed chrome.exe"

# From here the app is the installed one, on a profile of the run's own.
app=$installed
procs="$(cygpath -m "$app/chrome.exe")"
profile_dir="$PWD/$out/first-run-profile"
saved="$profile_dir/Spaces.json"
export BROWSER_PROFILE_DIR="$(cygpath -w "$profile_dir")"
serve
# Launches the installed app as a user does, with no page and no onboarding override, logging to
# $1.
launch_installed() {
  log="$out/$1.log"
  step=0
  env -u BROWSER_ONBOARDING BROWSER_DRIVE=1 "$browser" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
}
capture() {
  ../tools/window-screenshot.sh "$pid" "$out/first-run-$1.png" || fail "no window to capture"
}

# The first launch plays the onboarding: the intro goes on by itself, and Esc goes on from each step
# after it to the end, which leaves the command bar up. That skips the import and the default browser's step, so the run changes
# neither.
launch_installed first-run
await '^browser: onboarding started$' || fail "the first launch didn't start the onboarding"
await '^browser: onboarding step intro settled$' || fail "the onboarding's intro didn't draw"
capture onboarding
steps=$(grep -m1 '^browser: onboarding steps ' "$log" | cut -d' ' -f4-)
await '^browser: onboarding left intro via time$' || fail "the intro didn't go on by itself"
for each in $steps; do
  [ "$each" = intro ] && continue
  await "^browser: onboarding step $each settled\$" || fail "the onboarding's $each step didn't come"
  send key escape
  expect "^browser: onboarding left $each via escape\$" "Esc didn't leave the $each step"
done
expect '^browser: onboarding done$' "the onboarding didn't end after its last step"
expect '^browser: command bar shown$' "the onboarding didn't end with the command bar"
capture onboarded

# A page, from the command bar the onboarding left up.
send type "$site"
send key return
expect_late "^browser: loaded [0-9]* $site\$" "the page typed in the command bar didn't load"
tab=$(grep -m1 "^browser: loaded [0-9]* $site\$" "$log" | cut -d' ' -f3)
expect_tabs "$tab" "$tab" "after opening the page"
capture page
quit

# Launched again, it brings back the window, its Space and the tab, and plays no onboarding.
launch_installed first-run-relaunch
await '^browser: restored in ' || fail "the relaunch didn't bring the Spaces back"
await '^browser: window opened [0-9]' || fail "the relaunch opened no window"
await "^browser: today [0-9]*\$" || fail "the relaunch's Space came back without its tab"
await "^browser: loaded $(today) $site\$" || fail "the tab brought back didn't load $site"
grep -q '^browser: onboarding' "$log" && fail "the relaunch played the onboarding again"
capture relaunched
quit

# Uninstalled from the uninstaller in the user's Apps list, which copies itself out of the folder
# and removes it after the copy this starts has ended.
log="$out/first-run-uninstall.log"
: > "$log"
MSYS2_ARG_CONV_EXCL='*' "$installed/unins000.exe" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART ||
  fail "the uninstaller failed with status $?"
for _ in $(seq 120); do
  [ -e "$installed" ] || break
  sleep 0.5
done
if [ -e "$installed" ]; then
  find "$installed" >> "$log"
  fail "the uninstall left files in $installed"
fi
[ -e "$shortcut" ] && fail "the uninstall left the Start menu shortcut"
reg query "$uninstall_key" > /dev/null 2>&1 && fail "the uninstall left ilo in the Apps list"
echo "first-run: installed in $installed, onboarded, opened $site, relaunched with its tab, uninstalled"
