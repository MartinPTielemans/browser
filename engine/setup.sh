#!/bin/bash
# Prepares the engine's tree in .deps/src: Chromium at the version pins.ini names, with
# ungoogled-chromium's and ungoogled-chromium-macos's pruning, patches and domain substitution,
# their toolchain, then our series, then `gn gen`. The first run downloads about 3 GB and takes
# about 25 GB. Each stage that changes the tree runs once; running this again applies new
# patches from the series and regenerates the build files. `setup.sh dev` also prepares the dev
# build in out/Dev, which adds args.dev.gn.
set -euo pipefail
cd "$(dirname "$0")"
engine=$PWD
deps=$engine/.deps
ucm=$deps/ungoogled-chromium-macos
uc=$ucm/ungoogled-chromium
src=$deps/src
case ${1:-} in
  "") config=Release extra_args= ;;
  dev) config=Dev extra_args=args.dev.gn ;;
  *) echo "usage: setup.sh [dev]" >&2 && exit 2 ;;
esac
out=$src/out/$config
gn=$src/out/Release/gn
stamps=$deps/stamps
. ./tree.sh

version=$(git config -f pins.ini chromium.version)
uc_tag=$(git config -f pins.ini ungoogled-chromium.tag)
ucm_tag=$(git config -f pins.ini ungoogled-chromium-macos.tag)

fail() {
  echo "setup: $*" >&2
  exit 1
}

for tool in ninja go esbuild tsc greadlink uv node; do
  command -v "$tool" >/dev/null || fail "$tool is missing: brew install ninja go esbuild typescript coreutils uv node"
done
xcrun metal --version >/dev/null 2>&1 ||
  fail "the Metal toolchain is missing: xcodebuild -downloadComponent MetalToolchain"

mkdir -p "$deps/bin" "$deps/downloads" "$stamps"
# Spotlight would index millions of source files.
touch "$deps/.metadata_never_index"
# ungoogled-chromium-macos builds with Python 3.13, and Chromium's scripts aren't tested on newer.
uv python install -q 3.13
ln -sf "$(uv python find 3.13)" "$deps/bin/python3"
export PATH=$deps/bin:$PATH

# Runs a stage that changes the tree, once. One that stopped halfway leaves a tree no stage can
# trust, so the run stops.
stage() {
  local name=$1
  shift
  [ -e "$stamps/$name.done" ] && return
  [ -e "$stamps/$name.started" ] &&
    fail "the $name stage stopped halfway. Start over with: rm -rf engine/.deps/src engine/.deps/stamps"
  echo "setup: $name" >&2
  touch "$stamps/$name.started"
  "$@"
  mv "$stamps/$name.started" "$stamps/$name.done"
}

fetch_ungoogled() {
  rm -rf "$ucm"
  git clone -q -c advice.detachedHead=false --depth 1 --branch "$ucm_tag" --recurse-submodules \
    --shallow-submodules https://github.com/ungoogled-software/ungoogled-chromium-macos.git "$ucm"
  local pinned
  pinned=$(git ls-remote https://github.com/ungoogled-software/ungoogled-chromium.git \
    "refs/tags/$uc_tag^{}" | cut -f1)
  [ "$(git -C "$uc" rev-parse HEAD)" = "$pinned" ] ||
    fail "ungoogled-chromium-macos $ucm_tag doesn't build ungoogled-chromium $uc_tag"
  [ "$(cat "$uc/chromium_version.txt")" = "$version" ] ||
    fail "ungoogled-chromium $uc_tag is for Chromium $(cat "$uc/chromium_version.txt"), not $version"
  # Their scripts work in build/ beside themselves.
  mkdir -p "$ucm/build" "$src"
  ln -s "$src" "$ucm/build/src"
  ln -s "$deps/downloads" "$ucm/build/download_cache"
}

