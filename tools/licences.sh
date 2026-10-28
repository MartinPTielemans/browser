#!/bin/sh
# Puts what a package of ilo carries for its licences in <folder>: LICENSE, NOTICE and
# THIRD-PARTY.txt, the Rust crates the app links on <target> (tools/third-party.py). Chromium's own
# notices are the engine's, at chrome://credits. app/build.sh calls it for the Mac app and Linux's
# build/Browser; a package made from elsewhere calls it for its own folder.
# Usage: tools/licences.sh <folder> <Cargo.toml> <target triple> [cargo metadata flags...]
set -e
[ $# -ge 3 ] || { echo "usage: $0 <folder> <Cargo.toml> <target triple> [cargo metadata flags...]" >&2; exit 2; }
root=$(cd "$(dirname "$0")/.." && pwd)
folder=$1
shift
cp "$root/LICENSE" "$root/NOTICE" "$folder/"
python3 -I "$root/tools/third-party.py" "$@" > "$folder/THIRD-PARTY.txt"
