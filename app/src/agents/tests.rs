//! Settings › Agents and the tools, as the user and an agent use them, on a browser that makes up
//! its Spaces and tabs and records what the agents ask of it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime};

use gpui::{App, AppContext, Entity, TestAppContext};
use serde_json::json;

use super::*;

/// The user's Space 1 with the tab 10, on Wikipedia, and Claude Code's Space 2 with the tab 20, on
/// example.com, once it has one.
#[derive(Default)]
struct Made {
    agent_spaces: RefCell<HashMap<String, u64>>,
    /// Each tab's Space, title and URL.
    tabs: RefCell<Vec<(u64, u64, &'static str, &'static str)>>,
    made: RefCell<Vec<(String, String)>>,
    reads: RefCell<Vec<(u64, u64)>>,
    disconnected: RefCell<Vec<String>>,
    /// What the agents asked the core to do in a page: the tab, the request, the action, the
    /// element and the text.
    pages: RefCell<Vec<(u64, u64, PageAction, i64, String)>>,
    navigated: RefCell<Vec<(u64, String)>>,
    /// The question each window shows, as the agents last told them.
    asked: RefCell<Option<Question>>,
}

impl Made {
    fn with_agent_space() -> Rc<Made> {
        let made = Made::default();
        made.agent_spaces
            .borrow_mut()
            .insert("claude-code".into(), 2);
        made.tabs.borrow_mut().extend([
            (
                10,
                1,
                "Arc (web browser)",
                "https://en.wikipedia.org/wiki/Arc_(web_browser)",
            ),
            (20, 2, "Example Domain", "https://www.example.com/"),
        ]);
        Rc::new(made)
    }
}

impl Browser for Made {
    fn agent_space(&self, agent: &str) -> u64 {
        self.agent_spaces.borrow().get(agent).copied().unwrap_or(0)
    }

    fn make_agent_space(&self, agent: &str, name: &str, _: &mut App) -> u64 {
        self.made.borrow_mut().push((agent.into(), name.into()));
        self.agent_spaces.borrow_mut().insert(agent.into(), 2);
        2
    }

    fn space_tabs(&self, space: u64, _: &App) -> Vec<TabInfo> {
        (self.tabs.borrow().iter())
            .filter(|tab| tab.1 == space)
            .map(|&(id, _, title, url)| TabInfo {
                id,
                title: title.into(),
                url: url.into(),
            })
            .collect()
    }

    fn tab(&self, tab: u64, _: &App) -> TabInfo {
        let tabs = self.tabs.borrow();
        let found = tabs.iter().find(|each| each.0 == tab);
        found.map_or(
            TabInfo {
                id: tab,
                ..Default::default()
            },
            |&(id, _, title, url)| TabInfo {
                id,
                title: title.into(),
                url: url.into(),
            },
        )
    }

    fn may_act(&self, agent: &str, tab: u64, _: &str, _: &mut App) -> Result<(), String> {
        let space = self.agent_space(agent);
        match self
            .tabs
            .borrow()
            .iter()
            .any(|each| each.0 == tab && each.1 == space && space != 0)
        {
            true => Ok(()),
            false => Err("The tab isn't in the agent's own Space.".into()),
        }
    }

    fn read_text(&self, tab: u64, request: u64) {
        self.reads.borrow_mut().push((tab, request));
    }

    fn disconnected(&self, agent: &str, _: &mut App) {
        self.disconnected.borrow_mut().push(agent.into());
    }

    /// The new tab is 30.
    fn open_tab(&self, agent: &str, url: &str, _: &mut App) -> u64 {
        let space = self.agent_space(agent);
        let url: &'static str = Box::leak(url.to_string().into_boxed_str());
        self.tabs.borrow_mut().push((30, space, "Opened", url));
        30
    }

    fn navigate(&self, _: &str, tab: u64, url: &str, _: &mut App) -> bool {
        self.navigated.borrow_mut().push((tab, url.into()));
        true
    }

    fn page(&self, _: &str, tab: u64, request: u64, action: PageAction, element: i64, text: &str) {
        (self.pages.borrow_mut()).push((tab, request, action, element, text.into()));
    }

    /// A name with a dot is a site's, as the address bar takes it; other words are a search.
    fn url_for(&self, text: &str) -> Option<String> {
        Some(match (text.contains("://"), text.contains('.')) {
            (true, _) => text.into(),
            (false, true) => format!("https://{text}/"),
            (false, false) => format!("https://duckduckgo.com/?q={text}"),
        })
    }

