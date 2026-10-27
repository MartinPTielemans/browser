# Engine

Chromium built as Chrome's framework (`//chrome:chrome_framework`), from ungoogled-chromium plus our patch series. `./setup.sh` prepares the tree in `.deps/src`, and `./build.sh` builds `Chromium.app`, with the framework in it, in `.deps/src/out/Release`. `./setup.sh dev` and `./build.sh dev` do the same for the dev build in `.deps/src/out/Dev`, which has no link-time optimization, so its framework relinks much faster, and checks Chromium's DCHECKs. Work on the core uses it.

Work that doesn't change the core needs no build: `./fetch.sh` downloads the prebuilt engine `prebuilt.ini` names, and the spikes use it when there's no local build. `./publish.sh` makes one from the committed tree and uploads it as a GitHub release named after its commit, which is its source. A prebuilt engine has its commit's core, so `fetch.sh` refuses it once `core/public/browser_core.h` has changed; publish a new one with the API change.

## Layout

| Path | What |
|---|---|
| `pins.ini` | Chromium version and the ungoogled-chromium, ungoogled-chromium-macos and ungoogled-chromium-portablelinux tags. Bumped together, every Chromium release |
| `series` | Ordered list of patches applied after ungoogled-chromium's own series |
| `patches/helium/` | Patches taken from Helium, header naming the upstream path and commit |
| `patches/ardia/` | Patches taken from Ardia, same convention |
| `patches/ours/` | Our own patches |
| `args.gn` | Our GN args, after ungoogled-chromium's `flags.gn` and ungoogled-chromium-macos's `flags.macos.gn` |
| `args.dev.gn` | The dev build's args, after `args.gn` |
| `tree.sh` | What `setup.sh` and `check-linux.sh` both do to a prepared tree: remove Chromium's agent instructions, link the core and apply `series` |
| `check-linux.sh` | Compiles the core on Linux, as CI does |
| `point-release.sh` | Finds a security point release and dry-runs the patches on it, for the daily Action (Point releases, below) |
| `prebuilt.ini` | The prebuilt engine: the commit it was built from, its URL and its SHA-256. Written by `publish.sh` |

## Order

`setup.sh` does this, using ungoogled-chromium-macos's tooling at the pinned tag, which is how its releases are built:

1. Download Chromium's source tarball at the pinned version, checked against its published hashes.
2. Apply ungoogled-chromium's pruning, its patches and ungoogled-chromium-macos's, and domain substitution.
3. Download the toolchain ungoogled-chromium-macos names (LLVM, Rust, Node), in place of Google's.
4. Download Chrome's optimization profile for Apple silicon, which the source tarball lacks, and build `gn` and `bindgen`, each only if it's missing.
5. Link the repository's `core/` into the tree as `//browser_core`, so the next build has every edit to it. Apply `series` in order, with no fuzz. A patch already in the tree is skipped, and one that neither applies nor is in the tree stops the run as drift.
6. Write `out/Release/args.gn` and run `gn gen`.

Steps 1–3 change the tree and run once; a run that stopped in one of them means starting over with a fresh tree. Steps 4–6 run every time, so adding a patch to `series` and running `setup.sh` again applies it. Once `pins.ini` names another version, a plain run stops, and `setup.sh update` takes it instead (Point releases, below).

`setup.sh` names what it needs and isn't installed: Homebrew's `ninja`, `go`, `esbuild`, `typescript`, `coreutils` and `uv`, and Apple's Metal toolchain. It builds with Python 3.13, as ungoogled-chromium-macos does, through `uv`.

It also needs the macOS SDK of Chromium's official builds (`mac_sdk_official_version` in `build/config/mac/mac_sdk.gni`: 26.5 for Chromium 156), from Xcode or the Command Line Tools, even when a newer Xcode is installed. The pinned LLVM's linker can't read newer SDKs.

## Linux

