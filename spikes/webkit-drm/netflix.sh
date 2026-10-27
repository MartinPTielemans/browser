#!/bin/sh
# For the owner: opens Netflix in the spike, in front, with sound, so you can sign in and press
# play. Its cookies stay in the spike's own "netflix" store, apart from Safari's, and the log goes
# to ~/.cache/browser-spike-webkit/netflix.log. `./netflix.sh forget` deletes that store.
# SPIKE_UA=default tries WKWebView's own user agent instead of Safari's.
set -e
cd "$(dirname "$0")"
[ -x build/WebKitDRM.app/Contents/MacOS/WebKitDRM ] || ./build.sh
mkdir -p ~/.cache/browser-spike-webkit
export SPIKE_STORE=netflix
if [ "$1" = forget ]; then exec build/WebKitDRM.app/Contents/MacOS/WebKitDRM forget; fi
SPIKE_UA="${SPIKE_UA:-safari}" SPIKE_SOUND=1 SPIKE_ACTIVATE=1 SPIKE_URL=https://www.netflix.com/login \
  build/WebKitDRM.app/Contents/MacOS/WebKitDRM 2> ~/.cache/browser-spike-webkit/netflix.log
echo "log: ~/.cache/browser-spike-webkit/netflix.log (look for page-asks, page-granted and playing)"
