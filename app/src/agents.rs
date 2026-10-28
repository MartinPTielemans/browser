//! Agents: Claude Code, Codex or any other MCP client works in the browser, each in a Space of its
//! own, through the browser's MCP server (decision 8). Settings › Agents turns the server on, which
//! it isn't at first, connects the clients and lists what each agent did. The server's threads
//! sleep in the kernel between requests, so an idle browser wakes for none of it, and nothing here
//! runs on tab switch, new tab or launch: the saved settings are read after the first window shows.
//!
//! A client proves which agent it is with the token Connect wrote into its config. Each tool call
//! reaches the UI thread as an event, and the tools answer through the core, which checks each
//! call on a tab with bc_agent_may_act. That check's answer is the call's activity-log entry.

pub mod clients;
mod server;
pub mod tools;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, mpsc};
use std::time::SystemTime;

use futures::StreamExt;
use gpui::{App, AppContext, Context, Entity, Global, SharedString, Task};
use serde_json::{Value, json};

pub use clients::{Client, Clients, Entry, System};

/// The port the server listens on, on 127.0.0.1: fixed, as the clients' configs name it. A dev
/// build has its own, so it runs beside the user's copy.
pub const PORT: u16 = if cfg!(feature = "prod") { 47615 } else { 47616 };

/// The name the clients list the server by: the browser's, a dev build's apart.
pub fn server_name() -> String {
    let name = env!("APP_NAME").to_lowercase();
    match cfg!(feature = "prod") {
        true => name,
        false => format!("{name}-dev"),
    }
}

/// A tab as an agent sees it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabInfo {
    pub id: u64,
    pub title: SharedString,
    pub url: SharedString,
}

/// What the agents ask of the browser: the core, in main.rs, or what a test makes up. Calls into
/// the core take the app, as one may tell the windows something before it returns.
pub trait Browser {
    /// The Space of the agent with the id `agent`, or 0 if it has none.
    fn agent_space(&self, agent: &str) -> u64;
    /// Makes the agent's Space, named as the agent, and returns it.
    fn make_agent_space(&self, agent: &str, name: &str, cx: &mut App) -> u64;
    /// The tabs of `space`, pinned first, then Today's.
    fn space_tabs(&self, space: u64, cx: &App) -> Vec<TabInfo>;
    /// `tab`'s title and URL.
    fn tab(&self, tab: u64, cx: &App) -> TabInfo;
    /// Asks the core whether the agent may do `action` to `tab` (bc_agent_may_act): nothing, or
    /// why not.
    fn may_act(&self, agent: &str, tab: u64, action: &str, cx: &mut App) -> Result<(), String>;
    /// Asks the core for the text of `tab`'s page (bc_tab_read_text), which comes to
    /// `Agents::text_read` with `request`.
    fn read_text(&self, tab: u64, request: u64);
    /// Tells the core the agent's client went (bc_agent_disconnected).
    fn disconnected(&self, agent: &str, cx: &mut App);
}

/// What the server's threads hand the UI thread.
pub enum Event {
    /// A client began a session as the agent with the id `agent`. `client` is the name it gave.
    Hello {
        agent: String,
        client: String,
    },
    /// The agent's last session ended.
    Bye {
        agent: String,
    },
    Call(Call),
}

/// A tool call, which the server's thread waits on until `reply` has the answer.
pub struct Call {
    pub agent: String,
    pub tool: String,
    pub arguments: Value,
    pub reply: mpsc::Sender<ToolResult>,
}

/// A tool's answer.
#[derive(Debug, PartialEq)]
pub struct ToolResult {
    pub text: String,
    pub error: bool,
}

impl ToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        ToolResult {
            text: text.into(),
            error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        ToolResult {
            text: text.into(),
            error: true,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({ "content": [{ "type": "text", "text": self.text }], "isError": self.error })
    }
}

/// An entry of the activity log: what an agent did, or was refused, where and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub at: SystemTime,
    /// The agent's id.
    pub agent: SharedString,
    /// What it did, as the core's entry names it, such as "read_page" (`tools::describe`).
    pub action: SharedString,
    /// The host of the page it did it to, or empty.
    pub site: SharedString,
    /// Why it was refused, or empty if it wasn't.
    pub refused: SharedString,
}

