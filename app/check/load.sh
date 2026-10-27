# The start page loads in the browser window, over the GPUI sidebar drawn in it, and quitting
# through AppKit closes the page and ends every Chromium process.

log="$out/load.log"
rm -f "$saved"
"$app/Contents/MacOS/Browser" < /dev/null > "$log" 2>&1 &
pid=$!
await_load
proof load.png
quit
echo "ok: $(grep '^browser: loaded' "$log" | cut -d' ' -f4-) loaded, quit cleanly. Window: $out/load.png"
