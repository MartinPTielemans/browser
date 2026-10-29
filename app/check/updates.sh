# The Mac's updates (docs/ux-spec.md › Updates), on a check build with Sparkle (build.sh updates)
# that checks a feed the run serves, for updates signed with a key the run makes and deletes after.
# Neither is ever the product's: the run never reads branding/update-feed or the owner's key.
# The app menu has Check for Updates… first. Sparkle checks every 8 hours, from the last check.
# Check for Updates… finds the feed's update and shows it with its Markdown notes. A scheduled
# check that finds one while a window is full screen holds it, and shows it with the keyboard once
# the window leaves full screen. A scheduled check downloads an update by itself, the check build at
# the next build signed ad hoc, and Relaunch to update draws over the sidebar's footer. While the
# Mac is idle, as the driver's idle has it without the screen locking, the update waits with
# Install updates automatically off, as it starts, and with it on while the window is full screen
# or a page plays, muted so the run makes no sound, and installs once neither holds it. A click on
# Relaunch to update installs it at once. Sparkle relaunches neither under the driver, which would
# open the check build outside the run, and says so; the build in place is the update after each.
# Not here: the release notes the first launch of an update opens, which only the prod build does,
# for https notes only.

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
  check_build() {
    UPDATE_FEED="${site}updates/appcast.xml" UPDATE_PUBLIC_KEY="$public" ./build.sh updates \
      > /dev/null || fail "build.sh updates didn't make the check build"
  }
  check_build
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
    # Found, or Sparkle's reason it isn't: none, with the feed's newest build, or failed.
    expect "^browser: update \($next found\|none:\|failed:\)" \
      "Check for Updates… didn't end: no update found, none or failed"
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
    echo "ok: the check build's app menu starts with Check for Updates…, and Sparkle checks 8" \
      "hours after the last check, in $interval s here. Check for Updates… found the run's" \
      "signed update and showed it with its Markdown notes. A check in full screen held its" \
      "update, which came with the keyboard once the window left full screen." \
      "Interval: $out/updates-interval.txt." \
      "Windows: $out/updates-menu.png, $out/updates-found.png, $out/updates-held.png," \
      "$out/updates-after-full-screen.png"
  fi

  # The update Sparkle downloads by itself: the check build at build $next, signed ad hoc, as
  # Sparkle needs of an update to an app with a signature, and its installer puts it in the check
  # build's place. The page plays a quiet tone on p until x.
  mkdir "$scratch/update"
  cp -cR "$app" "$scratch/update/Updates.app"
  plutil -replace CFBundleVersion -string "$next" "$scratch/update/Updates.app/Contents/Info.plist"
  codesign --force --deep --sign - "$scratch/update/Updates.app" > /dev/null 2>&1 ||
    fail "the update to build $next couldn't be signed ad hoc"
  feed --app "$scratch/update/Updates.app"
  page='data:text/html,<title>loading</title>
