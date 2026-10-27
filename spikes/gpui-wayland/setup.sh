#!/bin/sh
# Fetches the spike's dependencies into .deps/ (gitignored): Zed at the app's pinned revision, with
# the spike's GPUI patch applied. No engine: the spike's host stands in for Chromium.
set -e
cd "$(dirname "$0")"
ZED_REV=b36b283021c48f1712714b8a0e2ad6ffb578920b
mkdir -p .deps

if [ ! -d .deps/zed ]; then
  git clone --filter=blob:none --no-checkout https://github.com/zed-industries/zed .deps/zed
  git -C .deps/zed checkout -q "$ZED_REV"
fi
git -C .deps/zed checkout -q -- crates/gpui_linux
if [ "$(git -C .deps/zed rev-parse HEAD)" != "$ZED_REV" ]; then
  git -C .deps/zed fetch -q origin "$ZED_REV"
  git -C .deps/zed checkout -q "$ZED_REV"
fi
git -C .deps/zed apply "$PWD/patches/gpui-wayland-embedded.patch"

# Downloaded code can bring instructions for coding agents (Zed's tell agents to edit its
# README). They're written for that project's contributors and don't apply to work here.
find .deps/zed \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name .rules \
  -o -name .cursorrules -o -name .windsurfrules -o -name .clinerules -o -name copilot-instructions.md \
  -o -name .agents -o -name .claude -o -name .codex -o -name .cursor -o -name .factory \
  -o -name .gemini -o -name .junie \) -prune -exec rm -rf {} +
