# The menu bar's Extensions menu lists the turned-on extensions of the shown Space's profile, by
# name, over Manage Extensions…. Choosing one runs its action on the tab shown, as a click on its
# toolbar button does in Chrome: one without a popup hears the click, and one with a popup opens it
# under the URL field, with the keyboard, and escape closes it, giving the browser window the
# keyboard back. Manage Extensions… opens Chrome's extensions page in a new tab.

# Two unpacked extensions of the check's own, loaded as a developer loads one.
extensions="$PWD/$out/extensions"
mkdir -p "$extensions/clicked" "$extensions/popup"
cat > "$extensions/clicked/manifest.json" << 'EOF'
{"manifest_version": 3, "name": "Check Clicked", "version": "1", "action": {},
 "background": {"service_worker": "background.js"}}
EOF
echo 'chrome.action.onClicked.addListener(() => chrome.tabs.create({url: "clicked.html"}));' \
  > "$extensions/clicked/background.js"
echo '<title>clicked</title>' > "$extensions/clicked/clicked.html"
cat > "$extensions/popup/manifest.json" << 'EOF'
{"manifest_version": 3, "name": "Check Popup", "version": "1",
 "action": {"default_popup": "popup.html"}}
EOF
echo '<body style="width:240px;height:120px;display:grid;place-items:center;font:16px system-ui">
the popup</body>' > "$extensions/popup/popup.html"
page='data:text/html,<title>loading</title>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/extensions.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" \
  --load-extension="$extensions/clicked,$extensions/popup" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: extensions menu [a-p]* [a-p]*$' ||
  fail "the Extensions menu didn't list the two extensions: $(grep '^browser: extensions' "$log")"
ids=$(grep -m1 '^browser: extensions menu [a-p]* ' "$log" | cut -d' ' -f4-)
# By name.
clicked=${ids% *}
popup=${ids#* }
send menu "Check Clicked"
expect "^browser: run extension $clicked ran\$" "Check Clicked didn't run from the Extensions menu"
expect "^browser: loaded [0-9]* chrome-extension://$clicked/clicked.html\$" \
  "Check Clicked didn't hear its click: it didn't open its page"
send menu "Check Popup"
expect "^browser: run extension $popup ran\$" "Check Popup didn't run from the Extensions menu"
expect "^browser: extension popup $popup\$" "Check Popup's popup didn't show"
send state
[ "$key" != BrowserCoreNSWindow ] || fail "the extension's popup didn't take the keyboard"
../tools/window-screenshot.sh "$pid" "$out/extension-popup.png" || fail "no window to capture"
send key escape
expect '^browser: extension popup none$' "escape didn't close the extension's popup"
send state
[ "$key" = BrowserCoreNSWindow ] ||
  fail "the browser window didn't get the keyboard back from the popup: it's in $key"
send menu "Manage Extensions…"
expect '^browser: command manage-extensions via ui$' "Manage Extensions… didn't reach the UI"
expect '^browser: loaded [0-9]* chrome://extensions/$' \
  "Manage Extensions… didn't open Chrome's extensions page"
../tools/window-screenshot.sh "$pid" "$out/extensions.png" || fail "no window to capture"
quit_by_key
await '^browser: command quit via ' || fail "⌘Q didn't quit"
exited
echo "ok: the Extensions menu lists the profile's extensions by name, and choosing one runs its action on the tab shown: one without a popup hears the click, and one with a popup shows it with the keyboard, which escape closes, giving the browser window the keyboard back. Manage Extensions… opens Chrome's extensions page. Windows: $out/extension-popup.png, $out/extensions.png"
