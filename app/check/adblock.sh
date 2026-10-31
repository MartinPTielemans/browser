# The browser blocks ads and trackers from a new profile's first page, with nothing to install:
# uBlock Origin, in the framework, loads into every profile, listed last in the Extensions menu. A
# filter given for the check, through the same managed settings the browser gives it its own,
# blocks the served page's ad script and hides its ad, and it stays in the menu all along, as it
# doesn't reload itself as it first starts. The run reports how long that first launch let requests
# through. It pins as any extension, and its action's menu has no Remove. Its own lists take fbclid
# out of an address. The Site Controls' Block ads and trackers turns that off for the site,
# reloading the page, which then shows the ad, and on again. Off for the site, Settings lists it,
# and its Turn On turns it on there again. Settings' Block ads and trackers turns it off in every
# profile, which takes it out of the Extensions menu and the Site Controls, and on again. A private
# window blocks too. The app ships the version ublock.ini pins, trusting no site, YouTube neither,
# and launched again on the same profile, it blocks there too, the run reporting whether it did on
# that launch's first page.

# The served ad page: its title says whether its script ran and whether its ad shows. uBlock Origin
# hides the ad from a stylesheet of its own, which the page can't hear arrive, so it looks again
# every 100 ms.
serve
printf '%s' 'self.adLoaded = true;' > "$out/site/check-ad.js"
printf '%s' '<title>loading</title>
<div class=check-ad style="height:60px;background:#c33;color:#fff;font:20px system-ui">an ad</div>
<script src="check-ad.js"></script>
<script>
const report = () => {
  const hidden = getComputedStyle(document.querySelector(".check-ad")).display == "none";
  const title = "ad script " + (self.adLoaded ? "loaded" : "blocked") +
    ", element " + (hidden ? "hidden" : "shown");
  if (document.title != title) {
    document.title = title;
  }
};
requestAnimationFrame(() => setTimeout(report));
setInterval(report, 100);
</script>' > "$out/site/ads.html"
ads="${site}ads.html"
ublock=oghkpboejbohceocapipmnjifoklonij
# A profile of the run's own, so uBlock Origin starts as on a new install: it keeps the managed
# settings it last read, and reads its lists into the profile on its first launch.
profile_dir="$PWD/$out/adblock/profile"
rm -rf "$profile_dir"
mkdir -p "$profile_dir"
log="$out/adblock.log"
step=0
filters="$(printf '/check-ad.js\n127.0.0.1##.check-ad')"
launched=$(python3 -I -c 'import time; print(time.time())')
BROWSER_DRIVE=1 BROWSER_URL="$ads" BROWSER_PROFILE_DIR="$profile_dir" \
  "$browser" --ad-block-filters="$filters" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await "^browser: extensions menu .*$ublock\$" ||
  fail "uBlock Origin wasn't in the Extensions menu: $(grep '^browser: extensions' "$log")"
tab=$(today | cut -d' ' -f1)
await "^browser: title $tab ad script " || fail "the ad page didn't report"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  kill "$server"
  echo "skip: adblock. Browser wasn't the active app, so the page didn't get its keys."
  exit 0
fi
# Reloads the tab until its page says $1, for at most 20 reloads, each waiting up to 2 s for the
# page to say it. On a new profile, uBlock Origin lets everything through until it has read its
# lists, a few seconds in.
reload_until() {
  for _ in $(seq 20); do
    send key r cmd
    expect "^browser: title $tab ad script " "the ad page didn't report after ⌘R"
    for _ in $(seq 20); do
      none_since "^browser: title $tab $1\$" || return 0
      sleep 0.1
    done
  done
  fail "$2: $(grep "^browser: title $tab " "$log" | tail -1)"
}
blocked="ad script blocked, element hidden"
reload_until "$blocked" "the check's filter didn't block the ad script and hide the ad"
# How long the first launch let requests through, to the reload that found the ad blocked: the
# spec's about 3 s, which this bounds from above by the reload's wait.
first_block=$(python3 -I -c 'import sys, time; print("%.1f" % (time.time() - float(sys.argv[1])))' \
  "$launched")
