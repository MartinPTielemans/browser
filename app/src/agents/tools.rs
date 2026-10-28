//! The tools an agent calls, as MCP lists them, and what they answer. Each works only in the
//! agent's own Space (decision 8), with prototypes/arcish/agent.js as the reference: it reads
//! pages through the core, never through script of its own, and never sees a password, a cookie
//! or autofill.

use serde_json::{Value, json};

use super::TabInfo;

/// What `initialize` tells the agent about the browser.
pub const INSTRUCTIONS: &str = concat!(
    env!("APP_NAME"),
    " is the user's own browser. You work in a Space of your own, which shows in the user's \
     sidebar with your name, so they can watch. You can only see the tabs in your Space, never \
     the user's, and never passwords, cookies or autofill. list_tabs lists your tabs, and \
     read_page reads a page's text. The user opens pages for you in your Space."
);

/// A page's text an agent gets at most, as a long page would fill its context.
pub const MAX_TEXT: usize = 40_000;

/// The tools, for tools/list.
pub fn list() -> Value {
    let tab = json!({
        "type": "integer",
        "description": "The tab's id, from list_tabs. Without it, the tab you used last, else your first."
    });
    json!([
        {
            "name": "list_tabs",
            "title": "List tabs",
            "description": "List the tabs in your Space: each one's id, title and URL. * marks the tab you used last.",
            "inputSchema": { "type": "object", "properties": {} },
            "annotations": { "readOnlyHint": true, "openWorldHint": false },
        },
        {
            "name": "read_page",
            "title": "Read page",
            "description": "Read the text of a page in your Space, as a screen reader would.",
            "inputSchema": { "type": "object", "properties": { "tab": tab } },
            "annotations": { "readOnlyHint": true, "openWorldHint": true },
        },
    ])
}

/// list_tabs' answer.
pub fn tab_list(tabs: &[TabInfo], current: Option<u64>) -> String {
    if tabs.is_empty() {
        return "Your Space has no tabs yet. The user can open pages in it for you.".into();
    }
    let lines = tabs.iter().map(|tab| {
        let mark = if Some(tab.id) == current { '*' } else { ' ' };
        format!("{mark} {}  {}  {}", tab.id, tab.title, tab.url)
    });
    lines.collect::<Vec<_>>().join("\n")
}

/// read_page's answer: which page it is, then its text, cut at `MAX_TEXT` bytes.
pub fn page_text(tab: &TabInfo, text: &str) -> String {
    let mut end = text.len().min(MAX_TEXT);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let cut = match end < text.len() {
        true => "\n\n[The page goes on: this is its first part.]",
        false => "",
    };
    format!(
        "Tab {}: {}\nURL: {}\n\n{}{cut}",
        tab.id,
        tab.title,
        tab.url,
        &text[..end]
    )
}

/// The `tab` argument, as a number or a string of one.
pub fn tab_argument(arguments: &Value) -> Result<Option<u64>, String> {
    match arguments.get("tab") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number.as_u64().map(Some).ok_or_else(not_a_tab),
        Some(Value::String(text)) => text.trim().parse().map(Some).map_err(|_| not_a_tab()),
        Some(_) => Err(not_a_tab()),
    }
}

fn not_a_tab() -> String {
    "tab is a tab's id, a number from list_tabs.".into()
}

/// What an action an agent took is, for the activity log, by the name the core's entry gives it.
pub fn describe(action: &str) -> &'static str {
    match action {
        "list_tabs" => "Listed its tabs",
        "read_page" => "Read the page",
        "connected" => "Connected",
        "disconnected" => "Disconnected",
        "released" => "Let go of the tab",
        "taken back" => "You took the tab back",
        _ => "Acted",
    }
}
