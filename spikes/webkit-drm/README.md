# webkit-drm

Can an ilo tab play Netflix-class video with Apple's WebKit (WKWebView, FairPlay) instead of sending the user to Safari, and what would that cost? This is a minimal Mac app with one WKWebView in a persistent `WKWebsiteDataStore` from `dataStoreForIdentifier:` (macOS 14+), its own bundle id (`dev.browser.spike-webkit-drm`, so its data lives in `~/Library/WebKit/dev.browser.spike-webkit-drm`, apart from Safari's), and an injected script that logs every EME request, WebKit's answer, and each video's events to stderr.

`./build.sh` makes `build/WebKitDRM.app` with clang. Run `build/WebKitDRM.app/Contents/MacOS/WebKitDRM` directly; the environment variables at the top of `main.m` pick the page (`SPIKE_URL`), the user agent (`SPIKE_UA=safari`), the store, the display (built-in by default) and whether it makes sound (muted by default). It takes `js <code>`, `load <url>` and `quit` on stdin. `./netflix.sh` is the owner's run (below).

The sites pass and anything run since are muted (`_setPageMuted:`, WebKit SPI, plus a script that mutes every media element) and don't autoplay. Captures and logs from the runs on 2026-10-27 (macOS 27.0, Apple silicon, Safari's WebKit 605.1.15) are in `~/.cache/browser-spike-webkit/`, outside the repo.

## Findings

**1. FairPlay in a non-Safari WKWebView: yes.** On `https://example.com`, with WKWebView's default user agent and no entitlement:

```
"com.apple.fps":"granted, initDataTypes sinf,skd,cenc, mediaKeys true"
"com.apple.fps.1_0" / "com.apple.fps.2_0": granted, mediaKeys true
"prefixed com.apple.fps.1_0" / 2_0 / 3_0: true      (WebKitMediaKeys.isTypeSupported)
"com.widevine.alpha":"refused: NotSupportedError"
"com.microsoft.playready":"refused: NotSupportedError"
"org.w3.clearkey":"granted"   mse true, managedMse true, hls "maybe"
```

**2. A real FairPlay stream: yes, it plays.** Axinom's public test vector (CMAF H.264 cbcs, `media.axprod.net/TestVectors/Cmaf/protected_1080p_h264_cbcs`) in Axinom's hosted Shaka player with `keySystem=com.apple.fps`, its FairPlay certificate and license server, and no account:

```
page {"keySystem":"com.apple.fps","kind":"page-prefixed-keys"}
page {"type":"webkitneedkey" ...} {"initDataType":"skd","type":"encrypted"} {"type":"waitingforkey"}
page {"size":"512x288","time":0.51,"type":"playing"}
page {"time":2.73,"type":"timeupdate"} … {"time":10.81,"type":"timeupdate"}
js -> visible total 432 dropped 19 t=18.6 1920x1080
js -> visible total 457 dropped 29 t=23.6 1920x1080
```

To repeat it, muted: `SPIKE_AUTOPLAY=1 SPIKE_IGNORE_OCCLUSION=1 SPIKE_URL=<the player URL from Axinom's README, FairPlay row>`, then `js document.getElementById('open-player').click()` and `js document.querySelector('video').play()` on stdin.

The license came back (the video went from `waitingforkey` to readyState 4), playback ran at real time and climbed to 1920×1080, and the decoder counted frames. The capture of the FairPlay video is blank (`fps-playing.png`), while the same player on Axinom's clear stream shows its picture (`clear-playing.png`). That is macOS keeping protected video out of screen captures, as it does in Safari, so only a person at the screen can see the FairPlay picture. The owner's run below confirms it.

Two traps found on the way:
- A covered window's page is `hidden` to WebKit, which then stops painting it and its video. The spike's window sat under another app's window, so `SPIKE_IGNORE_OCCLUSION=1` turns WebKit's occlusion detection off (SPI). ilo's window is the one the user looks at, so it doesn't need that.
- Shaka took WebKit's prefixed `WebKitMediaKeys` path for FairPlay, not `requestMediaKeySystemAccess`. ilo's core logs and gates both.

**3. The real sites, logged out:** **the user agent matters, and Safari's is the one to send.** Each site loaded muted in a fresh store, under each user agent (`site-<name>-<ua>.png`):

| Site | WKWebView's own UA | Safari's UA |
|---|---|---|
| netflix.com | normal landing, `netflix.com/dk-en/`, "Ready to watch? Enter your email…" | the same |
| disneyplus.com | normal landing, `disneyplus.com/en-dk`, "LOG IN … SIGN UP NOW"; the page asks `WebKitMediaKeys('com.apple.fps.1_0')` | the same, and the same FairPlay request |
| open.spotify.com | **the mobile "Open App" page**, no player, no sidebar | the full web player: library, search, Log in |
| primevideo.com | normal logged-out landing, `primevideo.com/offers/nonprimehomepage` | the same |

None showed an "unsupported browser" page. Disney+ already picks FairPlay on the landing page, before sign-in. Spotify sniffs the UA and treats a WKWebView without "Version/… Safari/…" as a phone. So a WebKit tab in ilo sends Safari's user agent (`applicationNameForUserAgent`, as `SPIKE_UA=safari` does).

