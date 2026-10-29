# ilo

a good tool.

ilo is an open-source web browser. Your tabs sit in a sidebar, in Spaces: each Space keeps its own pinned tabs and today's tabs, and today's tabs you leave alone go to the archive. A command bar opens pages, tabs and commands, and two pages can share a window side by side. Underneath is Chromium, from ungoogled-chromium, so web pages and Chrome extensions work in it.

ilo is also built to be the browser AI agents work in. Claude Code, Codex or any MCP client can work in your logged-in sessions, in a Space of its own, with the permissions you give it. It never reads your passwords or cookies. In 0.1.0, agents are off until you turn them on.

## Install

0.1.0 is the first public beta. Its builds come from [Releases](https://github.com/MartinPTielemans/browser/releases).

| OS | How | State |
|---|---|---|
| macOS, Apple silicon | A signed, notarized DMG: open it and drag ilo into Applications. It updates itself. | 0.1.0 |
| Windows | winget and the Microsoft Store | coming |
| Linux | A Flatpak: `flatpak install --user ilo.flatpak`, then start ilo from your apps. | 0.1.0 |

## Status

Beta. 0.1.0 is the first build for other people to use, and some of it is still missing: [#454](https://github.com/MartinPTielemans/browser/issues/454) lists what's in and what's left.

| Phase | What | State |
|---|---|---|
| 0 | UX spec: Arc structure + Dia polish, measured | in progress: Dia values imported; Arc's layout, sidebar and Spaces motion, command bar, find, split view, Library and Little Arc measured; Peek, extensions, passwords, themes and drags left |
| 1 | Architecture spike: GPUI + Chromium in one Mac process | done: go for GPUI |
| 2 | Engine: ungoogled-chromium + patch series, prebuilt framework | done: the prebuilt engine downloads into the spike, passwords and autofill are on, Manifest V2 works, CI compiles the core on Linux |
| 3 | Daily-driver MVP on Mac | in progress: sidebar, Spaces, pinned tabs and folders, favorites, command bar, split view, archive, downloads, find, DevTools, extensions and the password prompts; import, updates, password fill and sign-in popups left |
| 4 | Polish and parity | — |
| 5 | Windows, then Linux | in progress: on Windows, `Browser.dll` and the core's Windows layer are written and CI cross-builds the engine, none of it run against an engine yet; on Linux, the browser runs on the Linux engine CI publishes and packs as a Flatpak |

See [docs/roadmap.md](docs/roadmap.md) for each phase's exit criteria and [docs/decisions.md](docs/decisions.md) for why the stack is what it is.

## Build

Each OS builds the app on a prebuilt engine, which `app/setup.sh` downloads, so no Chromium build is needed.

- **macOS:** `app/setup.sh`, then `app/build.sh` makes `app/build/Browser.app`. [AGENTS.md › Dev](AGENTS.md#dev) has the dev engine, the checks and the prod build, and `tools/release.sh` makes the signed DMG.
- **Linux:** `app/setup.sh`, then `app/build.sh` makes `app/build/Browser`, and `tools/flatpak.sh` packs it as a Flatpak. Without an engine for your checkout, `setup.sh` sets up the UI's tests and its preview alone.
- **Windows:** `app/setup.sh` in Git Bash, then `cargo build --release --manifest-path app/windows/Cargo.toml` builds `Browser.dll`, which the engine's `chrome.exe` loads.
- **The engine:** [engine/README.md](engine/README.md) covers building it, on each OS, and its patches.

[CONTRIBUTING.md](CONTRIBUTING.md) has the rules for changes.

## Releases

`.github/workflows/release.yml` makes a release from an annotated tag, `git tag -a v<version>` on a commit whose `app/Cargo.toml` has that version, with the release notes as the tag's message. Each OS builds on its prebuilt engine for that commit, and the run stops if there isn't one. It makes a draft release whose files keep their names, so `releases/latest/download/<name>` always links the newest: `ilo-mac-arm64.dmg`, `ilo-windows-x64.msix`, `ilo-windows-x64-setup.exe`, `ilo-linux-x86_64.flatpak`, `ilo.flatpakref`, `SHA256SUMS` and `appcast.xml`. Once the draft is published, its `appcast.xml` goes to the updates feed: `gh release download v<version> -p appcast.xml && gh release upload updates appcast.xml --clobber`.

The Mac's DMG is built only with these repository secrets:

| Secret | What |
|---|---|
| `MAC_CERTIFICATE_P12` | The Developer ID Application certificate and its key, as a base64 `.p12` |
| `MAC_CERTIFICATE_PASSWORD` | The `.p12`'s password |
| `MAC_NOTARY_KEY` | An App Store Connect API key for notarization, as a base64 `.p8` |
| `MAC_NOTARY_KEY_ID`, `MAC_NOTARY_ISSUER_ID` | That key's ID and its issuer's |
| `UPDATE_PRIVATE_KEY` | The updates' signing key: the contents of the file `generate_keys --account ilo -x <file>` writes (`tools/appcast.sh`) |

The Mac and Windows builds carry its public half from `app/branding/update-key`, or from an `UPDATE_PUBLIC_KEY` secret, which overrides the file when it's set.

The Windows packages are unsigned: the Store signs the MSIX, and winget takes the installer.

Linux installs from GitHub update from ilo's Flatpak repository, a static OSTree repository in a Cloudflare R2 bucket (`tools/flatpak-repo.sh`). The bundle and `ilo.flatpakref` name it and carry its public key. The run adds the build to the bucket, unseen; publishing the draft makes it live. Publish each draft before building the next, since a build doesn't follow a draft that isn't live yet. The Linux files are built only with these:

| Secret | What |
|---|---|
| `FLATPAK_GPG_KEY` | The repository's signing key, ASCII-armoured, without a passphrase |
| `R2_BUCKET` | The bucket's name |
| `R2_ENDPOINT` | `https://<account-id>.r2.cloudflarestorage.com` |
| `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | An R2 API token with Object Read & Write on that bucket alone |

and the repository variable `FLATPAK_REPO_URL`, the bucket's public address, such as `https://flatpak.<domain>/`.

Once, before the first release:

1. Make the key: `gpg --quick-gen-key "ilo Flatpak repository" rsa4096 sign never`, with an empty passphrase. Put `gpg --armor --export-secret-keys <fingerprint>` in `FLATPAK_GPG_KEY` and keep a copy offline forever. Every installed copy checks it, and a new key strands them.
2. In Cloudflare, create the R2 bucket, connect a custom domain to it for public reads, and create the API token.
3. Add the secrets and the variable. The address is permanent too, as installs keep it.

## Architecture

```
UI            GPUI (Rust)                 sidebar, Spaces, command bar, split view
Bridge        plain C API (bindgen)       core/public/browser_core.h
Core          portable C++ in Chromium     one window, one Chrome Browser per profile, popups become tabs
Platform      the only per-OS code         native view parenting, run loop
Engine        ungoogled-chromium + series  built as Chrome's framework
```

## Layout

| Path | What |
|---|---|
| `app/` | The browser: GPUI on the core's C API (`./setup.sh && ./build.sh`), and a preview of its UI without the engine (`cargo run --release --bin preview`) |
| `core/` | Our C++ inside Chrome's framework (`//browser_core`), and its C API for the app |
| `docs/` | Decisions, roadmap, UX spec and its parity checklist |
| `engine/` | Chromium pins and the patch series |
| `prototypes/arcish/agent.js` | What's left of the Electron prototype: its agent tools and their rules, the reference for agents |
| `tools/` | Scripts for working on the repo |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE), and [NOTICE](NOTICE) for the other projects ilo uses and carries. The name ilo and its icon are the project's trademarks and are not covered by the license. Forks are welcome under their own name: replace `app/branding/`, which holds the name, the icon and the crash collector's URL, and recompile the icon with `app/icon.sh`.

Contributions use the [Developer Certificate of Origin](CONTRIBUTING.md#sign-off), not a CLA.
