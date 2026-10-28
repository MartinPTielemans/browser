#!/bin/bash
# Downloads the prebuilt engine prebuilt.ini names, checked against its hash, into .deps/prebuilt,
# and prints the path of its Chromium.app. publish.sh makes one.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "fetch: $*" >&2
  exit 1
}

[ -e prebuilt.ini ] || fail "no prebuilt engine is published yet. Build one with engine/build.sh."
[ "$(uname -sm)" = "Darwin arm64" ] || fail "the prebuilt engine is for Macs with Apple silicon."
commit=$(git config -f prebuilt.ini prebuilt.commit)
url=$(git config -f prebuilt.ini prebuilt.url)
sha256=$(git config -f prebuilt.ini prebuilt.sha256)

# The engine holds the core of the commit it was built from. A different header would bind the app
# to functions or layouts the engine doesn't have.
git cat-file -e "$commit^{commit}" 2>/dev/null ||
  fail "the prebuilt engine's commit ${commit:0:12} isn't in this clone. Fetch the history with git fetch --unshallow."
git diff --quiet "$commit" -- ../core/public/browser_core.h ||
  fail "core/public/browser_core.h has changed since the prebuilt engine (${commit:0:12}). Build the engine with engine/build.sh."

dir=$PWD/.deps/prebuilt/$sha256
if [ ! -d "$dir" ]; then
  mkdir -p .deps/prebuilt
  archive=$dir.tar.xz
  echo "fetch: downloading the prebuilt engine (${commit:0:12})" >&2
  curl -fL --progress-bar -o "$archive" "$url"
  echo "$sha256  $archive" | shasum -a 256 -c --status ||
    { rm -f "$archive"; fail "the download doesn't match prebuilt.ini's sha256."; }
  rm -rf "$dir.part"
  mkdir "$dir.part"
  tar -xf "$archive" -C "$dir.part"
  rm "$archive"
  mv "$dir.part" "$dir"
  # Earlier prebuilt engines go. A built app keeps its own copy.
  find .deps/prebuilt -mindepth 1 -maxdepth 1 ! -name "$sha256" -exec rm -rf {} +
fi
echo "$dir/Chromium.app"