<script>
requestAnimationFrame(() => setTimeout(() => document.title = "ready"));
let sound;
onkeydown = e => {
  if (e.key == "p") {
    sound = new AudioContext();
    const tone = sound.createOscillator();
    const gain = sound.createGain();
    gain.gain.value = 0.002;
    tone.connect(gain).connect(sound.destination);
    tone.start();
    document.title = "playing";
  }
  if (e.key == "x") sound.close();
};
</script>'
  # Waits up to 2 minutes for Sparkle's scheduled check to download the update and ready it, and
  # for Relaunch to update to draw over the footer, where $row says it is.
  await_ready() {
    for _ in $(seq 1200); do
      grep -q "^browser: update $next ready\$" "$log" && break
      grep -q '^browser: update failed:' "$log" &&
        fail "Sparkle didn't ready build $next: $(grep '^browser: update failed:' "$log" | tail -1)"
      kill -0 "$pid" 2>/dev/null || fail "the app exited"
      sleep 0.1
    done
    grep -q "^browser: update $next ready\$" "$log" ||
      fail "Sparkle didn't ready build $next in 2 minutes: $(grep '^browser: update' "$log" |
        tr '\n' ' ')"
    await '^browser: update row at [0-9]* [0-9]*$' ||
      fail "Relaunch to update didn't draw once build $next was ready"
    row=$(sed -n 's/^browser: update row at //p' "$log" | tail -1)
  }
  # The app ends, Sparkle's installer puts build $next in the check build's place, and nothing of
  # the check build is left running, within a minute. $1 is what installed it.
  install_ends() {
    exec 3>&-
    for _ in $(seq 120); do
      built=$(plutil -extract CFBundleVersion raw "$app/Contents/Info.plist" 2>/dev/null || true)
      if ! kill -0 "$pid" 2>/dev/null && [ "$built" = "$next" ] &&
        ! pgrep -f "$procs" > /dev/null; then
        return 0
      fi
      sleep 0.5
    done
    kill -0 "$pid" 2>/dev/null && fail "$1 didn't end the app to install build $next"
    fail "$1 didn't install build $next: build ${built:-none} is in place"
  }

  # Install updates automatically is off, as it starts: the idle Mac leaves the update ready.
  log="$out/updates-ready.log"
  launch -28775S
  await_ready
  ../tools/window-screenshot.sh "$pid" "$out/updates-ready.png" || fail "no window to capture"
  send idle
  expect "^browser: update $next idle waits: setting off\$" \
    "the idle Mac with Install updates automatically off didn't leave the update ready"
  send state
  if [ "$active" = 1 ]; then
    # shellcheck disable=SC2086
    send click $row
    expect '^browser: update relaunch via sidebar$' "a click on Relaunch to update didn't reach it"
    await "^browser: update $next relaunch held: driven\$" ||
      fail "Relaunch to update didn't install build $next and relaunch"
    install_ends "Relaunch to update"
    relaunched="A click on it installed the update and asked to relaunch."
  else
    quit_app
    install_ends "the quit"
    relaunched="It installed at the quit."
    echo "skip: updates' click on Relaunch to update. Browser wasn't the active app."
  fi

  # Turned on, the idle Mac installs the update, unless a window is full screen or a page plays.
  check_build
  defaults write "$app_id" InstallUpdatesWhileIdle -bool YES
  log="$out/updates-idle.log"
  launch -28775S
  await_ready
  send fullscreen
  expect '^browser: window entered full screen$' "the window didn't go full screen"
  send idle
  expect "^browser: update $next idle waits: full screen\$" \
    "the idle Mac installed the update while the window was full screen"
  ../tools/window-screenshot.sh "$pid" "$out/updates-idle-full-screen.png" ||
    fail "no window to capture"
  send fullscreen
  expect '^browser: window left full screen$' "the window didn't leave full screen"
  send state
  if [ "$active" = 1 ]; then
    await '^browser: today [0-9][0-9]*$' || fail "the page isn't in Today"
    a=$(today)
    # Muted first, as the tabs run does, so the run makes no sound: a page playing muted holds the
    # update too.
    bar_command toggle-mute "Mute/Unmute Tab" mute
    expect "^browser: page $a audible 0 muted 1\$" "Mute/Unmute Tab didn't mute the tab"
    send click $page_point
    send key p
    expect "^browser: title $a playing\$" "p in the page didn't play its sound"
    expect "^browser: page $a audible 1 muted 1\$" "the page's sound didn't show"
    send idle
    expect "^browser: update $next idle waits: a page plays sound\$" \
      "the idle Mac installed the update while a page played"
    ../tools/window-screenshot.sh "$pid" "$out/updates-idle-sound.png" ||
      fail "no window to capture"
    send key x
    expect "^browser: page $a audible 0 muted 1\$" "the page's sound didn't stop"
    played="It waited while a page played, $out/updates-idle-sound.png."
  else
    played=
    echo "skip: updates' page playing as the Mac goes idle. Browser wasn't the active app, so" \
      "the page ignored the keys that play it."
  fi
  send idle
  expect "^browser: update $next idle installs\$" "the idle Mac didn't install the update"
  await "^browser: update $next relaunch held: driven\$" ||
    fail "the idle install didn't install build $next and relaunch"
  install_ends "the idle install"
  echo "ok: a scheduled check downloaded the run's update, build $next, and Relaunch to update" \
    "drew over the sidebar's footer at $row. The idle Mac left it ready with Install updates" \
    "automatically off. $relaunched Turned on, the idle Mac waited while the window was full" \
    "screen. $played With neither, it installed the update and asked to relaunch, held under" \
    "the driver. Windows: $out/updates-ready.png, $out/updates-idle-full-screen.png"

  defaults delete "$app_id" > /dev/null 2>&1 || true
  rm -rf "$scratch"
  scratch=
  app=$saved_app browser=$saved_browser procs=$saved_procs app_id=dev.browser.app
}
updates_check
