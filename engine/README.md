# Engine

Chromium built as Chrome's framework (`//chrome:chrome_framework`), from ungoogled-chromium plus our patch series. `./setup.sh` prepares the tree in `.deps/src`, and `./build.sh` builds `Chromium.app`, with the framework in it, in `.deps/src/out/Release`. `./setup.sh dev` and `./build.sh dev` do the same for the dev build in `.deps/src/out/Dev`, which has no link-time optimization, so its framework relinks much faster, and checks Chromium's DCHECKs. Work on the core uses it.

## Layout

| Path | What |
|---|---|
| `pins.ini` | Chromium version, ungoogled-chromium tag and ungoogled-chromium-macos tag. Bumped together, every Chromium release |
| `series` | Ordered list of patches applied after ungoogled-chromium's own series |
| `patches/helium/` | Patches taken from Helium, header naming the upstream path and commit |
| `patches/ardia/` | Patches taken from Ardia, same convention |
| `patches/ours/` | Our own patches |
| `args.gn` | Our GN args, after ungoogled-chromium's `flags.gn` and ungoogled-chromium-macos's `flags.macos.gn` |
| `args.dev.gn` | The dev build's args, after `args.gn` |

## Order

`setup.sh` does this, using ungoogled-chromium-macos's tooling at the pinned tag, which is how its releases are built:

1. Download Chromium's source tarball at the pinned version, checked against its published hashes.
2. Apply ungoogled-chromium's pruning, its patches and ungoogled-chromium-macos's, and domain substitution.
3. Download the toolchain ungoogled-chromium-macos names (LLVM, Rust, Node), in place of Google's.
4. Download Chrome's optimization profile for Apple silicon, which the source tarball lacks, and build `gn` and `bindgen`, each only if it's missing.
5. Link the repository's `core/` into the tree as `//browser_core`, so the next build has every edit to it. Apply `series` in order, with no fuzz. A patch already in the tree is skipped, and one that neither applies nor is in the tree stops the run as drift.
6. Write `out/Release/args.gn` and run `gn gen`.

Steps 1–3 change the tree and run once; a run that stopped in one of them means starting over with a fresh tree. Steps 4–6 run every time, so adding a patch to `series` and running `setup.sh` again applies it.

`setup.sh` names what it needs and isn't installed: Homebrew's `ninja`, `go`, `esbuild`, `typescript`, `coreutils` and `uv`, and Apple's Metal toolchain. It builds with Python 3.13, as ungoogled-chromium-macos does, through `uv`.

It also needs the macOS SDK of Chromium's official builds (`mac_sdk_official_version` in `build/config/mac/mac_sdk.gni`: 26.5 for Chromium 155), from Xcode or the Command Line Tools, even when a newer Xcode is installed. The pinned LLVM's linker can't read newer SDKs.

## Rules

- Keep patches small and separate. New code goes in new files under `//browser_core` where possible. Patches to Chrome's files add hooks; they don't move logic.
- One concern per patch, named `chromium-<area>-<what>.patch`.
- Each release, diff Helium's and Ardia's series against ours for anything worth taking.
- A new Chromium version, or a change to the GN args, means a full build.
