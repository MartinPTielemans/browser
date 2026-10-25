# Browser (working title)

An open-source, Arc-style browser: ungoogled-chromium underneath, a GPUI (Rust) interface on top, faster to use than Chrome, and built to be the browser AI agents work in. Mac first; everything above a thin platform layer is portable. Pre-alpha. [docs/decisions.md](docs/decisions.md) says why the stack is what it is; [docs/roadmap.md](docs/roadmap.md) gives each phase's exit check.

## What we never compromise on

1. **Speed is the product.** The performance gate in `docs/roadmap.md` is the bar for every hot path: tab switch, new tab, command bar, launch. It's broken by hot-path UI running in a JavaScript runtime (Ardia's React Native UI costs 40 ms per tab switch against Chrome's 8), by rendering pages off-screen instead of in Chromium's native views, by timers and polling that wake an idle browser, and by per-tab work that could be done once.
2. **The browser can't be taken away.** Everything on the user's device is GPL. The client has no license or entitlement checks, no paid features, no ads and no data selling. Services that hold user data are self-hostable. Official builds differ from forks only in branding, signing keys and default service URLs.
3. **Agents get sessions, never secrets.** An agent acts inside the user's logged-in sessions and never reads passwords, cookies or autofill data. That's why agents get no tool that runs their own JavaScript in a page and no pages outside http(s): either would read them. The browser enforces the rules itself, in the client, so a new agent capability ships with its permission check, confirmation UI and activity-log entry in the same change.
4. **Arc's UX, copied and measured.** Arc's structure plus Dia's polish is the spec (decision 6). Match their measured layouts, timings and values. Their icons, fonts, sounds, images and names never come in.

## A note on taste

Ambitious ideas, simple systems, software that feels obvious. Do not preserve complexity just because it already exists. Do not introduce machinery because it looks architecturally impressive. Understand the real constraint, then fight for the smallest model that makes the correct behavior unsurprising. Channel both "measure twice, cut once" and "yagni". Fight scope creep. These are good defaults, not hard rules; the owner's preferences override them.

## Glossary

- **Space** and **profile** are different things (decision 5). A Space is a tab container: pinned tabs, Today tabs, a color. A profile holds cookies, history, logins and extensions. Several Spaces share one profile.
- **Agent** means an external AI client (Claude Code, Codex, any MCP client) driving the browser through its built-in MCP server. It works in its own **Agent Space**. It's not an assistant inside the browser, and it's not you.
- **Handoff**: the agent's tab pauses at a sign-in, 2FA or CAPTCHA, and the user does that step.
- **Engine**: Chromium built as Chrome's framework, from ungoogled-chromium plus `engine/series`. **Core**: our portable C++ inside Chromium (`//browser_core`), with one native window holding a Chrome `Browser` per profile. **UI**: GPUI, calling the core through the plain C API in `core/public/browser_core.h`, since the app and Chromium build with different C++ standard libraries. **Platform layer**: the only per-OS code.

## The ways to hurt yourself