/// An agent the user connected.
#[derive(Clone, Debug)]
pub struct Agent {
    pub client: Client,
    token: String,
}

/// The entries the log keeps, all agents' together, as they're for this run of the browser.
const LOG_LENGTH: usize = 1000;

pub struct Agents {
    browser: Rc<dyn Browser>,
    clients: Arc<dyn Clients>,
    /// Where the settings are saved: None in the tests.
    path: Option<PathBuf>,
    port: u16,
    on: bool,
    agents: Vec<Agent>,
    /// The agents' tokens, which the server's threads read.
    tokens: server::Tokens,
    server: Option<(server::Server, Task<()>)>,
    /// Why the server isn't running, though on.
    trouble: Option<SharedString>,
    /// The client whose Connect is writing its config, and the client whose last failed, and why.
    connecting: Option<Client>,
    failed: Option<(Client, SharedString)>,
    /// The agents with a session open.
    live: HashSet<String>,
    log: Vec<Activity>,
    /// The reads waiting on the core, by request.
    reading: HashMap<u64, (String, u64, mpsc::Sender<ToolResult>)>,
    next_request: u64,
    /// The tab each agent used last, which a call without a tab uses.
    current: HashMap<String, u64>,
}

/// The app's agents, once main.rs has made them.
pub struct GlobalAgents(pub Entity<Agents>);

impl Global for GlobalAgents {}

pub fn global(cx: &App) -> Option<Entity<Agents>> {
    cx.try_global::<GlobalAgents>()
        .map(|agents| agents.0.clone())
}

impl Agents {
    /// Off, with none connected, until `load` reads what was saved.
    pub fn new(
        browser: Rc<dyn Browser>,
        clients: Arc<dyn Clients>,
        path: Option<PathBuf>,
        port: u16,
    ) -> Self {
        Agents {
            browser,
            clients,
            path,
            port,
            on: false,
            agents: Vec::new(),
            tokens: Arc::default(),
            server: None,
            trouble: None,
            connecting: None,
            failed: None,
            live: HashSet::new(),
            log: Vec::new(),
            reading: HashMap::new(),
            next_request: 1,
            current: HashMap::new(),
        }
    }

