# UX parity checklist

One row per item in [ux-spec.md](ux-spec.md). The spec holds the full rule and wins when the two disagree; change both in the same commit. When an Arc capture replaces a value, change the row's target and source.

- Tick an item only when the running app matches its target, checked against a capture of our window (AGENTS.md › Verifying).
- **Phase.** MVP is Phase 3, 4 is Phase 4, and later is anything not yet on the roadmap.
- **Target.** "measure" means there's no number yet; the item is in the spec's Arc capture plan.
- **Source.** The spec's source tags: `D›` is Ardia's `dia-spec.md`, `P§` its `dia-feature-parity.md`, `O` its `omnibox-ux.md`.
- **Attribution.** Items marked Dia, Arc (Ardia) and Ardia are adapted from Ardia (https://github.com/mantrakp04/ardia) at `v0.2.40-nightly.5`, commit `fce3a02ffb917d2902af4ce47b398ce8b03a440d`, Copyright 2026 Mantra Patel, Apache-2.0. The spec's Attribution section has the full notice.

## Window

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Blur behind the window | MVP | material 29 dark / 13 light, emphasized, following the active state | Dia 1.50.1 bin, D›Window translucency |
| [ ] | Base tint | MVP | black 0.4 / white 0.8 | Dia 1.50.1 bin |
| [ ] | Space gradient | MVP | colour → HSL L + 0.25, stops 0.36 (neutral 0.12), layer 0.5 / 0.75 | Dia 1.50.1 bin |
| [ ] | Card fill | MVP | #121212 0.5 / white 0.7, no grain | Dia 1.50.1 bin |
| [ ] | Composite matches Dia's within 2 levels | MVP | dark `0.18·tint + 0.82·0.6·blur`; light `0.27·tint + 0.73·(204 + 0.2·blur)` | Dia 1.50.1 cap |
| [ ] | Inactive, Reduce Transparency, Increase Contrast | MVP | opaque wallpaper-tinted fill (AppKit) | Dia 1.50.1 cap |
| [ ] | Card inset | MVP | 10 on every side, docked or hidden | Arc (Ardia) rec |
| [ ] | Card radius | MVP | 10 continuous *interim*; Arc: measure | Dia cap |
| [ ] | Traffic lights | MVP | centre y 23.75; x measure | Arc (Ardia) P§4 |
| [ ] | Page before first paint | MVP | card colour over the tint, never white | Ardia P§7 |
| [ ] | Window drag regions and double-click action | MVP | empty sidebar, header, card margins; system title-bar action; empty sidebar double-click = new tab | Dia 1.50, Arc (Ardia) P§1 |
| [ ] | Resize edge | MVP | outer ≈ 3 pt | Dia P§1 |
| [ ] | New window / closed window | MVP | at its final frame with content / gone at once | Dia P§1 |
| [ ] | Close confirmations | MVP | "Close 3 tabs?" per Space with "Don't ask me again"; ⌘Q warning; quit guard for downloads. Arc: measure | Dia P§1 |
| [ ] | Page full screen | MVP | sidebar gone; Esc or exitFullscreen restores; tab switch ends it; no stale frame | Dia P§18 |
| [ ] | Window full screen with the sidebar | MVP | measure | — |

## Sidebar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Default width | MVP | measure (Dia 190) | Dia 1.50.1 bin |
| [ ] | Width range | MVP | measure (Ardia 160–400); rubber-band; saved; double-click resets | Ardia, Dia P§4 |
| [ ] | Header row centre | MVP | 23.75 below the window top | Arc (Ardia) P§4 |
| [ ] | Header hover plates | MVP | 32 × 32, radius 6, top 8; only the plate clicks | Arc (Ardia) P§4 |
| [ ] | Back / forward / reload positions | MVP | measure (Ardia 137.75 / 171.75 / 205.75 at 227.5) | Arc (Ardia) |
| [ ] | Narrow sidebar | MVP | reload hides first, then forward | Ardia |
| [ ] | Disabled nav glyph | MVP | about half brightness | Dia 1.52 cap |
| [ ] | URL field top | MVP | 46 below the window top | Arc (Ardia) P§4 |
| [ ] | URL field size, radius, fill, font | MVP | measure | — |
| [ ] | URL text | MVP | host only, no `www.`; Show Full URL off by default | Dia 1.50.1 bin |
| [ ] | URL on hover | MVP | full URL, host bright, path dim | Dia 1.49 cap |
| [ ] | Insecure and zoom indicators | MVP | lock-warning glyph; magnifier with − / + / Reset | Dia, Chrome P§8 |
| [ ] | Favorite tile | MVP | 41 tall, 6 apart, inset 6, radius 10 *interim* | Dia 1.50.1 bin + cap |
| [ ] | Favorite columns | MVP | 3 at 190 *interim*; Arc's rule: measure | Dia P§2 |
| [ ] | Favorite resting look | MVP | fill white 0.10, 0.5 pt stroke white 0.14 | Dia 1.50.1 bin |
| [ ] | Favorite selection theming | MVP | colourful (≥ 0.055): blur 5 at 22 %, 3 pt ring; one colour: mean fill, white template, soft-light ring; near-white: black fill, white 0.30 ring | Dia 1.50.1 bin |
| [ ] | Favorites fixed while the list scrolls | MVP | yes; scrolls with the list above 40 % of the sidebar | Dia, Ardia |
| [ ] | Favorite double-click | MVP | back to the pinned URL | Dia P§2 |
| [x] | Pinned tabs section | MVP | rows 6 under the tiles, above the Clear divider | Dia 1.52, Ardia |
| [x] | Pinned tab ⌘W | MVP | unloads, keeps the row | Dia rec P§2 |
| [ ] | Pinned tab away from its URL | MVP | ↩ on hover, "Back to Pinned URL", ⌘↩ *interim*; Arc's look: measure | Dia 1.53.0 bin |
| [ ] | Pinned tab in two windows | MVP | one live page; frozen frame washed white 0.40 / black 0.25 | Dia cap, Ardia |
| [x] | Clear divider line | MVP | 1 pt white ≈ 0.08, centre 25 above the first row's centre | Arc (Ardia) D›Arc note: Clear |
| [ ] | Clear label and arrow | MVP | arrow 7 × 8, 3 pt from "Clear", semibold ≈ 10 pt, ends with the titles | Arc (Ardia) |
| [ ] | Row pitch / height | MVP | 36 Arc (confirm) / Dia 37 and 34 | Arc (Ardia), Dia 1.50.1 bin |
| [ ] | Row radius | MVP | min(h/3, 10) continuous | Dia 1.50.1 bin |
| [ ] | Row favicon and title | MVP | 16 pt at x 9; title 7 after, 13 pt, trailing fade | Dia 1.50.1 bin |
| [ ] | Row states | MVP | hover white 0.16, pressed 0.31, selected #121212 0.5 + 0.5 pt white 0.15 hairline, unselected 0.06 | Dia asset catalogs |
| [ ] | Row titles | MVP | selected white 1, unselected 0.78 | Dia asset catalogs |
| [ ] | Row fill change | MVP | 300 ms ease-out | Dia 1.50.1 bin |
| [ ] | Close ✕ | MVP | on hover, ≈ 22 pt hover square | Dia cap |
| [ ] | Loading spinner | MVP | 12 × 12, 1.5 pt, ring 72 %, track 0.18, clockwise 1.88 s linear; only in rows ≥ 48 wide | Dia 1.50.1 bin |
| [ ] | Sleeping tab | MVP | faded favicon (alpha: measure) | Dia |
| [ ] | "+ New Tab" row | MVP | label white 0.55, plus 0.25, 3 under the last row, sticky at the bottom | Dia 1.50.1 bin |
| [ ] | Scroll edge | MVP | 1 pt P3 white 0.09 at favorites + 4; fades 100 ms cubic-bezier(0.25, 0.46, 0.45, 0.94) | Dia 1.53.0 bin |
| [ ] | Open tab motion | MVP | new row in place at once; rows below slide 140 ms cubic-bezier(0.25, 0.1, 0.25, 1) | Dia rec 120 fps |
| [ ] | Close tab motion | MVP | fade 100 ms cubic-bezier(0, 0, 0.58, 1); rows below slide 140 ms over it | Dia rec 120 fps |
| [ ] | Reorder | MVP | neighbour moves at the centre crossing; critically damped ω ≈ 90 (done 76 ms) | Dia rec 60 fps |
| [ ] | Drag to another window / desktop | MVP | moves the live page; new window under the pointer | Dia P§1 |
| [ ] | Drops on the list | MVP | files and http(s) open at the gap; never javascript:, data:, file: from text | Dia, Ardia |
| [ ] | Hover card show | MVP | delay 0.5 s; opacity 100 ms + scale spring 0.92 → 1 (0.25 s, ζ 0.75) | Dia bin |
| [ ] | Hover card dismiss | MVP | opacity 100 ms + spring to 0.92 (0.2 s, ζ 0.6); click or key dismisses | Dia bin |
| [ ] | Background tabs | MVP | load when first shown; sidebar scrolls to them | Dia |
| [ ] | Opener order | MVP | Chrome's TabStripModel rules | Chrome |
| [ ] | ⌘S hide (docked) | MVP | ease-out cubic 100 ms | Arc (Ardia) rec |
| [ ] | ⌘S show (hidden) | MVP | measure (*interim* Dia spring 0.215 s / ζ 0.79) | Dia rec 240 fps |
| [ ] | Sidebar reveal in | MVP | spring 0.185 s / ζ 0.77, no fade | Arc (Ardia) rec |
| [ ] | Sidebar reveal out | MVP | ease-in-out ≈ 150 ms | Arc (Ardia) rec |
| [ ] | Reveal panel | MVP | inset 5 left/top/bottom, docked width, 0.5 pt light border | Arc (Ardia) rec |
| [ ] | Reveal hold zone | MVP | 24 pt past the panel (Arc ≈ 21); trigger the full left edge | Arc (Ardia), Ardia |
| [ ] | Dock from the reveal | MVP | instant | Arc (Ardia) rec |
| [ ] | Tab switcher ⌃Tab | MVP | most recent first; overlay after 140 ms; commits on ⌃ release | Dia 1.50.1 bin P§2 |
| [ ] | ⌘1–⌘8 / ⌘9 | MVP | sidebar order, a split is one row | Dia |
| [ ] | Rename and emoji icon | MVP | double-click renames inline | Dia P§2 |
| [ ] | Folders | MVP | measure (Dia groups: chevron 8 after the name, ≈ 150 ms) | Dia rec |

## Spaces

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Space model | MVP | name, colour, pinned, Today, profile; many Spaces per profile | decision 5 |
| [ ] | Switching to another profile's Space | MVP | the window's profile switches with it | decision 5 |
| [ ] | Moving a tab across profiles | MVP | reloads after a one-time warning | Dia P§1 |
| [ ] | Favorites per Space or per profile | MVP | measure | — |
| [ ] | Switcher footer | MVP | 41 tall; library cell left; overflow 42 wide, 3 from right | Dia 1.50.1 bin |
| [ ] | Switcher band | MVP | 28 tall, 31 in from each side, 12 pt per item, 7 before scrolling, edge scale 0.75–1 | Dia 1.50.1 bin |
| [ ] | Switcher dot | MVP | 6 pt; alpha 0.85 / 0.4 / 0.2 | Dia 1.50.1 bin |
| [ ] | Space icons in the switcher | MVP | measure | — |
| [ ] | Swipe tracking | MVP | −0.5 × horizontal delta; ignores the system swipe setting | Dia 1.50.1 bin |
| [ ] | Rubber band | MVP | `255·(1 − 1/(0.15·x/255 + 1))` | Dia 1.50.1 bin |
| [ ] | Release | MVP | ≥ 5 pt/s goes on; ±1 Space; 100 ms velocity history | Dia 1.50.1 bin |
| [ ] | Settle | MVP | critically damped 0.25 s, carrying the velocity; detent haptic | Dia 1.50.1 bin |
| [ ] | Wheel mice | MVP | sum deltas < 0.049 s apart; > 1 pt pages once; ignore 0.25 s | Dia 1.50.1 bin |
| [ ] | ⌃1–⌃9 | MVP | switch Space | Dia (as profiles) |
| [ ] | Colour cross-fade | MVP | measure | Ardia |
| [ ] | Space title, create, rename, colour, icon | MVP | measure | — |
| [ ] | Agent Space kind in the data model | MVP | a Space kind owned by an agent | decision 8, roadmap |
| [ ] | Agent Space UI: switcher mark, confirm card, handoff card, take over, status, activity log, allowed sites with Revoke | 4 | ours to design; confirm times out 5 min, handoff 15 min | decision 8, Ours (prototype) |
| [ ] | Agent actions never move the user's focus | 4 | no change of Space, tab, focus or cursor | decision 8 |

## Command bar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Panel width | MVP | field + ≈ 41 left, ≈ 40 right; Arc: measure | Dia 1.52 rec |
| [ ] | Text alignment | MVP | same x, baseline and 13 pt as the field; 48 in; glyph centred 30 in | Dia 1.52 rec |
| [ ] | Open | MVP | grows from the field, exponential ease-out ≈ 130 ms; content doesn't move | Dia 1.52 rec |
| [ ] | Close | MVP | measure | — |
| [ ] | Fill / border / radius | MVP | #2D2D2D / #FEFFFF; 1 device px rgba(120,125,134,0.32); radius measure (≈ 16) | Dia 1.50.1 bin, 1.49 cap |
| [ ] | Shadow | MVP | black 0.12, + black 0.08 r2 (0, 0.5), + black 0.04 r1 (0, 2) | Dia 1.50.1 bin |
| [ ] | No scrim | MVP | page undimmed | Dia 1.49 cap |
| [ ] | Rows | MVP | input 54; suggestions 36; favicon 16; title ≈ 15 pt; " — url" 0.5 | Dia 1.49 cap |
| [ ] | Selected row | MVP | ≈ #5E5A5D, radius 10, inset 4 | Dia 1.49 cap |
| [ ] | Bottom row | MVP | 53 tall; Go ↩ pill ≈ 92 × 29 naming the engine; no AI chip or mic | Dia 1.49 cap, Ardia |
| [ ] | Colours | MVP | placeholder white 0.53, secondary rgba(239,244,255,0.6), caret #4A77D4, selection rgba(120,125,134,0.7) | Dia asset catalogs |
| [x] | Default match | MVP | inline match, else URL, else search; search row after the top hit | Chrome, Dia 1.49 |
| [x] | Stable row 0 | MVP | local only; remote rows append | Chrome O |
| [x] | Inline completion rules | MVP | one term, caret at end; never during IME, after delete, after paste, after whitespace | Chrome O |
| [x] | Inline eligibility | MVP | typed ≥ 1 host / ≥ 2 deeper | Chrome O |
| [x] | Well-known sites | MVP | built in; fill the rows left under the profile's own, never row 0 | ours |
| [ ] | Site search | MVP | Tab when hinted, Space after exact host; ⌫ in empty leaves and restores | Chrome O |
| [ ] | Keys | MVP | ↩, ⌘↩, ⌥↩, ⇧↩, ⇧⌘↩, ⌃↩, ⌃⇧↩, ↑↓ wrap, ⌃N/⌃P, ⇞⇟, ⇧⌦, Esc | Chrome, Dia O |
| [ ] | ⌘V after ⌘T / ⌘L / ⌘F | MVP | lands in the new field (≤ 400 ms) | Ardia P§7 |
| [ ] | Click outside | MVP | closes and still acts | AGENTS.md |
| [ ] | Switch to Tab rows | MVP | across windows, never the current tab | Dia, O |
| [ ] | URL parsing | MVP | Chrome's Parse; https first, http fallback; local hosts http | Chrome O |
| [ ] | Calculator, commands, Paste and Go, clean-link copy ⇧⌘C / ⌥⇧⌘C | MVP | as Dia | Dia P§7 |
| [ ] | Arc's command list and ⌘T over the page | MVP | measure | — |

## New tab

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Drawn by the UI, no renderer until navigation | MVP | yes | decision 4 |
| [ ] | Box width | MVP | 652 (708–1700), 774 above, min(w − 20, 616) below | Dia bin |
| [ ] | Box position | MVP | x max(mid − w/2, 14) + 1; top max(h/2 − 158, 100) + 38 + 1 | Dia bin + cap |
| [ ] | Box size and look | MVP | 112 tall, radius 20 continuous, panel fill, border and shadow | Dia 1.50.1 |
| [ ] | Box contents | MVP | magnifier 30.5 from top, ≈ 30 in; bottom row 30.5 from bottom; Go pill inert while empty | Dia 1.50.1 cap, Ardia |
| [ ] | Mark slot | MVP | 84 view, 76 mark, centre 48.6 above the box; our mark or none | Dia 1.50.1 |
| [ ] | Intro band | MVP | Space colour, blur 24, peak 0.10–0.20 s, plateau to 1.2 s, gone 3.25 s | Dia 1.50.1 cap |
| [ ] | Intro halo | MVP | bottom 225 ms → top 376 ms → gone 529 ms; line × 0.4, glow × 0.1 / 0.12 | Dia 1.50.1 bin |
| [ ] | Reduce Motion | MVP | no intro | Dia |
| [ ] | Idle New Tab does no work | MVP | no display link after the intro | principle 1 |
| [ ] | Abandoned New Tabs close | MVP | on app resign or screen lock | Dia |
| [ ] | Query restored on Back | MVP | yes | Dia |

## Split view

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Panes | MVP | up to 3, horizontal or vertical *interim*; Arc max: measure | Dia P§6 |
| [ ] | Divider or gap, focus indicator, unfocused dim | MVP | measure | — |
| [ ] | Keys | MVP | ⌃⇧= add; ⌃⇧] / ⌃⇧[ focus *interim* | Dia |
| [ ] | ⌥-click + New Tab; ⌥⇧-click link | MVP | split New Tab; right pane | Dia P§6 |
| [ ] | Sidebar row | MVP | one row, a segment per pane | Dia |
| [ ] | Targets at rest | MVP | 198.5 × 333.5, 37.5 in, radius ≈ 20, dashed 6/6 ≈ 1.75 pt | Dia rec |
| [ ] | Targets grown | MVP | 322 × 532, 30 in | Dia rec |
| [ ] | Target under the card | MVP | 362 × 600, leaning 43 % toward the pointer, accent ≈ 0.15, dashes 7/7 ≈ 2 pt | Dia rec |
| [ ] | Target timing | MVP | slide ≈ 110 ms, grow ≈ 120 ms, no overshoot | Dia rec |
| [ ] | Drag picture | MVP | tab for 1–2 frames, then 209 × 109.5 window card | Dia rec |
| [ ] | Drop away from a target | MVP | new window grows from the card ≈ 200 ms, takes focus | Dia rec |
| [ ] | Commands | MVP | Flip, Change Orientation, Move Pane Left/Right, Remove, Separate All | Dia P§6 |

## Archive

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Clear (⇧⌘K and the divider button) | MVP | closes all Today tabs of the Space; pinned and favorites stay | Arc (Ardia) |
| [ ] | Clear toast with Undo | MVP | Undo restores all in place; look and timing: measure | Arc (Ardia), Ardia |
| [ ] | Clear unselected Today tabs | MVP | as Arc's `clearOthers` | Arc (Ardia) bin |
| [ ] | Reopen closed tab ⇧⌘T | MVP | position, pin, group, name, icon, mute, back/forward | Dia P§2 |
| [ ] | Auto-archive interval and default | MVP | measure | — |
| [ ] | ⌘W on a Today tab | MVP | measure | — |
| [ ] | Archive view, search, restore | MVP | measure | — |

## Profiles

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Profile holds cookies, history, logins, autofill, extensions | MVP | yes | decision 5 |
| [ ] | Create, rename, delete, pick for a Space | MVP | measure | — |
| [ ] | Where a Space shows its profile | MVP | measure | — |
| [ ] | Incognito ⇧⌘N | MVP | per profile, dark, one session per profile, nothing written | Chrome, Dia |

## Extensions

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Pinned actions | MVP | 16 pt icons, 30.25 pitch, no puzzle piece, overflow when full *interim*; Arc placement: measure | Dia 1.52 cap |
| [ ] | Extensions menu in the menu bar | MVP | per extension, Add, Manage, Pin | Dia |
| [ ] | Popups | MVP | 25 × 25 to 800 × 600, shrink and grow; openPopup works | Chrome |
| [ ] | Action state without polling | MVP | from change notifications only | principle 1 |
| [ ] | Side panel | MVP | card right of the page, header, 320–600 saved | Dia, Ardia |
| [ ] | Install, re-enable, new-permission dialog | MVP | ours, Chrome's warnings; store buttons "Add to / Remove from <browser>"; Arc's: measure | Dia P§15 |
| [ ] | Remove | MVP | "Remove “<name>”?", Cancel / destructive Remove | Dia P§15 |
| [ ] | Keyboard commands | MVP | fire with the page focused; never reserved keys; clashing suggestions not assigned | Chrome P§15 |
| [ ] | Site Controls | MVP | Ardia's layout (58 pt cells, 8 gaps, rows of 4, open 150 / close 100 ms); Arc's: measure | Ardia |

## Passwords and autofill

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Save / update prompt | MVP | page top right; Save / Update with username picker / Never / Not Now; look: measure | Dia P§16 |
| [ ] | Address and card offers | MVP | same prompt, Chrome's words | Chrome |
| [ ] | Pending offer on tab switch | MVP | closes | Dia P§16 |
| [ ] | Fill dropdown | MVP | Chrome's, under the field | Chrome |
| [ ] | Edit › AutoFill items | MVP | open the dropdown at the focused field, else settings | Dia |
| [ ] | Password generation | MVP | Use Strong Password / Choose Your Own | Chrome |
| [ ] | Eye reveal in pages | MVP | once the user types, never for a filled password | Dia |
| [ ] | Settings unlock | MVP | one device check, 5 min grace | Chrome |
| [ ] | Passkey dialogs | MVP | Chrome's, centred over the page, as child windows | Chrome |
| [ ] | No agent path | MVP | agents never read or fill these | decision 8 |

## Downloads

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Button | MVP | shown while the profile has downloads; no show/hide animation; place in Arc's layout: measure | Dia 1.50.1 bin |
| [ ] | Fly-in | MVP | measure (Dia constants 0.1 s / 1.3 undecoded) | Dia bin |
| [ ] | Panel ⇧⌘J | MVP | progress, pause/resume, cancel, open, Show in Finder, Open When Done, Trash, drag out, clear | Dia P§14 |
| [ ] | Per profile, persisted; private never written | MVP | yes | Dia P§14 |
| [ ] | Quit guard | MVP | asks while downloads run | Dia P§1 |
| [ ] | Panel look and motion | MVP | measure | — |

## Find in page

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Keys | MVP | ⌘F, ⌘G, ⇧⌘G, ⇧↩, ⌘E, ⌘J, ⌥⌘F | Dia, Chrome P§8 |
| [x] | Per-tab query, "n of m" | MVP | yes | Dia P§8 |
| [x] | ⌘F with the page focused | MVP | ours unless the page keeps it | AGENTS.md |
| [x] | IME in the field | MVP | Japanese works | AGENTS.md |
| [ ] | Bar position, size, look, no-match state, motion | MVP | measure | — |

## DevTools

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Keys | MVP | ⌥⌘I / F12, ⌥⌘J, ⌥⌘C, ⌥⌘U; Inspect in the page menu | Dia, Chrome P§23 |
| [ ] | Docking | MVP | right by default, saved per profile; undocked 640 × 640 first, "Developer Tools - <url>" | Dia P§23 |
| [ ] | Task Manager | MVP | Window › Task Manager | Chrome |
| [ ] | No agent DevTools or debugging port | MVP | none | decision 8 |

## Import

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Arc: Spaces → Spaces, profiles → profiles | MVP | yes | decision 5 |
| [ ] | Arc: favorites, pinned tabs, custom names | MVP | favorites → favorites; pinned → pinned rows | Dia 1.52 P§20 |
| [ ] | Arc: themes | MVP | `windowTheme` colours, intensity, noise, texture, appearance, light and dark | Arc 1.167 P§20 |
| [ ] | Arc: folders | MVP | measure | — |
| [ ] | Chrome: bookmarks, history, open tabs, passwords, addresses, cards | MVP | yes | P§20 |
| [ ] | Chrome: cookies | MVP | never over existing ones, never into private | P§20 |
| [ ] | Chrome: extensions | MVP | listed with store links, not installed | P§20 |
| [ ] | Full Disk Access | MVP | explain, link to System Settings, re-check on return | P§20 |
| [ ] | Rerun | MVP | adds nothing | P§20 |
| [ ] | Where Chrome bookmarks land | MVP | measure | — |

## Updates

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Check for Updates… | MVP | app menu | Dia, Ardia P§24 |
| [ ] | Automatic checks | MVP | every 8 h (Ardia) | Ardia |
| [ ] | No install during full screen | MVP | waits | Ardia P§24 |
| [ ] | Release notes after an update | MVP | first launch | Ardia P§24 |
| [ ] | Feed URL is a default service URL | MVP | forks can change it | principle 2 |

## Keyboard

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Reserved commands before the page | MVP | Chrome's reserved set | Chrome, AGENTS.md |
| [ ] | Other browser keys only if the page doesn't keep them | MVP | yes | Chrome, AGENTS.md |
| [ ] | Arc's keys win clashes | MVP | ⌘D pin, ⇧⌘K Clear, ⌘S sidebar; owner to confirm | Arc (Ardia) |
| [ ] | Every command rebindable | MVP | Settings › Keyboard Shortcuts | Dia P§22 |
| [ ] | Full table in the spec | MVP | ux-spec.md › Keyboard | Dia, Chrome |
| [ ] | Arc's keys not yet recorded | MVP | measure | — |

## Phase 4

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Peek: triggers | 4 | measure | — |
| [ ] | Peek: geometry, backdrop, buttons | 4 | measure | — |
| [ ] | Peek: open, close, expand motion | 4 | measure | — |
| [ ] | Little Arc: open | 4 | ⌥⌘N; centred above middle of the pointer's screen; last size, 900 × 640 first | Arc (Ardia), Ardia |
| [ ] | Little Arc: bar | 4 | Space colour; favicon, host, title; ⌘L compact bar; Copy Link; ⌘O | Arc (Ardia), Ardia |
| [ ] | Little Arc: ⌘O | 4 | moves the live page to the main window | Arc (Ardia) |
| [ ] | Little Arc: close | 4 | ⌘W, close, unused Esc drop the page; ⇧⌘T reopens there | Ardia |
| [ ] | Little Arc: links from other apps | 4 | new Little Arc each time, in the last-used Space; setting for a tab | Ardia, decision 5 |
| [ ] | Little Arc: visuals and motion | 4 | measure | — |
| [ ] | Themes: rendering | 4 | OKLab 45° mix to 75 %; third colour top → bottom; alpha 0.2 + 0.55 × intensity | Arc 1.167 P§21 |
| [ ] | Themes: texture | 4 | noise × strength (0.8 / 0.15 / 0.6 / 0.4), quarter when inactive; our own textures | Arc 1.167 |
| [ ] | Themes: editor | 4 | 1–3 handles, intensity, translucency, noise, appearance; live; layout: measure | Arc 1.167 P§21 |
| [ ] | Appearance menu | 4 | Automatic / Light / Dark | Dia |

## Later (picked by the owner, not on the roadmap)

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Boosts: CSS, script and Zap per site | later | measure | Arc |
| [ ] | Boosts: storage | later | plain files per site in a folder, kept in git if the user likes; no account, no gallery | Flecha, principle 2 |
| [ ] | Boosts: applying | later | CSS live; scripts on the next load | Flecha |
| [ ] | Boosts: seeing and turning off | later | listed in Site Controls with an off switch; all off in Settings; look: measure | Arc |
| [ ] | Boosts: script changed outside the browser | later | stays off until the user turns it on again | principle 3 |
| [ ] | Boosts: no agent path | later | agents can't read, write or turn on a Boost | decision 8 |
| [ ] | Ad blocking: on by default | later | every profile | Helium, Flecha |
| [ ] | Ad blocking: engine | later | Helium's uBlock Origin fork *interim*; MV2 works on Chromium 155 through ungoogled-chromium's patch | Helium |
| [ ] | Ad blocking: turning it off | later | per site in Site Controls; everywhere in Settings | Ours |
| [ ] | Ad blocking: filter lists | later | updates never wake an idle browser; list URL is a default service URL | principles 1, 2 |
| [ ] | Ad blocking: cost | later | page-load and memory numbers against the performance gate | principle 1 |
| [ ] | Ad blocking: no agent path | later | agents can't change it | decision 8 |
| [ ] | Environments: setup | later | add, edit, remove a project's origins in Settings | Flecha |
| [ ] | Environments: switching | later | ⌥⌘E *interim*; only the origin changes; command bar lists the other origins | Flecha |
| [ ] | Environments: badge | later | in the URL field; production red | Flecha |
| [ ] | Environments: agents on production | later | ask first, as for pay and send; owner to confirm | Ours |

## Later (measured in Dia, not on the roadmap)

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | PiP at rest | later | 6 pt corners, 1 pt white 0.17 rim; appears ≈ 125 ms after the switch | Dia 1.50.1 rec |
| [ ] | PiP hover | later | scrim black 0.35 over 200 ms; buttons 28 × 28 / 29 × 28, 12 in, white 0.09 / 0.13 | Dia 1.50.1 rec |
| [ ] | PiP throw | later | > 350 pt/s or swipe delta > 6 pt; corner ±45°; 10 in; spring 0.9 s / ζ 0.9 | Dia 1.53.0 bin |
| [ ] | Swipe back/forward overlay | later | follows the system setting; scrollable carousels keep it | Dia P§8 |
