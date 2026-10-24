#!/bin/sh
# Builds build/Spike.app: the GPUI executable in a copy of our engine's Chromium.app, which holds
# Chrome's framework with the core in it, and Chromium's helper apps.
set -e
cd "$(dirname "$0")"
[ -e .deps/Chromium.app ] || ./setup.sh
cargo build --release

app=build/Spike.app
rm -rf "$app"
mkdir -p build
# A clone, so it's instant, and rebuilding the engine leaves a built spike as it was.
cp -cR "$(cd .deps/Chromium.app && pwd -P)" "$app"
rm "$app/Contents/MacOS/Chromium"
cp target/release/Spike "$app/Contents/MacOS/"
# The dev engine's framework and helpers load its component libraries from beside the app. These
# are links, so a rebuilt dev engine's core reaches the built spike at once.
rm -f build/*.dylib
engine_out=$(dirname "$(cd .deps/Chromium.app && pwd -P)")
if [ -e "$engine_out/libchrome_dll.dylib" ]; then
  ln -s "$engine_out"/*.dylib build/
fi

plist=$app/Contents/Info.plist
plutil -replace CFBundleExecutable -string Spike "$plist"
plutil -replace CFBundleIdentifier -string dev.browser.spike-core "$plist"
plutil -replace CFBundleName -string Spike "$plist"
plutil -replace CFBundleDisplayName -string Spike "$plist"
# The spike doesn't offer to open the owner's links and files.
plutil -remove CFBundleURLTypes "$plist"
plutil -remove CFBundleDocumentTypes "$plist"
# Chromium touches Bluetooth, the camera, the microphone, location and the local network for
# pages and for its own metrics. Without a usage description, macOS kills the app the first time
# it does, instead of asking.
plutil -replace NSBluetoothAlwaysUsageDescription -string "Websites can ask to use Bluetooth devices." "$plist"
plutil -replace NSCameraUsageDescription -string "Websites can ask to use the camera." "$plist"
plutil -replace NSMicrophoneUsageDescription -string "Websites can ask to use the microphone." "$plist"
plutil -replace NSLocationUsageDescription -string "Websites can ask for your location." "$plist"
plutil -replace NSLocationWhenInUseUsageDescription -string "Websites can ask for your location." "$plist"
plutil -replace NSLocalNetworkUsageDescription -string "Websites and casting can reach devices on your network." "$plist"
echo "$app"
