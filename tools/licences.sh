#!/bin/sh
# Puts what a package of ilo carries for its licences in <folder>: LICENSE, NOTICE and
# THIRD-PARTY.txt, the Rust crates the app links on <target> (tools/third-party.py). Chromium's own
# notices are the engine's, at chrome://credits. app/build.sh calls it for the Mac app and the
# others' build/Browser, and app/platform/windows/package/package.sh for the Windows packages.
# Usage: tools/licences.sh <folder> <Cargo.toml> <target triple> [cargo metadata flags...]
set -e
[ $# -ge 3 ] || { echo "usage: $0 <folder> <Cargo.toml> <target triple> [cargo metadata flags...]" >&2; exit 2; }
root=$(cd "$(dirname "$0")/.." && pwd)
folder=$1
shift
cp "$root/LICENSE" "$root/NOTICE" "$folder/"
# Windows's Python is python alone, and writes in the system's code page unless told UTF-8.
python=python3
command -v python3 > /dev/null || python=python
"$python" -I -X utf8 "$root/tools/third-party.py" "$@" > "$folder/THIRD-PARTY.txt"
