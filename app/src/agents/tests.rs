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
    let answer = call(&agents, "claude-code", "click", json!({}), cx)
        .recv()
        .unwrap();
    assert_eq!(answer, ToolResult::error("There's no tool click."));
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
