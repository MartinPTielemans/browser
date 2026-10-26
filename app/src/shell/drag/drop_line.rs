//! A dragged row over the list: the rows stay put and a line marks where it would go.

use gpui::TestAppContext;

use super::detents::{X, drag_to, press, row_y, shell_with};
use crate::shell::{Folders, NAME_HEIGHT, ROW_HEIGHT, ROW_PITCH, Shell, Slot};

/// Where the drop line is, down the list, or None.
fn line(shell: &gpui::Entity<Shell>, cx: &mut TestAppContext) -> Option<f32> {
    shell.read_with(cx, |shell, _| shell.drag.as_ref().and_then(|drag| drag.line))
}

/// Halfway between the plates of the row at `top` and the one after it.
fn under(top: f32) -> f32 {
    top + ROW_PITCH - (ROW_PITCH - ROW_HEIGHT) / 2.
}

/// Dragged down past two rows and back up, no other row moves from its place, and the line is in
/// the gap the row would drop into.
#[gpui::test]
fn the_rows_stay_put_and_the_line_marks_the_gap(cx: &mut TestAppContext) {
    let (shell, _) = shell_with(cx, &[], &[1, 2, 3, 4], Folders::default());
    let layout = shell.read_with(cx, |shell, _| shell.layout());
    let y = press(&shell, cx, 1);
    let still = |shell: &gpui::Entity<Shell>, cx: &mut TestAppContext| {
        shell.read_with(cx, |shell, _| {
            let others = [Slot::Tab(2), Slot::Tab(3), Slot::Tab(4), Slot::NewTab];
            shell.slides.is_empty() && others.iter().all(|&slot| shell.slide(slot) == 0.)
        })
    };
    // Where it was, there's no line, since it would drop where it is.
    drag_to(&shell, cx, X, y + 8.);
    assert_eq!(line(&shell, cx), None);
    // Past 2 and 3, it would go under 3.
    drag_to(&shell, cx, X, y + 2. * ROW_PITCH);
    assert!(still(&shell, cx), "a row moved during the drag");
    assert_eq!(line(&shell, cx), Some(under(layout[&Slot::Tab(3)])));
    // Back up past 3, under 2.
    drag_to(&shell, cx, X, y + ROW_PITCH);
    assert!(still(&shell, cx), "a row moved during the drag");
    assert_eq!(line(&shell, cx), Some(under(layout[&Slot::Tab(2)])));
}

/// Into the pinned tabs, the line goes over the first pinned row when it would be first, and under
/// New Tab when it would be Today's first.
#[gpui::test]
fn the_line_marks_the_ends_of_the_lists(cx: &mut TestAppContext) {
    let (shell, _) = shell_with(cx, &[1, 2], &[3, 4], Folders::default());
    let layout = shell.read_with(cx, |shell, _| shell.layout());
    press(&shell, cx, 4);
    let y = shell.read_with(cx, |shell, _| row_y(shell, 1));
    drag_to(&shell, cx, X, y);
    assert_eq!(line(&shell, cx), Some(NAME_HEIGHT - (ROW_PITCH - ROW_HEIGHT) / 2.));
    let y = shell.read_with(cx, |shell, _| row_y(shell, 3));
    drag_to(&shell, cx, X, y);
    assert_eq!(line(&shell, cx), Some(under(layout[&Slot::NewTab])));
}
