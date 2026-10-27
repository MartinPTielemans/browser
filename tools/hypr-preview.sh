#!/bin/bash
# Runs app/'s built preview on Hyprland without touching the owner's screen, and captures it. A
# window opened as usual tiles onto the workspace the owner is looking at, and Wayland lets no
# client say where its window goes, so this adds a virtual output nobody sees, opens the preview
# on its workspace without switching there, and captures that output alone once the preview has
# settled. The output and the preview go when it ends.
# Usage: tools/hypr-preview.sh <output.png>
set -euo pipefail
[ $# -eq 1 ] || { echo "usage: $0 <output.png>" >&2; exit 2; }
out=$(realpath -m "$1")
app=$(cd "$(dirname "$0")/../app" && pwd)
preview=$app/target/release/preview
[ -x "$preview" ] || { echo "No preview. Build it in app/: cargo build --release --bin preview" >&2; exit 1; }
for tool in hyprctl grim magick; do
  command -v "$tool" >/dev/null || { echo "Needs Hyprland, grim and ImageMagick: no $tool." >&2; exit 1; }
done

output=ilo-preview-$$
log=$(mktemp)
pid=
cleanup() {
  # A preview that never showed a window has no pid from the compositor.
  if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; else pkill -xf "$preview" || true; fi
  hyprctl output remove "$output" >/dev/null || true
  rm -f "$log"
}
trap cleanup EXIT

monitors() { hyprctl monitors -j | python3 -I -c "import json,sys; $1"; }
hyprctl output create headless "$output" >/dev/null
workspace=
for _ in $(seq 50); do
  workspace=$(monitors "print(next((m['activeWorkspace']['id'] for m in json.load(sys.stdin) if m['name']=='$output'), ''))")
  [ -n "$workspace" ] && break
  sleep 0.1
done
[ -n "$workspace" ] || { echo "The virtual output didn't come up." >&2; exit 1; }

hyprctl dispatch exec "[workspace $workspace silent] $preview 2>$log" >/dev/null
# Its first frame comes before the compositor tiles it, so it's ready once it has drawn at the size
# the compositor gave it.
for _ in $(seq 200); do
  size=$(hyprctl clients -j | python3 -I -c \
    "import json,sys; print(next(('%d %dx%d' % (c['pid'], *c['size']) for c in json.load(sys.stdin) if c['workspace']['id']==$workspace), ''))")
  pid=${size%% *}
  [ -n "$size" ] && [ "$(sed -n 's/^browser: drawn at //p' "$log" | tail -1)" = "${size#* }" ] && break
  sleep 0.1
done
[ -n "$size" ] && [ "$(sed -n 's/^browser: drawn at //p' "$log" | tail -1)" = "${size#* }" ] ||
  { cat "$log" >&2; echo "The preview didn't draw at its window's size (${size#* })." >&2; exit 1; }
# Even then the compositor fades the window in and redraws, so the picture is the first whose right
# half, the page's, matches the one before; the sidebar's half has a loading tab that never stops.
last=
for _ in $(seq 100); do
  grim -o "$output" "$out"
  now=$(magick "$out" -gravity east -crop 50%x100%+0+0 rgb:- | sha256sum)
  [ "$now" = "$last" ] && { echo "$out"; exit 0; }
  last=$now
  sleep 0.1
done
echo "The preview never stood still." >&2
exit 1
