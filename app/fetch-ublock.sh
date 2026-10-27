#!/bin/bash
# Downloads the uBlock Origin ublock.ini names, checked against its hash, into .deps/ublock, adds
# our key to its manifest so its id doesn't depend on where the app is, takes its housekeeping off
# timers, and prints its folder.
# Nothing in the download runs here: it's unzipped into a folder of its own and only read.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "fetch-ublock: $*" >&2
  exit 1
}

version=$(git config -f ublock.ini ublock.version)
url=$(git config -f ublock.ini ublock.url)
sha256=$(git config -f ublock.ini ublock.sha256)
key=$(git config -f ublock.ini ublock.key)

dir=$PWD/.deps/ublock/$sha256
if [ ! -f "$dir/manifest.json" ] || ! grep -qF "\"key\": \"$key\"" "$dir/manifest.json" ||
  ! grep -qF ilo-commit "$dir/js/tab.js"; then
  mkdir -p .deps/ublock
  rm -rf "$dir" "$dir.part" "$dir.zip"
  echo "fetch-ublock: downloading uBlock Origin $version" >&2
  curl -fL --progress-bar -o "$dir.zip" "$url"
  echo "$sha256  $dir.zip" | shasum -a 256 -c --status ||
    { rm -f "$dir.zip"; fail "the download doesn't match ublock.ini's sha256."; }
  mkdir "$dir.part"
  unzip -q "$dir.zip" -d "$dir.part"
  rm "$dir.zip"
  manifest=$dir.part/uBlock0.chromium/manifest.json
  [ "$(head -n 1 "$manifest")" = "{" ] || fail "the manifest doesn't start as uBlock Origin's did."
  awk -v key="$key" 'NR == 1 { print; print "  \"key\": \"" key "\","; next } { print }' \
    "$manifest" > "$dir.part/manifest.json"
  mv "$dir.part/manifest.json" "$manifest"
  # Its housekeeping timers woke an idle browser in every profile (#252), so its tab contexts'
  # 10-minute and page stores' 15-minute cleanups and its hourly filter list update run once due
  # at a page's commit, when the browser is awake anyway, and its tabless requests' cache clears
  # once, 30 s after it fills, rather than every 30 s. Each edit replaces one line found once.
  edit() {
    local file=$dir.part/uBlock0.chromium/js/$1
    old=$2 new=$3 awk '$0 == ENVIRON["old"] { print ENVIRON["new"]; n++; next } { print }
      END { exit (n != 1) }' "$file" > "$file.new" ||
      fail "uBlock Origin's js/$1 no longer has this line once, so its timers need a new look: $2"
    mv "$file.new" "$file"
  }
  edit tab.js '            µb.tabContextManager.commit(tabId, url);' \
'            µb.tabContextManager.commit(tabId, url);
            self.dispatchEvent(new Event("ilo-commit"));'
  edit tab.js '    const gcPeriod = 10 * 60 * 1000;' \
'    const gcPeriod = 10 * 60 * 1000;
    const gcDue = new Map();
    self.addEventListener("ilo-commit", ( ) => {
        const now = Date.now();
        for ( const [ context, at ] of gcDue ) {
            if ( at > now ) { continue; }
            gcDue.delete(context);
            context.onGC();
        }
    });'
  edit tab.js '        this.gcTimer.on(gcPeriod);' '        gcDue.set(this, Date.now() + gcPeriod);'
  edit tab.js '            this.gcTimer.on(gcPeriod);' '            gcDue.set(this, Date.now() + gcPeriod);'
  edit tab.js '        pageStoreJanitorTimer.on(pageStoreJanitorPeriod);' ''
  edit tab.js '    pageStoreJanitorTimer.on(pageStoreJanitorPeriod);' \
'    let pageStoreJanitorAt = Date.now() + 15 * 60 * 1000;
    self.addEventListener("ilo-commit", ( ) => {
        if ( Date.now() < pageStoreJanitorAt ) { return; }
        pageStoreJanitorAt = Date.now() + 15 * 60 * 1000;
        pageStoreJanitor();
    });'
  edit storage.js '    let next = 0;' \
'    let next = 0;
    self.addEventListener("ilo-commit", ( ) => {
        if ( next === 0 || Date.now() < next ) { return; }
        launchTimer.on(1, µb.hiddenSettings.autoUpdateAssetFetchPeriod * 1000 || 60000);
    });'
  edit storage.js '        launchTimer.on(updateDelay, fetchDelay);' \
'        if ( Boolean(details.updateDelay) === false ) { return; }
        launchTimer.on(updateDelay, fetchDelay);'
  edit traffic.js '        gcTimer.on(30011);' '        reset();'
  mv "$dir.part/uBlock0.chromium" "$dir"
  rm -rf "$dir.part"
  # Earlier versions go.
  find .deps/ublock -mindepth 1 -maxdepth 1 ! -name "$sha256" -exec rm -rf {} +
fi
echo "$dir"
