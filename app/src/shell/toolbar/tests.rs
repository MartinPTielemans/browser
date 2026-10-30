//! The toolbar's toggle, what it keeps for the next launch, and where it puts the page, on a host
//! that records what the shell asks of the core.

use std::rc::Rc;

use gpui::{AppContext, Bounds, Entity, TestAppContext, point, px, size};

use super::super::{Command, Folders, Page, Shell, Split, recorder::Recorder, user_keys};
use super::{
    HEIGHT, full_url, full_url_shown, toolbar_from, toolbar_layout, toolbar_shown, toolbar_text,
    under_toolbar,
};
use crate::palette::Tone;
use crate::tokens::shell::{CARD_INSET, NAV_SIZE, URL_FIELD_HEIGHT, URL_FIELD_TOP};

/// A shell showing tab 10 of `tabs`, in a split view if there are more, in a window 1200 by 800.
fn shell_of(tabs: &[u64], cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Work".into(), None, 0, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.viewport = size(px(1200.), px(800.));
        let split = (tabs.len() > 1).then(|| Split {
            tabs: tabs.to_vec(),
            stacked: false,
            shares: Vec::new(),
            expanded: 0,
        });
        let today = tabs.to_vec();
        shell.set_tabs(1, Vec::new(), today, split.into_iter().collect(), Folders::default(), cx);
        shell.set_selected(10, cx);
        let page = Page { url: "https://example.org/".into(), ..Page::default() };
        shell.set_page(10, page, cx);
    });
    (shell, host)
}

fn toggle(shell: &Entity<Shell>, cx: &mut TestAppContext) {
    shell.update(cx, |shell, cx| shell.run(Command::ToggleToolbar, "ui", cx));
}

fn page(shell: &Entity<Shell>, cx: &mut TestAppContext) -> Bounds<gpui::Pixels> {
    shell.read_with(cx, |shell, _| shell.page_frame(shell.viewport).0)
}

/// Off at first, as Arc's. ⇧⌘D shows it in every window and keeps that for the next launch, the
/// View menu's item then says Hide Toolbar, and ⇧⌘D again takes it away.
#[gpui::test]
fn shift_cmd_d_shows_the_toolbar_and_takes_it_away(cx: &mut TestAppContext) {
    let (shell, host) = shell_of(&[10], cx);
    let command = cx.update(|cx| user_keys(cx).command_for("shift-cmd-d", false));
    assert_eq!(command, Some(Command::ToggleToolbar));
    assert_eq!(host.menu_bar.borrow().toolbar_title(), "Show Toolbar");
    assert!(!cx.update(|cx| toolbar_shown(cx)));

    toggle(&shell, cx);
    assert_eq!(host.menu_bar.borrow().toolbar_title(), "Hide Toolbar");
    assert!(cx.update(|cx| toolbar_shown(cx)), "for every window");
    assert_eq!(host.toolbar_saved.get(), Some(true));
    let other = cx.new(|cx| Shell::new(Rc::new(Recorder::default()), cx));
    other.update(cx, |other, cx| other.set_toolbar(toolbar_shown(cx), cx));
    assert!(other.read_with(cx, |other, _| other.toolbar_shows()), "as it draws");

    toggle(&shell, cx);
    assert_eq!(host.menu_bar.borrow().toolbar_title(), "Show Toolbar");
    assert!(!cx.update(|cx| toolbar_shown(cx)));
    assert_eq!(host.toolbar_saved.get(), Some(false));
}

/// What it keeps reads back as it was, and anything else, such as no file, leaves it off.
#[test]
fn the_toolbar_reads_back_as_it_was_kept() {
    assert!(toolbar_from(toolbar_text(true)));
    assert!(!toolbar_from(toolbar_text(false)));
    assert!(!toolbar_from(""));
    assert!(toolbar_from("shown\n"));
}

/// The page starts under the strip, its top corners square, and its card keeps its place; back
/// where it was once the toolbar goes, and over the whole window with a page full screen.
#[gpui::test]
fn the_page_starts_under_the_toolbar(cx: &mut TestAppContext) {
    let (shell, host) = shell_of(&[10], cx);
    let without = page(&shell, cx);
    toggle(&shell, cx);
    let with = page(&shell, cx);
    assert_eq!(with.top(), without.top() + px(HEIGHT));
    assert_eq!(with.size.height, without.size.height - px(HEIGHT));
    assert_eq!((with.left(), with.size.width), (without.left(), without.size.width));
    shell.update(cx, |shell, cx| shell.lay_out_page(cx));
    assert!(host.under_toolbar.get(), "square at the top");

    shell.update(cx, |shell, _| shell.page_fullscreen = true);
    let full = page(&shell, cx);
    assert_eq!(full.origin, point(px(0.), px(0.)), "full screen has no toolbar");
    shell.update(cx, |shell, _| shell.page_fullscreen = false);

    toggle(&shell, cx);
    assert_eq!(page(&shell, cx), without);
    shell.update(cx, |shell, cx| shell.lay_out_page(cx));
    assert!(!host.under_toolbar.get());
}