    fn ask(&self, question: Option<Question>, _: &mut App) {
        *self.asked.borrow_mut() = question;
    }
}

/// The clients' configs as Connect and Disconnect left them.
#[derive(Default)]
struct Configs {
    connected: Mutex<Vec<(Client, Entry)>>,
    disconnected: Mutex<Vec<(Client, String)>>,
    /// Why the next Connect fails, as when Claude Code isn't installed.
    failing: Mutex<Option<String>>,
}

impl Clients for Configs {
    fn connect(&self, client: Client, entry: &Entry) -> Result<(), String> {
        if let Some(why) = self.failing.lock().unwrap().take() {
            return Err(why);
        }
        self.connected.lock().unwrap().push((client, entry.clone()));
        Ok(())
    }

    fn disconnect(&self, client: Client, name: &str) -> Result<(), String> {
        self.disconnected
            .lock()
            .unwrap()
            .push((client, name.into()));
        Ok(())
    }
}

fn agents(
    browser: &Rc<Made>,
    configs: &Arc<Configs>,
    path: Option<PathBuf>,
    cx: &mut TestAppContext,
) -> Entity<Agents> {
    let (browser, configs) = (browser.clone(), configs.clone());
    // A socket of the test's own, so the tests run beside a browser and each other.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let number = NEXT.fetch_add(1, Ordering::SeqCst);
    let folder = std::env::temp_dir().join(format!("agents-ui-{}-{number}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let (socket, bridge) = (folder.join("Agents.sock"), PathBuf::from("/app/mcp-bridge"));
    cx.new(|_| Agents::new(browser, configs, path, socket, bridge))
}

/// Claude Code, connected.
fn connected(cx: &mut TestAppContext) -> (Entity<Agents>, Rc<Made>, Arc<Configs>) {
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    let agents = agents(&browser, &configs, None, cx);
    agents.update(cx, |agents, cx| agents.connect(Client::ClaudeCode, cx));
    cx.run_until_parked();
    (agents, browser, configs)
}

/// Calls `tool` as the agent with the id `agent` would, and gives the answer's receiver.
fn call(
    agents: &Entity<Agents>,
    agent: &str,
    tool: &str,
    arguments: Value,
    cx: &mut TestAppContext,
) -> mpsc::Receiver<ToolResult> {
    let (reply, answer) = mpsc::channel();
    let call = Call {
        agent: agent.into(),
        tool: tool.into(),
        arguments,
        reply,
    };
    agents.update(cx, |agents, cx| agents.handle(Event::Call(call), cx));
    answer
}

fn log(
    agents: &Entity<Agents>,
    agent: &str,
    cx: &mut TestAppContext,
) -> Vec<(String, String, String)> {
    agents.read_with(cx, |agents, _| {
        (agents.activity(agent))
            .map(|entry| {
                (
                    entry.action.to_string(),
                    entry.site.to_string(),
                    entry.refused.to_string(),
                )
            })
            .collect()
    })
}

/// The switch is off at first, and the server runs only while it's on: on, its socket is in place
/// for the user alone and takes a connection; off, the socket is gone.
#[gpui::test]
fn agents_are_off_until_the_user_turns_them_on(cx: &mut TestAppContext) {
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    let agents = agents(&browser, &configs, None, cx);
    assert!(!agents.read_with(cx, |agents, _| agents.is_on()));
    assert!(agents.read_with(cx, |agents, _| agents.server.is_none()));
    let socket = agents.read_with(cx, |agents, _| agents.socket.clone());
    assert!(!socket.exists());

    agents.update(cx, |agents, cx| agents.set_on(true, cx));
    let mode = std::fs::metadata(&socket).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    assert!(UnixStream::connect(&socket).is_ok());

    agents.update(cx, |agents, cx| agents.set_on(false, cx));
    assert!(agents.read_with(cx, |agents, _| agents.server.is_none()));
    assert!(!socket.exists());
    assert!(
        UnixStream::connect(&socket).is_err(),
        "the server still takes connections once off"
    );
}

/// Connect writes the bridge, the socket and a token into the client's config, and the server
/// takes that token as the agent's. Connecting again replaces the token, and the old one stops
/// working.
#[gpui::test]
fn connect_gives_the_client_a_token_the_server_takes(cx: &mut TestAppContext) {
    let (agents, _, configs) = connected(cx);
    let (client, entry) = configs.connected.lock().unwrap()[0].clone();
    assert_eq!(client, Client::ClaudeCode);
    assert_eq!(entry.name, server_name());
    assert_eq!(entry.bridge, "/app/mcp-bridge");
    let socket = agents.read_with(cx, |agents, _| agents.socket.clone());
    assert_eq!(entry.socket, socket.to_str().unwrap());
    assert_eq!(entry.token.len(), 64);
    let tokens = |cx: &mut TestAppContext| {
        agents.read_with(cx, |agents, _| agents.tokens.lock().unwrap().clone())
    };
    assert_eq!(
        tokens(cx).get(&entry.token).map(String::as_str),
        Some("claude-code")
    );
    assert!(agents.read_with(cx, |agents, _| agents.is_connected(Client::ClaudeCode)));
    assert!(!agents.read_with(cx, |agents, _| agents.is_connected(Client::Codex)));

    agents.update(cx, |agents, cx| agents.connect(Client::ClaudeCode, cx));
    cx.run_until_parked();
    let again = configs.connected.lock().unwrap()[1].1.token.clone();
    assert_ne!(again, entry.token);
    assert_eq!(tokens(cx).len(), 1);
    assert!(tokens(cx).contains_key(&again));
}

/// A Connect that fails, as when the client isn't installed, connects nothing and says why on
/// the client's row.
#[gpui::test]
fn a_failed_connect_says_why(cx: &mut TestAppContext) {
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    *configs.failing.lock().unwrap() = Some("Claude Code's claude command isn't installed.".into());
    let agents = agents(&browser, &configs, None, cx);
    agents.update(cx, |agents, cx| agents.connect(Client::ClaudeCode, cx));
    assert!(agents.read_with(cx, |agents, _| agents.is_connecting(Client::ClaudeCode)));
    cx.run_until_parked();
    assert!(!agents.read_with(cx, |agents, _| agents.is_connected(Client::ClaudeCode)));
    let failure = agents.read_with(cx, |agents, _| agents.failure(Client::ClaudeCode));
    assert_eq!(
        failure.as_deref(),
        Some("Claude Code's claude command isn't installed.")
    );
    assert!(agents.read_with(cx, |agents, _| agents.tokens.lock().unwrap().is_empty()));
}

/// Any other client gets the address and its token to paste, once it's connected.
#[gpui::test]
fn another_client_gets_its_setup_to_paste(cx: &mut TestAppContext) {
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    let agents = agents(&browser, &configs, None, cx);
    assert_eq!(
        agents.read_with(cx, |agents, _| agents.setup(Client::Other)),
        None
    );
    agents.update(cx, |agents, cx| agents.connect(Client::Other, cx));
    cx.run_until_parked();
    let setup = agents
        .read_with(cx, |agents, _| agents.setup(Client::Other))
        .unwrap();
    let setup: Value = serde_json::from_str(&setup).unwrap();
    let server = &setup["mcpServers"][server_name()];
    let socket = agents.read_with(cx, |agents, _| agents.socket.clone());
    assert_eq!(server["type"], "stdio");
    assert_eq!(server["command"], "/app/mcp-bridge");
    assert_eq!(server["args"], json!([socket.to_str().unwrap()]));
    let token = configs.connected.lock().unwrap()[0].1.token.clone();
    assert_eq!(server["env"][bridge::TOKEN], token);
}

/// list_tabs makes the agent's Space the first time, named as the agent, and lists only its tabs,
/// never the user's.
#[gpui::test]
fn list_tabs_lists_only_the_agents_own_tabs(cx: &mut TestAppContext) {
    let (browser, configs) = (Rc::new(Made::default()), Arc::new(Configs::default()));
    browser
        .tabs
        .borrow_mut()
        .push((10, 1, "Arc (web browser)", "https://en.wikipedia.org/"));
    let agents = agents(&browser, &configs, None, cx);
    agents.update(cx, |agents, cx| agents.connect(Client::ClaudeCode, cx));
    cx.run_until_parked();

    let answer = call(&agents, "claude-code", "list_tabs", json!({}), cx)
        .recv()
        .unwrap();
    assert_eq!(
        *browser.made.borrow(),
        [("claude-code".to_string(), "Claude Code".to_string())]
    );
    assert!(!answer.error);
    assert!(answer.text.starts_with("Your Space has no tabs yet."));

    browser
        .tabs
        .borrow_mut()
        .push((20, 2, "Example Domain", "https://example.com/"));
    let answer = call(&agents, "claude-code", "list_tabs", json!({}), cx)
        .recv()
        .unwrap();
    assert_eq!(answer.text, "  20  Example Domain  https://example.com/");
    assert_eq!(
        browser.made.borrow().len(),
        1,
        "the agent got a second Space"
    );
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        ("list_tabs".into(), "".into(), "".into())
    );
}

/// read_page on a tab of the user's is refused, with the core's reason, which the activity log
/// keeps with the page's site, and the core reads nothing.
#[gpui::test]
fn reading_a_tab_of_the_users_is_refused_and_logged(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(
        &agents,
        "claude-code",
        "read_page",
        json!({ "tab": 10 }),
        cx,
    )
    .recv()
    .unwrap();
    assert_eq!(
        answer,
        ToolResult::error("The tab isn't in the agent's own Space.")
    );
    assert!(browser.reads.borrow().is_empty());
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        (
            "read_page".into(),
            "en.wikipedia.org".into(),
            "The tab isn't in the agent's own Space.".into()
        )
    );
}

