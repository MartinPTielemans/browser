# An MCP client, scripted with curl, reaches the browser's server once Settings › Agents is on and
# it's connected, as the saved settings say here. Nothing gets in without its token or from a web
# page. Its first call makes its Space, and it may read only the tabs there: a read of the user's
# tab is refused, with an activity-log entry, and a read of example.com, opened in its Space, gives
# the page's text. Ending its session ends its session with the tab.

log="$out/mcp.log"
step=0
rm -f "$saved" "$launch_frame"
agents_file="$(dirname "$saved")/Agents"
token=$(python3 -I -c 'import secrets; print(secrets.token_hex(32))')
printf '{"on":true,"agents":[{"client":"mcp-client","token":"%s"}]}' "$token" > "$agents_file"
chmod 600 "$agents_file"
BROWSER_DRIVE=1 BROWSER_URL='https://en.wikipedia.org/wiki/Arc_(web_browser)' \
  "$app/Contents/MacOS/Browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
user_tab=$(today | cut -d' ' -f1)
await '^browser: agents server on 127\.0\.0\.1:[0-9]*$' || fail "the saved switch didn't start the server"
port=$(grep '^browser: agents server on' "$log" | tail -1 | sed 's/.*://')
session=

# Posts the JSON-RPC message $1 as the client, with its session once it has one, and prints the
# answer's body. The headers go in mcp-head.
mcp() {
  curl -s -m 30 -D "$out/mcp-head" -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
    ${session:+-H "Mcp-Session-Id:$session"} -d "$1" "http://127.0.0.1:$port/mcp"
}
status() { head -1 "$out/mcp-head" | cut -d' ' -f2; }
# Calls the tool $1 with the arguments $2, and prints its text, after "error: " if it's an error.
call() {
  mcp "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"$1\",\"arguments\":$2}}" |
    python3 -I -c '
import json, sys
result = json.load(sys.stdin)["result"]
print(("error: " if result["isError"] else "") + result["content"][0]["text"])'
}
await_log() { await "$1" || fail "$2"; }

body='{"jsonrpc":"2.0","id":1,"method":"ping"}'
none=$(curl -s -o /dev/null -w '%{http_code}' -d "$body" "http://127.0.0.1:$port/mcp")
[ "$none" = 401 ] || fail "a client without the token got $none, not 401"
page=$(curl -s -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $token" \
  -H 'Origin: https://example.com' -d "$body" "http://127.0.0.1:$port/mcp")
[ "$page" = 403 ] || fail "a web page with the token got $page, not 403"

mcp '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"check","version":"1"}}}' \
  > "$out/mcp-initialize.json"
[ "$(status)" = 200 ] || fail "initialize got $(status)"
grep -q '"protocolVersion":"2025-06-18"' "$out/mcp-initialize.json" ||
  fail "initialize didn't answer in the client's version"
session=$(grep -i '^mcp-session-id:' "$out/mcp-head" | cut -d' ' -f2 | tr -d '\r')
[ -n "$session" ] || fail "initialize gave no session"
await_log '^browser: agent mcp-client hello from check$' "the app didn't hear the client's hello"
mcp '{"jsonrpc":"2.0","method":"notifications/initialized"}' > /dev/null
[ "$(status)" = 202 ] || fail "the initialized notification got $(status), not 202"
mcp '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | grep -q '"name":"read_page"' ||
  fail "tools/list didn't list read_page"

listed=$(call list_tabs '{}')
await_log '^browser: made space [1-9][0-9]* for agent mcp-client$' "list_tabs didn't make the agent's Space"
agent=$(grep '^browser: made space' "$log" | tail -1 | cut -d' ' -f4)
await_log "^browser: added space $agent MCP client\$" "the agent's Space didn't reach the switcher"
case "$listed" in
  "Your Space has no tabs yet."*) ;;
  *) fail "list_tabs in a new Space said: $listed" ;;
esac
case "$listed" in *"$user_tab"*) fail "list_tabs listed the user's tab" ;; esac

refused=$(call read_page "{\"tab\":$user_tab}")
[ "$refused" = "error: The tab isn't in the agent's own Space." ] ||
  fail "reading the user's tab answered: $refused"
await_log "^browser: agent activity mcp-client $user_tab read_page refused: The tab isn't in the agent's own Space\.\$" \
  "the refused read didn't reach the activity log"
grep -q "^browser: tab text $user_tab " "$log" && fail "the core read the user's tab"

send click $page_point
driven=$active
if [ "$driven" = 1 ]; then
  send key 2 ctrl
  expect "^browser: space $agent\( in .*\)\{0,1\}\$" "⌃2 didn't show the agent's Space"
  open_from_bar example.com
  agent_tab=$tab
  listed=$(call list_tabs '{}')
  case "$listed" in
    *"$agent_tab  Example Domain  https://example.com/"*) ;;
    *) fail "list_tabs didn't list the agent's tab: $listed" ;;
  esac
  read=$(call read_page "{\"tab\":$agent_tab}")
  case "$read" in
    "Tab $agent_tab: Example Domain"*"Example Domain"*) ;;
    *) fail "reading the agent's tab answered: $read" ;;
  esac
  await_log "^browser: agent activity mcp-client $agent_tab read_page\$" "the read didn't reach the activity log"
  await_log "^browser: tab text $agent_tab [0-9]* [1-9][0-9]*\$" "the core didn't read the page"
  await_log "^browser: agent mcp-client at work in $agent_tab\$" "the agent's tab didn't show it at work"
  # Ending its session disconnects the agent, which ends its session with the tab.
  curl -s -o /dev/null -X DELETE -H "Authorization: Bearer $token" -H "Mcp-Session-Id:$session" \
    "http://127.0.0.1:$port/mcp"
  await_log "^browser: agent session ended mcp-client $agent_tab disconnected\$" \
    "ending the client's session didn't end its session with the tab"
  await_log "^browser: agent mcp-client done in $agent_tab\$" "the tab still shows a gone agent at work"
fi
quit
rm -f "$agents_file"
if [ "$driven" = 1 ]; then
  echo "ok: an MCP client gets in with its token only, gets a Space of its own, is refused the user's tab and reads its own"
else
  echo "ok: an MCP client gets in with its token only, gets a Space of its own and is refused the user's tab. skip: reading a page in its Space. Browser wasn't the active app, so the page ignored the click."
fi