reloads=$(grep -c "^browser: title $tab ad script " "$log" || true)
echo "adblock: a new profile's first launch blocked within $first_block s, over $reloads loads"
../tools/window-screenshot.sh "$pid" "$out/adblock-blocked.png" || fail "no window to capture"
# It never left the menu: reloading itself as it first started unloaded it under the first pages,
# and Chromium then dropped what they were asking for (#954).
left=$(grep '^browser: extensions menu' "$log" | grep -v "$ublock\$" | head -1)
[ -z "$left" ] || fail "uBlock Origin left the Extensions menu on the first launch: $left"
# Pinnable as any extension, its action alone at the URL field's end, 3 in from the 228-wide
# sidebar's field ending at 220, with the count of what it blocked on the page. Its menu has no
# Remove, since Settings turns it off.
send menu "Pin to URL Field › uBlock Origin"
expect "^browser: pin extension $ublock\$" "Pin to URL Field › uBlock Origin didn't reach the UI"
expect "^browser: extension actions $ublock\(:[0-9]*\)\{0,1\}\$" \
  "the pinned uBlock Origin didn't show in the URL field"
../tools/window-screenshot.sh "$pid" "$out/adblock-pinned.png" || fail "no window to capture"
post rightclick 202 64
expect "^browser: menu shown $(cased 'Unpin, Options, Manage Extension')\$" \
  "a right-click on uBlock Origin's action didn't show its menu without Remove"
send choose Unpin
expect "^browser: unpin extension $ublock\$" "Unpin didn't unpin uBlock Origin"
expect '^browser: extension actions $' "unpinned, uBlock Origin's action stayed in the URL field"
send click $page_point

# Its own lists take Facebook's click id out of an address before the page loads, and leave the
# site's own parameters.
go_from_bar "${ads#http://}?id=1&fbclid=2"
expect "^browser: loaded $tab ${ads}.id=1\$" \
  "the tab kept fbclid in its address: $(grep "^browser: loaded $tab " "$log" | tail -1)"
expect "^browser: title $tab $blocked\$" "the page without fbclid didn't block its ad"

# Off for the site in the Site Controls, which reload the page with its ad, and on again. With no
# extension turned on but the ad blocker, the panel's first row's middle is at y 262, as in the
# clear run.
controls="^browser: site controls http://127.0.0.1:$port: [^;]*; "
open_site_controls() {
  send key l cmd
  expect '^browser: command bar shown$' "⌘L didn't open the command bar"
  send type site controls
  expect '^browser: command bar commands "site controls": Site Controls$' \
    "the command bar didn't find Site Controls"
  send key return
  expect '^browser: site controls settled$' "the Site Controls didn't open"
}
open_site_controls
expect "${controls}Ad blocking On" "the Site Controls didn't list ad blocking as on, first"
send click 207 262
expect '^browser: site controls set ad blocking off$' "a click on Block ads and trackers didn't run"
expect "^browser: title $tab ad script loaded, element shown\$" \
  "off for the site, the page didn't load its ad script and show its ad"
expect "${controls}Ad blocking Off" "the Site Controls didn't show ad blocking as off for the site"
../tools/window-screenshot.sh "$pid" "$out/adblock-off-for-site.png" || fail "no window to capture"
send click 207 262
expect '^browser: site controls set ad blocking on$' "a second click didn't turn it on again"
expect "^browser: title $tab $blocked\$" "on again, the page didn't block its ad"
send key escape
expect '^browser: site controls hidden$' "escape didn't close the Site Controls"

# Off for the site again, Settings lists it under Block ads and trackers, and its Turn On turns it
# on again there.
open_site_controls
send click 207 262
expect '^browser: site controls set ad blocking off$' \
  "a third click on Block ads and trackers didn't run"
expect "^browser: title $tab ad script loaded, element shown\$" \
  "off for the site again, the page didn't load its ad script and show its ad"
send key escape
expect '^browser: site controls hidden$' "escape didn't close the Site Controls"
send menu Settings…
expect '^browser: settings shown General$' "Settings… didn't show General"
expect '^browser: settings ad blocking off on 127\.0\.0\.1$' \
  "Settings didn't list the site ad blocking is off on, alone"
expect '^browser: settings turn on 127\.0\.0\.1 at ' "Settings didn't draw the site's Turn On"
turn_on=$(sed -n "s/^browser: settings turn on 127\.0\.0\.1 at //p" "$log" | tail -1)
../tools/window-screenshot.sh "$pid" "$out/adblock-off-sites.png" all || fail "no window to capture"
send clickkey $turn_on
expect '^browser: ad blocking on for 127\.0\.0\.1$' "a click on the site's Turn On didn't run"
expect '^browser: settings ad blocking off on $' "turned on, Settings still listed the site"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
send click $page_point
reload_until "$blocked" "turned on again in Settings' list, the page didn't block its ad"

