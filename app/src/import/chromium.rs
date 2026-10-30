//! What a Chromium-based browser's profile holds: its bookmarks, from its `Bookmarks`, and its
//! history, from its `History`, a SQLite file. Arc's sidebar is arc.rs's.

use std::path::Path;
use std::time::SystemTime;

use serde_json::Value;

use super::extensions;
use super::found::{Node, Page, Tab, opens_here};
use super::read::{Copy, Kind, Read, ReadError, folder_name, micros, pinned};

/// Chromium's times count microseconds from 1601, 11,644,473,600 seconds before 1970.
const FROM_1601: i64 = 11_644_473_600_000_000;

/// What of `picked` the profile at `path` has, its history since `since`.
pub fn read(path: &Path, picked: &[Kind], since: SystemTime) -> Result<Read, ReadError> {
    let mut read = Read::default();
    let bookmarks = path.join("Bookmarks");
    if picked.contains(&Kind::Bookmarks) && bookmarks.is_file() {
        read.pinned = self::bookmarks(&std::fs::read_to_string(&bookmarks)?)?;
    }
    let history = path.join("History");
    if picked.contains(&Kind::History) && history.is_file() {
        read.pages = self::history(&history, since)?;
    }
    if picked.contains(&Kind::Extensions) {
        read.extensions = extensions::read(path);
    }
    Ok(read)
}

/// A profile's bookmarks as pinned tabs and folders: the bar's at the top, or in a folder of its
/// name once it holds more than BAR_ROWS, then the other bookmarks and the phone's, each in a
/// folder of its name. Pages that can't open here and folders left empty stay out.
pub fn bookmarks(json: &str) -> Result<Vec<Node>, String> {
    let root: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let roots = root.get("roots").ok_or("no bookmarks in the file")?;
    let folder = |key: &str, fallback: &str| {
        let folder = roots.get(key)?;
        let name = folder
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(fallback);
        Some((name.to_string(), nodes(folder)))
    };
    let folders = [("other", "Other Bookmarks"), ("synced", "Mobile Bookmarks")];
    Ok(pinned(
        folder("bookmark_bar", "Bookmarks Bar"),
        folders
            .iter()
            .filter_map(|(key, fallback)| folder(key, fallback))
            .collect(),
    ))
}

/// A bookmark folder's children that come in.
fn nodes(folder: &Value) -> Vec<Node> {
    let children = folder.get("children").and_then(Value::as_array);
    let text = |node: &Value, key| {
        node.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    children
        .into_iter()
        .flatten()
        .filter_map(|node| match node.get("type").and_then(Value::as_str)? {
            "url" => {
                let url = text(node, "url");
                opens_here(&url).then(|| {
                    Node::Tab(Tab {
                        url,
                        name: text(node, "name"),
                    })
                })
            }
            "folder" => {
                let children = nodes(node);
                let name = text(node, "name");
                (!children.is_empty()).then(|| Node::Folder {
                    name: folder_name(&name),
                    children,
                })
            }
            _ => None,
        })
        .collect()
}

/// The pages of the `History` file at `path` visited since `since`, the latest first, but for
/// those Chromium hides, such as a frame's, and those that can't open here. It reads a copy, as the
/// browser keeps the file locked while it runs.
pub fn history(path: &Path, since: SystemTime) -> Result<Vec<Page>, ReadError> {
    let copy = Copy::of(path)?;
    let db = copy.open()?;
    let mut query = db.prepare(
        "SELECT url, title, visit_count, typed_count, last_visit_time FROM urls \
         WHERE hidden = 0 AND last_visit_time > ?1 ORDER BY last_visit_time DESC",
    )?;
    let rows = query.query_map([micros(since) + FROM_1601], |row| {
        Ok(Page {
            url: row.get(0)?,
            title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
            visits: row.get::<_, i64>(2)?.max(0) as u32,
            typed: row.get::<_, i64>(3)?.max(0) as u32,
            last_visit: row.get::<_, i64>(4)? - FROM_1601,
        })
    })?;
    let mut pages = Vec::new();
    for page in rows {
        let page = page?;
        if opens_here(&page.url) {
            pages.push(page);
        }
    }
    Ok(pages)
}