/// read_page on the agent's own tab asks the core for its text, and answers with it once it
/// comes, saying which page it is. Without a tab, it reads the one the agent used last.
#[gpui::test]
fn read_page_reads_the_agents_own_tab(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(
        &agents,
        "claude-code",
        "read_page",
        json!({ "tab": "20" }),
        cx,
    );
    let (tab, request) = browser.reads.borrow()[0];
    assert_eq!(tab, 20);
    assert!(
        answer.try_recv().is_err(),
        "it answered before the core read the page"
    );
    agents.update(cx, |agents, cx| {
        agents.text_read(
            request,
            Some("Example Domain\nThis domain is for examples.".into()),
            cx,
        )
    });
    let answer = answer.recv().unwrap();
    assert!(!answer.error);
    assert_eq!(
        answer.text,
        "Tab 20: Example Domain\nURL: https://www.example.com/\n\n\
         Example Domain\nThis domain is for examples."
    );
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        ("read_page".into(), "example.com".into(), "".into())
    );

    let answer = call(&agents, "claude-code", "read_page", json!({}), cx);
    let (tab, request) = browser.reads.borrow()[1];
    assert_eq!(tab, 20);
    agents.update(cx, |agents, cx| agents.text_read(request, None, cx));
    assert!(
        answer.recv().unwrap().error,
        "a tab without its page read as text"
    );
}

