#include "browser_core/agent_page.h"

#include <deque>
#include <map>
#include <string_view>
#include <utility>
#include <vector>

#include "base/containers/span.h"
#include "base/functional/bind.h"
#include "base/json/json_reader.h"
#include "base/json/json_writer.h"
#include "base/strings/string_util.h"
#include "base/task/single_thread_task_runner.h"
#include "base/time/time.h"
#include "browser_core/host.h"
#include "browser_core/tab.h"
#include "build/build_config.h"
#include "components/autofill/content/browser/content_autofill_client.h"
#include "components/autofill/core/browser/filling/filling_product.h"
#include "components/autofill/core/browser/suggestions/suggestion_hiding_reason.h"
#include "content/public/browser/devtools_agent_host.h"
#include "content/public/browser/navigation_controller.h"
#include "content/public/browser/navigation_entry.h"
#include "content/public/browser/web_contents.h"
#include "ui/gfx/geometry/size.h"
#include "url/gurl.h"

namespace browser_core {

namespace {

constexpr base::TimeDelta kLoadTimeout = base::Seconds(15);
// What an input starts, such as a navigation or a menu, has started by then.
constexpr base::TimeDelta kSettle = base::Milliseconds(400);
// How long a hidden page keeps drawing after an agent's last action, for its next one.
constexpr base::TimeDelta kRest = base::Seconds(30);

// Select All's modifier: ⌘ on the Mac, Ctrl elsewhere, as DevTools' Input domain numbers them.
#if BUILDFLAG(IS_MAC)
constexpr int kSelectAllModifier = 4;
#else
constexpr int kSelectAllModifier = 2;
#endif

// The keys an agent may press, with their key codes and the text they type.
struct Key {
  std::string_view name;
  int code;
  std::string_view text;
};
constexpr Key kKeys[] = {
    {"Enter", 13, "\r"},   {"Tab", 9, ""},         {"Escape", 27, ""},  {"Backspace", 8, ""},
    {"Delete", 46, ""},    {"Space", 32, " "},     {"ArrowUp", 38, ""}, {"ArrowDown", 40, ""},
    {"ArrowLeft", 37, ""}, {"ArrowRight", 39, ""}, {"PageUp", 33, ""},  {"PageDown", 34, ""},
    {"Home", 36, ""},      {"End", 35, ""},
};

// Whether the node is a field, whose value is the user's: a text field of any kind, a select, a
// date, time or number input, or an editable region such as a mail's body.
bool IsField(const base::DictValue& node) {
  if (const base::ListValue* properties = node.FindList("properties")) {
    for (const base::Value& property : *properties) {
      const base::DictValue* dict = property.GetIfDict();
      const std::string* name = dict ? dict->FindString("name") : nullptr;
      if (name && *name == "editable") {
        return true;
      }
    }
  }
  const std::string* role = node.FindStringByDottedPath("role.value");
  if (!role) {
    return false;
  }
  static constexpr std::string_view kFields[] = {
      "textbox", "searchbox", "combobox",  "spinbutton",
      "date",    "datetime",  "inputtime", "colorwell",
  };
  const std::string lower = base::ToLowerASCII(*role);
  for (std::string_view field : kFields) {
    if (lower == field) {
      return true;
    }
  }
  return false;
}

void Report(bc_id tab, uint64_t request, std::string text, bool error) {
  if (Host().agent_page_done) {
    Host().agent_page_done(Host().context, tab, request, text.c_str(), error);
  }
}

}  // namespace

base::ListValue AgentTree(base::ListValue nodes, const std::set<int>& passwords) {
  std::map<std::string, const base::DictValue*> by_id;
  for (const base::Value& node : nodes) {
    const base::DictValue* dict = node.GetIfDict();
    const std::string* id = dict ? dict->FindString("nodeId") : nullptr;
    if (id) {
      by_id[*id] = dict;
    }
  }
  // Every node under a field, which goes, so no step down from another node reaches it either.
  std::set<std::string> under_fields;
  std::vector<std::string> steps;
  const auto children_of = [&steps](const base::DictValue& node) {
    if (const base::ListValue* children = node.FindList("childIds")) {
      for (const base::Value& child : *children) {
        if (const std::string* id = child.GetIfString()) {
          steps.push_back(*id);
        }
      }
    }
  };
  for (const auto& [id, node] : by_id) {
    if (IsField(*node)) {
      children_of(*node);
    }
  }
  while (!steps.empty()) {
    const std::string id = std::move(steps.back());
    steps.pop_back();
    if (!under_fields.insert(id).second) {
      continue;
    }
    if (auto found = by_id.find(id); found != by_id.end()) {
      children_of(*found->second);
    }
  }
  by_id.clear();
  base::ListValue kept;
  for (base::Value& node : nodes) {
    base::DictValue* dict = node.GetIfDict();
    const std::string* id = dict ? dict->FindString("nodeId") : nullptr;
    if (!id || under_fields.contains(*id)) {
      continue;
    }
    dict->Remove("value");
    if (IsField(*dict)) {
      dict->Remove("childIds");
    }
    if (base::ListValue* properties = dict->FindList("properties")) {
      properties->EraseIf([](const base::Value& property) {
        const base::DictValue* each = property.GetIfDict();
        const std::string* name = each ? each->FindString("name") : nullptr;
        return !name || *name == "valuetext";
      });
    }
    const std::optional<int> backend = dict->FindInt("backendDOMNodeId");
    if (backend && passwords.contains(*backend)) {
      dict->Set("password", true);
    }
    kept.Append(std::move(node));
  }
  return kept;
}

AgentPage::AgentPage(content::WebContents* contents)
    : content::WebContentsObserver(contents),
      content::WebContentsUserData<AgentPage>(*contents),
      tab_(Tab::IdOf(contents)) {}

AgentPage::~AgentPage() {
  Detach();
}

void AgentPage::Act(uint64_t request, int action, int64_t element, std::string text) {
  jobs_.push_back({request, action, element, std::move(text)});
  Next();
}

// static
void AgentPage::SessionEnded(bc_id tab) {
  content::WebContents* contents = Tab::FromId(tab);
  if (AgentPage* page = contents ? AgentPage::FromWebContents(contents) : nullptr) {
    page->LetGo();
  }
}

void AgentPage::Next() {
  if (job_ || jobs_.empty()) {
    return;
  }
  job_ = std::move(jobs_.front());
  jobs_.pop_front();
  rest_.Stop();
  // A hidden tab otherwise stops drawing, and its page then takes no input and gives no picture.
  if (!drawing_) {
    drawing_ = GetWebContents().IncrementCapturerCount(gfx::Size(), /*stay_hidden=*/true,
                                                       /*stay_awake=*/false,
                                                       /*is_activity=*/false);
  }
  AfterLoad(base::BindOnce(&AgentPage::Run, weak_factory_.GetWeakPtr()));
}

bool AgentPage::IsWebPage() {
  return GetWebContents().GetLastCommittedURL().SchemeIsHTTPOrHTTPS();
}

void AgentPage::Run() {
  const int action = job_->action;
  const bool history = action == BC_AGENT_PAGE_BACK || action == BC_AGENT_PAGE_FORWARD;
  if (!history && !IsWebPage()) {
    Fail("The tab's page isn't an http or https page, the only pages agents may use.");
    return;
  }
  HideSuggestions();
  switch (action) {
    case BC_AGENT_PAGE_SNAPSHOT:
      Snapshot();
      return;
    case BC_AGENT_PAGE_CLICK:
      Click();
      return;
    case BC_AGENT_PAGE_TYPE:
    case BC_AGENT_PAGE_TYPE_SUBMIT:
      Type(action == BC_AGENT_PAGE_TYPE_SUBMIT);
      return;
    case BC_AGENT_PAGE_KEY:
      PressKey(job_->text, base::BindOnce(&AgentPage::Settle, weak_factory_.GetWeakPtr()));
      return;
    case BC_AGENT_PAGE_SCROLL_DOWN:
    case BC_AGENT_PAGE_SCROLL_UP:
      Scroll(action == BC_AGENT_PAGE_SCROLL_DOWN);
      return;
    case BC_AGENT_PAGE_BACK:
    case BC_AGENT_PAGE_FORWARD:
      History(action == BC_AGENT_PAGE_FORWARD);
      return;
    case BC_AGENT_PAGE_SCREENSHOT:
      Screenshot();
      return;
  }
  Fail("There's no such action.");
}

void AgentPage::AfterLoad(base::OnceClosure then) {
  if (!GetWebContents().IsLoading()) {
    std::move(then).Run();
    return;
  }
  loaded_ = std::move(then);
  // A page that never stops loading, as one that streams, is used as it is after a while.
  wait_.Start(FROM_HERE, kLoadTimeout,
              base::BindOnce(&AgentPage::DidStopLoading, base::Unretained(this)));
}

void AgentPage::Settle() {
  wait_.Start(
      FROM_HERE, kSettle,
      base::BindOnce(
          [](AgentPage* page) {
            page->AfterLoad(base::BindOnce(&AgentPage::Snapshot, page->weak_factory_.GetWeakPtr()));
          },
          base::Unretained(this)));
}

void AgentPage::Click() {
  Center(base::BindOnce(
      [](base::WeakPtr<AgentPage> page, std::optional<std::pair<double, double>> at) {
        if (!page) {
          return;
        }
        if (!at) {
          page->Fail("The element isn't on the page any more, or is hidden. Take a new snapshot.");
          return;
        }
        base::WeakPtr<AgentPage> weak = page;
        page->Mouse("mouseMoved", *at,
                    base::BindOnce(
                        [](base::WeakPtr<AgentPage> page, std::pair<double, double> at) {
                          if (!page) {
                            return;
                          }
                          page->Mouse(
                              "mousePressed", at,
                              base::BindOnce(
                                  [](base::WeakPtr<AgentPage> page, std::pair<double, double> at) {
                                    if (page) {
                                      page->Mouse("mouseReleased", at,
                                                  base::BindOnce(&AgentPage::Settle, page));
                                    }
                                  },
                                  page, at));
                        },
                        weak, *at));
      },
      weak_factory_.GetWeakPtr()));
}

void AgentPage::Mouse(const char* type, std::pair<double, double> at, base::OnceClosure then) {
  base::DictValue params =
      base::DictValue().Set("type", type).Set("x", at.first).Set("y", at.second);
  if (std::string_view(type) != "mouseMoved") {
    params.Set("button", "left");
    params.Set("clickCount", 1);
  }
  Send("Input.dispatchMouseEvent", std::move(params),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, base::OnceClosure then, const base::DictValue* done) {
             if (!page) {
               return;
             }
             if (!done) {
               page->Fail("The page didn't take the click.");
               return;
             }
             std::move(then).Run();
           },
           weak_factory_.GetWeakPtr(), std::move(then)));
}

