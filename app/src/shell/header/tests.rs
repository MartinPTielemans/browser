//! Where the shell asks for the Site Controls, and the tooltips its buttons ask for, on a host
//! that records them.

use std::rc::Rc;

use gpui::{
    AppContext, Bounds, Entity, Modifiers, MouseButton, NavigationDirection, TestAppContext,
    VisualTestContext, point, px, size,
};

use crate::{
    passwords::Key,
    shell::{
        Command, FOOTER_HEIGHT, Folders, NAV_SIZE, NAV_TOP, Page, SIDEBAR_BUTTON_X, Shell,
        library::LIBRARY_BACK_INSET, recorder::Recorder,
    },
    site_controls::Place,
};

/// A shell showing one Space with `pinned` and `today` tabs and `folders`.
fn shell_with(
    cx: &mut TestAppContext,
    pinned: &[u64],
    today: &[u64],
    folders: Folders,
) -> (gpui::Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        let space = (1, "Work".into(), None, 0, "".into(), "".into(), "".into());
        shell.set_spaces(vec![space], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, pinned.to_vec(), today.to_vec(), Vec::new(), folders, cx);
    });
    (shell, host)
}

/// From the command bar as from its button: hanging from the button at x 191 of the 228 pt
/// sidebar, where Arc's is, under the URL field.
#[gpui::test]
fn site_controls_open_under_the_url_field(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1, 2], Folders::default());
    shell.update(cx, |shell, cx| {
        shell.set_selected(2, cx);
        shell.run(Command::SiteControls, "ui", cx);
    });
    let button = Bounds::new(point(px(191.), px(49.)), size(px(30.), px(30.)));
    let field = Bounds::new(point(px(8.), px(46.)), size(px(212.), px(36.)));
    assert_eq!(host.site_controls.take(), [(2, Place::Button { field, button })]);
}

/// With the sidebar hidden they open at the page's top left corner, as an extension's popup does.
#[gpui::test]
fn site_controls_open_at_the_corner_with_the_sidebar_hidden(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1], Folders::default());
    shell.update(cx, |shell, cx| {
        shell.set_selected(1, cx);
        shell.sidebar_hidden = true;
        shell.run(Command::SiteControls, "ui", cx);
    });
    let corner = point(px(18.), px(8.));
    assert_eq!(host.site_controls.take(), [(1, Place::Corner(corner))]);
}

/// The new tab page has no site.
#[gpui::test]
fn site_controls_need_a_tab(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1], Folders::default());
    shell.update(cx, |shell, cx| shell.run(Command::SiteControls, "ui", cx));
    assert!(host.site_controls.take().is_empty());
}

/// A drawn shell showing one Space.
fn drawn_shell(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        let space = (1, "Space".into(), None, 1, "".into(), "".into(), "".into());
        shell.set_spaces(vec![space], cx);
        shell.set_space(1, cx);
    });
    cx.run_until_parked();
    (shell, cx, host)
}

/// The tooltips the shell asked for as the pointer moved to (`x`, `y`).
fn move_to(cx: &mut VisualTestContext, host: &Recorder, x: f32, y: f32) -> Vec<String> {
    host.take();
    cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::none());
    cx.run_until_parked();
    host.take().into_iter().filter(|call| call.starts_with("tooltip")).collect()
}

/// Resting on the footer's Library button or the header's sidebar button asks for its name and
/// keys, and leaving it ends the tooltip. Coming straight from a favorite's tile, whose leaving
/// ends its own tooltip later in the same move, the button's still shows, as in the check.
#[gpui::test]
fn sidebar_buttons_name_themselves(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    shell.update(cx, |shell, cx| shell.set_favorites(vec![(10, 1)], cx));
    cx.run_until_parked();
    let (tile, height) = shell.read_with(cx, |shell, _| {
        let tile = shell.tip_frames.borrow()[&10].center();
        (tile, f32::from(shell.viewport.height))
    });
    move_to(cx, &host, tile.x.into(), tile.y.into());
    let library = (LIBRARY_BACK_INSET + NAV_SIZE / 2., height - FOOTER_HEIGHT / 2.);
    let asked = move_to(cx, &host, library.0, library.1);
    assert_eq!(asked.last().map(String::as_str), Some("tooltip View Library ⇧⌘L"));
    assert_eq!(move_to(cx, &host, 100., 300.), ["tooltip none"]);
    let sidebar = move_to(cx, &host, SIDEBAR_BUTTON_X + NAV_SIZE / 2., NAV_TOP + NAV_SIZE / 2.);
    assert_eq!(sidebar, ["tooltip Hide Sidebar ⌘S"]);
    assert_eq!(move_to(cx, &host, 100., 300.), ["tooltip none"]);
}