/// A page's text longer than an agent should take in is cut, and says so.
#[test]
fn a_long_page_is_cut() {
    let tab = TabInfo {
        id: 1,
        title: "Long".into(),
        url: "https://long.example/".into(),
    };
    let text = "é".repeat(tools::MAX_TEXT);
    let answer = tools::page_text(&tab, &text);
    assert!(answer.ends_with("[The page goes on: this is its first part.]"));
    assert!(answer.len() < tools::MAX_TEXT + 200);
}

/// An agent the user hasn't connected, or one they disconnected, gets nothing.
#[gpui::test]
fn an_agent_not_connected_gets_nothing(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(&agents, "codex", "list_tabs", json!({}), cx)
        .recv()
        .unwrap();
    assert_eq!(answer, ToolResult::error("This agent isn't connected."));
    let answer = call(&agents, "claude-code", "drag", json!({}), cx)
        .recv()
        .unwrap();
    assert_eq!(answer, ToolResult::error("There's no tool drag."));
    assert!(browser.made.borrow().is_empty());
}

/// Disconnect takes the server out of the client's config, forgets the agent's token, ends its
/// sessions in the core and answers no read it was waiting for. Its Space stays.
#[gpui::test]
fn disconnect_ends_the_agents_sessions_and_forgets_its_token(cx: &mut TestAppContext) {
    let (agents, browser, configs) = connected(cx);
    agents.update(cx, |agents, cx| {
        let hello = Event::Hello {
            agent: "claude-code".into(),
            client: "claude-code".into(),
        };
        agents.handle(hello, cx)
    });
    assert!(agents.read_with(cx, |agents, _| agents.is_live(Client::ClaudeCode)));
    let waiting = call(
        &agents,
        "claude-code",
        "read_page",
        json!({ "tab": 20 }),
        cx,
    );

    agents.update(cx, |agents, cx| agents.disconnect(Client::ClaudeCode, cx));
    cx.run_until_parked();
    assert_eq!(*browser.disconnected.borrow(), ["claude-code"]);
    assert_eq!(
        *configs.disconnected.lock().unwrap(),
        [(Client::ClaudeCode, server_name())]
    );
    assert!(agents.read_with(cx, |agents, _| agents.tokens.lock().unwrap().is_empty()));
    assert!(
        waiting.recv().is_err(),
        "a read went on for a disconnected agent"
    );
    assert_eq!(log(&agents, "claude-code", cx)[0].0, "disconnected");
    let answer = call(&agents, "claude-code", "list_tabs", json!({}), cx)
        .recv()
        .unwrap();
    assert!(answer.error);
}

