#!/bin/bash
# Builds the Windows engine in the tree prepare.sh made: chrome.exe and chrome.dll, with the core in
# it, and what they run with. The first build takes hours; after that a change recompiles what it
# touches and relinks chrome.dll.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: build.sh <work directory>" >&2 && exit 2; }
out=$1/src/out/Default
# As linux/build.sh: a BUILD.gn change is regenerated before the cap below.
[ -f "$out/build.ninja" ] || (cd "$1/src" && out/Default/gn gen out/Default --fail-on-unused-args)
ninja -C "$out" build.ninja >/dev/null
export PYTHON_CPU_COUNT=2
jobs=${BUILD_JOBS:-$(($(awk '/^MemTotal:/ { print $2 }' /proc/meminfo) / (3 << 20)))}
exec ninja -C "$out" -j "$jobs" chrome
