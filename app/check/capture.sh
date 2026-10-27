# ⇧⌘2 from the page opens capture mode over it, with the keyboard, and the pointer over an element
# lights the box the core hit tests. A click takes it, and the menu's Copy copies a picture of just
# that box to a pasteboard of the run's own and shows the toast with Save and Share. Escape and a
# second ⇧⌘2 leave capture mode and give the page the keyboard back. File › Capture Full Page copies
# a picture of the whole long page, past its view.

# A page longer than any view, with a box at a known place in it.
page='data:text/html;charset=utf-8,<title>loading</title>
<body style="margin:0;height:6000px">
<div style="position:absolute;left:300px;top:200px;width:200px;height:100px;background:teal"></div>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/capture.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "the capture test page didn't draw"
send click $page_point
if [ "$active" != 1 ]; then
  exec 3>&-
  quit
  echo "skip: capture. Browser wasn't the active app, so the page ignored the click."
  exit 0
fi

# ⇧⌘2 from the page: capture mode shows over it, with the keyboard. Sets $left and $top to where
# the page is in the window.
capture_mode() {
  send key 2 cmd shift
  expect "^browser: command capture via $1\$" "⇧⌘2 from the $1 didn't run Capture"
  expect '^browser: capture mode at ' "⇧⌘2 from the $1 didn't show capture mode"
  frame=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: capture mode at ' |
    sed 's/^browser: capture mode at //')
  left=$(echo "$frame" | cut -d' ' -f1)
  top=$(echo "$frame" | cut -d' ' -f2)
  # The window shows on the main queue's next turn, after the picture of the page is decoded.
  expect '^browser: capture shown at ' "capture mode's window didn't show"
}
# Points in the window, from $1 and $2 in the page.
in_page() {
  awk -v x="$1" -v y="$2" -v left="$left" -v top="$top" 'BEGIN { print x + left, y + top }'
}

capture_mode page
# The box is 200 by 100 at 300, 200 in the page.
send move $(in_page 400 250)
expect '^browser: capture element at [0-9.]* [0-9.]*: 300 200 200 100$' \
  "the pointer over the box didn't light it: $(grep '^browser: capture element' "$log" | tail -1)"
send click $(in_page 400 250)
expect '^browser: capture chose 300 200 200 100$' "a click on the box didn't take it"
# The menu is 10 in from the ring's left, level with its top; Copy is its first row, 50 to 90
# down.
send click $(in_page 442 266)
expect '^browser: capture run copy$' "the menu's Copy didn't run"
expect '^browser: tab [0-9]* captured [0-9]*: [0-9]*x[0-9]*, [0-9]* bytes$' \
  "Copy didn't get a picture of the box"
size=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: tab [0-9]* captured ' |
  sed 's/.*: \([0-9]*x[0-9]*\),.*/\1/')
case "$size" in
  200x100 | 400x200) ;;
  *) fail "the picture of the 200 by 100 box is $size" ;;
esac
expect '^browser: pasteboard png [0-9]* bytes, private$' \
  "Copy didn't copy the PNG to the run's pasteboard"
expect '^browser: capture left (copy)$' "capture mode didn't leave after Copy"
expect '^browser: toast shown Screenshot Copied to Clipboard \[save, share\]$' \
  "Copy didn't show its toast with Save and Share"
expect '^browser: capture hidden$' "capture mode's window didn't hide after Copy"
send state
in_browser RenderWidgetHostViewCocoa "after Copy"

# Escape leaves, and so does ⇧⌘2 again, from capture mode's own window.
capture_mode page
send state
[ "$key" = GPUIPanel ] || fail "capture mode doesn't have the keyboard: $key does"
send key escape
expect '^browser: capture left (escape)$' "escape didn't leave capture mode"
send state
in_browser RenderWidgetHostViewCocoa "after escape in capture mode"
capture_mode page
send key 2 cmd shift
expect '^browser: capture left (toggle)$' "a second ⇧⌘2 didn't leave capture mode"
send state
in_browser RenderWidgetHostViewCocoa "after a second ⇧⌘2"

# The whole page, beyond its view: at least its 6000 points high.
send menu "Capture Full Page"
expect '^browser: command capture-full-page via ui$' "File › Capture Full Page didn't run"
expect '^browser: toast shown Capture in Progress$' "Capture Full Page didn't say it's capturing"
expect '^browser: tab [0-9]* captured [0-9]*: [0-9]*x[0-9]*, [0-9]* bytes$' \
  "Capture Full Page didn't get a picture"
full=$(sed -n "/^browser: drive $step: /,\$p" "$log" | grep -m1 '^browser: tab [0-9]* captured ' |
  sed 's/.*: \([0-9]*x[0-9]*, [0-9]* bytes\)$/\1/')
height=$(echo "$full" | sed 's/^[0-9]*x\([0-9]*\),.*/\1/')
[ "$height" -ge 6000 ] || fail "the full page's picture is $full, not the page's 6000 points high"
expect '^browser: pasteboard png [0-9]* bytes, private$' \
  "Capture Full Page didn't copy the PNG to the run's pasteboard"
expect '^browser: toast shown Screenshot Copied to Clipboard \[save, share\]$' \
  "Capture Full Page didn't show its toast"

exec 3>&-
quit
echo "ok: capture. The box's picture is $size, the full page's $full."
