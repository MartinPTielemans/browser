# A Space the driver makes for an agent, through the core as agent code will, with the Spaces menu
# open, reaches the menu once it closes. It shows in the switcher as the agent's, and comes back the
# agent's, with its owner, after a quit. The agent may act only on the tabs of its own Space: not on
# the user's, nor another agent's on its. ⌃2 shows it as any second Space, a tab opened there is the
# agent's, and the user's tab's Move to leaves it out, listing only the user's own Space, checked,
# and New Space…. The agent's mark on its tab goes as its session there ends: it releases the tab,
# its client disconnects, or the user takes the tab back, which stays theirs after a quit until
# they hand it back from its menu.

log="$out/agents.log"
step=0
rm -f "$saved" "$launch_frame"
# The user's tab, as the first run opens none.
BROWSER_DRIVE=1 BROWSER_URL='https://en.wikipedia.org/wiki/Arc_(web_browser)' \
  "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await_load
first=$(space)
user_tab=$(today | cut -d' ' -f1)
# On the Mac the Space is made with the Spaces menu open, which holds the menu bar back until the
# menu closes, as AppKit can crash when the main menu is replaced under an open menu. Linux shows
# no menu bar.
mac=
made="a Space made for an agent reaches the Spaces menu"
if [ "$(uname)" = Darwin ]; then
  mac=1
  made="a Space made with the Spaces menu open reaches the menu bar once it closes"
  post openmenu Spaces
  expect '^browser: menu tracked New Space…' "the Spaces menu didn't open"
fi
post agentspace claude-code Claude Code
expect '^browser: made space [1-9][0-9]* for agent claude-code$' "the driver didn't make the agent's Space"
agent=$(grep '^browser: made space' "$log" | tail -1 | cut -d' ' -f4)
expect "^browser: added space $agent Claude Code\$" "the agent's Space didn't reach the switcher"
expect "^browser: agent space $agent Claude Code\$" "the switcher didn't hear whose the Space is"
if [ -n "$mac" ]; then
  expect '^browser: menu bar waits for the menu open$' "the menu bar was replaced under the open menu"
  none_since '^browser: menu bar put up' || fail "the menu bar was put up with the menu still open"
  post dismiss
  expect '^browser: menu bar put up once the menus closed$' \
    "the menu bar didn't come once the menu closed"
fi
send menubar Spaces
expect '^browser: menu bar Spaces: .*Claude Code' "the Spaces menu didn't list the Space made while it was open"
grep -q "^browser: agent space $first " "$log" && fail "the user's Space is an agent's"
send agentmayact claude-code "$user_tab"
expect "^browser: agent claude-code may act on $user_tab: 0\$" "the agent may act on the user's tab"
expect "^browser: agent activity claude-code $user_tab navigate refused: The tab isn't in the agent's own Space.\$" \
  "the refusal didn't reach the activity log"
