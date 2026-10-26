# The menu bar's Extensions menu lists the turned-on extensions of the shown Space's profile, by
# name, over Pin to URL Field and Manage Extensions…. Choosing one runs its action on the tab shown,
# as a click on its toolbar button does in Chrome: one without a popup hears the click, and one
# with a popup opens it with the keyboard, which escape closes, giving the browser window the
# keyboard back. Pin to URL Field's items pin an extension's action at the URL field's end, in the
# order pinned, showing its badge for the tab shown as it changes. A click on a pinned action runs
# it, its popup hanging from it, and its right-click menu unpins it, opens its options or its
# details on Chrome's extensions page, or removes it once the user confirms. The actions that don't
# fit are behind an overflow button in the last place, whose menu runs them. Manage Extensions…
# opens Chrome's extensions page. An extension's keyboard commands run with the page focused and
# with the UI, but a key it suggests doesn't take one of the browser's.

# Three unpacked extensions of the check's own, loaded as a developer loads one. A click on the
# first gives the tab it's on a badge, then opens its page, and its command ⇧⌘Y opens another. The
# second's popup gives it a badge on every tab, and ⌥⇧P opens it. The third has a popup and
# options, and suggests ⌘J, Jump to Selection's key, which Chrome gives away, for a command. The
# check removes it, and the next loads it again, as Chrome leaves an unpacked extension's folder.
extensions="$PWD/$out/extensions"
script='chrome.action.onClicked.addListener(async tab => {
  await chrome.action.setBadgeText({tabId: tab.id, text: "7"});
  chrome.tabs.create({url: "clicked.html"});
});
chrome.commands.onCommand.addListener(() => chrome.tabs.create({url: "command.html"}));'
# Chrome keeps running the script it first registered for an extension, so a changed script is a
# new extension, in a folder of its own.
clicked_dir="$extensions/clicked-$(printf %s "$script" | cksum | cut -d' ' -f1)"
mkdir -p "$clicked_dir" "$extensions/popup" "$extensions/third"
printf '%s\n' "$script" > "$clicked_dir/background.js"
cat > "$clicked_dir/manifest.json" << 'EOF'
{"manifest_version": 3, "name": "Check Clicked", "version": "1", "action": {},
 "background": {"service_worker": "background.js"},
 "commands": {"hello": {"suggested_key": {"default": "Ctrl+Shift+Y"}, "description": "Hello"}}}
EOF
echo '<title>clicked</title>' > "$clicked_dir/clicked.html"
echo '<title>command</title>' > "$clicked_dir/command.html"
cat > "$extensions/popup/manifest.json" << 'EOF'
{"manifest_version": 3, "name": "Check Popup", "version": "1",
 "action": {"default_popup": "popup.html"},
 "commands": {"_execute_action": {"suggested_key": {"default": "Alt+Shift+P"}}}}
EOF
echo '<body style="width:240px;height:120px;display:grid;place-items:center;font:16px system-ui">
the popup<script src="popup.js"></script></body>' > "$extensions/popup/popup.html"
echo 'chrome.action.setBadgeText({text: "1"});' > "$extensions/popup/popup.js"
cat > "$extensions/third/manifest.json" << 'EOF'
{"manifest_version": 3, "name": "Check Third", "version": "1",
 "action": {"default_popup": "popup.html"}, "options_ui": {"page": "options.html"},
 "commands": {"jump": {"suggested_key": {"default": "Ctrl+J"}, "description": "Jump"}}}
EOF
echo '<body style="width:240px;height:120px;display:grid;place-items:center;font:16px system-ui">
the third popup</body>' > "$extensions/third/popup.html"
echo '<body style="font:16px system-ui">the third options</body>' > "$extensions/third/options.html"
page='data:text/html,<title>loading</title>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/extensions.log"
step=0
rm -f "$saved"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" \
  --load-extension="$clicked_dir,$extensions/popup,$extensions/third" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: extensions menu [a-p]* [a-p]* [a-p]*$' ||
  fail "the Extensions menu didn't list the three extensions: $(grep '^browser: extensions' "$log")"
