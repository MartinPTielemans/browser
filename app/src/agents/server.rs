//! The MCP server: MCP's Streamable HTTP transport, on a port of the loopback interface, answering
//! each POST with JSON and offering no event stream, which a server may. A thread waits in
//! `accept`, and one for each connection waits in `read`, so an idle server wakes nothing. The
//! threads answer what needs no browser themselves, and send the rest to the UI thread as events.
//!
//! Only this user's processes should reach it, which the loopback interface alone doesn't
//! ensure, so each request carries an agent's token, which also says which agent it is. A web
//! page can't send one, and the Host and Origin checks keep a page that rebinds a name to
//! 127.0.0.1 out even before that.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use futures::channel::mpsc::UnboundedSender;
use serde_json::{Value, json};

use super::{Call, Event, ToolResult, random_token, tools};

/// Agents' tokens, each to the id of the agent it's for, which the UI thread changes as the user
/// connects and disconnects agents.
pub type Tokens = Arc<Mutex<HashMap<String, String>>>;

/// The protocol versions the server speaks, newest first.
const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// A request's head and body may be no longer, which no MCP message comes near.
const MAX_HEAD: usize = 16 * 1024;
const MAX_BODY: usize = 4 * 1024 * 1024;
/// Connections at once: a client keeps one or two.
const MAX_CONNECTIONS: usize = 32;

/// The server, running until it's dropped.
pub struct Server {
    shared: Arc<Shared>,
}

struct Shared {
    port: u16,
    stopped: AtomicBool,
    tokens: Tokens,
    /// The sessions `initialize` began, each to its agent's id.
    sessions: Mutex<HashMap<String, String>>,
    /// The connections open, by a number of their own, to shut them down as the server stops.
    connections: Mutex<HashMap<u64, TcpStream>>,
    next_connection: AtomicU64,
    events: UnboundedSender<Event>,
}

impl Server {
    /// Listens on `port` of 127.0.0.1, or on any free one for 0.
    pub fn start(port: u16, tokens: Tokens, events: UnboundedSender<Event>) -> io::Result<Server> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        let port = listener.local_addr()?.port();
        let shared = Arc::new(Shared {
            port,
            stopped: AtomicBool::new(false),
            tokens,
            sessions: Mutex::default(),
            connections: Mutex::default(),
            next_connection: AtomicU64::new(0),
            events,
        });
        let accepting = shared.clone();
        thread::Builder::new()
            .name("agents-server".into())
            .spawn(move || accept(listener, accepting))?;
        Ok(Server { shared })
    }

    pub fn port(&self) -> u16 {
        self.shared.port
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::SeqCst);
        // `accept` returns for a connection, which then finds the server stopped.
        _ = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, self.shared.port)));
        for (_, stream) in self.shared.connections.lock().unwrap().drain() {
            _ = stream.shutdown(Shutdown::Both);
        }
    }
}

fn accept(listener: TcpListener, shared: Arc<Shared>) {
    for stream in listener.incoming() {
        if shared.stopped.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let number = shared.next_connection.fetch_add(1, Ordering::SeqCst);
        {
            let mut connections = shared.connections.lock().unwrap();
            if connections.len() >= MAX_CONNECTIONS {
                continue;
            }
            let Ok(kept) = stream.try_clone() else {
                continue;
            };
            connections.insert(number, kept);
        }
        let serving = shared.clone();
        let spawned = thread::Builder::new()
            .name("agents-connection".into())
            .spawn(move || {
                serve(stream, &serving);
                serving.connections.lock().unwrap().remove(&number);
            });
        if spawned.is_err() {
            shared.connections.lock().unwrap().remove(&number);
        }
    }
}

/// Answers the connection's requests in turn until it closes.
fn serve(stream: TcpStream, shared: &Shared) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    while let Ok(Some(request)) = read_request(&mut reader) {
        if shared.stopped.load(Ordering::SeqCst) {
            return;
        }
        let close = request
            .header("connection")
            .is_some_and(|c| c.eq_ignore_ascii_case("close"));
        let response = respond(shared, &request);
        if write_response(&mut writer, &response, close).is_err() || close {
            return;
        }
    }
}

