# Decisions

Each entry gives what was decided, why, and what would make us revisit it. Newest last.

## 1. Engine: Chromium

Chromium, not WebKit, Gecko, Servo or Ladybird.

- The product has to run on Mac, Windows and Linux. WebKit is only first-class on macOS.
- Chrome extensions and site compatibility are requirements.
- Chromium's engine is not what makes Chromium browsers feel slow. Bloat, UI layers and background work are, and we control those.

## 2. Base: ungoogled-chromium, with Helium and Ardia as patch sources

Not raw Chromium and not all of Helium.

- De-Googling Chromium is continuous work across hundreds of call sites and every release. ungoogled-chromium has done it for a decade.
- Helium is a product on top of ungoogled-chromium. Ardia built on Helium and needs about 15 skipped patches, about 10 patches that undo Helium, and a guard stage to keep that working (`docs/engine-build.md` in mantrakp04/ardia). We cherry-pick from Helium instead.
- One fewer upstream means Chromium security fixes reach us sooner.
- We take back one thing ungoogled-chromium removes: the updates that keep the browser from trusting what is known to be bad. These are certificate revocations, the root store, and Google's malware verdicts on extensions. Without them, a revoked certificate or a malicious extension stays trusted until the next release. Ardia's patches fetch them from Google with the identifying fields stripped. `engine/README.md` lists what is fetched and what is sent.

Revisit if cherry-picking from Helium costs more per release than taking its whole series would.

## 3. Architecture: Chrome's framework plus our own window host

Following Ardia's design: Chrome's `chrome_framework` with our layer linked in. One native window holds a Chrome `Browser` per profile, each with a stand-in `BrowserWindow` that has no UI. Tab views are parented directly into our window, and popups become tabs. The window is the core's, a Chrome Views widget as in Ardia, and GPUI draws as a view inside it, under the tab views.

- This keeps the whole `chrome/` layer, so extensions, the Web Store, autofill, passkeys and DevTools keep working.
- Ardia ships on this design today.
- Unlike Ardia, whose layer is about 80% Objective-C++, ours is portable C++ with a thin platform shim.

## 4. UI: GPUI, with Chromium Views as the fallback

- GPUI is compiled, fast, and already runs on Mac, Windows and Linux.
- Ardia's React Native UI measures 40 ms per tab switch against Chrome's 8 ms (`docs/perf/vs-browsers.md` in mantrakp04/ardia). Hot-path UI must not run in a JavaScript runtime.
- Pages render in Chromium's own native views, not off-screen, so we keep Chrome's compositing, latency, video overlays and battery life.
- GPUI can't draw over a native view. Anything that floats over the page (command bar, menus, Peek) is a borderless child window, which is also how Chrome does its bubbles.

Settled by the Phase 1 spikes (`spikes/gpui-cef/`, `spikes/gpui-ardia/`). On CEF and on Ardia's engine, GPUI runs inside Chromium's run loop, and focus, the browser's shortcuts with the page focused, IME composition, the command bar over the page, resize and full screen all work across the boundary. A long page scrolls at 120 fps with GPUI in the window.

Revisit if a later engine or GPUI change breaks run-loop sharing or input across the boundary in a way we can't patch. The fallback is Views (C++): equally cross-platform, with none of these integration risks.

## 5. Product model: Spaces are separate from profiles

A Space is a tab container (pinned tabs, Today, color). Several Spaces can share one profile, and the profile holds cookies, history, logins and extensions. This is Arc's model. Ardia merges the two, which costs memory and repeated sign-ins.

## 6. UX: copy Arc and Dia

No open-ended UX exploration. Arc's structure (sidebar, Spaces, pinned and Today tabs, favorites, archive, Peek, Little Arc, split view, Boosts) plus Dia's polish (command bar, motion, window materials) is the spec. Features beyond Arc and Dia come in only when the owner picks them, and `docs/ux-spec.md` lists each one.

Dia's surfaces built around its assistant aren't copied, since our agents work from outside the browser and nothing would fill them. So ⌘T opens Arc's command bar over the page, not Dia's New Tab page, whose box is the assistant's chat field and whose empty card holds its answers. Without the assistant, that page was Arc's bar moved into the card with the page hidden behind it.

- Start from Ardia's measured `docs/dia-spec.md` and `docs/dia-feature-parity.md` (Apache-2.0, credited).
- Measure Arc ourselves before it is shut down on 2026-12-22.
- Copy behavior, layout, timings and values. Never ship their assets or names.

AI features are out of scope for v1, but the sidebar, command bar and core leave room for them.

## 7. License: open source, GPL-3.0-or-later

- The first users are Arc refugees whose browser was taken away. A browser that can't be taken away is the pitch, and only source code makes it credible.
- We can use Helium's and Ardia's GPL patches directly.
- Our own code is GPL too, so improvements to it can't be taken into a closed browser.
- Money comes from services (end-to-end encrypted sync, later AI), search revenue sharing and an optional Pro tier, not from closed client code.
- The name and logo are trademarks, which is what actually protects the product.
- Contributions use DCO sign-off, not a CLA.

## 8. Agents: the browser is the tool agents use

Not an assistant inside the browser, which is what Dia and Comet do. External agents (Claude Code, Codex, any MCP client) drive the user's real browser as a first-class, built-in capability.

- **Built-in MCP server.** Local-only at first, on a socket only this user's processes can reach. Cloud agents come later, through explicit pairing.
- **Agent Spaces.** An agent works in its own Space, visible in the sidebar. It never takes the user's focus or cursor. The user can watch, take over and hand back at any time.
- **Engine-native page access.** The core serves the accessibility tree, a cleaned-up DOM, stable element references and screenshots straight from Chromium. No extension, and no remote-debugging port.
- **Permissions per agent and per site,** plus an activity log. Defaults are conservative: no payments, no sending messages or forms on sensitive sites without confirmation, and no banking sites unless the user allows them.
- **Handoff to the human.** At a login, 2FA prompt or CAPTCHA, the agent's tab pauses and the sidebar asks the user to step in.
- **Sessions, never secrets.** Agents act inside the user's logged-in sessions but can never read passwords, cookies or autofill data.

Why:

- Every current option fails an agent working beside a human. Headless Playwright has no logins and gets flagged as a bot. Cloud browsers do the same and take the data off the machine. Remote-debugging the user's Chrome is fragile and insecure, and Chrome 136 blocks it on the default profile.
- Owning the Chromium core layer (decision 3) is what makes this possible. Spaces (decision 5) are the natural unit of isolation.
- It sets us apart from Ardia and the other Arc-style browsers.

Consequences now:

- The core exposes one API that both the UI and agents use. Agents are not a bolt-on.
- "Agent Space" is a kind of Space in the data model from the start.
- The main risk is prompt injection from page content. Permissions and confirmations limit the damage but don't solve it, so confirmation UI and the activity log ship together with the MCP server, never after it.
