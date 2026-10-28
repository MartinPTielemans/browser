//! The server as an MCP client and a web page reach it, over the loopback interface.

use std::io::{Cursor, Read, Write};
use std::net::TcpStream;
use std::thread;

use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;

use super::*;

const TOKEN: &str = "s3cret";

/// A server on any free port, with Claude Code's token.
fn server() -> (Server, UnboundedReceiver<Event>) {
    let tokens: Tokens = Arc::default();
    tokens
        .lock()
        .unwrap()
        .insert(TOKEN.into(), "claude-code".into());
    let (events, handed) = futures::channel::mpsc::unbounded();
    (Server::start(0, tokens, events).unwrap(), handed)
}

/// What the server answers: its status, headers and body.
struct Answer {
    status: u16,
    head: String,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }

    fn header(&self, name: &str) -> Option<String> {
        let prefix = format!("{}: ", name.to_ascii_lowercase());
        (self.head.lines())
            .find(|line| line.to_ascii_lowercase().starts_with(&prefix))
            .map(|line| line[prefix.len()..].to_string())
    }
}

/// Sends a request with `headers` and closes the connection after the answer.
fn send(port: u16, method: &str, headers: &[(&str, &str)], body: &str) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut request = format!("{method} /mcp HTTP/1.1\r\nConnection: close\r\n");
    for (name, value) in headers {
        request += &format!("{name}: {value}\r\n");
    }
    request += &format!("Content-Length: {}\r\n\r\n{body}", body.len());
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    let (head, body) = answer.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    Answer {
        status,
        head: head.into(),
        body: body.into(),
    }
}

/// A client's POST, with its token and the Host it sends.
fn post(port: u16, session: Option<&str>, body: Value) -> Answer {
    let host = format!("127.0.0.1:{port}");
    let mut headers = vec![
        ("Host", host.as_str()),
        ("Authorization", "Bearer s3cret"),
        ("Content-Type", "application/json"),
        ("Accept", "application/json, text/event-stream"),
    ];
    if let Some(session) = session {
        headers.push(("Mcp-Session-Id", session));
    }
    send(port, "POST", &headers, &body.to_string())
}

fn initialize(port: u16) -> Answer {
    post(
        port,
        None,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "claude-code", "version": "2.0.0" },
            },
        }),
    )
}

/// initialize begins a session, in the version the client asked for, says what the server is and
/// how to use it, and tells the UI which agent said hello.
#[test]
fn initialize_begins_a_session() {
    let (server, mut handed) = server();
    let answer = initialize(server.port());
    assert_eq!(answer.status, 200);
    let result = &answer.json()["result"];
    assert_eq!(result["protocolVersion"], "2025-03-26");
    assert_eq!(result["serverInfo"]["name"], env!("APP_NAME"));
    assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
    assert!(
        result["instructions"]
            .as_str()
            .unwrap()
            .contains("Space of your own")
    );
    assert_eq!(answer.json()["id"], 1);
    assert_eq!(
        answer.header("content-type").as_deref(),
        Some("application/json")
    );
    assert!(
        answer
            .header("mcp-session-id")
            .is_some_and(|session| session.len() == 64)
    );
    match futures::executor::block_on(handed.next()) {
        Some(Event::Hello { agent, client }) => assert_eq!(
            (agent, client),
            ("claude-code".into(), "claude-code".into())
        ),
        _ => panic!("the UI didn't hear the agent's hello"),
    }
}

/// A version the server doesn't speak gets its newest.
#[test]
fn an_unknown_version_gets_the_newest() {
    let (server, _handed) = server();
    let answer = post(
        server.port(),
        None,
        json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "1999-01-01" },
        }),
    );
    assert_eq!(answer.json()["result"]["protocolVersion"], VERSIONS[0]);
}

/// Without the agent's token, from a web page, or under another name, nothing gets in.
#[test]
fn only_a_client_with_a_token_gets_in() {
    let (server, _handed) = server();
    let port = server.port();
    let host = format!("127.0.0.1:{port}");
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string();
    let none = send(port, "POST", &[("Host", &host)], &body);
    assert_eq!(none.status, 401);
    assert_eq!(none.header("www-authenticate").as_deref(), Some("Bearer"));
    let wrong = send(
        port,
        "POST",
        &[("Host", &host), ("Authorization", "Bearer guess")],
        &body,
    );
    assert_eq!(wrong.status, 401);
    let token = ("Authorization", "Bearer s3cret");
    let page = send(
        port,
        "POST",
        &[("Host", &host), token, ("Origin", "https://evil.example")],
        &body,
    );
    assert_eq!(page.status, 403);
    let rebound = send(
        port,
        "POST",
        &[("Host", &format!("evil.example:{port}")), token],
        &body,
    );
    assert_eq!(rebound.status, 403);
    let localhost = format!("localhost:{port}");
    assert_eq!(
        send(port, "POST", &[("Host", &localhost), token], &body).status,
        200
    );
}

