# About ilo shows Settings on its About tab, and the Open of each of its Licence, Notices and
# Libraries rows opens the file the build carries for it (tools/licences.sh) in a tab, which loads.

log="$out/about.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL='data:text/html,<title>about</title>' "$browser" \
  < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
loaded=
for row in licence:LICENSE notices:NOTICE libraries:THIRD-PARTY.txt; do
  id=${row%%:*} file=${row#*:}
  send menu "About ilo"
  # Settings says which tab it shows as it opens, changes tab or draws, which differ by OS.
  expect '^browser: settings \(shown\|tab\|drew\) About$' "About ilo didn't show Settings' About tab"
  # About logs where each Open is once, on its first draw.
  await "^browser: settings open $id at " || fail "About didn't draw the $id row's Open"
  [ -n "$loaded" ] || ../tools/window-screenshot.sh "$pid" "$out/about.png" all ||
    fail "no window to capture"
  # shellcheck disable=SC2046 # x and y
  send clickkey $(sed -n "s/^browser: settings open $id at //p" "$log" | tail -1)
  expect '^browser: settings hidden$' "a click on the $id row's Open didn't hide Settings"
  expect "^browser: loaded [0-9]* file://.*/${file%.txt}" "a click on the $id row's Open didn't load $file"
  loaded="$loaded $file"
done
../tools/window-screenshot.sh "$pid" "$out/about-opened.png" all || fail "no window to capture"
exec 3>&-
quit
echo "ok: About ilo showed About, and its rows' Open loaded$loaded in tabs. Windows: $out/about.png, $out/about-opened.png"