/// Resting on Clear says what it does in Arc's words, with no keys, and leaving it, or pressing
/// it, ends the tooltip.
#[gpui::test]
fn clear_says_what_it_does(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, Vec::new(), vec![10, 11], Vec::new(), Folders::default(), cx)
    });
    // The pointer coming onto the sidebar fades Clear in.
    move_to(cx, &host, 100., 300.);
    shell.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    let clear = shell.read_with(cx, |shell, _| shell.button_frames.borrow()["clear"].center());
    let (x, y) = (f32::from(clear.x), f32::from(clear.y));
    assert_eq!(move_to(cx, &host, x, y), ["tooltip Clear unpinned tabs in this space "]);
    assert_eq!(move_to(cx, &host, 100., 300.), ["tooltip none"]);
    move_to(cx, &host, x, y);
    host.take();
    cx.simulate_mouse_down(clear, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert!(host.take().iter().any(|call| call == "tooltip none"));
}

/// A press in the sidebar with no move before it, as the check's click on a permission question's
/// Allow, tells the sidebar the pointer is over it, so Clear and the URL field's buttons show once
/// the question closes, and a move to the page hides them.
#[gpui::test]
fn a_press_with_no_move_before_it_puts_the_pointer_over_the_sidebar(cx: &mut TestAppContext) {
    let (shell, cx, _host) = drawn_shell(cx);
    let tools = |shell: &Entity<Shell>, cx: &mut VisualTestContext| {
        cx.run_until_parked();
        shell.read_with(cx, |shell, _| (shell.sidebar_hovered, shell.tools_shown()))
    };
    assert_eq!(tools(&shell, cx), (false, false));
    cx.simulate_click(point(px(100.), px(300.)), Modifiers::none());
    assert_eq!(tools(&shell, cx), (true, true));
    cx.simulate_mouse_move(point(px(600.), px(400.)), None, Modifiers::none());
    assert!(!tools(&shell, cx).0);
    // A press on the page leaves the sidebar's hover where it is.
    cx.simulate_click(point(px(600.), px(400.)), Modifiers::none());
    assert!(!tools(&shell, cx).0);
}

/// A mouse's back and forward buttons over the sidebar go back and forward in the selected tab,
/// as over Chrome's toolbar, and do nothing where its page can't.
#[gpui::test]
fn mouse_back_and_forward_over_the_sidebar_go_back_and_forward(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    let mut page = Page { url: "https://example.org/".into(), ..Page::default() };
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, Vec::new(), vec![10, 11], Vec::new(), Folders::default(), cx);
        shell.set_selected(10, cx);
    });
    let mut press = |page: &Page, direction| {
        shell.update(cx, |shell, cx| shell.set_page(10, page.clone(), cx));
        cx.run_until_parked();
        host.take();
        let button = MouseButton::Navigate(direction);
        cx.simulate_mouse_down(point(px(100.), px(300.)), button, Modifiers::none());
        cx.run_until_parked();
        host.take().into_iter().filter(|call| call.ends_with(" 10")).collect::<Vec<_>>()
    };
    assert!(press(&page, NavigationDirection::Back).is_empty());
    assert!(press(&page, NavigationDirection::Forward).is_empty());
    page.can_go_back = true;
    assert_eq!(press(&page, NavigationDirection::Back), ["back 10"]);
    assert!(press(&page, NavigationDirection::Forward).is_empty());
    page.can_go_forward = true;
    assert_eq!(press(&page, NavigationDirection::Forward), ["forward 10"]);
}

