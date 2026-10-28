//! What a Chromium-based browser's profiles hold: their bookmarks, from each profile's
//! `Bookmarks`, and their history, from its `History`, a SQLite file. Arc's sidebar is arc.rs's.
//!
//! A browser without Spaces gets one here for each of its profiles, named after the browser, whose
//! pinned tabs and folders are its bookmarks.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use serde_json::Value;

use super::found::{Found, FromProfile, Node, Page, Space, Tab, opens_here};
use super::sources::Source;

/// The most a bookmarks bar puts straight among the pinned tabs; a longer one comes in as a folder
/// of its name, as more would push the Space's Today tabs off the sidebar.
pub const BAR_ROWS: usize = 12;

/// How far back history comes: as far as Chromium keeps it, so all of it is kept here too.
pub const HISTORY_DAYS: u64 = 90;

/// Chromium's times count microseconds from 1601, 11,644,473,600 seconds before 1970.
const FROM_1601: i64 = 11_644_473_600_000_000;

/// What the user can bring from a browser, each shown as a switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Arc's Spaces, with their pinned tabs and folders.
    Spaces,
    /// Arc's favorites.
    Favorites,
    Bookmarks,
    History,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Spaces => "Spaces, with their pinned tabs and folders",
            Kind::Favorites => "Favorites",
            Kind::Bookmarks => "Bookmarks, as a Space's pinned tabs",
            Kind::History => "History",
        }
    }
}

/// What can come from `source`: Arc's sidebar in place of its bookmarks, which it doesn't show.
pub fn kinds(source: &Source) -> &'static [Kind] {
    if source.arc_sidebar().is_some() {
        &[Kind::Spaces, Kind::Favorites, Kind::History]
    } else {
        &[Kind::Bookmarks, Kind::History]
    }
}

/// Whether `source` has anything of `kinds(source)` to bring.
pub fn has_any(source: &Source) -> bool {
    source.arc_sidebar().is_some()
        || source.profiles.iter().any(|profile| {
            profile.path.join("Bookmarks").is_file() || profile.path.join("History").is_file()
        })
}

/// Reads what of `picked` `source` has, for the import to bring in. History goes back
/// HISTORY_DAYS from `now`. Slow for a long history, so it runs away from the UI.
pub fn read(source: &Source, picked: &[Kind], now: SystemTime) -> Result<Found, String> {
    let mut found = Found::default();
    let wants = |kind| picked.contains(&kind);
    if let Some(path) = source.arc_sidebar()
        && (wants(Kind::Spaces) || wants(Kind::Favorites))
    {
        let json = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let sidebar = super::arc::parse(&json)?;
        if wants(Kind::Spaces) {
            found.spaces = sidebar.spaces;
        }
        if wants(Kind::Favorites) {
            found.favorites = sidebar.favorites;
        }
    }
    let name = source.browser.name;
    let since = now
        .checked_sub(Duration::from_secs(HISTORY_DAYS * 24 * 60 * 60))
        .unwrap_or(UNIX_EPOCH);
    for profile in &source.profiles {
        let from = FromProfile::of(&profile.dir);
        let bookmarks = profile.path.join("Bookmarks");
        if wants(Kind::Bookmarks) && bookmarks.is_file() {
            let json = std::fs::read_to_string(&bookmarks).map_err(|error| error.to_string())?;
            let pinned = self::bookmarks(&json)?;
            if !pinned.is_empty() {
                found.spaces.push(Space {
                    // One Space for the browser, or one for each of its profiles, by its name.
                    name: if source.profiles.len() == 1 {
                        name.into()
                    } else {
                        format!("{name} – {}", profile.name)
                    },
                    profile: from.clone(),
                    icon: String::new(),
                    theme: None,
                    pinned,
                });
            }
        }
        let history_file = profile.path.join("History");
        if wants(Kind::History) && history_file.is_file() {
            let pages = history(&history_file, since)?;
            if !pages.is_empty() {
                found.history.push((from, pages));
            }
        }
    }
    Ok(found)
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
        let children = nodes(folder);
        (!children.is_empty()).then(|| (name.to_string(), children))
    };
    let mut pinned = Vec::new();
    if let Some((name, children)) = folder("bookmark_bar", "Bookmarks Bar") {
        if children.len() <= BAR_ROWS {
            pinned = children;
        } else {
            pinned.push(Node::Folder { name, children });
        }
    }
    for (key, fallback) in [("other", "Other Bookmarks"), ("synced", "Mobile Bookmarks")] {
        if let Some((name, children)) = folder(key, fallback) {
            pinned.push(Node::Folder { name, children });
        }
    }
    Ok(pinned)
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
                    name: if name.is_empty() {
                        "Folder".into()
                    } else {
                        name
                    },
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
pub fn history(path: &Path, since: SystemTime) -> Result<Vec<Page>, String> {
    let copy = tempfile(path)?;
    let pages = read_history(&copy, since);
    std::fs::remove_file(&copy).ok();
    pages.map_err(|error| error.to_string())
}

fn read_history(path: &Path, since: SystemTime) -> rusqlite::Result<Vec<Page>> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let db = Connection::open_with_flags(path, flags)?;
    let since = since
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_micros() as i64)
        + FROM_1601;
    let mut query = db.prepare(
        "SELECT url, title, visit_count, typed_count, last_visit_time FROM urls \
         WHERE hidden = 0 AND last_visit_time > ?1 ORDER BY last_visit_time DESC",
    )?;
    let rows = query.query_map([since], |row| {
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

/// A copy of the file at `path` in the system's temporary folder.
fn tempfile(path: &Path) -> Result<std::path::PathBuf, String> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let name = format!(
        "ilo-import-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let copy = std::env::temp_dir().join(name);
    std::fs::copy(path, &copy).map_err(|error| error.to_string())?;
    Ok(copy)
}
