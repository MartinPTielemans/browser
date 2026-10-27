#!/bin/bash
# Builds the Linux engine in the tree prepare.sh made: chrome, with the core in it, and what it
# runs with. The first build takes hours; after that a change recompiles what it touches and
# relinks chrome.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: build.sh <work directory>" >&2 && exit 2; }
out=$1/src/out/Default
# GN's scripts read the core count too, into every link command, so a BUILD.gn change is
# regenerated before the cap below, as engine/build.sh does on the Mac.
[ -f "$out/build.ninja" ] || (cd "$1/src" && out/Default/gn gen out/Default)
ninja -C "$out" build.ninja >/dev/null
# Two workers for each of Chromium's Python generators, and a job per 3 GB of memory, or BUILD_JOBS
# jobs, as on the Mac (engine/build.sh says why).
export PYTHON_CPU_COUNT=2
jobs=${BUILD_JOBS:-$(($(awk '/^MemTotal:/ { print $2 }' /proc/meminfo) / (3 << 20)))}
exec ninja -C "$out" -j "$jobs" chrome chrome_crashpad_handler
