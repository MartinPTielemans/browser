#!/bin/sh
# Builds build/Browser.app: the GPUI executable in a copy of our engine's Chromium.app, which holds
# Chrome's framework with the core in it, and Chromium's helper apps. `build.sh prod` builds the
# copy for daily use instead, which tools/install.sh installs: the user's profile, the real
# Keychain and the product's bundle id, on the Release engine.
set -e
cd "$(dirname "$0")"
[ -e .deps/Chromium.app ] || ./setup.sh
engine_out=$(dirname "$(cd .deps/Chromium.app && pwd -P)")
id=dev.browser.app
features=
if [ "${1:-}" = prod ]; then
  # The dev engine stops the browser on a failed DCHECK, where Release goes on.
  if [ -e "$engine_out/libchrome_dll.dylib" ]; then
    echo "A prod build needs the Release engine: ./setup.sh" >&2
    exit 1
  fi
  id=$(cat branding/id)
  features="--features prod"
fi
cargo build --release --bin Browser $features

app=build/Browser.app
rm -rf "$app"
mkdir -p build
# A clone, so it's instant, and rebuilding the engine leaves a built app as it was.
cp -cR "$(cd .deps/Chromium.app && pwd -P)" "$app"
rm "$app/Contents/MacOS/Chromium"
cp target/release/Browser "$app/Contents/MacOS/"
# The dev engine's framework and helpers load its component libraries from beside the app. These
# are links, so a rebuilt dev engine's core reaches the built app at once.
rm -f build/*.dylib
if [ -e "$engine_out/libchrome_dll.dylib" ]; then
  ln -s "$engine_out"/*.dylib build/
fi

plist=$app/Contents/Info.plist
plutil -replace CFBundleExecutable -string Browser "$plist"
plutil -replace CFBundleIdentifier -string "$id" "$plist"
# The name and icon the user sees, from branding/ (icon.sh compiles the icon). Finder, the Dock
# and ⌘-Tab name the app by its file, though, so this one shows as Browser there.
name=$(cat branding/name)
plutil -replace CFBundleName -string "$name" "$plist"
plutil -replace CFBundleDisplayName -string "$name" "$plist"
rm "$app/Contents/Resources/app.icns"
cp branding/Assets.car "branding/$name.icns" "$app/Contents/Resources/"
plutil -replace CFBundleIconFile -string "$name" "$plist"
plutil -replace CFBundleIconName -string "$name" "$plist"
# Chromium's translations of the strings below name Chromium, and macOS shows a translation over
# Info.plist's string. Emptied, they leave the English ones below in every language.
for strings in "$app"/Contents/Resources/*.lproj/InfoPlist.strings; do
  plutil -create binary1 "$strings"
done
# The prod build opens web links and files, so it can be the default browser: http, https and file
# URLs, and the documents Chrome views, such as .html. Not what's Chromium's own, which the core
# doesn't open: its chromium:// scheme, its extension and shortcut files and the types it exports
# for them, and the sign-in sheets apps ask the default browser for.
python3 -I - "$plist" << 'EOF'
import plistlib, sys
with open(sys.argv[1], "rb") as f:
    info = plistlib.load(f)
info["CFBundleURLTypes"] = [
    kind for kind in info.get("CFBundleURLTypes", [])
    if set(kind["CFBundleURLSchemes"]) <= {"http", "https", "file"}
]
info["CFBundleDocumentTypes"] = [
    kind for kind in info.get("CFBundleDocumentTypes", [])
    if not any(name.startswith("org.chromium.") for name in kind.get("LSItemContentTypes", []))
]
info.pop("UTExportedTypeDeclarations", None)
info.pop("ASWebAuthenticationSessionWebBrowserSupportCapabilities", None)
with open(sys.argv[1], "wb") as f:
    plistlib.dump(info, f)
EOF
# A dev build offers none of it: every lane's build registers with the system too, and none may
# show in its list of browsers or catch the owner's links.
if [ -z "$features" ]; then
  plutil -remove CFBundleURLTypes "$plist"
  plutil -remove CFBundleDocumentTypes "$plist"
fi
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