# Off everywhere in Settings, whose fifth row of General is Block ads and trackers, under
# Favorites, its switch's middle at 592 296: 87 under Unload's at 209, past a pop-up's row.
send menu Settings…
expect '^browser: settings shown General$' "Settings… didn't show General again"
send clickkey 592 296
expect '^browser: ad blocking off$' "a click on Block ads and trackers' switch didn't turn it off"
expect '^browser: extensions menu $' "turned off, uBlock Origin stayed in the Extensions menu"
../tools/window-screenshot.sh "$pid" "$out/adblock-settings.png" all || fail "no window to capture"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
send click $page_point
send key r cmd
expect "^browser: title $tab ad script loaded, element shown\$" \
  "turned off in Settings, the page didn't load its ad"
open_site_controls
expect "$controls" "the Site Controls didn't list the site's rows"
none_since "${controls}Ad blocking" || fail "turned off in Settings, the Site Controls listed it"
send key escape
expect '^browser: site controls hidden$' "escape didn't close the Site Controls"
send menu Settings…
expect '^browser: settings shown General$' "Settings… didn't show General a third time"
send clickkey 592 296
expect '^browser: ad blocking on$' "a second click on the switch didn't turn it on"
expect "^browser: extensions menu .*$ublock\$" "turned on, uBlock Origin didn't come back"
send key w cmd
expect '^browser: settings hidden$' "⌘W didn't hide Settings"
send click $page_point
reload_until "$blocked" "turned on again in Settings, the page didn't block its ad"

# A private window blocks too.
send key n cmd shift
expect '^browser: private window opened [0-9]* profile [0-9]*$' "⇧⌘N didn't open a private window"
send window 2
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar in the private window"
private_ads="$ads?private"
send insert "$private_ads"
send key return
expect "^browser: loaded [0-9]* $private_ads\$" "the private window didn't load the ad page"
tab=$(sed -n "s|^browser: loaded \([0-9]*\) $private_ads\$|\1|p" "$log" | tail -1)
expect "^browser: title $tab ad script " "the private window's ad page didn't report"
# Its uBlock Origin is another, which reads its lists as it first starts.
send window 2
send click $page_point
reload_until "$blocked" "the private window didn't block the ad"
../tools/window-screenshot.sh "$pid" "$out/adblock-private.png" all || fail "no window to capture"

quit
# Its housekeeping timers that re-armed themselves, waking an idle browser, as the app ships it
# (#252): fetch-ublock.sh takes them out.
if [ -d "$app/ublock" ]; then
  ublock_js="$app/ublock/js"
else
  ublock_js="$(echo "$app"/Contents/Frameworks/*Framework.framework)/Resources/ublock/js"
fi
rearming=$(cat "$ublock_js/tab.js" "$ublock_js/traffic.js" | grep -cxF \
  -e '        gcTimer.on(30011);' -e '            this.gcTimer.on(gcPeriod);' \
  -e '        pageStoreJanitorTimer.on(pageStoreJanitorPeriod);' || true)
[ "$rearming" = 0 ] || fail "uBlock Origin ships with $rearming timers that wake an idle browser"
echo "adblock: uBlock Origin timers that wake an idle browser: $rearming"
# The version ublock.ini pins, which fetch-ublock.sh checked against its sha256.
pinned_version=$(git config -f ublock.ini ublock.version)
shipped_version=$(sed -n 's/^ *"version": "\([^"]*\)",*$/\1/p' "$ublock_js/../manifest.json")
[ "$shipped_version" = "$pinned_version" ] ||
  fail "the app ships uBlock Origin $shipped_version, not ublock.ini's $pinned_version"
# It trusts no site to show its ads, YouTube neither, as Arc's did: by default it trusts only
# extension pages, and the core's managed settings add none.
trusted=$(sed -n '/^ *netWhitelistDefault: \[$/,/^ *\],$/p' "$ublock_js/background.js")
[ -n "$trusted" ] || fail "the run found no trusted sites in uBlock Origin's background.js"
echo "$trusted" | grep -qi youtube && fail "uBlock Origin trusts YouTube by default: $trusted"

