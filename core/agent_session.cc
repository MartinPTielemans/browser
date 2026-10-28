#include "browser_core/agent_session.h"

#include <map>
#include <optional>
#include <set>
#include <string_view>
#include <utility>
#include <vector>

#include "base/no_destructor.h"
#include "browser_core/agent_page.h"
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

// The tabs the user holds from agents.
std::set<bc_id>& Held() {
  static base::NoDestructor<std::set<bc_id>> held;
  return *held;
}

bool End(bc_id tab, std::optional<std::string_view> agent, int why) {
  auto it = Sessions().find(tab);
  if (it == Sessions().end() || (agent && it->second != *agent)) {
    return false;
  }
  const std::string held = std::move(it->second);
  Sessions().erase(it);
  AgentPage::SessionEnded(tab);
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
    if (Space::AgentMayAct(session.second, session.first)) {
      return false;
    }
    AgentPage::SessionEnded(session.first);
    return true;
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

void Hold(bc_id tab) {
  Held().insert(tab);
}

bool HandBack(bc_id tab) {
  return Held().erase(tab) > 0;
}

bool IsHeld(bc_id tab) {
  return Held().contains(tab);
}

}  // namespace browser_core
