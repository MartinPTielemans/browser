#!/bin/bash
# Compiles the core on Linux, as CI does (.github/workflows/core-linux.yml), to keep per-OS code
# in the platform layer. It prepares Chromium as the Linux engine is built (linux/prepare.sh), then
# builds only //browser_core: the core's own files and the headers they include that the build
# generates. Nothing links. Takes the directory to work in, which needs about 40 GB.
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: check-linux.sh <work directory>" >&2 && exit 2; }

src=$("$(dirname "$0")/linux/prepare.sh" "$1")
cd "$src"
# The core as the root, so the build has only it and what it depends on. Arguments for targets
# outside that are unused, so they don't fail it.
out/Default/gn gen out/Default --root-target=//browser_core
# The core's own objects. The browser_core target would compile everything it depends on as well,
# most of Chrome.
objects=$(ninja -C out/Default -t targets all | sed -n 's#^\(obj/browser_core/[^:]*\.o\):.*#\1#p')
[ -n "$objects" ] || { echo "check-linux: the build has no objects for //browser_core" >&2; exit 1; }
# shellcheck disable=SC2086 # one object per word
ninja -C out/Default $objects