/// The tools list says what each takes; a notification and a client's response are taken with
/// 202; there's no event stream to GET; an unknown method is an error.
#[test]
fn the_server_answers_what_needs_no_browser() {
    let (server, _handed) = server();
    let port = server.port();
    let list = post(
        port,
        None,
        json!({ "jsonrpc": "2.0", "id": "a", "method": "tools/list" }),
    );
    let names: Vec<_> = (list.json()["result"]["tools"].as_array().unwrap().iter())
        .map(|tool| tool["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["list_tabs", "read_page"]);
    assert_eq!(list.json()["id"], "a");
    let note = post(
        port,
        None,
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    );
    assert_eq!((note.status, note.body.as_str()), (202, ""));
    let ping = post(
        port,
        None,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
    );
    assert_eq!(ping.json()["result"], json!({}));
    let unknown = post(
        port,
        None,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
    );
    assert_eq!(unknown.json()["error"]["code"], -32601);
    let host = format!("127.0.0.1:{port}");
    let get = send(
        port,
        "GET",
        &[("Host", &host), ("Authorization", "Bearer s3cret")],
        "",
    );
    assert_eq!(
        (get.status, get.header("allow").as_deref()),
        (405, Some("POST, DELETE"))
    );
    let bad = send(
        port,
        "POST",
        &[("Host", &host), ("Authorization", "Bearer s3cret")],
        "{",
    );
    assert_eq!(bad.status, 400);
}

/// A tool call waits for the UI thread's answer, and gives it as MCP's result.
#[test]
fn a_tool_call_is_the_ui_threads_to_answer() {
    let (server, mut handed) = server();
    let port = server.port();
    let session = initialize(port).header("mcp-session-id").unwrap();
    futures::executor::block_on(handed.next());
    let calling = thread::spawn(move || {
        let call = json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": { "name": "read_page", "arguments": { "tab": 20 } },
        });
        post(port, Some(&session), call)
    });
    match futures::executor::block_on(handed.next()) {
        Some(Event::Call(call)) => {
            assert_eq!(
                (call.agent.as_str(), call.tool.as_str()),
                ("claude-code", "read_page")
            );
            assert_eq!(call.arguments, json!({ "tab": 20 }));
            call.reply.send(ToolResult::text("Example Domain")).unwrap();
        }
        _ => panic!("the call didn't reach the UI thread"),
    }
    let answer = calling.join().unwrap().json();
    assert_eq!(answer["id"], 7);
    assert_eq!(
        answer["result"]["content"][0],
        json!({ "type": "text", "text": "Example Domain" })
    );
    assert_eq!(answer["result"]["isError"], false);
}

/// A call the UI drops, as when the user turns agents off, is answered with an error.
#[test]
fn a_dropped_call_is_an_error() {
    let (server, mut handed) = server();
    let port = server.port();
    let calling = thread::spawn(move || {
        let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "list_tabs" } });
        post(port, None, call)
    });
    drop(futures::executor::block_on(handed.next()));
    let answer = calling.join().unwrap().json();
    assert_eq!(answer["result"]["isError"], true);
}

/// DELETE ends the session, and the agent's last one tells the UI it's gone. A session the server
/// doesn't know gets 404, which asks the client to initialize again.
#[test]
fn a_session_ends_with_delete() {
    let (server, mut handed) = server();
    let port = server.port();
    let session = initialize(port).header("mcp-session-id").unwrap();
    futures::executor::block_on(handed.next());
    let host = format!("127.0.0.1:{port}");
    let headers = [
        ("Host", host.as_str()),
        ("Authorization", "Bearer s3cret"),
        ("Mcp-Session-Id", session.as_str()),
    ];
    assert_eq!(send(port, "DELETE", &headers, "").status, 200);
    match futures::executor::block_on(handed.next()) {
        Some(Event::Bye { agent }) => assert_eq!(agent, "claude-code"),
        _ => panic!("the UI didn't hear the agent go"),
    }
    let ping = post(
        port,
        Some(&session),
        json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }),
    );
    assert_eq!(ping.status, 404);
}

/// A connection kept open takes one request after another, and a body is read by its length.
#[test]
fn requests_are_read_one_after_another() {
    let two =
        "POST /mcp HTTP/1.1\r\nContent-Length: 2\r\nX-A: 1\r\n\r\n{}GET /mcp HTTP/1.1\r\n\r\n";
    let mut reader = Cursor::new(two.as_bytes());
    let first = read_request(&mut reader).unwrap().unwrap();
    assert_eq!(
        (
            first.method.as_str(),
            first.path.as_str(),
            first.body.as_slice()
        ),
        ("POST", "/mcp", &b"{}"[..])
    );
    assert_eq!(first.header("x-a"), Some("1"));
    let second = read_request(&mut reader).unwrap().unwrap();
    assert_eq!(second.method, "GET");
    assert!(read_request(&mut reader).unwrap().is_none());
    let chunked = "POST /mcp HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n";
    assert!(read_request(&mut Cursor::new(chunked.as_bytes())).is_err());
    let endless = format!("POST /mcp HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(MAX_HEAD));
    assert!(read_request(&mut Cursor::new(endless.as_bytes())).is_err());
}
