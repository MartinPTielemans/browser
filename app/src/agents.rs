//! Agents: Claude Code, Codex or any other MCP client works in the browser, each in a Space of its
//! own, through the browser's MCP server (decision 8). Settings › Agents turns the server on, which
//! it isn't at first, connects the clients and lists what each agent did. The server is a Unix
//! socket in the profile directory, which a client reaches through the bridge it launches. Its
//! threads sleep in the kernel between messages, so an idle browser wakes for none of it, and nothing here
//! runs on tab switch, new tab or launch: the saved settings are read after the first window shows.
//!
//! A client proves which agent it is with the token Connect wrote into its config, which its bridge
//! hands the server first. Each tool call
//! reaches the UI thread as an event, and the tools answer through the core, which checks each
//! call on a tab with bc_agent_may_act. That check's answer is the call's activity-log entry.
//! A payment, an order, a message sent or a bank's site asks the user first, in a card in each
//! window: once, always or never on that site, for that agent. Settings › Agents and the agent's
//! Space's menu list what the user answered for good, to revoke.

pub mod bridge;
pub mod clients;
pub mod page;
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

/// What the core does in an agent's tab (bc_agent_page).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageAction {
    Snapshot,
    Click,
    Type,
    TypeSubmit,
    Key,
    ScrollDown,
    ScrollUp,
    Back,
    Forward,
    Screenshot,
}

/// What an agent asks the user's leave for, as the card in each window asks it.
#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub id: u64,
    /// The agent's name, as its Space has it.
    pub agent: SharedString,
    /// What it would do, following "wants to", such as "click “Place order” on shop.example".
    pub what: SharedString,
    /// Why that asks, such as "It looks like a payment or an order."
    pub why: SharedString,
}

/// The user's answer to an agent's question.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Answer {
    /// This once.
    Once,
    /// Now and whenever the agent asks on the site again.
    Always,
    /// Not now, nor when it asks on the site again.
    Never,
    /// Not now: the ×.
    Dismiss,
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
    /// Opens `url`, http(s) only, in a new tab at the top of the agent's Space, not shown
    /// (bc_agent_open_tab): the tab, or 0 if it couldn't.
    fn open_tab(&self, agent: &str, url: &str, cx: &mut App) -> u64;
    /// Loads `url`, http(s) only, in the agent's `tab` (bc_agent_navigate).
    fn navigate(&self, agent: &str, tab: u64, url: &str, cx: &mut App) -> bool;
    /// Asks the core to do `action` in the agent's `tab` (bc_agent_page), to the element whose
    /// node is `element`, with `text`. Its answer comes to `Agents::page_done` with `request`.
    fn page(
        &self,
        agent: &str,
        tab: u64,
        request: u64,
        action: PageAction,
        element: i64,
        text: &str,
    );
    /// The URL the address bar goes to for `text`: the page it names, else a search for it.
    fn url_for(&self, text: &str) -> Option<String>;
    /// Shows the question for the user to answer, in every window, or takes it away (None).
    fn ask(&self, question: Option<Question>, cx: &mut App);
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
    /// A picture, as a JPEG in base64.
    pub image: Option<String>,
}

impl ToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        ToolResult {
            text: text.into(),
            error: false,
            image: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        ToolResult {
            text: text.into(),
            error: true,
            image: None,
        }
    }

    pub fn image(jpeg: String) -> Self {
        ToolResult {
            text: String::new(),
            error: false,
            image: Some(jpeg),
        }
    }

    pub fn to_json(&self) -> Value {
        match &self.image {
            Some(jpeg) => json!({
                "content": [{ "type": "image", "data": jpeg, "mimeType": "image/jpeg" }],
                "isError": self.error,
            }),
            None => {
                json!({ "content": [{ "type": "text", "text": self.text }], "isError": self.error })
            }
        }
    }
}

/// An agent's action, from its call until the core answers, and through the question it asks the
/// user first if it does.
struct Act {
    agent: String,
    /// What the activity log names it, as `tools::describe` knows it.
    action: &'static str,
    /// The tab, or 0 for open_tab, which makes it.
    tab: u64,
    /// Where open_tab and navigate go.
    url: String,
    page: PageAction,
    element: i64,
    text: String,
    /// What the answer says it did, before the outline.
    note: String,
    /// What it would do, as a question has it.
    what: String,
    reply: mpsc::Sender<ToolResult>,
}