/// An HTTP request, as much of it as MCP's transport uses.
pub struct Request {
    pub method: String,
    pub path: String,
    /// Names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, value)| value.as_str())
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Response {
    fn empty(status: u16) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn json(status: u16, body: &Value) -> Response {
        Response {
            status,
            headers: vec![("Content-Type", "application/json".into())],
            body: body.to_string().into_bytes(),
        }
    }
}

/// The next request, or None once the client has closed the connection.
pub fn read_request(reader: &mut impl BufRead) -> io::Result<Option<Request>> {
    let mut head = Vec::new();
    loop {
        let start = head.len();
        let room = MAX_HEAD.saturating_sub(start) as u64;
        if reader.by_ref().take(room).read_until(b'\n', &mut head)? == 0 {
            return match head.is_empty() {
                true => Ok(None),
                false => Err(io::ErrorKind::UnexpectedEof.into()),
            };
        }
        if !head.ends_with(b"\n") {
            return Err(io::ErrorKind::InvalidData.into());
        }
        // A blank line ends the head.
        if start > 0 && matches!(&head[start..], b"\r\n" | b"\n") {
            break;
        }
    }
    let head = String::from_utf8(head).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    let mut lines = head.lines();
    let mut first = lines.next().unwrap_or_default().split(' ');
    let (method, path) = (
        first.next().unwrap_or_default(),
        first.next().unwrap_or_default(),
    );
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    let length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .map_or(Ok(0), |(_, value)| value.parse::<usize>())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
    // MCP's clients send a length; a chunked body would read as the next request.
    if length > MAX_BODY || headers.iter().any(|(name, _)| name == "transfer-encoding") {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(Request {
        method: method.into(),
        path: path.into(),
        headers,
        body,
    }))
}

fn write_response(stream: &mut impl Write, response: &Response, close: bool) -> io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let mut head = format!("HTTP/1.1 {} {reason}\r\n", response.status);
    for (name, value) in &response.headers {
        head += &format!("{name}: {value}\r\n");
    }
    head += &format!("Content-Length: {}\r\n", response.body.len());
    if close {
        head += "Connection: close\r\n";
    }
    head += "\r\n";
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

/// The response to `request`, with what the UI thread answers for a tool.
fn respond(shared: &Shared, request: &Request) -> Response {
    // A page that rebinds a name of its own to 127.0.0.1 sends that name, and a page's request
    // has an Origin, which MCP's clients don't send.
    let host = request.header("host").unwrap_or_default();
    let port = shared.port;
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return Response::empty(403);
    }
    if request.header("origin").is_some() {
        return Response::empty(403);
    }
    if request.path.split('?').next() != Some("/mcp") {
        return Response::empty(404);
    }
    let token = request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "));
    let agent = token.and_then(|token| shared.tokens.lock().unwrap().get(token.trim()).cloned());
    let Some(agent) = agent else {
        let mut response = Response::empty(401);
        response.headers.push(("WWW-Authenticate", "Bearer".into()));
        return response;
    };
    let session = request.header("mcp-session-id");
    match request.method.as_str() {
        "POST" => post(shared, &agent, session, &request.body),
        "DELETE" => {
            if let Some(session) = session {
                end_session(shared, &agent, session);
            }
            Response::empty(200)
        }
        // There's no stream of the server's own messages: it sends none.
        _ => {
            let mut response = Response::empty(405);
            response.headers.push(("Allow", "POST, DELETE".into()));
            response
        }
    }
}

fn post(shared: &Shared, agent: &str, session: Option<&str>, body: &[u8]) -> Response {
    let Ok(message) = serde_json::from_slice::<Value>(body) else {
        return Response::json(400, &error(Value::Null, -32700, "The body isn't JSON."));
    };
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        // A response to a request of the server's, which sends none.
        return Response::empty(202);
    };
    let Some(id) = message.get("id").cloned() else {
        // A notification, such as notifications/initialized.
        return Response::empty(202);
    };
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    if method == "initialize" {
        return initialize(shared, agent, id, &params);
    }
    // A session the server doesn't know, as one from before the browser quit, starts again with
    // initialize, which a 404 asks the client for.
    if let Some(session) = session {
        let known = shared
            .sessions
            .lock()
            .unwrap()
            .get(session)
            .is_some_and(|each| each == agent);
        if !known {
            return Response::json(404, &error(id, -32001, "The session has ended."));
        }
    }
    let result = match method {
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools::list() }),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            call(shared, agent, name, arguments).to_json()
        }
        _ => return Response::json(200, &error(id, -32601, "There's no such method.")),
    };
    Response::json(
        200,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

fn initialize(shared: &Shared, agent: &str, id: Value, params: &Value) -> Response {
    let asked = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version = VERSIONS
        .iter()
        .find(|each| **each == asked)
        .unwrap_or(&VERSIONS[0]);
    let client = params
        .pointer("/clientInfo/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let session = random_token();
    shared
        .sessions
        .lock()
        .unwrap()
        .insert(session.clone(), agent.into());
    _ = shared.events.unbounded_send(Event::Hello {
        agent: agent.into(),
        client: client.into(),
    });
    let result = json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": env!("APP_NAME"), "version": env!("CARGO_PKG_VERSION") },
        "instructions": tools::INSTRUCTIONS,
    });
    let mut response = Response::json(
        200,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    );
    response.headers.push(("Mcp-Session-Id", session));
    response
}

/// Runs a tool on the UI thread, and waits for its answer.
fn call(shared: &Shared, agent: &str, tool: &str, arguments: Value) -> ToolResult {
    let (reply, answer) = mpsc::channel();
    let call = Call {
        agent: agent.into(),
        tool: tool.into(),
        arguments,
        reply,
    };
    if shared.events.unbounded_send(Event::Call(call)).is_err() {
        return ToolResult::error("The browser has turned agents off.");
    }
    // The UI drops the call unanswered when the user turns agents off or disconnects the agent.
    answer
        .recv()
        .unwrap_or_else(|_| ToolResult::error("The browser has turned agents off."))
}

fn end_session(shared: &Shared, agent: &str, session: &str) {
    let mut sessions = shared.sessions.lock().unwrap();
    if sessions.get(session).is_some_and(|each| each == agent) {
        sessions.remove(session);
        if !sessions.values().any(|each| each == agent) {
            _ = shared.events.unbounded_send(Event::Bye {
                agent: agent.into(),
            });
        }
    }
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests;
