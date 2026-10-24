#!/bin/sh
# Builds build/Spike.app: the GPUI executable, the CEF framework and Chromium's helper apps.
set -e
cd "$(dirname "$0")"
[ -f .deps/cef/build/libcef_dll_wrapper/libcef_dll_wrapper.a ] || ./setup.sh
cargo build --release

cef=.deps/cef
app=build/Spike.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks"
cp target/release/Spike "$app/Contents/MacOS/"
cp -R "$cef/Release/Chromium Embedded Framework.framework" "$app/Contents/Frameworks/"

# plist <path> <executable> <bundle id> [extra keys]
plist() {
  cat > "$1" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$2</string>
  <key>CFBundleIdentifier</key><string>$3</string>
  <key>CFBundleName</key><string>$2</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.0.0</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  $4
</dict>
</plist>
PLIST
}
# Chromium touches Bluetooth, the camera, the microphone, location and the local network for
# pages and for its own metrics. Without a usage description, macOS kills the app the first time
# it does, instead of asking.
plist "$app/Contents/Info.plist" Spike dev.browser.spike '<key>NSPrincipalClass</key><string>SpikeApplication</string>
  <key>NSBluetoothAlwaysUsageDescription</key><string>Websites can ask to use Bluetooth devices.</string>
  <key>NSCameraUsageDescription</key><string>Websites can ask to use the camera.</string>
  <key>NSMicrophoneUsageDescription</key><string>Websites can ask to use the microphone.</string>
  <key>NSLocationUsageDescription</key><string>Websites can ask for your location.</string>
  <key>NSLocationWhenInUseUsageDescription</key><string>Websites can ask for your location.</string>
  <key>NSLocalNetworkUsageDescription</key><string>Websites and casting can reach devices on your network.</string>'

# Chromium finds its helpers by name: "<main executable> Helper (<kind>).app".
mkdir -p build/helper
clang++ -std=c++20 -fobjc-arc -O2 -DNDEBUG -I"$cef" host/helper.mm \
  "$cef/build/libcef_dll_wrapper/libcef_dll_wrapper.a" -framework Cocoa -o build/helper/helper
for kind in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  name="Spike Helper$kind"
  id=$(echo "dev.browser.spike.helper$kind" | tr -d '()' | tr ' ' '.' | tr '[:upper:]' '[:lower:]')
  contents="$app/Contents/Frameworks/$name.app/Contents"
  mkdir -p "$contents/MacOS"
  cp build/helper/helper "$contents/MacOS/$name"
  plist "$contents/Info.plist" "$name" "$id" '<key>LSUIElement</key><true/>'
done

codesign --force --sign - "$app/Contents/Frameworks/Chromium Embedded Framework.framework"
for helper in "$app"/Contents/Frameworks/*.app; do codesign --force --sign - "$helper"; done
codesign --force --sign - "$app"
echo "$app"
