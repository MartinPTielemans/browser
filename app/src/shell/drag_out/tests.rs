//! A row dragged out of its window, and another window's tab dragged onto the list.

use std::rc::Rc;

use gpui::{Entity, TestAppContext, VisualTestContext, px, size};

use crate::shell::{
    Command, Folders, Shell,
    drag::detents::{X, at, row_y},
    recorder::Recorder,
};

/// A window 1000 × 700 showing one Space with `today` tabs.
fn window<'a>(
    cx: &'a mut TestAppContext,
    today: &[u64],
) -> (Entity<Shell>, Rc<Recorder>, &'a mut VisualTestContext) {
    let host = Rc::new(Recorder::default());
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(host.clone(), cx));
    let today = today.to_vec();
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "Space 1".into(), None, 0, "".into(), "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(1, Vec::new(), today, Vec::new(), Folders::default(), cx);
    });
    cx.simulate_resize(size(px(1000.), px(700.)));
    cx.run_until_parked();
    (shell, host, cx)
}

fn press(shell: &Entity<Shell>, cx: &mut VisualTestContext, tab: u64) -> f32 {
    shell.update(cx, |shell, _| {
        let y = row_y(shell, tab);
        shell.press_row(tab, at(X, y));
        y
    })
}

fn drag_to(shell: &Entity<Shell>, cx: &mut VisualTestContext, x: f32, y: f32) {
    shell.update(cx, |shell, cx| shell.drag_to(at(x, y), cx));
}

/// The drag-out calls the browser was asked, in order.
fn outs(host: &Recorder) -> Vec<String> {
    host.take().into_iter().filter(|call| call.contains(" out ")).collect()
}

/// Past the window's left edge the row is out: its line goes and the browser hears where the
/// pointer is, and let go there, the browser takes the tab, which this window doesn't move.
#[gpui::test]
fn a_row_past_the_edge_goes_to_the_browser(cx: &mut TestAppContext) {
    let (shell, host, cx) = window(cx, &[1, 2, 3]);
    let y = press(&shell, cx, 1);
    drag_to(&shell, cx, X, y + 60.);
    assert!(shell.read_with(cx, |shell, _| shell.drag.as_ref().unwrap().line.is_some()));
    drag_to(&shell, cx, -20., y + 60.);
    assert!(shell.read_with(cx, |shell, _| shell.drag.as_ref().unwrap().line.is_none()));
    assert_eq!(outs(&host), [format!("drag out 1 -20 {}", y + 60.)]);
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert_eq!(outs(&host), [format!("drop out 1 -20 {}", y + 60.)]);
    assert_eq!(host.moves.get(), 0);
    assert!(shell.read_with(cx, |shell, _| shell.drag.is_none()));
}

/// Back in, the drag goes on in the list, and the window it was over hears it left; escape out
/// there puts it back the same way, and nothing drops.
#[gpui::test]
fn back_in_or_put_back_it_leaves_the_other_window(cx: &mut TestAppContext) {
    let (shell, host, cx) = window(cx, &[1, 2, 3]);
    let y = press(&shell, cx, 1);
    drag_to(&shell, cx, X, 720.);
    drag_to(&shell, cx, X, y + 60.);
    assert_eq!(outs(&host), [format!("drag out 1 {X} 720"), "drag out 1 none".to_string()]);
    assert!(shell.read_with(cx, |shell, _| shell.drag.as_ref().unwrap().line.is_some()));
    drag_to(&shell, cx, 1010., y);
    shell.update(cx, |shell, cx| shell.run(Command::Stop, "ui", cx));
    assert_eq!(outs(&host), [format!("drag out 1 1010 {y}"), "drag out 1 none".to_string()]);
    assert!(shell.read_with(cx, |shell, _| shell.drag.is_none()));
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert_eq!(outs(&host), Vec::<String>::new());
    assert_eq!(host.moves.get(), 0);
}

