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
| [x] | Base tint | MVP | black 0.4. Ours: black 0.25, for the composite (row below) | Dia 1.50.1 bin |
| [x] | Space gradient | MVP | colour → HSL L + 0.25, stops 0.36 (neutral 0.12), layer 0.5; system accent colour when the Space has none | Dia 1.50.1 bin |
| [x] | Card fill | MVP | #121212 0.5, no grain | Dia 1.50.1 bin |
| [ ] | Composite matches Dia's within 2 levels | MVP | dark `0.18·tint + 0.82·0.6·blur`; light `0.27·tint + 0.73·(204 + 0.2·blur)`. Built: material 29 reads darker in our window than Dia's measures, so our base tint is black 0.25 (Window material › Ours). Over black / grey 20 / grey 60 / white a check VM reads 23 / 27 / 34 / 70 against Dia's 24 / 29 / 36 / 69, within 2.5 levels, which `check/window.sh` holds to 3. Light comes with the Appearance menu | Dia 1.50.1 cap |
| [ ] | Inactive, Reduce Transparency, Increase Contrast | MVP | opaque wallpaper-tinted fill (AppKit). Built: the material follows the window's active state, so AppKit swaps the fill; no capture of an inactive window or either setting yet | Dia 1.50.1 cap |
| [x] | Card inset | MVP | 10 on every side, docked or hidden; docked, the 10 on the left is inside the sidebar | Arc 1.170.0 ax and cap |
| [x] | Card radius | MVP | 6, circular, on the card and the page in it | Arc cap |
| [x] | Traffic lights | MVP | centres (20, 24), (43, 24), (66, 24) | Arc 1.170.0 ax |
| [x] | Page before first paint | MVP | card colour over the tint, never white, in every page view a tab swaps in | Ardia P§7 |
| [x] | Window drag regions and double-click action | MVP | empty sidebar, header, card margins; system title-bar action; empty sidebar double-click = new tab | Dia 1.50, Arc (Ardia) P§1 |
| [ ] | Resize edge | MVP | outer ≈ 3 pt. Partly: the window is a standard resizable one, so the edge is AppKit's; it isn't measured against 3 pt and no check drags it | Dia P§1 |
| [x] | New window / closed window | MVP | at its final frame with content / gone at once; ⌘N cascaded from the key window, its size and Space, no page *interim*; a launch brings back every window at its frame, Space, sidebar width and tab, the others after the first's first frame, or after the last window closed, that one at its frame *interim*; Arc's ⌘N and launch: measure | Dia P§1 |
| [x] | Close confirmations | MVP | none: a closed window's tabs go to the window in front before it, live, so nothing is lost *interim*; Dia asks "Close 3 tabs?". Arc: measure | Dia P§1 |
| [x] | Page full screen | MVP | sidebar gone; Esc or exitFullscreen restores; tab switch ends it; no stale frame | Dia P§18 |
| [x] | Window full screen with the sidebar | MVP | the sidebar stays docked, the window's buttons come down with it, the command bar opens; "To exit full screen, press ⎋." 41 tall at the card's top right for 4.8 s, again per Space *interim*; ⎋ leaves *interim* | Arc 1.170 rec |
| [x] | Toast | MVP | 46.5 tall, 8 inside the card's top right corner; medium text with a key's chip; grows from 0.7 on spring 0.32 s / ζ 0.52, fades in over 100 ms, leaves over 70 ms. Text size and fill *interim* | Arc 1.170 rec |
| [ ] | Toast's other uses | MVP | ⇧⌘C's says "Copied Current URL" (Arc 1.170 rec), for 2 s *interim*; its share button comes with Share; ⌘W's note for an archived tab isn't shown; whether Arc has one: measure | Arc 1.170 rec |

