//! Which tabs' links to other sites open in a link preview, on a host that records what the shell
//! asks.

use std::rc::Rc;

use gpui::{AppContext, TestAppContext};

use crate::shell::{Folders, Shell, recorder::Recorder};

/// A pinned tab and a favorite keep their own site, so their links to others preview. A Today
/// tab's go where they lead, and nothing previews in an agent's Space.
#[gpui::test]
fn pinned_tabs_and_favorites_preview_links(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "Space 1".into(), None, 0, "".into(), "".into(), "".into()),
                (2, "Research".into(), None, 0, "".into(), "Claude Code".into(), "".into()),
            ],
            cx,
        );
        shell.set_favorites(vec![(30, 0)], cx);
        shell.set_tabs(1, vec![10], vec![11], Vec::new(), Folders::default(), cx);
        shell.set_tabs(2, vec![20], vec![21], Vec::new(), Folders::default(), cx);
        shell.set_space(1, cx);
    });
    shell.read_with(cx, |shell, _| {
        assert!(shell.previews_links_of(10));
        assert!(shell.previews_links_of(30));
        assert!(!shell.previews_links_of(11));
        // A tab of a Space not shown isn't in this window.
        assert!(!shell.previews_links_of(20));
    });
    shell.update(cx, |shell, cx| shell.set_space(2, cx));
    shell.read_with(cx, |shell, _| {
        assert!(!shell.previews_links_of(20));
        assert!(!shell.previews_links_of(30));
    });
}