/// Turning agents off ends every live agent's sessions and answers the calls waiting, and keeps
/// the agents connected for when it's on again.
#[gpui::test]
fn turning_agents_off_ends_their_sessions(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    agents.update(cx, |agents, cx| agents.set_on(true, cx));
    let waiting = call(
        &agents,
        "claude-code",
        "read_page",
        json!({ "tab": 20 }),
        cx,
    );
    agents.update(cx, |agents, cx| agents.set_on(false, cx));
    assert!(waiting.recv().is_err());
    assert_eq!(*browser.disconnected.borrow(), ["claude-code"]);
    assert!(agents.read_with(cx, |agents, _| agents.is_connected(Client::ClaudeCode)));
}

/// What's saved: the switch and each agent's token, in a file only the user can read, which a
/// later launch loads, starting the server if it was on.
#[gpui::test]
fn the_switch_and_the_agents_come_back(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("agents-test-{}", random_token()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Agents");
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    let agents = agents(&browser, &configs, Some(path.clone()), cx);
    agents.update(cx, |agents, cx| agents.connect(Client::Codex, cx));
    cx.run_until_parked();
    agents.update(cx, |agents, cx| agents.set_on(true, cx));
    let saved = std::fs::read_to_string(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    agents.update(cx, |agents, cx| agents.set_on(false, cx));

    let later = self::agents(&browser, &configs, None, cx);
    later.update(cx, |agents, cx| agents.load(&saved, cx));
    assert!(later.read_with(cx, |agents, _| agents.is_on() && agents.server.is_some()));
    assert!(later.read_with(cx, |agents, _| agents.is_connected(Client::Codex)));
    let token = configs.connected.lock().unwrap()[0].1.token.clone();
    let tokens = later.read_with(cx, |agents, _| agents.tokens.lock().unwrap().clone());
    assert_eq!(tokens.get(&token).map(String::as_str), Some("codex"));
    later.update(cx, |agents, cx| agents.load("", cx));
    assert!(
        later.read_with(cx, |agents, _| !agents.is_on()),
        "a lost file left agents on"
    );
    std::fs::remove_dir_all(dir).ok();
}

/// The tree the core gives for tab 20's page: a heading, a field, a password field and a button
/// that orders.
fn shop_tree() -> String {
    json!([
        { "nodeId": "1", "role": { "value": "RootWebArea" }, "name": { "value": "Shop" },
          "childIds": ["2", "3", "4", "5"] },
        { "nodeId": "2", "role": { "value": "heading" }, "name": { "value": "Basket" } },
        { "nodeId": "3", "role": { "value": "textbox" }, "name": { "value": "Coupon" },
          "backendDOMNodeId": 31 },
        { "nodeId": "4", "role": { "value": "textbox" }, "name": { "value": "Password" },
          "backendDOMNodeId": 41, "password": true },
        { "nodeId": "5", "role": { "value": "button" }, "name": { "value": "Place order" },
          "backendDOMNodeId": 51 },
    ])
    .to_string()
}

/// The core's answer to the agents' last request of it, and the call's answer.
fn page_done(
    agents: &Entity<Agents>,
    browser: &Made,
    answer: mpsc::Receiver<ToolResult>,
    text: String,
    cx: &mut TestAppContext,
) -> ToolResult {
    let request = browser.pages.borrow().last().unwrap().1;
    agents.update(cx, |agents, cx| agents.page_done(request, text, false, cx));
    answer.recv().unwrap()
}

/// An agent opens a page in its Space, gets its outline with refs, and acts by ref: each action
/// goes to the core with the element's node, and answers with what it did and the page after.
#[gpui::test]
fn an_agent_opens_a_page_and_acts_by_ref(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(&agents, "claude-code", "open_tab", json!({ "url": "example.com" }), cx);
    let (tab, _, action, _, _) = browser.pages.borrow()[0].clone();
    assert_eq!((tab, action), (30, PageAction::Snapshot));
    let opened = page_done(&agents, &browser, answer, shop_tree(), cx);
    assert!(
        opened.text.starts_with("Opened tab 30.\n\nTab 30: Shop\nURL: https://example.com/"),
        "{}",
        opened.text
    );
    assert!(opened.text.contains("- textbox \"Coupon\" [e1]"));
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        ("open_tab".into(), "example.com".into(), "".into())
    );

    // Without a tab, the one it opened.
    let answer = call(&agents, "claude-code", "type", json!({ "ref": "e1", "text": "SAVE10" }), cx);
    let (tab, _, action, element, text) = browser.pages.borrow().last().cloned().unwrap();
    assert_eq!((tab, action, element, text.as_str()), (30, PageAction::Type, 31, "SAVE10"));
    let typed = page_done(&agents, &browser, answer, shop_tree(), cx);
    assert!(typed.text.starts_with("Typed into “Coupon”.\n\nTab 30"));
    assert_eq!(log(&agents, "claude-code", cx)[0].0, "type");

    let answer = call(&agents, "claude-code", "press_key", json!({ "key": "Tab" }), cx);
    assert_eq!(browser.pages.borrow().last().unwrap().4, "Tab");
    assert!(!page_done(&agents, &browser, answer, shop_tree(), cx).error);

    let answer = call(&agents, "claude-code", "screenshot", json!({}), cx);
    let picture = page_done(&agents, &browser, answer, "/9j/4AAQ".into(), cx);
    assert_eq!(
        picture.to_json()["content"][0],
        json!({ "type": "image", "data": "/9j/4AAQ", "mimeType": "image/jpeg" })
    );

    let answer = call(&agents, "claude-code", "navigate", json!({ "url": "back" }), cx);
    assert_eq!(browser.pages.borrow().last().unwrap().2, PageAction::Back);
    assert!(page_done(&agents, &browser, answer, shop_tree(), cx).text.starts_with("Went back."));

    let answer = call(&agents, "claude-code", "navigate", json!({ "url": "iana.org" }), cx);
    assert_eq!(browser.navigated.borrow()[0], (30, "https://iana.org/".into()));
    assert!(!page_done(&agents, &browser, answer, shop_tree(), cx).error);

    let pages = browser.pages.borrow().len();
    let stale = call(&agents, "claude-code", "click", json!({ "ref": "e9" }), cx);
    assert!(stale.recv().unwrap().text.starts_with("There's no element e9 in tab 30."));
    let key = call(&agents, "claude-code", "press_key", json!({ "key": "F5" }), cx);
    assert!(key.recv().unwrap().error);
    assert_eq!(browser.pages.borrow().len(), pages, "a bad call reached the core");

    // The core's error is the call's.
    let answer = call(&agents, "claude-code", "click", json!({ "ref": "e1" }), cx);
    let request = browser.pages.borrow().last().unwrap().1;
    agents.update(cx, |agents, cx| agents.page_done(request, "It's gone.".into(), true, cx));
    assert_eq!(answer.recv().unwrap(), ToolResult::error("It's gone."));
}

/// An agent can't act on the user's tabs, type into a password field or open a page that isn't
/// on the web; each refusal is logged.
#[gpui::test]
fn what_an_agent_may_not_do_is_refused_and_logged(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(&agents, "claude-code", "click", json!({ "tab": 10, "ref": "e1" }), cx);
    assert!(answer.recv().unwrap().error);
    let answer = call(&agents, "claude-code", "snapshot", json!({ "tab": 10 }), cx);
    assert_eq!(
        answer.recv().unwrap(),
        ToolResult::error("The tab isn't in the agent's own Space.")
    );

    let answer = call(&agents, "claude-code", "snapshot", json!({ "tab": 20 }), cx);
    let outline = page_done(&agents, &browser, answer, shop_tree(), cx).text;
    assert!(outline.contains("- textbox \"Password\" [e2] (password: off limits)"));
    let pages = browser.pages.borrow().len();
    let typing = json!({ "ref": "e2", "text": "hunter2" });
    let answer = call(&agents, "claude-code", "type", typing, cx);
    assert!(answer.recv().unwrap().text.starts_with("Password fields are off limits"));
    assert_eq!(browser.pages.borrow().len(), pages, "the password reached the core");
    assert_eq!(log(&agents, "claude-code", cx)[0].0, "type");
    assert!(log(&agents, "claude-code", cx)[0].2.starts_with("Password fields are off limits"));

    let file = json!({ "url": "file:///etc/passwd" });
    let answer = call(&agents, "claude-code", "open_tab", file, cx);
    assert_eq!(
        answer.recv().unwrap(),
        ToolResult::error("Agents may open only http and https pages.")
    );
    let settings = json!({ "url": "chrome://settings" });
    let answer = call(&agents, "claude-code", "navigate", settings, cx);
    assert!(answer.recv().unwrap().error);
    assert!(browser.navigated.borrow().is_empty());
    assert_eq!(log(&agents, "claude-code", cx)[0].0, "navigate");
}

/// An order asks the user first, in the card: Allow Once lets it go this time and asks again
/// next time, Always Allow lets it go on the site from then on, and is saved.
#[gpui::test]
fn an_order_asks_the_user_first(cx: &mut TestAppContext) {
    let dir = std::env::temp_dir().join(format!("agents-test-{}", random_token()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Agents");
    let (browser, configs) = (Made::with_agent_space(), Arc::new(Configs::default()));
    let agents = agents(&browser, &configs, Some(path.clone()), cx);
    agents.update(cx, |agents, cx| agents.connect(Client::ClaudeCode, cx));
    cx.run_until_parked();
    let answer = call(&agents, "claude-code", "snapshot", json!({ "tab": 20 }), cx);
    page_done(&agents, &browser, answer, shop_tree(), cx);
    let pages = browser.pages.borrow().len();

    let order = |cx: &mut TestAppContext| {
        call(&agents, "claude-code", "click", json!({ "tab": 20, "ref": "e3" }), cx)
    };
    let answer = order(cx);
    let question = browser.asked.borrow().clone().expect("the order didn't ask");
    assert_eq!(question.agent, "Claude Code");
    assert_eq!(question.what, "click “Place order” on example.com");
    assert_eq!(question.why, "It looks like a payment or an order.");
    assert_eq!(browser.pages.borrow().len(), pages, "it clicked before the user answered");
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Once, cx));
    assert_eq!(*browser.asked.borrow(), None);
    assert_eq!(browser.pages.borrow().last().unwrap().3, 51);
    let clicked = page_done(&agents, &browser, answer, shop_tree(), cx);
    assert!(clicked.text.starts_with("Clicked button “Place order”."));

    let answer = order(cx);
    let question = browser.asked.borrow().clone().expect("once let it go again");
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Always, cx));
    assert!(!page_done(&agents, &browser, answer, shop_tree(), cx).error);
    let answer = order(cx);
    assert_eq!(*browser.asked.borrow(), None, "always asked again");
    assert!(!page_done(&agents, &browser, answer, shop_tree(), cx).error);

    let saved = std::fs::read_to_string(&path).unwrap();
    let later = self::agents(&browser, &configs, None, cx);
    later.update(cx, |agents, cx| agents.load(&saved, cx));
    let grants = later.read_with(cx, |agents, _| agents.grants.clone());
    assert_eq!(grants["claude-code"]["example.com"], true);
    std::fs::remove_dir_all(dir).ok();
}