# Diagnostic for #744: how long after launch an existing profile starts blocking.
printf '%s' '<title>probe</title><script>
let n = 0;
const probe = () => {
  const sent = Date.now();
  fetch("check-ad.js?n=" + n++, {cache: "no-store"}).then(() => setTimeout(probe, 10), () => {
    document.title = "probe " + location.search.slice(1) + " blocked " + sent + " page " + Math.round(performance.timeOrigin) + " tries " + n;
  });
};
probe();
</script>' > "$out/site/probe.html"
for i in 1 2 3 4; do
  log="$out/adblock-probe-$i.log"
  step=0
  launched=$(python3 -I -c 'import time; print(time.time())')
  BROWSER_DRIVE=1 BROWSER_URL="${site}probe.html?$i" BROWSER_PROFILE_DIR="$profile_dir" \
    "$browser" --ad-block-filters="$filters" --enable-logging=stderr --v=0 \
    --vmodule='*extension_host*=2,*process_manager*=2,*background*=2,*extension_registrar*=2' \
    < "$out/drive" > "$log" 2>&1 &
  pid=$!
  exec 3> "$out/drive"
  await "^browser: title [0-9]* probe $i blocked " || fail "probe $i: never blocked"
  line=$(grep "^browser: title [0-9]* probe $i blocked " "$log" | head -1 | sed "s/ probe $i / probe /")
  python3 -I -c 'import sys
t = float(sys.argv[1]) * 1000
w = sys.argv[2].split()
print("adblock probe " + sys.argv[3] + ": launch to first page %d ms, launch to blocking %d ms, page unblocked for %d ms, %s tries" % (int(w[7]) - t, int(w[5]) - t, int(w[5]) - int(w[7]), w[9]))' "$launched" "$line" "$((i + 1))"
  # Diag (#744): the first stays until uBlock Origin has written its selfie, 53 s after its lists.
  if [ "$i" = 1 ]; then
    for _ in 1 2 3 4 5 6; do await 'Filtering engine selfie created' && break; done
    # And until it has saved the registry that names the selfie, 3 s later.
    for _ in $(seq 100); do
      sed -n '/Filtering engine selfie created/,$p' "$log" | grep -q 'cacheStorage.set: assetCacheRegistry' && break
      sleep 0.1
    done
  fi
  python3 -I -c 'import re, sys
t = float(sys.argv[1]) * 1000
for line in open(sys.argv[2], errors="replace"):
    m = re.search(r"\[uBO\] (\d+) (.*?)\", source", line)
    if m:
        text = re.sub(r"\b1\d{12}\b", lambda n: "at %d ms" % (int(n.group(0)) - t), m.group(2))
        print("  ublock %d %6d ms: %s" % (int(sys.argv[3]), int(m.group(1)) - t, text[:150]))' \
    "$launched" "$log" "$((i + 1))"
  grep -c 'CONSOLE' "$log" | sed "s/^/  console lines in launch $((i + 1)): /"
  quit
done

# An existing profile blocks too: launched again on it, uBlock Origin reads the lists it kept. The
# run reports whether that launch's first page was blocked before any reload.
log="$out/adblock-again.log"
step=0
BROWSER_DRIVE=1 BROWSER_URL="$ads?again" BROWSER_PROFILE_DIR="$profile_dir" \
  "$browser" --ad-block-filters="$filters" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await "^browser: loaded [0-9]* $ads?again\$" ||
  fail "launched again, the app didn't load the ad page"
tab=$(sed -n "s|^browser: loaded \([0-9]*\) $ads?again\$|\1|p" "$log" | head -1)
await "^browser: title $tab ad script " || fail "launched again, the ad page didn't report"
await "^browser: extensions menu .*$ublock\$" ||
  fail "launched again, uBlock Origin wasn't in the menu: $(grep '^browser: extensions' "$log")"
first_page=$(grep "^browser: title $tab ad script " "$log" | head -1 | cut -d' ' -f4-)
echo "adblock: an existing profile's first page, before a reload: $first_page"
send click $page_point
reload_until "$blocked" "launched again on its profile, the page didn't block its ad"
../tools/window-screenshot.sh "$pid" "$out/adblock-existing.png" || fail "no window to capture"
quit
echo "ok: a new profile blocks ads from its first page: uBlock Origin is in the Extensions menu, and a filter given through its managed settings blocks the page's ad script and hides its ad, with uBlock Origin in the menu all along; its own lists take fbclid out of an address. The Site Controls' Block ads and trackers, first in its rows, turns it off for the site and on again, reloading the page, and Settings lists the site while it's off, with a Turn On that lets it block there again; Settings' turns it off everywhere, taking it out of the Extensions menu and the Site Controls, and on again. A private window blocks too, and uBlock Origin ships at ublock.ini's $pinned_version without the timers that woke an idle browser, trusting no site, YouTube neither. It pins at the URL field with no Remove in its menu, and launched again, the existing profile blocks (its first page: $first_page). The first launch blocked within $first_block s. Windows: $out/adblock-blocked.png, $out/adblock-pinned.png, $out/adblock-off-for-site.png, $out/adblock-off-sites.png, $out/adblock-settings.png, $out/adblock-private.png, $out/adblock-existing.png"
