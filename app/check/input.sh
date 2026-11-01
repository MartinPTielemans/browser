# Input methods on Windows (#454): typing through Microsoft's Japanese input method, which Check on
# Windows adds to the runner's languages, in a page's textarea and in ilo's own fields: the command
# bar, the find bar and Settings' search. In each, romaji compose hiragana inline, Escape cancels
# what's composing, space converts にほんご to 日本語 and Enter commits it without doing anything else.
# In ilo's fields the input method is asked to compose at the field's caret, which moves on past
# what was committed. Captures of each composition, with the input method's own windows, show where
# it drew. The Mac's runs type through drive.mm's scripted input method (keys, find) and Linux's
# through fcitx5 (check/ime.sh), so it's Windows' alone, and runs only when named:
# ./check.sh input.

if [ "$(uname)" = Darwin ] || [ "$(uname)" = Linux ]; then
  echo "skip: input, Windows' alone: the Mac's runs type through drive.mm's input method and" \
    "Linux's check/ime.sh through fcitx5."
  return 0
fi

log="$out/input.log"
step=0
# A textarea over the whole page, whose title says what it holds and whether it composes.
page='<title>start</title><textarea id=t style="position:fixed;inset:0;font-size:32px"></textarea><script>
const t=document.getElementById("t");let c=0;
function u(){document.title=(c?"composing ":"value ")+t.value}
t.addEventListener("compositionstart",()=>{c=1;u()});
t.addEventListener("compositionupdate",()=>setTimeout(u));
t.addEventListener("compositionend",()=>{c=0;setTimeout(u)});
t.addEventListener("input",()=>setTimeout(u));
</script>'
url="data:text/html,$(python3 -I -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1]))' "$page")"
BROWSER_DRIVE=1 BROWSER_URL="$url" "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load

# The input method's state for the field with the keys, as the driver logs it for the last command.
ime_state() {
  sed -n "/^browser: drive $step: /,\$p" "$log" | grep '^browser: drive: ime open ' | tail -1 |
    sed 's/^browser: drive: ime //'
}
# The place the field asked the input method to compose at, "x y".
ime_at() { ime_state | sed -n 's/.* at \(-*[0-9]*\) \(-*[0-9]*\) candidates .*/\1 \2/p'; }
# Waits up to 3 s for the page's title to be $1 since command $2.
titled() {
  for _ in $(seq 30); do
    sed -n "/^browser: drive $2: /,\$p" "$log" | grep -q "^browser: title [0-9]* $1\$" && return 0
    sleep 0.1
  done
  return 1
}
page_title() { grep '^browser: title ' "$log" | tail -1 | cut -d' ' -f4-; }

send ime system
grep -q '^browser: drive: ime system 00000000 layout [0-9a-f]*0411$' "$log" ||
  fail "Microsoft's Japanese input method didn't become the app's: $(grep '^browser: drive: ime system' "$log")"
send click 600 300
in_browser RenderWidgetHostViewCocoa "after a click in the page"

# The keys reach the input method as the window's own, or else only through the system's keyboard
# input; and it opens with its on key, or else as the field opens it. The first way that composes.
how=
for way in "imeon" "open" "send imeon" "send open"; do
  for word in $way; do
    case $word in
      imeon) send key imeon ;;
      *) send ime "$word" ;;
    esac
  done
  from=$((step + 1))
  send type ka
  if titled 'composing か' "$from"; then
    how=$way
    break
  fi
  echo "note: input: \"$way\" didn't compose か in the page, which shows \"$(page_title)\"; $(ime_state)"
  send key escape
  send key a cmd
  send key backspace
done
[ -n "$how" ] || fail "Microsoft's input method composed nothing in the page"
echo "note: input: the page composes with \"$how\""