What can't be known without an account: whether each service's player accepts this WKWebView once signed in. Whether Netflix's player, which checks the browser again after sign-in, accepts it, and at what resolution. Whether Spotify's web player streams. Whether Disney+ and Prime Video ask for HDCP or a higher FairPlay security level than a third-party WKWebView gets. `./netflix.sh` answers that for Netflix.

**4. The owner's run:** `spikes/webkit-drm/netflix.sh`. It builds if needed, opens `netflix.com/login` in front on the built-in screen, with sound and Safari's user agent, in its own `netflix` store. Sign in, press play on any title, then quit (⌘Q). The log at `~/.cache/browser-spike-webkit/netflix.log` shows `page-asks`, `page-granted` and `playing`. `SPIKE_UA=default ./netflix.sh` tries WKWebView's own user agent. `./netflix.sh forget` deletes the store and its cookies.

## How it fits ilo

The owner decided on 2026-10-27 to build this. Here is the shape the findings support.

- **The view.** The core's window already shows pages through one seam: `platform::ShowPages` takes a `gfx::NativeView` per pane (`core/platform/window_views.h`), which is an NSView on the Mac, and a WKWebView is one. So a WebKit tab is a tab whose pane is a WKWebView instead of a `WebContents`'s view, with the same frame, corners and split-view ring. It lives in `core/platform/mac/` behind a portable "page engine" interface in the core. The tab model, Spaces, the sidebar and the C API don't learn which engine drew the page beyond a flag the app shows.
- **When a tab switches engines.** The trigger is the one the protected-media note already has: a page asks for a key system ilo can't play (Widevine or PlayReady), and a profile has opted in. The note's "Open in Safari" becomes "Play here with Apple's engine", which reloads the tab's URL in WebKit. Doing it automatically, without a click, would be quicker, but the switch loses the page's state and its sign-in (below), so the first time per site is the user's call, and after that the site is remembered per profile (a site setting, listed and revocable in site controls).
- **The way back.** The tab returns to Chromium when it navigates to another site, as the note already goes away at another site, and from the same card ("Back to ilo's engine"). Forgetting the site setting stops the switch.
- **Cookies.** Each profile gets its own `WKWebsiteDataStore` by `dataStoreForIdentifier:` (a UUID kept with the profile, as the spike hashes its store name). Chromium's cookies don't cross over, so the user signs in to Netflix once more in the WebKit store. Copying cookies over by `WKHTTPCookieStore` would move session secrets between engines, and these sites tie sessions to the device, so it's not worth it. Deleting a profile removes its store (`removeDataStoreForIdentifier:`), and clearing a site's data clears it in both engines.
- **What the tab loses.** Chrome extensions, so ad blocking too; DevTools (Safari's Web Inspector works instead, as `inspectable`); autofill and passwords from ilo's store (the user types them, or uses the macOS AutoFill that WebKit offers); Chromium's site settings, permission prompts and downloads, which need their WebKit counterparts (`WKUIDelegate`, `WKDownload`) or are refused. **Agents get no tool on such a tab:** the agent API must refuse every call on it, say why, and log the refusal, since the browser can't enforce "agents get sessions, never secrets" (AGENTS.md) inside an engine whose page it doesn't control. A WebKit tab in the Agent Space isn't made at all.
- **Memory and launch.** Unused, it costs nothing if the core doesn't link WebKit: `dlopen` of WebKit.framework into a process that doesn't link it takes 3–4 ms and 0.4 MB here, after which `NSClassFromString(@"WKWebView")` works. So the core weak-loads it on the first WebKit tab, and launch is untouched (check with `otool -L` that Chrome's framework doesn't already pull it in). In use, WebKit adds its own WebContent, GPU and Networking processes for the WebKit tabs, beside Chromium's. Measure that per tab before shipping; it only exists while a protected-media tab is open.
- **Platforms.** Only the Mac has WebKit. On Windows the same engine slot takes Edge's WebView2 (PlayReady), and Linux keeps the note. The portable interface is what makes that one more implementation, not a second design.
- **Decision 1's text.** It says "Chromium, not WebKit". It needs one more bullet: *Chromium renders every page, except that on the Mac a tab whose site needs DRM Chromium can't provide may play in the system's WebKit, and on Windows in WebView2, on the user's choice per site. Such a tab has no extensions and no agent access. Revisit when ilo ships a signed Widevine CDM with VMP that these sites accept.*
- **Effort, roughly.** Core: a page-engine interface, the WebKit page (view, navigation, title, favicon, loading, audio, fullscreen, find, downloads refused or handled) and its per-profile store, 2–3 weeks. App: the card's new button, the engine flag in the sidebar and URL field, the site setting in site controls, about a week. Agents refused and logged, and the check runs (a WebKit tab on a public FairPlay vector, muted, as here), about a week. So 4–5 weeks for one person to a version the owner uses, plus the Netflix sign-in run to confirm each service.

## Recommendation

Build it, as decided, as a narrow escape hatch rather than a second browser: one opt-in per site, per profile, with the way back always on the card. Keep Widevine (#236) and the VMP application going. Chromium is still where extensions, agents and everything else live, and WebKit tabs should stay rare. Before the build lane commits to the shape, run `./netflix.sh` once: Netflix is the site most likely to reject a non-Safari WKWebView.