send click $page_point
driven=$active
if [ "$driven" = 1 ]; then
  send key 2 ctrl
  expect "^browser: space $agent\( in .*\)\{0,1\}\$" "⌃2 didn't show the agent's Space"
  open_from_bar example.com
  agent_tab=$tab
  # Even in its own Space, the core refuses an agent ad blocking, passwords and DevTools, and any
  # action agents don't have, each with why in the activity log, and none starts a session.
  refused() {
    send agentmayact claude-code "$agent_tab" "$1"
    expect "^browser: agent claude-code may act on $agent_tab: 0\$" "the agent may $1 in its own Space"
    expect "^browser: agent activity claude-code $agent_tab $1 refused: $2\$" \
      "the $1 refusal didn't reach the activity log"
  }
  before=$step
  refused ad_blocking "Agents can't change ad blocking. It's the user's, in Site Controls."
  refused passwords "Agents never read passwords, cookies or autofill data."
  refused devtools "Agents get no DevTools or debugging port."
  refused run_script 'Agents have no action "run_script".'
  sed -n "/^browser: drive $before: /,\$p" "$log" | grep -q "^browser: agent claude-code at work" &&
    fail "a refused action started the agent's session"
  send agentmayact claude-code "$agent_tab"
  expect "^browser: agent claude-code may act on $agent_tab: 1\$" \
    "the agent may not act on a tab of its own Space"
  expect "^browser: agent claude-code at work in $agent_tab\$" "the agent's tab didn't show it at work"
  # Each way an agent's session with the tab ends takes its mark off, with an activity-log entry.
  send agentrelease claude-code "$agent_tab"
  expect "^browser: agent claude-code released $agent_tab: 1\$" "the core had no session to release"
  expect "^browser: agent session ended claude-code $agent_tab released\$" \
    "the released session didn't reach the activity log"
  expect "^browser: agent claude-code done in $agent_tab\$" "the released tab still shows the agent at work"
  send agentrelease claude-code "$agent_tab"
  expect "^browser: agent claude-code released $agent_tab: 0\$" "a session ended twice"
  send agentmayact claude-code "$agent_tab"
  expect "^browser: agent claude-code at work in $agent_tab\$" "acting again didn't show the agent at work"
  send agentdisconnect claude-code
  expect "^browser: agent claude-code disconnected, 1 sessions ended\$" \
    "the disconnect didn't end the agent's session"
  expect "^browser: agent session ended claude-code $agent_tab disconnected\$" \
    "the disconnect didn't reach the activity log"
  expect "^browser: agent claude-code done in $agent_tab\$" "the tab still shows a disconnected agent at work"
  send agentmayact claude-code "$agent_tab"
  expect "^browser: agent claude-code at work in $agent_tab\$" "acting again didn't show the agent at work"
  send takeback "$agent_tab"
  expect "^browser: took back $agent_tab: 1\$" "the user couldn't take the tab back"
  expect "^browser: agent session ended claude-code $agent_tab taken back\$" \
    "taking the tab back didn't reach the activity log"
  expect "^browser: agent claude-code done in $agent_tab\$" "the tab taken back still shows the agent at work"
  send agentmayact codex "$agent_tab"
  expect "^browser: agent codex may act on $agent_tab: 0\$" "another agent may act on its tab"
  send key 1 ctrl
  expect "^browser: space $first\( in .*\)\{0,1\}\$" "⌃1 didn't show the user's Space again"
  expect "^browser: settled space $first\$" "the user's Space's page didn't settle"
  post rightclick $(row 0)
  expect '^browser: menu shown .*, Move to › [^,]* (checked), Move to › New Space…, ' \
    "the user's tab's menu offered the agent's Space to move to"
  send choose Duplicate
fi
quit

# Launched again, the agent's Space comes back the agent's, with its tab, and the user's the user's.
log="$out/agents-restored.log"
step=0
BROWSER_DRIVE=1 "$browser" < "$out/drive" > "$log" 2>&1 &
pid=$!
exec 3> "$out/drive"
await '^browser: restored in ' || fail "the relaunch didn't restore the Spaces"
await '^browser: spaces [0-9]* [0-9]*$' || fail "the relaunch didn't bring back both Spaces"
first=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f3)
agent=$(grep '^browser: spaces' "$log" | tail -1 | cut -d' ' -f4)
grep -q "^browser: added space $agent Claude Code\$" "$log" ||
  fail "the agent's Space came back without its name"
grep -q "^browser: agent space $agent Claude Code\$" "$log" ||
  fail "the agent's Space came back the user's"
grep -q "^browser: agent space $first " "$log" && fail "the user's Space came back an agent's"
if [ "$driven" = 1 ]; then
  await_load
  send click $page_point
  send key 2 ctrl
  expect "^browser: space $agent\( in .*\)\{0,1\}\$" "⌃2 didn't show the agent's Space after the relaunch"
  expect '^browser: today [1-9][0-9]*$' "the agent's Space didn't come back with its tab"
  agent_tab=$(today)
  # The user took it back before the quit, and it stays theirs until they hand it back (#404).
  send agentmayact claude-code "$agent_tab"
  expect "^browser: agent claude-code may act on $agent_tab: 0\$" \
    "the relaunch gave the agent back the tab the user took"
  expect "^browser: agent activity claude-code $agent_tab navigate refused: The user has this tab until they hand it back. Ask for it with ask_for_tab.\$" \
    "the held tab's refusal didn't reach the activity log"
  post rightclick $(row 0)
  expect "^browser: menu shown $(cased 'Taken Back from') Claude Code (disabled), $(cased 'Hand Back to') Claude Code, " \
    "the tab's menu didn't say the user took it back"
  send choose "$(cased 'Hand Back to') Claude Code"
  expect "^browser: agent claude-code handed back $agent_tab\$" "Hand Back didn't hand the tab back"
  send agentmayact claude-code "$agent_tab"
  expect "^browser: agent claude-code may act on $agent_tab: 1\$" \
    "the agent may not act on the tab handed back"
fi
quit
if [ "$driven" = 1 ]; then
  echo "ok: $made; an agent's Space shows and switches as the user's, keeps the user's tabs out, refuses an agent ad blocking, passwords, DevTools and any action agents lack, with why in the activity log, clears its mark as each session ends, and comes back the agent's, with the tab the user took back still theirs until Hand Back"
else
  echo "ok: $made; an agent's Space comes back the agent's. skip: switching to it and its tab. Browser wasn't the active app, so the page ignored the click."
fi
