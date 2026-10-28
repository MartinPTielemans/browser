# Settings › Import finds the made-up Arc in src/import/fixture and a made-up Chrome and Firefox
# beside it, never the Mac's own browsers. From Arc it brings its two Spaces, one in a new profile
# named Work as Arc's Local State names it, their pinned tabs and folders, and its favorites; from
# Chrome and from Firefox their bookmarks as a Space named after each and their history, which the
# core adds. Run again, each adds nothing. Each step is captured.

serve
printf '%s' '<title>import ready</title>' > "$out/site/import.html"
log="$out/import.log"
step=0
rm -f "$saved"
# A copy of the fixture's home, with Chrome's History and Firefox's places.sqlite written as SQLite
# here rather than kept in the repo as binary files.
home="$out/import-home"
rm -rf "$home"
cp -R src/import/fixture "$home"
chrome="$home/Library/Application Support/Google/Chrome"
mkdir -p "$chrome/Default"
printf '%s' '{"profile":{"info_cache":{"Default":{"name":"Me"}}}}' > "$chrome/Local State"
printf '%s' '{}' > "$chrome/Default/Preferences"
cat > "$chrome/Default/Bookmarks" << 'EOF'
{"roots": {
  "bookmark_bar": {"type": "folder", "name": "Bookmarks bar", "children": [
    {"type": "url", "name": "Mail", "url": "https://mail.example.org/"},
    {"type": "url", "name": "", "url": "https://example.org/"}]},
  "other": {"type": "folder", "name": "Other bookmarks", "children": [
    {"type": "url", "name": "Docs", "url": "https://docs.example.org/"}]},
  "synced": {"type": "folder", "name": "Mobile bookmarks", "children": []}}, "version": 1}
EOF
command -v sqlite3 > /dev/null || fail "no sqlite3 to write Chrome's History with"
# Chromium counts microseconds from 1601: an hour and two hours ago.
now=$(( ($(date +%s) + 11644473600) * 1000000 ))
sqlite3 "$chrome/Default/History" "
  CREATE TABLE urls(id INTEGER PRIMARY KEY AUTOINCREMENT, url LONGVARCHAR, title LONGVARCHAR,
    visit_count INTEGER DEFAULT 0 NOT NULL, typed_count INTEGER DEFAULT 0 NOT NULL,
    last_visit_time INTEGER NOT NULL, hidden INTEGER DEFAULT 0 NOT NULL);
  INSERT INTO urls(url, title, visit_count, typed_count, last_visit_time) VALUES
    ('https://history-one.example.org/', 'One', 3, 1, $now - 3600000000),
    ('https://history-two.example.org/', 'Two', 1, 0, $now - 7200000000);"
firefox="$home/Library/Application Support/Firefox"
mkdir -p "$firefox/Profiles/x.default-release"
printf '[Profile0]\nName=default-release\nIsRelative=1\nPath=Profiles/x.default-release\n' \
  > "$firefox/profiles.ini"
# Firefox counts microseconds from 1970: its two bookmarks never visited, two pages visited an hour
# and two hours ago.
now=$(( $(date +%s) * 1000000 ))
sqlite3 "$firefox/Profiles/x.default-release/places.sqlite" "
  CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url LONGVARCHAR, title LONGVARCHAR,
    visit_count INTEGER DEFAULT 0, hidden INTEGER DEFAULT 0 NOT NULL,
    typed INTEGER DEFAULT 0 NOT NULL, last_visit_date INTEGER, guid TEXT);
  CREATE TABLE moz_bookmarks(id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER DEFAULT NULL,
    parent INTEGER, position INTEGER, title LONGVARCHAR, guid TEXT);
  INSERT INTO moz_places(id, url, title, visit_count, last_visit_date) VALUES
    (1, 'https://fox-mail.example.org/', 'Mail', 0, NULL),
    (2, 'https://fox-docs.example.org/', 'Docs', 0, NULL),
    (3, 'https://history-three.example.org/', 'Three', 2, $now - 3600000000),
    (4, 'https://history-four.example.org/', 'Four', 1, $now - 7200000000);
  INSERT INTO moz_bookmarks(id, type, fk, parent, position, title, guid) VALUES
    (1, 2, NULL, NULL, 0, '', 'root________'), (2, 2, NULL, 1, 0, 'menu', 'menu________'),
    (3, 2, NULL, 1, 1, 'toolbar', 'toolbar_____'), (4, 1, 1, 3, 0, 'Mail', NULL),
    (5, 1, 2, 2, 0, 'Docs', NULL);"
# Arc used last, so it's listed first, then Chrome, then Firefox.
touch -t 200001010000 "$chrome/Default/History" "$chrome/Default/Preferences"
touch -t 199901010000 "$firefox/Profiles/x.default-release/places.sqlite"
touch "$home/Library/Application Support/Arc/StorableSidebar.json"
BROWSER_IMPORT_HOME="$home" BROWSER_DRIVE=1 BROWSER_URL="${site}import.html" \
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
# one. Its list has a row for each browser, its Import… button at the row's end; then a browser's
# choices, a row each, with Back and Import under them.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
send clickkey 370 60
expect '^browser: settings tab Import$' "a click on Import didn't show its tab"
expect '^browser: import found Arc, Chrome, Firefox$' \
  "the import didn't find the fixture's Arc, Chrome and Firefox, and only them"
