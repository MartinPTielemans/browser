# A crash leaves a minidump in the profile's Crashpad folder: --crash-after-start crashes the
# browser once it has started and Crashpad's handler is up, the handler writes the report, and no
# Chromium process outlives the crash.

log="$out/crash.log"
crashpad="$(dirname "$saved")/Crashpad"
rm -rf "$crashpad"
"$app/Contents/MacOS/Browser" --crash-after-start < /dev/null > "$log" 2>&1 &
pid=$!
await '^browser: crash test$' || fail "the app didn't start its crash test"
for _ in $(seq 20); do
  kill -0 "$pid" 2>/dev/null || break
  sleep 0.5
done
kill -0 "$pid" 2>/dev/null && fail "the app didn't crash"
status=0
wait "$pid" || status=$?
[ "$status" -ne 0 ] || fail "the app exited cleanly from its crash test"
# The handler writes the report as the browser dies, and may still be at it.
for _ in $(seq 20); do
  dump=$(find "$crashpad" -name '*.dmp' 2>/dev/null | head -1)
  [ -z "$dump" ] || break
  sleep 0.5
done
[ -n "$dump" ] || fail "Crashpad wrote no minidump in $crashpad"
head -c 4 "$dump" | grep -q '^MDMP$' || fail "$dump isn't a minidump"
for _ in $(seq 20); do
  pgrep -f "$app/Contents" > /dev/null || break
  sleep 0.5
done
pgrep -f "$app/Contents" > /dev/null && fail "Chromium processes outlived the crash"
echo "ok: the crash left a minidump of $(wc -c < "$dump" | tr -d ' ') bytes in ${dump#"$crashpad"/}"
