//! The Space the user picked for a site in Little Arc, which its next link from another app opens
//! in (docs/ux-spec.md › Little Arc). Kept in the profile directory as a line a site, the site and
//! the Space's key (bc_space_key) between a tab. The user's alone: agents get none of it.

use crate::link_preview::site_of;

/// The site a link is remembered by: its host without "www.", lowercase. None for a link with no
/// host, which has no site to remember.
pub fn site(url: &str) -> Option<String> {
    let (scheme, _) = url.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let site = site_of(url);
    let site = site.split(':').next().unwrap_or(site);
    (!site.is_empty()).then(|| site.to_lowercase())
}

/// The key of the Space picked for `url`'s site in `kept`, the file's text.
pub fn picked<'a>(kept: &'a str, url: &str) -> Option<&'a str> {
    let site = site(url)?;
    kept.lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|&(kept, _)| kept == site)
        .map(|(_, key)| key)
}

/// `kept` with the Space of key `key` picked for `url`'s site, the site's line last. `kept` as it
/// is for a link with no site.
pub fn pick(kept: &str, url: &str, key: &str) -> String {
    let Some(site) = site(url) else {
        return kept.to_string();
    };
    let mut lines: Vec<&str> = kept
        .lines()
        .filter(|line| line.split_once('\t').is_some_and(|(kept, _)| kept != site))
        .collect();
    let line = format!("{site}\t{key}");
    lines.push(&line);
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_is_its_host_without_www() {
        assert_eq!(site("https://www.Example.com:8080/a?b").as_deref(), Some("example.com"));
        assert_eq!(site("http://news.example.com/").as_deref(), Some("news.example.com"));
        assert_eq!(site("file:///Users/me/a.html"), None);
        assert_eq!(site("about:blank"), None);
    }

    #[test]
    fn a_pick_replaces_the_sites_last_one() {
        let kept = pick("", "https://example.com/a", "one");
        let kept = pick(&kept, "https://other.org/", "two");
        let kept = pick(&kept, "https://www.example.com/b", "three");
        assert_eq!(kept, "other.org\ttwo\nexample.com\tthree\n");
        assert_eq!(picked(&kept, "https://example.com/c"), Some("three"));
        assert_eq!(picked(&kept, "https://other.org/x"), Some("two"));
        assert_eq!(picked(&kept, "https://elsewhere.net/"), None);
    }
}