    /// Takes what `saved` says, from the settings file, and starts the server if it's on.
    pub fn load(&mut self, saved: &str, cx: &mut Context<Self>) {
        let saved: Value = serde_json::from_str(saved).unwrap_or_default();
        let agents = saved
            .get("agents")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        self.agents = (agents.iter())
            .filter_map(|agent| {
                let client = Client::from_id(agent.get("client")?.as_str()?)?;
                let token = agent.get("token")?.as_str()?.to_string();
                Some(Agent { client, token })
            })
            .collect();
        self.tokens_changed();
        self.on = saved.get("on").and_then(Value::as_bool).unwrap_or(false);
        if self.on {
            self.start(cx);
        }
        cx.notify();
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The switch in Settings › Agents. Off stops the server and ends every agent's session.
    pub fn set_on(&mut self, on: bool, cx: &mut Context<Self>) {
        if on == self.on {
            return;
        }
        self.on = on;
        match on {
            true => self.start(cx),
            false => self.stop(cx),
        }
        self.save();
        cx.notify();
    }

    /// Where the clients reach the server.
    pub fn url(&self) -> String {
        let port = self
            .server
            .as_ref()
            .map_or(self.port, |(server, _)| server.port());
        format!("http://127.0.0.1:{port}/mcp")
    }

    /// Why the server isn't running, though on.
    pub fn trouble(&self) -> Option<SharedString> {
        self.trouble.clone()
    }

    pub fn is_connected(&self, client: Client) -> bool {
        self.agents.iter().any(|agent| agent.client == client)
    }

    /// Whether the client's Connect is still writing its config.
    pub fn is_connecting(&self, client: Client) -> bool {
        self.connecting == Some(client)
    }

    /// Why the client's last Connect or Disconnect failed.
    pub fn failure(&self, client: Client) -> Option<SharedString> {
        self.failed
            .as_ref()
            .filter(|(each, _)| *each == client)
            .map(|(_, why)| why.clone())
    }

    /// Whether the agent has a session open now.
    pub fn is_live(&self, client: Client) -> bool {
        self.live.contains(client.id())
    }

    /// Makes the client the agent's token, and writes it into the client's config, with the
    /// server's address. A client connected before gets a new token, and the old one stops
    /// working.
    pub fn connect(&mut self, client: Client, cx: &mut Context<Self>) {
        if self.connecting.is_some() {
            return;
        }
        let token = random_token();
        let entry = Entry {
            name: server_name(),
            url: self.url(),
            token: token.clone(),
        };
        self.connecting = Some(client);
        self.failed = None;
        cx.notify();
        let clients = self.clients.clone();
        cx.spawn(async move |agents, cx| {
            let done = cx
                .background_spawn(async move { clients.connect(client, &entry) })
                .await;
            agents
                .update(cx, |agents, cx| agents.connected(client, token, done, cx))
                .ok();
        })
        .detach();
    }

    fn connected(
        &mut self,
        client: Client,
        token: String,
        done: Result<(), String>,
        cx: &mut Context<Self>,
    ) {
        self.connecting = None;
        match done {
            Ok(()) => {
                // check.sh reads this line.
                log!("browser: agent {} connected", client.id());
                self.agents.retain(|agent| agent.client != client);
                self.agents.push(Agent { client, token });
                self.tokens_changed();
                self.save();
                self.note(client.id(), "connected", "", "");
            }
            Err(why) => {
                log!("browser: agent {} couldn't connect: {why}", client.id());
                self.failed = Some((client, why.into()));
            }
        }
        cx.notify();
    }

    /// Takes the server out of the client's config, and its token from the agent, which ends its
    /// sessions. Its Space stays the user's to keep or delete.
    pub fn disconnect(&mut self, client: Client, cx: &mut Context<Self>) {
        if !self.is_connected(client) {
            return;
        }
        // check.sh reads this line.
        log!("browser: agent {} disconnected", client.id());
        self.agents.retain(|agent| agent.client != client);
        self.tokens_changed();
        self.save();
        self.reading.retain(|_, (agent, _, _)| agent != client.id());
        if self.live.remove(client.id()) {
            self.browser.disconnected(client.id(), cx);
        }
        self.note(client.id(), "disconnected", "", "");
        self.failed = None;
        let clients = self.clients.clone();
        let name = server_name();
        cx.spawn(async move |agents, cx| {
            let done = cx
                .background_spawn(async move { clients.disconnect(client, &name) })
                .await;
            if let Err(why) = done {
                log!("browser: agent {} config kept: {why}", client.id());
                agents
                    .update(cx, |agents, cx| {
                        agents.failed = Some((client, why.into()));
                        cx.notify();
                    })
                    .ok();
            }
        })
        .detach();
        cx.notify();
    }

    /// What an MCP client's config needs, for the user to paste: the server's address and the
    /// agent's token, as most clients write it.
    pub fn setup(&self, client: Client) -> Option<String> {
        let agent = self.agents.iter().find(|agent| agent.client == client)?;
        let server = json!({
            "type": "http",
            "url": self.url(),
            "headers": { "Authorization": format!("Bearer {}", agent.token) },
        });
        let config = json!({ "mcpServers": { server_name(): server } });
        serde_json::to_string_pretty(&config).ok()
    }

    /// Every agent's activity, newest first.
    pub fn recent(&self) -> impl Iterator<Item = &Activity> {
        self.log.iter().rev()
    }

    /// The agent's activity, newest first.
    pub fn activity(&self, agent: &str) -> impl Iterator<Item = &Activity> {
        self.log
            .iter()
            .rev()
            .filter(move |entry| entry.agent == agent)
    }

    /// The core checked a call of an agent's on `tab` not made here, as the driver's: an entry
    /// for the log. The calls made here log their own, as the core's report comes while they
    /// run.
    pub fn checked(
        &mut self,
        agent: &str,
        tab: u64,
        action: &str,
        refused: &str,
        cx: &mut Context<Self>,
    ) {
        let site = site(&self.browser.tab(tab, cx).url);
        self.note(agent, action, &site, refused);
        cx.notify();
    }

    /// The core ended the agent's session with `tab`: it let go of the tab, or the user took it
    /// back. A disconnect has its own entry.
    pub fn session_ended(&mut self, agent: &str, tab: u64, why: &str, cx: &mut Context<Self>) {
        if why != "disconnected" {
            let site = site(&self.browser.tab(tab, cx).url);
            self.note(agent, why, &site, "");
            cx.notify();
        }
    }

    /// The core's answer to the read `request`: the page's text, or None if it had none.
    pub fn text_read(&mut self, request: u64, text: Option<String>, cx: &mut Context<Self>) {
        let Some((_, tab, reply)) = self.reading.remove(&request) else {
            return;
        };
        let tab = self.browser.tab(tab, cx);
        _ = reply.send(match text {
            Some(text) => ToolResult::text(tools::page_text(&tab, &text)),
            None => ToolResult::error(
                "The tab has no page loaded: it loads once the user shows it. Try another tab.",
            ),
        });
    }

    /// What the server's threads hand over.
    pub fn handle(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Hello { agent, client } => {
                log!("browser: agent {agent} hello from {client}");
                if self.live.insert(agent.clone()) {
                    self.note(&agent, "connected", "", "");
                    cx.notify();
                }
            }
            Event::Bye { agent } => {
                if self.live.remove(&agent) {
                    self.browser.disconnected(&agent, cx);
                    self.note(&agent, "disconnected", "", "");
                    cx.notify();
                }
            }
            Event::Call(call) => self.call(call, cx),
        }
    }

