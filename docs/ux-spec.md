# UX spec

What every surface looks like, how it moves and what it does, with the measured value and its source for each. Decision 6 sets the rule: Arc's structure plus Dia's polish. [ux-parity.md](ux-parity.md) is the checklist built from this file. The last section is the plan for measuring Arc before it shuts down on 2026-12-22.

## Attribution

Parts of this file and of `ux-parity.md` are adapted from Ardia (https://github.com/mantrakp04/ardia), from `docs/dia-spec.md` and `docs/dia-feature-parity.md`, with supporting values from `docs/omnibox-ux.md` and `apps/browser/src/lib/profileTheme.ts`. They are taken at tag `v0.2.40-nightly.5`, commit `fce3a02ffb917d2902af4ce47b398ce8b03a440d`. Copyright 2026 Mantra Patel. Licensed under the Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0). Changes from the originals: the material is reorganized by our features, Ardia's implementation is removed, and it is merged with our decisions and with Arc notes. The adapted parts keep their Apache-2.0 terms inside this GPL-3.0-or-later repository. Ardia's authors measured the values from Dia, a browser by The Browser Company. No Dia or Arc assets are included here.

## Reading this spec

**Precedence.**

1. Decision 5 overrides everything else. A Space holds tabs, pinned tabs and a colour. A profile holds cookies, history, logins and extensions. Dia and Ardia merge the two, so where a Dia value below says "profile", read it as "Space".
2. Structure comes from Arc: what exists, where it sits, layout sizes and behavior. That covers the sidebar, Spaces, favorites, pinned and Today tabs, archive, split view, Peek, Little Arc and Boosts.
3. Polish comes from Dia: materials, colours, the command bar, the New Tab page and motion. Interactions that only Arc has (sidebar reveal, Clear, Peek, Little Arc) keep Arc's motion.
4. Where an Arc value hasn't been measured yet, the Dia value is the target and is marked *interim*. The Arc capture plan at the end lists what will replace it.
5. Values marked **Ardia** are Ardia's own choices, not measurements of Dia or Arc. Treat them as placeholders.

**Sources.** `D›X` is section X of Ardia's `docs/dia-spec.md`. `P§n` is section n of `docs/dia-feature-parity.md`. `O` is `docs/omnibox-ux.md`. Dia versions are 1.49.1, 1.50.1, 1.52 and 1.53.0. *bin* means read from Dia's binary or asset catalogs, *cap* a 2× screen capture, *rec* a 60–240 fps recording, *ax* a frame read from the app's accessibility tree. **Arc (Ardia)** marks Arc values Ardia recorded: Ardia's owner's Arc recordings and measurements of 2026-10-06 and 2026-10-22, a user's screenshot, and Arc's binary (Arc 1.167 for themes). **Arc** marks our own measurements of Arc 1.170.0 from 2026-10-24 on: a fresh install in a 1512 × 949 window on a MacBook display, in dark mode, with the window key. Recordings are 120 fps, and "×3" says how many runs a fit covers. **Chrome** means Chromium's own behavior. **Flecha** marks behavior read from Flecha's site and code at v0.1.0 (https://flecha.dev, MIT). **Helium** marks Helium's. **Ours** marks our decisions or the `prototypes/arcish` reference.

**Units.** Sizes are points. Times are ms or s. Colours are sRGB unless marked P3. "white 0.16" means white at alpha 0.16. Where two values appear, dark comes first, then light. Dia was measured in dark mode, so light values are Dia's asset-catalog tokens and no light capture exists yet. Springs are given as response and damping ratio ζ (mass 1, stiffness (2π/response)², damping 4πζ/response). Curves are given as `cubic-bezier(x1, y1, x2, y2)` and a duration.

**Assets and names.** SF Symbol names identify a shape only. We draw our own icons, and they have to work off the Mac too. The names Little Arc, Top Apps and Dia's mark are references, not UI text. A feature named Peek is Arc's link preview. Ardia uses "peek" for the hidden sidebar sliding in, which this spec calls the *sidebar reveal*.

## Shared rules

### Window material (Dia 1.50.1 bin, D›Window translucency)

Layers, bottom to top. There is no grain: 1.50.1 measures 0.05–0.09 levels of noise.

| Layer | Dark | Light |
|---|---|---|
| Blur behind the window: emphasized, follows the window's active state | private material 29 | material 13 (`.hudWindow`) |
| Base tint, full opacity | black 0.4 | white 0.8 |
| Space gradient: the Space colour at the top, the same colour at HSL lightness + 0.25 (clamped) at the bottom, each stop at 0.36 (0.12 for a neutral colour) | in a layer at 0.5 | in a layer at 0.75 |
| Content card | #121212 0.5 | white 0.7 |

- Dark composite: `out = 0.18·tint_y + 0.82·0.6·blur`. Light: `out = 0.27·tint_y + 0.73·(0.8·255 + 0.2·blur)`. Here `tint_y` runs down the gradient.
- When the window is inactive, AppKit swaps the blur for an opaque fill tinted by the wallpaper. Reduce Transparency and Increase Contrast do the same. Dia adds no fallback of its own. Compare captures only in the same key state.
- With no Space colour set, the gradient uses the system accent colour and follows its changes (Dia).
- Fitted Space colour (Dia's pink): P3 (177.6, 91.1, 106.8), which is #B25B6B in P3 and ≈ #BF556A in sRGB (HSL 349°, 0.36, 0.53). The other colours are unmeasured.

Material output measured over solid backdrops. Ports without AppKit materials approximate these (sRGB, active state):

| Material | Over black / grey 20 / grey 60 / white | Inactive fill |
|---|---|---|
| 29, dark | 24 / 32 / 45 / 104 | (52, 43, 41), tinted by the wallpaper |
| 29, light | 192 / 197 / 208 / 255 | 248 |
| 13, light | 137 / 146 / 165 / 243 | (239, 237, 236) |
| 13, dark | 33 / 44 / 72 / 182 | (58, 54, 53) |

### Colour tokens (Dia asset catalogs, unchanged 1.49.1 → 1.50.1, D›Color tokens)

| Token | Dark | Light |
|---|---|---|
| Tab hovered | white 0.16 | white 0.55 |
| Tab pressed | white 0.31 | white 0.70 |
| Tab selected (1.50) | #121212 0.5 | white 0.7 |
| Tab selected, secondary (multi-select) | white 0.25 | white 0.70 |
| Tab unselected | white 0.06 | black 0.06 |
| Tab title selected / unselected | white 1 / 0.78 | black 1 / 0.68 |
| Favorite tile resting fill / stroke | white 0.10 / 0.14 | black 0.05 / 0.18 |
| Favorite tile default selection ring | white 0.45 | black 0.48 |
| Menu item hovered | white 0.07 | black 0.07 |
| Menu hairline border | white 0.30 | black 0.25 |
| Command bar hover / Go | rgba(120,125,134,0.32) | rgba(15,24,44,0.08) |
| Command bar placeholder | white 0.53 | black 0.40 |
| Command bar primary / secondary text | white / rgba(239,244,255,0.6) | #0F182C / #0F182C 0.6 |
| Command bar caret | #4A77D4 | #6395FC |
| Command bar selected text | rgba(120,125,134,0.7) | — |
| Downloads button tint | rgba(247,245,255,0.78) | black 0.78 |
| Secondary text | white 0.60 | black 0.60 |
| Subtle fill | white 0.12 | black 0.12 |
| Separator line | P3 white 0.09 | P3 black 0.09 |

### Motion

Every curve used in this spec. Each feature's section says where it applies.

| Name | Value | Used for | Source |
|---|---|---|---|
| List spring | spring 0.13 s / ζ 0.9 (CSS ease ≈ 120 ms): within 0.75 pt of rest 83–92 ms after it starts | rows making or closing room, a closed row fading out, rows coming back after Undo, a new row's selection fading in | Arc rec ×9 |
| Row fill change | ease-out, 300 ms | a row's hover fill (*interim*). The selection moves to another row in one frame (Arc rec ×3) | Dia 1.50.1 bin |
| Reorder | critically damped, ω ≈ 90 rad/s: 91 % in 43 ms, done by 76 ms | neighbours making way during a drag | Dia rec 60 fps, D›Reordering |
| Hover card in | opacity cubic-bezier(0.25, 0.46, 0.45, 0.94) 100 ms; scale 0.92 → 1 spring 0.25 s / ζ 0.75 | tab and favorite hover card | Dia bin, D›Tab hover card motion |
| Hover card out | opacity same curve 100 ms; scale → 0.92 spring 0.2 s / ζ 0.6 | same | same |
| Scroll edge | cubic-bezier(0.25, 0.46, 0.45, 0.94), 100 ms | sidebar separator fading in and out | Dia 1.53.0 bin |
| Panel grow | exponential ease-out, ≈ 130 ms | command bar out of the URL field | Dia 1.52 rec |
| Sidebar hide | ease-out cubic, 110 ms: within 0.75 pt of rest 67–83 ms after it starts | ⌘S hiding the docked sidebar | Arc rec ×3 |
| Sidebar show | ease-out cubic, 100 ms | ⌘S showing it | Arc rec ×3 |
| Sidebar reveal in | spring 0.175 s / ζ 0.80, no fade, ≈ 4 pt overshoot. It starts ≈ 100 ms after the pointer reaches the edge | sidebar reveal at the left edge | Arc rec ×3 |
| Sidebar reveal out | spring 0.175 s / ζ 0.76, starting as soon as the pointer leaves | same | Arc rec ×3 |
| Space page | spring 0.21 s / ζ 0.98: 228 pt, within 0.75 pt of rest 210–225 ms after it starts | paging to another Space from a dot, a key or a menu | Arc rec ×3 |
| Space settle | critically damped, 0.25 s | Space pages after a swipe (*interim*) | Dia 1.50.1 bin |
| Library push | spring 0.245 s / ζ 0.92 | the card moving aside for the Library | Arc rec ×1 |
| Group expand | spring 0.30 s / ζ 0.82 | folder open and close (*interim*) | Dia rec |
| Spinner | 0 → −2π (clockwise), 1.88 s, linear, repeating | tab loading | Dia 1.50.1 bin |

Under Reduce Motion, Dia skips the New Tab intro and the PiP throw. Ardia also drops the sidebar slide and turns the panel grow into a fade. Ours: nothing moves or scales, and fades stay.

### Overlays

The command bar, menus, hover cards, Peek, find and every prompt over the page are child windows (decision 4). AGENTS.md › Over the page has the rules: a click outside closes the overlay and still does its own thing, and the host draws the overlay before showing it. Panels anchored to the URL field share one grid. **Ardia** set it on the owner's request; it is not measured from Dia:

- Content inset 16 on every side, corner radius 12. Rows 32 tall, with the hover fill inset 6 at radius 6.
- A 20 pt icon slot holds 13 pt glyphs, with text 10 pt after the slot, so titles start at 46. Separators sit 16 in, with 5 above and below.
- Header: the icon box is 16 from the top and left, with title and subtitle at 13/16 and 11/14 pt centred on it, and 8 pt below it.
- Placement: 8 from the card's side and 8 below the page's top. A panel taller than the page scrolls inside an 8 pt bottom margin.

### Keyboard

- Focus rules are in AGENTS.md › Keyboard with the page focused. Chrome's reserved commands reach the browser before the page sees them: New Tab, New Window, New Incognito Window, Close Tab, Close Window, Reopen Closed Tab, Next/Previous Tab and Quit. Other browser keys reach the browser only if the page doesn't keep them. A page keeps ⌘F, for example. Extensions can't take the reserved keys (Chrome).
- Where Arc and Dia bind different keys, Arc's key wins, because Arc refugees are the first users (decision 7). Chrome's key stays as a hidden alternate where it is free. *The owner should confirm this rule.*
- Every command gets a binding in Settings › Keyboard Shortcuts. A user's binding beats every other binding except the reserved commands (Dia, P§22).

| Action | Arc 1.170 (menus) | Dia | Ours |
|---|---|---|---|
| New tab / window / incognito window | ⌘T / ⌘N / ⇧⌘N | ⌘T / ⌘N / ⇧⌘N | ⌘T / ⌘N / ⇧⌘N |
| Command bar on the current URL | ⌘L | ⌘L | ⌘L |
| Close tab / window | ⌘W (archives) / ⇧⌘W | ⌘W / ⇧⌘W | ⌘W / ⇧⌘W |
| Reopen closed tab | ⇧⌘T | ⇧⌘T | ⇧⌘T |
| Clear Today tabs | ⇧⌘K | ⇧⌘K (Close All Tabs) | ⇧⌘K |
| Pin / unpin tab | ⌘D | — (⌘D bookmarks) | ⌘D, *owner to confirm* |
| Back to pinned URL | not in the menus | ⌘↩ | ⌘↩ |
| Toggle sidebar | ⌘S | ⌘S (Auto-Hide Tabs) | ⌘S |
| Little Arc / open it in the main window | ⌥⌘N / ⌘O | — | ⌥⌘N / ⌘O (⌘O stays Open File in main windows) |
| Switch Space 1–9 | ⌃1, one per Space | ⌃1–⌃9 (profiles) | ⌃1–⌃9 |
| Next / previous Space | ⌥⌘→ / ⌥⌘← | — | ⌥⌘→ / ⌥⌘← |
| Next / previous tab | ⌥⌘↓ / ⌥⌘↑ | ⇧⌘] / ⇧⌘[, ⌥⌘→ / ⌥⌘←, ⌥⌘↓ / ⌥⌘↑ | ⌥⌘↓ / ⌥⌘↑, with Chrome's ⇧⌘] / ⇧⌘[ hidden |
| Select tab 1–8 / last | ⌘1–⌘8 / ⌘9 in sidebar order, counting tabs in open folders (not in the menus; cap) | ⌘1–⌘8 / ⌘9 in sidebar order | same |
| Most recent tab | ⌃Tab, no switcher (rec) | ⌃Tab / ⌃⇧Tab with a switcher | Dia's (see Tab switcher) |
| Search tabs | not in the menus; measure | ⇧⌘A | ⇧⌘A |
| Split: add pane / close pane / focus next / previous | ⌃⇧= / ⌃⇧- / — / — | ⌃⇧= / measure / ⌃⇧] / ⌃⇧[ | ⌃⇧= / ⌃⇧- / ⌃⇧] / ⌃⇧[ |
| Find / next / previous | ⌘F / ⌘G / ⇧⌘G | ⌘F / ⌘G / ⇧⌘G | ⌘F / ⌘G / ⇧⌘G |
| Use selection for find / jump to selection / find and replace | — / ⌘J / ⌥⌘F | — (⌘E is Chat) / ⌘J / ⌥⌘F | ⌘E / ⌘J / ⌥⌘F |
| Copy URL / as Markdown / as quote | ⇧⌘C / ⌥⇧⌘C / ⌃⇧⌘C | ⇧⌘C / ⌥⇧⌘C / — | ⇧⌘C / ⌥⇧⌘C / — |
| Library / downloads | ⇧⌘L / ⇧⌘J (the Library's Downloads page) | — / ⇧⌘J | ⇧⌘L / ⇧⌘J (the downloads panel) |
| DevTools / console / inspect / view source | ⌥⌘I / ⌥⌘J / ⌥⌘C / ⌥⌘U | ⌥⌘I or F12 / ⌥⌘J / ⌥⌘C / ⌥⌘U | same |
| Back / forward | ⌘[ / ⌘] | ⌘[ / ⌘], ⌘← / ⌘→ | same as Dia |
| Reload / force reload / stop | ⌘R / ⇧⌘R / ⌘. | ⌘R / ⇧⌘R / — | ⌘R / ⇧⌘R / ⌘. and Esc |
| Zoom actual / in / out | ⌘0 / ⌘+ / ⌘- | ⌘0 / ⌘+ / ⌘- | same, plus hidden ⌘= |
| History / clear browsing data | ⌘Y / — | ⌘Y / ⇧⌘⌫ | same as Dia |
| Full screen | ⌃⌘F | Globe-F | Globe-F and ⌃⌘F |
| Settings | ⌘, | ⌘, | ⌘, |

Mouse (Dia and Chrome, P§ Keyboard shortcuts):

- ⌘-click opens a background tab. ⇧⌘-click opens a foreground tab. ⇧-click opens a new window. Middle-click opens a background tab.
- ⌥-click downloads. ⌥⇧-click opens a split pane.
- ⌘-click on back, forward or reload opens the result in a background tab.
- Double-click a tab to rename it.

## Phase 3 features

### Window

| Item | Value | Source |
|---|---|---|
| Card inset | docked: the card starts at the sidebar's width, with 10 at the top, right and bottom. Hidden: 10 on every side | Arc ax, cap |
| Card inset (reference) | left = sidebar width, top 6, right 7, bottom 7. With the sidebar hidden, 6 / 6 / 6 | Dia 1.49 cap |
| Card corner radius | Arc ≈ 6; a continuous corner fits the 2× capture a little better than a circular one. Dia 10, continuous | Arc cap, Dia cap |
| Traffic lights | Arc: 16 × 16, centres (20, 24), (43, 24) and (66, 24), a 23 pt pitch. Dia: centres x 24.75 / 47.75 / 70.75, y 26.75 | Arc ax, Dia 1.50.1 bin |
| A page before its first paint | the card colour, composited opaque over the tint, never white | Ardia, P§7 |

- **Dragging the window.** The empty sidebar below the rows, the header row outside its buttons, and the margins around the card all move the window (the left margin only while the sidebar is hidden). A double-click there does the system's title-bar action from System Settings › Desktop & Dock. The exception is a double-click on the empty sidebar, which opens a new tab (Dia 1.50; Arc: confirm). Rows, tiles, buttons and the URL field keep their own presses. The outer ~3 pt is the resize edge (Dia and Arc, P§1).
- **New window.** It appears at its final frame, already showing its content, with no empty frames. A closed window leaves the screen at once (Dia, P§1).
- **Close confirmations** (Dia, P§1). Closing a window with 2 or more tabs, or pressing ⌘W on the last tab, asks "Close 3 tabs?" with a breakdown per Space and "Don't ask me again". ⌘Q warns, and so does quitting with downloads in progress. Arc's equivalents: measure.
- **Page full screen** (Dia, P§18). The page fills the window and the sidebar goes away. Esc or the page's `exitFullscreen` brings the layout back. Switching or closing the tab ends full screen. The window enters macOS full screen for the page and leaves it only if it entered for that page. No frame of the transition may show the other layout. Arc's behavior in window full screen (user-initiated): measure.
- **Other window commands** (Dia, P§1). Keep Window on Top, Merge All Windows, Move Tab to Window. Moving keeps the live page: no reload, history kept.

### Sidebar

Arc's layout: the URL field and back, forward and reload live in the sidebar, and the card has no toolbar (decision 6). Dia's card toolbar, its top tab strip and the toolbar that hides on scroll are not ours.

**Header and URL field**

| Item | Value | Source |
|---|---|---|
| Row centre | traffic lights 24 below the window top; sidebar button, back, forward and reload 23.5 | Arc ax |
| Button hover plate | the button's own 32 × 32 frame, its top 7.5 below the window top, white ≈ 0.09; radius 6 (Arc (Ardia)). Only the plate takes clicks; the rest of the row moves the window | Arc ax, cap; P§4 |
| Sidebar button, back, forward, reload | 32 wide at x 77, 122, 156 and 190 (a 34 pitch), so back, forward and reload end 6 before the sidebar's 228 pt edge | Arc ax |
| Narrow sidebar | reload goes first, then forward; back and forward stay together | Ardia, from Arc users |
| Back or forward disabled | dimmed, about half the brightness of the other glyphs. Arc dims both on a fresh tab; while a page loads, reload becomes ✕ | Dia 1.52 cap, Arc cap |
| Nav glyph size | Dia 13 pt regular. Arc: measure | Dia 1.52 cap |
| URL field box | x 8–220 and 36 tall from 46 below the window top, the same plate as a selected tab row; radius ≈ 12, circular; fill white ≈ 0.10. Font: measure | Arc cap |
| URL field text | the host alone, `www.` dropped, at x 21 in Arc. View › Show Full URL (off by default) adds the path, query and fragment, with no trailing "/" | Dia 1.50.1 bin, D›URL bar; Arc cap |
| Hovering the URL | the full URL, host bright and path dim, as Dia. Arc keeps the host and shows only its two buttons | Dia 1.49 cap, Arc cap |
| Insecure page / zoom not 100 % | a lock-warning glyph / a magnifier (+ or −) that opens − / + / Reset. Arc shows neither in the field, so an http page looks like any other; ours keeps both. Arc's zoom flashes "125%" in a child window at the card's top right | Dia and Chrome, P§8; Arc cap |
| URL field buttons | Copy Current URL and Site Control Center, 30 × 30 at x 163 and 191, shown while the pointer is over the sidebar | Arc ax |
| Downloads button (reference) | Dia: centre (166, 26) at 190 pt, in the header. Arc has none there: downloads are a page of the Library | Dia, D›Sidebar; Arc |

**Width**

| Item | Value | Source |
|---|---|---|
| Default | Arc 228 on a fresh install, in a 1280 × 800 window and at 1512 × 949. Dia 190 | Arc ax and its saved windows, Dia 1.50.1 bin |
| Minimum / maximum | measure. Ardia uses 160 / 400. Dragging past a limit rubber-bands | Dia, P§4 |
| Persistence | kept per window; double-click on the handle resets it | Dia, P§4 |

**Sections, top to bottom** (Arc ax). Header, URL field, favorites, then the Space's name, its pinned tabs, the Clear divider, "+ New Tab" and the Today tabs, newest first. The name row has the Space's icon (26 × 26 at x 13), the name at x 41 and a More button at the right. The footer is 50 tall: a button named Home (32 × 32 at x 8), the Space switcher (x 42–186) and a + button (32 × 32 at x 188). The favorites stay put while the list scrolls. A favorites block taller than 40 % of the sidebar scrolls with the list instead (Ardia).

**Favorites** (Arc's favorites are Dia's Top Apps dock, so these values are *interim*)

| Item | Value | Source |
|---|---|---|
| Tile height / gap / side inset | 41 / 6 / 6 | Dia 1.50.1 bin and cap (1.49: 40) |
| First row top | 54 below the window top under Dia's header. Re-measure under Arc's URL field | Dia 1.50.1 |
| Columns | 3 across at 190 pt (x 6, 67, 128). Arc's rule for columns at other widths: measure | Dia, P§2 |
| Radius | 10 | Dia bin |
| Resting | fill white 0.10, 0.5 pt stroke white 0.14 | Dia 1.50.1 bin |
| Favicon | centred. Size: measure | Dia cap |
| Badge for a tile away from its URL | none | Dia 1.53.0 bin |
| Hover | hover card with title and URL (see Motion) | Dia, D›Tab hover card motion |
| Double-click | goes back to the pinned URL, like ⌘↩. A tile already there doesn't reload | Dia, P§2 |

The selected tile is themed by its icon (Dia 1.50.1 bin, D›Sidebar). Draw the icon at no more than 32 px on its long side. Average its pixels with alpha ≥ 0.975, then take their mean RGB distance from that average:

- **Distance ≥ 0.055 (colourful).** The tile is white 0.20 (dark) or white (light). Over it goes the icon at 22 % opacity, blurred with radius 5 and aspect-filled to 2.5× the tile. The ring is a 3 pt border showing the same blurred icon over black (dark: at 75 %, saturation 2, brightness −0.1) or white (light: at 100 %).
- **Distance < 0.055 (one colour).** The tile is filled with the mean colour, the icon is drawn as a white template, and the ring is white with soft-light compositing. A near-white icon (relative luminance > 0.88) gets a black fill and a white 0.30 ring instead.
- **No usable icon.** Fill black (dark) or white (light), with the default selection ring.
- A secondary selection (multi-select) gets the secondary fill and no ring. Emoji icons draw at 16 pt and are themed the same way.

**Pinned tabs** (Arc: per Space. Dia 1.52 has the same section.)

- They are rows that look like tab rows, between the Space's name and the Clear divider (Arc).
- **Folders** (Arc). A folder row shows an open or closed folder icon and the folder's name, and its tabs sit 16 further right. Hovering it shows the name as a tooltip. Closing and opening it through the accessibility tree moves the rows below in one frame; a click's motion is still to measure (Group expand, *interim*).
- Closing one (⌘W or ✕) unloads its page and keeps the row. With no Today tab left, ⌘W on a pinned tab shows another pinned tab whose page is still loaded. Closing the last tab closes the window (Dia rec, P§2).
- Away from its pinned URL, the favicon turns into a back arrow under the pointer and the title reads "Back to Pinned URL". A click goes back, and so does ⌘↩ (Dia 1.53.0 bin). Arc's look for a navigated-away pinned tab: measure.
- A pinned tab has one live page. Selecting it in a second window moves the page there, with history and playing media intact. The first window keeps a frozen frame under a wash: white 0.40 in light mode (Dia cap), black 0.25 in dark mode (Ardia). Clicking the frozen frame pulls the page back (Dia, P§2).
- Links ⌘-clicked in a pinned tab gather in a group just below the pinned tabs (Dia rec, P§2). Measure what Arc does: Peek covers some of these links.
- ⌘D pins or unpins. Dragging a tab above the divider pins it, below unpins it, and onto the favorites makes it a favorite.

**Clear divider and Today tabs** (Arc cap, ax and rec; the arrow, label and tooltip from Arc's binary and a user's screenshot, D›Arc note: Clear)

- Today tabs are the Space's unpinned tabs. They are archived (see Archive). "+ New Tab" is Today's first row, and new tabs open under it.
- The divider shows with the section while the Space has Today tabs. It has no hover state.
- The line is one device pixel (0.5 pt) at white ≈ 0.11, inset 18 from both sides of the sidebar, 7.5 under the last pinned row. While "Clear" shows, it stops 7.5 pt before the arrow.
- "Clear" shows only while the pointer is over the sidebar, on the divider's line, ending at 216 (12 before the sidebar's edge). The down arrow is 7 × 8 and sits 3 pt before the label. The label is semibold, with a cap height 0.75 of the tab titles' (≈ 10 pt beside 13 pt titles).
- The button's tooltip is "Clear unpinned tabs in this space". The menu item is Clear Unpinned Tabs (⇧⌘K). The command bar finds it by "close unpinned tabs", "clear tabs" and "clear today". No toast follows; ⌘Z undoes it (see Archive).

**Tab rows** (sizes from Arc ax and cap at the 228 pt default; the rest Dia 1.50.1 bin and cap, D›Sidebar rows)

| Item | Value |
|---|---|
| Pitch / row height | 41 / 40. The selected and hover plate is 36 tall, 2 in from the row's top and bottom (Arc). Dia: 37 / 34 |
| Side inset | the plate is inset 8 from both sides of the sidebar (Arc). Dia: 6 |
| Radius | Arc ≈ 12. Dia: min(height / 3, 10), continuous corners |
| Favicon | 18 × 18 at x 17, 11 below the row's top (Arc). Dia: 16 × 16 |
| Title | at x 43, 8 after the favicon; in a folder the favicon and title sit 16 further right (Arc). 13–14 pt (Dia 1.49 estimate; Ardia uses 13). It fades out at the trailing edge, never an ellipsis (Dia; Arc truncates with an ellipsis) |
| Selected | the selected fill plus a 0.5 pt inner hairline of white 0.15 on all four sides, and a white title (Dia). Arc's plate is white ≈ 0.2 over the sidebar |
| Hover | the hover fill (Dia; Arc's is white ≈ 0.11) and a close button 26 × 26 at x 189, 7 below the row's top, which the title truncates before. The selected row shows it under the pointer too (Arc) |
| Audio | a speaker glyph after the favicon. Muting applies to the site in that profile and follows navigation (Dia, P§2) |
| Loading | a 12 × 12 ring at width − 8 − 12, only in rows at least 48 wide, hidden under the hover ✕. The track and ring are 1.5 pt lines on a circle inset 1.25. The ring covers 72 % with round caps in the secondary label colour, the track is the same colour at 0.18, and it spins per Motion |
| Sleeping tab | faded favicon (alpha: measure) |
| "+ New Tab" row | Today's first row (Arc). Label white 0.55 (secondary), plus sign white 0.25 (tertiary); hover as rows (Dia) |
| New Tab tab's icon | our mark, 16 wide, white 0.28 in dark mode over any row state |

**Behavior**

- **Opening a tab** (Arc rec). The new row appears at the top of Today, under "+ New Tab", already selected, and the old selection drops at once. Its selected fill fades in and the rows below slide down one pitch, both on the List spring. They are drawn under the new row and revealed from beneath its bottom edge (Dia). The title reads "Untitled", then the host, then the page's title.
- **Closing a tab** (Arc rec). ⌘W on a Today tab archives it (see Archive) and selects the next row down at once. The rows below slide up one pitch on the List spring. The closed row stays where it was and fades out on the same spring, drawn under the rows sliding up over it. Hover follows the final layout at once, and several closes move together.
- **Hover card** for rows and tiles (Dia bin). It is a child window that scales from its top-left corner. In and out per Motion; both start from the current values, so a leaving card comes back from where it is.
  - It shows after 0.5 s, or 0.1 s from one hover source that wasn't identified. It shows at once when another card is already up.
  - Moving from row to row moves the card without animation.
  - A click or key press dismisses it. The leave delay is unknown (Ardia uses 160 ms). It takes no clicks until it has faded in.
  - Actions (Dia): Pin, Bookmark, Split, Picture in Picture, and Reset for a pinned tab.
- **Scroll edge** (Dia 1.53.0 bin). A 1 pt full-width separator sits 4 under the favorites. The list starts 5 under them with a 1 pt top inset. The separator fades in (Scroll edge curve) once the list scrolls past its top and fades out at the top or while rubber-banding.
- **Reorder** (Dia rec). The dragged item follows the pointer within a frame, stays opaque and draws over its neighbours. A neighbour moves aside once the item's leading edge passes the neighbour's centre (Reorder spring). On drop nothing else moves.
- **Dragging out of the list.** A tab dragged onto another window's list lights that list up and moves the live page there. Dropped on the desktop, it opens a new window under the pointer. Esc cancels a sidebar drag.
- **Dropping onto the tab list.** Files and http(s) links dropped on the list open as tabs at the drop gap (Dia). `javascript:`, `data:` and `file:` links from dragged text never open (Ardia).
- **Opening rules.** Links opened behind line up after their opener in the order opened. A link opened in front goes right after its opener. Closing a page goes to a tab it opened, then a sibling, then the opener (Chrome). Tabs opened behind load only when first shown; until then the row shows the address and the site's icon (Dia). The list scrolls to show a tab opened behind (Dia).
- **Other row actions** (Dia). Double-click a row to rename it inline. Change its icon to an emoji. ⌘-click and ⇧-click select several rows for bulk actions. The context menu shows shortcut hints. A haptic tick sounds while reordering, as a setting.
- **Hiding and showing** (Arc rec).
  - ⌘S or the sidebar button hides the docked sidebar (Sidebar hide) and shows it again (Sidebar show). The sidebar and the card's left edge move together, and the page is laid out at every step. Hidden, the card is inset 10 on every side, with no traffic lights, header or nav.
- **Sidebar reveal** (Arc rec, cap, ax).
  - The pointer at the window's left edge, at any height including the top-left corner, slides a floating panel over the page (Sidebar reveal in). The page doesn't move.
  - The panel is 228 × 939 at (5, 5): inset 5 from the window's left, top and bottom edges, as wide as the docked sidebar, with everything in it 5 right and 5 down of the docked layout. It is rounded and dark, with a 0.5 pt light border (Arc (Ardia)) and a shadow ≈ 11 pt around it. It holds the traffic lights, the header buttons, the URL field and the tabs.
  - It stays while the pointer is within 24 pt of its edge (Arc (Ardia) ≈ 21), then slides away (Sidebar reveal out).
  - Docking from the panel is instant: the page is laid out narrower in one frame (Arc (Ardia)).

**Tab switcher (⌃Tab)**

- Ours is Dia's (owner, 2026-10-24). A quick ⌃Tab still lands on the last tab with nothing drawn, as Arc's does, and holding ⌃ adds the way to reach older tabs.
- Arc (rec, 3 presses): ⌃Tab goes to the most recently used tab on the key press. No switcher shows, even while ⌃ stays down.
- Dia's (1.50.1 bin, P§2): recent tabs, most recent first. The overlay appears after 140 ms (Ardia; source unclear), and the switch commits on ⌃ release. → and ← move. Esc or a click outside cancels. Clicking a row switches to it, and the pointer highlights rows. Leaving the app switches. Other keys are swallowed while it's up.

### Spaces

**Model** (decision 5)

- A Space has a name, a colour, its pinned tabs, its Today tabs and a profile.
- Several Spaces can share a profile. Switching to a Space of another profile switches the window's cookies, logins and extensions with it.
- Moving a tab to a Space of another profile reloads it in that profile, after a one-time warning that the tab loses its session (Dia warns when moving a tab to another profile, P§1).
- Whether favorites belong to the Space or the profile: measure in Arc.

**Switcher** (Dia 1.50.1 bin, D›Profile paging. Dia's ARCUI and TabSidebar modules look inherited from Arc, so these values are probably Arc's too: confirm.)

| Item | Value |
|---|---|
| Footer | Arc: 50 tall, with Home, the switcher and + (see Sections, ax). Dia: 41 tall, a Library cell at the left, an overflow button 42 wide, 3 from the right |
| Switcher band | 28 tall, centred in the footer above its bottom 3 pt, 31 in from each side when those buttons show |
| Items | 12 pt per item, centred. At most 7 (84 pt) before it scrolls, with edge items scaled 0.75–1 |
| Dot | 6 × edge scale, round. Label colour at 0.85 selected, 0.4 hovered, 0.2 otherwise |
| Click | focuses that Space |

Arc shows Space icons in place of dots when a Space has one: measure both looks.

**Switching**

- Click a dot, press ⌃1–⌃9 or ⌥⌘→ / ⌥⌘←, or swipe horizontally with two fingers over the sidebar.
- A click, key or menu pages both sidebars together by the sidebar's width (Space page, Arc rec ×3).
- Paging is Dia's (bin):
  - Tracking moves the pages at half the scroll distance (−0.5 × the horizontal delta), whatever the system's "Swipe between pages" setting says. Magic Mouse one-finger swipes count too. Ardia tolerates 12 pt of vertical wobble, then locks to horizontal.
  - Past the first or last Space, the pages rubber-band: `255·(1 − 1/(0.15·x/255 + 1))`.
  - Release velocity comes from the last 100 ms of samples. At |velocity| ≥ 5 pt/s the target is the next Space in that direction, otherwise the nearest. A swipe moves at most one Space from where it began. The settle carries the release velocity when it points at the target.
  - Settle per Motion. A haptic detent sounds as the nearest Space changes.
  - Wheel mice: deltas sum while they come less than 0.049 s apart. Once |sum| > 1 pt, it pages one Space and then ignores the wheel for 0.25 s.
- The window tint cross-fades between the two Spaces' colours during a swipe (Ardia; Dia and Arc durations: measure).
- **New Space** (Arc cap, ax, rec). Spaces › New Space… pages to a "Create a Space" form in the sidebar: a name field, a Profile picker ("Default"), Choose a Theme, then Create Space and Cancel at the bottom. The card empties to the new Space's background, and the URL field's row goes for as long as the form shows. Esc doesn't cancel. Cancel doesn't page back: the old Space replaces the form in one frame, 28 pt left of its place, and springs the rest (0.195 s / ζ 1.0, rec ×3).
- Measure in Arc: rename, icon and colour flows, and how a Space shows its profile outside the form.

**Colour.** The Space colour drives the window gradient (Shared rules), the New Tab band and halo, and the accent. Dia's hue families are neutral, red, orange, yellow, green, blue, pink and purple. Only pink is measured: tint #B25B6B (P3), New Tab band #B5556B. Arc's presets and editor are under Themes.

**Agent Space** (ours: decision 8 and the `prototypes/arcish` reference). The kind exists in the Phase 3 data model; its UI comes in Phase 4. Neither Arc nor Dia has a reference, so the layout values are ours to set.

- It is a Space whose tabs an agent opens. It shows in the switcher with its own mark and in the sidebar like any Space.
- Agent actions never change the user's selected Space or tab, the keyboard focus or the cursor. Agent tabs never come to the front.
- **Confirmations.** A card in the sidebar offers Allow, Deny or Always allow on <site>. The Space lists always-allowed sites, each with Revoke. A confirmation times out after 5 minutes (prototype).
- **Handoff.** At a sign-in, 2FA prompt or CAPTCHA, the tab pauses and a card reads "<agent> needs you", with Show tab and Hand back. It times out after 15 minutes (prototype).
- **Take over / Hand back.** The Space has the button; while the user drives, the agent's actions pause.
- **Status and activity.** The status line reads "Waiting for you" or "Waiting for your OK". The activity log in the Space lists actions newest first.
- Dragging one of the user's tabs onto an Agent Space gives the agent that tab.

### Command bar

Dia's polish (decision 6). Parsing, inline completion and ranking follow Chrome's omnibox, because Dia runs Chrome's AutocompleteController (O).

**Opening**

- ⌘L or a click on the URL field opens the panel out of the field, with the URL selected. On the New Tab page in our layout, ⌘L and a click both open it at the sidebar field, not at the centred box (Ardia, from user reports).
- ⌘T opens the New Tab page (see New tab). Arc opens the command bar over the current page instead and makes no tab until a pick (Ardia's opt-in, P§7). Decision 6 picks Dia's.
- ⇧⌘A opens Search Tabs.

**Panel** (Dia 1.52 rec unless noted)

| Item | Value |
|---|---|
| Width | the field's width plus ≈ 41 on the left and ≈ 40 on the right: 888 for an 810 field. Arc's ⌘T and ⌘L panel is 766 wide, centred on the window rather than the card and in the same place with the sidebar hidden, with suggestion rows 50 tall, 5 of them showing and the rest scrolling, and opens in one frame (Arc cap, ax, rec) |
| Text | the same x, baseline and size as in the field (13 pt); 48 in from the panel's left edge. The search glyph is centred 30 in |
| Open | grows out of the field's box (Panel grow). Its left edge reaches 63, 82, 92, 95 and 99 % on successive 60 fps frames, and the height grows with it. The content doesn't move or scale; the growing shape uncovers it |
| Close | Arc: in one frame, no fade (rec ×3). Dia: measure |
| Radius | 20 continuous for the New Tab box (bin); ≈ 16 for the URL panel (1.49 cap): measure |
| Fill | opaque #2D2D2D / #FEFFFF (1.50.1 bin; the cap reads (45, 45, 45)). The 1.49 URL panel was ≈ #2A2A2A |
| Border | 1 device pixel, rgba(120,125,134,0.32) in dark |
| Shadow | black 0.12 (Dia's default), plus from 1.50 two layer shadows: black 0.08 radius 2 offset (0, 0.5), and black 0.04 radius 1 offset (0, 2) |
| Scrim | none: the page stays visible and undimmed (1.49) |
| Input row | 54 tall (1.49 cap). The leading icon becomes the selected suggestion's favicon |
| Suggestion row | 36 tall, 16 pt favicon, title ≈ 15 pt white, " — url" at 0.5 (1.49 cap) |
| Selected row | warm grey ≈ #5E5A5D, radius 10, inset 4 (1.49 cap). Arc fills it with the Space's colour (Arc cap) |
| Bottom row | 53 tall (1.49). Dia's "+ Add tabs or files" chip and mic are AI and are dropped. The "Go ↩" pill (≈ 92 × 29, lavender ≈ #DCCBD8, dark text) stays and names the search engine (Ardia) |

**Rows**

- The Enter target is the inline match, else a typed URL, else a search for what was typed. Page titles are never the default (Chrome). Dia's "Use website first" setting puts a strongly matching page first instead, and "Prefer search engine" turns that off.
- The search row comes right after the top hit (Dia 1.49).
- Row 0 is computed from local data only; remote suggestions append below it, so row 0 never flickers (Chrome).
- Open tabs appear as "Switch to Tab" rows across windows, never the current tab (Dia).
- History rows can be removed with a hover ✕ or ⇧⌦, and the selection moves to the next row (Chrome; Dia has the ✕).
- Bookmarks, a calculator (↩ copies the result) and commands are also rows (Dia). A command row shows its shortcut as a chip at the right, such as Clear Today ⇧⌘K, and the field's icon becomes the command's (Arc cap).
- Hovering highlights a row; Enter uses the keyboard selection (Chrome). Arc's pointer didn't highlight a row while the keyboard selection was elsewhere (cap ×1). Dia: measure.
- Arc updates the suggestions 33–42 ms after a typed character appears, in one frame, and the placeholder goes with the first character (rec ×3). Ours has to match or beat that (decision 4).

**Inline completion** (Chrome; Dia rec, O)

- It completes only one term, with the caret at the end.
- It never completes during IME composition, and ↩ then commits the composition instead of navigating. It completes again after the composition is committed.
- It never completes after a deletion (⌫, ⌦, ⌥⌫, ⌘⌫ or cut) until the next typed character. After a deletion, the search row goes first.
- It never completes right after a paste, or when the input ends in whitespace.
- Hosts complete first; paths only when the typed text contains "/". The typed prefix keeps its case.
- Only hosts the user has typed are eligible: typed once for a host, twice for a deeper URL. Bookmark-only and open-tab-only hosts are listed but don't inline (Chrome).
- → or ⌘→ accepts the completion, and so does a click at its end. A ⌫ after accepting deletes one character and doesn't complete again.

**Site search.** Tab on a hinted row, or Space after an exact host or engine keyword, enters the site's search. Arc hints it at the field's right, "Search Wikipedia" with a Tab key, while the completion is that site (Arc cap). ⌫ in the empty field leaves it and restores the text, keeping the space when Space entered it, and the bar stays open (Chrome).

**Keys**

| Key | Action | Source |
|---|---|---|
| ↩ | open in this tab | Chrome |
| ⌘↩ / ⌥↩ | new tab: ⌘↩ behind and ⌥↩ in front. Arc puts the ⌘↩ tab at the top of Today and keeps the bar open for the next one. Dia: measure | Chrome; Arc cap |
| ⇧↩ | new window | Chrome |
| ⇧⌘↩ | search for what was typed | Dia |
| ⌃↩ / ⌃⇧↩ | add www. and .com / the same in a new window | Chrome |
| ↑ ↓, ⌃N ⌃P | move, wrapping at the ends | Chrome |
| ⇞ ⇟ | first / last row | Chrome |
| Tab / ⇧Tab | enter site search when hinted / previous row | Chrome |
| Esc | leave site search; otherwise close the panel, drop the typed text and give the page its focus back | Ardia |
| ⇧⌦ | remove the selected history row | Chrome |

- ⌘V, ⌘C, ⌘X or ⌘A pressed right after ⌘T, ⌘L or ⌘F waits up to 400 ms for the new field and lands there, never in the old page (Ardia, from a user report).
- A click anywhere outside the panel, or another window or app taking focus, closes it like Esc; the click still does its own thing.
- **URLs** (Chrome's `AutocompleteInput::Parse`). A dotted host without a scheme goes to https, falling back to http. Single labels, localhost, IP addresses and `.local` hosts go to http.
- Paste and Go and Paste and Search are in the context menus. Copied links have trackers stripped (Dia).

### New tab

Dia's New Tab page (decision 6). The UI draws it; it is not a web page. That keeps New Tab → first paint off the JavaScript runtime (decision 4). A New Tab tab has a sidebar row and no page until the user goes somewhere.

| Item | Value | Source |
|---|---|---|
| Page | the card over the Space tint, flat once the intro ends | Dia 1.50.1 cap |
| Box width | 652 for a content area 708–1700 wide, 774 above that; below 708, min(width − 20, 616) | Dia bin, D›New Tab page |
| Box x | max(midX − width/2, 14), measured 1 pt right of that | Dia bin + cap |
| Box top | max(contentHeight/2 − 158, 100) + 38 within the card's content area, measured 1 pt lower | Dia bin + cap |
| Box height / radius | 112 / 20 continuous. Fill, border and shadow as the panel | Dia 1.50.1 cap, bin |
| Magnifier | centre 30.5 below the box top, ≈ 30 from its left, ≈ 14 wide | Dia 1.50.1 cap |
| Bottom row | centred 30.5 above the box bottom. Holds the Go pill, greyed and inert while empty | Dia cap, Ardia |
| Placeholder | "Search or enter address". Dia's "Ask anything…" is AI | Ardia |
| Mark slot | Dia's: an 84 pt view, the mark 76 wide, centred 48.6 above the box top. The mark itself is ours or none | Dia 1.50.1 bin + cap |

**Intro** (Dia 1.50.1 bin and 60 fps cap, D›1.50 New Tab intro). There is no box entrance, mark entrance, area light or edge light. Reduce Motion skips the whole intro.

- **Band.** A soft band in the Space colour rises from the bottom (neutral: grey 0.5 at 0.65). It is blurred with radius 24 and covers from ≈ 125 above the box to ≈ 20 above the bottom edge. It peaks 100–200 ms after the page appears, passes the box at ≈ 300 ms, holds a low plateau from 400 to 1200 ms, then fades linearly to nothing by ≈ 3.25 s.
- **Halo.** A 1 pt line just outside the box edge, half under the box, at radius 22, with a glow inside a frame 50 outside the box. It lights at the bottom centre at 225 ms, runs up both sides (peak ≈ 300 ms), meets at the top at 376 ms and is gone from the top by 529 ms. The line is at × 0.4 in dark mode; the glow is × 0.1 dark and × 0.12 light. The fragment maths is in D›1.50 New Tab intro.

**Behavior** (Dia)

- New Tab pages left untouched close when the app resigns active or the screen locks.
- Going back to a New Tab page restores its query.
- Dia 1.49's area light ran a 30 fps display link for up to 60 s, and its edge light and glass orb went with it. They are off in 1.50.1 and not ours: an idle New Tab page does no work.

### Split view

Dia's values (P§6, D›Dragging tabs and split targets) are the targets where Arc isn't measured. In Arc (cap, ax, rec ×1), View › Add Split View (⌃⇧=) splits in one frame with no animation: the page keeps the left pane, (228, 10) 632 × 929, and the new right pane (x 870–1502) opens empty with the command bar over it, centred at x 1049 rather than on the pane or the window. A 10 pt gap separates the panes, and ⌃⇧- closes the focused one. Measure in Arc: the maximum pane count, the focus indicator and the drop zones.

- **Panes.** Up to 3, side by side or stacked; a bottom split can be added. Each pane has its own nav state and close button. Unfocused panes dim (amount: measure). Commands: Flip, Change Orientation, Move Pane Left/Right, Remove from Split View, Separate All.
- **In the sidebar.** A split is one row with a segment per pane, and it counts as one row for ⌘1–⌘9. In Arc (cap) two panes give two equal segments with a thin divider between them, each with an 18 pt favicon and a truncated title: favicons at x 33 and 129, titles at x 61 and 157, the divider at x ≈ 122.
- **Creating a split.** ⌃⇧= adds a pane. ⌥-click on "+ New Tab" opens a New Tab pane. ⌥⇧-click on a link opens it in a right pane. Dragging a tab onto the page shows the targets.

**Drag targets** ("Add left split" and "Add right split" at each pane's sides)

| Stage | Size (fraction of the card) | From the card's side | Look |
|---|---|---|---|
| At rest | 198.5 × 333.5 (0.132 × 0.383), centred vertically | 37.5 | radius ≈ 20; fill a step lighter than the page with a lighter 0.5 pt edge; a dashed rounded rect 9.5 inside it (6 pt dashes, 6 pt gaps, ≈ 1.75 pt wide); a ≈ 19 pt half-filled-rectangle glyph above a 13 pt semibold title at white ≈ 0.6 |
| Grown, right after sliding in | 322 × 532 (0.215 × 0.611) | 30 | the same |
| Under the dragged card | 362 × 600 (0.241 × 0.69); its centre leans 43 % of the way toward the pointer's x | — | fill the accent at ≈ 0.15; dashes, glyph and title in the accent; 7 pt dashes and gaps, ≈ 2 pt wide |

- **Timing.** The targets slide in from outside the card's edges in ≈ 110 ms, then grow over ≈ 120 ms with no overshoot. In the top strip they appeared when the pointer was 26 pt below the strip. For the sidebar, measure the trigger in Arc.
- **What the drag looks like.** For a frame or two the tab itself follows the pointer. Then it becomes a 209 × 109.5 picture of a window, centred on the pointer, with a 0.5 pt light rim, a shadow and traffic lights. Over another window's tab list it turns back into a tab in ≈ 50 ms. Ardia then opens a tab-wide gap at the pointer in 120 ms; Dia's: measure.
- **Drop.** On a target, the page splits and the new pane slides in on that side (timing: measure). Away from a target, the tab opens in its own window, which grows out of the card at the pointer in ≈ 200 ms and takes focus. A pane leaves its split the same way. Dragging the only tab on a page splits it with the tab used before it.

### Archive

Arc's structure. Dia replaced it with Clean Up Tabs, recorded below for reference.

- **Clear** (Arc rec ×3). ⇧⌘K or the Clear button closes every Today tab of the Space except the selected one, in one frame. Pinned tabs and favorites stay. No toast shows. ⌘Z brings the tabs back in place, fading in on the List spring, with the shown tab still selected and the rest unloaded (Ardia's detail). Ardia's note, from Arc's binary, has Clear close the selected tab too and a toast offer Undo; Arc 1.170 does neither.
- **Arc's clear actions.** Arc's binary has three: clear all, clear unselected Today tabs, and tidy-and-clear. Tidy is AI and out of scope.
- **Dia's Clean Up Tabs** (⌥⌘K) moves untouched tabs to "Recently Cleaned", with a daily automatic run. It is not ours.
- **Closed tabs.** Closed tabs reopen with ⇧⌘T, newest first, with position, pin, group, custom name and icon, mute state and back/forward list (Dia, P§2).
- **Archived tabs** (Arc cap, ax). ⌘W on a Today tab archives it; the menu item is Archive Tab. The Library's Archived Tabs page lists them newest first under day headings ("Today"): a 16 pt favicon at x 134, the title at x 164 with the host under it, on a 56 pt pitch.
- **The Library** (Arc cap, ax, rec ×1). Window › View Library (⇧⌘L) and View Downloads (⇧⌘J) put it in place of the sidebar: an icon column (Media, Downloads, Easels & Notes, Spaces, Archived Tabs; 97 × 80 buttons on a 100 pitch) and the chosen page beside it, with a search field and Filter on the traffic-light row. The card moves aside to x 417 without resizing, running off the window's right edge (Library push). Esc doesn't close it; its back button does. Media and Easels & Notes are out of scope. Ours keeps the Library for archived tabs; ⇧⌘J opens the downloads panel instead (see Downloads; owner, 2026-10-24).
- **Auto-archive** (Arc cap). A Today tab not visited for 12 hours is archived, the default in Settings › General › Archive tabs after.
- Measure in Arc: the other auto-archive intervals, searching the archive, and restoring a tab from it.

### Profiles

A profile is cookies, history, logins, passwords, autofill and extensions (decision 5). The user switches Spaces, and the profile comes with the Space.

- Create, rename and delete a profile, and pick it for a Space. Arc's UI for both: measure.
- **Incognito** (Chrome and Dia). ⇧⌘N opens a private window of the current Space's profile, dark by default. Private windows of one profile share one session; other profiles' don't. The session ends with the profile's last private window. Its downloads and history are never written to disk.
- **Dia's profile indicator**, kept for reference only (Dia 1.50.1 bin, D›Profile paging). It sits in the header beside the traffic lights: the name in 13 pt semibold with 9 pt padding, 34 tall, and a hover/press fill at radius 10. Its colour is the Space colour mixed 0.6 toward white (dark) or 0.4 toward black (light). It shows the whole name, a truncated name if at least 52 pt fit, or else a person glyph (14 pt) in a 32 pt button. Its tooltip is "Switch between profiles". It appears only with 2 or more profiles. Measure whether and where Arc shows a Space's profile.

### Extensions

Chrome's extension system, per profile.

- **Toolbar** (Dia 1.52 cap, D›Toolbar).
  - Only pinned extensions show: 16 pt icons on a 30.25 pitch, with no puzzle-piece button. An overflow button appears only when the pinned ones don't fit.
  - A disabled action is dimmed.
  - In our layout they sit in the sidebar's URL field (Ardia: as many as leave 84 pt for the host, which is two at 190 pt). Measure where Arc puts them.
- **Menu bar.** An Extensions menu has one item per enabled extension (opening its popup or running its action), then Add Extension…, Manage Extensions… and Pin Extensions… (Dia).
- **Popups** (Chrome). A popup sizes to its page between 25 × 25 and 800 × 600, shrinking as well as growing. `chrome.action.openPopup()` opens it over the active tab.
- **Action state.** Badge, title, icon and enabled state update from Chrome's change notifications. Ardia polls every 1.5 s; we don't (principle 1).
- **Side panel.** A card right of the page with a header (icon, name, ⋯ menu and close), resizable from 320 to 600 with its width saved (Dia; the range is **Ardia**'s).
- **Install** (Chrome's flow, Dia's dialog). The Web Store's own button starts it. The confirmation is our dialog listing Chrome's permission warnings, and the store's buttons read "Add to <browser>" and "Remove from <browser>". Re-enable and new-permission prompts use the same dialog. Removal asks "Remove “<name>”?" with Cancel and a destructive Remove. Measure Arc's prompt: size, layout and position.
- **Keyboard commands.** Extension `commands` fire with the page focused. A key the user sets wins over ours, except the reserved commands. Suggested keys that clash with ours aren't assigned (Chrome).
- **Site Controls**, the panel from the URL field. Arc's layout per Ardia's tester, in Dia's styling, with **Ardia**'s numbers:
  - Action tiles (Share, Copy Clean Link, Find, then PiP or Cast).
  - An extensions grid: 58 pt cells, 8 pt gaps, rows of 4, the + tile in the next free cell. Right-click gives Pin/Unpin, Options, Turn Off, Remove and Manage.
  - The site's permissions, each with its value.
  - A footer with the connection state.
  - It opens in 150 ms and closes in 100 ms back to its anchor. Measure Arc's.

### Passwords and autofill

Chrome's password manager and autofill fill pages. Our prompts replace Chrome's bubbles (Dia, P§16). No Dia values were measured: the layout uses the panel grid (**Ardia**) until Arc is measured.

- **Saving** (Chrome, Dia's prompt). After a login, the prompt "Save password for <site>?" sits at the page's top right with Save, Never and Not Now. Update replaces Save for a changed password and has a username picker. Address and card offers ("Save address?", "Save card?") use the same prompt in Chrome's words. Switching tabs closes a pending offer.
- **Filling** (Chrome). Chrome's dropdown sits under the focused field. ↓ ↩ fills every field. Edit › AutoFill › Contact…, Passwords… and Credit Card… open it at the focused field; with no field focused they open the settings page.
- **Generation** (Chrome). Chrome's popup on new-password fields offers "Use Strong Password" and "Choose Your Own". Its text says the password is saved on this device.
- **Reveal.** An eye button appears in a page's password field once the user types. It never appears for a filled saved password (Dia).
- **Settings.** The password list unlocks with one device check (Touch ID or the account password). Revealing a password within 5 minutes asks nothing more (Chrome).
- **Passkeys.** Chrome's WebAuthn dialogs appear centred over the page as child windows.
- **Agents.** Agents never read or type into password fields and never see autofill data (decision 8). There is no agent path to any of this.
- Measure Arc's save prompt and password settings.

### Downloads

- **Button** (Dia 1.50.1 bin). It shows while the window's profile has downloads, in progress or finished, and disappears when they're cleared, with no show or hide animation. A new download flies into it; the "magnet" constants 0.1 s and 1.3 are not decoded, so record it in Arc. Arc's header has no such button; where Arc shows a download in progress: measure.
- **Panel** (Dia, P§14). ⇧⌘J or the button opens it, laid out on the panel grid.
  - Each download has progress, Pause/Resume, Cancel, Open, Show in Finder, Open When Done and Move to Trash, and its file can be dragged out to Finder, other apps, the page or the tab list.
  - Clear empties the list.
  - The list is per profile and persists across launches; private windows' downloads are never written.
- Quitting with downloads in progress asks first.
- Ours is Dia's panel (owner, 2026-10-24). Arc has none: ⇧⌘J opens the Library on its Downloads page (see Archive), which pushes the page aside to show one file, while the panel keeps the page in view and the file one drag away.

### Find in page

- **Keys** (Dia, P§8). ⌘F, ⌘G, ⇧⌘G and ⇧↩ for previous. ⌘E (Use Selection for Find), ⌘J (Jump to Selection), ⌥⌘F (Find and Replace in the focused field).
- **State.** Each tab keeps its own query and shows "n of m".
- **Find bar.** It floats over the page, so it is a child window. With the page focused, ⌘F reaches us unless the page keeps it (as Google Docs does). Japanese IME works in the field. ⌘V right after ⌘F lands in the field.
- **Arc's bar** (cap, ax, rec ×3). 300 × 32, floating 10 in from the card's top and right edges, radius ≈ 6, fill (19, 29, 50), with a soft shadow mostly below it (≈ 16 pt). Inside: a search icon 28 × 28, the field (152.5 wide), the count as "1/12", previous and next 28 × 28, a divider and close 28 × 28. With no match, "NOT FOUND" in red capitals (219, 25, 41) replaces the count and arrows. It opens and closes in one frame.

### DevTools

Chrome's DevTools (Dia, P§23).

- **Keys.** ⌥⌘I or F12 toggles them. ⌥⌘J opens the console, ⌥⌘C inspect, ⌥⌘U view source (a `view-source:` tab). The page menu has Inspect.
- **Docking.** They dock on the right by default, and the dock side is remembered per profile. Arc docks them inside the card at a third of its width (421 of 1274), and opens them in two steps: an empty dark pane at once, then DevTools and the narrower page ≈ 140 ms later (cap, ax, rec ×1). An undocked window is titled "Developer Tools - <url>", opens where it was last left, and is 640 × 640 the first time.
- **Task Manager.** Window › Task Manager opens Chrome's.
- **Agents.** DevTools are for the user. Agents get no DevTools and no remote-debugging port (decision 8).

### Import from Arc and Chrome

The user runs it; there is no agent path, because it reads passwords and cookies.

- **From Arc.** Arc Spaces become our Spaces and Arc profiles become our profiles (decision 5; Ardia maps Spaces to profiles). Favorites come in as favorites and each Space's pinned tabs as its pinned tabs (Dia 1.52 imports Arc this way, P§20).
  - Custom tab names come along.
  - Each Space's theme comes from Arc's `windowTheme`: colours in extended sRGB, a third overlay colour, intensity, noise, texture, appearance, translucency style, and separate light and dark looks (Arc 1.167 schema, P§20).
  - Measure Arc's folders and its "Air Traffic Control"-style link rules, if the captured version has them.
- **From Chrome** (P§20).
  - It brings bookmarks, history, open tabs, passwords, addresses and cards.
  - It brings cookies, so the user stays signed in. Imported cookies never replace one the profile already has and never go into a private profile.
  - The source's Web Store extensions are listed with links to the store and never installed silently.
  - On current macOS, Chrome and Brave need Full Disk Access. The importer explains why, links to System Settings and checks again when the user comes back.
  - Running the import again adds nothing new.
- Arc has no bookmarks bar. Where imported Chrome bookmarks land: measure Arc's own importer.

### Updates

Sparkle 2 (Dia and Ardia, P§24). Its feed URL is a default service URL, so forks change it (principle 2).

- "Check for Updates…" sits in the app menu. Automatic checks run every 8 hours (**Ardia**).
- An install waits until no window is in full screen.
- The first launch after an update opens its release notes.
- Settings › General › Updates.
- Measure Arc's update prompt, if one can still be triggered.

## Phase 4 features

### Peek

Ardia's docs record nothing of Arc's Peek. Everything about it is in the capture plan:

- which clicks open it;
- its geometry, backdrop and buttons;
- how it opens, closes and expands into a tab.

It floats over the page, so it is a child window (decision 4).

### Little Arc

Arc's keys are ⌥⌘N, and ⌘O to open the page in the main window. Measured in Arc 1.170 (cap, ax) unless marked.

- **Opening.** ⌥⌘N opens a 490 × 329 window at the primary display's top right, 20 in from its right edge and ≈ 20 below the menu bar, whichever display the main window and the pointer are on. At first it holds only a command bar: the field, a close button and suggestion rows 50 tall, as in the main window.
- **Bar.** After a pick, the same window shows the page under a 47 pt bar: the window buttons (16 × 16 at x 9, 32 and 55), the favicon, the host in a pill, a copy-link button, and "Open in <Space> ⌘O". The first time, a callout over the page explains "Links Open in Little Arc", "Open in" and ⌘W. ⌘L or a click on the bar opens a compact command bar (Ardia). ⌘O moves the live page, with history, form text, scroll and playing media, into the main window and closes Little Arc (Ardia).
- **Closing.** ⌘W closes the window and drops the page. Esc closes it too when the page doesn't use it (Ardia). The page stays in History, and ⇧⌘T reopens it in Little Arc with its back/forward list (Ardia).
- **Links from other apps** open in a new Little Arc each time; a setting switches this to "a new tab". The page's own links stay in it. `target=_blank` and ⌘-click open as tabs of the main window, behind (Ardia).
- **Where links land.** Ardia uses the profile of the window used last. Ours: the Space used last.
- Measure in Arc: the bar's colours, the window's radius, the open and close animations, Esc, and how ⌘O moves the page.

### Themes

Arc's theme, read by Ardia from Arc 1.167's shader and the code that feeds it (P§21, `profileTheme.ts`). The theme is per Space.

- **Editor.** A colour pad (hue around the circle, saturation outward) with 1–3 handles; Intensity; translucency from opaque to pass-through blur; Noise with a choice of 4 textures; Light / Dark / Match System. The window repaints live while the user drags.
- **Rendering.**
  - Colours 1 and 2 mix in OKLab along a 45° diagonal, with colour 2 reached at 75 % of the way. A third colour fades in from the top to the bottom.
  - The layer's alpha is 0.2 + 0.55 × intensity. It sits over a white (light) or black (dark) underlay, over the blur behind the window.
  - The texture is multiplied in at noise × its strength: Arc's four textures have strengths 0.8, 0.15, 0.6 and 0.4. Inactive windows show the texture at a quarter strength, and the blur goes opaque.
  - Our textures and their names are our own.
- **Appearance.** View › Appearance › Automatic, Light, Dark (Dia).
- Measure Arc's editor UI and preset swatches.

### Agents UI

See Spaces › Agent Space. The confirmation UI and the activity log ship in the same change as the MCP server (decision 8).

## Picked by the owner, not on the roadmap

Boosts are Arc's. Ad blocking and environments go beyond Arc and Dia; the owner picked them (decision 6). Add them to a phase before building them.

### Boosts

Arc kept Boosts in its account and shared them in a gallery. Ours are local files with no account (principle 2).

- **What a Boost is** (Arc). CSS and a script the user writes for one site, plus Zap: click an element to hide it. Measure Arc's editor, Zap, and where a Boost starts from.
- **Storage** (Flecha). A folder of plain files, one Boost per site, so the user can keep them in git.
- **Applying** (Flecha). CSS changes apply live; a script change applies on the page's next load.
- **Seeing and turning off.** Site Controls lists the site's Boosts, each with an off switch, and Settings turns them all off. Arc's look: measure.
- **Agents.** A Boost script runs inside the user's logged-in pages, where it can read what principle 3 keeps from agents. Agents get no path to read, write or turn on a Boost (decision 8). Any local program can write to the folder, a coding agent included, so a script that changed outside the browser stays off until the user turns it on again in the browser.

### Ad blocking

Helium and Flecha block ads and trackers by default; Arc and Dia don't.

- **Default.** On in every profile.
- **Engine** (Helium). Helium's fork of uBlock Origin, with its filters and CNAME uncloaking, *interim*. uBlock Origin is a Manifest V2 extension. ungoogled-chromium's patch keeps Manifest V2, and on Chromium 155 uBlock Origin 1.75 blocks requests and hides elements with nothing more from the engine. On its first launch it lets requests through for about 3 s, until its lists have loaded.
- **Turning it off.** Per site in Site Controls, and everywhere in Settings.
- **Filter lists.** Updating them never wakes an idle browser (principle 1). Where they're fetched from is a default service URL that forks can change (principle 2).
- **Cost.** Page-load and memory numbers, with and without it, against the performance gate.
- **Agents.** They can't change it.

### Environments

From Flecha: one project's origins, such as local, staging and production.

- **Setup.** The user adds, edits and removes a project's origins in Settings.
- **Switching** (Flecha). ⌥⌘E (*interim*; check for clashes) opens the same page on the project's next origin. Only the origin changes; the path, query and hash stay. The command bar lists the page's other origins.
- **Badge** (Flecha). The URL field names the environment of the page's origin. Production is red, so it's never mistaken for a safe place to click around.
- **Agents.** On a production origin, an agent asks first, as for pay and send (decision 8). Owner to confirm.

## Measured in Dia, not on the roadmap

These are kept so the numbers aren't lost. Add them to a phase before building them.

- **Picture in Picture** (Dia 1.50.1 rec, D›Picture in Picture).
  - At rest: the video only, with 6 pt circular corners, a 1 pt rim of white 0.17 inside the edge and a soft shadow. It appears ≈ 125 ms after the tab switch with no animation.
  - On hover: a black 0.35 scrim fades in over 200 ms, ease-in-out. Back-to-tab (28 × 28) and close (29 × 28) buttons sit 12 from the top and their sides, radius ≈ 6, white 0.09 (≈ 0.13 under the pointer). The origin shows in 13 pt, centred, baseline 30. Pause is two 7 × 42 capsules 10 apart. A 5 pt progress bar runs from 12 to width − 12, 9 above the bottom; the played part is white, the rest white ≈ 0.27.
  - Close fades it out in ≈ 100 ms and the video keeps playing in its tab. Back to tab fades it out in ≈ 70 ms.
  - Throw (Dia 1.53.0 bin): a release faster than 350 pt/s (or a last swipe delta over 6 pt) throws it to the screen corner within 45°. It lands 10 pt in, on a spring of 0.9 s / ζ 0.9 that starts at the throw velocity. A slow release stays where it is let go.
- **Tab groups in the sidebar.** The header shows an icon, the name and a chevron 8 after the name at white 0.5. Opening and closing take ≈ 150 ms (140–160 measured, fast at first). Arc has folders instead: measure them.
- **Two-finger swipe back and forward.** A custom overlay with a chevron and a haptic at the threshold. It follows System Settings › Swipe between pages, and a carousel that can still scroll keeps the gesture.
- **Not our structure.** Dia's top tab strip, its card toolbar, and the toolbar that hides while scrolling. Their values stay in D›Top tab strip and P§4.

## Where Ardia conflicts with our decisions

| Ardia or Dia | Ours |
|---|---|
| Spaces merged into profiles: colour, pinned tabs, the switcher dots, swipe paging and ⌃1–⌃9 all act on profiles. Several Ardia profiles can share one engine profile to fake Spaces (P§5, D›Profile paging) | Decision 5: all of these act on Spaces |
| Pinned tabs and favorites belong to the profile (P§2) | Pinned tabs belong to the Space. Favorites: settled by the Arc capture |
| Arc import turns Arc Spaces into profiles (P§20) | Spaces stay Spaces |
| The default layout is Dia's (address bar on the card, optional top tab strip); Arc's sidebar address bar is a setting (P§4) | Arc's layout only (decision 6) |
| Dia's and Chrome's keys win clashes, so Arc's ⌘D pin moves to ⌥⇧⌘D (P§4) | Arc's keys win; owner to confirm |
| AppleScript `execute` runs JavaScript in pages, off by default behind View › Developer (P§23) | Not built: it gives any local process, agents included, what principle 3 forbids. AppleScript, if any, covers windows, tabs and URLs |
| Extension action state polled every 1.5 s, unused profiles checked every 15 s (P§15, P§5) | Event-driven only (principle 1) |
| The 1.49 New Tab area light ran a 30 fps display link for up to 60 s (D›Area light) | Not built; Dia 1.50.1 dropped it too |
| AI surfaces: "Ask anything…", the "+ Add tabs or files" chip, mic, Chat, ⌘E Chat, Tidy | Out of scope for v1 (decisions 6, 8) |
| "Peek" names the sidebar sliding in at the window edge | Peek is Arc's link preview; the edge slide is the sidebar reveal |
| Site Controls, the panel grid, Little Arc sizes and the 8 h update check are Ardia's choices | Placeholders until Arc is measured |

## What Arc still has to give us

Arc shuts down on 2026-12-22. This is what's left to capture, by feature; what's been measured is in the sections above. Where it says "screenshot", it means a 2× window capture of the named state. Where it says "record", it means a recording of the interaction.

**Setup**

- Arc 1.170.0 from a fresh install, so its defaults are its own, signed in to the owner's own account. The owner signs in; the agent never sees the password. Captures happen in a Space that holds only public test pages, so the owner's synced tabs never show. Settings shows the owner's name and email, so those captures stay out of the repo.
- A 1512 × 949 window on the MacBook's built-in display, so nothing covers the external monitor (AGENTS.md, trap 3). Capture dark and light, key and inactive.
- Capture Arc's window only, by pid, with `tools/window-screenshot.sh` (AGENTS.md, trap 2). Window-only captures never show the blur. For translucency, put a solid-colour window of our own behind Arc and composite the windows below Arc's frame (`CGWindowListCreateImage` with on-screen-below-window and including-window), cropped to Arc's frame (D›Window translucency).
- An agent drives Arc while the owner is away. It starts only after the owner has been idle for 60 s and stops at their next input. It presses Arc's buttons and menu items through Accessibility and posts keys to Arc's process only, so nothing it sends can reach another app (AGENTS.md, trap 1), and it moves the pointer only over Arc's window, for hover. Arc ignores clicks posted to its process, so a real click, a drag or a swipe needs the owner's hand; those items say so.
- Little Arc opens on the primary display, the owner's external monitor, whichever display Arc is on. Record it while the owner is at the desk.

**Static values** (measurable any time, machine load doesn't matter)

| Feature | Capture |
|---|---|
| Asset reads, no UI | Arc's colour tokens are read (`assetutil --info` on its `Assets.car` files); fold the ones we use into Colour tokens. Values only; never copy images, icons or fonts |
| Window | Translucency composites over black, grey and white backdrops, for each preset Space colour, key and inactive |
| Sidebar header | The nav glyphs' size; the URL field's font, and the field focused; with two pinned extensions; the zoom indicator, which is its own window, so capture that window too |
| Sidebar width | The minimum and maximum, by dragging to each limit (owner's hand) |
| Favorites | Screenshots with 1, 3, 4, 6, 9 and 12 favorites at the default width and at the minimum and maximum (columns, tile size, gaps, radius, favicon size). Selected colourful, one-colour and white icons; hovered tile with its tooltip; audio playing. Favorites sync to the owner's account, so remove them afterwards |
| Pinned tabs | A pinned tab navigated away (and hovered while away), and unloaded after ⌘W. Select it with ⌘1–⌘8: Accessibility presses don't select sidebar rows |
| Today tabs | Rows pressed, loading, sleeping, playing audio, with a long title; the scroll-edge line after scrolling |
| Sidebar hidden | The reveal panel's radius and border |
| Spaces | The switcher with 2, 5 and 10 Spaces, with icons and without; hovered and selected; rename, colour and icon flows; the profile picker; a Space whose profile differs. Answer: do favorites follow the Space or the profile? |
| Command bar | ⌥↩ on a typed host, from a closed bar (⌘T while the bar is open closes it). Answer: whether the pointer highlights a row (one capture showed none) |
| Split view | 3 and 4 panes, side by side and stacked; the focused-pane indicator; the unfocused dim; drop zones while a tab is dragged over the page (owner's hand) |
| Archive | Searching the archive, restoring a tab from it, the archive settings with every interval option |
| Profiles | Profile settings, creating a profile, an incognito window |
| Extensions | Where pinned extensions sit, an extension popup anchored, the install prompt for a throwaway Web Store extension, the remove confirmation, Arc's site-controls panel |
| Passwords | The save prompt after a fake login on a local page, the autofill dropdown, the password settings |
| Downloads | The indicator during and after a download, the Library's Downloads page with downloads in it |
| DevTools | Undocked |
| Import | Each step of Arc's import from Chrome. Answer: where do Chrome's bookmarks land? |
| Peek | Open from a favorite's link, and from every other click that opens one: size, position, radius, shadow, backdrop dim, buttons |
| Little Arc | Opened by a link from another app; the bar's colours; after ⌘L |
| Themes | Each editor state; the window at intensity 0, 0.5 and 1, noise 0 and 1 for each texture, translucency on and off, as composites |
| Boosts | Every way to start a Boost, each editor state, Zap picking an element, a site with a Boost in Site Controls, on and off. Answer: do Boosts follow the Space, the profile or the account? |
| Updates | The update prompt, if one can still be triggered |

**Timings** (need an unloaded machine: no Chromium build running, display awake, on mains power)

Record at 120 fps on a ProMotion display, 60 fps at the least. Use each frame's own timestamp, because recorders deliver frames unevenly. Repeat each interaction 3 times or more. Trace an edge per frame at 2× and fit a spring (response, damping ratio) or a curve and duration, reporting the rms error, as Ardia did.

| Feature | Record | Time |
|---|---|---|
| Sidebar | Closing a pinned tab | what the row does |
| Sidebar | Dragging a row past three neighbours, into favorites and back (owner's hand) | neighbour spring; drop settle |
| Sidebar | Hovering a favorite, then moving to the next | show delay, leave delay, tooltip motion |
| Sidebar | Folder expand and collapse by a click (owner's hand; Accessibility toggles it in one frame) | spring |
| Spaces | A slow two-finger swipe, a flick, an overscroll at the last Space (owner's hand); paging between two Spaces of different colours | tracking ratio, settle spring, rubber band, colour cross-fade |
| Split view | Dragging a tab onto the page until the targets show; a drop on each side; dragging the divider (owner's hand); closing a pane | target slide-in and grow; pane slide-in; close |
| Downloads | Starting a download | indicator fly-in and its duration |
| Peek | Opening, closing, expanding into a tab | each curve |
| Little Arc | A link from another app, then ⌘O, then a second link and ⌘W (owner at the desk) | window appear, the move to the main window, close |
| Extensions | Popup open and close | curve |
| Window | ⌘N; entering and leaving full screen with the sidebar docked | what the sidebar does, frame by frame |
