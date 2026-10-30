//! The notes services New Note opens a new document in, as Arc's Settings › Profiles › New
//! Documents picks one per profile. Each profile's pick is kept by the core under its name here.

/// Each service: the name the profile keeps, its title, and the URL that opens a new document,
/// in its own .new shortcut, in the user's account there. Arc's list and order.
pub const SERVICES: &[(&str, &str, &str)] = &[
    ("notion", "Notion", "https://notion.new"),
    ("google-docs", "Google Docs", "https://docs.new"),
    ("word", "Word", "https://word.new"),
    ("confluence", "Confluence", "https://confluence.new"),
];

/// Ours: what New Note opens until the profile picks a service, Arc's own notes being gone.
const DEFAULT: usize = 1;

/// The index in `SERVICES` of the service the profile keeps as `kept`, or the default's for none
/// or one this build doesn't know.
pub fn service(kept: &str) -> usize {
    SERVICES.iter().position(|&(name, _, _)| name == kept).unwrap_or(DEFAULT)
}

/// The URL New Note opens for the service the profile keeps as `kept`.
pub fn new_document(kept: &str) -> &'static str {
    SERVICES[service(kept)].2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_with_no_service_picked_gets_google_docs() {
        assert_eq!(new_document(""), "https://docs.new");
        assert_eq!(new_document("arc-notes"), "https://docs.new");
        assert_eq!(new_document("notion"), "https://notion.new");
    }
}
