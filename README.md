# ilo

a good tool.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/ilo-dark.webp">
  <source media="(prefers-color-scheme: light)" srcset="docs/images/ilo-light.webp">
  <img alt="ilo: a window with the sidebar of Spaces and tabs on the left and a web page on the right" src="docs/images/ilo-light.webp">
</picture>

ilo is an open-source web browser, and Arc's successor. Your tabs live in a sidebar, in Spaces: each Space keeps its own pinned tabs and today's tabs, and today's tabs you leave alone move to the archive. A command bar opens pages, tabs and commands, and two pages can share a window side by side. Underneath is Chromium, by way of ungoogled-chromium, so websites and Chrome extensions work as they do in Chrome.

ilo is also the browser AI agents work in. Claude Code, Codex or any MCP client can work in your logged-in sessions, in a Space of its own, with only the permissions you give it, and it never reads your passwords or cookies. Agents are off until you turn them on.

## "What are you selling?"

Nothing. ilo is free, and licensed under the GPL. The browser has no paid features, no license checks and no ads, and it doesn't sell your data.

Services that hold your data, such as sync, can be self-hosted. The hosted ones we run may cost money one day; the browser won't.

Official builds differ from a fork only in their branding, signing keys and default service URLs. If ilo ever goes the wrong way, you have everything you need to build your own.

## Install

Every release is on [GitHub Releases](https://github.com/MartinPTielemans/browser/releases), with [what's new](CHANGELOG.md) in each. Once installed, ilo keeps itself up to date.

### macOS

macOS 15 (Sequoia) or later, on Apple silicon. Download [ilo-mac-arm64.dmg](https://github.com/MartinPTielemans/browser/releases/latest/download/ilo-mac-arm64.dmg), open it and drag ilo into Applications.

### Windows

Windows 10 (1809) or later, on x64:

```powershell
winget install ilo.ilo
```

ilo will be in the Microsoft Store once it's listed there.

### Linux

x86-64, as a Flatpak:

```bash
flatpak install --user https://github.com/MartinPTielemans/browser/releases/latest/download/ilo.flatpakref
```

Then start ilo from your apps. It updates with your other Flatpaks.

## Some notes

ilo is in beta. Expect bugs, and please report them in [Issues](https://github.com/MartinPTielemans/browser/issues/new/choose); in ilo, Help › Report a Problem… fills in the details for you. [#454](https://github.com/MartinPTielemans/browser/issues/454) tracks what's left for 0.1.0.

Contributions are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) says what's likely to land and how to send it.

## Documentation

- [Decisions](docs/decisions.md): why the stack is what it is, and what would change it.
- [Roadmap](docs/roadmap.md): each phase, where it stands and the check that ends it.
- [UX principles](docs/ux-principles.md): why Arc's design works, and how ilo builds on it.
- [UX spec](docs/ux-spec.md): every surface, how it looks, moves and behaves, measured.
- [UX parity](docs/ux-parity.md): the checklist built from the spec.
- [Releasing](docs/releasing.md): how a release is made, and what it needs.

## Build from source

The app builds on a prebuilt engine that `app/setup.sh` downloads, so you don't need to build Chromium:

```bash
cd app
./setup.sh
./build.sh
```

On the Mac that makes `app/build/Browser.app`, and on Linux `app/build/Browser`, which `tools/flatpak.sh` packs as a Flatpak. On Windows, run `app/setup.sh` in Git Bash, then `cargo build --release --manifest-path app/windows/Cargo.toml` builds `Browser.dll`, which the engine's `chrome.exe` loads.

[AGENTS.md › Dev](AGENTS.md#dev) has the UI's preview, the checks and the rest, and [engine/README.md](engine/README.md) covers building the engine and its patches.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE), and [NOTICE](NOTICE) for the other projects ilo uses and carries. In the app, Settings › About opens both, the Rust libraries built into it and their licenses, and Chromium's credits. The name ilo and its icon are the project's trademarks and are not covered by the license. Forks are welcome under their own name: replace `app/branding/`, which holds the name, the icon and the crash collector's URL, and recompile the icon with `app/icon.sh`. On Windows the engine has the name too, for its folders and the names it registers with Windows (`engine/patches/ours/chromium-windows-product-folder.patch`, `chromium-windows-app-names.patch`), so a fork builds its own Windows engine with its name there.

Contributions use the [Developer Certificate of Origin](CONTRIBUTING.md#sign-off), not a CLA.
