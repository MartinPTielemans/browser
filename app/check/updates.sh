# The Mac's updates (docs/ux-spec.md › Updates), on a check build with Sparkle (build.sh updates)
# that checks a feed the run serves, for updates signed with a key the run makes and deletes after.
# Neither is ever the product's: the run never reads branding/update-feed or the owner's key.
# The app menu has Check for Updates… first. Sparkle checks every 8 hours, from the last check.
# Check for Updates… finds the feed's update and shows it with its Markdown notes. A scheduled
# check that finds one while a window is full screen holds it, and shows it with the keyboard once
# the window leaves full screen.
# Not here: the release notes the first launch of an update opens, which only the prod build does,
# for https notes only, and the install, which needs a signed app to update to.

updates_check() {
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
    expect "^browser: menu bar $app_name: Check for Updates…, Settings…" \
      "the app menu doesn't start with Check for Updates…"
    post openmenu "$app_name"
    expect "^browser: menu tracked Check for Updates…, Settings…" "the app menu didn't open"
    ../tools/window-screenshot.sh "$pid" "$out/updates-menu.png" all || fail "no window to capture"
    send dismiss
    send menu Check for Updates…
    expect "^browser: update $next found\$" "Check for Updates… didn't find the feed's update"
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
    echo "ok: the check build's app menu starts with Check for Updates…, and Sparkle checks 8" \
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
updates_check
