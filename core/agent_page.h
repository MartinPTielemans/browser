#ifndef BROWSER_CORE_AGENT_PAGE_H_
#define BROWSER_CORE_AGENT_PAGE_H_

#include <cstdint>
#include <deque>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

#include "base/containers/flat_map.h"
#include "base/functional/callback.h"
#include "base/functional/callback_helpers.h"
#include "base/memory/scoped_refptr.h"
#include "base/memory/weak_ptr.h"
#include "base/timer/timer.h"
#include "base/values.h"
#include "browser_core/public/browser_core.h"
#include "content/public/browser/devtools_agent_host_client.h"
#include "content/public/browser/web_contents_observer.h"
#include "content/public/browser/web_contents_user_data.h"

namespace content {
class DevToolsAgentHost;
}  // namespace content

namespace browser_core {

// The page of a tab an agent works in, as bc_agent_page acts on it: its accessibility tree, input
// and screenshots, through Chromium's DevTools protocol from inside the browser, as Capture's
// PageCapture is. Only the commands here are ever sent, so an agent never runs script of its own.
// One action runs at a time, in the order asked.
class AgentPage : public content::WebContentsObserver,
                  public content::WebContentsUserData<AgentPage>,
                  public content::DevToolsAgentHostClient {
 public:
  AgentPage(const AgentPage&) = delete;
  AgentPage& operator=(const AgentPage&) = delete;
  ~AgentPage() override;

  // See bc_agent_page.
  void Act(uint64_t request, int action, int64_t element, std::string text);

  // The agent's session with |tab| ended: its page stops drawing while hidden, and DevTools lets
  // it go. What's still asked is answered as an error.
  static void SessionEnded(bc_id tab);

 private:
  friend class content::WebContentsUserData<AgentPage>;

  struct Job {
    uint64_t request;
    int action;
    int64_t element;
    std::string text;
  };
  using Reply = base::OnceCallback<void(const base::DictValue* result)>;

  explicit AgentPage(content::WebContents* contents);

  // Runs the next job once the one before has answered.
  void Next();
  void Run();
  // Whether the tab's page is one agents may use.
  bool IsWebPage();
  // Runs |then| once the page has stopped loading, or after a while.
  void AfterLoad(base::OnceClosure then);
  // Waits a moment for what an input starts, such as a navigation, then for it to load, then
  // answers with the tree.
  void Settle();

  void Click();
  void Mouse(const char* type, std::pair<double, double> at, base::OnceClosure then);
  void Type(bool submit);
  void Focus(int64_t element, bool submit);
  void PressKey(const std::string& key, base::OnceClosure then);
  void Scroll(bool down);
  void History(bool forward);
  void Screenshot();

  // The tree, as bc_agent_page answers it.
  void Snapshot();
  // Marks the text fields in |fields| that are for passwords, one at a time, then answers.
  void CheckPasswords(base::ListValue nodes, std::vector<int> fields, std::set<int> passwords);
  void Answer(std::string text, bool error);
  void Fail(std::string why);

  // Scrolls the job's element into view and gives its center, in the viewport's CSS pixels.
  void Center(base::OnceCallback<void(std::optional<std::pair<double, double>>)> then);
  // Whether the element is a password field, from its attributes.
  void IsPassword(int64_t element, base::OnceCallback<void(std::optional<bool>)> then);
  // Closes Chrome's autofill and password suggestions, which the keys would otherwise pick from.
  void HideSuggestions();

  void Send(const char* method, base::DictValue params, Reply reply);
  void Detach();
  void LetGo();

  // content::DevToolsAgentHostClient:
  void DispatchProtocolMessage(content::DevToolsAgentHost* agent_host,
                               base::span<const uint8_t> message) override;
  void AgentHostClosed(content::DevToolsAgentHost* agent_host) override;

  // content::WebContentsObserver:
  void DidStopLoading() override;
  void WebContentsDestroyed() override;

  const bc_id tab_;
  std::deque<Job> jobs_;
  // The job running, which has yet to answer.
  std::optional<Job> job_;
  base::OnceClosure loaded_;
  base::OneShotTimer wait_;
  base::OneShotTimer rest_;
  scoped_refptr<content::DevToolsAgentHost> agent_host_;
  int next_command_ = 1;
  base::flat_map<int, Reply> replies_;
  // Keeps the page drawing while it's hidden, for the agent's input and screenshots.
  base::ScopedClosureRunner drawing_;
  base::WeakPtrFactory<AgentPage> weak_factory_{this};

  WEB_CONTENTS_USER_DATA_KEY_DECL();
};

// What a page's accessibility tree, as DevTools' Accessibility.getFullAXTree gives it, may show
// an agent: each node without its value, and without the nodes under a field, where Chromium puts
// what was typed or filled into it, and with "password": true on the nodes in |passwords|.
base::ListValue AgentTree(base::ListValue nodes, const std::set<int>& passwords);

}  // namespace browser_core

#endif  // BROWSER_CORE_AGENT_PAGE_H_
