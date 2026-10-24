# Arcish

A throwaway prototype for trying Arc-style browser UX on Chromium (via Electron) before building the real browser on ungoogled-chromium (see [decision 2](../../docs/decisions.md#2-base-ungoogled-chromium-with-helium-and-ardia-as-patch-sources)).

```sh
npm install
npm start
```

If you launch it from inside another Electron app's terminal and get `Cannot read properties of undefined (reading 'setName')`, run `env -u ELECTRON_RUN_AS_NODE npm start`.

## What to try

| | |
|---|---|
| ⌘T / ⌘K | Command bar: URLs, search, open tabs, archive, history, commands |
| ⌘L | Edit the current URL |
| ⌘W / ⌘⇧T | Archive tab / reopen it |
| ⌘D | Pin / unpin tab |
| ⌘S | Toggle sidebar |
| Ctrl 1–9, ⌥⌘←/→, two-finger swipe on sidebar | Switch spaces |
| ⌥⌘↑/↓, Ctrl Tab | Previous / next tab |
| ⌘⇧C | Copy URL |
| Right-click a tab | Pin, favorite, split view, move to space, sleep, archive |
| Right-click a link | Open in new tab or in split view |
| Drag tabs | Reorder, drop into Favorites / Pinned / Today, or onto a space dot |
| Double-click the space name | Rename it; right-click a space dot to recolor or delete |

Speed features:

- **Lazy restore**: tabs restored on launch stay asleep until opened.
- **Tab sleeping**: background tabs are discarded after 10 idle minutes and come back with their back/forward history and scroll position. "Put Background Tabs to Sleep" in the command bar does it right away.
- **Auto-archive**: unpinned tabs move to the archive after 12 idle hours. Set "Auto-Archive After 5 Minutes (demo)" in the command bar to see it happen.
- **Built-in ad and tracker blocking** (Ghostery's engine, EasyList-based).
- The sidebar footer shows live memory for all processes, awake vs. total tabs, and the number of blocked requests.

State lives in `~/Library/Application Support/Arcish/state.json` on macOS.

## Agents

Claude Code, Codex or any MCP client can work in Arcish ([decision 8](../../docs/decisions.md#8-agents-the-browser-is-the-tool-agents-use)). Each agent gets its own Agent Space with a sparkle in the space switcher. Its tabs never take your focus, and it's signed in wherever you are.

```sh
claude mcp add arcish -- node "$PWD/tools/mcp-bridge.js"
codex mcp add arcish -- node "$PWD/tools/mcp-bridge.js"
```

The bridge connects the agent to Arcish's socket (`agent.sock` next to `state.json`, readable only by you) and starts Arcish if it isn't running. Agents get `open_tab`, `navigate`, `snapshot`, `read_page`, `screenshot`, `click`, `type`, `select_option`, `press_key`, `close_tab`, `list_tabs` and `request_human`. Snapshots are the page's accessibility tree, with refs like `e12` that stay put until the page navigates.

| | |
|---|---|
| Your OK first | Clicking pay, buy, order, subscribe and the like; sending or posting on mail, chat and social sites; opening banking sites. A card in the sidebar offers Allow, Deny or Always allow on that site. Allowed sites can be revoked from the agent's space. |
| Never | Typing into password fields, or seeing what's in them. |
| Handoff | At a sign-in, 2FA or CAPTCHA the agent calls `request_human`. You get a card, show the tab, do the step and hand it back. |
| Take over | The agent's space has a Take over button. Its actions pause until you hand back. |
| Activity | Everything the agent did, newest first, in its space. |

Drag one of your tabs onto an agent's space to give it that tab.

`tools/agent-demo.sh` records Claude Code doing a task with a scripted user answering its requests. It uses a throwaway profile and leaves a video in `/tmp/arcish-agent-demo`.

Known gaps: cross-origin iframes are missing from snapshots, the risk checks are keyword based, only navigations the agent asks for are checked against banking sites, and two sessions of the same agent share one space. Prompt injection from page content is limited by the confirmations, not solved.

## Not in this prototype

Profiles, Peek and Little Arc, Boosts, find-in-page, downloads UI, and Chrome extensions (Electron only supports a small subset).
