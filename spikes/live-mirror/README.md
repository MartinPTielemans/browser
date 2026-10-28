# Live mirror of a page another window shows (#359)

The question: can the window a page left show it live, at close to zero cost, in place of its picture? This branch only answers it. Nothing here merges as it is.

## What Chromium's code says

On the Mac, each page view in ilo has a compositor of its own (`BrowserCompositorMac` in its `HasOwnCompositor` state), since the view isn't inside a views layer tree. The GPU process draws that compositor's frames into a `CAContext`. The page's view in the browser holds one `CALayerHost` with that context's id (`ui::DisplayCALayerTree::GotCALayerFrame`). Each new frame is a commit in the GPU process, and the window server draws every host of the context. The browser process does nothing per frame: the context id stays the same, and `GotCALayerFrame` returns early.

So a second host of the same context, in the other window, shows the page's frames as they're drawn. Nothing in Chromium renders them a second time. The only added work is the window server compositing one more scaled layer when the page changes. With the page still, nothing changes, so nothing wakes.

A viz embed (a `SurfaceLayer` on the page's `FrameSinkId`, or `ui::Layer::Mirror`) would cost more. A GPUI window has no `ui::Compositor`, so the mirror would need one per window: a second viz `Display` that aggregates and draws the page's quads on the GPU for every frame the page draws, and a `CAContext` of its own. That's a second composite for each page frame, so it isn't close to free on the Mac. It's the path to look at on Linux and Windows, where Aura has `Layer::Mirror`.

The `CAContext` exists only while the page draws. Hidden or occluded, Chromium drops the page's compositor (`TransitionToState(HasNoCompositor)`), and a host of the old id shows nothing. When the page is shown again, a new compositor brings a new context id. The mirror therefore shows only while the page is `VISIBLE`, and the picture stands in otherwise. In `host` mode the mirror follows a new id through KVO on the layer that holds the page's host.

## What the spike does

`BROWSER_MIRROR` in the core picks a mode. Without it, nothing changes.

- `host`: a second `CALayerHost` of the page's context (`ui/base/cocoa/remote_layer_api.h`). It mirrors Chromium's view only, not a page in Safari's engine.
- `portal`: a `CAPortalLayer` of the page view's layer. It's private, and only used where `NSClassFromString` finds it. It follows the view through any change of context, and it would mirror a page in Safari's engine too.

The window that let the page go puts a view-only mirror over its picture, in the same place: the page's shape, fitted to 0.8 of the card and centred, under the veil. Clicks go through to the picture, which takes the page back as before. The mirror is updated only when its window or the page lays out, the card moves, the page's visibility changes, or the page gets a new view (`Window::UpdateMirrors`). Never per frame. The log says each change, as `browser: mirror …`.

None of it has been compiled. The core needs the Mac engine, and this branch was written on Linux.

## What to run (on the VM)

1. Build the dev engine with this branch's core: `engine/build.sh dev`. Link it with `app/setup.sh dev`, then run `app/build.sh`.
2. Run `spikes/live-mirror/measure.sh`, with `ROUNDS=3` by default.
   - It runs `app/check.sh mirror` with the picture alone, then with `MIRROR=host`, then with `MIRROR=portal`, in turns.
   - Each run puts two windows side by side and moves a page that paints every frame into the second.
   - It checks that the first window's card changes between two captures only when there's a mirror.
   - It then measures for 20 s (`MIRROR_SECONDS`) with the page painting, and again once a click has stopped it.
   - The measures:
     - CPU seconds of the app, its GPU process, its renderers and the window server, from `ps`;
     - their idle wakeups a second, from `top`;
     - with passwordless `sudo`, the GPU's active residency from `powermetrics --samplers gpu_power`.
   - If the VM's GPU reports no residency, record the window server's GPU time in Instruments' Metal System Trace for one painting run of each mode instead.
3. Look at `app/build/mirror-runs/*/mirror-*-1.png`: the card with the mirror, and it upright. Read `mirror.log` for the `browser: mirror` lines.

The bar (#359): against the picture alone, painting and still, no measure may rise beyond the spread between rounds.
- **The app, GPU process and renderers.** Expect no change at all.
- **The window server.** It's the only process that should do more, and only while the page paints.
- **Still.** Every idle wakeup count should match the picture's.

Also look by hand:
- Hide the window holding the page, or cover it. The mirror should go and the picture show.
- Show that window again. In `host` mode, `browser: mirror hosts context` should come back with a new id.
- Navigate the page to another site. The mirror should follow.
- Resize either window. The mirror should fit its card again.

## If it passes, and if it doesn't

- **Passes:** the picture becomes a live mirror in the real tree, with the docs updated (`docs/ux-spec.md` › Several windows).
- **Fails:** the picture stays, and it's also taken again when the page finishes a load or navigates, from those events (fix/mirror-refresh-359).
