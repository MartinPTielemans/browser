//! Where the shell asks for the Site Controls, and the tooltips its buttons ask for, on a host
//! that records them.

use std::rc::Rc;

use gpui::{
    AppContext, Bounds, Entity, Modifiers, TestAppContext, VisualTestContext, point, px, size,
};

use crate::shell::{
    Command, FOOTER_HEIGHT, Folders, NAV_SIZE, NAV_TOP, SIDEBAR_BUTTON_X, Shell,
    library::LIBRARY_BACK_INSET, recorder::Recorder,
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
        shell.set_spaces(vec![(1, "Work".into(), None, 0, "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, pinned.to_vec(), today.to_vec(), Vec::new(), folders, cx);
    });
    (shell, host)
}

/// From the command bar as from its button: under the URL field, growing from the button at x 191
/// of the 228 pt sidebar, where Arc's is.
#[gpui::test]
fn site_controls_open_under_the_url_field(cx: &mut TestAppContext) {
    let (shell, host) = shell_with(cx, &[], &[1, 2], Folders::default());
    shell.update(cx, |shell, cx| {
        shell.set_selected(2, cx);
        shell.run(Command::SiteControls, "ui", cx);
    });
    let button = Bounds::new(point(px(191.), px(49.)), size(px(30.), px(30.)));
    assert_eq!(host.site_controls.take(), [(2, point(px(8.), px(88.)), button)]);
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
    assert_eq!(host.site_controls.take(), [(1, corner, Bounds::new(corner, Default::default()))]);
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
        shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into(), "".into())], cx);
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

/// A click on the URL field opens the command bar over the field, as Arc's does, and ⌘L centred.
/// With the sidebar hidden there's no field, and the bar is centred (*interim*).
#[gpui::test]
fn a_click_on_the_url_field_opens_the_command_bar_over_it(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Work".into(), None, 0, "".into(), "".into())], cx);
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
