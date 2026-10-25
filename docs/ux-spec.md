# UX spec

What every surface looks like, how it moves and what it does, with the measured value and its source for each. Decision 6 sets the rule: Arc's structure plus Dia's polish. [ux-parity.md](ux-parity.md) is the checklist built from this file, and [ux-principles.md](ux-principles.md) says why Arc's values are what they are. The last section is the plan for measuring Arc before it shuts down on 2026-12-22.

## Attribution

Parts of this file and of `ux-parity.md` are adapted from Ardia (https://github.com/mantrakp04/ardia), from `docs/dia-spec.md` and `docs/dia-feature-parity.md`, with supporting values from `docs/omnibox-ux.md` and `apps/browser/src/lib/profileTheme.ts`. They are taken at tag `v0.2.40-nightly.5`, commit `fce3a02ffb917d2902af4ce47b398ce8b03a440d`. Copyright 2026 Mantra Patel. Licensed under the Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0). Changes from the originals: the material is reorganized by our features, Ardia's implementation is removed, and it is merged with our decisions and with Arc notes. The adapted parts keep their Apache-2.0 terms inside this GPL-3.0-or-later repository. Ardia's authors measured the values from Dia, a browser by The Browser Company. No Dia or Arc assets are included here.

## Reading this spec

**Precedence.**

1. Decision 5 overrides everything else. A Space holds tabs, pinned tabs and a colour. A profile holds cookies, history, logins and extensions. Dia and Ardia merge the two, so where a Dia value below says "profile", read it as "Space".
2. Structure comes from Arc: what exists, where it sits, layout sizes and behavior. That covers the sidebar, Spaces, favorites, pinned and Today tabs, archive, split view, Peek, Little Arc and Boosts.
3. Polish comes from Dia: materials, colours, the command bar, the New Tab page and motion. Interactions that only Arc has (sidebar reveal, Clear, Peek, Little Arc) keep Arc's motion.
4. Where an Arc value hasn't been measured yet, the Dia value is the target and is marked *interim*. The Arc capture plan at the end lists what will replace it.
5. Values marked **Ardia** are Ardia's own choices, not measurements of Dia or Arc. Treat them as placeholders.

**Sources.** `D›X` is section X of Ardia's `docs/dia-spec.md`. `P§n` is section n of `docs/dia-feature-parity.md`. `O` is `docs/omnibox-ux.md`. Dia versions are 1.49.1, 1.50.1, 1.52 and 1.53.0. *bin* means read from Dia's binary or asset catalogs, *cap* a 2× screen capture, *rec* a 60–240 fps recording. **Arc (Ardia)** marks Arc values Ardia recorded: Ardia's owner's Arc recordings and measurements of 2026-10-06 and 2026-10-22, a user's screenshot, and Arc's binary (Arc 1.167 for themes). **Chrome** means Chromium's own behavior. **Flecha** marks behavior read from Flecha's site and code at v0.1.0 (https://flecha.dev, MIT). **Helium** marks Helium's. **Ours** marks our decisions or the `prototypes/arcish` reference.

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

- Ours (*interim*):
  - Dark only. The light layers come with the Appearance menu.
  - A colour under HSL saturation 0.1 counts as neutral.
  - Where the window's look has to be opaque, it stands the material in with its output over grey 20 under the tint, #131313, and puts the gradient over that. That's the reveal's panel over the page, a dragged row, and the toast over the rows.
  - Over solid backdrops of black / grey 20 / grey 60 / white, our sidebar reads 11 / 19 / 35 / 51 before the gradient. That's the material at 18 / 32 / 58 / 85, against Dia's 24 / 32 / 45 / 104.

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
| Row motion | spring 0.13 s / ζ 0.9 (CSS ease ≈ 115–120 ms): within 0.75 pt of rest 83–92 ms in | rows making or closing room; a closed row fading, its opacity the slide's remaining fraction; a new tab's plate fading in; rows Undo brings back fading in | Arc 1.170.0 rec 120 fps: ⌘T then a pick, ⌘W, Undo after Clear |
| Row fill change | ease-out, 300 ms | tab row background changes. Ours: the hover fill fading once the pointer leaves a row; it fills at once, and selection moves at once (Tab rows › Behavior) | Dia 1.50.1 bin |
| Reorder | critically damped, ω ≈ 90 rad/s: 91 % in 43 ms, done by 76 ms | neighbours making way during a drag | Dia rec 60 fps, D›Reordering |
| Hover card in | opacity cubic-bezier(0.25, 0.46, 0.45, 0.94) 100 ms; scale 0.92 → 1 spring 0.25 s / ζ 0.75 | tab and favorite hover card | Dia bin, D›Tab hover card motion |
| Hover card out | opacity same curve 100 ms; scale → 0.92 spring 0.2 s / ζ 0.6 | same | same |
| Scroll edge | cubic-bezier(0.25, 0.46, 0.45, 0.94), 100 ms | sidebar separator fading in and out | Dia 1.53.0 bin |
| Panel grow | exponential ease-out, ≈ 130 ms | command bar out of the URL field | Dia 1.52 rec |
| Sidebar hide | ease-out cubic, 110 ms: within 0.75 pt of rest 67–83 ms in | ⌘S hiding a docked sidebar | Arc 1.170.0 rec 120 fps, 3 runs |
| Sidebar show | ease-out cubic, 100 ms | ⌘S showing the hidden sidebar | Arc 1.170.0 rec 120 fps, 3 runs |
| Sidebar reveal in | no fade: spring 0.175 s / ζ 0.80, ≈ 4 pt past rest, starting ≈ 100 ms after the pointer reaches the edge | sidebar reveal at the left edge | Arc 1.170.0 rec 120 fps, 3 runs |
| Sidebar reveal out | spring 0.175 s / ζ 0.76, starting as the pointer leaves the hold zone | same | Arc 1.170.0 rec 120 fps, 3 runs |
| Space settle | critically damped, 0.19 s (ω ≈ 33), from the moment the fingers lift | Space pages after a swipe, a switch's slide, the pull's spring back | Arc 1.170 rec at 2×: 6 swipes, ω 30–33 in the 4 tracked best, and 3 pulls springing back, 0.18 s. Dia 1.50.1 bin has 0.25 |
| Library push | spring 0.245 s / ζ 0.92 | the card pushed right and the Library sliding in with it, from its width to the left, as it opens, and back as it closes | Arc 1.170.0 rec 120 fps, 1 run; Arc 1.x video 60 fps, 2 opens and 2 closes |
| Library fade | out in 33 ms; in over 130 ms from 80 ms | the sidebar under the Library, where it is, opened or closed by a click | Arc 1.x video 60 fps, 2 opens and 2 closes |
| Library recede | opacity 1 − p/0.66; scale 1 − 0.076p about (sidebar edge + 128, window middle); the Library's edge from 0.29 at 0.92 its width a way | the sidebar under the Library, with p how far the card has gone (0 to 1), while fingers move it and on after they let go, both ways | Arc 1.170 rec at 2×, a slow swipe open and one closed |
| New Space pull | readying: white closes in over 60 ms, the dot shrinks into the + over 70 ms, pop to 1.13× and back over 120 ms from 40 ms; un-readying: the colour grows back over 120 ms; let go readied: the form at once, the circle back under the edge over 100 ms, ease-in-out | the circle past the last Space, and the Space it makes | Arc 1.170 rec at 1× and 2×, 3 pulls |
| Space form leave | each part over 150 ms, opacity 1 − t³ and sinking 20t² pt: the buttons from 0 without sinking (opacity 1 − t), the field from 80 ms, the title from 130 ms | the Create a Space form after Create, from the bottom up | Arc 1.170 rec at 2×, one Space made |
| Group expand | spring 0.30 s / ζ 0.82 | group or folder open and close (*interim*) | Dia rec |
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

| Action | Arc (recorded) | Dia | Ours |
|---|---|---|---|
| New tab / window / incognito window | ⌘T | ⌘T / ⌘N / ⇧⌘N | ⌘T / ⌘N / ⇧⌘N |
| Command bar on the current URL | measure | ⌘L | ⌘L |
| Close tab / window | measure | ⌘W / ⇧⌘W | ⌘W / ⇧⌘W |
| Reopen closed tab | measure | ⇧⌘T | ⇧⌘T |
| Clear Today tabs | ⇧⌘K | ⇧⌘K (Close All Tabs) | ⇧⌘K |
| Pin / unpin tab | ⌘D | — (⌘D bookmarks) | ⌘D, *owner to confirm* |
| Back to pinned URL | measure | ⌘↩ | ⌘↩ |
| Toggle sidebar | ⌘S | ⌘S (Auto-Hide Tabs) | ⌘S |
| Library | ⇧⌘L (menus) | — | ⇧⌘L, which closes it too |
| Little Arc / open it in the main window | ⌥⌘N / ⌘O | — | ⌥⌘N / ⌘O (⌘O stays Open File in main windows) |
| Switch Space 1–9 | measure | ⌃1–⌃9 (profiles) | ⌃1–⌃9 |
| Next / previous tab | measure | ⇧⌘] / ⇧⌘[, ⌥⌘→ / ⌥⌘←, ⌥⌘↓ / ⌥⌘↑ | Dia's, *interim* |
| Select tab 1–8 / last | measure | ⌘1–⌘8 / ⌘9 in sidebar order | Dia's |
| Tab used last, then further back | ⌃Tab (rec) | ⌃Tab / ⌃⇧Tab (switcher) | ⌃Tab / ⌃⇧Tab, no switcher |
| Search tabs | measure | ⇧⌘A | ⇧⌘A |
| Split: add pane / close pane / focus next / previous | ⌃+ / ⌃⇧- (menus) | ⌃⇧= / — / ⌃⇧] / ⌃⇧[ | Arc's; focus Dia's, *interim* |
| Find / next / previous | measure | ⌘F / ⌘G / ⇧⌘G | ⌘F / ⌘G / ⇧⌘G |
| Use selection for find / jump to selection / find and replace | — | — (⌘E is Chat) / ⌘J / ⌥⌘F | ⌘E / ⌘J / ⌥⌘F |
| Copy URL / as Markdown / as Quote | ⇧⌘C / ⌥⇧⌘C / ⌃⇧⌘C (menus) | ⇧⌘C / ⌥⇧⌘C | ⇧⌘C / ⌥⇧⌘C |
| Downloads | measure | ⇧⌘J | ⇧⌘J |
| Save page / print | ⇧⌘S / ⌘P (menus) | ⇧⌘S / ⌘P | same |
| DevTools / console / inspect / view source | — | ⌥⌘I or F12 / ⌥⌘J / ⌥⌘C / ⌥⌘U | same |
| Back / forward | measure | ⌘[ / ⌘], ⌘← / ⌘→ | same |
| Reload / force reload / stop | measure | ⌘R / ⇧⌘R / — | ⌘R / ⇧⌘R / ⌘. and Esc |
| Zoom actual / in / out | — | ⌘0 / ⌘+ / ⌘- | same, plus hidden ⌘= |
| History / clear browsing data | — | ⌘Y / ⇧⌘⌫ | same |
| Full screen | — | Globe-F | Globe-F, plus hidden ⌃⌘F |
| Settings | ⌘, | ⌘, | ⌘, |

Mouse (Dia and Chrome, P§ Keyboard shortcuts):

- ⌘-click opens a background tab. ⇧⌘-click opens a foreground tab. ⇧-click opens a new window. Middle-click opens a background tab.
- ⌥-click downloads. ⌥⇧-click opens a split pane.
- ⌘-click on back, forward or reload opens the result in a background tab.
- Double-click a tab to rename it.

### Menus

Arc 1.170.0's menu bar, with the items the browser has so far. An item runs the same command as its key and shows that key.

- **Order.** App, File, Edit, View, Spaces, Tabs, Archive, Window, as Arc's App, File, Edit, View, Spaces, Tabs, Archive, Extensions, Window and Help. Extensions and Help come with their features.
- **File.** New Tab…, Restore Last Closed Tab, Open Command Bar, Archive Tab (⌘W), then Save Page As (⇧⌘S) and Print (⌘P), as Arc's. Save Page As shows Chrome's save panel and Print its print preview, over the page. Arc's window, profile, Note, Easel, Share and Capture items come with their features.
- **Archive.** Go Back, Go Forward, View Archive and Clear Archive, as Arc's. Arc's View History (⌘Y) comes with history.
- **Edit.** Undo, Redo, Cut, Copy, Paste and Select All go to the first responder, so a focused page or text field does them itself. Blink on Mac leaves those keys to the menu, so without them a page can't undo, copy or paste. Copy URL and Copy URL as Markdown sit between Copy and Paste, as in Arc, and Find is a submenu.
- **App.** Hide (⌘H), Hide Others (⌥⌘H), Show All and Quit (⌘Q).
- **View.** Show/Hide Sidebar, then Stop Loading, Refresh the Page, Force Refresh the Page, Clear Cookies and Refresh and Clear Cache and Refresh, then the split view items, zoom and Developer, as Arc's. The two Clear items, also in the command bar, delete the profile's cookies or cache for the page's site, its registrable domain or IP address, then load the page again, the cache's bypassing the cache. Pages that aren't http(s) have no site, so the command bar leaves the items out and they do nothing (*interim*: what Arc clears beyond the site isn't measured).
- **Window.** Minimize Window (⌘M), then View Library (⇧⌘L) and View Downloads (⇧⌘J). AppKit adds its window items (Fill, Center, Move & Resize, Full Screen Tile) above them, and Enter Full Screen to View.
- **Page menu** (*interim* until Arc's is captured). A right-click on the page shows Chrome's context menu, as Arc's does, with Chrome's items working as in Chrome: a link opens in a tab under the page, Save As and Save Link As show the save panel, Print shows print preview and Inspect docks DevTools at the element. It leaves out the items for what the browser doesn't have yet, Open Link in New Window, in Split View and in Incognito Window, and those whose UI is a bubble on Chrome's toolbar or its side panel: Cast, Open in Reading Mode, Listen to This Page, QR Code and Translate. Each comes back with its feature.
- **Tab dialogs**, such as print preview, Chrome's, as in Chrome's window: centred over the page, with their top at the page's, and gone while another tab shows.
- **Names.** Arc's, where they're plain words: Restore Last Closed Tab, Archive Tab (⌘W), Refresh the Page, Force Refresh the Page, Go Back, Go Forward, Zoom to Actual Size, Zoom In, Zoom Out, and View › Developer's View Source, Developer Tools, Inspect Elements and JavaScript Console.

## Phase 3 features

### Window

| Item | Value | Source |
|---|---|---|
| Card inset, sidebar docked or hidden | 10 on every side. Measure whether the docked left inset is counted inside the sidebar width | Arc (Ardia), D›Arc note: hiding |
| Card inset (reference) | left = sidebar width, top 6, right 7, bottom 7. With the sidebar hidden, 6 / 6 / 6 | Dia 1.49 cap |
| Card corner radius | 6, circular; the page in the card is clipped to it. Dia's is 10, continuous | Arc cap, Dia cap |
| Traffic lights | Arc: centre y 23.75, x measure. Dia: centres x 24.75 / 47.75 / 70.75, y 26.75 (a 23 pt pitch) | Arc (Ardia) P§4, Dia 1.50.1 bin |
| A page before its first paint | the card colour, composited opaque over the tint, never white | Ardia, P§7 |

- **Dragging the window.** The empty sidebar below the rows, the header row outside its buttons, and the margins around the card all move the window (the left margin only while the sidebar is hidden). A double-click there does the system's title-bar action from System Settings › Desktop & Dock. The exception is a double-click on the empty sidebar, which opens a new tab (Dia 1.50; Arc: confirm). Rows, tiles, buttons and the URL field keep their own presses. The outer ~3 pt is the resize edge (Dia and Arc, P§1).
- **New window.** It appears at its final frame, already showing its content, with no empty frames. A closed window leaves the screen at once (Dia, P§1).
- **Close confirmations** (Dia, P§1). Closing a window with 2 or more tabs, or pressing ⌘W on the last tab, asks "Close 3 tabs?" with a breakdown per Space and "Don't ask me again". Arc's equivalent: measure.
- **Warn before quitting** (Arc 1.170.0 bin and cap). The Quit menu item and ⌘Q ask "Quit Arc?", with the app's icon and the buttons Quit, "Quit, and don't ask again" and Cancel, unless Settings › General › Warn before quitting is off. The second button turns the setting off. A quit from the Dock or at logout doesn't ask. Ours: "Quit Browser?" in a system alert, on by default as in Arc's capture. With downloads running, Chrome's alert asks after Quit, as Arc's quit asks again for them.
- **Page full screen** (Dia, P§18). The page fills the window and the sidebar goes away. Esc or the page's `exitFullscreen` brings the layout back. Switching or closing the tab ends full screen. The window enters macOS full screen for the page and leaves it only if it entered for that page. No frame of the transition may show the other layout. Arc's behavior in window full screen (user-initiated): measure.
- **Other window commands** (Dia, P§1). Keep Window on Top, Merge All Windows, Move Tab to Window. Moving keeps the live page: no reload, history kept.

### Sidebar

Arc's layout: the URL field and back, forward and reload live in the sidebar, and the card has no toolbar (decision 6). Dia's card toolbar, its top tab strip and the toolbar that hides on scroll are not ours.

**Header and URL field**

| Item | Value | Source |
|---|---|---|
| Row centre (traffic lights, sidebar button, back, forward, reload) | 23.75 below the window top | Arc (Ardia), P§4 |
| Button hover plate | 32 × 32, radius 6, top edge 8 below the window top. Only the plate takes clicks; the rest of the row moves the window | Arc (Ardia), P§4 |
| Back / forward / reload | right-aligned. Ardia's centres at 227.5 pt are 137.75 / 171.75 / 205.75, within 2 pt of Arc's. Measure Arc's exactly | Ardia, P§4 |
| Narrow sidebar | reload goes first, then forward; back and forward stay together | Ardia, from Arc users |
| Forward when disabled | about half the brightness of the other glyphs | Dia 1.52 cap |
| Nav glyph size | Dia 13 pt regular. Arc: measure | Dia 1.52 cap |
| URL field top | 46 below the window top | Arc (Ardia), P§4 |
| URL field height, radius, fill, font | measure | — |
| URL field text | the host alone, `www.` dropped. View › Show Full URL (off by default) adds the path, query and fragment, with no trailing "/" | Dia 1.50.1 bin, D›URL bar |
| Hovering the URL | Arc: the field's buttons, not the full URL (Arc 1.170.0 cap). Dia: the full URL, host bright and path dim (Dia 1.49 cap). Ours: Arc's | Arc 1.170.0 cap |
| Field buttons | Copy Current URL and Site Control Center, 30 × 30 at x 163 and 191 of 228, while the pointer is over the sidebar | Arc 1.170.0 cap |
| Insecure page / zoom not 100 % | a lock-warning glyph / a magnifier (+ or −) that opens − / + / Reset. Arc: no warning glyph, and nothing for zoom but the HUD (› Zoom) | Dia and Chrome, P§8; Arc 1.170.0 cap |
| Downloads button (reference) | Dia: centre (166, 26) at 190 pt, in the header. Arc: measure | Dia, D›Sidebar |

- Ours (*interim* until Site Control Center is built): Copy URL takes Site Control Center's place, 3 pt inside the field's end rather than Arc's 1 pt past it, with Lucide's link glyph at 14 pt, the header's glyph colour and hover plate. The host truncates before it.
- **Copy URL** (Arc 1.170.0 menus). ⇧⌘C copies the tab's URL, and ⌥⇧⌘C a Markdown link, `[title](url)`. Arc also has Copy URL as Quote (⌃⇧⌘C). Ours: the title's brackets are escaped, the URL is copied as it is (Dia strips trackers), and the toast says "Copied URL" or "Copied URL as Markdown" for 2 s, since Arc's feedback isn't recorded.

**Width**

| Item | Value | Source |
|---|---|---|
| Default | Dia 190. Arc: measure on a fresh install. The 227.5 in Ardia's Arc recording may have been resized | Dia 1.50.1 bin |
| Minimum / maximum | measure. Ardia uses 160 / 400. Dragging past a limit rubber-bands | Dia, P§4 |
| Persistence | kept per window; double-click on the handle resets it | Dia, P§4 |

- Ours (*interim* until Arc's are measured): 228 at first, 160 to 400. The handle is the 6 pt margin right of the rows, with the left-right resize cursor, or the left or right one at a limit. Past a limit the width rubber-bands on the Space pages' curve and, let go, settles at the limit on their spring, as does a double-click's reset to 228. The core saves the width with the Spaces. A header button shows while its plate ends at least 5 pt inside the sidebar, so reload goes under 227 and forward under 193.

**Sections, top to bottom.** Header, URL field, favorites, then the Space's pinned tabs, the Clear divider, the Today tabs and "+ New Tab". The footer holds the Space switcher (Arc structure). Measure in Arc where the Space name sits and what the footer holds. The favorites stay put while the list scrolls. A favorites block taller than 40 % of the sidebar scrolls with the list instead (Ardia).

**Favorites** (Arc's favorites are Dia's Top Apps dock, so these values are *interim*)

| Item | Value | Source |
|---|---|---|
| Tile height / gap | 41 / 6 | Dia 1.50.1 bin and cap (1.49: 40) |
| Side inset | 8, the rows' plates' and the URL field's. Dia's is 6 | ours |
| First row top | 90 below the window top, 9 under the URL field. Dia's is 54 under its header, and Arc's favorites area starts at 82 (ax): re-measure with favorites in Arc | ours |
| Columns | as many as fit at least 48 wide, all as wide; fewer favorites share the row, and the last row starts on the left. That gives Dia's 3 across at 190 pt (x 6, 67, 128) and 4 at 228. Arc's rule: measure | Dia, P§2; the rule ours |
| Radius | 10 | Dia bin |
| Resting | fill white 0.10, 0.5 pt stroke white 0.14. Under the pointer white 0.15, fading out on the rows' hover curve (ours) | Dia 1.50.1 bin |
| Favicon | centred, as a row's: 16 in an 18 box, at half opacity while unloaded. Size: measure | Dia cap |
| Badge for a tile away from its URL | none | Dia 1.53.0 bin |
| Hover | hover card with title and URL (see Motion) | Dia, D›Tab hover card motion |
| Double-click | goes back to the favorite's URL, like ⌘↩. A tile already there doesn't reload | Dia, P§2 |

The selected tile is themed by its icon (Dia 1.50.1 bin, D›Sidebar). Draw the icon at no more than 32 px on its long side. Average its pixels with alpha ≥ 0.975, then take their mean RGB distance from that average:

- **Distance ≥ 0.055 (colourful).** The tile is white 0.20 (dark) or white (light). Over it goes the icon at 22 % opacity, blurred with radius 5 and aspect-filled to 2.5× the tile. The ring is a 3 pt border showing the same blurred icon over black (dark: at 75 %, saturation 2, brightness −0.1) or white (light: at 100 %).
- **Distance < 0.055 (one colour).** The tile is filled with the mean colour, the icon is drawn as a white template, and the ring is white with soft-light compositing. A near-white icon (relative luminance > 0.88) gets a black fill and a white 0.30 ring instead.
- **No usable icon.** Fill black (dark) or white (light), with the default selection ring.
- A secondary selection (multi-select) gets the secondary fill and no ring. Emoji icons draw at 16 pt and are themed the same way.
- Until tiles are themed, the tile shown is white 0.20 with the default ring, white 0.45, 0.5 pt (ours).

What a favorite does (ours, *interim* until Arc's are captured):

- A favorite is a tab of the profile's, in no Space, so every Space of the profile shows the same tiles (Arc's favorites follow the profile; confirm in the capture). With one profile for now, they're the browser's. The tiles stay put while the Spaces' pages swipe under them.
- Add to Favorites in a tab's menu makes the tab the last favorite, at its pinned URL if it's pinned, else at its page, and takes it out of its split. Remove from Favorites in the tile's menu puts it at the top of Today in the Space shown.
- Dragging moves a tab between the tiles and the rows. Above the bottom of the tiles it would have with the dragged tab among them, a dragged row is drawn as a tile centred on the pointer, and the tiles make room for it at the place nearest its centre. A dragged tile below that is drawn as a row centred on the pointer, and the rows make room as for a dragged row. A split view's row stays in the list. Dropped on the tiles, a row becomes a favorite there, and dropped in the list a tile goes there, pinned at its favorite's URL among the pinned tabs. Between the tiles, a tile reorders.
- The tiles and the list's top move on the Reorder spring while they make room, and on Row motion when they settle, as the favorites change and a dropped tab lands. A new tile is in its place at once. Nothing moves while the Space shown is being filled, as at launch.
- A click shows it. ⌘1–⌘9, next and previous tab and ⌃Tab count the favorites first. A Space remembers the favorite it showed, across a quit too.
- The tile's menu: Back to Favorite URL (⌘↩, away from it), Duplicate Tab (a copy at the top of Today, shown), Remove from Favorites, then Close Tab (⌘W). Closing unloads it and keeps the tile, as for a pinned tab, and shows the Space's top Today tab, else its lowest pinned tab still loaded. No Rename or Move to Space yet.
- Links a favorite opens go at the top of Today in the Space shown.
- Later: the hover card, theming, and Arc's measured drag: how the dragged row turns into a tile and back, which we do at once.

**Pinned tabs** (Arc: per Space. Dia 1.52 has the same section.)

- They are rows that look like tab rows, sitting between the favorites and the Clear divider, 6 pt under the tiles (Ardia).
- Closing one (⌘W or ✕) unloads its page and keeps the row. With no Today tab left, ⌘W on a pinned tab shows another pinned tab whose page is still loaded. Closing the last tab closes the window (Dia rec, P§2).
- Away from its pinned URL, the favicon turns into a back arrow under the pointer and the title reads "Back to Pinned URL". A click goes back, and so does ⌘↩ (Dia 1.53.0 bin). Arc's look for a navigated-away pinned tab: measure.
- A pinned tab has one live page. Selecting it in a second window moves the page there, with history and playing media intact. The first window keeps a frozen frame under a wash: white 0.40 in light mode (Dia cap), black 0.25 in dark mode (Ardia). Clicking the frozen frame pulls the page back (Dia, P§2).
- Links ⌘-clicked in a pinned tab gather in a group just below the pinned tabs (Dia rec, P§2). Measure what Arc does: Peek covers some of these links.
- ⌘D pins or unpins. Dragging a tab above the divider pins it, below unpins it, and onto the favorites makes it a favorite.

**Folders** (Arc: in the pinned tabs, per Space. Arc 1.170.0 ax and cap, sessions 10–14)

| Item | Value | Source |
|---|---|---|
| Indent | 16 per folder: in one, the favicon is at x 33, the title at x 59 and the plate from 24 | Arc 1.170.0 ax, cap |
| Row | a tab row's, with the folder's icon in the favicon's place, open or closed, and its name as the title | Arc 1.170.0 cap |
| Closing and opening | Arc's accessibility call moves the rows below in one frame; a click: measure | Arc 1.170.0 rec |
| Hover | the row plate, and a tooltip with the name after a moment | Arc 1.170.0 cap |
| ⌘1–⌘9 | count the tabs in an open folder | Arc 1.170.0 |
| New Folder… | in the Tabs menu, under Open Little Arc or Peek | Arc 1.170.0 menus |

- Ours (*interim* until Arc's folders are captured):
  - The icon is Lucide's folder or folder-open at the favicon's size, in the secondary colour.
  - New Folder… makes an open folder named "New Folder" at the bottom of the pinned tabs, with its name in a field, all selected. ↩ keeps what's typed, and Esc or nothing typed keeps "New Folder".
  - A click opens or closes it. The rows under it move on Group expand, and the rows it shows fade in together, as rows Undo brings back do.
  - A double-click renames it, after its first click has opened or closed it, and so does Rename Folder in its menu. The menu also has Delete Folder, which leaves what was in it where it was, in the folder that held it.
  - Closed, it hides its rows but the tab shown's, which stays under it until another tab shows (Arc; confirm in the capture). ⌘1–⌘9 skip what a closed folder hides, so each row keeps its number.
  - Dragging. Where a dragged row would land at the end of folders or after one, the pointer's travel right of where it pressed takes it in, a level for each 16 pt, between the levels that place allows: from the row below's to one inside the row above, if that's a folder. A folder drags as a row, among the pinned tabs only, never into itself. What's in it stays while it's dragged and moves with it when it lands.
  - Folders come back after a quit with their names and whether they're open.
  - Collapse All Folders in the command bar closes every folder of the Space shown, on one fold.
  - Later: the tooltip, making a folder inside one, and Arc's folder drag, captured.

**Clear divider and Today tabs** (Arc bin and a user's screenshot, D›Arc note: Clear)

- Today tabs are the Space's unpinned tabs. They are archived (see Archive).
- The divider shows with the section while the Space has Today tabs. It has no hover state.
- Clear shows only while the pointer is over the sidebar (Arc 1.170.0 cap). Ours: it shows and hides at once, since Arc's fade isn't recorded. The reveal's panel, which shows only under the pointer, always shows it.
- The line is one device pixel of white 0.15 (Arc's separator token; the capture reads 0.14–0.16), 18 pt in from each side of the sidebar (Arc 1.170.0 cap). While Clear shows, it ends 7.5 pt before the arrow. Its centre is 25 pt above the centre of the first Today row.
- The down arrow is 7 × 8 and sits 3 pt before the label "Clear". The label is semibold, with a cap height 0.75 of the tab titles' (≈ 10 pt beside 13 pt titles), and it ends where the titles end.
- The button's tooltip is "Clear unpinned tabs in this space". The menu item is Clear Unpinned Tabs (⇧⌘K). The command bar finds it by "close unpinned tabs", "clear tabs" and "clear today".

**Tab rows** (Arc 1.170.0 ax, cap and colour tokens; Dia 1.50.1 bin and cap where noted)

| Item | Value |
|---|---|
| Pitch / row height | 41 / 40 |
| Plate | 36 tall, centred in the row, from x 8 to the width − 8 (from 24 in a folder), radius ≈ 12 |
| Favicon | 18 × 18 at x 17, vertically centred |
| Title | at x 43, 14 pt. Fades out at the trailing edge, never an ellipsis (Dia) |
| Selected | white 0.20 (Arc's p3 (0.98, 0.984, 1)), with no line or shadow in dark mode. Light mode: white 0.85 with a black 0.2 shadow (tokens). White title (Dia) |
| Hover | white 0.12, and a close button 26 × 26, 5 inside the plate's right edge, that the title truncates before. The selected row shows it too |
| Pressed | white 0.065 |
| Audio | a speaker glyph after the favicon. Muting applies to the site in that profile and follows navigation (Dia, P§2) |
| Loading | a 12 × 12 ring at width − 8 − 12, only in rows at least 48 wide, hidden under the hover ✕. The track and ring are 1.5 pt lines on a circle inset 1.25. The ring covers 72 % with round caps in the secondary label colour, the track is the same colour at 0.18, and it spins per Motion |
| Sleeping tab | faded favicon (alpha: measure) |
| "+ New Tab" row | the first row under the divider, where new tabs open below it (Arc). Label white 0.55 (secondary), plus sign white 0.25 (tertiary); hover as rows (Dia) |
| New Tab tab's icon | our mark, 16 wide, white 0.28 in dark mode over any row state |

- Ours: titles are Dia's, white for the selected row and 0.78 for the others, since Arc's aren't measured. The ✕'s square is the subtle fill, white 0.12, at the header buttons' radius 6. A selected or dragged row shows no pressed fill. Titles end in an ellipsis, since GPUI has no mask for the fade.

**Behavior**

- **⌘T's command bar** (Arc rec 120 fps). While it's open, "+ New Tab" has the selected plate, and the tab shown loses it and its selected title, in one frame. A pick, Esc or a click outside the bar gives it back, in one frame. ⌘L's bar doesn't move the plate.
- **Opening a tab** (Arc rec 120 fps). The new row appears in its slot at once, already selected, and its plate fades in (Row motion). The old selection drops at once. Rows below slide down one pitch (Row motion), drawn under the new row and revealed from beneath its bottom edge (Dia).
- **Closing a tab** (Arc rec 120 fps). Selection moves to the neighbour at once. The closed row drops its selected fill and stays in place, fading as the rows below slide up over it (Row motion), all starting at the same moment. Hover follows the final layout at once. Several closes fade together.
- **Tabs coming back.** When several rows come at once, as after Undo, they fade in in place (Arc rec) and the rows below them slide.
- **Hover card** for rows and tiles (Dia bin). It is a child window that scales from its top-left corner. In and out per Motion; both start from the current values, so a leaving card comes back from where it is.
  - It shows after 0.5 s, or 0.1 s from one hover source that wasn't identified. It shows at once when another card is already up.
  - Moving from row to row moves the card without animation.
  - A click or key press dismisses it. The leave delay is unknown (Ardia uses 160 ms). It takes no clicks until it has faded in.
  - Actions (Dia): Pin, Bookmark, Split, Picture in Picture, and Reset for a pinned tab.
- **Scroll edge** (Dia 1.53.0 bin). A 1 pt full-width separator sits 4 under the favorites. The list starts 5 under them with a 1 pt top inset. The separator fades in (Scroll edge curve) once the list scrolls past its top and fades out at the top or while rubber-banding.
- **Reorder** (Dia rec). The dragged item follows the pointer within a frame, stays opaque and draws over its neighbours. A neighbour moves aside once the item's leading edge passes the neighbour's centre (Reorder spring). On drop nothing else moves.
  - Ours: a row comes away once the pointer has moved 4 pt, and a drag never shows its tab. The row follows the pointer up and down only, within the list's first and last places. Above the divider it pins the tab and below it unpins it; a tab pinned this way is pinned at its page. Letting go anywhere drops it at its place; escape, or letting go where it started, puts it back. The dragged row shows its hover fill, made opaque, and the other rows none. The divider goes while the place would leave Today empty.
- **Dragging out of the list.** A tab dragged onto another window's list lights that list up and moves the live page there. Dropped on the desktop, it opens a new window under the pointer. Esc cancels a sidebar drag.
- **Dropping onto the tab list.** Files and http(s) links dropped on the list open as tabs at the drop gap (Dia). `javascript:`, `data:` and `file:` links from dragged text never open (Ardia).
- **Opening rules.** Links opened behind line up after their opener in the order opened. A link opened in front goes right after its opener. Closing a page goes to a tab it opened, then a sibling, then the opener (Chrome). Tabs opened behind load only when first shown; until then the row shows the address and the site's icon (Dia). The list scrolls to show a tab opened behind (Dia).
- **Other row actions** (Dia). Double-click a row to rename it inline. Change its icon to an emoji. ⌘-click and ⇧-click select several rows for bulk actions. The context menu shows shortcut hints. A haptic tick sounds while reordering, as a setting.
  - Ours (*interim* until Arc's menu is captured): a row's right-click menu has Pin Tab or Unpin Tab (⌘D), Add to Favorites, Back to Pinned URL (⌘↩, a pinned tab away from its URL), Rename Tab, Duplicate Tab, Move to Space with the other Spaces in the switcher's order, then Close Tab (⌘W) and, for a Today tab, Close Tabs Below. The row keeps its hover fill while the menu is up, and a menu from the reveal's panel opens at the pointer there. Duplicate Tab copies the tab with its history under it, or at the top of Today for a pinned tab, and shows it. Rename Tab, or a double-click on the row, turns the title into a field with all of it selected, as for a Space's name: ↩ keeps what's typed, Esc cancels and a click elsewhere keeps it. The name shows in place of the page's title wherever the tab's title does, the archive too, and stays through a quit, ⇧⌘T and Duplicate Tab. Kept empty, the tab shows its page's title again, and left as it was, nothing changes. Close Tabs Below shows the right-clicked tab first if one below was shown, so none loads on the way out, and ⇧⌘T reopens them one at a time. Move to Space keeps a pinned tab pinned, at the bottom of the other Space's pinned tabs, and puts a Today tab at the top of its Today. A window showing the tab goes with it, as Chrome's active tab goes to the window it's dragged to, and the old Space shows next what it would once the tab closed.
- **Hiding and showing** (Arc layout, Arc 1.170.0 rec).
  - ⌘S or the sidebar button hides the docked sidebar (Sidebar hide). The sidebar and the card's left edge leave together, and the page is laid out at every step. Hidden, the card fills the window with no traffic lights, header or nav.
  - ⌘S shows it again (Sidebar show). The window's buttons come back once it's at rest, so they never sit over the moving card.
  - The sidebar button sits a pitch before back, its plate at x 88 (*interim*: Arc's position isn't measured, and its glyph is ours). The View menu has Toggle Sidebar ⌘S.
  - ⌘S isn't reserved: a page that keeps it, as an editor's save does, keeps it (Chrome's rule for Save Page).
- **Sidebar reveal** (Arc (Ardia), Arc 1.170.0 rec).
  - The pointer kept at the window's left edge, at any height including the top-left corner, slides a floating panel over the page (Sidebar reveal in). The page doesn't move. A pointer passing the edge, on its way to another window or display, brings nothing out.
  - The panel is inset 5 from the window's left, top and bottom edges and as wide as the docked sidebar. It's dark, at radius 12, with a 0.5 pt light border and no shadow; in a capture at 1× the border is one pixel of white 0.27 over the panel (Arc 1.170.0 cap). It holds the traffic lights, the header buttons, the URL field and the tabs.
  - It stays while the pointer is within 24 pt of its edge (Arc ≈ 21), then slides away (Sidebar reveal out).
  - Docking from the panel is instant: the page is laid out narrower in one frame.
  - Ours (*interim* until the Arc capture): the trigger is the card's left margin, x < 10. The border is white 0.54, which gives that pixel at 1×; a 1 pt border of 0.27 would too, and only a capture at 2× tells them apart. The panel draws its own traffic lights, since AppKit's stay in the browser window under it.
  - Ours: the panel never takes the keyboard, so renaming a Space or making one from it docks the sidebar first. The command bar opening, page full screen and the app going to the background send it away.

**⌃Tab** (Arc 1.170.0 rec, sessions 10–12; Dia 1.50.1 bin, P§2)

- Arc: ⌃Tab shows the tab used last at once, in one frame, with no switcher, whether ⌃ goes up at once or is held. With no tab shown, it shows the tab used last.
- Dia: a switcher of recent tabs, most recent first, that appears after 140 ms (Ardia; source unclear) and switches when ⌃ goes up. → and ← move. Esc or a click outside cancels. Clicking a row switches to it, and the pointer highlights rows. Leaving the app switches. Other keys are swallowed while it's up.
- Ours: Arc's, among the tabs of the Space shown. While ⌃ stays down, each ⌃Tab goes one tab further back in the order they were shown, and ⌃⇧Tab one forward, so holding ⌃ reaches past the last two. ⌃ going up, a key or a click without it, or the app going to the background ends the walk, and only the tab it ends on counts as shown. Tabs not shown since launch come last, in sidebar order. Arc's walk with ⌃ held isn't recorded.

### Spaces

**Model** (decision 5)

- A Space has a name, a colour, its pinned tabs, its Today tabs and a profile.
- Several Spaces can share a profile. Switching to a Space of another profile switches the window's cookies, logins and extensions with it.
- Moving a tab to a Space of another profile reloads it in that profile, after a one-time warning that the tab loses its session (Dia warns when moving a tab to another profile, P§1).
- Favorites belong to the profile: every Space of a profile shows the same ones (Arc, to confirm in the capture).

**Switcher** (Dia 1.50.1 bin, D›Profile paging. Dia's ARCUI and TabSidebar modules look inherited from Arc, so these values are probably Arc's too: confirm.)

| Item | Value |
|---|---|
| Footer | 41 tall. Library cell at the left; overflow button 42 wide, 3 from the right |
| Switcher band | 28 tall, centred in the footer above its bottom 3 pt, 31 in from each side when those buttons show |
| Items | 12 pt per item, centred. At most 7 (84 pt) before it scrolls, with edge items scaled 0.75–1 |
| Dot | 6 × edge scale, round. Label colour at 0.85 selected, 0.4 hovered, 0.2 otherwise |
| Click | focuses that Space |

Arc shows Space icons in place of dots when a Space has one: measure both looks.

**Switching**

- Click a dot, press ⌃1–⌃9, or swipe horizontally with two fingers over the sidebar.
- Paging is Dia's (bin):
  - Tracking moves the pages at half the scroll distance (−0.5 × the horizontal delta), whatever the system's "Swipe between pages" setting says. Magic Mouse one-finger swipes count too. Ardia tolerates 12 pt of vertical wobble, then locks to horizontal.
  - Past the first or last Space, the pages rubber-band: `255·(1 − 1/(0.15·x/255 + 1))`.
- Past the first Space is the Library (Arc's release notes, June 2022: "Swipe all the way left to get to Library"). A 2023 video (Arc 1.x, 25 fps) shows a wheel's scroll there playing the Library's open, with the Space's rows gone within a frame.
  - Fingers going right on the first Space move the Library as its open moves it, the Library and the card at the fingers' way over the Library's 417 (Arc 1.170 rec at 2×, a slow swipe open and one closed). The sidebar recedes under it with them: its rows, fields, footer and the window's buttons fade out by 0.66 of the way and shrink toward a point 128 right of its edge at the window's middle, so the top goes down and right and the bottom up and right (Library recede). The Library comes late, from 0.29 of the way at twice the card's speed, and covers the sidebar up to its edge, except the window's buttons, which recede over it. Arc's swipe was recorded in its Spaces section, whose Library stops at 264 with the card at 420 and then brings its column in. Ours is one panel with its column, so once the sidebar is gone it catches the card up as it stops. They follow the card after the fingers let go too, and back the same way. Ours: the detent sounds at halfway. Let go, the Library opens on a flick toward it or past halfway, else goes back, on its spring from the fingers' speed. In the Library, fingers going left bring back the Space shown the same way, and going right rubber-band. A wheel's tick opens it from the first Space and closes it from the Library. Fingers catch the Library while it moves. With the sidebar hidden, the reveal's panel rubber-bands as before.
- Past the last Space, a pull makes a new Space (Arc 1.170, recorded at 1× and 2×, three pulls; sizes in pt).
  - The Space's title and tabs move with the rubber band; the URL field and the favorites stay. A circle 30 across comes out from under the page's left edge, halfway down. Its centre starts 20 right of the edge and moves 3.1× as fast as the page, stopping 27 left of the edge once the page is 15 out; the page goes on.
  - It's a disc of the Space's colour with a white + (13 across, 1.5 thick), a soft shadow, and a white ring 2.5 thick on its edge, with a 0.5 line of the sidebar's colour inside it. The ring grows anticlockwise from 12 o'clock with round ends, `out / 22` of the way, and the white is tinted with 9% of the colour.
  - A white glow comes out with the circle: a cone centred on the edge halfway down, 80 wide and half the window tall, fading linearly from 0.18. It slides out from under the edge as the circle does.
  - At 22.7 out (the fingers 333 across) the circle readies a new Space, and the trackpad clicks. The + goes, the white closes in on the colour to a dot 13 across over 60 ms, and the dot shrinks into a + of the colour over 70 ms more, as the circle pops to 1.13× and back from 40 ms over 120 ms. Pulled back under 22.7, it un-readies at once and clicks again: the colour grows from the middle over 120 ms, the white + on it, and the white thins back to the ring. No other moment clicks.
  - Let go readied, the Space is made, "Space N", with no icon: the Create a Space form and the new colour show within a frame, with no slide, and the circle and its glow go back under the edge over 100 ms, ease-in-out. Let go before, the page springs back, critically damped with a 0.18 s response, and the circle goes with it.
  - Ours: the circle's colour is the Space's, or the system accent's for none, at a lightness of 0.55 at most so the white reads. The glow is drawn in 4 pt strips across, as GPUI has no radial gradient, and the ring's round ends are dots.
  - Release velocity comes from the last 100 ms of samples. At |velocity| ≥ 5 pt/s the target is the next Space in that direction, otherwise the nearest. A swipe moves at most one Space from where it began. The settle carries the release velocity when it points at the target.
  - Settle per Motion. A haptic detent sounds as the nearest Space changes.
  - Wheel mice: deltas sum while they come less than 0.049 s apart. Once |sum| > 1 pt, it pages one Space and then ignores the wheel for 0.25 s.
- Ours, where Dia and Arc aren't measured:
  - The first move decides the axis: across if it's more across than along. Momentum after a page swipe is ignored, and a wheel tick counts as 10 pt, so one tick pages.
  - The window shows the new Space when the fingers lift, not while they move. Until then the page beside shows that Space's rows as they will show, with the tab it would show selected, and takes no input.
  - A dot click, ⌃1–⌃9 and ⌥⌘←/→ slide the new Space's page in from its side on the Space settle curve, pushing the old one out.
  - The detent is AppKit's alignment haptic. Swipes don't page while the Create a Space form, a rename or a row drag is up.
- The window tint cross-fades between the two Spaces' colours during a swipe (Ardia; Dia and Arc durations: measure).
  - Ours (*interim*): the gradient is as much of each Space's colour as its page shows, during a swipe and while a switch's pages slide.
- Measure in Arc: the Space title, icon and colour flows, the form's Profile and Choose a Theme rows, and how a Space shows its profile.

**Creating** (Arc 1.170 rec at 2×, one Space named and made)

- New Space, from the menu or a pull past the last Space, shows the Create a Space form in the sidebar. Arc's has an illustration, a title and a subtitle, the name field, Profile and Choose a Theme rows, and at the bottom Create Space and Cancel. Ours has the title "New Space" and the field, as the illustration and the copy are Arc's.
- Create Space is Arc's blue, #2129FB, 36 tall with a radius of 8 and a white label, semibold 13. Until the field has a name, all of the button is at 0.4. Cancel is white 0.6, semibold 12. Create Space doesn't change under the pointer or while pressed; Cancel's wasn't recorded.
- Create, or ↩ with a name, sends the form away from the bottom up (Space form leave): the buttons fade, then the rows, the field and the title fade as they sink. Arc then plays its illustration for about 2 s and shows the Space's rows within a frame. Ours shows them once the form is gone, 0.28 s after Create.

**Renaming and deleting** (Arc's menu; our wording)

- A right-click on the Space's name, or on its dot in the switcher, shows a native menu with Rename Space and Delete Space. Delete Space is off for the last Space.
- Rename Space, or a double-click on the name, turns the name into a field with all of it selected. ↩ keeps what's typed unless it's empty, Esc cancels, and a click elsewhere in the sidebar keeps it.
- Delete Space asks first in an alert naming the Space, if it has tabs, and its tabs close with it. Its Today tabs reopen with ⇧⌘T in the Space shown; its pinned tabs don't come back. An empty Space goes without asking.

**Colour.** The Space colour drives the window gradient (Shared rules), the New Tab band and halo, and the accent. Dia's hue families are neutral, red, orange, yellow, green, blue, pink and purple. Only pink is measured: tint #B25B6B (P3), New Tab band #B5556B. Arc's presets and editor are under Themes.

- Ours (*interim* until Arc's colour flow is captured): the Space's menu has Color, between Rename Space and Delete Space. Its submenu has System Accent, for no colour, then Dia's families as Gray, Red, Orange, Yellow, Green, Blue, Pink and Purple. Each has a round swatch, and the Space's own is checked.
  - Pink is Dia's, #BF556A in sRGB. The others share its HSL saturation 0.45 and lightness 0.54, at hues 4°, 26°, 46°, 145°, 214° and 268°; Gray has no saturation.
  - A new Space has no colour. The core keeps the colour with the Space's name.

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
| Width | the field's width plus ≈ 41 on the left and ≈ 40 on the right: 888 for an 810 field. Arc's sidebar field: measure |
| Text | the same x, baseline and size as in the field (13 pt); 48 in from the panel's left edge. The search glyph is centred 30 in |
| Open | grows out of the field's box (Panel grow). Its left edge reaches 63, 82, 92, 95 and 99 % on successive 60 fps frames, and the height grows with it. The content doesn't move or scale; the growing shape uncovers it |
| Close | measure (Dia and Arc) |
| Radius | 20 continuous for the New Tab box (bin); ≈ 16 for the URL panel (1.49 cap): measure |
| Fill | opaque #2D2D2D / #FEFFFF (1.50.1 bin; the cap reads (45, 45, 45)). The 1.49 URL panel was ≈ #2A2A2A |
| Border | 1 device pixel, rgba(120,125,134,0.32) in dark |
| Shadow | black 0.12 (Dia's default), plus from 1.50 two layer shadows: black 0.08 radius 2 offset (0, 0.5), and black 0.04 radius 1 offset (0, 2) |
| Scrim | none: the page stays visible and undimmed (1.49) |
| Input row | 54 tall (1.49 cap). The leading icon becomes the selected suggestion's favicon |
| Suggestion row | 36 tall, 16 pt favicon, title ≈ 15 pt white, " — url" at 0.5 (1.49 cap) |
| Selected row | warm grey ≈ #5E5A5D, radius 10, inset 4 (1.49 cap) |
| Bottom row | 53 tall (1.49). Dia's "+ Add tabs or files" chip and mic are AI and are dropped. The "Go ↩" pill (≈ 92 × 29, lavender ≈ #DCCBD8, dark text) stays and names the search engine (Ardia) |

- Ours (*interim* until Arc's sidebar panel is measured):
  - From the sidebar's URL field, the panel's left edge is the field's and it's 640 wide, ending at least 8 inside the window. Its input row is centred on the field, so the text keeps the field's x, baseline and 13 pt. There's no search glyph, since the field has none to the left of its text.
  - The grow is 1 − 2^(−10t / 0.13): 59, 83, 93 and 97 % on 60 fps frames. The radius goes from the field's 12 to 16, and the shadow follows the shape.
  - ⌘T's panel, until the New Tab page, is the same panel centred a fifth of the way down the window, shown at once.
  - Rows are 13 pt like the input, without favicons for now. A hovered row takes the hover token.
  - The shadow is the system's window shadow on the panel's shape, without Dia's two layer shadows. The bottom row isn't built yet.

**Rows**

- The Enter target is the inline match, else a typed URL, else a search for what was typed. Page titles are never the default (Chrome). Dia's "Use website first" setting puts a strongly matching page first instead, and "Prefer search engine" turns that off.
- The search row comes right after the top hit (Dia 1.49).
- Row 0 is computed from local data only; remote suggestions append below it, so row 0 never flickers (Chrome).
- Open tabs appear as "Switch to Tab" rows across windows, never the current tab (Dia).
- History rows can be removed with a hover ✕ or ⇧⌦, and the selection moves to the next row (Chrome; Dia has the ✕).
- Bookmarks, a calculator (↩ copies the result) and commands are also rows (Dia).
- **Commands** (Arc 1.170.0 cap). "clear" lists Clear Today with a ⇧⌘K chip at the row's right, then the search row, then Clear Cache and Refresh, Collapse All Folders and Clear Archive, so a name matches by its letters in order, not only its start. The field's icon becomes the command's.
  - Ours (*interim*):
    - A command matches when its name starts with what's typed, then when a word of it does, then when what's typed has its letters in order from a word's start. It takes two letters or more.
    - A command whose name starts with what's typed, three letters or more, comes first, unless the core completes what's typed. The search row follows it. Otherwise the commands follow the core's first row. The other commands come next, at most four in all.
    - The names are Arc's bar's where they're known (Clear Today, Clear Cache and Refresh, Collapse All Folders, Clear Archive), else its menus'.
    - Clear Cookies and Refresh is in the bar too, though Arc's "clear" doesn't list it. A name that starts with what's typed comes before one that only has its letters, so "clear" shows it instead of Collapse All Folders.
    - Only commands that would do something now are listed: Go Back with history behind, Close this Split Pane in a split, and so on.
    - The chip is the shortcut in 11 pt at the secondary colour on white 0.08, radius 4.
    - ↩ or a click runs the command. The bar closes first, as for a pick.
    - Not yet: the field's icon, and Arc's commands we don't have.
- Well-known sites that what's typed could start, such as YouTube for "you", fill the rows the profile's own leave, under them and never as row 0, so a new profile's list isn't bare. The list is built in and ranked by popularity alone (ours).
- Hovering highlights a row; Enter uses the keyboard selection (Chrome). Dia: measure.

**Inline completion** (Chrome; Dia rec, O)

- It completes only one term, with the caret at the end.
- It never completes during IME composition, and ↩ then commits the composition instead of navigating. It completes again after the composition is committed.
- It never completes after a deletion (⌫, ⌦, ⌥⌫, ⌘⌫ or cut) until the next typed character. After a deletion, the search row goes first.
- It never completes right after a paste, or when the input ends in whitespace.
- Hosts complete first; paths only when the typed text contains "/". The typed prefix keeps its case.
- Only hosts the user has typed are eligible: typed once for a host, twice for a deeper URL. Bookmark-only and open-tab-only hosts are listed but don't inline (Chrome).
- → or ⌘→ accepts the completion, and so does a click at its end. A ⌫ after accepting deletes one character and doesn't complete again.

**Site search.** Tab on a hinted row, or Space after an exact host or engine keyword, enters the site's search. ⌫ in the empty field leaves it and restores the text, keeping the space when Space entered it, and the bar stays open (Chrome).

**Keys**

| Key | Action | Source |
|---|---|---|
| ↩ | open where the bar shows pages: a new tab for ⌘T, the tab for ⌘L | Chrome |
| ⌘↩ | a tab behind the one shown, at the top of Today; the bar stays open. A command runs as for ↩ | Arc 1.170.0 cap |
| ⌥↩ | a new pane on the right of the page, in a split view | Arc 1.170.0 cap |
| ⇧↩ | new window | Chrome |
| ⇧⌘↩ | search for what was typed, even if it names a page | Dia |
| ⌃↩ / ⌃⇧↩ | www.<word>.com for a word typed, else as ↩ / the same in a new window | Chrome |
| ↑ ↓, ⌃N ⌃P | move, wrapping at the ends | Chrome |
| ⇞ ⇟ | first / last row | Chrome |
| Tab / ⇧Tab | enter site search when hinted, else the next row / previous row | Chrome |
| Esc | leave site search; otherwise close the panel, drop the typed text and give the page its focus back | Ardia |
| ⇧⌦ | take the selected row's page out of the history; the row under it takes its place | Chrome |

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

Arc 1.170.0's values where measured (cap, ax, rec), Dia's (P§6, D›Dragging tabs and split targets) for the rest.

- **Panes.** Up to 3 (Dia; Arc's maximum: measure), side by side and 10 pt apart, sharing the card. Each has its own page, history and docked DevTools. The focused one has a 2 pt ring inside its edge and a ≈ 8 pt glow outside it (Arc), in the Space's colour, else the accent. Unfocused panes aren't dimmed (*interim*, measure). A page in full screen takes the window alone.
- **Focus.** A click in a pane focuses it. ⌘W and Close this Split Pane close the focused pane, and the one beside it takes focus. Separate Page from Split View puts the focused pane in its own row under the split, shown alone. The find bar, the zoom HUD and a tab's dialogs, such as print preview, go over the focused pane.
- **In the sidebar.** A split is one row of equal segments with a 1 pt line between them: each segment's favicon (18) 7 into it, its title 10 after that, ending 5 before the next line (Arc). The row counts as one for ⌘1–⌘9, ⌃Tab and next or previous tab, which show the pane focused last. Its ✕ closes all its tabs, and on a pinned split it unloads them and the row stays a split. Moving or pinning the row moves the split; Move to Space takes one tab out.
- **Creating a split.** Add Split View: in one frame the page narrows, an empty pane opens on the right and the command bar opens over it, its panel 77 in from the card's right edge (Arc). A URL opens in the pane; a tab picked joins the split. ⌥↩ in the command bar opens its URL in a right pane (Arc). Dragging a tab from the sidebar onto the page shows the targets, below. To build: ⌥-click on "+ New Tab" opens a New Tab pane, and ⌥⇧-click on a link opens it in a right pane (Dia).
- **Commands.** Arc's View menu: Add Split View ⌃+, Close this Split Pane ⌃⇧-, Separate Page from Split View, and Expand Current Split (to build). To build, from Dia: Flip, Change Orientation, Move Pane Left/Right, Separate All. ⌃⇧] and ⌃⇧[ focus the next and previous pane (Dia, *interim*).
- **Kept.** A split comes back after a quit.

**Drag targets** ("Add left split" and "Add right split" at each pane's sides)

| Stage | Size (fraction of the card) | From the card's side | Look |
|---|---|---|---|
| At rest | 198.5 × 333.5 (0.132 × 0.383), centred vertically | 37.5 | radius ≈ 20; fill a step lighter than the page with a lighter 0.5 pt edge; a dashed rounded rect 9.5 inside it (6 pt dashes, 6 pt gaps, ≈ 1.75 pt wide); a ≈ 19 pt half-filled-rectangle glyph above a 13 pt semibold title at white ≈ 0.6 |
| Grown, right after sliding in | 322 × 532 (0.215 × 0.611) | 30 | the same |
| Under the dragged card | 362 × 600 (0.241 × 0.69); its centre leans 43 % of the way toward the pointer's x | — | fill the accent at ≈ 0.15; dashes, glyph and title in the accent; 7 pt dashes and gaps, ≈ 2 pt wide |

- **Timing.** The targets slide in from outside the card's edges in ≈ 110 ms, then grow over ≈ 120 ms with no overshoot. In the top strip they appeared when the pointer was 26 pt below the strip. For the sidebar, measure the trigger in Arc.
- **What the drag looks like.** For a frame or two the tab itself follows the pointer. Then it becomes a 209 × 109.5 picture of a window, centred on the pointer, with a 0.5 pt light rim, a shadow and traffic lights. Over another window's tab list it turns back into a tab in ≈ 50 ms. Ardia then opens a tab-wide gap at the pointer in 120 ms; Dia's: measure.
- **Drop.** On a target, the page splits and the new pane slides in on that side (timing: measure). Away from a target, the tab opens in its own window, which grows out of the card at the pointer in ≈ 200 ms and takes focus. A pane leaves its split the same way. Dragging the only tab on a page splits it with the tab used before it.
- **Ours, until Arc's are measured:**
  - The targets come once the pointer is 26 pt past the card's left edge, Dia's distance from its top strip. They go when it's back over the sidebar, and the tab is a row again.
  - A target is under the tab while the pointer is in it. It stays so until the pointer leaves the bigger target it grows to, and grows and shrinks over 120 ms on an ease-out cubic.
  - The look: a fill white 0.06 over the window's, a 0.5 pt edge of white 0.14, and dashes of white 0.25.
  - The picture of the window: radius 10 with a white 0.25 rim, traffic lights 6 pt, and the tab's favicon and title on a page under them. Dia's picture has the page's image.
  - The tab becomes the picture at once, not after a frame or two.
  - On a drop, the split shows at once, focused on the tab dropped. Away from a target, the tab goes back to its row until there are windows to open.
  - Targets are at the card's sides only, not at each pane's. Favorites, folders and split rows get none, and neither does a tab of the split shown or a split that is full.

### Settings

Arc's window (Arc 1.170.0 cap and ax), opened by ⌘, and Arc › Settings…. Its tabs are Arc's that have a feature here; the rest come with their features.

| Item | Value |
|---|---|
| Window | 684 × 650, fixed, its own for every tab; dark; close only (minimize and zoom off); centred over the browser window the first time. ⌘W and the close button hide it, and it opens again on the tab shown last |
| Title bar and toolbar | #282A2A to 88, a 1 pt line of #303131 under it. The title (the tab's name) centred in the first 32, #A4A6A6. The toolbar's tabs are 55 wide (Shortcuts 63, Advanced 64) at a 56 pitch, centred, 56 tall from 32: a 20 pt glyph over an 11 pt label, #9E9F9F, and the tab shown in the accent colour on a 44-tall plate of #444747 with a light edge |
| Content | #202222. Boxes 64 from the sides and 20 below the toolbar, 9 apart, fill #232525, 1 pt #353737 border, radius 6. Rows 13 pt #C2C2C2 at 10 in, 10 above and below; 1 pt lines between them, 8 in. A row's 20 pt colour tile, radius 4.5, puts its text at 38 |
| Controls | Switch 36 × 16 at the row's right: the accent with a #E1EBFE knob, or #383939 with a #E1E2E2 knob, 21 × 13. Pop-up and buttons 24 tall, #313333 at radius 5, text 12 in; a pop-up is 155 wide with its chevrons 10 from its right, ends 22.5 short of the switches, and opens its menu over itself with the chosen item on its text. Buttons under a box go 11 below it |

- **General.** Warn before quitting (Window › Warn before quitting) and Archive tabs after (Archive › Auto-archive). Arc's Previews comes with that feature.
- **Shortcuts.** Ours: the browser's shortcuts by section, read-only, each in a chip as the command bar shows it. Arc's lets the user change some; that comes with rebinding (Keyboard).
- **Advanced.** Haptic feedback when swiping between Spaces, on by default, which Arc doesn't have (it has one for reordering tabs). More Settings… opens Chromium's settings in a tab and hides the window.

### Archive

Arc's structure. Dia replaced it with Clean Up Tabs, recorded below for reference.

- **Clear** (Arc 1.170.0 rec, 3 runs). ⇧⌘K or the Clear button closes every Today tab of the Space but the one shown, including grouped, split and playing ones. Pinned tabs and favorites stay. The rows go in one frame and the one left is in its place at once, with no toast. ⌘Z brings them back in place, fading in on Row motion, unloaded, splits still split, and the tab shown stays. Ours: ⌘Z in a text field, in a page or ours, is the field's own, and Edit › Undo clicked in the menu goes only to the focused page or field.
- **Arc's clear actions.** Arc's binary has three: clear all, clear unselected Today tabs, and tidy-and-clear. Tidy is AI and out of scope.
- **Dia's Clean Up Tabs** (⌥⌘K) moves untouched tabs to "Recently Cleaned", with a daily automatic run. It is not ours.
- **Closed tabs.** Closed tabs reopen with ⇧⌘T, newest first, with position, pin, group, custom name and icon, mute state and back/forward list (Dia, P§2).
- **The archive.** A Today tab that closes, by ⌘W, its ✕, Clear or its menu, goes in the archive, newest first, with its URL, title and when it closed. Arc's menu calls ⌘W Archive Tab. A tab ⇧⌘T or ⌘Z brings back leaves it. It's saved with the Spaces. Ours: it keeps a tab 30 days, up to 500 tabs, and archived tabs keep no Space.
- **Auto-archive.** Arc archives Today tabs not shown for 12 hours by default. Settings › General › Archive tabs after offers 12 hours, 24 hours, 7 days and 30 days (Arc 1.170.0 bin), and Arc keeps the choice per profile; ours keeps one, with the Spaces, until profiles land. Ours: a Today tab's 12 hours start when no window shows it any more, or when it opens unshown, and count on the wall clock, through sleep and while the browser isn't running, so a launch archives the tabs whose time ran out. The tab a window shows and one playing sound stay. It goes in the archive as a closed tab does, so ⇧⌘T can reopen it.
- **The Library** (Arc 1.170.0 cap and ax, ⇧⌘L, Window › View Library, or the footer's button at its left). It slides in from the left, 417 wide, as it pushes the card's left edge to 417 without resizing it, so the page runs off the window's right edge (Library push). It starts its own width to the left and moves with the card's edge, so they meet as they stop. Opened or closed by a click, the sidebar under it fades out where it is in 2 frames as it comes, and back in over 130 ms from 80 ms after it starts to go (Library fade); moved by fingers, it recedes (Spaces › Switching). While it moves, the sidebar draws the window's buttons, which go with it, and the window's own show again once it's open, on the column, as in Arc. A swipe or a wheel's tick past the first Space opens it too (Spaces › Switching). A column 112 wide, white 0.06 with a 1 pt line of white 0.04 at its edge, holds the sections, 97 × 80 at x 8 with 100 between them, centred 5 above the middle; the selected one has a white 0.09 plate at radius 12, its glyph in the Space's colour 31 below its top and its label, 12 pt semibold at white 0.85, 56 below. Back (←) is 32 × 32 at (8, h − 40). Arc's sections are Media, Downloads, Easels & Notes, Spaces and Archived Tabs; ours are Downloads and Archived Tabs, until the others' features land. The footer's button and ⇧⌘L open the Library on the section shown last.
- **Archived Tabs.** A search field on the window buttons' row, x 120, 36 tall at y 10, white 0.09 at radius 10, its glyph 15 pt at x 134.5 and its text 14 pt at x 164, "Search Archive…" in white 0.56. Arc's Filter button (73 × 36 at x 336) is left out and the field runs to 409. The list starts at y 56: day headings at x 124, 12 pt semibold at white 0.6, and a row per tab, 56 apart, with its favicon 16 pt at x 134 and its title (14 pt, white 0.78) over its host (12 pt, white 0.6) at x 164. Ours: headings Today, Yesterday, Earlier This Week, Last Week and Older (Arc's capture showed Today and Earlier This Week); a hovered row's plate is the sidebar's (x 120–413, 50 tall, white 0.12); search matches titles and URLs as typed; a click reopens the tab at the top of the Today tabs of the Space shown, with its history if it closed since the browser started, and closes the Library. Esc doesn't close it (Arc).
- **Clear Archive** (Archive menu) asks first, then empties the archive.
- Measure in Arc: the archive's limits, the Filter menu, a row's hover and right-click, whether restoring closes the Library, Clear with a pinned tab shown, and ⌘Z after Clear with a page's text field focused.

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

- **The Library's Downloads** (Arc 1.170.0 menus and cap). Downloads is a section of the Library, above Archived Tabs. ⇧⌘J and Window › View Downloads open the Library on it, and close it from there. The list is laid out as the archive's: search, then day headings, newest first.
  - Each row has the file's icon from the system, its name, and under it where it is: "48.2 MB of 121.5 MB" over a 2 pt bar while it runs, "Paused · …", "18 KB · data.example.org" once done, "Cancelled · …" or "Failed · …" with its site, and "May harm your computer" while Chrome holds it. Sizes count in thousands, as Finder's do.
  - A click opens a finished file, and on a held one shows its menu. A right-click shows what can be done with it: Keep and Delete while held; Pause or Resume, Cancel and Open When Done while it runs; Open and Show in Finder once done. Then Remove from List and Clear List, which take finished downloads off the list and keep their files.
  - Still to come: Move to Trash, and dragging the file out to Finder, other apps, the page or the tab list.
- **Footer button** (ours until measured). While a download runs, the footer's Library button shows a download glyph over a 16 × 2 bar of the running downloads' progress, and a click opens Downloads. A new download flies into it in Arc and Dia; Dia's "magnet" constants 0.1 s and 1.3 are not decoded, so record it in Arc.
- The list is per profile and persists across launches in Chrome's history, up to the newest 500; private windows' downloads are never written.
- **Quit guard** (Chrome's). Quitting with downloads running asks first, in Chrome's alert: "Download is in progress", with Continue Downloading or Quit. Continue Downloading keeps the browser running and shows the Library's Downloads, where Chrome would open its downloads page; Quit cancels them, and they come back cancelled. The alert names the product with Chromium's strings until branding lands. Arc's wording: measure.
- Measure Arc's Downloads section, its button while downloading, and the fly-in.

### Find in page

- **Keys** (Dia, P§8). ⌘F, ⌘G, ⇧⌘G and ⇧↩ for previous. ⌘E (Use Selection for Find), ⌘J (Jump to Selection), ⌥⌘F (Find and Replace in the focused field).
- **State.** Each tab keeps its own query and shows "n of m".
- **Find bar.** It floats over the page, so it is a child window. With the page focused, ⌘F reaches us unless the page keeps it (as Google Docs does). Japanese IME works in the field. ⌘V right after ⌘F lands in the field.
- **Arc's bar** (Arc 1.170.0 rec ×3, cap, ax, on iana.org).
  - It's 300 × 32, 10 in from the card's top and right edges, radius ≈ 6, fill (19,29,50), with a soft shadow mostly below, ≈ 16 pt.
  - From its left edge, all 28 pt squares 2 pt from the top: the search glyph at 2; the field at 31, 152.5 wide and 18 tall; the count ("1/12") at 185.5; previous and next at 212 and 238; a divider; close at 270.
  - With no match, "NOT FOUND" in capitals and red (219,25,41) takes the place of the count and the arrows.
  - It opens and closes in one frame, with no animation.
- Ours: Arc's layout. The fill is the window's colour, as the toast's, since Arc's (19,29,50) looks like its Space's colour, and AppKit's window shadow stands in for Arc's. The glyphs are Lucide's search, chevrons and x at 14 pt, in the header's colours and hover plate; the count is 12 pt and NOT FOUND 11 pt semibold.

### Zoom

- **Keys.** ⌘0, ⌘+ and ⌘- in the View menu, as Arc's Zoom to Actual Size, Zoom In and Zoom Out, and ⌘= as Chrome's hidden second ⌘+. Chrome's levels, for the page's whole site, which the profile keeps.
- **Arc's HUD** (Arc 1.170.0 cap). A zoom shows its percent ("125%", "100%") in a child window over the card: the text's right edge 20 pt in from the card's, its middle 29.5 below the card's top. The URL field shows nothing.
- Ours: the percent in 14 pt medium on a pill of the window's colour, 32 tall, radius 6, with AppKit's shadow, its right edge 10 in from the card's as the find bar's, and 8 pt under the find bar while that's open. The mouse goes through it. It stays 1.5 s after the last zoom, then fades out over 0.15 s, and goes at once when another tab shows. Arc's pill, timing and fade aren't recorded.

### DevTools

Chrome's DevTools (Dia, P§23), docked in the card as Arc's.

- **Keys.** ⌥⌘I opens or closes them, ⌥⌘J opens or closes them at the console, ⌥⌘C opens them picking an element, and ⌥⌘U opens the page's source in a `view-source:` tab under it. They're Arc's View › Developer items: View Source, Developer Tools, Inspect Elements and JavaScript Console. From the page and from DevTools they reach the UI unless the page keeps them. The page menu's Inspect opens them at the element under the pointer. Dia and Chrome also have F12.
- **Arc's docking** (Arc 1.170.0 cap). On the right inside the card, full height, with their own 26 pt toolbar at the card's top. The page keeps the card's left part: 853 of 1274, so DevTools take a third. They open in two steps with no animation: an empty dark pane at once, then DevTools drawn in it ≈140 ms later with the page laid out narrower. ⌥⌘I again closes them.
- **Docking.** As Chrome's window does it: DevTools fill the card, and the page goes where they leave it, rounded at the card's corners it reaches. Each tab has its own; a tab without them shows its page across the card. Closing them gives the page the keyboard back. DevTools' own menu docks them on another side, where the page goes the same way.
- Ours: Chrome's width, 555 pt the first time, then wherever the user drags the divider, which DevTools keep per profile. Arc's third may be the capture's saved width.
- **Undocked.** DevTools' Dock side menu undocks them into Chrome's own window, which takes the keyboard, and docks them again. Dia's is titled "Developer Tools - <url>", opens where it was last left, and is 640 × 640 the first time.
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

Arc's keys are recorded: ⌥⌘N, and ⌘O to open the page in the main window. The behavior below is Ardia's version (P§1), modeled on Arc and not measured from it.

- **Opening.** A small window with one page and no sidebar, centred a little above the middle of the screen the pointer is on. It opens on the current macOS desktop at its last size, never its last position. The first size is 900 × 640 (**Ardia**).
- **Bar.** A thin bar in the Space colour holds the window buttons, favicon, host and title. A click on it or ⌘L opens a compact command bar. The bar also has Copy Link and Open in the main window (⌘O). ⌘O moves the live page, with history, form text, scroll and playing media, into the main window and closes Little Arc.
- **Closing.** ⌘W, the close button or Esc (when the page doesn't use it) closes the window and drops the page. The page stays in History, and ⇧⌘T reopens it in Little Arc with its back/forward list.
- **Links from other apps** open in a new Little Arc each time; a setting switches this to "a new tab". The page's own links stay in it. `target=_blank` and ⌘-click open as tabs of the main window, behind.
- **Where links land.** Ardia uses the profile of the window used last. Ours: the Space used last.
- Measure everything visual in Arc: size, bar height, radius, colours, the open and close animations, and how ⌘O moves the page.

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
| Pinned tabs and favorites belong to the profile (P§2) | Pinned tabs belong to the Space, favorites to the profile |
| Arc import turns Arc Spaces into profiles (P§20) | Spaces stay Spaces |
| The default layout is Dia's (address bar on the card, optional top tab strip); Arc's sidebar address bar is a setting (P§4) | Arc's layout only (decision 6) |
| Dia's and Chrome's keys win clashes, so Arc's ⌘D pin moves to ⌥⇧⌘D (P§4) | Arc's keys win, since Arc's users bring their hands with them: ⌘D pins, ⌘S hides the sidebar, not Chrome's Bookmark and Save Page (2026-10-25, to review) |
| AppleScript `execute` runs JavaScript in pages, off by default behind View › Developer (P§23) | Not built: it gives any local process, agents included, what principle 3 forbids. AppleScript, if any, covers windows, tabs and URLs |
| Extension action state polled every 1.5 s, unused profiles checked every 15 s (P§15, P§5) | Event-driven only (principle 1) |
| The 1.49 New Tab area light ran a 30 fps display link for up to 60 s (D›Area light) | Not built; Dia 1.50.1 dropped it too |
| AI surfaces: "Ask anything…", the "+ Add tabs or files" chip, mic, Chat, ⌘E Chat, Tidy | Out of scope for v1 (decisions 6, 8) |
| "Peek" names the sidebar sliding in at the window edge | Peek is Arc's link preview; the edge slide is the sidebar reveal |
| Site Controls, the panel grid, Little Arc sizes and the 8 h update check are Ardia's choices | Placeholders until Arc is measured |

## What Arc still has to give us

Arc shuts down on 2026-12-22. This is the capture plan, by feature. Where it says "screenshot", it means a 2× window capture of the named state. Where it says "record", it means a recording of the interaction.

**Setup**

- Use a throwaway Arc profile on public sites, never the owner's accounts (AGENTS.md, trap 4).
- Note Arc's version. Start from a fresh install so defaults (sidebar width, archive interval) are Arc's own, and screenshot before resizing anything.
- Use a 1512 × 949 window to match the Dia captures. Capture dark and light, key and inactive.
- Capture Arc's window only, by pid, with `tools/window-screenshot.sh` (AGENTS.md, trap 2). Window-only captures never show the blur. For translucency, put a solid-colour window of our own behind Arc and composite the windows below Arc's frame (`CGWindowListCreateImage` with on-screen-below-window and including-window), cropped to Arc's frame (D›Window translucency).
- The owner performs the interactions; nothing sends synthetic input to Arc (AGENTS.md, trap 1).

**Static values** (measurable any time, machine load doesn't matter)

| Feature | Capture |
|---|---|
| Asset reads, no UI | `assetutil --info` on Arc's `Assets.car` files for every colour token, light and dark. The full menu bar with shortcuts. Settings defaults (archive interval, sidebar width) from Arc's preferences. Values only; never copy images, icons or fonts |
| Window | Screenshots, sidebar docked and hidden: card insets on all four sides, card radius, traffic-light centres. Translucency composites over black, grey and white backdrops, for each preset Space colour, key and inactive |
| Sidebar header | Screenshot at the default width: sidebar button, back, forward, reload (enabled, disabled, hovered), URL field idle, hovered and focused, an insecure site, a zoomed site, with two pinned extensions |
| Sidebar width | The default on a fresh install; the minimum and maximum, by dragging to each limit and screenshotting |
| Favorites | Screenshots with 1, 3, 4, 6, 9 and 12 favorites at the default width and at the minimum and maximum (columns, tile size, gaps, radius, favicon size). Selected colourful, one-colour and white icons; hovered tile with its tooltip; audio playing |
| Pinned tabs | Screenshots: a pinned tab resting, selected, navigated away (and hovered while away), unloaded after ⌘W; a folder expanded and collapsed |
| Today tabs | Screenshots: rows resting, hovered (✕), pressed, selected, loading, sleeping, playing audio, with a long title; "+ New Tab" row; the Clear divider; the scroll-edge line after scrolling. Row height and pitch (confirm 36) |
| Sidebar hidden | Screenshot of the reveal panel at rest at 2×, to tell its 0.5 pt border from a 1 pt one |
| Spaces | Screenshots: the switcher with 2, 5 and 10 Spaces, with icons and without; hovered and selected; the Space title row; create, rename, colour and icon flows; the profile picker for a Space; a Space whose profile differs. Answer: do favorites follow the Space or the profile? |
| Command bar | Screenshots: ⌘T empty; typing "git" with an inline completion; 8 suggestions with one selected and another hovered; a Switch to Tab row; a command row; ⌘L on a page; the bar with the sidebar hidden. Answer: what ⌘↩ and ⌥↩ do |
| Split view | Screenshots: 2, 3 and 4 panes, side by side and stacked; the divider or gap; the focused-pane indicator; the unfocused dim; the sidebar row of a split; drop zones while a tab is dragged over the page |
| Archive | Screenshots: the archive view with archived tabs, search in it, the archive settings with every interval option, the toast after an archive |
| Profiles | Screenshots: profile settings, creating a profile, an incognito window |
| Extensions | Screenshots: where pinned extensions sit, an extension popup anchored, the install prompt for a throwaway Web Store extension, the remove confirmation, Arc's site-controls panel |
| Passwords | Screenshots: the save prompt after a fake login on a local page, the autofill dropdown, the password settings |
| Downloads | Screenshots: the indicator during and after a download, the downloads list |
| Find | Screenshots: the find bar with matches and with none, its position against the page |
| DevTools | Screenshots: the default docked side and size, undocked |
| Import | Screenshots of each step of Arc's import from Chrome. Answer: where do Chrome's bookmarks land? |
| Peek | Screenshots: open from a favorite's link, and from every other click that opens one: size, position, radius, shadow, backdrop dim, buttons |
| Little Arc | Screenshots: opened by a link from another app: size, position, bar height and colours, the buttons; after ⌘L |
| Themes | Screenshots of each editor state; the window at intensity 0, 0.5 and 1, noise 0 and 1 for each texture, translucency on and off, as composites |
| Boosts | Screenshots: every way to start a Boost, each editor state, Zap picking an element, a site with a Boost in Site Controls, on and off. Answer: do Boosts follow the Space, the profile or the account? |
| Updates | Screenshot of the update prompt, if one can still be triggered |

**Timings** (need an unloaded machine: no Chromium build running, display awake, on mains power)

Record at 120 fps on a ProMotion display, 60 fps at the least. Use each frame's own timestamp, because recorders deliver frames unevenly. Repeat each interaction 3 times or more. Trace an edge per frame at 2× and fit a spring (response, damping ratio) or a curve and duration, reporting the rms error, as Ardia did.

| Feature | Record | Time |
|---|---|---|
| Sidebar | Dragging a row past three neighbours, into favorites and back | neighbour spring; drop settle |
| Sidebar | Hovering a favorite, then moving to the next | show delay, leave delay, tooltip motion |
| Sidebar | Folder expand and collapse | spring |
| Spaces | A slow two-finger swipe, a flick, an overscroll at the last Space; a click on the switcher; ⌃2 | tracking ratio, settle spring, rubber band, colour cross-fade duration |
| Command bar | ⌘T open and Esc; ⌘L open from the sidebar field and Esc; typing the first character | open and close curves; time to the first suggestion |
| Split view | Dragging a tab onto the page until the targets show; a drop on each side; dragging the divider; closing a pane | target slide-in and grow; pane slide-in; close |
| Downloads | Starting a download | indicator fly-in and its duration |
| Find | ⌘F and Esc | bar in and out |
| Peek | Opening, closing, expanding into a tab | each curve |
| Little Arc | A link from another app, then ⌘O, then a second link and ⌘W | window appear, the move to the main window, close |
| Extensions | Popup open and close | curve |
| Window | ⌘N; entering and leaving full screen with the sidebar docked | what the sidebar does, frame by frame |
