# Quitting through the menu and with ⌘Q, answering Quit when the app asks first, closes the window
# after its tabs and ends every Chromium process with status 0. It leaves no crash report, where
# on Linux a crash as the window closed once left one at every quit (#474). A quit by signal, as
# the other runs' is, never closes the window, so only a quit the user makes finds it.

# The reports Crashpad keeps until they're sent, in the profile's Crashpad folder, as crash.sh
# finds them.
reports() {
  find "$(dirname "$saved")/Crashpad/pending" -name '*.dmp' 2>/dev/null | wc -l | tr -d ' '
}
# Quits through $1, menu or key, and answers Quit in the dialog if it asks, which an earlier run may
# have turned off. Quit is the driver's last command.
quit_through() {
  if [ "$1" = menu ]; then
    post menu "Quit $app_name"
  else
    post key q cmd
  fi
  await '^browser: command quit via ' || fail "Quit through the $1 didn't quit"
  for _ in $(seq 50); do
    kill -0 "$pid" 2>/dev/null || break
    if grep -q '^browser: dialog settled$' "$log"; then
      post key return
      break
    fi
    sleep 0.1
  done
  exec 3>&-
}

for way in menu key; do
  log="$out/quit-$way.log"
  step=0
  before=$(reports)
  # #905 trial: every task Chrome runs, for 15 s, to see where a quit during launch stops.
  BROWSER_DRIVE=1 BROWSER_URL='data:text/html,<h1>Quit</h1>' "$browser" \
    --trace-startup=toplevel,toplevel.flow,shutdown,browser,navigation,disabled-by-default-toplevel.flow \
    --trace-startup-duration=15 --enable-tracing-format=json \
    --trace-startup-file="$PWD/$out/quit-$way-trace.json" < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  # #905 trial: Quit as the window comes on screen, before the UI is shown.
  await '^browser: window on screen at ' || fail "the window never came on screen"
  quit_through "$way"
  exited
  [ "$(reports)" = "$before" ] || fail "quitting through the $way left a crash report"
done
echo "ok: quitting through the menu and with ⌘Q, answering Quit, ends every Chromium process with status 0 and leaves no crash report"
