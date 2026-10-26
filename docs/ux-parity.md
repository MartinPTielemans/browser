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
| [x] | Blur behind the window | MVP | material 29, emphasized, following the active state | Dia 1.50.1 bin, D›Window translucency |
| [x] | Base tint | MVP | black 0.4 | Dia 1.50.1 bin |
| [x] | Space gradient | MVP | colour → HSL L + 0.25, stops 0.36 (neutral 0.12), layer 0.5; system accent colour when the Space has none | Dia 1.50.1 bin |
| [x] | Card fill | MVP | #121212 0.5, no grain | Dia 1.50.1 bin |
| [ ] | Composite matches Dia's within 2 levels | MVP | dark `0.18·tint + 0.82·0.6·blur`; light `0.27·tint + 0.73·(204 + 0.2·blur)` | Dia 1.50.1 cap |
| [ ] | Inactive, Reduce Transparency, Increase Contrast | MVP | opaque wallpaper-tinted fill (AppKit) | Dia 1.50.1 cap |
| [x] | Card inset | MVP | 10 on every side, docked or hidden; docked, the 10 on the left is inside the sidebar | Arc 1.170.0 ax and cap |
| [x] | Card radius | MVP | 6, circular, on the card and the page in it | Arc cap |
| [x] | Traffic lights | MVP | centres (20, 24), (43, 24), (66, 24) | Arc 1.170.0 ax |
| [x] | Page before first paint | MVP | card colour over the tint, never white, in every page view a tab swaps in | Ardia P§7 |
| [x] | Window drag regions and double-click action | MVP | empty sidebar, header, card margins; system title-bar action; empty sidebar double-click = new tab | Dia 1.50, Arc (Ardia) P§1 |
| [ ] | Resize edge | MVP | outer ≈ 3 pt | Dia P§1 |
| [ ] | New window / closed window | MVP | at its final frame with content / gone at once *interim*; Arc's ⌘N: measure | Dia P§1 |
| [ ] | Close confirmations | MVP | "Close 3 tabs?" per Space with "Don't ask me again". Arc: measure | Dia P§1 |
| [x] | Page full screen | MVP | sidebar gone; Esc or exitFullscreen restores; tab switch ends it; no stale frame | Dia P§18 |
| [ ] | Window full screen with the sidebar | MVP | measure | — |
| [x] | Toast | MVP | at the window's top right, 46.5 tall, 4.5 and 4 in; medium text with a key's chip; grows from 0.7 on spring 0.32 s / ζ 0.52, fades in over 100 ms, leaves over 70 ms. Text size and fill *interim* | Arc 1.170 rec |
| [ ] | Toast's other uses | MVP | Copy URL's note and a tab archived by ⌘W: Arc's words and timing, measure | Arc |