## Sidebar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Default width | MVP | 228 on a fresh install | Arc 1.170.0 ax and preferences |
| [x] | Width range | MVP | 164–500, stops at each; past the minimum hides in one frame, keeping its width (threshold *interim*); saved; double-click resets to 228; Arc's bounce at the maximum not built | Arc 1.170 rec |
| [x] | Header row centre | MVP | traffic lights 24 below the window top, button plates 23.5 | Arc 1.170.0 ax |
| [x] | Header hover plates | MVP | 32 × 32, top 7.5, radius 6; only the plate clicks | Arc 1.170.0 ax, Ardia P§4 |
| [ ] | Every button under the pointer | MVP | glyph buttons the header's plate, rows the row fill, the URL field white 0.15, text buttons brighter, filled buttons 0.9 darker; Create Space and Settings' AppKit controls don't change; Arc's: measure. Built on every surface, the Space's icon last; to tick: a capture of ours | ours, the owner 2026-10-26 |
| [x] | Sidebar button, back, forward, reload positions | MVP | plates at x 77 / 122 / 156 / 190 in a 228 sidebar; the same at other widths *interim*, Arc's: measure | Arc 1.170.0 ax |
| [x] | Narrow sidebar | MVP | reload hides first, then forward | Ardia |
| [ ] | Nav glyphs | MVP | white 0.60; dimmed as disabled ones in an inactive window; reload turns to stop while loading. Built but the inactive window | Arc 1.170.0 cap |
| [x] | Disabled nav glyph | MVP | white 0.26 against 0.60; reload too with no page | Arc 1.170.0 cap |
| [x] | Nav glyph size | MVP | sidebar 18.5 × 14.5, back and forward 14.5 × 12, reload 14 across; 1.75 pt strokes | Arc 1.170.0 cap |
| [x] | URL field top | MVP | 46 below the window top | Arc 1.170.0 cap |
| [x] | URL field size, radius, fill, font | MVP | 36 tall, 8 in, radius 12, white 0.10; text 13 pt, white 0.60 | Arc 1.170.0 cap |
| [x] | URL text | MVP | host only, no `www.`; Arc has no Show Full URL | Arc 1.170.0 cap, Dia 1.50.1 bin |
| [x] | URL on hover | MVP | the field's buttons, not the full URL (Dia shows it) | Arc 1.170.0 cap |
| [x] | URL field buttons | MVP | Copy URL and Site Controls, 30 × 30 at x 163 and 191 of 228, while the pointer is over the sidebar | Arc 1.170.0 cap |
| [x] | A page's permission question | MVP | card under the URL field: "<site> wants to", Chrome's phrases, Block / Allow, × dismisses; follows its tab; brings out a hidden sidebar *interim*; Arc's look: measure | Chrome |
| [x] | Insecure indicator | MVP | none, as Arc's (Dia and Chrome show a lock-warning glyph); owner to confirm | Dia, Chrome P§8, Arc 1.170.0 cap |
| [x] | Favorite tile | MVP | 41 tall, 6 apart, inset 8, radius 10 *interim* | Dia 1.50.1 bin + cap; inset ours |
| [x] | Favorite columns | MVP | as many as fit at ≥ 48, fewer share the row: 3 at 190, 4 at 228 *interim*; Arc's rule: measure | Dia P§2; rule ours |
| [x] | Favorite resting look | MVP | fill white 0.10, 0.5 pt stroke white 0.14 | Dia 1.50.1 bin |
| [x] | Favorite selection theming | MVP | colourful (≥ 0.055): blur 5 at 22 %, 3 pt ring; one colour: mean fill, white template, soft-light ring; near-white: black fill, white 0.30 ring; emoji icons not yet | Dia 1.50.1 bin |
| [x] | Favorites fixed while the list scrolls | MVP | yes | Dia |
| [x] | Tall favorites block | MVP | scrolls with the list above 40 % of the sidebar | Ardia |
| [x] | Favorite double-click | MVP | back to the favorite's URL | Dia P§2 |
| [x] | Favorites in a tab's menu | MVP | Add to Favorites; the tile's menu: Back to Favorite URL, Duplicate Tab, Remove from Favorites, Close Tab *interim* | ours |
| [x] | Favorite ⌘W | MVP | unloads, keeps the tile | as pinned tabs |
| [x] | Drag into, out of and between favorites | MVP | a row over the tiles is a tile at the place nearest its centre, a tile below them a row; tiles make room critically damped, ω ≈ 25, settled in ≈ 240 ms | Arc 1.170 rec |
| [x] | Row ↔ tile morph while dragging | MVP | plate's size, corners and fill to the tile's on the tiles' spring, title out over the first half, favicon to the middle, around the pointer; reverses mid-turn *interim*, Arc's: measure | Arc |
| [x] | Pinned tabs section | MVP | rows 6 under the tiles, above the Clear divider | Dia 1.52, Ardia |
| [x] | Pinned tab ⌘W | MVP | unloads, keeps the row | Dia rec P§2 |
| [x] | URL typed into a pinned tab | MVP | ⌘L's URL opens as a new Today tab at the top, selected; closing it goes back to the pinned tab | Arc cap |
| [x] | Pinned tab away from its URL | MVP | the favicon turns into a back arrow under the pointer and the title reads "Back to Pinned URL"; a click or ⌘↩ goes back *interim*; Arc's look: measure | Dia 1.53.0 bin |
| [x] | Page in two windows (a pinned tab's too) | MVP | one live page; the other window draws a picture of it, 0.8 of the card, veiled black 0.12, which a click takes the page back from; steps back in 200 ms, comes forward in 120 ms *ours* | Dia cap, Ardia; ours |
| [x] | Clear divider line | MVP | one device pixel, white 0.15, 18 in from each side; ends 9 before Clear while it shows; centre 25 above the first row's centre | Arc 1.170.0 cap and tokens, Arc (Ardia) |
| [x] | Clear label and arrow | MVP | the word, semibold ≈ 10 pt, 29.5 × 15, ends with the titles, 9 after the line; an arrow 8 tall, 3 before it, while the tab shown is all of Today, and Clear then closes it too; white 0.32, 0.81 under the pointer, no plate; ours: the arrow on Glyph change | Arc 1.170.0 cap and ax, Arc in use |
| [x] | Clear on hover | MVP | shows only while the pointer is over the sidebar | Arc 1.170.0 cap |
| [x] | Sidebar hover fades | MVP | Clear and the URL field's buttons coming and going with the pointer. Ours: 150 ms ease-out fade. Arc's: measure | Arc |
| [x] | Row pitch / height | MVP | 41 / 40 | Arc 1.170.0 ax |
| [x] | Row plate | MVP | 36 tall, x 8 to width − 8, radius ≈ 12 | Arc 1.170.0 cap |
| [x] | Row favicon and title | MVP | favicon 18 at x 17, title at x 43, ending at x 211 with an ellipsis | Arc 1.170.0 ax and cap |
| [x] | Row states | MVP | selected white 0.20 (0.36 under the pointer, Arc 1.170 rec), hover 0.12; pressed takes the selected fill, as the press selects; no line or shadow in dark mode; unselected none | Arc 1.170.0 tokens and cap, Arc 1.170 rec |
| [x] | Row press | MVP | the plate shrinks to 0.99 over 70 ms, linearly, and the press shows the tab, before the release; ours grows back the same way *interim* | Arc 1.170 rec |
| [x] | Row titles | MVP | selected white 1, unselected 0.78 | Dia asset catalogs |
| [x] | Row hover | MVP | the fill comes and goes in one frame; only the selected row eases back from its hover (white 0.36) to its fill over ≈ 170 ms | Arc 1.170 rec |
| [x] | Close ✕ | MVP | on hover, the selected row's too; 26 × 26, 5 inside the plate's right edge; two bars 11.4 × 2, white 0.6; − on the selected pinned row, which unloads it, the bars turning level over 110 ms | Arc 1.170.0 cap, Arc 1.170 rec |
| [x] | Status dot | MVP | ours (#321) in the spinner's 12 × 12 slot: 6 pt, white 0.6 while loading, Signal green #0E9F55 in a 1.5 pt #F3F0E8 ring while an agent is at work in the tab, breathing while it loads; only in rows ≥ 48 wide | ours, after Dia 1.50.1 bin's slot |
| [x] | Speaker | MVP | after the favicon while the page plays and 2 s after, crossed out while muted; a click mutes or unmutes the tab, as Arc mutes tabs *interim*; Arc's look: measure | Arc 1.170.0 bin, Dia, Chrome |
| [x] | Sleeping tab | MVP | favicon at half opacity while unloaded, in rows, tiles and splits *interim*; Arc's alpha: measure | Dia |
| [x] | "+ New Tab" row | MVP | the first row under the divider, where new tabs open below it (Arc); label white 0.55, plus 0.25; hover as rows | Arc, Dia 1.50.1 bin |
| [x] | Scroll edge | MVP | 1 pt P3 white 0.09 at favorites + 4; fades 100 ms cubic-bezier(0.25, 0.46, 0.45, 0.94) *interim*, Arc's: measure | Dia 1.53.0 bin |
| [x] | ⌘T's plate | MVP | "+ New Tab" has the selected plate while ⌘T's bar is open, in one frame | Arc 1.170.0 rec 120 fps |
| [x] | Open tab motion | MVP | new row in place at once, its plate fading in; rows below slide; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Close tab motion | MVP | the row fades as the rows below slide up over it; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Undo motion | MVP | the tabs Undo brings back fade in in place; spring 0.13 s / ζ 0.9 | Arc 1.170.0 rec 120 fps |
| [x] | Reorder | MVP | the rows stay put during the drag, a ghost follows the pointer, and a 2 pt white 0.9 line with an 8 pt ring marks the gap; on drop the rows glide on Row motion (41 pt in ≈ 115 ms) | Arc 1.170 rec |
| [ ] | Drag to another window / desktop | MVP | moves the live page; new window under the pointer. Not started: multi-window step 7 | Dia P§1 |
| [x] | Drops on the list | MVP | files and http(s) open at the gap, which the drop line marks as for a dragged row *interim*; never javascript:, data:, file: from text; Arc's: measure | Dia, Ardia |
| [ ] | Hover card show | MVP | delay 0.5 s; opacity 100 ms + scale spring 0.92 → 1 (0.25 s, ζ 0.75). Not started: the spec leaves the hover card undecided (Arc has none recorded) | Dia bin |
| [ ] | Hover card dismiss | MVP | opacity 100 ms + spring to 0.92 (0.2 s, ζ 0.6); click or key dismisses. Not started, as above | Dia bin |
| [x] | Background tabs | MVP | a link opened behind loads when first shown, with the link's address and the site's icon until then | Dia |
| [x] | List scrolls to a row | MVP | the least that shows it whole, for a tab opened behind and the tab shown; instant | Dia, Chrome tab strip |
| [x] | Opener order | MVP | Chrome's TabStripModel rules | Chrome |
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
| [x] | Emoji icon | MVP | Change Icon… hangs the Space icon picker's Emoji tab (search, recents, grid) under the favicon; the emoji picked is the icon, kept as a name is, and Remove Icon or the picker's trash gives the favicon back. Dia's picker: measure | Dia P§2, ours |
| [x] | Tab context menu | MVP | shortcut hints. A Today tab's is Arc's: Copy Link ⇧⌘C, Share › · Change Icon…, Rename…, Mute · Add Split View, Duplicate, Move to › (Spaces, New Space…) · Archive Tab, Archive Tabs Below, without Customize Page… until its feature. A pinned tab's: Unpin, Add to Favorites, Back to Pinned URL, Rename, Duplicate, Move to Space, Close *interim* until Arc's is measured | Dia P§2, Arc 1.170.0 ax |
| [x] | Folders | MVP | in the pinned tabs, indented 16 a level: New Folder…, a press opens and closes, a double-click or the menu renames, Delete Folder keeps the tabs, a drag right or left takes a row in or out, kept over a quit; motion and icon *interim* | Arc ax, cap, menus |
| [x] | Tab shown in a closed folder | MVP | its row stays under the folder until another tab shows | Arc (confirm in the capture) |
| [ ] | Folder motion and drag | MVP | a press opens it on a 200 ms ease-out and closes it critically damped at ω ≈ 25, neither overshooting (Arc 1.170 rec); a dragged row goes in or out with the pointer's travel *interim*; how Arc's dragged row goes into a folder: measure | Arc 1.170 rec |
| [x] | Folder tooltip | MVP | the name after a moment; ours 0.73 s *interim* | Arc cap |
| [x] | Favorite tooltip | MVP | the name after 0.73 s *interim*, at once from tile to tile, in one frame and out over 0.27 s as Arc's; Arc's move to the next tile: measure | Arc cap |
| [ ] | Button tooltip | MVP | Arc's: View Library and ⇧ ⌘ L on chips, 3.5 right of the Library button; a capsule 27 tall in #090C4B with a 0.5 pt rim, 12 pt semibold text 11 in, chips 18.5 with a 2 pt lip on a 22 pitch, in one frame and out over 0.27 s. Built, to check against a capture: the header's buttons and Copy URL with the menus' keys, under them, and the square corner's side there *interim*; Arc's other buttons: measure | Arc in use, cap 2× |
| [x] | Collapse All Folders | MVP | a command bar command | Arc cap |

## Spaces

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Space model | MVP | name, colour, pinned, Today, profile; many Spaces per profile | decision 5 |
| [x] | Switching to another profile's Space | MVP | the window's profile switches with it, and its favorites | decision 5 |
| [x] | Moving a tab across profiles | MVP | reloads in the new profile after a one-time warning, with its history | Dia P§1 |
| [x] | Favorites per Space or per profile | MVP | per profile; confirm in Arc | Arc |
| [x] | Switcher footer | MVP | 50 tall; Library 32 at 8 in on the left; + 32 at 8 in on the right, its menu New Space… and New Folder… *interim*, Arc's: measure | Arc 1.170.0 cap, ax |
| [x] | Switcher band | MVP | 42 in from each side; items 36 × 34 at a 34 pitch; none with one Space; more than fit: narrower *interim*, Arc's: measure | Arc 1.170.0 ax |
| [x] | Switcher dot | MVP | 8 pt; white 0.50 shown, 0.19 others | Arc 1.170.0 cap |
| [ ] | Space icons in the switcher | MVP | built: an emoji takes the dot's place in its item, the Space shown at full strength *interim*; Arc's look: measure | — |
| [x] | Swipe tracking | MVP | −0.5 × horizontal delta; ignores the system swipe setting | Dia 1.50.1 bin |
| [x] | Rubber band | MVP | `255·(1 − 1/(0.15·x/255 + 1))` | Dia 1.50.1 bin |
| [x] | Past the first Space, the Library | MVP | fingers move its open from the first Space, never in the swipe that reached it, opening past halfway or on a flick; a wheel's tick opens it; back from it the same way | Arc release notes, Arc 1.x video, Arc 1.170 rec |
| [x] | Past the last Space, a new Space | MVP | the page pulls ≤ 41 (2×); a circle's ring closes at 38 and readies it, with Arc's morph and pop; let go readied makes the Space with its form, the page in from 46 over 170 ms. Arc's 4-point sparkle: ours is the dot growing into the + | Arc 1.x video |
| [x] | Release | MVP | ≥ 5 pt/s goes on; ±1 Space; 100 ms velocity history | Dia 1.50.1 bin |
| [x] | Settle | MVP | critically damped 0.19 s, carrying the velocity; detent haptic | Arc 1.170 rec |
| [x] | Wheel mice | MVP | sum deltas < 0.049 s apart; > 1 pt pages once; ignore 0.25 s | Dia 1.50.1 bin |
| [x] | ⌃1–⌃9 | MVP | switch Space | Dia (as profiles) |
| [x] | Colour cross-fade | MVP | uniform across the window; next door exponentially, τ ≈ 30 ms; a longer jump with the 0.19 s settle; a swipe with the pages | Arc 1.170 rec, Ardia |
| [ ] | Space title, create, colour, icon | MVP | built: the Create a Space form (Arc's), Arc's theme picker from its Choose a Theme row and Edit Theme Color… (Themes), Remove Theme Color back to the accent *interim*, and Change Icon… opening Arc's icon picker (below); Arc's title flow: measure | — |
| [x] | Space icon picker | MVP | popover 273 × 273 at radius 20 under the name, arrow at the icon; Emoji \| Icon and a trash; 56 glyphs 8 × 7 on a 31 pitch (Lucide's outlines), the Space's on a white 0.25 plate; Emoji tab a search and a scrolling 8-column grid of Unicode's groups, recents first, picked by click or ↩ (ours), only the emoji the system's font draws (CoreText, asked once a launch); grows in from the arrow and goes in one frame; Emoji ↔ Icon slides the content 273 over 0.2 s; hover white 0.12 on a glyph, the trash and the other segment *interim*, Arc's: measure | Arc 1.170.0 cap, Arc 1.170 rec |
| [x] | Rename and delete | MVP | native menu on the name or dot; double-click renames; delete asks in Arc's dialog if it has tabs | Arc, ours |
| [x] | Undo delete | MVP | the toast says "Space deleted! Use ⌘Z to undo." for 5 s; ⌘Z brings the Space back where it was, with its tabs, and pages to it | Arc 1.170 rec |
| [x] | Undo delete motion | MVP | the switch's slide *interim*; Arc's: measure | Arc |
| [x] | Agent Space kind in the data model | MVP | a Space is the user's or an agent's (the id and name its client gives) from when it's made, saved with the Spaces; no tab moves into an agent's; New Space makes the user's | decision 8, roadmap |
| [ ] | Agent Space UI: switcher mark, confirm card, handoff card, take over, status, activity log, allowed sites with Revoke | 4 | ours to design; confirm times out 5 min, handoff 15 min | decision 8, Ours (prototype) |
| [ ] | Agent actions never move the user's focus | 4 | no change of Space, tab, focus or cursor | decision 8 |

## Command bar

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Panel width and place | MVP | 765, centred on the window, its top 169 above the window's middle | Arc 1.170 rec |
| [x] | Text alignment | MVP | input row 64; glyph 18 pt at 23 in; text ≈ 18 pt at 53 in; hint at the field's end | Arc 1.170.0 ax |
| [x] | Open | MVP | one frame, for ⌘T, ⌘L and a click on the URL field | Arc 1.170 rec |
| [x] | Close | MVP | one frame, as it opens | Arc 1.170 rec |
| [x] | Over the URL field | MVP | a click on the field: 350 wide at the field's left, 2 above its top, 310 tall with five rows; input row 50 with the rows' columns, no line; radius ≈ 12; titles without the host. ⌘L stays centred. With the sidebar hidden, centred *interim* | Arc 1.170 rec |
| [x] | Selection motion | MVP | ↓ and ↑ move the selection in one frame | Arc 1.170 rec |
| [x] | Hover plate | MVP | the row under the pointer: the selection's shape in white 0.05, in one frame; kept as ↓ ↑ move the selection; none on the selected row | Arc 1.170 rec |
| [x] | Fill / border / radius | MVP | #191919 at 0.86 over an untinted blur of what's behind; 0.5 pt outline and 1 pt grey edge; radius ≈ 18 | Arc 1.170.0 cap, Dia 1.50.1 bin |
| [x] | Shadow | MVP | soft, no offset, ≈ 30 pt; drawn at radius 20 and 0.23 black, fitted to Arc's over a light page | Arc 1.170.0 cap |
| [x] | No scrim | MVP | page undimmed | Dia 1.49 cap |
| [x] | Rows | MVP | input 64; rows 50 (five shown, then scroll); favicon at + 16, title 14 pt medium at + 43; " — url" dim; Switch to Tab and an arrow tile | Arc 1.170.0 cap, ax |
| [x] | Selected row | MVP | the accent, radius ≈ 6.5, inset 2; icon on a white tile | Arc 1.170.0 cap |
| [x] | Bottom row | MVP | none: no Go pill, no AI chip or mic; the site-search hint is at the field's end | Arc 1.170.0 ax |
| [x] | Colours | MVP | placeholder white 0.53, secondary rgba(239,244,255,0.6), caret #4A77D4, selection rgba(120,125,134,0.7) | Dia asset catalogs |
| [x] | Default match | MVP | inline match, else URL, else search; search row after the top hit | Chrome, Dia 1.49 |
| [x] | Stable row 0 | MVP | local only; remote rows append | Chrome O |
| [x] | Inline completion rules | MVP | one term, caret at end; never during IME, after delete, after paste, after whitespace | Chrome O |
| [x] | Inline eligibility | MVP | typed ≥ 1 host / ≥ 2 deeper | Chrome O |
| [x] | Well-known sites | MVP | built in; fill the rows left under the profile's own, never row 0 | ours |
| [x] | Site search | MVP | Tab when hinted, Space after the keyword; ⌫ at the start, ⇧Tab or Esc leaves and restores; Arc's look: measure | Chrome O |
| [x] | Keys | MVP | ↩, ⌘↩ behind with the bar open, ⌥↩ beside, ⇧⌘↩, ⌃↩, ↑↓ and Tab ⇧Tab wrap, ⌃N/⌃P, ⇞⇟, ⇧⌦, Esc | Arc 1.170.0 cap, Chrome, Dia O |
| [x] | ⇧↩, ⌃⇧↩ | MVP | in a new window, with windows: where ↩ / ⌃↩ go, in a new window made as ⌘N makes one (from a private window, a private one), showing the page; a command or sum as ↩, over a new pane as ↩ *interim*; whether Arc opens a window, Little Arc or Peek: measure | Chrome |
| [x] | ⌘V after ⌘T / ⌘L / ⌘F | MVP | lands in the new field (≤ 400 ms), with every key typed and a composition, from the page or the UI; a page that keeps the shortcut gets them after. The checks type, never paste | Ardia P§7 |
| [x] | Click outside | MVP | closes and still acts | AGENTS.md |
| [x] | Switch to Tab rows | MVP | across windows, never the current tab: a tab another window shows live brings that window forward, key, rather than taking its page, as Chrome's does *interim*; whether Arc raises or moves: measure | Dia, O |
| [x] | URL parsing | MVP | Chrome's Parse; https first, http fallback; local hosts http | Chrome O |
| [x] | Commands | MVP | found by name, with their shortcuts; one whose name starts with what's typed comes first | Arc 1.170.0 cap |
| [x] | Row icons | MVP | a clock for a page only in history; a favicon for an open tab or a site; a glyph for each command, Lucide's | Arc 1.170.0 cap |
| [x] | Empty text | MVP | the Space's open tabs with Switch to Tab, most recently shown first; Arc's recents and commands under them: measure | Arc 1.170.0 cap |
| [x] | Calculator | MVP | the row after the first; ↩ copies the result | Dia P§7 |
| [x] | Paste and Go | MVP | with Paste and Search, in the command bar field's menu; the clipboard is read only once one is chosen | Dia P§7 |
| [x] | Clean-link copy | MVP | Copy URL leaves out utm_ parameters and click IDs | Dia P§7 |
| [x] | Copy URL / as Markdown | MVP | ⇧⌘C / ⌥⇧⌘C; as Quote ⌃⇧⌘C later | Arc 1.170.0 menus |
| [ ] | Arc's command list and ⌘T over the page | MVP | ⌘T and ⌘L: the centred panel over the page, undimmed. One list of up to 15 rows, 5 shown, no section headings; commands among the suggestions from one letter, matched by name and by hidden keywords, each with its glyph and key; about 110 commands (ux-spec.md › Command bar › Commands). Built: ⌘T over the page; one letter finds a command, by name or hidden word, among up to 15 rows, the strong ones above the suggestions and the rest after; Arc's bar names and hidden words for the commands we have. Not yet: Arc's commands we don't have, such as Mute All Tabs, Reveal Tab in Sidebar and Exit | Arc 1.170.0 ax, cap |

## New tab

Dia's New Tab page isn't ours (decision 6). ⌘T opens the command bar over the page: see Command bar.

## Split view

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Panes | MVP | up to 3 side by side or stacked, sharing the card; Arc max: measure | Arc rec, Dia P§6 |
| [x] | Gap, focus indicator | MVP | 10 pt gap; 2 pt ring inside the focused pane, ≈ 8 pt glow outside, in the Space's colour (Arc's read P3 (49, 57, 251)) | Arc 1.170.0 rec |
| [x] | Unfocused dim | MVP | none *interim*; Arc's: measure | — |
| [x] | Pane sizes | MVP | the gap drags them, a tenth of the card each, 50–200; snap to even within 15; double-click evens; kept after a quit; Arc's: measure | Chrome *interim* |
| [x] | Keys | MVP | ⌃+ add, ⌃⇧- close pane; ⌃⇧] / ⌃⇧[ focus *interim* | Arc menus, Dia |
| [x] | Add Split View | MVP | in one frame: page narrows, empty pane, command bar over it 77 in from the card's edge | Arc 1.170.0 rec |
| [x] | ⌥↩ in the command bar | MVP | the URL in a right pane | Arc 1.170.0 rec |
| [x] | ⌥-click + New Tab; ⌥⇧-click link | MVP | Add Split View, as Dia's New Tab page in the pane isn't ours; the link in a right pane, focused, and ⌥⌘-click too (Chrome) | Dia P§6, Chrome |
| [x] | Sidebar row | MVP | one row, equal segments: favicon 18 at 7, title 10 after, 5 before the line | Arc 1.170.0 cap |
| [x] | Kept across a quit | MVP | yes | ours |
| [x] | Targets at rest | MVP | 198.5 × 333.5, 37.5 in, radius ≈ 20, dashed 6/6 ≈ 1.75 pt | Dia rec |
| [x] | Targets grown | MVP | 322 × 532, 30 in | Dia rec |
| [x] | Target under the card | MVP | 362 × 600, leaning 43 % toward the pointer, accent ≈ 0.15, dashes 7/7 ≈ 2 pt | Dia rec |
| [x] | Target timing | MVP | slide ≈ 110 ms, grow ≈ 120 ms, no overshoot | Dia rec |
| [x] | Drag picture | MVP | tab for 1–2 frames, then 209 × 109.5 window card; ours shows the favicon and title, not the page's image | Dia rec |
| [ ] | Pane motion | MVP | a dropped tab's pane coming in on its side, the divider dragged, a pane closing: built with a drop showing the split at once and the divider following the pointer *interim*; Arc's: measure | Arc |
| [ ] | Drop away from a target | MVP | new window grows from the card ≈ 200 ms, takes focus. Not started: multi-window step 7 | Dia rec |
| [x] | Close pane, Separate | MVP | Close this Split Pane, Separate Page from Split View | Arc menus |
| [x] | Expand Current Split | MVP | the focused pane takes the card, the other keeps 220 pt at any width; one frame; again does nothing; kept after a quit. A flag in the core's split, worked out at each layout so a resize holds the 220, and saved with the session. In View after Developer and in the command bar, no key; dragging or double-clicking the gap, or a pane joining or leaving, ends it *interim* | Arc 1.170.0 ax, rec |
| [x] | Commands | MVP | Flip, Move Pane Left/Right, Separate All, in the command bar | Dia P§6 |
| [x] | Change Orientation | MVP | the panes stacked top to bottom, kept after a quit; drop targets at the sides *interim* | Dia P§6 |

## Settings

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Settings window | MVP | 684 × 650, ⌘, and Settings…; toolbar of tabs with the accent on a plate; ⌘W and close hide it; opens on the tab shown last | Arc 1.170.0 cap, ax |
| [x] | Boxes, rows, switch, pop-up and buttons | MVP | the spec's measures; the pop-up's menu opens over it | Arc 1.170.0 cap, ax |
| [x] | Shortcuts tab | MVP | the browser's shortcuts by section, read-only | Ours |
| [x] | Advanced › More Settings… | MVP | opens Chromium's settings in a tab | Arc 1.170.0 cap |
| [x] | Haptics setting | MVP | Advanced: haptic feedback when swiping between Spaces, kept with the Spaces | Ours |
| [x] | Warn before quitting | MVP | General's switch, on; Quit and ⌘Q ask "Quit ilo?" in the dialog: "Quit, and don't ask again" at the left, Cancel ESC and a blue Quit ↩ at the right; no icon, so 172 tall where Arc's is 249; Dock and logout quits don't | Arc 1.170.0 cap, bin; Arc 1.170 rec |
| [x] | Profiles tab | MVP | ours: a row per profile with its Spaces, rename on a double-click, Delete for an unused one, New Profile *interim*; Arc's: measure | Arc 1.170.0 cap |
| [x] | Default browser | MVP | ours: General's Make ilo your default browser, macOS's confirmation, the way back in System Settings; links from other apps open as tabs in the front regular window's user Space *interim* until Little Arc | Ours |
| [ ] | Arc's other tabs | MVP | Links, Icon, Previews: with their features; none built | Arc 1.170.0 cap |

## Archive

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Clear (⇧⌘K and the divider button) | MVP | closes the Space's Today tabs but the one shown, in one frame, no toast, or that one too when it's all of Today; pinned and favorites stay | Arc 1.170.0 rec, Arc in use |
| [x] | Undo Clear | MVP | ⌘Z brings them back in place, unloaded; the tab shown stays; one Clear closed as the last shows again (ours) | Arc 1.170.0 rec |
| [x] | Undo and redo everywhere | MVP | ⌘Z and ⇧⌘Z across close, move, pin, favorite, folder, split, rename and archive edits, with Undo in the Edit menu named and a toast with an Undo ⌘Z button where the change isn't plain; each back in its exact place | ours (Arc: Clear and Delete Space only) |
| [x] | Reopen closed tab ⇧⌘T | MVP | back in its place with its name, icon, mute and back/forward, and a split's tab back in its split. Pinned tabs don't close | Dia P§2 |
| [x] | Auto-archive | MVP | a Today tab unseen for 12 hours goes in the archive, counted across a quit | Arc 1.170.0 settings |
| [x] | Auto-archive's other intervals | MVP | Settings › General: 12 hours (default), 24 hours, 7 days, 30 days, kept with the Spaces | Arc 1.170.0 bin |
| [x] | ⌘W on a Today tab | MVP | Archive Tab: the tab goes in the archive | Arc 1.170.0 menus |
| [x] | Library | MVP | slides in from its width to the left as it pushes the card to 417 at its width, on spring 0.245 s / ζ 0.92; the sidebar fades out in 33 ms, and back in over 130 ms from 80 ms; column of sections, back button; ⇧⌘L, View Library, footer button | Arc 1.170.0 cap, ax, rec; Arc 1.x video |
| [x] | Archived Tabs | MVP | search on the buttons' row, day headings, rows 56 apart with favicon, title and host; a click reopens the tab | Arc 1.170.0 cap, ax |
| [x] | Clear Archive | MVP | Archive menu; asks first | Arc 1.170.0 menus |
| [x] | Library's Spaces | MVP | 216 + 264 a Space wide, no search; a card per Space with its sidebar rows at the sidebar's metrics, a brush for its menu, a blue + for a new Space; a click shows the Space | Arc 1.170.0 cap (one Space); Arc 1.170 rec (three Spaces) |
| [ ] | Library's other sections | MVP | Downloads, Spaces and Archived Tabs done; Media, Easels & Notes as each feature lands | Arc 1.170.0 cap |
| [x] | Archive filter, row hover and menu | MVP | ours: the sidebar's hover plate, and a menu of Restore Tab, Copy URL and Delete from Archive; Arc's Filter left out *interim*; Arc's: measure | — |

## Profiles

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Profile holds cookies, history, logins, autofill, extensions | MVP | yes | decision 5 |
| [x] | Create, rename, delete, pick for a Space | MVP | ours: Settings › Profiles, the New Space form's Profile row (Arc's) and the Space menu's Profile items *interim*; Arc's: measure | Arc 1.170.0 cap |
| [ ] | Where a Space shows its profile | MVP | built: the Space's menu checks it and the form's Profile row names it, nothing in the sidebar *interim*; Arc's: measure | — |
| [x] | Incognito ⇧⌘N | MVP | per profile, dark, one session per profile, nothing written. Built (private check): a private window of the key window's profile with one unsaved Space of its own, Today tabs only, black 0.35 in the gradient's place *interim*; Arc's and Dia's private window: measure | Chrome, Dia |

## Extensions

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Pinned actions | MVP | at the URL field's end, 16 pt on a 30 pitch, badges, click, right-click and overflow menu *interim*; Arc placement: measure | Dia 1.52 cap |
| [x] | Extensions menu in the menu bar | MVP | per extension, Pin to URL Field, Add (the Web Store in a new tab) and Manage | Dia |
| [x] | Popups | MVP | Chrome's, from the action or under the URL field *interim*; 25 × 25 to 800 × 600, shrink and grow; openPopup works; Arc's opening and closing: measure | Chrome |
| [x] | Action state without polling | MVP | from change notifications and tab switches only | principle 1 |
| [x] | Side panel | MVP | card right of the page, header, 320–600 saved; opening motion: measure | Dia, Ardia |
| [ ] | Install, re-enable, new-permission dialog | MVP | ours, Chrome's words and warnings, done, and the store's Add to Chrome asks with it; Arc's: measure | Dia P§15 |
| [x] | Remove | MVP | "Remove “<name>”?", Cancel / destructive Remove, from the pinned action's menu; Chrome's page asks with Chrome's dialog *interim* | Dia P§15 |
| [ ] | Desktop apps | MVP | native messaging hosts the browser's folders lack, from Chrome's, Chromium's, Edge's, Brave's, Arc's or Vivaldi's | ours |
| [x] | Keyboard commands | MVP | fire with the page and the UI focused; actions' keys; never reserved keys; suggestions that are ours stay ours; media keys: later | Chrome P§15 |
| [x] | Site Controls | MVP | 238 wide, radius 17, beak 12.5, tiles 48 × 32 on 8 gaps; Share and the two captures; the extensions with + and Manage; Chrome's permissions as Arc's rows with More; the lock opens the certificate, … the menu; Boost items: with Boosts; motion *interim* | Arc 1.170.0 cap |

## Ad blocking

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [ ] | On by default | MVP | every profile, new and existing, and private windows, from the first page; nothing to install | Helium, Flecha |
| [ ] | Engine | MVP | upstream uBlock Origin 1.75.0, pinned with its sha256, in the framework, loaded at each launch; updates with the browser; listed in the Extensions menu, pinnable, no Remove | Ours |
| [ ] | Turning it off | MVP | Site Controls' first row, per site, reloading the page; Settings › General, everywhere | Ours |
| [ ] | Filter lists | MVP | weekly while open, and 37 s after a launch when stale; never more often; source a default service URL | principles 1, 2 |
| [ ] | First launch | MVP | requests through for about 3 s until the lists load; closing it: measure first | principle 1 |
| [ ] | Cost | MVP | page-load and memory numbers, with and without it, against the performance gate | principle 1 |
| [ ] | No agent path | MVP | agents can't change it, everywhere or for a site | decision 8 |

## Passwords and autofill

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Save / update prompt | MVP | page top right; Save / Update Password, username picker, Never, No Thanks in Chrome's words; look: measure | Dia P§16, Chrome |
| [ ] | Address and card offers | MVP | same prompt, Chrome's words. Chrome's save-address and save-card offers show in the password offer's card, with Chrome's title, buttons and what they save; the autofill run saves an address (address-offer), declines one card and saves another (card-offer). An offer to update a saved address isn't shown yet. Unverified until that runs | Chrome |
| [x] | Pending offer on tab switch | MVP | closes; the URL field's key brings it back (ours) | Dia P§16 |
| [ ] | Fill dropdown | MVP | Chrome's, under the field. The passwords run checks it fills a saved login in; unverified until that runs | Chrome |
| [ ] | Edit › AutoFill items | MVP | open the dropdown at the focused field, else settings. Contact…, Passwords… and Credit Card… open Chrome's dropdown at the field the core saw take the keyboard, else Settings' Passwords; the passwords run checks Passwords… and the no-field case (autofill-menu-passwords), the autofill run Contact… and Credit Card…. Unverified until those run | Dia |
| [ ] | Password generation | MVP | Use Strong Password / Choose Your Own. Chrome's popup, which Chrome offers only with a Google account to sync to, the core turns on without one, saying the password is saved on this device; the autofill run captures it on a new password field (password-generation) and takes the password. Unverified until that runs | Chrome |
| [ ] | Eye reveal in pages | MVP | once the user types, never for a filled password. Blink's PasswordReveal, on from the core; the passwords run captures the eye (password-reveal) and its absence on a filled password (password-filled-reveal). Unverified until that runs | Dia |
| [ ] | Settings unlock | MVP | one device check, 5 min grace. Not started: Chrome's page should do it in a tab, unverified | Chrome |
| [ ] | Passkey dialogs | MVP | Chrome's, centred over the page, as child windows. Not started: unverified, probably works | Chrome |
| [ ] | No agent path | MVP | agents never read or fill these. Holds today, since no call returns them and there is no MCP server; ticks with its allowlist test | decision 8 |

## Downloads

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Library section, ⇧⌘J, View Downloads | MVP | above Archived Tabs; rows as the archive's, with the system's icon, status and bar | Arc 1.170.0 menus, cap |
| [x] | Row actions | MVP | progress, pause/resume, cancel, open, Show in Finder, Open When Done, keep or delete a held file, Remove from List, Clear List | Dia P§14 |
| [x] | Move to Trash, drag out | MVP | to Finder, other apps, the page, the tab list | Dia P§14 |
| [x] | Footer button while downloading | MVP | ours: glyph over a 16 × 2 bar *interim*; Arc's look: measure | — |
| [x] | Recent downloads over the Library button | MVP | up to 4 finished, newest at the bottom, 56 apart: 40 pt preview or 26 pt icon at 8, name 15 pt at 61 over its age; ours: after a 0.5 s rest (its own, not the tooltip's 0.73 s), 0.15 s fade, a row acts as the Library's *interim*; tooltip with key chips and glyph motion: #189 | owner's Arc cap |
| [x] | Fly-in | MVP | ours: 32 → 16 pt icon from under the URL field to the button on a slight arc, 0.45 s on cubic-bezier(0.55, 0, 0.85, 0.4), fading over the last 30 %; Arc's: measure (Dia constants 0.1 s / 1.3 undecoded) | Dia bin |
| [x] | Per profile, persisted | MVP | Chrome's history; private never written once private windows land | Dia P§14 |
| [x] | Quit guard | MVP | asks while downloads run, in Chrome's alert; Continue Downloading shows the Library's Downloads; Arc's wording: measure | Dia P§1, Chrome |

## Find in page

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Keys | MVP | ⌘F, ⌘G, ⇧⌘G, ⇧↩, ⌘J; no ⌘E, as in Arc; ⌥⌘F left out, AppKit's text views only | Arc 1.170.0 menus, Dia P§8 |
| [x] | Per-tab query and count | MVP | yes; Arc's "1/12" | Dia P§8, Arc 1.170.0 cap |
| [x] | ⌘F with the page focused | MVP | ours unless the page keeps it | AGENTS.md |
| [x] | IME in the field | MVP | Japanese works | AGENTS.md |
| [x] | Bar position, size, contents, no-match state, motion | MVP | 300 × 32, 10 in from the card's top right, radius 6; search, field, count, previous, next, divider, close; NOT FOUND in red; one frame in and out | Arc 1.170.0 rec, cap |
| [x] | Bar colour, shadow, glyphs | MVP | Arc's (19,29,50), ours too; Arc's ≈16 pt shadow, ours AppKit's; glyphs Lucide's *interim*; Arc's glyphs: measure | Arc 1.170.0 cap |

## Zoom

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Keys and menu | MVP | ⌘0 / ⌘+ / ⌘-, hidden ⌘=; View › Zoom to Actual Size, Zoom In, Zoom Out; per site, kept | Arc 1.170.0 menus, Chrome |
| [x] | HUD | MVP | the percent over the card's top right until another tab shows; nothing in the URL field | Arc 1.170.0 cap |
| [x] | HUD look and timing | MVP | a pill in the toast's style, 41.5 tall, 8 inside the card; pops in from 0.4, dips to 0.45 for a change, shrinks out; 5 s after the last zoom *interim*, Arc's ≥ 4.9 s | Arc 1.170 rec |

## DevTools

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Keys and menu | MVP | ⌥⌘I, ⌥⌘J, ⌥⌘C, ⌥⌘U, from the page, DevTools and the UI; View › Developer | Arc 1.170.0 menus |
| [x] | Inspect in the page menu | MVP | Chrome's: docks DevTools at the element | Chrome P§23 |
| [x] | F12 | MVP | opens or closes DevTools, as ⌥⌘I, from the page and the UI; not in the menu | Dia, Chrome P§23 |
| [x] | Docked in the card | MVP | right, full card height, page on the left; each tab its own; closing gives the page the keyboard | Arc 1.170.0 cap |
| [x] | Docked width | MVP | Arc a third of the card; ours Chrome's 555 pt, then the user's, kept per profile | Arc 1.170.0 cap |
| [x] | Undocked | MVP | Chrome's window, from DevTools' Dock side menu; Dia's 640 × 640 first, "Developer Tools - <url>" | Dia P§23, Chrome |
| [x] | Task Manager | MVP | Chrome's, from Help › Troubleshooting › Open Task Manager as Arc's; ⌘W closes it | Arc 1.170.0 menus, Chrome |
| [ ] | No agent DevTools or debugging port | MVP | none. Holds today, since nothing opens a debugging port and there is no MCP server; ticks with its allowlist test | decision 8 |

## Capture

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Capture mode | MVP | ⇧⌘2 from the page and the UI, File and the command bar; grey page (luma × 0.749), 4 pt ring (21,28,47) at radius 6, camera pointer; pill 356 × 67 at radius 18, 35.5 up, 14 pt semibold (71,28,157); glow *interim* | Arc 1.170 cap |
| [ ] | Element under the pointer, click, drag | MVP | the core's hit test, no page JavaScript; click takes the box, drag the rectangle; Esc, ⇧⌘2, a click outside or a resize leaves | Arc 1.170 cap, ours |
| [x] | Menu | MVP | 273 wide, radius 7, (18,15,44), at the ring's top left + 10; field, 40 pt rows; Copy, Save…, Send via iMessage…, Retake; Save to Library, Edit and New Easel with their features | Arc 1.170 cap |
| [x] | Toast after Copy | MVP | "Screenshot Copied to Clipboard" with Save and Share; time and hover *interim* | Arc 1.170 cap |
| [x] | Full page | MVP | the whole page past its view, copied; "Capture in Progress" *interim* | ours |
| [x] | Site Controls' capture tiles | MVP | Capture a Portion of This Page and Capture Full Page after Share, in Arc's order; each closes the panel, then runs its command; glyphs *interim* | Arc 1.170.0 cap |

## Import

| Done | Item | Phase | Target | Source |
|---|---|---|---|---|
| [x] | Arc: Spaces → Spaces, profiles → profiles | MVP | yes; Settings › Import | decision 5 |
| [x] | Arc: favorites, pinned tabs, custom names | MVP | favorites → favorites; pinned → pinned rows | Dia 1.52 P§20 |
| [ ] | Arc: themes | MVP | `windowTheme` colours, intensity, noise, texture, appearance, light and dark; built: colour, count, intensity, noise, appearance | Arc 1.167 P§20 |
| [x] | Arc: folders | MVP | ours: folders, nested; split views as their tabs *interim*; measure | ours |
| [ ] | Chrome: bookmarks, history, open tabs, passwords, addresses, cards | MVP | yes, passwords from the user's exported CSV; built: bookmarks and history, for Chrome's whole family, and passwords from any browser's exported CSV | P§20 |
| [x] | Chrome: cookies | MVP | not imported: no other browser's cookies or passwords are decrypted, and the user signs in again | #453 |
| [x] | Firefox, Zen, LibreWolf: bookmarks, history | MVP | ours: as Chrome's family's, from `places.sqlite` | ours |
| [x] | Safari: bookmarks, Reading List, history | MVP | ours: as Chrome's family's, the Reading List as a folder | ours |
| [ ] | Chrome: extensions | MVP | listed with store links, not installed | P§20 |
| [x] | Full Disk Access | MVP | explain, link to System Settings, re-check on return; Safari's | P§20 |
| [x] | Rerun | MVP | adds nothing | P§20 |
| [x] | Where Chrome bookmarks land | MVP | ours: a Space named after the browser, the bar's bookmarks pinned, or in a folder past 12; measure Arc's | ours |

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
| [x] | Every command rebindable | MVP | Settings › Shortcuts | Dia P§22 |
| [x] | Full table in the spec | MVP | ux-spec.md › Keyboard: every key marked built or with its feature | Dia, Chrome |
| [ ] | Arc's other default keys | MVP | Arc has no Search Tabs, and Back to Pinned URL (its Reset Tab to Original URL) has no key; ours are ⌘↩, built, and ⇧⌘A, not built. Arc's ⌃⇧] / ⌃⇧[ switch split views, where ours focus panes. Not built: ⌘{ ⌘}, ⌘E, ⌘O, ⇧⌘O, ⌃D, ⇧⌘D, ⌃⇧1–⌃⇧4, ⌥⌘F, ⇧⌘V, ⌃⇧⌘C (ux-spec.md › Keyboard) | Arc 1.170.0 Settings › Shortcuts |
| [x] | Menu bar | MVP | Arc's menus in Arc's order, with the commands the browser has | Arc 1.170.0 menus |
| [x] | Edit menu in pages | MVP | Undo, Redo, Cut, Copy, Paste, Select All reach the focused page or field | Arc 1.170.0 menus |
| [x] | Hide, Hide Others, Minimize | MVP | ⌘H, ⌥⌘H, ⌘M | Arc 1.170.0 menus |
| [x] | Save Page As, Print | MVP | ⇧⌘S, ⌘P: Chrome's save panel and print preview | Arc 1.170.0 menus |
| [x] | Clear Cache and Refresh, Clear Cookies and Refresh | MVP | View menu and command bar; the page's site only, then reload | Arc 1.170.0 menus, cap |
| [x] | Page context menu | MVP | Chrome's, without what the window can't do yet *interim*; its tab dialogs, such as print preview, centred over the page at its top | Chrome |
| [ ] | Arc's page menu items | MVP | Chrome's, plus Open Link in Split View and in Little Arc after New Window, and Capture Image…; without Lens, Reading Mode, Listen, QR Code, Send to Devices and Copy Link Text (ux-spec.md › Menus › Page menu). Built: a link's items in Arc's order, with Open Link in New Window and in Incognito Window; without Lens, Reading Mode, Listen, QR Code, Send to Devices and Copy Link Text. Little Arc, Capture Image…, Cast… and Translate wait for their features *interim* | Arc 1.170.0 ax |
| [x] | Space list, Rename Space, View History, Services, Essential Keyboard Shortcuts | MVP | Spaces by name with ⌃1–⌃9; ⌘Y opens Chrome's history page; Help opens Settings › Shortcuts | Arc 1.170.0 menus |
| [ ] | Arc's other menu items | MVP | ux-spec.md › Menus: Blank Window and New Little Arc Window; Edit Theme…, Manage Spaces…, Network Inspector, Turn on Developer Mode, as each feature lands. Partly: Arc's titles (Clear Today, Hide/Show Sidebar, Pin/Unpin Tab, Open Library…, View Downloads…, View Archive…), windows named by their shown tab, Reveal Tab in Sidebar, Change Profile and Paste and Match Style are done, as are the menus Arc has for built features | Arc 1.170.0 ax |

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
| [ ] | Link rules: routing | 4 | links from other apps: first rule matching URL text and/or sending app opens in its Space; else The Space in Front, a named Space or Little Arc; never an Agent Space or private window *interim* | Arc (Air Traffic Control), ours |
| [ ] | Link rules: Settings › Links | 4 | rows of URL contains, From, → Space, −; + adds; drag reorders; a deleted Space's rule says No Space *interim*; measure Arc's | ours |
| [ ] | Link rules: import | 4 | Arc's rules come in with its Spaces | ours |
| [ ] | Themes: rendering | 4 | OKLab 45° mix to 75 %; third colour top → bottom; alpha 0.2 + 0.55 × intensity; the automatic appearance turns a light colour's window light. Built, dark only: one colour keeps the Space gradient, intensity scales its alpha, two and three as Arc's rule *interim* | Arc 1.167 P§21, Arc 1.170 rec |
| [ ] | Themes: texture | 4 | noise × strength (0.8 / 0.15 / 0.6 / 0.4), quarter when inactive; our own textures. Built: one grain of our own noise at 0.075 × the dial *interim* | Arc 1.167, Arc 1.170 rec |
| [ ] | Themes: picker | 4 | card 357 × 509 at radius 20, arrow at the row; HSL field 340 with 1–3 handles (44 / 26, + 180°, + 150° / + 210°); appearance ×3, − +, 9 swatches, wave slider for intensity, 24-dot dial for grain; grows from the arrow, shrinks to it in 110 ms; the window follows each drag. *Interim*: swatch clicks (the swatch's own colour), < >, −, Remove Theme Color, the menu's anchor, appearance not drawn, translucency not offered | Arc 1.170 rec |
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
