# #1120 diagnostic, not for merging: find.sh's ⌥⌘I, resize, F12, many times over, saying where
# each F12 went.
page='data:text/html,<title>loading</title><p>hello</p>
<script>requestAnimationFrame(() => setTimeout(() => document.title = "ready"));</script>'
log="$out/zzf12.log"
step=0
rm -f "$saved" "$launch_frame"
BROWSER_DRIVE=1 BROWSER_URL="$page" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* ready$' || fail "no page"
send click $page_point
toggles() { grep -c '^browser: command devtools via page$' "$log" || true; }
# Waits up to 10 s for $1 toggles.
reached() {
  for _ in $(seq 100); do
    [ "$(toggles)" -ge "$1" ] && return 0
    sleep 0.1
  done
  return 1
}
xfocus() {
  command -v xdotool > /dev/null || return 0
  w=$(xdotool getwindowfocus 2> /dev/null || true)
  echo "x focus $w $(xdotool getwindowname "$w" 2> /dev/null || true) $(xdotool getwindowgeometry "$w" 2> /dev/null | tr '\n' ' ')"
}
lost=0
note() { echo "$*" | tee -a "$out/zzf12-cases.txt"; }
rm -f "$out/zzf12-cases.txt"
for i in $(seq 16); do
  send click $page_point
  n=$(toggles)
  send key i cmd alt
  reached $((n + 1)) || fail "case $i: ⌥⌘I didn't reach"
  case $((i % 4)) in
    0) send resize 1100 700 ;;
    1) send resize 1180 760 ;;
    2) ;;
    3) sleep 0.3 ;;
  esac
  at=$(wc -l < "$log")
  send key f12
  if reached $((n + 2)); then
    note "case $i: F12 reached: $(tail -n "+$((at + 1))" "$log" | grep 'diag F12' | tr '\n' ';')"
  else
    lost=$((lost + 1))
    note "case $i: F12 LOST: $(tail -n "+$((at + 1))" "$log" | grep 'diag F12' | tr '\n' ';') $(xfocus)"
    send layout
    tail -n "+$((at + 1))" "$log" | grep -v '^\[' | sed "s/^/case $i lost: /" >> "$out/zzf12-cases.txt"
    ../tools/window-screenshot.sh "$pid" "$out/zzf12-lost-$i.png" || true
    at=$(wc -l < "$log")
    send key f12
    if reached $((n + 2)); then
      note "case $i: a second F12 reached: $(tail -n "+$((at + 1))" "$log" | grep 'diag F12' | tr '\n' ';')"
    else
      note "case $i: a second F12 was lost too: $(tail -n "+$((at + 1))" "$log" | grep 'diag F12' | tr '\n' ';')"
      send click $page_point
      send key i cmd alt
      reached $((n + 2)) || fail "case $i: couldn't close DevTools"
    fi
  fi
  # As find.sh's next step, whatever F12 did.
  sleep 1
done
exec 3>&-
quit
[ "$lost" = 0 ] || fail "$lost of 16 F12s lost"
echo "ok: zzf12, every F12 reached the UI"