## Sidebar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Default width | MVP | 228 on a fresh install | Arc 1.170.0 ax and preferences |
| [x] | Width range | MVP | 160–400 *interim* (Ardia); rubber-band; saved; double-click resets; Arc's: measure | Ardia, Dia P§4 |
| [x] | Header row centre | MVP | traffic lights 24 below the window top, button plates 23.5 | Arc 1.170.0 ax |
| [x] | Header hover plates | MVP | 32 × 32, top 7.5, radius 6; only the plate clicks | Arc 1.170.0 ax, Ardia P§4 |
| [x] | Sidebar button, back, forward, reload positions | MVP | plates at x 77 / 122 / 156 / 190 in a 228 sidebar; the same at other widths *interim*, Arc's: measure | Arc 1.170.0 ax |
| [x] | Narrow sidebar | MVP | reload hides first, then forward | Ardia |
| [x] | Disabled nav glyph | MVP | white 0.26 against 0.60; reload too with no page | Arc 1.170.0 cap |
| [x] | Nav glyph size | MVP | sidebar 18.5 × 14.5, back and forward 14.5 × 12, reload 14 across; 1.75 pt strokes | Arc 1.170.0 cap |
| [x] | URL field top | MVP | 46 below the window top | Arc 1.170.0 cap |
| [x] | URL field size, radius, fill, font | MVP | 36 tall, 8 in, radius 12, white 0.10; text 13 pt, white 0.60 | Arc 1.170.0 cap |
| [x] | URL text | MVP | host only, no `www.`; Arc has no Show Full URL | Arc 1.170.0 cap, Dia 1.50.1 bin |
| [x] | URL on hover | MVP | the field's buttons, not the full URL (Dia shows it) | Arc 1.170.0 cap |
| [x] | URL field Copy URL button | MVP | 30 × 30, while the pointer is over the sidebar; Site Control Center: later | Arc 1.170.0 cap |
| [ ] | Insecure indicator | MVP | Arc: none; Dia and Chrome: a lock-warning glyph | Dia, Chrome P§8, Arc 1.170.0 cap |
| [x] | Favorite tile | MVP | 41 tall, 6 apart, inset 8, radius 10 *interim* | Dia 1.50.1 bin + cap; inset ours |
| [x] | Favorite columns | MVP | as many as fit at ≥ 48, fewer share the row: 3 at 190, 4 at 228 *interim*; Arc's rule: measure | Dia P§2; rule ours |
| [x] | Favorite resting look | MVP | fill white 0.10, 0.5 pt stroke white 0.14 | Dia 1.50.1 bin |
| [ ] | Favorite selection theming | MVP | colourful (≥ 0.055): blur 5 at 22 %, 3 pt ring; one colour: mean fill, white template, soft-light ring; near-white: black fill, white 0.30 ring | Dia 1.50.1 bin |
| [x] | Favorites fixed while the list scrolls | MVP | yes | Dia |
| [ ] | Tall favorites block | MVP | scrolls with the list above 40 % of the sidebar | Ardia |
| [x] | Favorite double-click | MVP | back to the favorite's URL | Dia P§2 |
| [x] | Favorites in a tab's menu | MVP | Add to Favorites; the tile's menu: Back to Favorite URL, Duplicate Tab, Remove from Favorites, Close Tab *interim* | ours |
| [x] | Favorite ⌘W | MVP | unloads, keeps the tile | as pinned tabs |
| [x] | Drag into, out of and between favorites | MVP | a row over the tiles is a tile at the place nearest its centre, a tile below them a row; tiles make room on the reorder spring *interim* | Arc |
| [ ] | Row ↔ tile morph while dragging | MVP | measure | Arc |
| [x] | Pinned tabs section | MVP | rows 6 under the tiles, above the Clear divider | Dia 1.52, Ardia |
| [x] | Pinned tab ⌘W | MVP | unloads, keeps the row | Dia rec P§2 |
| [ ] | Pinned tab away from its URL | MVP | ↩ on hover, "Back to Pinned URL", ⌘↩ *interim*; Arc's look: measure | Dia 1.53.0 bin |
| [ ] | Pinned tab in two windows | MVP | one live page; frozen frame washed white 0.40 / black 0.25 | Dia cap, Ardia |
| [x] | Clear divider line | MVP | one device pixel, white 0.15, 18 in from each side; ends 7.5 before Clear while it shows; centre 25 above the first row's centre | Arc 1.170.0 cap and tokens, Arc (Ardia) |
| [ ] | Clear label and arrow | MVP | arrow 7 × 8, 3 pt from "Clear", semibold ≈ 10 pt, ends with the titles | Arc (Ardia) |
| [x] | Clear on hover | MVP | shows only while the pointer is over the sidebar | Arc 1.170.0 cap |
| [ ] | Sidebar hover fades | MVP | Clear and the URL field's buttons coming and going with the pointer: measure | Arc |
| [x] | Row pitch / height | MVP | 41 / 40 | Arc 1.170.0 ax |
| [x] | Row plate | MVP | 36 tall, x 8 to width − 8, radius ≈ 12 | Arc 1.170.0 cap |
| [ ] | Row favicon and title | MVP | favicon 18 at x 17, title at x 43; the title's trailing fade (ours: an ellipsis) | Arc 1.170.0 ax |
| [x] | Row states | MVP | selected white 0.20, hover 0.12, pressed 0.065; no line or shadow in dark mode; unselected none | Arc 1.170.0 tokens and cap |
| [x] | Row titles | MVP | selected white 1, unselected 0.78 | Dia asset catalogs |
| [x] | Row fill change | MVP | 300 ms ease-out, as the hover fill leaves *interim*; Arc's: measure | Dia 1.50.1 bin |
| [x] | Close ✕ | MVP | on hover, the selected row's too; 26 × 26, 5 inside the plate's right edge | Arc 1.170.0 cap |
| [x] | Loading spinner | MVP | 12 × 12, 1.5 pt, ring 72 %, track 0.18, clockwise 1.88 s linear; only in rows ≥ 48 wide | Dia 1.50.1 bin |
| [ ] | Sleeping tab | MVP | faded favicon (alpha: measure) | Dia |
| [x] | "+ New Tab" row | MVP | the first row under the divider, where new tabs open below it (Arc); label white 0.55, plus 0.25; hover as rows | Arc, Dia 1.50.1 bin |
| [x] | Scroll edge | MVP | 1 pt P3 white 0.09 at favorites + 4; fades 100 ms cubic-bezier(0.25, 0.46, 0.45, 0.94) *interim*, Arc's: measure | Dia 1.53.0 bin |
| [x] | ⌘T's plate | MVP | "+ New Tab" has the selected plate while ⌘T's bar is open, in one frame | Arc 1.170.0 rec 120 fps |
| [x] | Open tab motion | MVP | new row in place at once, its plate fading in; rows below slide; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Close tab motion | MVP | the row fades as the rows below slide up over it; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Undo motion | MVP | the tabs Undo brings back fade in in place; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Reorder | MVP | neighbour moves at the centre crossing; critically damped ω ≈ 90 (done 76 ms) *interim* | Dia rec 60 fps |
| [ ] | Reorder and drop, Arc's | MVP | the neighbours making way and the dropped row settling: measure | Arc |
| [ ] | Drag to another window / desktop | MVP | moves the live page; new window under the pointer | Dia P§1 |
| [ ] | Drops on the list | MVP | files and http(s) open at the gap; never javascript:, data:, file: from text | Dia, Ardia |
| [ ] | Hover card show | MVP | delay 0.5 s; opacity 100 ms + scale spring 0.92 → 1 (0.25 s, ζ 0.75) | Dia bin |
| [ ] | Hover card dismiss | MVP | opacity 100 ms + spring to 0.92 (0.2 s, ζ 0.6); click or key dismisses | Dia bin |
| [ ] | Background tabs | MVP | load when first shown; sidebar scrolls to them | Dia |
| [ ] | Opener order | MVP | Chrome's TabStripModel rules | Chrome |
| [x] | ⌘S hide (docked) | MVP | ease-out cubic 110 ms | Arc 1.170.0 rec 120 fps |
| [x] | ⌘S show (hidden) | MVP | ease-out cubic 100 ms | Arc 1.170.0 rec 120 fps |
| [x] | Sidebar reveal in | MVP | after ≈ 100 ms at the edge; spring 0.175 s / ζ 0.80, no fade, ≈ 4 pt overshoot | Arc 1.170.0 rec 120 fps |
| [x] | Sidebar reveal out | MVP | spring 0.175 s / ζ 0.76 | Arc 1.170.0 rec 120 fps |
| [x] | Reveal panel | MVP | inset 5 left/top/bottom, docked width, 0.5 pt light border | Arc (Ardia) rec |
| [x] | Reveal hold zone | MVP | 24 pt past the panel (Arc ≈ 21); trigger the full left edge | Arc (Ardia), Ardia |
| [x] | Dock from the reveal | MVP | instant | Arc (Ardia) rec |
| [x] | ⌃Tab | MVP | the tab used last, at once, no switcher; with ⌃ held, on back (ours) | Arc rec |
| [x] | ⌘1–⌘8 / ⌘9 | MVP | sidebar order, a split is one row | Dia |
| [x] | Rename | MVP | double-click or Rename Tab renames inline; the name shows over the page's title and is kept across a quit; an empty one gives the title back | Dia P§2, ours |
| [ ] | Emoji icon | MVP | a row's icon changed to an emoji | Dia P§2 |
| [x] | Tab context menu | MVP | shortcut hints; Pin/Unpin, Back to Pinned URL, Rename, Duplicate, Move to Space, Close, Close Tabs Below *interim*; Arc's items: measure | Dia P§2 |
| [x] | Folders | MVP | in the pinned tabs, indented 16 a level: New Folder…, a click opens and closes, a double-click or the menu renames, Delete Folder keeps the tabs, a drag right or left takes a row in or out, kept over a quit; motion and icon *interim* | Arc ax, cap, menus |
| [x] | Tab shown in a closed folder | MVP | its row stays under the folder until another tab shows | Arc (confirm in the capture) |
| [ ] | Folder motion and drag | MVP | measure Arc's open and close by a click, and how a dragged row goes into a folder | — |
| [ ] | Folder tooltip | MVP | the name after a moment | Arc cap |
| [ ] | Favorite tooltip | MVP | the name after a moment; its delay, motion and the move to the next tile: measure | Arc cap |
| [x] | Collapse All Folders | MVP | a command bar command | Arc cap |