    fn call(&mut self, call: Call, cx: &mut Context<Self>) {
        let Call {
            agent,
            tool,
            arguments,
            reply,
        } = call;
        // A token's agent is always one the user connected.
        let Some(client) = Client::from_id(&agent).filter(|client| self.is_connected(*client))
        else {
            _ = reply.send(ToolResult::error("This agent isn't connected."));
            return;
        };
        // check.sh reads this line.
        log!("browser: agent {agent} calls {tool}");
        self.live.insert(agent.clone());
        match tool.as_str() {
            "list_tabs" => {
                let space = self.space(client, cx);
                let tabs = self.browser.space_tabs(space, cx);
                self.note(&agent, "list_tabs", "", "");
                cx.notify();
                _ = reply.send(ToolResult::text(tools::tab_list(
                    &tabs,
                    self.current.get(&agent).copied(),
                )));
            }
            "read_page" => {
                let tab = match tools::tab_argument(&arguments) {
                    Ok(Some(tab)) => tab,
                    Ok(None) => match self.default_tab(client, cx) {
                        Some(tab) => tab,
                        None => {
                            _ = reply.send(ToolResult::text(tools::tab_list(&[], None)));
                            return;
                        }
                    },
                    Err(why) => {
                        _ = reply.send(ToolResult::error(why));
                        return;
                    }
                };
                if !self.act(&agent, tab, "read_page", &reply, cx) {
                    return;
                }
                let request = self.next_request;
                self.next_request += 1;
                self.reading.insert(request, (agent, tab, reply));
                self.browser.read_text(tab, request);
            }
            _ => _ = reply.send(ToolResult::error(format!("There's no tool {tool}."))),
        }
    }

    /// Asks the core whether the agent may do `action` to `tab`, and logs the answer. A refusal
    /// answers the call with why.
    fn act(
        &mut self,
        agent: &str,
        tab: u64,
        action: &str,
        reply: &mpsc::Sender<ToolResult>,
        cx: &mut Context<Self>,
    ) -> bool {
        let checked = self.browser.may_act(agent, tab, action, cx);
        let site = site(&self.browser.tab(tab, cx).url);
        let refused = checked.as_ref().err().map_or("", String::as_str);
        self.note(agent, action, &site, refused);
        cx.notify();
        match checked {
            Ok(()) => {
                self.current.insert(agent.into(), tab);
                true
            }
            Err(why) => {
                _ = reply.send(ToolResult::error(why));
                false
            }
        }
    }

