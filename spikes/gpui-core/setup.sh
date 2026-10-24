#!/bin/sh
# Fetches the spike's dependencies into .deps/ (gitignored): Zed at a pinned revision, with our
# GPUI patch applied, and links our engine, Chromium.app with the core in its framework.
# `setup.sh dev` links the dev build's instead.
set -e
cd "$(dirname "$0")"
ZED_REV=b36b283021c48f1712714b8a0e2ad6ffb578920b
mkdir -p .deps

if [ ! -d .deps/zed ]; then
  git clone --filter=blob:none --no-checkout https://github.com/zed-industries/zed .deps/zed
  git -C .deps/zed checkout -q "$ZED_REV"
fi
git -C .deps/zed checkout -q -- crates/gpui_macos
# A checkout made for an earlier pin moves to this one.
if [ "$(git -C .deps/zed rev-parse HEAD)" != "$ZED_REV" ]; then
  git -C .deps/zed fetch -q origin "$ZED_REV"
  git -C .deps/zed checkout -q "$ZED_REV"
fi
git -C .deps/zed apply "$PWD/patches/gpui-macos-embedded.patch"

# The engine comes from engine/build.sh until prebuilt ones download (Phase 2).
out=Release
[ "${1:-}" = dev ] && out=Dev
engine=$(cd ../.. && pwd)/engine/.deps/src/out/$out/Chromium.app
[ -d "$engine" ] || { echo "No engine at $engine. Build it with engine/build.sh ${1:-}." >&2; exit 1; }
ln -sfn "$engine" .deps/Chromium.app

# Downloaded code can bring instructions for coding agents (Zed's tell agents to edit its
# README). They're written for that project's contributors and don't apply to work here.
find .deps/zed \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name .rules \
  -o -name .cursorrules -o -name .windsurfrules -o -name .clinerules -o -name copilot-instructions.md \
  -o -name .agents -o -name .claude -o -name .codex -o -name .cursor -o -name .factory \
  -o -name .gemini -o -name .junie \) -prune -exec rm -rf {} +