/// Whether the pointer is over the sidebar and Clear and the URL field's buttons show, once the
/// sidebar has stopped moving.
fn tools(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> (bool, bool) {
    shell.update(cx, |shell, cx| {
        shell.sidebar_motion = None;
        cx.notify();
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| (shell.sidebar_hovered, shell.tools_shown()))
}

/// The pointer leaving while the sidebar is hidden, with nothing drawn to hear it go, leaves ⌘S's
/// sidebar without its tools until the pointer comes over it.
#[gpui::test]
fn the_sidebar_comes_back_with_its_tools_only_under_the_pointer(cx: &mut TestAppContext) {
    let (shell, cx, _host) = drawn_shell(cx);
    cx.simulate_mouse_move(point(px(100.), px(300.)), None, Modifiers::none());
    assert_eq!(tools(&shell, cx), (true, true));
    shell.update(cx, |shell, cx| shell.toggle_sidebar(cx));
    assert!(!tools(&shell, cx).0);
    cx.simulate_mouse_move(point(px(600.), px(400.)), None, Modifiers::none());
    shell.update(cx, |shell, cx| shell.toggle_sidebar(cx));
    assert_eq!(tools(&shell, cx), (false, false));
    cx.simulate_mouse_move(point(px(100.), px(300.)), None, Modifiers::none());
    assert_eq!(tools(&shell, cx), (true, true));
}

/// The reveal's panel shows them, and docking it, under the pointer that keeps it out, keeps them
/// with no fade.
#[gpui::test]
fn the_revealed_sidebar_keeps_its_tools_as_it_docks(cx: &mut TestAppContext) {
    let (shell, cx, _host) = drawn_shell(cx);
    shell.update(cx, |shell, cx| shell.toggle_sidebar(cx));
    assert_eq!(tools(&shell, cx), (false, true));
    shell.update(cx, |shell, cx| shell.set_revealed(true, cx));
    assert_eq!(tools(&shell, cx), (false, true));
    shell.update(cx, |shell, cx| shell.toggle_sidebar(cx));
    assert_eq!(tools(&shell, cx), (true, true));
    assert_eq!(shell.read_with(cx, |shell, _| shell.tools_opacity()), 1.);
}

/// A click on the URL field opens the command bar over the field, as Arc's does, and ⌘L centred.
/// With the sidebar hidden there's no field, and the bar is centred (*interim*).
#[gpui::test]
fn a_click_on_the_url_field_opens_the_command_bar_over_it(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        let space = (1, "Work".into(), None, 0, "".into(), "".into(), "".into());
        shell.set_spaces(vec![space], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, Vec::new(), vec![1, 2], Vec::new(), Folders::default(), cx);
        shell.set_selected(2, cx);
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(100.), px(64.)), Modifiers::none());
    let field = Bounds::new(point(px(8.), px(46.)), size(px(212.), px(36.)));
    assert_eq!(host.command_bars.take(), [Some(field)]);
    shell.update(cx, |shell, cx| shell.run(Command::EditUrl, "ui", cx));
    assert_eq!(host.command_bars.take(), [None]);
    shell.update(cx, |shell, cx| {
        shell.sidebar_hidden = true;
        shell.run(Command::EditUrl, "sidebar", cx);
    });
    assert_eq!(host.command_bars.take(), [None]);
}

/// A click on the URL field's Copy URL button says so in the toast, as ⇧⌘C does and Arc's button
/// does, and swaps its glyph for the checkmark, without opening the command bar.
#[gpui::test]
fn copy_url_shows_the_toast_and_the_checkmark(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, Vec::new(), vec![1], Vec::new(), Folders::default(), cx);
        shell.set_selected(1, cx);
        let page = Page { url: "https://example.org/?utm_source=x".into(), ..Page::default() };
        shell.set_page(1, page, cx);
    });
    // The pointer coming onto the sidebar fades the button in, at check.sh's point.
    move_to(cx, &host, 100., 300.);
    shell.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    cx.simulate_click(point(px(178.), px(64.)), Modifiers::none());
    assert_eq!(host.toasts.take(), ["Copied Current URL"]);
    // With Arc's share button, for the page without its tracking parameters.
    let calls = host.take();
    let shares: Vec<&String> =
        calls.iter().filter(|call| call.starts_with("toast share")).collect();
    assert_eq!(shares.len(), 1);
    assert!(shares[0].starts_with("toast share https://example.org/ "), "{shares:?}");
    assert!(host.command_bars.take().is_empty());
    assert!(shell.read_with(cx, |shell, _| shell.url_copied.is_some()));
}

