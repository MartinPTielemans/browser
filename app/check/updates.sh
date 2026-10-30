# The Mac's updates (docs/ux-spec.md › Updates), on a check build with Sparkle (build.sh updates)
# that checks a feed the run serves, for updates signed with a key the run makes and deletes after.
# Neither is ever the product's: the run never reads branding/update-feed or the owner's key.
# The app menu has Check for Updates… under About. Sparkle checks every 8 hours, from the last check.
# Check for Updates… finds the feed's update and shows it with its Markdown notes. A scheduled
# check that finds one while a window is full screen holds it, and shows it with the keyboard once
# the window leaves full screen.
# Not here: the release notes the first launch of an update opens, which only the prod build does,
# for https notes only, and the install, which needs a signed app to update to.
# On Linux, ilo's Flatpak, built as a release's (tools/flatpak.sh), installed from a repository of
# the run's own, where a newer build then comes, as a release puts one in ilo's. Flatpak's own
# portal, on a session bus of the run's own and looking every 10 s rather than every half hour,
# announces it, ilo installs it beside the build running and offers to restart into it, and
# Restart starts the new build once this one has quit. Flatpak asks the user once whether an app
# may update itself; the run says yes for them, in a home of its own, which has the profile too.
# It needs flatpak, flatpak-builder and ostree, and skips where ilo's Flatpak is installed from
# another repository, which it would replace.

