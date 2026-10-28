# Settings › Import finds the made-up Arc in src/import/fixture, never the Mac's own browsers, and
# brings in its sidebar: its two Spaces, one in a new profile named Work as Arc's Local State names
# it, their pinned tabs and folders, and its favorites. Run again, it adds nothing. Each step is
# captured.

serve
printf '%s' '<title>import ready</title>' > "$out/site/import.html"
log="$out/import.log"
step=0
rm -f "$saved"
BROWSER_IMPORT_HOME="$PWD/src/import/fixture" BROWSER_DRIVE=1 BROWSER_URL="${site}import.html" \
  "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* import ready$' || fail "the import's page didn't draw"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: import. Browser wasn't the active app, so Settings would ignore the clicks."
  exit 0
fi
first=$(space)
# The toolbar's tabs, 55 wide or as wide as their names, 1 apart, centred: Import is the middle
# one. Its list has a row for Arc, its Import… button at the row's end, then Arc's choices, two
# rows with Back and Import under them.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey 336 60
expect '^browser: settings tab Import$' "a click on Import didn't show its tab"
expect '^browser: import found Arc$' "the import didn't find the fixture's Arc, and only it"
../tools/window-screenshot.sh "$pid" "$out/import-list.png" || fail "no window to capture"
send clickkey 575 134
expect '^browser: import pick Arc$' "Import… on Arc's row didn't show what comes from it"
../tools/window-screenshot.sh "$pid" "$out/import-pick.png" || fail "no window to capture"
send clickkey 590 245
expect '^browser: import from Arc: 2 Spaces, 6 pinned tabs, 3 favorites$' \
  "Import didn't plan Arc's 2 Spaces, 6 pinned tabs and 3 favorites"
expect '^browser: added profile [0-9]* Work$' "Arc's Work profile didn't come in"
await '^browser: import done$' || fail "the import didn't finish once the Work profile loaded"
../tools/window-screenshot.sh "$pid" "$out/import-done.png" || fail "no window to capture"
expect "^browser: spaces $first [0-9]* [0-9]*\$" "the import didn't add Arc's two Spaces after the first"
spaces=$(grep '^browser: spaces' "$log" | tail -1)
personal=$(echo "$spaces" | cut -d' ' -f4)
work=$(echo "$spaces" | cut -d' ' -f5)
work_profile=$(sed -n 's/^browser: added profile \([0-9]*\) Work$/\1/p' "$log" | tail -1)
grep -q "^browser: profiled space $work $work_profile\$" "$log" ||
  fail "Arc's Work Space isn't in the Work profile"
# Done goes back to the list, and the same import again plans nothing.
send clickkey 590 145
send clickkey 575 134
expect '^browser: import pick Arc$' "Import… didn't show Arc's choices again"
send clickkey 590 245
expect '^browser: import from Arc: 0 Spaces, 0 pinned tabs, 0 favorites$' \
  "the second import would add something again"
../tools/window-screenshot.sh "$pid" "$out/import-again.png" || fail "no window to capture"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
# The Personal Space shows its pinned tabs: HN, then Reading with a, b and c and Deep with d.
send key 2 ctrl
expect "^browser: space $personal in " "⌃2 didn't show Arc's Personal Space"
expect '^browser: pinned [0-9]*\( [0-9]*\)\{6\}$' \
  "Personal's pinned tabs and folders aren't the 7 rows Arc's are: $(pinned)"
../tools/window-screenshot.sh "$pid" "$out/import-personal.png" || fail "no window to capture"
quit
echo "ok: Settings › Import finds the fixture's Arc and brings in its Spaces, one in a new profile,"
echo "  their pinned tabs and folders, and its favorites, and a second import adds nothing."
echo "  Windows: $out/import-list.png, $out/import-pick.png, $out/import-done.png,"
echo "  $out/import-again.png, $out/import-personal.png"