/// An action that waits on the core's answer.
struct Acting {
    agent: String,
    tab: u64,
    note: String,
    reply: mpsc::Sender<ToolResult>,
}

/// An action that waits on the user.
struct Asking {
    question: Question,
    site: String,
    act: Act,
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
    /// The server's socket, and the bridge a client runs to reach it.
    socket: PathBuf,
    bridge: PathBuf,
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
    /// The actions waiting on the core, by request.
    acting: HashMap<u64, Acting>,
    /// Each tab's refs, from its outlines.
    refs: HashMap<u64, page::Refs>,
    /// The actions waiting on the user, the first asked now.
    asking: Vec<Asking>,
    /// The sites where each agent may (true) or may never (false) do what asks first.
    grants: HashMap<String, HashMap<String, bool>>,
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
        socket: PathBuf,
        bridge: PathBuf,
    ) -> Self {
        Agents {
            browser,
            clients,
            path,
            socket,
            bridge,
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
            acting: HashMap::new(),
            refs: HashMap::new(),
            asking: Vec::new(),
            grants: HashMap::new(),
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
        let grants = saved.get("grants").and_then(Value::as_object).into_iter().flatten();
        self.grants = (grants.filter_map(|(agent, sites)| {
            let sites = (sites.as_object()?.iter())
                .filter_map(|(site, grant)| match grant.as_str()? {
                    "always" => Some((site.clone(), true)),
                    "never" => Some((site.clone(), false)),
                    _ => None,
                })
                .collect();
            Some((agent.clone(), sites))
        }))
        .collect();
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

    /// The server as a client's config names it, with the agent's token.
    fn entry(&self, token: &str) -> Entry {
        Entry {
            name: server_name(),
            bridge: self.bridge.to_string_lossy().into_owned(),
            socket: self.socket.to_string_lossy().into_owned(),
            token: token.into(),
        }
    }

    /// What a client runs to reach the server: the bridge, with the socket.
    pub fn command(&self) -> String {
        format!(
            "\"{}\" \"{}\"",
            self.bridge.display(),
            self.socket.display()
        )
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
    /// bridge that reaches the server. A client connected before gets a new token, and the old one stops
    /// working.
    pub fn connect(&mut self, client: Client, cx: &mut Context<Self>) {
        if self.connecting.is_some() {
            return;
        }
        let token = random_token();
        let entry = self.entry(&token);
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
        self.forget_waiting(client.id(), cx);
        if let Some((server, _)) = &self.server {
            server.disconnect(client.id());
        }
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

    /// What an MCP client's config needs, for the user to paste: the bridge, its socket and the
    /// agent's token, as most clients write a stdio server.
    pub fn setup(&self, client: Client) -> Option<String> {
        let agent = self.agents.iter().find(|agent| agent.client == client)?;
        let config = json!({ "mcpServers": { server_name(): self.entry(&agent.token).json() } });
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
                    self.forget_waiting(&agent, cx);
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
            "open_tab" => {
                let Some(url) = self.url_argument(&arguments, &reply) else {
                    return;
                };
                let host = site(&url);
                let act = Act {
                    agent: agent.clone(),
                    action: "open_tab",
                    tab: 0,
                    what: format!("open {host}"),
                    url,
                    page: PageAction::Snapshot,
                    element: 0,
                    text: String::new(),
                    note: String::new(),
                    reply,
                };
                if !is_web(&act.url) {
                    return self.refuse(act, &host, WEB_ONLY, cx);
                }
                let risk = page::opening_risk(&host);
                self.check(act, host, risk, cx);
            }
            "navigate" | "snapshot" | "click" | "type" | "press_key" | "scroll" | "screenshot" => {
                let tab = match tools::tab_argument(&arguments) {
                    Ok(Some(tab)) => tab,
                    Ok(None) => match self.default_tab(client, cx) {
                        Some(tab) => tab,
                        None => {
                            _ = reply.send(ToolResult::error(
                                "You have no tab open yet. Open one with open_tab.",
                            ));
                            return;
                        }
                    },
                    Err(why) => {
                        _ = reply.send(ToolResult::error(why));
                        return;
                    }
                };
                self.page_tool(&agent, &tool, tab, &arguments, reply, cx);
            }
            _ => _ = reply.send(ToolResult::error(format!("There's no tool {tool}."))),
        }
    }

    /// A tool that acts in a tab or looks at it: what it asks of the core, and whether it asks the
    /// user first.
    fn page_tool(
        &mut self,
        agent: &str,
        tool: &str,
        tab: u64,
        arguments: &Value,
        reply: mpsc::Sender<ToolResult>,
        cx: &mut Context<Self>,
    ) {
        let host = site(&self.browser.tab(tab, cx).url);
        let string = |name: &str| arguments.get(name).and_then(Value::as_str).map(str::to_string);
        let element = match string("ref") {
            Some(name) => match self.refs.get(&tab).and_then(|refs| refs.get(&name)) {
                Some(element) => Some(element.clone()),
                None => {
                    _ = reply.send(ToolResult::error(format!(
                        "There's no element {name} in tab {tab}. Use a ref from its latest outline."
                    )));
                    return;
                }
            },
            None => None,
        };
        let label = |element: &page::Element| match element.name.is_empty() {
            true => element.role.clone(),
            false => format!("“{}”", page::clip(&element.name, 60)),
        };
        let mut act = Act {
            agent: agent.into(),
            action: "snapshot",
            tab,
            url: String::new(),
            page: PageAction::Snapshot,
            element: element.as_ref().map_or(0, |element| element.node),
            text: String::new(),
            note: String::new(),
            what: String::new(),
            reply,
        };
        let mut risk = None;
        match tool {
            "navigate" => {
                let url = string("url").unwrap_or_default();
                match url.trim() {
                    "back" => {
                        (act.action, act.page, act.note) =
                            ("back", PageAction::Back, "Went back.".into())
                    }
                    "forward" => {
                        (act.action, act.page, act.note) =
                            ("forward", PageAction::Forward, "Went forward.".into())
                    }
                    _ => {
                        let Some(url) = self.url_argument(arguments, &act.reply) else {
                            return;
                        };
                        let to = site(&url);
                        act.action = "navigate";
                        act.what = format!("open {to}");
                        act.url = url;
                        if !is_web(&act.url) {
                            return self.refuse(act, &to, WEB_ONLY, cx);
                        }
                        // What asks is the site it goes to.
                        return self.check(act, to.clone(), page::opening_risk(&to), cx);
                    }
                }
            }
            "snapshot" => {}
            "screenshot" => (act.action, act.page) = ("screenshot", PageAction::Screenshot),
            "click" => {
                let Some(element) = &element else {
                    _ = act.reply.send(ToolResult::error("click needs a ref."));
                    return;
                };
                act.action = "click";
                act.page = PageAction::Click;
                act.note = format!("Clicked {} {}.", element.role, label(element));
                act.what = format!("click {} on {host}", label(element));
                risk = page::risk(&host, &element.name);
            }
            "type" => {
                let (Some(element), Some(text)) = (&element, string("text")) else {
                    _ = act.reply.send(ToolResult::error("type needs a ref and text."));
                    return;
                };
                act.action = "type";
                if element.password {
                    return self.refuse(act, &host, PASSWORDS, cx);
                }
                let submit = arguments.get("submit").and_then(Value::as_bool).unwrap_or(false);
                act.page = if submit { PageAction::TypeSubmit } else { PageAction::Type };
                act.note = format!(
                    "Typed into {}{}.",
                    label(element),
                    if submit { " and pressed Enter" } else { "" }
                );
                act.what = format!("send “{}” on {host}", page::clip(&text, 60));
                act.text = text;
                risk = submit.then(|| page::risk(&host, "send")).flatten();
            }
            "press_key" => {
                let key = string("key").unwrap_or_default();
                if !page::KEYS.contains(&key.as_str()) {
                    let keys = page::KEYS.join(", ");
                    _ = act.reply.send(ToolResult::error(format!("key is one of {keys}.")));
                    return;
                }
                act.action = "press_key";
                act.page = PageAction::Key;
                act.note = format!("Pressed {key}.");
                act.what = format!("press Enter to send on {host}");
                risk = (key == "Enter").then(|| page::risk(&host, "send")).flatten();
                act.text = key;
            }
            _ => {
                let up = string("direction").as_deref() == Some("up");
                act.action = "scroll";
                act.page = if up { PageAction::ScrollUp } else { PageAction::ScrollDown };
                act.note = match &element {
                    Some(element) => format!("Scrolled to {}.", label(element)),
                    None => format!("Scrolled {}.", if up { "up" } else { "down" }),
                };
            }
        }
        self.check(act, host, risk, cx);
    }

    /// The `url` argument as the address bar takes it, or None once the call has its error.
    fn url_argument(&self, arguments: &Value, reply: &mpsc::Sender<ToolResult>) -> Option<String> {
        let typed = arguments.get("url").and_then(Value::as_str).unwrap_or_default();
        let url = (!typed.trim().is_empty()).then(|| self.browser.url_for(typed)).flatten();
        if url.is_none() {
            _ = reply.send(ToolResult::error("url is a URL, or words to search for."));
        }
        url
    }

    /// Runs the action, unless what it does on `site` asks the user first (`why`): then it waits
    /// for the user's answer, or the one they gave for good.
    fn check(&mut self, act: Act, site: String, why: Option<&'static str>, cx: &mut Context<Self>) {
        let Some(why) = why else {
            return self.run(act, cx);
        };
        match self.grants.get(&act.agent).and_then(|sites| sites.get(&site)) {
            Some(true) => return self.run(act, cx),
            Some(false) => return self.refuse(act, &site, SAID_NEVER, cx),
            None => {}
        }
        // A tab not the agent's is the core's to refuse, without a question.
        let space = self.browser.agent_space(&act.agent);
        if act.tab != 0 && !self.browser.space_tabs(space, cx).iter().any(|tab| tab.id == act.tab) {
            return self.run(act, cx);
        }
        let name = Client::from_id(&act.agent).map_or(act.agent.as_str(), |client| client.name());
        let question = Question {
            id: self.next_request,
            agent: name.to_string().into(),
            what: act.what.clone().into(),
            why: why.into(),
        };
        self.next_request += 1;
        // check.sh reads this line.
        log!("browser: agent {} asks {}: {}", act.agent, question.id, act.what);
        self.asking.push(Asking {
            question,
            site,
            act,
        });
        self.asked(cx);
    }

    /// The user's answer to the question `id`.
    pub fn answer(&mut self, id: u64, answer: Answer, cx: &mut Context<Self>) {
        let Some(at) = self.asking.iter().position(|asking| asking.question.id == id) else {
            return;
        };
        let Asking { site, act, .. } = self.asking.remove(at);
        // check.sh reads this line.
        log!("browser: agent {} answered {id} {answer:?}", act.agent);
        if let Answer::Always | Answer::Never = answer {
            let sites = self.grants.entry(act.agent.clone()).or_default();
            sites.insert(site.clone(), answer == Answer::Always);
            self.save();
        }
        match answer {
            Answer::Once | Answer::Always => self.run(act, cx),
            Answer::Never | Answer::Dismiss => self.refuse(act, &site, SAID_NO, cx),
        }
        self.asked(cx);
    }

    /// The sites where the user answered the agent for good, by name: Always Allow (true) or
    /// Never (false).
    pub fn grants(&self, agent: &str) -> Vec<(SharedString, bool)> {
        let mut grants: Vec<(SharedString, bool)> = (self.grants.get(agent).into_iter())
            .flatten()
            .map(|(site, &may)| (site.clone().into(), may))
            .collect();
        grants.sort();
        grants
    }

    /// Forgets the user's answer for good to the agent on `site`, or on every site with None, so
    /// what asks first there asks again.
    pub fn revoke(&mut self, agent: &str, site: Option<&str>, cx: &mut Context<Self>) {
        let Some(sites) = self.grants.get_mut(agent) else {
            return;
        };
        let revoked = match site {
            Some(site) => sites.remove(site).is_some(),
            None => !std::mem::take(sites).is_empty(),
        };
        if sites.is_empty() {
            self.grants.remove(agent);
        }
        if !revoked {
            return;
        }
        // check.sh reads this line.
        log!("browser: agent {agent} revoked {}", site.unwrap_or("all"));
        self.save();
        match site {
            Some(site) => self.note(agent, "revoked", site, ""),
            None => self.note(agent, "revoked all", "", ""),
        }
        cx.notify();
    }

    /// The question asked now, if any.
    pub fn question(&self) -> Option<&Question> {
        self.asking.first().map(|asking| &asking.question)
    }

    fn asked(&self, cx: &mut Context<Self>) {
        self.browser.ask(self.question().cloned(), cx);
        cx.notify();
    }

    /// The agent went: what waits for it goes unanswered, its questions with it.
    fn forget_waiting(&mut self, agent: &str, cx: &mut Context<Self>) {
        let asked = self.question().map(|question| question.id);
        self.asking.retain(|asking| asking.act.agent != agent);
        self.acting.retain(|_, acting| acting.agent != agent);
        if self.question().map(|question| question.id) != asked {
            self.asked(cx);
        }
    }

    /// Answers the action with why it's refused, which the log has.
    fn refuse(&mut self, act: Act, site: &str, why: &str, cx: &mut Context<Self>) {
        self.note(&act.agent, act.action, site, why);
        cx.notify();
        let answer = match why {
            SAID_NO => format!("The user said no: you can't {}. Don't try again.", act.what),
            SAID_NEVER => {
                format!("The user said never on {site}: you can't {}. Don't try again.", act.what)
            }
            _ => why.to_string(),
        };
        _ = act.reply.send(ToolResult::error(answer));
    }

    /// Does the action: the core checks it and logs it, and answers with the page.
    fn run(&mut self, act: Act, cx: &mut Context<Self>) {
        let Act {
            agent,
            action,
            mut tab,
            url,
            page,
            element,
            text,
            mut note,
            reply,
            ..
        } = act;
        if action == "open_tab" {
            let site = site(&url);
            tab = self.browser.open_tab(&agent, &url, cx);
            if tab == 0 {
                self.note(&agent, action, &site, "The tab couldn't open.");
                cx.notify();
                _ = reply.send(ToolResult::error("The tab couldn't open."));
                return;
            }
            self.note(&agent, action, &site, "");
            cx.notify();
            self.current.insert(agent.clone(), tab);
            note = format!("Opened tab {tab}.");
        } else {
            if !self.act(&agent, tab, action, &reply, cx) {
                return;
            }
            if action == "navigate" && !self.browser.navigate(&agent, tab, &url, cx) {
                _ = reply.send(ToolResult::error("The tab couldn't go there."));
                return;
            }
        }
        let request = self.next_request;
        self.next_request += 1;
        self.acting.insert(request, Acting { agent: agent.clone(), tab, note, reply });
        self.browser.page(&agent, tab, request, page, element, &text);
    }

    /// The core's answer to the action `request`: with `error`, why it couldn't; else a
    /// screenshot's JPEG in base64, or the nodes of the page's tree, which the answer outlines.
    pub fn page_done(&mut self, request: u64, text: String, error: bool, cx: &mut Context<Self>) {
        let Some(Acting { tab, note, reply, .. }) = self.acting.remove(&request) else {
            return;
        };
        if error {
            _ = reply.send(ToolResult::error(text));
            return;
        }
        if !text.starts_with('[') {
            _ = reply.send(ToolResult::image(text));
            return;
        }
        let nodes: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
        let info = self.browser.tab(tab, cx);
        let outline = page::outline(&info, &nodes, self.refs.entry(tab).or_default());
        _ = reply.send(ToolResult::text(match note.is_empty() {
            true => outline,
            false => format!("{note}\n\n{outline}"),
        }));
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
        match server::Server::start(&self.socket, self.tokens.clone(), events) {
            Ok(server) => {
                // check.sh reads this line.
                log!("browser: agents server on {}", server.path().display());
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
        self.acting.clear();
        if !std::mem::take(&mut self.asking).is_empty() {
            self.browser.ask(None, cx);
        }
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
        let grants: serde_json::Map<String, Value> = (self.grants.iter())
            .map(|(agent, sites)| {
                let sites = (sites.iter())
                    .map(|(site, &may)| {
                        (site.clone(), json!(if may { "always" } else { "never" }))
                    });
                (agent.clone(), Value::Object(sites.collect()))
            })
            .collect();
        let saved = json!({ "on": self.on, "agents": agents, "grants": grants });
        if let Err(error) = clients::write_private(path, saved.to_string().as_bytes()) {
            log!("couldn't save the agents' settings: {error}");
        }
    }
}

/// What the log says of an action the user refused, now or for good on its site.
const SAID_NO: &str = "You said no.";
const SAID_NEVER: &str = "You said never on this site.";
/// Why an agent can't open a page that isn't on the web.
const WEB_ONLY: &str = "Agents may open only http and https pages.";
/// Why an agent can't type into a password field.
const PASSWORDS: &str =
    "Password fields are off limits to agents: signing in is the user's to do. Ask them to.";

fn is_web(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
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

// The tests run the server, which is a Unix socket until Windows has its named pipe.
#[cfg(all(test, unix))]
mod tests;
