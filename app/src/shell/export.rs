//! The user's Spaces as a Markdown file of links: each Space's favorites, then its pinned tabs with
//! each folder a list of what it holds (docs/ux-spec.md › Settings).

use super::{Shell, Space};

/// A pinned row as it's written: a link, or a folder and what it holds.
enum Item {
    Link(String, String),
    Folder(String, Vec<Item>),
}

impl Shell {
    /// Every Space of the user's, top to bottom, as Markdown, and how many links it holds. An
    /// agent's Spaces are the agent's, and Today tabs come and go, so neither is written.
    pub fn links_markdown(&self) -> (String, usize) {
        let mut markdown = String::from("# Spaces\n");
        let mut links = 0;
        for space in self.spaces.iter().filter(|space| space.agent.is_empty()) {
            let name = if space.name.is_empty() { "Space" } else { space.name.as_ref() };
            markdown.push_str(&format!("\n## {}\n", text(name)));
            let favorites: Vec<Item> =
                self.favorites_of(space).into_iter().filter_map(|tab| self.link(tab)).collect();
            let pinned = self.pinned_items(space, 0);
            for (heading, items) in [("Favorites", favorites), ("Pinned", pinned)] {
                if !items.is_empty() {
                    markdown.push_str(&format!("\n### {heading}\n\n"));
                    links += write(&mut markdown, &items, 0);
                }
            }
        }
        (markdown, links)
    }

    /// The pinned rows `folder` holds in `space`, or its rows outside folders for 0, top to
    /// bottom, each tab of a split view a link of its own.
    fn pinned_items(&self, space: &Space, folder: u64) -> Vec<Item> {
        let later = |row: &u64| space.splits.iter().any(|split| split.tabs[1..].contains(row));
        let rows =
            space.pinned.iter().filter(|row| !later(row) && space.folder_of(**row) == folder);
        rows.flat_map(|&row| match space.folders.folders.get(&row) {
            Some((name, _)) => vec![Item::Folder(name.to_string(), self.pinned_items(space, row))],
            None => space.panes(row).into_iter().filter_map(|tab| self.link(tab)).collect(),
        })
        .collect()
    }

    /// `tab` as a link to the URL it goes back to, or to its page's for a tab without one.
    fn link(&self, tab: u64) -> Option<Item> {
        let mut url = self.browser.pinned_url(tab);
        if url.is_empty() {
            url = self.tabs.get(&tab).map(|tab| tab.page.url.to_string()).unwrap_or_default();
        }
        (!url.is_empty()).then(|| Item::Link(self.title(tab).to_string(), url))
    }
}

/// Writes `items` as a list, nested `depth` deep, and returns how many links it wrote.
fn write(markdown: &mut String, items: &[Item], depth: usize) -> usize {
    let indent = "  ".repeat(depth);
    items.iter().fold(0, |links, item| match item {
        Item::Link(title, url) => {
            markdown.push_str(&format!("{indent}- [{}]({})\n", text(title), href(url)));
            links + 1
        }
        Item::Folder(name, held) => {
            markdown.push_str(&format!("{indent}- {}\n", text(name)));
            links + write(markdown, held, depth + 1)
        }
    })
}

/// `name` as Markdown text, which a bracket, a backslash or a list's mark would otherwise change.
fn text(name: &str) -> String {
    let mut text = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(c, '\\' | '[' | ']' | '*' | '_' | '`' | '#' | '<' | '>') {
            text.push('\\');
        }
        text.push(if c == '\n' { ' ' } else { c });
    }
    text
}

/// `url` as a link's destination, which a space or a parenthesis would otherwise end.
fn href(url: &str) -> String {
    let mut href = String::with_capacity(url.len());
    for c in url.chars() {
        match c {
            ' ' => href.push_str("%20"),
            '(' => href.push_str("%28"),
            ')' => href.push_str("%29"),
            '<' => href.push_str("%3C"),
            '>' => href.push_str("%3E"),
            c => href.push(c),
        }
    }
    href
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use gpui::{AppContext, TestAppContext};

    use crate::shell::{Folders, Shell, recorder::Recorder};

    /// Each of the user's Spaces gets its favorites and its pinned tabs, folders holding theirs
    /// nested under them, and an agent's Space and Today tabs are left out.
    #[gpui::test]
    fn spaces_export_as_markdown_links(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        host.pinned_urls.borrow_mut().extend([
            (30, "https://mail.example/".to_string()),
            (10, "https://docs.example/a (draft)".to_string()),
            (12, "https://plan.example/".to_string()),
            (13, "https://notes.example/".to_string()),
            (20, "https://agent.example/".to_string()),
        ]);
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(
                vec![
                    (1, "Work".into(), None, 0, "".into(), "".into(), "".into()),
                    (2, "Research".into(), None, 0, "".into(), "Claude Code".into(), "".into()),
                    (3, "Home".into(), None, 2, "".into(), "".into(), "".into()),
                ],
                cx,
            );
            shell.set_favorites(vec![(30, 1)], cx);
            let folders = Folders {
                folders: HashMap::from([(50, ("Q3 [plans]".into(), false))]),
                parents: HashMap::from([(12, 50), (13, 50)]),
            };
            shell.set_tabs(1, vec![10, 50, 12, 13], vec![11], Vec::new(), folders, cx);
            shell.set_tabs(2, vec![20], Vec::new(), Vec::new(), Folders::default(), cx);
            shell.set_space(1, cx);
            for (tab, title) in [(30, "Mail"), (10, "Draft"), (12, "Plan"), (13, "Notes")] {
                shell.set_title(tab, title.into(), cx);
            }
        });
        let (markdown, links) = shell.read_with(cx, |shell, _| shell.links_markdown());
        assert_eq!(
            markdown,
            "# Spaces\n\n## Work\n\n### Favorites\n\n- [Mail](https://mail.example/)\n\n\
             ### Pinned\n\n- [Draft](https://docs.example/a%20%28draft%29)\n- Q3 \\[plans\\]\n  \
             - [Plan](https://plan.example/)\n  - [Notes](https://notes.example/)\n\n## Home\n"
        );
        assert_eq!(links, 4);
    }
}