updates_check() {
  if [ "$(uname)" = Linux ]; then
    flatpak_updates_check
    return 0
  fi
  if [ "$(uname)" != Darwin ]; then
    echo "skip: updates. Only the Mac's build carries Sparkle so far."
    return 0
  fi
  serve
  scratch=$(mktemp -d)
  mkdir "$scratch/profile" "$out/site/updates"
  # The run's own key, in its temp dir, which cleanup deletes if the run fails.
  public=$(python3 -I ../tools/appcast.py key "$scratch/key")
  printf '%s\n\n%s\n' "A check update, from the updates run's own feed." \
    "- Its notes are Markdown." > "$scratch/notes.md"
  UPDATE_FEED="${site}updates/appcast.xml" UPDATE_PUBLIC_KEY="$public" ./build.sh updates \
    > /dev/null || fail "build.sh updates didn't make the check build"
  feed() {
    ../tools/appcast.sh "$scratch/notes.md" --local build/Updates.app "$out/site/updates" \
      "${site}updates/" "$scratch/key" "$@" > /dev/null ||
      fail "appcast.sh --local didn't make a feed"
  }
  feed
  next=$(($(plutil -extract CFBundleVersion raw build/Updates.app/Contents/Info.plist) + 1))
  # Sparkle's settings are in the check build's own defaults, which start empty.
  saved_app=$app saved_browser=$browser saved_procs=$procs
  app="$PWD/build/Updates.app"
  browser="$app/Contents/MacOS/Browser"
  procs="$app/Contents"
  app_id=dev.browser.app.updates
  defaults delete "$app_id" > /dev/null 2>&1 || true
  # A page that needs no server, so the feed is all the server has to serve.
  page='data:text/html,<title>updates</title>'
  # Launches the check build, which last checked for updates at the date $1, as date -v takes it.
  launch() {
    defaults write "$app_id" SULastCheckTime -date "$(date -u -v"$1" '+%Y-%m-%d %H:%M:%S +0000')"
    step=0
    BROWSER_DRIVE=1 BROWSER_URL="$page" BROWSER_PROFILE_DIR="$scratch/profile" "$browser" \
      < "$out/drive" > "$log" 2>&1 &
    pid=$!
    exec 3> "$out/drive"
    await_load
    await '^browser: updater started$' || fail "the check build didn't start Sparkle"
    await '^browser: update check in [0-9]* s$' || fail "Sparkle didn't schedule its next check"
    delay=$(sed -n 's/^browser: update check in \([0-9]*\) s$/\1/p' "$log" | head -1)
  }

  # Checked a moment ago, the next check is 8 hours from it.
  log="$out/updates.log"
  launch +0S
  [ "$delay" -gt 28700 ] && [ "$delay" -le 28800 ] ||
    fail "checked a moment ago, Sparkle's next check is in $delay s, not 8 hours"
  interval=$delay
  grep '^browser: update' "$log" > "$out/updates-interval.txt"
  send state
  if [ "$active" != 1 ]; then
    exec 3>&-
    quit
    echo "skip: updates. Browser wasn't the active app, so Sparkle's alert couldn't get the" \
      "keyboard."
  else
    send menubar "$app_name"
    expect "^browser: menu bar $app_name: About $app_name, Check for Updates…, Settings…" \
      "the app menu doesn't have Check for Updates… under About"
    post openmenu "$app_name"
    expect "^browser: menu tracked About $app_name, Check for Updates…, Settings…" \
      "the app menu didn't open"
    ../tools/window-screenshot.sh "$pid" "$out/updates-menu.png" all || fail "no window to capture"
    send dismiss
    send menu Check for Updates…
    # Found, or Sparkle's reason it isn't: none, with the feed's newest build, or failed. A check
    # that ends in none of them says how far it got, and Sparkle's log, the app's main thread and
    # the window say where it stopped.
    outcome="^browser: update \($next found\|none:\|failed:\)"
    for _ in $(seq 300); do
      grep -q "$outcome" "$log" && break
      sleep 0.1
    done
    if ! grep -q "$outcome" "$log"; then
      /usr/bin/log show --last 5m --style compact \
        --predicate 'subsystem == "org.sparkle-project.Sparkle"' > "$out/updates-sparkle.txt" 2>&1
      sample "$pid" 3 -file "$out/updates-sample.txt" > /dev/null 2>&1
      ../tools/window-screenshot.sh "$pid" "$out/updates-stalled.png" all
      got=$(grep '^browser: update \(feed loaded\|cycle ended\|alert shows\)' "$log" | tr '\n' ';')
      fail "Check for Updates… didn't end in 30 s: no update found, none or failed. Sparkle got to: \
${got:-nothing logged}. Sparkle's log: $out/updates-sparkle.txt, the app's threads: \
$out/updates-sample.txt, the window: $out/updates-stalled.png"
    fi
    if ! grep -q "^browser: update $next found\$" "$log"; then
      # Sparkle's own log, which says more, such as an item it couldn't parse.
      /usr/bin/log show --last 5m --style compact \
        --predicate 'subsystem == "org.sparkle-project.Sparkle"' > "$out/updates-sparkle.txt" 2>&1
      fail "Check for Updates… didn't find build $next: $(grep '^browser: update \(none\|failed\):' \
        "$log"). Sparkle's log: $out/updates-sparkle.txt"
    fi
    expect "^browser: update $next shown\$" "Sparkle didn't show the update it found"
    expect "^browser: update $next notes A check update, from the updates run's own feed\.\$" \
      "the update's alert didn't show its notes"
    expect "^browser: update $next alert has the keyboard\$" \
      "the update's alert didn't get the keyboard"
    ../tools/window-screenshot.sh "$pid" "$out/updates-found.png" all || fail "no window to capture"
    exec 3>&-
    quit

    # Last checked 7 hours 59 minutes 35 seconds ago, Sparkle checks 25 s after the launch, with
    # up to 15 s more of its timer's leeway, by when the window is full screen. The update is
    # informational, which an automatic check shows rather than downloads.
    feed --informational
    log="$out/updates-full-screen.log"
    launch -28775S
    [ "$delay" -le 25 ] || fail "checked 8 hours ago less 25 s, the next check is in $delay s"
    send fullscreen
    expect '^browser: window entered full screen$' "the window didn't go full screen"
    grep -q "^browser: update $next " "$log" &&
      fail "Sparkle checked before the window went full screen, $delay s after the launch"
    # The check comes when Sparkle's timer fires, up to 40 s after the launch.
    for _ in $(seq 600); do
      grep -q "^browser: update $next \(held in full screen\|shown\)\$" "$log" && break
      kill -0 "$pid" 2>/dev/null || fail "the app exited"
      sleep 0.1
    done
    grep -q "^browser: update $next held in full screen\$" "$log" ||
      fail "the check in full screen didn't hold its update: $(grep "^browser: update" "$log")"
    grep -q '^browser: update .* alert has the keyboard$' "$log" &&
      fail "the update held in full screen took the keyboard"
    ../tools/window-screenshot.sh "$pid" "$out/updates-held.png" all || fail "no window to capture"
    send fullscreen
    expect '^browser: window left full screen$' "the window didn't leave full screen"
    expect '^browser: update shown after full screen$' \
      "the held update wasn't shown after full screen"
    expect "^browser: update $next alert has the keyboard\$" \
      "the update shown after full screen didn't get the keyboard"
    ../tools/window-screenshot.sh "$pid" "$out/updates-after-full-screen.png" all ||
      fail "no window to capture"
    exec 3>&-
    quit
    echo "ok: the check build's app menu has Check for Updates… under About, and Sparkle checks 8" \
      "hours after the last check, in $interval s here. Check for Updates… found the run's" \
      "signed update and showed it with its Markdown notes. A check in full screen held its" \
      "update, which came with the keyboard once the window left full screen." \
      "Interval: $out/updates-interval.txt." \
      "Windows: $out/updates-menu.png, $out/updates-found.png, $out/updates-held.png," \
      "$out/updates-after-full-screen.png"
  fi
  defaults delete "$app_id" > /dev/null 2>&1 || true
  rm -rf "$scratch"
  scratch=
  app=$saved_app browser=$saved_browser procs=$saved_procs app_id=dev.browser.app
}
flatpak_updates_check() {
  local id=app.ilo.browser remote=ilo-updates-check portal= tool
  for tool in flatpak flatpak-builder ostree dbus-daemon; do
    command -v "$tool" > /dev/null || {
      echo "skip: updates. Linux's needs flatpak, flatpak-builder, ostree and dbus-daemon:" \
        "no $tool."
      return 0
    }
  done
  for tool in /usr/libexec/flatpak-portal /usr/lib/flatpak/flatpak-portal; do
    [ -x "$tool" ] && portal=$tool && break
  done
  [ -n "$portal" ] || { echo "skip: updates. No flatpak-portal."; return 0; }
  local origin
  origin=$(flatpak info --user --show-origin "$id" 2> /dev/null || true)
  if [ -n "$origin" ] && [ "$origin" != "$remote" ]; then
    echo "skip: updates. ilo's Flatpak is installed here from $origin, which the run would replace."
    return 0
  fi
  ../tools/flatpak.sh > "$out/updates-flatpak.log" 2>&1 ||
    fail "tools/flatpak.sh didn't build the Flatpak: $out/updates-flatpak.log"
  scratch=$(mktemp -d)
  local repo=$scratch/repo ref=app/$id/x86_64/master
  ostree init --mode=archive-z2 --repo="$repo"
  ostree pull-local --repo="$repo" build/flatpak/repo "$ref" > /dev/null
  flatpak build-update-repo "$repo" > /dev/null
  flatpak remote-delete --user --force "$remote" 2> /dev/null || true
  flatpak remote-add --user --no-gpg-verify "$remote" "file://$repo"
  flatpak install --user -y --noninteractive --reinstall "$remote" "$id" \
    > "$out/updates-install.log" 2>&1 ||
    fail "the Flatpak didn't install from the run's repository: $out/updates-install.log"
  local installed
  installed=$(flatpak info --user "$id" | sed -n 's/^ *Commit: //p')
  # The run's own home and session bus, with Flatpak's own installation.
  local saved_home=$HOME saved_user_dir=${FLATPAK_USER_DIR:-}
  local saved_bus=${DBUS_SESSION_BUS_ADDRESS:-}
  export FLATPAK_USER_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/flatpak"
  export HOME=$scratch/home
  mkdir "$HOME"
  local started
  started=$(env -u XDG_DATA_HOME dbus-daemon --session --fork --print-address=1 --print-pid=1 |
    tr '\n' ' ')
  export DBUS_SESSION_BUS_ADDRESS=${started%% *}
  buses=$(echo "$started" | cut -d' ' -f2)
  XDG_DATA_HOME='' flatpak permission-set flatpak updates "$id" yes ||
    fail "Flatpak's permission store didn't take the run's yes: is xdg-desktop-portal installed?"
  "$portal" --poll-timeout=10 --no-idle-exit > "$out/updates-portal.log" 2>&1 &
  buses="$buses $!"
  local saved_procs=$procs
  procs=/app/ilo/chrome
  log="$out/updates.log"
  step=0
  flatpak run --user --env=BROWSER_DRIVE=1 "$id" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await '^browser: updater started$' || fail "the Flatpak didn't start its updater"
  # A newer build, as a release publishes one.
  flatpak build-commit-from --src-ref="$ref" --subject="The updates run's build" --force \
    "$repo" "$ref" > /dev/null
  flatpak build-update-repo "$repo" > /dev/null
  local next
  next=$(ostree rev-parse --repo="$repo" "$ref")
  # The portal looks every 10 s, then installs it.
  for _ in $(seq 1200); do
    grep -q '^browser: update \(Ready\|Failed\)' "$log" && break
    kill -0 "$pid" 2> /dev/null || fail "the app exited"
    sleep 0.1
  done
  grep -q '^browser: update Installing$' "$log" ||
    fail "ilo didn't install the build Flatpak's portal announced: $out/updates-portal.log"
  grep -q '^browser: update Ready$' "$log" ||
    fail "the new build didn't install: $(grep '^browser: update' "$log" | tail -1)"
  [ "$(flatpak info --user "$id" | sed -n 's/^ *Commit: //p')" = "$next" ] ||
    fail "Flatpak didn't install build $next beside $installed"
  ../tools/window-screenshot.sh "$pid" "$out/updates-ready.png" || fail "no window to capture"
  send choose Restart
  for _ in $(seq 300); do
    kill -0 "$pid" 2> /dev/null || break
    sleep 0.1
  done
  kill -0 "$pid" 2> /dev/null && fail "Restart didn't quit the build running"
  local running= commit=
  for _ in $(seq 600); do
    running=$(flatpak ps --columns=application,commit,pid |
      awk -v id="$id" '$1 == id { print $2, $3; exit }')
    [ -n "$running" ] && break
    sleep 0.1
  done
  commit=${running%% *}
  [ -n "$commit" ] && [ "${next#"$commit"}" != "$next" ] ||
    fail "Restart didn't start build $next: running ${commit:-nothing}. $out/updates-portal.log"
  # The new build logs to the portal's output, which started it.
  pid=${running##* }
  log="$out/updates-portal.log"
  await '^browser: UI shown' || fail "the new build's window didn't come up"
  ../tools/window-screenshot.sh "$pid" "$out/updates-restarted.png" || fail "no window to capture"
  flatpak kill "$id" 2> /dev/null || true
  # Gone before the run ends, which cleanup would otherwise race to kill.
  for _ in $(seq 100); do
    kill -0 "$pid" 2> /dev/null || break
    sleep 0.1
  done
  kill $buses 2> /dev/null || true
  buses=
  export HOME=$saved_home DBUS_SESSION_BUS_ADDRESS=$saved_bus
  unset FLATPAK_USER_DIR
  [ -z "$saved_user_dir" ] || export FLATPAK_USER_DIR="$saved_user_dir"
  flatpak uninstall --user -y --noninteractive "$id" > /dev/null 2>&1 || true
  flatpak remote-delete --user --force "$remote" 2> /dev/null || true
  rm -rf "$scratch"
  scratch=
  procs=$saved_procs
  echo "ok: ilo's Flatpak, installed at $(echo "$installed" | cut -c1-12), installed the newer" \
    "build $(echo "$next" | cut -c1-12) Flatpak's portal announced, offered to restart into it," \
    "and Restart started it once the build running had quit." \
    "Windows: $out/updates-ready.png, $out/updates-restarted.png"
}
updates_check
