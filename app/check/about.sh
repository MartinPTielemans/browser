# About ilo shows Settings on its About tab, and its Notices' Open opens the NOTICE the build
# carries in a tab, which loads.

log="$out/about.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL='data:text/html,<title>about</title>' "$browser" \
  < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
send menu "About ilo"
expect '^browser: settings tab About$' "About ilo didn't show Settings' About tab"
expect '^browser: settings open notices at ' "About didn't draw the Notices' Open"
at=$(sed -n 's/^browser: settings open notices at //p' "$log" | tail -1)
../tools/window-screenshot.sh "$pid" "$out/about.png" all || fail "no window to capture"
send clickkey $at
expect '^browser: settings hidden$' "a click on the Notices' Open didn't hide Settings"
expect '^browser: loaded [0-9]* file://.*/NOTICE$' "a click on the Notices' Open didn't load NOTICE"
await "^browser: title [0-9]* .*NOTICE" || true
../tools/window-screenshot.sh "$pid" "$out/notice.png" all || fail "no window to capture"
loaded=$(grep '^browser: loaded [0-9]* file://.*/NOTICE$' "$log" | tail -1 | cut -d' ' -f4-)
exec 3>&-
quit
echo "ok: About ilo showed About, and its Notices' Open loaded $loaded. Windows: $out/about.png, $out/notice.png"
