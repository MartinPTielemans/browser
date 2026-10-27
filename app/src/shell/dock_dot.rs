//! Which Space's color the Dock icon's dot takes, with two windows, on a host that records what
//! the dot is asked to show.

use std::{cell::RefCell, rc::Rc};

use gpui::{AppContext, Entity, TestAppContext};

use crate::dock_dot::{self, QUIET};
use crate::shell::{Shell, recorder::Recorder};

const PURPLE: u32 = 0x8655BF;
const RED: u32 = 0xE0475B;

#[derive(Default)]
struct Dot(RefCell<Vec<Option<u32>>>);

impl dock_dot::Host for Dot {
    fn show(&self, color: Option<u32>) {
        self.0.borrow_mut().push(color);
    }
}

/// A window, the key one as it's the newest, with Space 1 in `color` and Space 2 in none,
/// showing `space`.
fn window(cx: &mut TestAppContext, color: u32, space: u64) -> Entity<Shell> {
    let shell = cx.new(|cx| Shell::new(Rc::new(Recorder::default()), cx));
    shell.update(cx, |shell, cx| {
        let spaces = vec![
            (1, "Space 1".into(), Some(color), 0, "".into(), "".into()),
            (2, "Space 2".into(), None, 0, "".into(), "".into()),
        ];
        shell.set_spaces(spaces, cx);
        shell.set_space(space, cx);
    });
    shell
}

fn shown(cx: &mut TestAppContext, dot: &Dot) -> Vec<Option<u32>> {
    cx.executor().advance_clock(QUIET);
    cx.run_until_parked();
    dot.0.take()
}

#[gpui::test]
fn the_dot_follows_the_key_windows_space(cx: &mut TestAppContext) {
    let dot = Rc::new(Dot::default());
    cx.update(|cx| dock_dot::open(dot.clone(), cx));
    let first = window(cx, PURPLE, 1);
    assert_eq!(shown(cx, &dot), [Some(PURPLE)]);
    window(cx, PURPLE, 2);
    assert_eq!(shown(cx, &dot), [None]);

    // A window that isn't key switches without the dot.
    first.update(cx, |shell, cx| shell.set_space(2, cx));
    first.update(cx, |shell, cx| shell.set_space(1, cx));
    assert!(shown(cx, &dot).is_empty());

    first.update(cx, |shell, cx| shell.became_key(cx));
    assert_eq!(shown(cx, &dot), [Some(PURPLE)]);

    // A new color for the Space shown recolors it, and a switch back to a green Space too.
    first.update(cx, |shell, cx| {
        let spaces = vec![
            (1, "Space 1".into(), Some(RED), 0, "".into(), "".into()),
            (2, "Space 2".into(), None, 0, "".into(), "".into()),
        ];
        shell.set_spaces(spaces, cx);
    });
    assert_eq!(shown(cx, &dot), [Some(RED)]);
    first.update(cx, |shell, cx| shell.set_space(2, cx));
    assert_eq!(shown(cx, &dot), [None]);
}
