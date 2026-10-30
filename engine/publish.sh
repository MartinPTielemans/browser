#!/bin/bash
# Publishes the Release engine as the prebuilt one fetch.sh downloads: brings the build up to date
# with the committed tree, packs Chromium.app, uploads it as a release of this repository on
# GitHub, named after the commit, and rewrites prebuilt.ini. Commit prebuilt.ini after.
set -euo pipefail
cd "$(dirname "$0")"

fail() {
  echo "publish: $*" >&2
  exit 1
}

[ "$(uname -sm)" = "Darwin arm64" ] || fail "the prebuilt engine is built on a Mac with Apple silicon."
# The release names the commit as the engine's source, so the build can't hold anything else.
[ -z "$(git status --porcelain -- . ../core)" ] ||
  fail "engine/ or core/ has uncommitted changes. Commit them first."
commit=$(git rev-parse HEAD)
./setup.sh
./build.sh

tag=engine-${commit:0:12}
mkdir -p .deps/publish
archive=.deps/publish/Chromium-arm64.tar.xz
# Without the Mac's metadata, which tar would otherwise add as ._ files.
tar -cJf "$archive" --no-mac-metadata --options xz:threads=0 -C .deps/src/out/Release Chromium.app
sha256=$(shasum -a 256 "$archive" | cut -d' ' -f1)

# Never GitHub's Latest, which releases/latest/download/<name> links mean as ilo's own (#804).
gh release create "$tag" "$archive" --title "Engine ${commit:0:12}" --latest=false \
  --notes "The Release engine (Chromium $(git config -f pins.ini chromium.version), Apple silicon) built from $commit, which holds its source."
cat >prebuilt.ini <<EOF
; The prebuilt engine fetch.sh downloads, written by publish.sh: the Release Chromium.app built
; from this commit, with the core in its framework.
[prebuilt]
	commit = $commit
	url = $(gh repo view --json url -q .url)/releases/download/$tag/Chromium-arm64.tar.xz
	sha256 = $sha256
EOF
rm "$archive"
echo "publish: released $tag. Commit engine/prebuilt.ini."