void AgentPage::Type(bool submit) {
  const int64_t element = job_->element;
  IsPassword(element,
             base::BindOnce(
                 [](base::WeakPtr<AgentPage> page, int64_t element, bool submit,
                    std::optional<bool> password) {
                   if (!page) {
                     return;
                   }
                   if (!password) {
                     page->Fail("The element isn't on the page any more. Take a new snapshot.");
                     return;
                   }
                   if (*password) {
                     page->Fail(
                         "Password fields are off limits to agents: signing in is the "
                         "user's to do.");
                     return;
                   }
                   page->Focus(element, submit);
                 },
                 weak_factory_.GetWeakPtr(), element, submit));
}

void AgentPage::Focus(int64_t element, bool submit) {
  Send("DOM.focus", base::DictValue().Set("backendNodeId", static_cast<int>(element)),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, bool submit, const base::DictValue* focused) {
             if (!page) {
               return;
             }
             if (!focused) {
               page->Fail("The element can't take typing. Take a new snapshot.");
               return;
             }
             // What the field holds is selected, so the text replaces it.
             const base::DictValue select = base::DictValue()
                                                .Set("key", "a")
                                                .Set("code", "KeyA")
                                                .Set("windowsVirtualKeyCode", 65)
                                                .Set("modifiers", kSelectAllModifier);
             page->Send("Input.dispatchKeyEvent",
                        select.Clone()
                            .Set("type", "rawKeyDown")
                            .Set("commands", base::ListValue().Append("selectAll")),
                        base::DoNothing());
             page->Send("Input.dispatchKeyEvent", select.Clone().Set("type", "keyUp"),
                        base::DoNothing());
             page->Send(
                 "Input.insertText", base::DictValue().Set("text", page->job_->text),
                 base::BindOnce(
                     [](base::WeakPtr<AgentPage> page, bool submit, const base::DictValue* typed) {
                       if (!page) {
                         return;
                       }
                       if (!typed) {
                         page->Fail("The page didn't take the text.");
                         return;
                       }
                       if (submit) {
                         page->PressKey("Enter", base::BindOnce(&AgentPage::Settle, page));
                       } else {
                         page->Settle();
                       }
                     },
                     page, submit));
           },
           weak_factory_.GetWeakPtr(), submit));
}