/// What the user answered for good is listed, by site, and once revoked, one site or all, what
/// asks first there asks again. Each revoke is logged.
#[gpui::test]
fn a_permission_given_for_good_is_listed_and_revoked(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(&agents, "claude-code", "snapshot", json!({ "tab": 20 }), cx);
    page_done(&agents, &browser, answer, shop_tree(), cx);
    let order = |cx: &mut TestAppContext| {
        call(&agents, "claude-code", "click", json!({ "tab": 20, "ref": "e3" }), cx)
    };
    let answer = order(cx);
    let question = browser.asked.borrow().clone().unwrap();
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Always, cx));
    page_done(&agents, &browser, answer, shop_tree(), cx);
    let answer = call(&agents, "claude-code", "open_tab", json!({ "url": "chase.com" }), cx);
    let question = browser.asked.borrow().clone().unwrap();
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Never, cx));
    assert!(answer.recv().unwrap().error);
    let grants = |cx: &mut TestAppContext| {
        agents.read_with(cx, |agents, _| agents.grants("claude-code"))
    };
    assert_eq!(grants(cx), [("chase.com".into(), false), ("example.com".into(), true)]);

    agents.update(cx, |agents, cx| agents.revoke("claude-code", Some("example.com"), cx));
    assert_eq!(grants(cx), [("chase.com".into(), false)]);
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        ("revoked".into(), "example.com".into(), "".into())
    );
    let answer = order(cx);
    let question = browser.asked.borrow().clone().expect("a revoked Always didn't ask again");
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Once, cx));
    assert!(!page_done(&agents, &browser, answer, shop_tree(), cx).error);

    agents.update(cx, |agents, cx| agents.revoke("claude-code", None, cx));
    assert!(grants(cx).is_empty());
    assert_eq!(log(&agents, "claude-code", cx)[0], ("revoked all".into(), "".into(), "".into()));
    let entries = log(&agents, "claude-code", cx).len();
    agents.update(cx, |agents, cx| agents.revoke("claude-code", None, cx));
    assert_eq!(log(&agents, "claude-code", cx).len(), entries, "revoking nothing was logged");
    call(&agents, "claude-code", "open_tab", json!({ "url": "chase.com" }), cx);
    assert!(browser.asked.borrow().is_some(), "a revoked Never didn't ask again");
}

