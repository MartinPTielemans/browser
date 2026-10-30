# View › Appearance gives a page its choice as prefers-color-scheme as it changes, and Automatic
# gives back the system's: on the Mac through the app's appearance, which the host sets, and off it
# through the core (bc_set_appearance). Each choice changes the scheme the one before gave, so the
# page says so.

serve
log="$out/appearance.log"
step=0
BROWSER_DRIVE=1 BROWSER_URL="${site}scheme.html" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
await '^browser: title [0-9]* scheme ' || fail "the scheme page didn't say its scheme"
system=$(grep '^browser: title [0-9]* scheme ' "$log" | tail -1 | sed 's/.* //')
other=dark
[ "$system" = light ] || other=light
for choice in $other $system $other automatic; do
  case $choice in
  dark) item=Dark scheme=dark ;;
  light) item=Light scheme=light ;;
  *) item=Automatic scheme=$system ;;
  esac
  send menu "Appearance › $item"
  expect "^browser: title [0-9]* scheme $scheme\$" \
    "View › Appearance › $item didn't make the page's prefers-color-scheme $scheme"
done
exec 3>&-
quit
echo "ok: View › Appearance gives the page its choice as prefers-color-scheme as it changes, and Automatic the system's ($system)"
