#!/bin/sh
# Fetches the app's dependencies into .deps/ (gitignored): Zed at a pinned revision, with our
# GPUI patches applied, and links our engine, Chromium.app with the core in its framework: the one
# engine/build.sh built, or else the prebuilt one engine/fetch.sh downloads. `setup.sh dev` links
# the dev build's, and `setup.sh prebuilt` the prebuilt one even beside a local build.
set -e
cd "$(dirname "$0")"
ZED_REV=b36b283021c48f1712714b8a0e2ad6ffb578920b
mkdir -p .deps

if [ ! -d .deps/zed ]; then
  git clone --filter=blob:none --no-checkout https://github.com/zed-industries/zed .deps/zed
  git -C .deps/zed checkout -q "$ZED_REV"
fi
git -C .deps/zed checkout -q -- crates/gpui crates/gpui_macos
# A checkout made for an earlier pin moves to this one.
if [ "$(git -C .deps/zed rev-parse HEAD)" != "$ZED_REV" ]; then
  git -C .deps/zed fetch -q origin "$ZED_REV"
  git -C .deps/zed checkout -q "$ZED_REV"
fi
# One patch a crate: gpui_macos's embeds GPUI in Chromium's window, gpui's scales what's painted,
# lets the tests stand in for an assistive app and adds the Edit menu's Paste and Match Style.
git -C .deps/zed apply "$PWD/patches/gpui-macos-embedded.patch" "$PWD/patches/gpui-scale.patch"

# GPUI's macOS accessibility adapter, at the version Cargo.lock has, with VoiceOver's menu action
# and a folder's open state, which upstream lacks. A repository of its own, so a change to it
# becomes the patch again with `git -C .deps/accesskit_macos diff`.
ACCESSKIT_MACOS=0.26.3
ACCESSKIT_MACOS_SHA256=ce02dc63b43f0c9296af9ac946312a2dc8814427d7a64d2d600971dac55b6076
crate=.deps/accesskit_macos-$ACCESSKIT_MACOS.crate
if ! echo "$ACCESSKIT_MACOS_SHA256  $crate" | shasum -a 256 -c -s 2>/dev/null; then
  curl -sSfL "https://static.crates.io/crates/accesskit_macos/accesskit_macos-$ACCESSKIT_MACOS.crate" -o "$crate"
  echo "$ACCESSKIT_MACOS_SHA256  $crate" | shasum -a 256 -c -s
fi
rm -rf .deps/accesskit_macos "${crate%.crate}"
tar xzf "$crate" -C .deps
mv "${crate%.crate}" .deps/accesskit_macos
git -C .deps/accesskit_macos init -q
git -C .deps/accesskit_macos add -A
git -C .deps/accesskit_macos -c user.name=setup -c user.email=setup@localhost commit -qm upstream
git -C .deps/accesskit_macos apply "$PWD/patches/accesskit-macos-actions.patch"

# The engine is built for the Mac only so far, so elsewhere there is the UI alone: the tests and
# the preview (AGENTS.md, Dev).
if [ "$(uname -s)" = Darwin ]; then
  engines=$(cd .. && pwd)/engine
  case ${1:-} in
    "")
      engine=$engines/.deps/src/out/Release/Chromium.app
      [ -d "$engine" ] || engine=$("$engines/fetch.sh")
      ;;
    dev) engine=$engines/.deps/src/out/Dev/Chromium.app ;;
    prebuilt) engine=$("$engines/fetch.sh") ;;
    *) echo "usage: setup.sh [dev|prebuilt]" >&2 && exit 2 ;;
  esac
  [ -d "$engine" ] || { echo "No engine at $engine. Build it with engine/build.sh ${1:-}." >&2; exit 1; }
  ln -sfn "$engine" .deps/Chromium.app
fi

# Downloaded code can bring instructions for coding agents (Zed's tell agents to edit its
# README). They're written for that project's contributors and don't apply to work here.
find .deps/zed \( -name AGENTS.md -o -name CLAUDE.md -o -name GEMINI.md -o -name .rules \
  -o -name .cursorrules -o -name .windsurfrules -o -name .clinerules -o -name copilot-instructions.md \
  -o -name .agents -o -name .claude -o -name .codex -o -name .cursor -o -name .factory \
  -o -name .gemini -o -name .junie \) -prune -exec rm -rf {} +