stage ungoogled fetch_ungoogled
stage source "$ucm/retrieve_and_unpack_resource.py" -d -g arm64
stage prune python3 "$uc/utils/prune_binaries.py" "$src" "$uc/pruning.list"
stage patches python3 "$uc/utils/patches.py" apply "$src" "$uc/patches" "$ucm/patches"
stage domains python3 "$uc/utils/domain_substitution.py" apply \
  -r "$uc/domain_regex.list" -f "$uc/domain_substitution.list" "$src"
stage agents remove_agent_instructions "$src"
stage toolchain "$ucm/retrieve_and_unpack_resource.py" -p arm64

# The profile Chrome's optimizer uses on Apple silicon. The source tarball has only Linux's;
# ungoogled-chromium-macos's git checkout downloads it from the same place.
pgo=$(cat "$src/chrome/build/mac-arm.pgo.txt")
if [ ! -e "$deps/downloads/$pgo" ]; then
  curl -fL --retry 3 -C - -o "$deps/downloads/$pgo.partial" \
    "https://storage.googleapis.com/chromium-optimization-profiles/pgo_profiles/$pgo"
  mv "$deps/downloads/$pgo.partial" "$deps/downloads/$pgo"
fi
ln -f "$deps/downloads/$pgo" "$src/chrome/build/pgo_profiles/$pgo"

# The SDK Chromium's official builds use. The pinned LLVM's linker can't read newer SDKs' stubs
# (the macOS 27 SDK lists arm64e.x1 targets it doesn't know).
sdk_version=$(sed -n 's/^ *mac_sdk_official_version = "\(.*\)"$/\1/p' "$src/build/config/mac/mac_sdk.gni")
sdk=
for dir in "$(xcode-select -p)/Platforms/MacOSX.platform/Developer/SDKs" \
  /Library/Developer/CommandLineTools/SDKs; do
  [ -d "$dir/MacOSX$sdk_version.sdk" ] && sdk=$dir/MacOSX$sdk_version.sdk && break
done
[ -n "$sdk" ] || fail "Chromium $version builds with the macOS $sdk_version SDK, which isn't installed.
Xcode $sdk_version or its Command Line Tools have it: developer.apple.com/download/all"
export SDKROOT=$sdk

# The build's own tools. They don't change the tree, so they're built when missing, not staged.
mkdir -p "$out" "$(dirname "$gn")"
cd "$src"
[ -x "$gn" ] || ./tools/gn/bootstrap/bootstrap.py -o "$gn" --skip-generate-buildfiles
[ -x third_party/rust-toolchain/bin/bindgen ] || ./tools/rust/build_bindgen.py --skip-test
cd "$engine"

# DevTools builds with Homebrew's esbuild through a module of the same version
# (ungoogled-chromium-macos's fix-esbuild patch).
esbuild_version=$(esbuild --version)
esbuild_module=$deps/esbuild-node/node_modules/esbuild
if [ "$(node -p 'require(process.argv[1]).version' "$esbuild_module/package.json" 2>/dev/null)" != "$esbuild_version" ]; then
  npm install -s --prefix "$deps/esbuild-node" --no-save --package-lock=false --no-bin-links \
    --ignore-scripts --omit=optional --no-audit --no-fund "esbuild@$esbuild_version"
fi
ln -sfn "$esbuild_module" "$src/third_party/devtools-frontend/src/node_modules/esbuild"

apply_series "$src"

# GN wants the SDK inside the output directory, so it's linked there, as Chromium links Xcode's.
mkdir -p "$out/sdk/xcode_links"
ln -sfn "$sdk" "$out/sdk/xcode_links/$(basename "$sdk")"
{
  cat "$uc/flags.gn" "$ucm/flags.macos.gn" args.gn $extra_args
  echo "mac_sdk_path = \"//out/$config/sdk/xcode_links/$(basename "$sdk")\""
} >"$out/args.gn"
(cd "$src" && "$gn" gen "$out" --fail-on-unused-args)
echo "setup: ready. Build with engine/build.sh ${1:-}" >&2