## Spaces

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Space model | MVP | name, colour, pinned, Today, profile; many Spaces per profile | decision 5 |
| [ ] | Switching to another profile's Space | MVP | the window's profile switches with it | decision 5 |
| [ ] | Moving a tab across profiles | MVP | reloads after a one-time warning | Dia P§1 |
| [x] | Favorites per Space or per profile | MVP | per profile; confirm in Arc | Arc |
| [x] | Switcher footer | MVP | 50 tall; Library 32 at 8 in on the left; + 32 at 8 in on the right, its menu New Space… and New Folder… *interim*, Arc's: measure | Arc 1.170.0 cap, ax |
| [x] | Switcher band | MVP | 42 in from each side; items 36 × 34 at a 34 pitch; none with one Space; more than fit: narrower *interim*, Arc's: measure | Arc 1.170.0 ax |
| [x] | Switcher dot | MVP | 8 pt; white 0.50 shown, 0.19 others | Arc 1.170.0 cap |
| [ ] | Space icons in the switcher | MVP | measure | — |
| [x] | Swipe tracking | MVP | −0.5 × horizontal delta; ignores the system swipe setting | Dia 1.50.1 bin |
| [x] | Rubber band | MVP | `255·(1 − 1/(0.15·x/255 + 1))` | Dia 1.50.1 bin |
| [x] | Past the first Space, the Library | MVP | fingers move its open, opening past halfway or on a flick; a wheel's tick opens it; back from it the same way. Arc's finger tracking: measure | Arc release notes, Arc 1.x video |
| [x] | Past the last Space, a new Space | MVP | the page pulls ≤ 41 (2×); a circle's ring closes at 38 and readies it, with Arc's morph and pop; let go readied makes the Space with its form, the page in from 46 over 170 ms. Arc's 4-point sparkle: ours is the dot growing into the + | Arc 1.x video |
| [x] | Release | MVP | ≥ 5 pt/s goes on; ±1 Space; 100 ms velocity history | Dia 1.50.1 bin |
| [x] | Settle | MVP | critically damped 0.19 s, carrying the velocity; detent haptic | Arc 1.170 rec |
| [x] | Wheel mice | MVP | sum deltas < 0.049 s apart; > 1 pt pages once; ignore 0.25 s | Dia 1.50.1 bin |
| [x] | ⌃1–⌃9 | MVP | switch Space | Dia (as profiles) |
| [ ] | Colour cross-fade | MVP | measure | Ardia |
| [ ] | Space title, create, colour, icon | MVP | measure | — |
| [x] | Rename and delete | MVP | native menu on the name or dot; double-click renames; delete asks in Arc's dialog if it has tabs | Arc, ours |
| [x] | Undo delete | MVP | the toast says "Space deleted! Use ⌘Z to undo." for 5 s; ⌘Z brings the Space back where it was, with its tabs, and pages to it | Arc 1.170 rec |
| [ ] | Undo delete motion | MVP | the switch's slide *interim*; Arc's: measure | Arc |
| [ ] | Agent Space kind in the data model | MVP | a Space kind owned by an agent | decision 8, roadmap |
| [ ] | Agent Space UI: switcher mark, confirm card, handoff card, take over, status, activity log, allowed sites with Revoke | 4 | ours to design; confirm times out 5 min, handoff 15 min | decision 8, Ours (prototype) |
| [ ] | Agent actions never move the user's focus | 4 | no change of Space, tab, focus or cursor | decision 8 |