    /// The agent's Space, made the first time it calls, so it shows in the sidebar for the user
    /// to open pages in.
    fn space(&mut self, client: Client, cx: &mut Context<Self>) -> u64 {
        match self.browser.agent_space(client.id()) {
            0 => self
                .browser
                .make_agent_space(client.id(), client.name(), cx),
            space => space,
        }
    }

    /// The tab the agent used last, if it's still in its Space, else its first.
    fn default_tab(&mut self, client: Client, cx: &mut Context<Self>) -> Option<u64> {
        let space = self.space(client, cx);
        let tabs = self.browser.space_tabs(space, cx);
        let last = self.current.get(client.id()).copied();
        (last.filter(|last| tabs.iter().any(|tab| tab.id == *last)))
            .or_else(|| tabs.first().map(|tab| tab.id))
    }

    fn note(&mut self, agent: &str, action: &str, site: &str, refused: &str) {
        if self.log.len() >= LOG_LENGTH {
            self.log.remove(0);
        }
        self.log.push(Activity {
            at: SystemTime::now(),
            agent: SharedString::from(agent.to_string()),
            action: SharedString::from(action.to_string()),
            site: SharedString::from(site.to_string()),
            refused: SharedString::from(refused.to_string()),
        });
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        if self.server.is_some() {
            return;
        }
        let (events, mut handed) = futures::channel::mpsc::unbounded();
        match server::Server::start(self.port, self.tokens.clone(), events) {
            Ok(server) => {
                // check.sh reads this line.
                log!("browser: agents server on 127.0.0.1:{}", server.port());
                // Wakes only as the server hands something over.
                let task = cx.spawn(async move |agents, cx| {
                    while let Some(event) = handed.next().await {
                        if agents
                            .update(cx, |agents, cx| agents.handle(event, cx))
                            .is_err()
                        {
                            return;
                        }
                    }
                });
                self.server = Some((server, task));
                self.trouble = None;
            }
            Err(error) => {
                log!("browser: agents server couldn't start: {error}");
                self.trouble = Some(format!("The server couldn't start: {error}.").into());
            }
        }
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        // Dropping the server's handover answers each call waiting on it.
        self.server = None;
        self.trouble = None;
        self.reading.clear();
        for agent in std::mem::take(&mut self.live) {
            self.browser.disconnected(&agent, cx);
            self.note(&agent, "disconnected", "", "");
        }
    }

    fn tokens_changed(&self) {
        let tokens = self
            .agents
            .iter()
            .map(|agent| (agent.token.clone(), agent.client.id().into()));
        *self.tokens.lock().unwrap() = tokens.collect();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let agents: Vec<Value> = (self.agents.iter())
            .map(|agent| json!({ "client": agent.client.id(), "token": agent.token }))
            .collect();
        let saved = json!({ "on": self.on, "agents": agents });
        if let Err(error) = clients::write_private(path, saved.to_string().as_bytes()) {
            log!("couldn't save the agents' settings: {error}");
        }
    }
}

/// A new token: 32 random bytes, in hex.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the system has no random numbers");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A URL's host, without "www.", for the log.
pub fn site(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else {
        return String::new();
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    let host = match host.split_once(']') {
        Some((address, _)) => &host[..address.len() + 1],
        None => host.split(':').next().unwrap_or_default(),
    };
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

/// How long ago `at` was, as the log says it.
pub fn ago(at: SystemTime, now: SystemTime) -> String {
    let seconds = now.duration_since(at).map_or(0, |since| since.as_secs());
    match seconds {
        0..60 => "Just now".into(),
        60..3600 => format!("{} min ago", seconds / 60),
        3600..86400 => format!("{} h ago", seconds / 3600),
        _ => format!("{} d ago", seconds / 86400),
    }
}

#[cfg(test)]
mod tests;
