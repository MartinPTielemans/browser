#!/bin/sh
# Builds build/Spike.app: the GPUI executable and Ardia's engine, with Chromium's helper apps
# inside it.
set -e
cd "$(dirname "$0")"
[ -f .deps/ardia/tree ] || ./setup.sh
cargo build --release

app=build/Spike.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks"
cp target/release/Spike "$app/Contents/MacOS/"
# A clone, so it's instant. Ardia strips the engine after signing it, so its signature no longer
# holds, and the helpers' hardened runtime won't load it. Everything is signed again ad hoc, the
# way Ardia's own build does (packages/ardiacore/scripts/stage-framework.sh).
cp -cR ".deps/ardia/Chromium Framework.framework" "$app/Contents/Frameworks/"
engine=$(cd "$app/Contents/Frameworks/Chromium Framework.framework/Versions/Current" && pwd -P)
for code in "$engine"/Libraries/*.dylib "$engine"/Helpers/*.app "$engine"/Helpers/chrome_crashpad_handler \
  "$engine"/Helpers/app_mode_loader "$engine"/Helpers/web_app_shortcut_copier; do
  [ -e "$code" ] && codesign --force --sign - "$code"
done
codesign --force --sign - "$app/Contents/Frameworks/Chromium Framework.framework"

# Chromium touches Bluetooth, the camera, the microphone, location and the local network for
# pages and for its own metrics. Without a usage description, macOS kills the app the first time
# it does, instead of asking.
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>Spike</string>
  <key>CFBundleIdentifier</key><string>dev.browser.spike-ardia</string>
  <key>CFBundleName</key><string>Spike</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.0.0</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSBluetoothAlwaysUsageDescription</key><string>Websites can ask to use Bluetooth devices.</string>
  <key>NSCameraUsageDescription</key><string>Websites can ask to use the camera.</string>
  <key>NSMicrophoneUsageDescription</key><string>Websites can ask to use the microphone.</string>
  <key>NSLocationUsageDescription</key><string>Websites can ask for your location.</string>
  <key>NSLocationWhenInUseUsageDescription</key><string>Websites can ask for your location.</string>
  <key>NSLocalNetworkUsageDescription</key><string>Websites and casting can reach devices on your network.</string>
</dict>
</plist>
PLIST

codesign --force --sign - "$app"
echo "$app"