## Command bar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Panel width | MVP | field + ≈ 41 left, ≈ 40 right; Arc: measure | Dia 1.52 rec |
| [x] | Text alignment | MVP | same x, baseline and 13 pt as the field; 48 in; glyph centred 30 in | Dia 1.52 rec |
| [x] | Open | MVP | grows from the field, exponential ease-out ≈ 130 ms; content doesn't move | Dia 1.52 rec |
| [ ] | Close | MVP | ⌘T's in one frame, as it opens; ⌘L's: measure | Arc 1.170.0 rec |
| [ ] | Selection motion | MVP | the selection moving with ↓ and ↑, and the pointer over the rows: measure | Arc |
| [x] | Fill / border / radius | MVP | #2D2D2D / #FEFFFF; 1 device px rgba(120,125,134,0.32); radius measure (≈ 16) | Dia 1.50.1 bin, 1.49 cap |
| [ ] | Shadow | MVP | black 0.12, + black 0.08 r2 (0, 0.5), + black 0.04 r1 (0, 2) | Dia 1.50.1 bin |
| [x] | No scrim | MVP | page undimmed | Dia 1.49 cap |
| [ ] | Rows | MVP | input 54; suggestions 36; favicon 16; title ≈ 15 pt; " — url" 0.5 | Dia 1.49 cap |
| [x] | Selected row | MVP | ≈ #5E5A5D, radius 10, inset 4 | Dia 1.49 cap |
| [ ] | Bottom row | MVP | 53 tall; Go ↩ pill ≈ 92 × 29 naming the engine; no AI chip or mic | Dia 1.49 cap, Ardia |
| [x] | Colours | MVP | placeholder white 0.53, secondary rgba(239,244,255,0.6), caret #4A77D4, selection rgba(120,125,134,0.7) | Dia asset catalogs |
| [x] | Default match | MVP | inline match, else URL, else search; search row after the top hit | Chrome, Dia 1.49 |
| [x] | Stable row 0 | MVP | local only; remote rows append | Chrome O |
| [x] | Inline completion rules | MVP | one term, caret at end; never during IME, after delete, after paste, after whitespace | Chrome O |
| [x] | Inline eligibility | MVP | typed ≥ 1 host / ≥ 2 deeper | Chrome O |
| [x] | Well-known sites | MVP | built in; fill the rows left under the profile's own, never row 0 | ours |
| [ ] | Site search | MVP | Tab when hinted, Space after exact host; ⌫ in empty leaves and restores | Chrome O |
| [x] | Keys | MVP | ↩, ⌘↩ behind with the bar open, ⌥↩ beside, ⇧⌘↩, ⌃↩, ↑↓ and Tab ⇧Tab wrap, ⌃N/⌃P, ⇞⇟, ⇧⌦, Esc | Arc 1.170.0 cap, Chrome, Dia O |
| [ ] | ⇧↩, ⌃⇧↩ | MVP | in a new window, with windows | Chrome |
| [ ] | ⌘V after ⌘T / ⌘L / ⌘F | MVP | lands in the new field (≤ 400 ms) | Ardia P§7 |
| [x] | Click outside | MVP | closes and still acts | AGENTS.md |
| [ ] | Switch to Tab rows | MVP | across windows, never the current tab | Dia, O |
| [ ] | URL parsing | MVP | Chrome's Parse; https first, http fallback; local hosts http | Chrome O |
| [x] | Commands | MVP | found by name, with their shortcuts; one whose name starts with what's typed comes first | Arc 1.170.0 cap |
| [x] | Calculator | MVP | the row after the first; ↩ copies the result | Dia P§7 |
| [ ] | Paste and Go | MVP | as Dia | Dia P§7 |
| [x] | Clean-link copy | MVP | Copy URL leaves out utm_ parameters and click IDs | Dia P§7 |
| [x] | Copy URL / as Markdown | MVP | ⇧⌘C / ⌥⇧⌘C; as Quote ⌃⇧⌘C later | Arc 1.170.0 menus |
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
| [x] | Panes | MVP | up to 3 side by side, sharing the card; stacked: to build; Arc max: measure | Arc rec, Dia P§6 |
| [x] | Gap, focus indicator | MVP | 10 pt gap; 2 pt ring inside the focused pane, ≈ 8 pt glow outside | Arc 1.170.0 rec |
| [ ] | Unfocused dim | MVP | none *interim*; measure | — |
| [x] | Keys | MVP | ⌃+ add, ⌃⇧- close pane; ⌃⇧] / ⌃⇧[ focus *interim* | Arc menus, Dia |
| [x] | Add Split View | MVP | in one frame: page narrows, empty pane, command bar over it 77 in from the card's edge | Arc 1.170.0 rec |
| [x] | ⌥↩ in the command bar | MVP | the URL in a right pane | Arc 1.170.0 rec |
| [ ] | ⌥-click + New Tab; ⌥⇧-click link | MVP | split New Tab; right pane | Dia P§6 |
| [x] | Sidebar row | MVP | one row, equal segments: favicon 18 at 7, title 10 after, 5 before the line | Arc 1.170.0 cap |
| [x] | Kept across a quit | MVP | yes | ours |
| [x] | Targets at rest | MVP | 198.5 × 333.5, 37.5 in, radius ≈ 20, dashed 6/6 ≈ 1.75 pt | Dia rec |
| [x] | Targets grown | MVP | 322 × 532, 30 in | Dia rec |
| [x] | Target under the card | MVP | 362 × 600, leaning 43 % toward the pointer, accent ≈ 0.15, dashes 7/7 ≈ 2 pt | Dia rec |
| [x] | Target timing | MVP | slide ≈ 110 ms, grow ≈ 120 ms, no overshoot | Dia rec |
| [x] | Drag picture | MVP | tab for 1–2 frames, then 209 × 109.5 window card; ours shows the favicon and title, not the page's image | Dia rec |
| [ ] | Pane motion | MVP | a dropped tab's pane coming in on its side, the divider dragged, a pane closing: measure | Arc |
| [ ] | Drop away from a target | MVP | new window grows from the card ≈ 200 ms, takes focus | Dia rec |
| [x] | Close pane, Separate | MVP | Close this Split Pane, Separate Page from Split View | Arc menus |
| [ ] | Expand Current Split | MVP | measure | Arc menus |
| [ ] | Commands | MVP | Flip, Change Orientation, Move Pane Left/Right, Separate All | Dia P§6 |

## Settings

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Settings window | MVP | 684 × 650, ⌘, and Settings…; toolbar of tabs with the accent on a plate; ⌘W and close hide it; opens on the tab shown last | Arc 1.170.0 cap, ax |
| [x] | Boxes, rows, switch, pop-up and buttons | MVP | the spec's measures; the pop-up's menu opens over it | Arc 1.170.0 cap, ax |
| [x] | Shortcuts tab | MVP | the browser's shortcuts by section, read-only | Ours |
| [x] | Advanced › More Settings… | MVP | opens Chromium's settings in a tab | Arc 1.170.0 cap |
| [x] | Haptics setting | MVP | Advanced: haptic feedback when swiping between Spaces, kept with the Spaces | Ours |
| [x] | Warn before quitting | MVP | General's switch, on; Quit and ⌘Q ask "Quit Browser?" with Quit, "Quit, and don't ask again" and Cancel; Dock and logout quits don't | Arc 1.170.0 cap, bin |
| [ ] | Arc's other tabs | MVP | Profiles, Links, Icon, Previews: with their features | Arc 1.170.0 cap |

## Archive

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Clear (⇧⌘K and the divider button) | MVP | closes the Space's Today tabs but the one shown, in one frame, no toast; pinned and favorites stay | Arc 1.170.0 rec |
| [x] | Undo Clear | MVP | ⌘Z brings them back in place, unloaded; the tab shown stays | Arc 1.170.0 rec |
| [ ] | Reopen closed tab ⇧⌘T | MVP | position, pin, group, name, icon, mute, back/forward | Dia P§2 |
| [x] | Auto-archive | MVP | a Today tab unseen for 12 hours goes in the archive, counted across a quit | Arc 1.170.0 settings |
| [x] | Auto-archive's other intervals | MVP | Settings › General: 12 hours (default), 24 hours, 7 days, 30 days, kept with the Spaces | Arc 1.170.0 bin |
| [x] | ⌘W on a Today tab | MVP | Archive Tab: the tab goes in the archive | Arc 1.170.0 menus |
| [x] | Library | MVP | slides in from its width to the left as it pushes the card to 417 at its width, on spring 0.245 s / ζ 0.92; the sidebar fades out in 33 ms, and back in over 130 ms from 80 ms; column of sections, back button; ⇧⌘L, View Library, footer button | Arc 1.170.0 cap, ax, rec; Arc 1.x video |
| [x] | Archived Tabs | MVP | search on the buttons' row, day headings, rows 56 apart with favicon, title and host; a click reopens the tab | Arc 1.170.0 cap, ax |
| [x] | Clear Archive | MVP | Archive menu; asks first | Arc 1.170.0 menus |
| [ ] | Library's other sections | MVP | Downloads done; Media, Easels & Notes, Spaces as each feature lands | Arc 1.170.0 cap |
| [ ] | Archive filter, row hover and menu | MVP | measure | — |

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
| [ ] | Popups | MVP | 25 × 25 to 800 × 600, shrink and grow; openPopup works; Arc's opening and closing: measure | Chrome |
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
| [x] | Library section, ⇧⌘J, View Downloads | MVP | above Archived Tabs; rows as the archive's, with the system's icon, status and bar | Arc 1.170.0 menus, cap |
| [x] | Row actions | MVP | progress, pause/resume, cancel, open, Show in Finder, Open When Done, keep or delete a held file, Remove from List, Clear List | Dia P§14 |
| [ ] | Move to Trash, drag out | MVP | to Finder, other apps, the page, the tab list | Dia P§14 |
| [ ] | Footer button while downloading | MVP | ours: glyph over a 16 × 2 bar; Arc's look: measure | — |
| [ ] | Fly-in | MVP | measure (Dia constants 0.1 s / 1.3 undecoded) | Dia bin |
| [x] | Per profile, persisted | MVP | Chrome's history; private never written once private windows land | Dia P§14 |
| [x] | Quit guard | MVP | asks while downloads run, in Chrome's alert; Continue Downloading shows the Library's Downloads; Arc's wording: measure | Dia P§1, Chrome |

## Find in page

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | Keys | MVP | ⌘F, ⌘G, ⇧⌘G, ⇧↩, ⌘E, ⌘J, ⌥⌘F | Dia, Chrome P§8 |
| [x] | Per-tab query and count | MVP | yes; Arc's "1/12" | Dia P§8, Arc 1.170.0 cap |
| [x] | ⌘F with the page focused | MVP | ours unless the page keeps it | AGENTS.md |
| [x] | IME in the field | MVP | Japanese works | AGENTS.md |
| [x] | Bar position, size, contents, no-match state, motion | MVP | 300 × 32, 10 in from the card's top right, radius 6; search, field, count, previous, next, divider, close; NOT FOUND in red; one frame in and out | Arc 1.170.0 rec, cap |
| [ ] | Bar colour, shadow, glyphs | MVP | Arc (19,29,50) and ≈16 pt shadow; ours the window's colour and AppKit's shadow; glyphs: measure | Arc 1.170.0 cap |

## Zoom

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Keys and menu | MVP | ⌘0 / ⌘+ / ⌘-, hidden ⌘=; View › Zoom to Actual Size, Zoom In, Zoom Out; per site, kept | Arc 1.170.0 menus, Chrome |
| [x] | HUD | MVP | the percent over the card's top right for a moment; nothing in the URL field | Arc 1.170.0 cap |
| [ ] | HUD look and timing | MVP | Arc's pill, time shown and fade: measure | — |

## DevTools

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Keys and menu | MVP | ⌥⌘I, ⌥⌘J, ⌥⌘C, ⌥⌘U, from the page, DevTools and the UI; View › Developer | Arc 1.170.0 menus |
| [x] | Inspect in the page menu | MVP | Chrome's: docks DevTools at the element | Chrome P§23 |
| [ ] | F12 | MVP | Dia's and Chrome's | Dia, Chrome P§23 |
| [x] | Docked in the card | MVP | right, full card height, page on the left; each tab its own; closing gives the page the keyboard | Arc 1.170.0 cap |
| [ ] | Docked width | MVP | Arc a third of the card; ours Chrome's 555 pt, then the user's, kept per profile | Arc 1.170.0 cap |
| [x] | Undocked | MVP | Chrome's window, from DevTools' Dock side menu; Dia's 640 × 640 first, "Developer Tools - <url>" | Dia P§23, Chrome |
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
| [x] | Reserved commands before the page | MVP | Chrome's reserved set | Chrome, AGENTS.md |
| [x] | Other browser keys only if the page doesn't keep them | MVP | yes | Chrome, AGENTS.md |
| [x] | Arc's keys win clashes | MVP | ⌘D pin, ⇧⌘K Clear, ⌘S sidebar | Arc (Ardia) |
| [ ] | Every command rebindable | MVP | Settings › Keyboard Shortcuts | Dia P§22 |
| [ ] | Full table in the spec | MVP | ux-spec.md › Keyboard | Dia, Chrome |
| [ ] | Arc's keys not yet recorded | MVP | Back to Pinned URL and Search Tabs: measure; the rest are its menus' | Arc 1.170.0 menus |
| [x] | Menu bar | MVP | Arc's menus in Arc's order, with the commands the browser has | Arc 1.170.0 menus |
| [x] | Edit menu in pages | MVP | Undo, Redo, Cut, Copy, Paste, Select All reach the focused page or field | Arc 1.170.0 menus |
| [x] | Hide, Hide Others, Minimize | MVP | ⌘H, ⌥⌘H, ⌘M | Arc 1.170.0 menus |
| [x] | Save Page As, Print | MVP | ⇧⌘S, ⌘P: Chrome's save panel and print preview | Arc 1.170.0 menus |
| [x] | Clear Cache and Refresh, Clear Cookies and Refresh | MVP | View menu and command bar; the page's site only, then reload | Arc 1.170.0 menus, cap |
| [x] | Page context menu | MVP | Chrome's, without what the window can't do yet *interim*; its tab dialogs, such as print preview, centred over the page at its top | Chrome |
| [ ] | Arc's page menu items | MVP | measure | Arc |
| [ ] | Arc's other menu items | MVP | windows, Developer › Network Inspector and Toggle Developer Mode, Space 1–9, View History, Extensions, Help, as each feature lands | Arc 1.170.0 menus |

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
| [ ] | Appearance menu | 4 | Automatic / Light / Dark; light window: material 13, tint white 0.8, gradient layer 0.75, card white 0.7 | Dia, Dia 1.50.1 bin |

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