/// The password manager's key shows after the host of the tab it's for. A press on it brings back
/// the offer that showing another tab closed, or opens the Site Controls, which list the site's
/// logins, and it goes once the page has nothing for the password manager.
#[gpui::test]
fn the_key_brings_the_offer_back_or_opens_the_site_controls(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    let page = Page { url: "https://example.com/".into(), ..Page::default() };
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, Vec::new(), vec![1, 2], Vec::new(), Folders::default(), cx);
        shell.set_selected(1, cx);
        shell.set_page(1, page, cx);
        shell.set_key(2, Some(Key::Offer), cx);
    });
    cx.run_until_parked();
    let key = |shell: &Entity<Shell>, cx: &mut VisualTestContext| {
        shell.read_with(cx, |shell, _| shell.button_frames.borrow().get("key").copied())
    };
    assert_eq!(key(&shell, cx), None, "another tab's key showed");
    let press = |at: Bounds<gpui::Pixels>, cx: &mut VisualTestContext| {
        cx.simulate_mouse_down(at.center(), gpui::MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at.center(), gpui::MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
    };

    shell.update(cx, |shell, cx| shell.set_key(1, Some(Key::Offer), cx));
    cx.run_until_parked();
    let at = key(&shell, cx).expect("the tab's key didn't show");
    host.take();
    press(at, cx);
    assert_eq!(host.take(), ["show offer 1"]);

    shell.update(cx, |shell, cx| shell.set_key(1, Some(Key::Saved), cx));
    cx.run_until_parked();
    press(at, cx);
    assert_eq!(host.site_controls.take().iter().map(|(tab, _)| *tab).collect::<Vec<_>>(), [1]);
    assert!(!host.take().iter().any(|call| call.starts_with("show offer")));

    shell.update(cx, |shell, cx| shell.set_key(1, None, cx));
    cx.run_until_parked();
    press(at, cx);
    assert!(host.site_controls.take().is_empty(), "a press where the key was opened them");
    assert!(!host.take().iter().any(|call| call.starts_with("show offer")));
}

/// ⌃⇧⌘C copies the page's selection as a Markdown quote, a line a line, with the link to the page
/// under it, and with nothing selected the link alone.
#[gpui::test]
fn copy_url_as_quote_quotes_the_selection(cx: &mut TestAppContext) {
    let (shell, cx, host) = drawn_shell(cx);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, Vec::new(), vec![1], Vec::new(), Folders::default(), cx);
        shell.set_selected(1, cx);
        let page = Page { url: "https://example.org/a?utm_source=x".into(), ..Page::default() };
        shell.set_page(1, page, cx);
        shell.set_title(1, "A [page]".into(), cx);
    });
    *host.selection.borrow_mut() = " First line\n\nsecond line \n".into();
    shell.update(cx, |shell, cx| shell.run(Command::CopyUrlAsQuote, "ui", cx));
    let link = r"[A \[page\]](https://example.org/a)";
    let quote = format!("> First line\n>\n> second line\n\n{link}");
    assert_eq!(host.copied.take(), [quote]);
    assert_eq!(host.toasts.take(), ["Copied URL as Quote"]);
    host.selection.borrow_mut().clear();
    shell.update(cx, |shell, cx| shell.run(Command::CopyUrlAsQuote, "ui", cx));
    assert_eq!(host.copied.take(), [link]);
    assert_eq!(host.toasts.take(), ["Copied URL as Markdown"]);
}
