#!/bin/sh
# Copies the app `app/build.sh` made to a folder the owner uses it from: ~/.cache/browser-try, or
# the folder named. It gets its own profile and bundle id, so the checks, which delete what the
# dev build's profile saved and stop what runs from app/build, never reach it. Run it again to
# update the copy; the profile stays. The copy's file takes the product's name, since Finder, the
# Dock and ⌘-Tab name an app by its file.
# The dev engine's framework loads its libraries from beside the app, so they're copied there.
# The first launch after that takes a minute or two while macOS checks each new library.
# Usage: tools/try-copy.sh [folder]
set -e
root="$(cd "$(dirname "$0")/../app" && pwd)"
app=$root/build
name=$(cat "$root/branding/name")
to=${1:-$HOME/.cache/browser-try}
# Browser.app is the copy's file from before it took the name.
for copy in "$name.app" Browser.app; do
  if pgrep -f "$to/$copy/Contents" > /dev/null; then
    echo "The copy in $to is running. Quit it first." >&2
    exit 1
  fi
done
mkdir -p "$to"
rm -rf "$to/$name.app" "$to/Browser.app"
ditto "$app/Browser.app" "$to/$name.app"
if [ -e "$app/libchrome_dll.dylib" ]; then
  rsync -aL "$app"/*.dylib "$to/"
fi
plist=$to/$name.app/Contents/Info.plist
plutil -replace CFBundleIdentifier -string dev.browser.try "$plist"
plutil -replace LSEnvironment.BROWSER_PROFILE_DIR -string \
  "$HOME/Library/Application Support/Browser" "$plist"
echo "open $to/$name.app"