CI (`.github/workflows/core-linux.yml`) runs `check-linux.sh` on pushes that change the core or the engine. It prepares the tree with ungoogled-chromium-portablelinux's scripts at the pinned tag, applies `series`, and compiles only the core's own objects; the `browser_core` target would compile most of Chrome, and nothing links. A run takes about half an hour and 40 GB of disk.

## Point releases

Chromium's security point releases (same major, such as 156.0.8078.47 → .60) update the engine without asking the owner. A new major waits for the owner's go.

- **Detect.** `.github/workflows/engine-point-release.yml` runs `point-release.sh` daily on GitHub's runners. For each platform section in `pins.ini`, it finds the newest tag on the pinned major. The bump follows the Mac, which never waits on another platform: `[chromium]` moves to the Mac's newest point release, and each section on the same version that has a tag for it moves too. A section whose tag lags stays on the Chromium its tag names, which `check_ungoogled` in `tree.sh` checks alone, and follows in its own PR once its tag is out, as Windows does on its own version. It extracts only the files the patches touch from Chromium's source tarball and dry-runs ungoogled-chromium's patches, the platform's, domain substitution and `series`. It then opens one PR bumping `pins.ini`, with "applies cleanly" or the conflicting patches, and the patch lag: the days since Chrome's stable release of that version. A new major gets an issue asking the owner, once.
- **Update in place.** `setup.sh update` (and `setup.sh update dev`) prepares the new version in `.deps/next`, series and all, then copies it over `.deps/src` with `rsync --checksum --delete`, leaving `out/` alone. Unchanged files keep their dates, so ninja rebuilds only what changed. The scratch tree needs about 25 GB while it runs. Afterwards, `setup.sh` for the other build regenerates its build files.
- **Merge.** The integrator builds Dev and Release incrementally, runs the full check, merges, and tags `good/…`. The PR comes from `GITHUB_TOKEN`, so no workflow runs on it. A conflict goes to a lane, as for a major.

## What reaches Google

ungoogled-chromium cuts the browser's calls to Google. Ardia's patches in `patches/ardia/` bring back two of them, for security only (decision 2).

- **Security components.** The component updater checks `https://update.googleapis.com/service/update2/json` for three components only: CRLSet (revoked certificates), PKI Metadata (Chrome Root Store, CT logs, pins) and the SSL error assistant. Every other component stays out of the request and is never downloaded. The first check runs a minute after launch, then one every five hours. Downloads may come only from `dl.google.com`, `*.gvt1.com` or `www.google.com/dl/`, over https, and redirects are held to the same hosts.
  - **What a check sends.** In the body: the protocol version, Chromium's version and channel, the updater's version, macOS's version, the CPU architecture, and a random session id and request id. For each component it sends the id and the installed version. The headers repeat the ids and say whether the check runs in the foreground.
  - **What it doesn't send.** No cookies and no pings. None of the fields Chrome uses to link one check to the next (install id, install date, cohorts, ping day counts). No language, hardware or enterprise state. Google still sees the IP address, and the versions can help it link checks.
  - **How responses are checked.** CUP signatures and the CRX3 publisher key, as in Chrome.
- **Extension kill switch.** The extension update check's `_malware`, `_policy_violation` and `_potentially_uws` answers blocklist or disable the extension, as in Chrome. That check covers only Web Store extensions, which install and update again through `patches/ours/chromium-extensions-web-store.patch`.

## Rules

- Keep patches small and separate. New code goes in new files under `//browser_core` where possible. Patches to Chrome's files add hooks; they don't move logic.
- Domain substitution rewrites Google's hosts in the source before the series is applied, so a patch that needs a real host writes it in full, and nothing rewrites it.
- One concern per patch, named `chromium-<area>-<what>.patch`.
- Each release, diff Helium's and Ardia's series against ours for anything worth taking.
- A new Chromium major, or a change to the GN args, means a full build. A point release taken with `setup.sh update` rebuilds only what it changes.
- An Xcode update doesn't: `chromium-xcode-build-define.patch` drops the `CR_XCODE_BUILD` define, which upstream compiles every object with only so that one does.