/// A bank's site asks before it opens. No, or the ×, refuses it, and Never refuses it from then
/// on without asking; each is logged.
#[gpui::test]
fn no_to_an_agent_is_kept(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let open = |cx: &mut TestAppContext| {
        call(&agents, "claude-code", "open_tab", json!({ "url": "chase.com" }), cx)
    };
    let answer = open(cx);
    let question = browser.asked.borrow().clone().unwrap();
    assert_eq!(question.what, "open chase.com");
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Dismiss, cx));
    assert_eq!(
        answer.recv().unwrap(),
        ToolResult::error("The user said no: you can't open chase.com. Don't try again.")
    );
    assert_eq!(
        log(&agents, "claude-code", cx)[0],
        ("open_tab".into(), "chase.com".into(), "You said no.".into())
    );

    let answer = open(cx);
    let question = browser.asked.borrow().clone().unwrap();
    agents.update(cx, |agents, cx| agents.answer(question.id, Answer::Never, cx));
    assert!(answer.recv().unwrap().error);
    let answer = open(cx);
    assert_eq!(*browser.asked.borrow(), None, "never asked again");
    assert!(answer.recv().unwrap().text.starts_with("The user said never on chase.com"));
    assert!(browser.pages.borrow().is_empty());
    assert!(browser.tabs.borrow().iter().all(|tab| tab.0 != 30));
}