/// Back, forward, reload and Copy URL from the strip's left, the URL field after them, then Site
/// Controls, and in a split view its controls at the end, the field making room.
#[test]
fn the_toolbar_lays_out_the_sidebars_controls_left_to_right() {
    let card = Bounds::new(point(px(238.), px(CARD_INSET)), size(px(952.), px(780.)));
    let alone = toolbar_layout(card, false);
    assert_eq!(alone.strip, Bounds::new(card.origin, size(px(952.), px(HEIGHT))));
    let order = [alone.back, alone.forward, alone.reload, alone.copy];
    assert!(order.windows(2).all(|pair| pair[0] + NAV_SIZE <= pair[1]), "{order:?}");
    assert!(alone.copy + NAV_SIZE < f32::from(alone.field.left()));
    assert!(f32::from(alone.field.right()) < alone.site_controls);
    assert!(alone.site_controls + NAV_SIZE <= f32::from(card.right()));
    assert_eq!(alone.field.size.height, px(URL_FIELD_HEIGHT), "the sidebar's field");
    assert_eq!(alone.field.center().y, alone.strip.center().y);
    assert_eq!(alone.split, None);

    let split = toolbar_layout(card, true);
    let end = split.split.expect("the split's controls");
    assert!(split.site_controls + NAV_SIZE <= end && end + NAV_SIZE <= f32::from(card.right()));
    assert!(split.field.size.width < alone.field.size.width);

    let low = Bounds::new(card.origin, size(px(952.), px(20.)));
    assert_eq!(under_toolbar(low).size.height, px(0.), "never less than nothing");
}

/// The URL field, back, forward and reload leave the sidebar for the toolbar, and the favorites
/// move up where the field was, as Arc's do; the split's handles over the card give way to the
/// toolbar's button.
#[gpui::test]
fn the_sidebars_field_and_split_handles_move_to_the_toolbar(cx: &mut TestAppContext) {
    let (shell, _) = shell_of(&[10, 11], cx);
    let list = |cx: &mut TestAppContext| shell.read_with(cx, |shell, _| shell.list_start());
    let field = |cx: &mut TestAppContext| shell.read_with(cx, |shell, _| shell.url_field_bounds());
    let handles =
        |cx: &mut TestAppContext| shell.update(cx, |shell, cx| shell.pane_handles(cx).len());
    let sidebar = field(cx).expect("in the sidebar");
    assert_eq!(sidebar.top(), px(URL_FIELD_TOP));
    let before = list(cx);
    assert_eq!(handles(cx), 2);

    toggle(&shell, cx);
    let layout = shell.read_with(cx, |shell, _| shell.toolbar_layout()).expect("it shows");
    assert_eq!(field(cx), Some(layout.field));
    assert!(layout.split.is_some());
    assert_eq!(list(cx), URL_FIELD_TOP);
    assert!(list(cx) < before);
    assert_eq!(handles(cx), 0);
    let nav = shell.update(cx, |shell, cx| shell.nav_buttons(cx).len());
    assert_eq!(nav, 1, "only the sidebar's own button");

    toggle(&shell, cx);
    assert_eq!(field(cx), Some(sidebar));
    assert_eq!(list(cx), before);
}

/// The strip takes the page's colour, and its glyphs the ink that reads on it.
#[gpui::test]
fn the_toolbar_takes_the_pages_colour(cx: &mut TestAppContext) {
    let (shell, _) = shell_of(&[10], cx);
    toggle(&shell, cx);
    for (color, tone) in [(0xffffff, Tone::Light), (0x101820, Tone::Dark)] {
        let url = "https://example.org/".into();
        let page = Page { url, color: Some(color), ..Page::default() };
        shell.update(cx, |shell, cx| shell.set_page(10, page, cx));
        let drawn = shell.read_with(cx, |shell, _| shell.toolbar_look());
        assert_eq!(drawn, (Some(color), tone));
    }
}

/// Show Full URL is the toolbar's, as Arc's: the command bar offers it only while the toolbar
/// shows, by what it would do, and it puts the whole URL in the toolbar's field in every window,
/// kept for the next launch, while the sidebar's field keeps the host.
#[gpui::test]
fn show_full_url_puts_the_whole_url_in_the_toolbar(cx: &mut TestAppContext) {
    let (shell, host) = shell_of(&[10], cx);
    let page = Page { url: "https://www.example.org/a/b?c=d#e".into(), ..Page::default() };
    shell.update(cx, |shell, cx| shell.set_page(10, page, cx));
    let offered = |cx: &mut TestAppContext| {
        shell.read_with(cx, |shell, _| shell.bar_name("Show Full URL", Command::ToggleFullUrl))
    };
    let words = |in_toolbar, cx: &mut TestAppContext| {
        let url = shell.read_with(cx, |shell, _| shell.url(10));
        shell.read_with(cx, |shell, _| shell.field_words(&url, in_toolbar).0.to_string())
    };
    assert_eq!(offered(cx), None, "only with the toolbar");

    toggle(&shell, cx);
    assert_eq!(offered(cx), Some("Show Full URL"));
    assert_eq!(words(true, cx), "example.org");
    shell.update(cx, |shell, cx| shell.run(Command::ToggleFullUrl, "ui", cx));
    assert_eq!(words(true, cx), "example.org/a/b?c=d#e");
    assert_eq!(words(false, cx), "example.org", "the sidebar's keeps the host");
    assert_eq!(offered(cx), Some("Hide Full URL"));
    assert!(cx.update(|cx| full_url_shown(cx)), "for every window");
    assert_eq!(host.full_url_saved.get(), Some(true));

    shell.update(cx, |shell, cx| shell.run(Command::ToggleFullUrl, "ui", cx));
    assert_eq!(words(true, cx), "example.org");
    assert_eq!(host.full_url_saved.get(), Some(false));
}

/// The whole URL as Dia shows it: no scheme, no "www.", no "/" at the end; one with no host as it
/// is.
#[test]
fn the_whole_url_reads_as_dias() {
    assert_eq!(full_url("https://www.iana.org/"), "iana.org");
    assert_eq!(full_url("https://iana.org/domains/"), "iana.org/domains");
    assert_eq!(full_url("http://p/x?y#z"), "p/x?y#z");
    assert_eq!(full_url("http://localhost:8080/a"), "localhost:8080/a");
    assert_eq!(full_url("https://www2.example.org/"), "www2.example.org");
    assert_eq!(full_url("about:blank"), "about:blank");
}
