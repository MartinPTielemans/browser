#!/bin/bash
# Downloads the prebuilt engine, checked against its hash, into .deps/prebuilt, and prints the
# path of its Chromium.app, or on Linux and Windows its Chromium directory. It's the newest
# engine-mac, engine-linux or engine-windows release built from a commit in this checkout's
# history, which .github/workflows/engine-mac.yml, engine-linux.yml and engine-windows-cross.yml
# make. On the Mac with none, it's the one prebuilt.ini names, which publish.sh makes. It exits
# with 2 when no engine fits this checkout, which CI takes for a reason to skip, and 1 on any
# other failure.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "fetch: $*" >&2
  exit 1
}
none() {
  echo "fetch: $*" >&2
  exit 2
}

# The release engine-<platform>-<commit> built from the nearest commit in this history, into
# commit, url and sha256, if there's one.
tagged() {
  local platform=$1 asset=$2 repo tagged distance best=
  repo=$(git config -f prebuilt.ini prebuilt.url | sed -E 's#^(https://github.com/[^/]+/[^/]+)/.*#\1#')
  commit=
  # The release's tag is on the commit it was built from.
  while read -r tagged _; do
    git merge-base --is-ancestor "$tagged" HEAD 2>/dev/null || continue
    distance=$(git rev-list --count "$tagged..HEAD")
    if [ -z "$best" ] || [ "$distance" -lt "$best" ]; then
      commit=$tagged
      best=$distance
    fi
  done < <(git ls-remote --tags "$repo.git" "refs/tags/engine-$platform-*")
  [ -n "$commit" ] || return 1
  url=$repo/releases/download/engine-$platform-${commit:0:12}/$asset
  sha256=$(curl -fsSL "$url.sha256") || none "the release engine-$platform-${commit:0:12} has no engine yet."
}

case $(uname -sm) in
"Darwin arm64")
  [ -e prebuilt.ini ] || none "no prebuilt engine is published yet. Build one with engine/build.sh."
  if ! tagged mac Chromium-arm64.tar.xz; then
    commit=$(git config -f prebuilt.ini prebuilt.commit)
    url=$(git config -f prebuilt.ini prebuilt.url)
    sha256=$(git config -f prebuilt.ini prebuilt.sha256)
  fi
  app=Chromium.app
  sha256sum() { shasum -a 256 "$@"; }
  ;;
"Linux x86_64" | MINGW*" x86_64")
  case $(uname -s) in
  Linux) platform=linux asset=Chromium-linux-x64.tar.xz ;;
  *) platform=windows asset=Chromium-windows-x64.zip ;;
  esac
  tagged "$platform" "$asset" ||
    none "no prebuilt $platform engine was built from this checkout's history. Fetch the history with git fetch --unshallow, or run the engine-$platform workflow."
  app=Chromium
  ;;
*) fail "there's a prebuilt engine for Macs with Apple silicon, x86-64 Linux and x64 Windows only." ;;
esac

# The engine holds the core of the commit it was built from. A different header would bind the app
# to functions or layouts the engine doesn't have.
git cat-file -e "$commit^{commit}" 2>/dev/null ||
  fail "the prebuilt engine's commit ${commit:0:12} isn't in this clone. Fetch the history with git fetch --unshallow."
git diff --quiet "$commit" -- ../core/public/browser_core.h ||
  none "core/public/browser_core.h has changed since the prebuilt engine (${commit:0:12}). Build the engine with engine/build.sh."

dir=$PWD/.deps/prebuilt/$sha256
if [ ! -d "$dir" ]; then
  mkdir -p .deps/prebuilt
  name=${url##*/}
  archive=$dir.${name#*.}
  echo "fetch: downloading the prebuilt engine (${commit:0:12})" >&2
  curl -fL --progress-bar -o "$archive" "$url"
  echo "$sha256  $archive" | sha256sum -c --status ||
    { rm -f "$archive"; fail "the download doesn't match its sha256."; }
  rm -rf "$dir.part"
  mkdir "$dir.part"
  case $archive in
  *.zip) unzip -q "$archive" -d "$dir.part" ;;
  *) tar -xf "$archive" -C "$dir.part" ;;
  esac
  rm "$archive"
  mv "$dir.part" "$dir"
  # Earlier prebuilt engines go. A built app keeps its own copy.
  find .deps/prebuilt -mindepth 1 -maxdepth 1 ! -name "$sha256" -exec rm -rf {} +
fi
echo "$dir/$app"