# By name.
ids=$(grep -m1 '^browser: extensions menu [a-p]* ' "$log" | cut -d' ' -f4-)
clicked=$(echo "$ids" | cut -d' ' -f1)
popup=$(echo "$ids" | cut -d' ' -f2)
third=$(echo "$ids" | cut -d' ' -f3)
pinned=$(grep '^browser: pinned extensions' "$log" | tail -1)
echo "$pinned" | grep -q '^browser: pinned extensions [0-9]* $' ||
  fail "an extension started pinned: $pinned"
first=$(today | cut -d' ' -f1)
# A just-installed extension's script adds its listener once it has run, and a click before then
# goes unheard, so on a fresh profile it's chosen until it's heard.
heard="^browser: loaded [0-9]* chrome-extension://$clicked/clicked.html\$"
for _ in $(seq 10); do
  send menu "Check Clicked"
  expect "^browser: run extension $clicked ran\$" \
    "Check Clicked didn't run from the Extensions menu"
  for _ in $(seq 20); do
    none_since "$heard" || break 2
    sleep 0.1
  done
done
none_since "$heard" && fail "Check Clicked didn't hear its click: it didn't open its page"
# Pinned in the order pinned, on the extension's page's tab, where Check Clicked has no badge.
send menu "Pin to URL Field › Check Popup"
expect "^browser: pin extension $popup\$" "Pin to URL Field › Check Popup didn't reach the UI"
expect "^browser: extension actions $popup\$" "the pinned Check Popup didn't show in the URL field"
send menu "Pin to URL Field › Check Clicked"
expect "^browser: extension actions $popup $clicked\$" \
  "the pinned Check Clicked didn't show after Check Popup"
# Its badge is the first tab's, which shows it once that tab is shown.
index=$(today | tr ' ' '\n' | grep -n "^$first\$" | cut -d: -f1)
send click $(row $((index - 1)))
expect "^browser: selected $first\( in .*\)\{0,1\}\$" \
  "a click on the first tab's row didn't show it"
expect "^browser: extension actions $popup $clicked:7\$" \
  "Check Clicked's badge for the first tab didn't show with it"
../tools/window-screenshot.sh "$pid" "$out/extension-actions.png" || fail "no window to capture"
# The actions sit at the URL field's end, 30 apart: their centres, the 228-wide sidebar's field
# ending at 220, 3 in from its end.
first_place="172 64"
second_place="202 64"
send click $first_place
expect "^browser: run extension $popup ran\$" "a click on Check Popup's action didn't run it"
expect "^browser: extension popup $popup\$" "Check Popup's popup didn't show"
expect "^browser: extension actions $popup:1 $clicked:7\$" \
  "the badge Check Popup's popup gave it didn't show at once"
send state
[ "$key" != BrowserCoreNSWindow ] || fail "the extension's popup didn't take the keyboard"
../tools/window-screenshot.sh "$pid" "$out/extension-popup.png" || fail "no window to capture"
send key escape
expect '^browser: extension popup none$' "escape didn't close the extension's popup"
send state
[ "$key" = BrowserCoreNSWindow ] ||
  fail "the browser window didn't get the keyboard back from the popup: it's in $key"
# The Extensions menu runs a pinned one too, its popup hanging from its action.
send menu "Check Popup"
expect "^browser: extension popup $popup\$" "Check Popup's popup didn't show from the menu"
send key escape
expect '^browser: extension popup none$' "escape didn't close the extension's popup"
# A right-click on an action shows its menu: Manage Extension opens its details, and Unpin
# unpins it. Check Clicked has no options.
post rightclick $second_place
expect '^browser: menu shown Unpin, Remove, Manage Extension$' \
  "a right-click on Check Clicked's action didn't show its menu"
