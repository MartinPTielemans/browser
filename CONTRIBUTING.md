# Contributing

## Sign-off

Every commit needs a `Signed-off-by:` line. It certifies the [Developer Certificate of Origin](https://developercertificate.org/): you wrote the change, or you have the right to submit it under this project's license.

```sh
git commit -s
```

## Code from other projects

The browser is GPL-3.0-or-later, so code under GPL-3.0-compatible licenses can come in: Chromium (BSD-3-Clause), ungoogled-chromium (BSD-3-Clause), Helium (GPL-3.0), Ardia (Apache-2.0 / GPL-3.0), GPUI (Apache-2.0), Lucide's icons (ISC, some MIT).

When you copy or adapt a patch or file:

- Keep its original copyright and license header.
- Name the source and the upstream commit in the patch header or the commit message.
- Add it to `NOTICE` if its license asks for attribution.

Don't bring in assets from Arc, Dia or any other browser: icons, fonts, sounds, images or names. Measured values (sizes, timings, colors) and behavior are fine.

## Performance

Speed is the product. Changes on hot paths (tab switch, new tab, command bar, launch) must not regress the benchmarks in `docs/roadmap.md`. A PR that does needs a stated reason.

## Portability

No Swift, Objective-C or Metal outside `platform/mac/`. Core code must compile on Linux.