void AgentPage::PressKey(const std::string& name, base::OnceClosure then) {
  const Key* key = nullptr;
  for (const Key& each : kKeys) {
    if (each.name == name) {
      key = &each;
    }
  }
  if (!key) {
    Fail(
        "There's no such key. Press Enter, Tab, Escape, Backspace, Delete, Space, an arrow, "
        "PageUp, PageDown, Home or End.");
    return;
  }
  HideSuggestions();
  base::DictValue event = base::DictValue()
                              .Set("key", key->name == "Space" ? " " : std::string(key->name))
                              .Set("code", std::string(key->name))
                              .Set("windowsVirtualKeyCode", key->code)
                              .Set("nativeVirtualKeyCode", key->code);
  base::DictValue down = event.Clone().Set("type", key->text.empty() ? "rawKeyDown" : "keyDown");
  if (!key->text.empty()) {
    down.Set("text", std::string(key->text));
  }
  Send("Input.dispatchKeyEvent", std::move(down), base::DoNothing());
  Send("Input.dispatchKeyEvent", event.Clone().Set("type", "keyUp"),
       base::BindOnce([](base::OnceClosure then, const base::DictValue*) { std::move(then).Run(); },
                      std::move(then)));
}

void AgentPage::Scroll(bool down) {
  if (job_->element) {
    Center(base::BindOnce(
        [](base::WeakPtr<AgentPage> page, std::optional<std::pair<double, double>> at) {
          if (!page) {
            return;
          }
          if (!at) {
            page->Fail(
                "The element isn't on the page any more, or is hidden. Take a new snapshot.");
            return;
          }
          page->Settle();
        },
        weak_factory_.GetWeakPtr()));
    return;
  }
  Send("Page.getLayoutMetrics", base::DictValue(),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, bool down, const base::DictValue* metrics) {
             if (!page) {
               return;
             }
             const base::DictValue* viewport =
                 metrics ? metrics->FindDict("cssLayoutViewport") : nullptr;
             const double width = viewport ? viewport->FindDouble("clientWidth").value_or(0) : 0;
             const double height = viewport ? viewport->FindDouble("clientHeight").value_or(0) : 0;
             if (width <= 0 || height <= 0) {
               page->Fail("The page has no view to scroll.");
               return;
             }
             page->Send("Input.dispatchMouseEvent",
                        base::DictValue()
                            .Set("type", "mouseWheel")
                            .Set("x", width / 2)
                            .Set("y", height / 2)
                            .Set("deltaX", 0)
                            .Set("deltaY", (down ? 0.8 : -0.8) * height),
                        base::BindOnce(
                            [](base::WeakPtr<AgentPage> page, const base::DictValue*) {
                              if (page) {
                                page->Settle();
                              }
                            },
                            page));
           },
           weak_factory_.GetWeakPtr(), down));
}