send choose Manage Extension
expect "^browser: loaded [0-9]* chrome://extensions/?id=$clicked\$" \
  "Manage Extension didn't open Check Clicked's details"
# A third doesn't fit the field, so the second place is the overflow button's, whose menu runs the
# actions behind it, their popups hanging from it.
send menu "Pin to URL Field › Check Third"
expect "^browser: extension actions $popup:1 $clicked $third\$" \
  "the pinned Check Third didn't show after Check Clicked"
../tools/window-screenshot.sh "$pid" "$out/extension-overflow.png" || fail "no window to capture"
post click $second_place
expect '^browser: menu shown Check Clicked, Check Third$' \
  "a click on the overflow button didn't show the actions behind it"
send choose Check Third
expect "^browser: run extension $third ran\$" "Check Third didn't run from the overflow button"
expect "^browser: extension popup $third\$" "Check Third's popup didn't show"
../tools/window-screenshot.sh "$pid" "$out/extension-overflow-popup.png" ||
  fail "no window to capture"
send key escape
expect '^browser: extension popup none$' "escape didn't close Check Third's popup"
# Check Clicked unpinned, Check Third has a place of its own, where a right-click unpins it.
send menu "Pin to URL Field › Check Clicked"
expect "^browser: unpin extension $clicked\$" "Pin to URL Field › Check Clicked didn't unpin it"
expect "^browser: extension actions $popup:1 $third\$" \
  "unpinning Check Clicked didn't give Check Third its place"
post rightclick $second_place
expect '^browser: menu shown Unpin, Options, Remove, Manage Extension$' \
  "a right-click on Check Third's action didn't show its menu"
# Chrome shows options that don't ask for a tab in a dialog on its extensions page.
send choose Options
expect "^browser: loaded [0-9]* chrome://extensions/?options=$third\$" \
  "Options didn't open Check Third's options"
post rightclick $second_place
expect '^browser: menu shown Unpin, Options, Remove, Manage Extension$' \
  "a right-click on Check Third's action didn't show its menu again"
send choose Unpin
expect "^browser: unpin extension $third\$" "Unpin didn't reach the UI"
expect "^browser: extension actions $popup:1\$" "Unpin didn't take Check Third's action away"
send menu "Pin to URL Field › Check Popup"
expect "^browser: unpin extension $popup\$" "Pin to URL Field › Check Popup didn't unpin it"
expect '^browser: extension actions $' "unpinning the last action left it in the URL field"
# Their keys, as Chrome gave them: those the extensions suggested, as no one set any.
grep -q "^browser: extension keys $clicked hello=shift-cmd-y suggested\$" "$log" &&
  grep -q "^browser: extension keys $popup _execute_action=alt-shift-p suggested\$" "$log" &&
  grep -q "^browser: extension keys $third jump=cmd-j suggested\$" "$log" ||
  fail "the extensions' keys weren't the ones they suggested: $(grep 'extension keys' "$log")"
# Keyboard commands, on the first tab's page.
index=$(today | tr ' ' '\n' | grep -n "^$first\$" | cut -d: -f1)
send click $(row $((index - 1)))
expect "^browser: selected $first\( in .*\)\{0,1\}\$" \
  "a click on the first tab's row didn't show it"
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the first tab's page"
send key y cmd shift
expect "^browser: extension command $clicked hello ran via page\$" \
  "⇧⌘Y with the page focused didn't run Check Clicked's command"
expect "^browser: loaded [0-9]* chrome-extension://$clicked/command.html\$" \
  "Check Clicked didn't hear its command from the page"
send click $sidebar_point
in_browser AccessKitSubclassOfGPUIView "after a click on the sidebar"
send key y cmd shift
expect "^browser: extension command $clicked hello ran via ui\$" \
  "⇧⌘Y with the UI focused didn't run Check Clicked's command"
