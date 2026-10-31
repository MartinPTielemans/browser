#!/bin/bash
# Builds the Linux engine in the tree prepare.sh made: chrome, with the core in it, and what it
# runs with, or the targets named after the tree (test.sh names the core's tests). The first build
# takes hours; after that a change recompiles what it touches and relinks chrome.
set -euo pipefail
[ $# -ge 1 ] || { echo "usage: build.sh <work directory> [target...]" >&2 && exit 2; }
out=$1/src/out/Default
targets=("${@:2}")
[ ${#targets[@]} -gt 0 ] || targets=(chrome chrome_crashpad_handler)
# The checkout's series and core, as engine/build.sh takes them on the Mac.
. "$(dirname "$0")/../tree.sh"
apply_series "$1/src"
# GN's scripts read the core count too, into every link command, so a BUILD.gn change is
# regenerated before the cap below, as engine/build.sh does on the Mac.
[ -f "$out/build.ninja" ] || (cd "$1/src" && out/Default/gn gen out/Default)
ninja -C "$out" build.ninja >/dev/null
# Two workers for each of Chromium's Python generators, and a job per 3 GB of memory, or BUILD_JOBS
# jobs, as on the Mac (engine/build.sh says why).
export PYTHON_CPU_COUNT=2
jobs=${BUILD_JOBS:-$(($(awk '/^MemTotal:/ { print $2 }' /proc/meminfo) / (3 << 20)))}
exec ninja -C "$out" -j "$jobs" "${targets[@]}"