# The page's textarea.
../tools/window-screenshot.sh "$pid" "$out/input-page-composing.png" all || fail "no window to capture"
from=$((step + 1))
send key escape
titled 'value *' "$from" || fail "Escape didn't cancel the page's composition: $(page_title)"
from=$((step + 1))
send type nihongo
titled 'composing にほんご' "$from" || fail "the page didn't compose にほんご: $(page_title)"
send key space
titled 'composing 日本語' "$from" || fail "space didn't convert the page's にほんご to 日本語: $(page_title)"
../tools/window-screenshot.sh "$pid" "$out/input-page-converted.png" all || fail "no window to capture"
from=$((step + 1))
send key return
titled 'value 日本語' "$from" || fail "Enter didn't commit the page's 日本語: $(page_title)"

# In ilo's field whose text the app logs as "browser: $1"<text>"", composes か and cancels it, then
# composes, converts and commits 日本語, and composes か after it, the input method asked for each at
# the field's caret, which has moved on. Captures the conversion as $2.
in_field() {
  send type ka
  expect "^browser: $1\"か\"" "$2: the field didn't show the composition か; $(ime_state)"
  [ "$marked" = 0,1 ] || fail "$2: the field didn't mark the composition か: marked $marked"
  first=$(ime_at)
  send key escape
  [ "$marked" = none ] || fail "$2: Escape didn't cancel the composition: marked $marked"
  send type nihongo
  expect "^browser: $1\"にほんご\"" "$2: the field didn't compose にほんご; $(ime_state)"
  send key space
  expect "^browser: $1\"日本語\"" "$2: space didn't convert にほんご to 日本語; $(ime_state)"
  [ "$marked" = 0,3 ] || fail "$2: the field didn't mark the conversion: marked $marked"
  ../tools/window-screenshot.sh "$pid" "$out/input-$2.png" all || fail "no window to capture"
  send key return
  [ "$marked" = none ] || fail "$2: Enter didn't commit 日本語: marked $marked"
  send type ka
  expect "^browser: $1\"日本語か\"" "$2: the field didn't compose か after 日本語; $(ime_state)"
  second=$(ime_at)
  size=$(ime_state | sed -n 's/.* in \([0-9]*\)x\([0-9]*\)$/\1 \2/p')
  echo "note: input: $2 asked to compose at $first, then after 日本語 at $second, in $size"
  # shellcheck disable=SC2086 # the numbers
  set -- $first $second $size
  [ "$#" = 6 ] && [ "$1" -ge 0 ] && [ "$3" -gt "$1" ] && [ "$3" -lt "$5" ] &&
    [ "$2" -ge 0 ] && [ "$2" -lt "$6" ] && [ "$4" -ge 0 ] && [ "$4" -lt "$6" ] ||
    fail "the input method wasn't asked to compose at the caret, at $first then $second in $size"
  send key escape
  [ "$marked" = none ] || fail "Escape didn't cancel the composition after 日本語: marked $marked"
}

# The command bar, which Enter while composing doesn't send anywhere.
send key t cmd
expect '^browser: command bar shown$' "⌘T didn't open the command bar"
in_field 'command bar drew ' command-bar
! grep -q '^browser: command bar go' "$log" || fail "Enter while composing in the command bar went"
none_since '^browser: command bar hidden$' || fail "Escape while composing closed the command bar"
send key escape
send key escape
expect '^browser: command bar hidden$' "Escape didn't close the command bar"

# The find bar, over the page.
send click 600 300
send key f cmd
expect '^browser: find bar shown$' "⌘F didn't open the find bar"
in_field 'find bar drew ' find-bar
send key escape

# Settings' search, in Settings › Shortcuts.
send menu Settings…
expect '^browser: settings shown ' "Settings… didn't show Settings"
settings_drew
send clickkey 401 60
expect '^browser: settings tab Shortcuts$' "a click on Shortcuts didn't show its tab"
settings_drew Shortcuts
send clickkey 300 120
in_field 'shortcuts search ' settings-search

send ime off
exec 3>&-
quit
echo "ok: Microsoft's Japanese input method composed, converted, committed and cancelled in the page, the command bar, the find bar and Settings' search, at each field's caret, with \"$how\". Windows: $out/input-*.png"