expect "^browser: loaded [0-9]* chrome-extension://$clicked/command.html\$" \
  "Check Clicked didn't hear its command from the UI"
# An action's key runs it as a click on its button does, its popup with the keyboard.
send key p alt shift
expect "^browser: extension command $popup _execute_action ran via ui\$" \
  "⌥⇧P with the UI focused didn't run Check Popup's action"
expect "^browser: extension popup $popup\$" "⌥⇧P with the UI focused didn't show the popup"
send key escape
expect '^browser: extension popup none$' "escape didn't close the extension's popup"
send click $page_point
in_browser RenderWidgetHostViewCocoa "after a click on the command page"
send key p alt shift
expect "^browser: extension command $popup _execute_action ran via page\$" \
  "⌥⇧P with the page focused didn't run Check Popup's action"
expect "^browser: extension popup $popup\$" "⌥⇧P with the page focused didn't show the popup"
../tools/window-screenshot.sh "$pid" "$out/extension-key-popup.png" || fail "no window to capture"
send key escape
expect '^browser: extension popup none$' "escape didn't close the extension's popup"
send menu "Manage Extensions…"
expect '^browser: command manage-extensions via ui$' "Manage Extensions… didn't reach the UI"
expect '^browser: loaded [0-9]* chrome://extensions/$' \
  "Manage Extensions… didn't open Chrome's extensions page"
../tools/window-screenshot.sh "$pid" "$out/extensions.png" || fail "no window to capture"
# A suggested key that's one of the browser's stays the browser's, with the UI focused and the
# page.
send click $sidebar_point
send key j cmd
expect '^browser: command jump-to-selection via ui$' "⌘J with the UI focused didn't reach the UI"
none_since "^browser: extension command $third " ||
  fail "⌘J with the UI focused ran Check Third's command, which suggested it"
send click $page_point
send key j cmd
expect '^browser: command jump-to-selection via page$' \
  "⌘J with the page focused didn't reach the UI"
none_since "^browser: extension command $third " ||
  fail "⌘J with the page focused ran Check Third's command, which suggested it"
# Remove asks first, then takes the extension from the menus and the URL field. Alone, its action
# is at the field's end.
send menu "Pin to URL Field › Check Third"
expect "^browser: extension actions $third\$" "the pinned Check Third didn't show again"
post rightclick $second_place
expect '^browser: menu shown Unpin, Options, Remove, Manage Extension$' \
  "a right-click on Check Third's action didn't show its menu to remove it"
send choose Remove
expect '^browser: dialog shown Remove “Check Third”?$' "Remove didn't ask first"
../tools/window-screenshot.sh "$pid" "$out/extension-remove.png" || fail "no window to capture"
send key return
expect "^browser: remove extension $third removed\$" "Remove didn't remove Check Third"
expect "^browser: extensions menu $clicked $popup\$" \
  "the removed Check Third stayed in the Extensions menu"
expect '^browser: extension actions $' "the removed Check Third's action stayed"
quit_by_key
await '^browser: command quit via ' || fail "⌘Q didn't quit"
exited
echo "ok: the Extensions menu lists the profile's extensions by name, and choosing one runs its action on the tab shown: one without a popup hears the click, and one with a popup shows it with the keyboard, which escape closes, giving the browser window the keyboard back. Pin to URL Field pins and unpins them, and the pinned actions show at the URL field's end in the order pinned, with their badges for the tab shown as they change. A click on one runs it, with its popup hanging from it, and its right-click menu unpins it, opens its options and its details, and removes it after asking. The ones that don't fit are behind the overflow button, whose menu runs them. Manage Extensions… opens Chrome's extensions page. Their keyboard commands and their actions' keys run from the page and the UI, but one they suggest that's the browser's stays the browser's. Windows: $out/extension-actions.png, $out/extension-popup.png, $out/extension-overflow.png, $out/extension-overflow-popup.png, $out/extension-key-popup.png, $out/extensions.png, $out/extension-remove.png"
