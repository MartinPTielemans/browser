#include "browser_core/agent_session.h"

#include <map>
#include <optional>
#include <string_view>
#include <utility>
#include <vector>

#include "base/containers/fixed_flat_map.h"
#include "base/containers/fixed_flat_set.h"
#include "base/no_destructor.h"
#include "browser_core/host.h"
#include "browser_core/space.h"

namespace browser_core {

namespace {

// The agent whose session holds each tab, by the tab's id. Only an agent's own Space's tabs are
// its to act in, so a tab has one at most.
std::map<bc_id, std::string>& Sessions() {
  static base::NoDestructor<std::map<bc_id, std::string>> sessions;
  return *sessions;
}

bool End(bc_id tab, std::optional<std::string_view> agent, int why) {
  auto it = Sessions().find(tab);
  if (it == Sessions().end() || (agent && it->second != *agent)) {
    return false;
  }
  const std::string held = std::move(it->second);
  Sessions().erase(it);
  // One whose tab closed or left the agent's Space ended then, and the app saw it.
  if (!Space::AgentMayAct(held, tab)) {
    return false;
  }
  if (Host().agent_session_ended) {
    Host().agent_session_ended(Host().context, held.c_str(), tab, why);
  }
  return true;
}

}  // namespace

void AgentActed(const std::string& agent, bc_id tab) {
  std::erase_if(Sessions(), [](const auto& session) {
    return !Space::AgentMayAct(session.second, session.first);
  });
  Sessions()[tab] = agent;
}

bool EndAgentSession(const std::string& agent, bc_id tab, int why) {
  return End(tab, agent, why);
}

size_t EndAgentSessions(const std::string& agent, int why) {
  // Collected first, as the app hears of each end while this runs.
  std::vector<bc_id> tabs;
  for (const auto& [tab, held] : Sessions()) {
    if (held == agent) {
      tabs.push_back(tab);
    }
  }
  size_t ended = 0;
  for (bc_id tab : tabs) {
    if (End(tab, agent, why)) {
      ++ended;
    }
  }
  return ended;
}

bool EndTabSession(bc_id tab, int why) {
  return End(tab, std::nullopt, why);
}

std::string RefusedAction(std::string_view action) {
  // The reference agent's tools that act on a tab (prototypes/arcish/agent.js), each something the
  // user does in the page.
  static constexpr auto kAllowed = base::MakeFixedFlatSet<std::string_view>(
      {"click", "close_tab", "navigate", "press_key", "read_page", "screenshot", "select_option",
       "snapshot", "type"});
  // Those agents ask for that the rules forbid outright, with why.
  static constexpr auto kForbidden = base::MakeFixedFlatMap<std::string_view, std::string_view>({
      {"ad_blocking", "Agents can't change ad blocking. It's the user's, in Site Controls."},
      {"devtools", "Agents get no DevTools or debugging port."},
      {"passwords", "Agents never read passwords, cookies or autofill data."},
  });
  if (kAllowed.contains(action)) {
    return std::string();
  }
  if (const auto forbidden = kForbidden.find(action); forbidden != kForbidden.end()) {
    return std::string(forbidden->second);
  }
  return "Agents have no action \"" + std::string(action) + "\".";
}

}  // namespace browser_core
