#!/bin/bash
# Downloads the uBlock Origin ublock.ini names, checked against its hash, into .deps/ublock, adds
# our key to its manifest so its id doesn't depend on where the app is, and prints its folder.
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
if [ ! -f "$dir/manifest.json" ] || ! grep -qF "\"key\": \"$key\"" "$dir/manifest.json"; then
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
  mv "$dir.part/uBlock0.chromium" "$dir"
  rm -rf "$dir.part"
  # Earlier versions go.
  find .deps/ublock -mindepth 1 -maxdepth 1 ! -name "$sha256" -exec rm -rf {} +
fi
echo "$dir"
