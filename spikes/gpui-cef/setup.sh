#!/bin/sh
# Fetches the spike's dependencies into .deps/ (gitignored): Zed at a pinned revision, with
# our GPUI patch applied, and a CEF binary distribution with its C++ wrapper built.
set -e
cd "$(dirname "$0")"
ZED_REV=b36b283021c48f1712714b8a0e2ad6ffb578920b
CEF=cef_binary_156.0.8+g426920c+chromium-156.0.8078.48_macosarm64_minimal
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

# A distribution of another version, or none, is replaced.
cef_version=$(echo "$CEF" | sed 's/^cef_binary_\(.*\)_macos.*/\1/')
if ! grep -qF "CEF_VERSION \"$cef_version\"" .deps/cef/include/cef_version.h 2>/dev/null; then
  rm -rf .deps/cef
  curl -fL "https://cef-builds.spotifycdn.com/$(echo "$CEF" | sed 's/+/%2B/g').tar.bz2" -o .deps/cef.tar.bz2
  tar -xjf .deps/cef.tar.bz2 -C .deps
  mv ".deps/$CEF" .deps/cef
  rm .deps/cef.tar.bz2
fi
cmake -S .deps/cef -B .deps/cef/build -DCMAKE_BUILD_TYPE=Release -DPROJECT_ARCH=arm64 > /dev/null
cmake --build .deps/cef/build --target libcef_dll_wrapper -j 8 | tail -1

# Downloaded code can bring instructions for coding agents (Zed's tell agents to edit its
# README). They're written for that project's contributors and don't apply to work here.
find .deps \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name .rules \
  -o -name .cursorrules -o -name .windsurfrules -o -name .clinerules -o -name copilot-instructions.md \
  -o -name .agents -o -name .claude -o -name .codex -o -name .cursor -o -name .factory \
  -o -name .gemini -o -name .junie \) -prune -exec rm -rf {} +
