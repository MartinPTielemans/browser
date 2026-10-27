#!/bin/sh
# Screenshots one process's main window with its child windows (menus, the command bar) drawn
# over it, as on screen, but nothing from other apps. A screen capture would include whatever
# else is open on the owner's Mac. With "all", it takes every window of the process on that display,
# such as two browser windows, in one picture.
# Usage: tools/window-screenshot.sh <pid> <output.png> [all]
# window-screenshot.swift does it. Interpreting it took about half a second a capture, so it's
# compiled the first time, into ~/.cache, and again only once it changes. With "compile" alone,
# the script only does that, as check.sh does before its runs, so no run's time includes it.
set -e
[ "$*" = compile ] || [ $# -eq 2 ] || [ $# -eq 3 ] && [ "${3:-all}" = all ] ||
  { echo "usage: $0 <pid> <output.png> [all] | compile" >&2; exit 2; }
source="$(dirname "$0")/window-screenshot.swift"
tool="${XDG_CACHE_HOME:-$HOME/.cache}/browser-tools/window-screenshot-$(cksum < "$source" | cut -d' ' -f1)"
if [ ! -x "$tool" ]; then
  mkdir -p "$(dirname "$tool")"
  # Under another name until it's whole, for a capture that starts meanwhile.
  swiftc -O "$source" -o "$tool.$$"
  mv "$tool.$$" "$tool"
fi
[ "$*" != compile ] || exit 0
PID="$1" OUT="$2" ALL="${3:-}" exec "$tool"
