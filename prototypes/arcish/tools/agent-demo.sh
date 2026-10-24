#!/bin/sh
# Records Claude Code doing a task in Arcish while a scripted user (tools/agent-demo.js)
# watches, answers its requests and signs in when asked. Uses a throwaway profile.
#
#   tools/agent-demo.sh [out-dir] [task-file]
#
# Leaves transcript.jsonl, stills and recording/demo.mp4 in out-dir. ARCISH_AGENT_MODEL picks
# the model (default sonnet); ARCISH_AGENT_ANSWER=deny refuses every confirmation.
set -e
cd "$(dirname "$0")/.."
out="${1:-/tmp/arcish-agent-demo}"
rm -rf "$out"
mkdir -p "$out"
export ARCISH_USER_DATA="$out/profile"

env -u ELECTRON_RUN_AS_NODE ARCISH_AGENT_DEMO="$out/recording" node_modules/.bin/electron . > "$out/arcish.log" 2>&1 &
app=$!
while [ ! -S "$ARCISH_USER_DATA/agent.sock" ]; do sleep 0.2; done

cat > "$out/mcp.json" <<JSON
{ "mcpServers": { "arcish": { "command": "node", "args": ["$PWD/tools/mcp-bridge.js"], "env": { "ARCISH_USER_DATA": "$ARCISH_USER_DATA" } } } }
JSON

task='You have a web browser through the arcish tools. Please:
1. Find the current #1 story on Hacker News (news.ycombinator.com).
2. Order a pizza on https://httpbin.org/forms/post for the customer "Arcish demo": large, with bacon and extra cheese, and the title of that #1 story as the delivery instructions. Submit the order and tell me what the server received.
3. Get into the secure area at https://the-internet.herokuapp.com/login. You do not have the credentials.
Finish with a short summary.'
[ -n "$2" ] && task="$(cat "$2")"

# Only Arcish's tools: no built-in web fetching, files or shell.
MCP_TOOL_TIMEOUT=900000 claude -p "$task" --model "${ARCISH_AGENT_MODEL:-sonnet}" --mcp-config "$out/mcp.json" --strict-mcp-config \
  --tools "" --allowedTools mcp__arcish --max-budget-usd 3 --no-session-persistence \
  --output-format stream-json --verbose > "$out/transcript.jsonl"

# The scripted user quits once the agent disconnects; don't hang if the agent never got going.
for i in $(seq 1 60); do kill -0 $app 2>/dev/null || break; sleep 0.25; done
kill $app 2>/dev/null || true
tools/agent-video.sh "$out/recording" 2
