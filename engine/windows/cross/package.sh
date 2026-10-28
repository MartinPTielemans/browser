#!/bin/bash
# Packs the Windows engine build.sh built into <output>.zip, a Chromium directory that fetch.sh
# unpacks: the files Chromium's FILES.cfg lists for an official x64 build, as
# ungoogled-chromium-windows packs them, and chrome.dll.lib, which the app links (app/windows).
# Nothing of Microsoft's toolchain goes in but what FILES.cfg names from the SDK's redistributable
# files (D3DCompiler_47.dll), which Chrome ships too.
set -euo pipefail
[ $# -eq 2 ] || { echo "usage: package.sh <work directory> <output.zip>" >&2 && exit 2; }
src=$1/src
out=$src/out/Default
archive=$(realpath -m "$2")

fail() {
  echo "package: $*" >&2
  exit 1
}

# The app calls into chrome.dll through the C API, so one that doesn't export it can't run it.
nm=$src/third_party/llvm-build/Release+Asserts/bin/llvm-nm
symbols=$("$nm" "$out/chrome.dll.lib")
grep -qE '(^| )bc_main$' <<<"$symbols" ||
  fail "chrome.dll doesn't export the core's C API (core/BUILD.gn)"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
# A zip's top directory is named after it.
python3 "$1/ucw/ungoogled-chromium/utils/filescfg.py" -c "$src/chrome/tools/build/win/FILES.cfg" \
  --build-outputs "$out" --cpu-arch 64bit archive -o "$stage/Chromium.zip" -i "$out/chrome.dll.lib"
mv "$stage/Chromium.zip" "$archive"
sha256sum "$archive" | cut -d' ' -f1 >"$archive.sha256"
echo "$archive"
