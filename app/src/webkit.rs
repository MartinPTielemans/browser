//! The sites whose video plays only with a key system our Chromium can't have, which open in a
//! WebKit tab instead (docs/decisions.md, decision 9).

/// Each site as a domain, matched with its subdomains, and the path its player lives under.
const SITES: &[(&str, &str)] = &[
    ("netflix.com", ""),
    ("disneyplus.com", ""),
    ("max.com", ""),
    ("hbomax.com", ""),
    ("primevideo.com", ""),
    ("hulu.com", ""),
    ("tv.apple.com", ""),
    ("open.spotify.com", ""),
    ("viaplay.com", ""),
    ("viaplay.dk", ""),
    ("viaplay.se", ""),
    ("viaplay.no", ""),
    ("viaplay.fi", ""),
    ("play.tv2.dk", ""),
    ("dr.dk", "/drtv"),
];

/// Where those sites send their sign-in, which has to happen in WebKit too for its cookies to land
/// there, but which a page in Chromium goes to for other sites as well.
const SIGN_IN: &[(&str, &str)] = &[
    ("accounts.spotify.com", ""),
    ("amazon.com", "/ap"),
    ("idmsa.apple.com", ""),
    ("appleid.apple.com", ""),
];

/// Whether `url` opens in a WebKit tab: one of the sites, or a host in `extra`, which the checks
/// give through `BROWSER_WEBKIT_SITES`.
pub fn plays_in_webkit(url: &str, extra: &[String]) -> bool {
    let Some((host, path)) = host_and_path(url) else {
        return false;
    };
    SITES.iter().any(|&(domain, under)| matches(&host, path, domain, under))
        || extra.iter().any(|domain| matches(&host, path, domain, ""))
}

/// Whether a link in a WebKit tab to `url` stays there, rather than going back to Chromium.
pub fn stays_in_webkit(url: &str, extra: &[String]) -> bool {
    plays_in_webkit(url, extra)
        || host_and_path(url).is_some_and(|(host, path)| {
            SIGN_IN.iter().any(|&(domain, under)| matches(&host, path, domain, under))
        })
}

fn matches(host: &str, path: &str, domain: &str, under: &str) -> bool {
    let on_domain = host == domain
        || host.strip_suffix(domain).is_some_and(|sub| sub.ends_with('.'));
    let under_path = under.is_empty()
        || path
            .strip_prefix(under)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']));
    on_domain && under_path
}

/// An http(s) URL's host, lowercased and without its port or a trailing dot, and its path.
fn host_and_path(url: &str) -> Option<(String, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(end);
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    let host = match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => name,
        _ => host,
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    (!host.is_empty()).then_some((host, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn webkit(url: &str) -> bool {
        plays_in_webkit(url, &[])
    }

    #[test]
    fn a_site_and_its_subdomains_play_in_webkit() {
        assert!(webkit("https://www.netflix.com/watch/81234567"));
        assert!(webkit("https://netflix.com"));
        assert!(webkit("HTTPS://WWW.NETFLIX.COM./browse"));
        assert!(webkit("https://www.disneyplus.com:443/en-dk/home"));
        assert!(webkit("https://tv.apple.com/dk/show/severance"));
        assert!(webkit("http://open.spotify.com/track/1"));
    }

    #[test]
    fn a_lookalike_or_another_scheme_does_not() {
        assert!(!webkit("https://notnetflix.com/"));
        assert!(!webkit("https://netflix.com.example.org/"));
        assert!(!webkit("https://example.org/?r=https://netflix.com"));
        assert!(!webkit("https://netflix.com@example.org/"));
        assert!(!webkit("https://www.apple.com/tv/"));
        assert!(!webkit("https://spotify.com/"));
        assert!(!webkit("file:///Users/x/netflix.com"));
        assert!(!webkit("chrome://settings"));
        assert!(!webkit("netflix.com"));
    }

    #[test]
    fn a_site_under_a_path_plays_only_there() {
        assert!(webkit("https://www.dr.dk/drtv"));
        assert!(webkit("https://www.dr.dk/drtv/serie/matador_123"));
        assert!(webkit("https://www.dr.dk/drtv?x=1"));
        assert!(!webkit("https://www.dr.dk/nyheder"));
        assert!(!webkit("https://www.dr.dk/drtvguide"));
    }

    #[test]
    fn the_checks_add_their_own_hosts() {
        let extra = vec!["127.0.0.1".to_string()];
        assert!(plays_in_webkit("http://127.0.0.1:8123/page.html", &extra));
        assert!(!plays_in_webkit("http://localhost:8123/page.html", &extra));
    }

    #[test]
    fn a_sign_in_stays_in_webkit_without_pulling_chromium_there() {
        assert!(stays_in_webkit("https://accounts.spotify.com/login", &[]));
        assert!(stays_in_webkit("https://www.amazon.com/ap/signin?x=1", &[]));
        assert!(!webkit("https://www.amazon.com/ap/signin?x=1"));
        assert!(!stays_in_webkit("https://www.amazon.com/dp/B000", &[]));
        assert!(!stays_in_webkit("https://www.amazon.com/apple-airpods", &[]));
        assert!(!stays_in_webkit("https://example.org/", &[]));
    }
}
