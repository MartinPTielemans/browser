#!/bin/sh
# Fetches the spike's dependencies into .deps/ (gitignored): Zed at a pinned revision, with our
# GPUI patch applied, and Ardia's prebuilt engine with the header for its API.
set -e
cd "$(dirname "$0")"
ZED_REV=b36b283021c48f1712714b8a0e2ad6ffb578920b
# Ardia publishes an engine for each engine tree it builds. These come from its
# packages/ardiacore/prebuilt-engines.tsv; the header is the one from the same release.
ARDIA_REPO=mantrakp04/ardia
ARDIA_RELEASE=v0.2.40-nightly.5
ARDIA_TREE=2c67e141131843cf5c74c72f67378e891810a46b
ARDIA_SHA256=8177d5324bef7f5e371bbc6b4fdc857b11605350772186b4ab0d4f8b7dcf8e62
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

# An engine from another pin, or none, is replaced. The archive is checked before it's unpacked.
if [ "$(cat .deps/ardia/tree 2>/dev/null)" != "$ARDIA_TREE" ]; then
  rm -rf .deps/ardia .deps/ardia.tmp
  mkdir -p .deps/ardia.tmp/include
  archive="ArdiaCore-$(echo "$ARDIA_TREE" | cut -c1-12).tar.xz"
  curl -fL "https://github.com/$ARDIA_REPO/releases/download/$ARDIA_RELEASE/$archive" -o ".deps/ardia.tmp/$archive"
  [ "$(shasum -a 256 ".deps/ardia.tmp/$archive" | cut -d' ' -f1)" = "$ARDIA_SHA256" ] \
    || { echo "$archive doesn't match its pinned SHA-256" >&2; exit 1; }
  tar -xJf ".deps/ardia.tmp/$archive" -C .deps/ardia.tmp
  rm ".deps/ardia.tmp/$archive"
  curl -fsSL "https://raw.githubusercontent.com/$ARDIA_REPO/$ARDIA_RELEASE/engine/ardiacore/src/ardia/core/public/ArdiaCore.h" \
    -o .deps/ardia.tmp/include/ArdiaCore.h
  echo "$ARDIA_TREE" > .deps/ardia.tmp/tree
  mv .deps/ardia.tmp .deps/ardia
fi

# Downloaded code can bring instructions for coding agents (Zed's tell agents to edit its
# README). They're written for that project's contributors and don't apply to work here.
find .deps \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name .rules \
  -o -name .cursorrules -o -name .windsurfrules -o -name .clinerules -o -name copilot-instructions.md \
  -o -name .agents -o -name .claude -o -name .codex -o -name .cursor -o -name .factory \
  -o -name .gemini -o -name .junie \) -prune -exec rm -rf {} +