/// In full screen the window covers its display, so a row stays in the list at its edge.
/// ⎋ taken before any view has it, as from a page in Safari's engine, puts a dragged-out row back,
/// and with no drag going it leaves the key alone.
#[gpui::test]
fn escape_before_any_view_puts_a_dragged_row_back(cx: &mut TestAppContext) {
    let (shell, host, cx) = window(cx, &[1, 2, 3]);
    assert!(!shell.update(cx, |shell, cx| shell.escape_drag(cx)), "it took ⎋ with no drag");
    let y = press(&shell, cx, 1);
    drag_to(&shell, cx, X, y + 60.);
    drag_to(&shell, cx, -20., y + 60.);
    host.take();
    assert!(shell.update(cx, |shell, cx| shell.escape_drag(cx)));
    let calls = host.take();
    assert!(calls.contains(&"drag out 1 none".to_string()), "the drag wasn't put back: {calls:?}");
    assert!(shell.read_with(cx, |shell, _| shell.drag.is_none()));
    assert!(!shell.update(cx, |shell, cx| shell.escape_drag(cx)), "it took a second ⎋");
}

#[gpui::test]
fn a_full_screen_window_keeps_its_rows(cx: &mut TestAppContext) {
    let (shell, host, cx) = window(cx, &[1, 2, 3]);
    shell.update(cx, |shell, cx| shell.set_full_screen(true, cx));
    let y = press(&shell, cx, 1);
    drag_to(&shell, cx, -20., y + 60.);
    assert_eq!(outs(&host), Vec::<String>::new());
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert_eq!(host.moves.get(), 1);
}

/// Another window's tab over the list gets a line where it would drop, and let go there, the
/// place, counted as the browser counts; one this list has lands at its own place, at once.
#[gpui::test]
fn another_windows_tab_drops_where_the_line_marks(cx: &mut TestAppContext) {
    let (shell, _, cx) = window(cx, &[1, 2, 3]);
    let y = shell.read_with(cx, |shell, _| row_y(shell, 2));
    shell.update(cx, |shell, cx| assert!(shell.drag_tab_in(9, at(X, y), cx)));
    assert!(shell.read_with(cx, |shell, _| shell.drag.as_ref().unwrap().line.is_some()));
    let place = shell.update(cx, |shell, cx| shell.drop_tab_in(9, at(X, y), cx));
    assert_eq!(place, Some((false, 0, 1)));
    // Off the list, it's taken nowhere.
    shell.update(cx, |shell, cx| shell.drag_away(cx));
    shell.update(cx, |shell, _| assert!(shell.end_drag(false)));
    let place = shell.update(cx, |shell, cx| shell.drop_tab_in(9, at(700., y), cx));
    assert_eq!(place, None);
    let place = shell.update(cx, |shell, cx| shell.drop_tab_in(2, at(X, y), cx));
    assert_eq!(place, Some((false, 0, 1)));
    assert!(shell.read_with(cx, |shell, _| shell.drag.is_none()));
}

/// Over the page away from the split targets, let go, the tab opens in a window of its own that
/// grows out of its picture, centred on the pointer; over a target it joins the split instead.
#[gpui::test]
fn away_from_the_targets_it_opens_a_window_from_its_picture(cx: &mut TestAppContext) {
    let (shell, host, cx) = window(cx, &[1, 2, 3]);
    shell.update(cx, |shell, cx| shell.set_selected(2, cx));
    let card = shell.read_with(cx, |shell, _| shell.page_frame(shell.viewport).0);
    let (middle, left) = (card.center(), card.left() + card.size.width * 0.1 + px(30.));
    let (x, y) = (f32::from(middle.x), f32::from(middle.y));
    let y_row = press(&shell, cx, 1);
    drag_to(&shell, cx, X, y_row + 60.);
    drag_to(&shell, cx, x, y);
    host.take();
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert_eq!(host.take(), [format!("open window 1 {} {} 209 109.5 0.2", x - 104.5, y - 54.75)]);
    assert_eq!(host.take_edits(), Vec::<String>::new());

    press(&shell, cx, 1);
    drag_to(&shell, cx, X, y_row + 60.);
    drag_to(&shell, cx, f32::from(left), y);
    shell.update(cx, |shell, cx| shell.drop_row(cx));
    assert!(host.take().iter().all(|call| !call.starts_with("open window")));
    assert_eq!(host.take_edits(), ["split 1 2 true"]);
}