/// A question goes with its agent's session, and the card with it.
#[gpui::test]
fn a_question_goes_with_its_agent(cx: &mut TestAppContext) {
    let (agents, browser, _) = connected(cx);
    let answer = call(&agents, "claude-code", "open_tab", json!({ "url": "chase.com" }), cx);
    assert!(browser.asked.borrow().is_some());
    agents.update(cx, |agents, cx| {
        agents.handle(Event::Hello { agent: "claude-code".into(), client: "check".into() }, cx);
        agents.handle(Event::Bye { agent: "claude-code".into() }, cx);
    });
    assert_eq!(*browser.asked.borrow(), None);
    assert!(answer.recv().is_err());
}

/// The driver's calls, checked by the core outside the tools, come to the log too, and a session
/// the user took back has its entry.
#[gpui::test]
fn the_cores_reports_come_to_the_log(cx: &mut TestAppContext) {
    let (agents, _, _) = connected(cx);
    agents.update(cx, |agents, cx| {
        agents.checked(
            "claude-code",
            10,
            "act",
            "The tab isn't in the agent's own Space.",
            cx,
        );
        agents.session_ended("claude-code", 20, "taken back", cx);
        agents.session_ended("claude-code", 20, "disconnected", cx);
    });
    let log = log(&agents, "claude-code", cx);
    assert_eq!(
        log[0],
        ("taken back".into(), "example.com".into(), "".into())
    );
    assert_eq!(log[1].1, "en.wikipedia.org");
    assert_eq!(tools::describe(&log[0].0), "You took the tab back");
    assert!(log.iter().all(|entry| entry.0 != "disconnected"));
}

#[test]
fn a_sites_host_is_its_name() {
    assert_eq!(site("https://www.example.com/a?b"), "example.com");
    assert_eq!(site("http://user@localhost:8000/"), "localhost");
    assert_eq!(site("https://[::1]:8080/"), "[::1]");
    assert_eq!(site("about:blank"), "");
}

#[test]
fn the_log_says_how_long_ago() {
    let now = SystemTime::now();
    assert_eq!(ago(now, now), "Just now");
    assert_eq!(ago(now - Duration::from_secs(150), now), "2 min ago");
    assert_eq!(ago(now - Duration::from_secs(7200), now), "2 h ago");
    assert_eq!(ago(now + Duration::from_secs(5), now), "Just now");
}