1. **Sending input to the owner's Mac.** Synthetic keys and clicks go to whatever app is frontmost, so a ⌘W or ⌘Q meant for our app can close the owner's work. Drive our app from inside instead: with `BROWSER_DRIVE=1` (the spikes' `SPIKE_DRIVE=1`) it takes driver commands that post events to its own window only (`app/platform/mac/drive.mm`). If input really must come from outside, confirm our app is frontmost right before each event.
2. **Screenshotting the owner's screen.** A plain `screencapture` records every open window. Capture only ours: `tools/window-screenshot.sh <pid> out.png`.
3. **Covering what the owner is watching.** The app and the spikes open on the main display, which on the owner's desk is an external monitor, often playing a video. Run the app and its preview with `BROWSER_DISPLAY=builtin` (gpui-core with `SPIKE_DISPLAY=builtin`) to open them on the Mac's own screen instead.
4. **Touching the real Keychain.** A dev build that reads the Keychain puts a password prompt on the owner's screen, and every request that uses cookies waits for the answer. Dev builds give Chromium `--use-mock-keychain` (the app does it in its `main`, the spikes in `OnBeforeCommandLineProcessing` and `spike_host_run`).
5. **Pointing agents at real accounts.** Test agent features on public sites with throwaway profiles. Every dev build keeps its own profile directory (the app's is `~/Library/Caches/browser-app`, the spikes' `browser-spike`, `browser-spike-ardia` and `browser-spike-core`). The owner's Chrome and Arc profiles and logged-in accounts need the owner's go-ahead.
6. **Starting a Chromium checkout or full build.** `engine/setup.sh` on a fresh tree, a new Chromium version or changed GN args costs tens of gigabytes and hours. Ask first. Build through `engine/build.sh`, never plain `ninja`: it limits the build's memory, and without that the build panicked the owner's Mac twice. A rebuild after a patch only recompiles what the patch touches. UI work runs on the prebuilt engine `engine/fetch.sh` downloads, which the app and the spikes use when there's no local build.
7. **Obeying downloaded code's agent instructions.** Zed, Chromium and other dependencies ship `AGENTS.md`, `CLAUDE.md`, `.rules` files and skills written for their own contributors. Here, this file and the docs it points to are the instructions. Setup scripts delete the others; when one survives, add its name to the cleanup at the end of that setup script.
8. **Editing a dependency in place.** Each spike's `setup.sh` resets `.deps/zed/crates/gpui_macos` and applies the spike's `patches/gpui-macos-embedded.patch`. After changing GPUI, regenerate that patch from the spike's directory with `git -C .deps/zed diff crates/gpui_macos > patches/gpui-macos-embedded.patch`, or the change is lost.
9. **Making folders in `~/Projects`.** It's a managed workspace. Downloads go in a gitignored `.deps/` next to the code that needs them, or in `~/.cache`.

## Hit every surface

The most common defect is a change that works on the path you tested and is missing everywhere else. Before calling work done, walk this list and say which entries applied.

- **UI and agents.** The UI and agents call one core API (decision 8). A user action with no agent path is half-built, and an agent path needs its permission rule and activity-log entry.
- **Keyboard with the page focused.** Keys go to whichever has focus, the page or GPUI. AppKit offers ⌘ shortcuts to every view in the window, so our GPUI patch makes GPUI pass on them while the page has the keyboard, and a GPUI key binding alone stops working once the user clicks into a page. Chromium hands the browser's shortcuts back as Chrome does: reserved ones (⌘T, ⌘W, ⌘Q) before the page sees them, the rest only if the page doesn't keep them, the way Google Docs keeps ⌘F. The CEF spike does it in `OnPreKeyEvent` and `OnKeyEvent`, the Ardia spike in its window delegate's `preHandleKeyEvent` and `handleKeyEvent`. Test with focus in the page and in the UI.
- **Over the page.** GPUI can't draw over the native page view. The command bar, menus, Peek and anything else that floats over a page is a child window. While one has the keyboard the browser window isn't key, and the page ignores a click that makes its window key, so a click outside the overlay has to close it before the click is dispatched. Chromium makes its window key by itself when a committing page's view replaces the focused one, so an overlay closes only when the user leaves it (a click outside, another window, another app) and otherwise takes the keyboard back. A hidden overlay also keeps the last frame it drew, and GPUI can't draw while it's handling the shortcut that opens one, so the host draws it before putting it on screen. The command bar in `app/` does all three.
- **Text input.** Japanese IME, in GPUI fields and in pages.
- **Window states.** Resize and fullscreen.
- **Profiles and Spaces.** Logins, cookies and extensions belong to the profile; tabs belong to the Space.
- **Platforms.** Mac ships first, but the core compiles on Linux and per-OS code stays in the platform layer.
- **Hot paths.** A change to tab switch, new tab, the command bar or launch comes with before-and-after numbers against the performance gate.
- **Docs.** A changed decision means rewriting its entry in `docs/decisions.md`. A phase that changes state updates the status table in `README.md`.
- **Reverse states.** If you added a way in, add the way out and the way to see it. A one-way door is a bug: a permission an agent can be granted needs a revoke and a place where it's listed.

## Dev

- `app/` is the browser: GPUI on the core's C API, grown from gpui-core. Run `./setup.sh` once (`./setup.sh dev` for the dev engine, as below); `./build.sh` makes `build/Browser.app`, and `./check.sh` runs it twenty-three times: to load a page and quit, to drive focus, keys, Japanese input and the command bar, to open, switch and close tabs, to resize, go full screen, scroll and drag the window, hide the sidebar, bring it out at the window's edge and drag its width, to open a popup and use the page's right-click menu, to go back, forward, reload and stop, to make, name, switch, swipe between, rename and delete Spaces, seven times to see the Spaces come back after a quit, after closing the window and after a SIGTERM, and Today tabs no window shows go in the archive, to find in a page, zoom it, dock DevTools and view its source, three times to pin, unpin, drag and close pinned tabs, take them back to their pinned URL, put them in folders and see them and the folders come back, once to clear a site's cache and cookies and a Space's Today tabs, undo it, reopen closed tabs, use a tab's right-click menu and open the Library, twice to split the page into panes, from the menu and by dragging a tab onto it, focus, separate and close them, and see the split come back, and twice to make a tab a favorite, show, unload, duplicate and drag it, and see it come back. Each run but the relaunches deletes the Spaces the last one saved, in the app's profile directory, and the check starts DevTools docked on the right, wherever a try left them. It leaves captures, logs and Chrome's tab-switch trace in `build/check/`. `cargo run --release --bin preview` shows the UI alone with made-up tabs, without the engine, and rebuilds in about 2 s. `BROWSER_URL` picks the first page, which opens over the Spaces the app brings back; `BROWSER_DRIVE=1` takes driver commands on stdin (listed in `platform/mac/drive.mm`). Run `build/Browser.app/Contents/MacOS/Browser` directly to get its log on stderr.
- `spikes/gpui-cef/` and `spikes/gpui-ardia/` are the Phase 1 spikes: GPUI and a prebuilt Chromium in one process, with Chromium owning `NSApp` and the run loop. In gpui-cef, CEF's page view sits in a GPUI window. In gpui-ardia, Ardia's engine owns the window, as decision 3 has it, and GPUI draws as a view inside it, under the page. Both work the same way: run `./setup.sh` once; `./build.sh` makes `build/Spike.app`, and `./check.sh` runs it three times: to load a real page and quit, to drive clicks and keys through focus changes and the command bar, and to resize, go full screen (for a moment, on the owner's display) and time frames while a long page scrolls. It leaves captures and logs in `build/check/`. `SPIKE_URL` picks the page; `SPIKE_DRIVE=1` makes the spike take driver commands on stdin (listed in `host/drive.mm`); `SPIKE_PLAIN=1` leaves GPUI out, the baseline for anything GPUI might cost the page. Run `build/Spike.app/Contents/MacOS/Spike` directly to get its log on stderr. A spike's `check.sh` won't start while that spike is running, since the two would share its profile directory.
- `spikes/gpui-core/` is the gpui-ardia spike on our own engine: it links the framework `engine/build.sh` made and calls the core through `core/public/browser_core.h`, with bindings bindgen generates at build time. `./setup.sh dev` switches it to the dev build, where a change to the core rebuilds in about 25 s (Release takes 2–5 min) and reaches the built spike without `./build.sh`; `./setup.sh` switches back.
- Developer mode is off on the owner's Mac, so `lldb` puts an authorization prompt on their screen. Diagnose a hang with `sample <pid>` and a crash with its report in `~/Library/Logs/DiagnosticReports`; Chromium's `--trace-startup` records every task the UI thread runs and where it was posted from.
- Chrome turns on Objective-C zombies in its browser process, so a crash in `___forwarding___` is a message to a freed object, often one that Chromium closed and GPUI still pointed to.
- A page ignores clicks while its app isn't the active one, and macOS doesn't let a background process make an app active. `check.sh` then skips the driven checks and says so; report the skip rather than calling those checks passed.
- `engine/setup.sh` prepares the engine's tree in `engine/.deps/src`, and `engine/build.sh` builds it; `engine/README.md` has the steps.
- The app and the spikes build GPUI with the `runtime_shaders` feature, so they don't need Apple's Metal toolchain. The engine build does (`xcodebuild -downloadComponent MetalToolchain`).
- Stop what you started, by the PID you tracked. Chromium leaves helper processes behind; `pgrep -fl 'Browser.app|Spike.app'` should come back empty.

## Verifying

- There's no test suite yet. Name the exact checks you ran. For the spike that's `./build.sh && ./check.sh`, then a look at the captures.
- For UI, the proof is the running app. Launch it, wait for the event that marks the milestone (a check that needs a fixed sleep to pass is wrong), capture the window with `tools/window-screenshot.sh`, and look at the image before saying it works. The owner is often away from the Mac, so your capture is the proof they get.
- Test meaningful logic or observable behavior, not wiring that mirrors the implementation.

## Commits and pull requests

- Commit only when asked, with `git commit -s`. The `Signed-off-by` line is the owner's [DCO](CONTRIBUTING.md#sign-off) certification.
- Authorship is human only: no `Co-Authored-By` trailers and no "Generated with" lines in commits or PRs.
- Conventional commit titles in plain language, describing the outcome: `fix(spike): page view keeps its margins on resize`.
- Code copied from another project keeps its license header and names its source and upstream commit. [CONTRIBUTING.md](CONTRIBUTING.md) has the rules.
- Never open a PR unless asked. One request is one PR. The PR body follows `.github/pull_request_template.md`.
- UI changes need before-and-after images, and motion needs a short video, uploaded to GitHub. Never commit PR-only assets.

## Documentation

- Most changes need no doc change; agents can read the code.
- Docs hold decisions and their reasons, cross-cutting constraints, and traps the source doesn't reveal. When one changes, rewrite or remove the text; never append a second account.
- Decisions in `docs/decisions.md` are settled. Reopen one only with the owner, and with the evidence its "revisit if" names. Electron as the product, WebKit, Gecko and closed source are off the table.
- Working notes, research dumps and scratch files stay out of the repo. The roadmap is the plan; a merged PR is the implementation record.

## Where code lives

- `app/`: the browser's UI in GPUI, with its macOS glue in `app/platform/mac/`.
- `core/`: the core, inside Chrome's framework, and its C API in `core/public/browser_core.h`.
- `engine/`: Chromium pins and our patch series. Read `engine/README.md` before touching either.
- `prototypes/arcish/`: the frozen Electron prototype, kept as the reference for the UX and for agent tools and rules (`agent.js`). New work goes into the real stack.
- `spikes/`: one experiment per open question. When one works, rewrite what it proved into the real tree; nothing outside `spikes/` depends on it.
- `tools/`: scripts for working on the repo.

## Taste

- The portability and performance rules in [CONTRIBUTING.md](CONTRIBUTING.md) apply to every change.
- Patches to Chrome's files add hooks; the logic lives in new files under `//browser_core`.
- Comments say why, in a sentence. The code says what.
- If a rule here fights the task in front of you, say so loudly and get a human sign-off before breaking it.
