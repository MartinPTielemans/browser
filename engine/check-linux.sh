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

ucl_tag=$(git config -f "$engine/pins.ini" ungoogled-chromium-portablelinux.tag)
if [ ! -d "$ucl" ]; then
  git clone -q -c advice.detachedHead=false --depth 1 --branch "$ucl_tag" --recurse-submodules \
    --shallow-submodules https://github.com/ungoogled-software/ungoogled-chromium-portablelinux.git "$ucl"
fi
check_ungoogled "$ucl/ungoogled-chromium" "ungoogled-chromium-portablelinux $ucl_tag"

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
# The core's own objects. The browser_core target would compile everything it depends on as well,
# most of Chrome.
objects=$(ninja -C out/Default -t targets all | sed -n 's#^\(obj/browser_core/[^:]*\.o\):.*#\1#p')
[ -n "$objects" ] || fail "the build has no objects for //browser_core"
# shellcheck disable=SC2086 # one object per word
ninja -C out/Default $objects
