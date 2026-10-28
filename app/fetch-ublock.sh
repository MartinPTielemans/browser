#!/bin/bash
# Downloads the uBlock Origin ublock.ini names, checked against its hash, into .deps/ublock, adds
# our key to its manifest so its id doesn't depend on where the app is, takes out the timers that
# wake an idle browser (below), and prints its folder.
# Nothing in the download runs here: it's unzipped into a folder of its own and only read.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "fetch-ublock: $*" >&2
  exit 1
}

# What starts each file this changes, as GPL 3 asks of a changed file.
changed="// Changed by ilo's app/fetch-ublock.sh: its housekeeping doesn't wake an idle browser."

# Puts $3, which may be several lines or none, in place of the line $2 of uBlock Origin's file $1,
# and fails unless that line is there once, so a new release is read again before it ships.
edit() {
  local file=$dir.part/uBlock0.chromium/$1
  [ "$(grep -cxF -- "$2" "$file")" = 1 ] || fail "$1 no longer has the line: $2"
  awk -v old="$2" -v new="$3" -v changed="$changed" '
    NR == 1 && $0 != changed { print changed }
    $0 == old { if (new != "") print new; next }
    { print }' "$file" > "$file.new"
  mv "$file.new" "$file"
}

version=$(git config -f ublock.ini ublock.version)
url=$(git config -f ublock.ini ublock.url)
sha256=$(git config -f ublock.ini ublock.sha256)
key=$(git config -f ublock.ini ublock.key)

dir=$PWD/.deps/ublock/$sha256
if [ ! -f "$dir/manifest.json" ] || ! grep -qF "\"key\": \"$key\"" "$dir/manifest.json" ||
  [ "$(head -n 1 "$dir/js/tab.js")" != "$changed" ]; then
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
  # Its housekeeping timers re-armed themselves, so an idle browser woke every 30 s after a request
  # with no tab, every 10 minutes for each tab and every 15 for each profile, to free what a tab
  # closed or discarded unseen left behind (#252). Nothing the browser uses needs them: the lists
  # update on their own weekly alarm, and the cache and the counts save once after a change. So
  # each runs once after what gives it work: the tabless requests' pages 30 s after them, a tab's
  # context 10 minutes after it's made, and the page stores' sweep 5 s after a tab closes.
  edit js/traffic.js '        gcTimer.on(30011);' '        reset();'
  edit js/tab.js '            this.gcTimer.on(gcPeriod);' ''
  edit js/tab.js '        pageStoreJanitorTimer.on(pageStoreJanitorPeriod);' ''
  edit js/tab.js '    const pageStoreJanitorPeriod = { min: 15 };' \
    '    const pageStoreJanitorPeriod = { sec: 5 };'
  edit js/tab.js '    pageStoreJanitorTimer.on(pageStoreJanitorPeriod);' \
    '    µb.pageStoreJanitorSoon = ( ) => { pageStoreJanitorTimer.on(pageStoreJanitorPeriod); };'
  edit js/tab.js '        super.onClosed(tabId);' \
    '        super.onClosed(tabId);\n        µb.pageStoreJanitorSoon();'
  mv "$dir.part/uBlock0.chromium" "$dir"
  rm -rf "$dir.part"
  # Earlier versions go.
  find .deps/ublock -mindepth 1 -maxdepth 1 ! -name "$sha256" -exec rm -rf {} +
fi
echo "$dir"
