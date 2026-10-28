# ilo

a good tool.

An open-source, Arc-style browser that is faster than Chrome to use. It has Arc's sidebar, Spaces and archive, Dia's polish, Chrome's engine, and Chrome extensions. It is also built to be the browser agents use: Claude Code, Codex or any MCP client can work in your real browser, in their own Space, with permissions you set.

Mac builds come first. Everything above a thin platform layer is portable, so Windows and Linux are ports, not rewrites.

## Status

Pre-alpha. Nothing here is a browser yet.

| Phase | What | State |
|---|---|---|
| 0 | UX spec: Arc structure + Dia polish, measured | in progress: Dia values imported; Arc's layout, sidebar and Spaces motion, command bar, find, split view, Library and Little Arc measured; Peek, extensions, passwords, themes and drags left |
| 1 | Architecture spike: GPUI + Chromium in one Mac process | done: go for GPUI |
| 2 | Engine: ungoogled-chromium + patch series, prebuilt framework | done: the prebuilt engine downloads into the spike, passwords and autofill are on, Manifest V2 works, CI compiles the core on Linux |
| 3 | Daily-driver MVP on Mac | in progress: sidebar, Spaces, pinned tabs and folders, favorites, command bar, split view, archive, downloads, find and DevTools; profiles, extensions, password and autofill prompts, import and updates left |
| 4 | Polish and parity | — |
| 5 | Windows, then Linux | in progress: on Windows, the UI preview and its tests run, `Browser.dll` and the core's Windows layer are written, and so is the CI workflow that builds the Windows engine, none of it run against an engine yet; on Linux, the UI's tests pass and the preview runs on Wayland, and CI runs the tests and builds the preview |

See [docs/roadmap.md](docs/roadmap.md) for each phase's exit criteria and [docs/decisions.md](docs/decisions.md) for why the stack is what it is.

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

GPL-3.0-or-later. See [LICENSE](LICENSE). The name ilo and its icon are the project's trademarks and are not covered by the license. Forks are welcome under their own name: replace `app/branding/`, which holds the name, the icon and the crash collector's URL, and recompile the icon with `app/icon.sh`.

Contributions use the [Developer Certificate of Origin](CONTRIBUTING.md#sign-off), not a CLA.
