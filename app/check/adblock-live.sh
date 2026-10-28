# With the lists uBlock Origin ships and nothing given for the check, a page's script from Google's
# ad network doesn't load, and an ad server's own page doesn't open: uBlock Origin shows its page
# saying it blocked it instead. It needs the network and those servers, so it runs only when named:
# ./check.sh adblock-live.

# The served page's title says whether Google's ad script loaded, as the network told it.
serve
printf '%s' '<title>loading</title>
<script src="https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"
  onload="document.title = &quot;ad script loaded&quot;"
  onerror="document.title = &quot;ad script blocked&quot;"></script>' > "$out/site/live.html"
ublock=oghkpboejbohceocapipmnjifoklonij
profile_dir="$PWD/$out/adblock-live/profile"
rm -rf "$profile_dir"
mkdir -p "$profile_dir"
log="$out/adblock-live.log"
step=0
BROWSER_DRIVE=1 BROWSER_URL="${site}live.html" BROWSER_PROFILE_DIR="$profile_dir" \
  "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
tab=$(today | cut -d' ' -f1)
await "^browser: title $tab ad script " || fail "the page didn't say whether the ad script loaded"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: adblock-live. Browser wasn't the active app, so the page didn't get its keys."
  exit 0
fi
# On a new profile uBlock Origin lets everything through until it has read its lists, so the page
# reloads until its ad script is blocked, for at most 20 reloads.
for _ in $(seq 20); do
  grep -q "^browser: title $tab ad script blocked\$" "$log" && break
  send key r cmd
  expect "^browser: title $tab ad script " "the page didn't report after ⌘R"
done
grep -q "^browser: title $tab ad script blocked\$" "$log" ||
  fail "uBlock Origin's lists didn't block Google's ad script"
../tools/window-screenshot.sh "$pid" "$out/adblock-live-script.png" || fail "no window to capture"
go_from_bar doubleclick.net
expect "^browser: loaded $tab chrome-extension://$ublock/document-blocked\.html" \
  "doubleclick.net opened, where uBlock Origin's page should have said it blocked it"
../tools/window-screenshot.sh "$pid" "$out/adblock-live-page.png" || fail "no window to capture"
quit
echo "ok: with the lists uBlock Origin ships, Google's ad script doesn't load, and doubleclick.net shows uBlock Origin's page saying it blocked it. Windows: $out/adblock-live-script.png, $out/adblock-live-page.png"