../tools/window-screenshot.sh "$pid" "$out/import-list.png" || fail "no window to capture"
send clickkey 575 134
expect '^browser: import pick Arc$' "Import… on Arc's row didn't show what comes from it"
../tools/window-screenshot.sh "$pid" "$out/import-pick.png" || fail "no window to capture"
# Arc's three choices: Spaces, Favorites and History.
send clickkey 590 282
expect '^browser: import from Arc: 2 Spaces, 6 pinned tabs, 3 favorites, 0 pages$' \
  "Import didn't plan Arc's 2 Spaces, 6 pinned tabs and 3 favorites"
expect '^browser: added profile [0-9]* Work$' "Arc's Work profile didn't come in"
await '^browser: import done: 2 Spaces, 6 pinned tabs and 3 favorites came in from Arc\.$' ||
  fail "the import didn't finish once the Work profile loaded"
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
send clickkey 590 282
expect '^browser: import from Arc: 0 Spaces, 0 pinned tabs, 0 favorites, 0 pages$' \
  "the second import would add something again"
expect '^browser: import done: Everything from Arc is here already\.$' "the second import said it added something"
../tools/window-screenshot.sh "$pid" "$out/import-again.png" || fail "no window to capture"
# Chrome, the second row, brings its bookmarks and history, which the core adds.
send clickkey 590 145
send clickkey 575 185
expect '^browser: import pick Chrome$' "Import… on Chrome's row didn't show what comes from it"
send clickkey 590 245
expect '^browser: import from Chrome: 1 Spaces, 3 pinned tabs, 0 favorites, 2 pages$' \
  "Import didn't plan Chrome's bookmarks and history"
await '^browser: history added 2 to profile [0-9]*$' || fail "the core didn't add Chrome's 2 pages"
expect '^browser: import done: 1 Space, 3 pinned tabs and 2 pages of history came in from Chrome\.$' \
  "the import didn't say what came from Chrome"
../tools/window-screenshot.sh "$pid" "$out/import-chrome.png" || fail "no window to capture"
send clickkey 590 145
send clickkey 575 185
expect '^browser: import pick Chrome$' "Import… didn't show Chrome's choices again"
send clickkey 590 245
await '^browser: history added 0 to profile [0-9]*$' || fail "the core added Chrome's pages again"
expect '^browser: import done: Everything from Chrome is here already\.$' \
  "the second import from Chrome said it added something"
# Firefox, the third row, brings its toolbar's bookmark, its menu's in a folder, and its history.
send clickkey 590 145
send clickkey 575 236
expect '^browser: import pick Firefox$' "Import… on Firefox's row didn't show what comes from it"
send clickkey 590 245
expect '^browser: import from Firefox: 1 Spaces, 2 pinned tabs, 0 favorites, 2 pages$' \
  "Import didn't plan Firefox's bookmarks and history"
await '^browser: history added 2 to profile [0-9]*$' || fail "the core didn't add Firefox's 2 pages"
expect '^browser: import done: 1 Space, 2 pinned tabs and 2 pages of history came in from Firefox\.$' \
  "the import didn't say what came from Firefox"
../tools/window-screenshot.sh "$pid" "$out/import-firefox.png" || fail "no window to capture"
send clickkey 590 145
send clickkey 575 236
expect '^browser: import pick Firefox$' "Import… didn't show Firefox's choices again"
send clickkey 590 245
await '^browser: history added 0 to profile [0-9]*$' || fail "the core added Firefox's pages again"
expect '^browser: import done: Everything from Firefox is here already\.$' \
  "the second import from Firefox said it added something"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
# The Personal Space shows its pinned tabs: HN, then Reading with a, b and c and Deep with d.
send key 2 ctrl
expect "^browser: space $personal in " "⌃2 didn't show Arc's Personal Space"
expect '^browser: pinned [0-9]*\( [0-9]*\)\{6\}$' \
  "Personal's pinned tabs and folders aren't the 7 rows Arc's are: $(pinned)"
../tools/window-screenshot.sh "$pid" "$out/import-personal.png" || fail "no window to capture"
quit
echo "ok: Settings › Import finds the fixture's Arc, Chrome and Firefox. From Arc come its Spaces,"
echo "  one in a new profile, their pinned tabs and folders, and its favorites; from Chrome and"
echo "  Firefox their bookmarks as a Space and their history; and a second import of each adds nothing."
echo "  Windows: $out/import-list.png, $out/import-pick.png, $out/import-done.png,"
echo "  $out/import-again.png, $out/import-chrome.png, $out/import-firefox.png,"
echo "  $out/import-personal.png"