void AgentPage::History(bool forward) {
  content::NavigationController& controller = GetWebContents().GetController();
  if (forward ? !controller.CanGoForward() : !controller.CanGoBack()) {
    Fail(forward ? "There's no page to go forward to." : "There's no page to go back to.");
    return;
  }
  content::NavigationEntry* entry = controller.GetEntryAtOffset(forward ? 1 : -1);
  if (!entry || !entry->GetURL().SchemeIsHTTPOrHTTPS()) {
    Fail("That page isn't an http or https page, the only pages agents may open.");
    return;
  }
  [[maybe_unused]] auto navigations = forward ? controller.GoForward() : controller.GoBack();
  Settle();
}

void AgentPage::Screenshot() {
  Send("Page.captureScreenshot", base::DictValue().Set("format", "jpeg").Set("quality", 75),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, const base::DictValue* result) {
             if (!page) {
               return;
             }
             const std::string* data = result ? result->FindString("data") : nullptr;
             if (!data) {
               page->Fail("The page couldn't be pictured.");
               return;
             }
             page->Answer(*data, /*error=*/false);
           },
           weak_factory_.GetWeakPtr()));
}

void AgentPage::Snapshot() {
  // After an action, the page may have gone somewhere agents may not.
  if (!IsWebPage()) {
    Fail("The tab's page isn't an http or https page, the only pages agents may use.");
    return;
  }
  Send("Accessibility.getFullAXTree", base::DictValue(),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, const base::DictValue* result) {
             if (!page) {
               return;
             }
             const base::ListValue* nodes = result ? result->FindList("nodes") : nullptr;
             if (!nodes) {
               page->Fail("The page's tree couldn't be read.");
               return;
             }
             // Its text fields, each checked for whether it's for a password.
             std::vector<int> fields;
             for (const base::Value& node : *nodes) {
               const base::DictValue* dict = node.GetIfDict();
               const std::string* role =
                   dict ? dict->FindStringByDottedPath("role.value") : nullptr;
               const std::optional<int> backend =
                   dict ? dict->FindInt("backendDOMNodeId") : std::nullopt;
               if (role && backend && (*role == "textbox" || *role == "searchbox")) {
                 fields.push_back(*backend);
               }
             }
             page->CheckPasswords(nodes->Clone(), std::move(fields), {});
           },
           weak_factory_.GetWeakPtr()));
}

