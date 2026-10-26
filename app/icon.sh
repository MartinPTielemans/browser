#!/bin/sh
# Compiles branding/'s Icon Composer icon into the Assets.car and .icns build.sh copies into the
# app, which are committed so the app builds without Xcode. Run it after changing the icon; it
# needs Xcode's actool. macOS 26 draws the four icon styles from Assets.car, and older ones show
# the .icns. The background and the stem keep their dark fill-specializations, or the ink stem
# disappears on the Dark style's ink tile, and the dot stays a layer of its own.
set -e
cd "$(dirname "$0")/branding"
name=$(cat name)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
xcrun actool "$name.icon" --compile "$tmp" --platform macosx --minimum-deployment-target 26.0 \
  --app-icon "$name" --output-partial-info-plist "$tmp/partial.plist" > /dev/null
cp "$tmp/Assets.car" "$tmp/$name.icns" .
