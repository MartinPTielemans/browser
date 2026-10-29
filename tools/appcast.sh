#!/bin/bash
# Makes the DMG tools/release.sh made the update every copy of ilo checks for: an entry in the
# appcast, with its release notes and signed with the owner's EdDSA key in their Keychain, in
# app/build/release/appcast/appcast.xml. With --publish, the DMG goes up as the GitHub release
# v<version>, and the appcast to the release the feed (app/branding/update-feed) is served from.
# The notes are Markdown, shown in the update's alert; its first launch opens the release's page.
#
# Once, on the owner's Mac, before the first release, with Sparkle from app/setup.sh:
#   app/.deps/Sparkle/bin/generate_keys --account ilo                      # the key, in the Keychain
#   app/.deps/Sparkle/bin/generate_keys --account ilo -x ~/ilo-update-key  # its backup, kept offline
#   app/.deps/Sparkle/bin/generate_keys --account ilo -p > app/branding/update-key  # committed
# Every prod build carries the key in app/branding/update-key. Then, for each release:
#   app/build.sh prod
#   SIGN_IDENTITY="Developer ID Application: … (TEAMID)" tools/release.sh
#   tools/appcast.sh notes.md && tools/appcast.sh notes.md --publish
# Usage: tools/appcast.sh notes.md [--publish]
set -euo pipefail
root="$(cd "$(dirname "$0")/../app" && pwd)"
notes=${1:?"usage: tools/appcast.sh notes.md [--publish]"}
publish=${2:-}
name=$(cat "$root/branding/name")
out=$root/build/release
app=$out/$name.app
dmg=$out/$name.dmg
bin=$root/.deps/Sparkle/bin
feed=$(cat "$root/branding/update-feed")
# The feed is a file in a release of this repository: https://github.com/<owner>/<repo>/releases/…
repo=${feed%%/releases/*}
slug=${repo#https://github.com/}
feed_tag=${feed#"$repo"/releases/download/}
feed_tag=${feed_tag%%/*}

[ -f "$dmg" ] || { echo "No $dmg. Make it with tools/release.sh." >&2; exit 1; }
[ -x "$bin/generate_appcast" ] || { echo "No Sparkle in app/.deps. Run app/setup.sh." >&2; exit 1; }
version=$(plutil -extract CFBundleShortVersionString raw "$app/Contents/Info.plist")
# An update signed with another key than the one the app carries is one it refuses.
key=$(plutil -extract SUPublicEDKey raw "$app/Contents/Info.plist")
if [ "$("$bin/generate_keys" --account "$name" -p)" != "$key" ]; then
  echo "The key in the Keychain isn't the one $app carries." >&2
  exit 1
fi

# generate_appcast adds to the appcast at its output, so it starts from the one published.
work=$out/appcast
rm -rf "$work"
mkdir "$work"
cp "$dmg" "$work/"
cp "$notes" "$work/$name.md"
before=0
case $(curl -sSL -w '%{http_code}' -o "$work/appcast.xml" "$feed") in
  200) before=$(grep -c '<item>' "$work/appcast.xml" || true) ;;
  404) rm "$work/appcast.xml" ;; # the first release
  *) echo "Couldn't fetch $feed." >&2 && exit 1 ;;
esac
"$bin/generate_appcast" --account "$name" \
  --download-url-prefix "$repo/releases/download/v$version/" \
  --full-release-notes-url "$repo/releases/tag/v$version" \
  --embed-release-notes --maximum-deltas 0 \
  -o "$work/appcast.xml" "$work"
after=$(grep -c '<item>' "$work/appcast.xml")
if ! grep -q 'sparkle:edSignature=' "$work/appcast.xml" || [ "$after" -lt "$before" ]; then
  echo "generate_appcast made an appcast without a signature, or lost an update: $work/appcast.xml" >&2
  exit 1
fi
echo "$work/appcast.xml"
[ "$publish" = --publish ] || exit 0

# The DMG first, as the appcast sends every copy to it.
gh release create "v$version" "$dmg" --repo "$slug" --target "$(git -C "$root" rev-parse HEAD)" \
  --title "$name $version" --notes-file "$notes" --latest
gh release view "$feed_tag" --repo "$slug" > /dev/null 2>&1 ||
  gh release create "$feed_tag" --repo "$slug" --prerelease --latest=false --title "Updates" \
    --notes "The appcast every copy of $name checks for updates (app/branding/update-feed)."
gh release upload "$feed_tag" "$work/appcast.xml" --repo "$slug" --clobber