void AgentPage::CheckPasswords(base::ListValue nodes,
                               std::vector<int> fields,
                               std::set<int> passwords) {
  if (fields.empty()) {
    std::optional<std::string> json =
        base::WriteJson(base::Value(AgentTree(std::move(nodes), passwords)));
    if (json) {
      Answer(std::move(*json), /*error=*/false);
    } else {
      Fail("The page's tree couldn't be read.");
    }
    return;
  }
  const int field = fields.back();
  fields.pop_back();
  IsPassword(field,
             base::BindOnce(
                 [](base::WeakPtr<AgentPage> page, base::ListValue nodes, std::vector<int> fields,
                    std::set<int> passwords, int field, std::optional<bool> password) {
                   if (!page) {
                     return;
                   }
                   // One that couldn't be read is taken for one.
                   if (password.value_or(true)) {
                     passwords.insert(field);
                   }
                   page->CheckPasswords(std::move(nodes), std::move(fields), std::move(passwords));
                 },
                 weak_factory_.GetWeakPtr(), std::move(nodes), std::move(fields),
                 std::move(passwords), field));
}

void AgentPage::Center(base::OnceCallback<void(std::optional<std::pair<double, double>>)> then) {
  using Then = base::OnceCallback<void(std::optional<std::pair<double, double>>)>;
  const int element = static_cast<int>(job_->element);
  Send("DOM.getDocument", base::DictValue().Set("depth", 0),
       base::BindOnce(
           [](base::WeakPtr<AgentPage> page, int element, Then then, const base::DictValue*) {
             if (!page) {
               return;
             }
             page->Send(
                 "DOM.scrollIntoViewIfNeeded", base::DictValue().Set("backendNodeId", element),
                 base::BindOnce(
                     [](base::WeakPtr<AgentPage> page, int element, Then then,
                        const base::DictValue*) {
                       if (!page) {
                         return;
                       }
                       page->Send(
                           "DOM.getContentQuads", base::DictValue().Set("backendNodeId", element),
                           base::BindOnce(
                               [](Then then, const base::DictValue* result) {
                                 const base::ListValue* quads =
                                     result ? result->FindList("quads") : nullptr;
                                 const base::ListValue* quad =
                                     quads && !quads->empty() ? (*quads)[0].GetIfList() : nullptr;
                                 if (!quad || quad->size() < 8) {
                                   std::move(then).Run(std::nullopt);
                                   return;
                                 }
                                 double x = 0;
                                 double y = 0;
                                 for (size_t corner = 0; corner < 8; corner += 2) {
                                   x += (*quad)[corner].GetIfDouble().value_or(0);
                                   y += (*quad)[corner + 1].GetIfDouble().value_or(0);
                                 }
                                 std::move(then).Run(std::pair(x / 4, y / 4));
                               },
                               std::move(then)));
                     },
                     page, element, std::move(then)));
           },
           weak_factory_.GetWeakPtr(), element, std::move(then)));
}

void AgentPage::IsPassword(int64_t element, base::OnceCallback<void(std::optional<bool>)> then) {
  Send("DOM.describeNode", base::DictValue().Set("backendNodeId", static_cast<int>(element)),
       base::BindOnce(
           [](base::OnceCallback<void(std::optional<bool>)> then, const base::DictValue* result) {
             const base::DictValue* node = result ? result->FindDict("node") : nullptr;
             if (!node) {
               std::move(then).Run(std::nullopt);
               return;
             }
             const base::ListValue* attributes = node->FindList("attributes");
             bool password = false;
             // Name, value, name, value.
             for (size_t i = 0; attributes && i + 1 < attributes->size(); i += 2) {
               const std::string* name = (*attributes)[i].GetIfString();
               const std::string* value = (*attributes)[i + 1].GetIfString();
               if (name && value && base::EqualsCaseInsensitiveASCII(*name, "type") &&
                   base::EqualsCaseInsensitiveASCII(*value, "password")) {
                 password = true;
               }
             }
             std::move(then).Run(password);
           },
           std::move(then)));
}

