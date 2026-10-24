#!/bin/bash
# Compiles the core on Linux, as CI does (.github/workflows/core-linux.yml), to keep per-OS code
# in the platform layer. It prepares Chromium at the version pins.ini names with
# ungoogled-chromium-portablelinux's scripts, the way its x64 releases are built, applies our
# series, then builds only //browser_core: the core's own files and the headers they include that
# the build generates. Nothing links. Takes the directory to work in, which needs about 40 GB.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: check-linux.sh <work directory>" >&2 && exit 2; }
work=$1
. "$(dirname "$0")/tree.sh"
ucl=$work/ungoogled-chromium-portablelinux

fail() {
  echo "check-linux: $*" >&2
  exit 1
}

for tool in ninja node gperf go clang-format; do
  command -v "$tool" >/dev/null || fail "$tool is missing"
done
# Where portablelinux's scripts expect npm's global esbuild.
[ -d /usr/lib/node_modules/esbuild ] || fail "esbuild is missing: sudo npm install -g --prefix /usr esbuild"

version=$(git config -f "$engine/pins.ini" chromium.version)
uc_tag=$(git config -f "$engine/pins.ini" ungoogled-chromium.tag)
ucl_tag=$(git config -f "$engine/pins.ini" ungoogled-chromium-portablelinux.tag)
if [ ! -d "$ucl" ]; then
  git clone -q -c advice.detachedHead=false --depth 1 --branch "$ucl_tag" --recurse-submodules \
    --shallow-submodules https://github.com/ungoogled-software/ungoogled-chromium-portablelinux.git "$ucl"
fi
pinned=$(git ls-remote https://github.com/ungoogled-software/ungoogled-chromium.git \
  "refs/tags/$uc_tag^{}" | cut -f1)
[ "$(git -C "$ucl/ungoogled-chromium" rev-parse HEAD)" = "$pinned" ] ||
  fail "ungoogled-chromium-portablelinux $ucl_tag doesn't build ungoogled-chromium $uc_tag"
[ "$(cat "$ucl/ungoogled-chromium/chromium_version.txt")" = "$version" ] ||
  fail "ungoogled-chromium $uc_tag is for Chromium $(cat "$ucl/ungoogled-chromium/chromium_version.txt"), not $version"

# Their scripts/build.sh, up to its build. Each step that changes the tree runs once.
. "$ucl/scripts/shared.sh"
setup_paths
fetch_sources false false
apply_patches
apply_domsub
write_gn_args
fix_tool_downloading
setup_toolchain

remove_agent_instructions "$_src_dir"
apply_series "$_src_dir"

cd "$_src_dir"
[ -x out/Default/gn ] || ./tools/gn/bootstrap/bootstrap.py -o out/Default/gn --skip-generate-buildfiles
# The core as the root, so the build has only it and what it depends on. Arguments for targets
# outside that are unused, so they don't fail it.
out/Default/gn gen out/Default --root-target=//browser_core
ninja -C out/Default browser_core
