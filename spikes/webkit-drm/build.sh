#!/bin/sh
# Builds build/WebKitDRM.app. Its own bundle id keeps its WebKit data in
# ~/Library/WebKit/dev.browser.spike-webkit-drm, apart from Safari's.
set -e
cd "$(dirname "$0")"
app=build/WebKitDRM.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp hook.js "$app/Contents/Resources/"
clang -fobjc-arc -O2 -Wall -mmacosx-version-min=14.0 -framework Cocoa -framework WebKit \
  main.m -o "$app/Contents/MacOS/WebKitDRM"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>WebKitDRM</string>
  <key>CFBundleIdentifier</key><string>dev.browser.spike-webkit-drm</string>
  <key>CFBundleName</key><string>WebKitDRM</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.0.0</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app"
echo "built $app"