void AgentPage::HideSuggestions() {
  if (autofill::ContentAutofillClient* client =
          autofill::ContentAutofillClient::FromWebContents(&GetWebContents())) {
    client->HideSuggestions(autofill::SuggestionHidingReason::kHiddenByCaller, std::nullopt);
  }
}

void AgentPage::Answer(std::string text, bool error) {
  if (!job_) {
    return;
  }
  const uint64_t request = job_->request;
  job_.reset();
  wait_.Stop();
  loaded_.Reset();
  // Posted, so the app never hears inside its own bc_agent_page.
  base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
      FROM_HERE, base::BindOnce(&Report, tab_, request, std::move(text), error));
  Next();
  if (!job_) {
    // A hidden page left drawing would keep its animations waking the browser while the agent
    // thinks or has gone quiet.
    rest_.Start(FROM_HERE, kRest,
                base::BindOnce([](AgentPage* page) { page->drawing_.RunAndReset(); },
                               base::Unretained(this)));
  }
}

void AgentPage::Fail(std::string why) {
  Answer(std::move(why), /*error=*/true);
}

void AgentPage::LetGo() {
  std::deque<Job> owed = std::exchange(jobs_, {});
  if (job_) {
    owed.push_front(*std::move(job_));
    job_.reset();
  }
  for (const Job& job : owed) {
    base::SingleThreadTaskRunner::GetCurrentDefault()->PostTask(
        FROM_HERE, base::BindOnce(&Report, tab_, job.request,
                                  std::string("The agent's session with the tab ended."), true));
  }
  weak_factory_.InvalidateWeakPtrs();
  wait_.Stop();
  rest_.Stop();
  loaded_.Reset();
  drawing_.RunAndReset();
  Detach();
}

void AgentPage::Send(const char* method, base::DictValue params, Reply reply) {
  if (!agent_host_) {
    scoped_refptr<content::DevToolsAgentHost> host =
        content::DevToolsAgentHost::GetOrCreateFor(&GetWebContents());
    // The profile's policy may keep DevTools from the page, which leaves agents without it too.
    if (!host || !host->AttachClient(this)) {
      std::move(reply).Run(nullptr);
      return;
    }
    agent_host_ = std::move(host);
  }
  const int id = next_command_++;
  std::optional<std::string> json = base::WriteJson(
      base::DictValue().Set("id", id).Set("method", method).Set("params", std::move(params)));
  if (!json) {
    std::move(reply).Run(nullptr);
    return;
  }
  replies_[id] = std::move(reply);
  agent_host_->DispatchProtocolMessage(this, base::as_byte_span(*json));
}

void AgentPage::Detach() {
  replies_.clear();
  if (scoped_refptr<content::DevToolsAgentHost> host = std::move(agent_host_)) {
    host->DetachClient(this);
  }
}

void AgentPage::DispatchProtocolMessage(content::DevToolsAgentHost* agent_host,
                                        base::span<const uint8_t> message) {
  std::optional<base::DictValue> dict = base::JSONReader::ReadDict(
      std::string_view(reinterpret_cast<const char*>(message.data()), message.size()),
      base::JSON_PARSE_RFC);
  std::optional<int> id = dict ? dict->FindInt("id") : std::nullopt;
  // Events, which nothing here asks for, have no id.
  if (!id) {
    return;
  }
  auto reply = replies_.find(*id);
  if (reply == replies_.end()) {
    return;
  }
  Reply run = std::move(reply->second);
  replies_.erase(reply);
  // An error has no result, which each reply takes for a failure.
  std::move(run).Run(dict->FindDict("result"));
}

void AgentPage::AgentHostClosed(content::DevToolsAgentHost* agent_host) {
  agent_host_ = nullptr;
  // The page's renderer went: what was asked is answered as nothing.
  base::flat_map<int, Reply> replies = std::move(replies_);
  replies_.clear();
  for (auto& [id, reply] : replies) {
    std::move(reply).Run(nullptr);
  }
}

void AgentPage::DidStopLoading() {
  // Only a wait for the load ends here. A load that stops while an action settles, as a page from
  // the cache can within the settle's time, leaves the settle to snapshot the page it loaded.
  if (loaded_) {
    wait_.Stop();
    std::move(loaded_).Run();
  }
}

void AgentPage::WebContentsDestroyed() {
  // The tab is closing: what's asked is answered now, as the replies never will be.
  LetGo();
}

WEB_CONTENTS_USER_DATA_KEY_IMPL(AgentPage);

}  // namespace browser_core
