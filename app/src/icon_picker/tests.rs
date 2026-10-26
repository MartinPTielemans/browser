//! The icon picker's tabs, clicks and keys, on a host that records what the picker asks of its
//! window and what it picks.

use std::{cell::RefCell, rc::Rc};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

use super::*;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Recorder {
    fn record(&self, call: String) {
        self.0.borrow_mut().push(call);
    }
}

impl Host for Recorder {
    fn show(&self, left: f32, top: f32) {
        self.record(format!("show {left} {top}"));
    }

    fn hide(&self) {
        self.record("hide".into());
    }

    fn show_character_palette(&self, _: &mut Window) {
        self.record("palette".into());
    }
}

/// The picker, shown for Space 7, which has `icon`, its arrow at (23, 66).
struct Picker {
    cx: VisualTestContext,
    window: WindowHandle<IconPicker>,
    host: Rc<Recorder>,
}

impl Picker {
    fn new(icon: &str, cx: &mut TestAppContext) -> Self {
        let host = Rc::new(Recorder::default());
        let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the picker");
        let recorded = host.clone();
        let picked: Picked = Rc::new(move |icon, _| recorded.record(format!("picked {icon:?}")));
        let icon = SharedString::from(icon.to_string());
        cx.update(|cx| show(7, icon, point(23., 66.), 0x3d81f9, picked, cx));
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        Picker { cx, window, host }
    }

    fn tab(&mut self) -> Option<Tab> {
        let picking = self.window.read_with(&self.cx.cx, |picker, _| picker.picking()).unwrap();
        picking.map(|(_, tab)| tab)
    }

    fn click(&mut self, x: f32, y: f32) {
        self.cx.simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// A click on the glyph `index` places from the first, row by row.
    fn click_glyph(&mut self, index: usize) {
        let (row, column) = (index / COLUMNS, index % COLUMNS);
        let x = GRID_X + PITCH * (column as f32 + 0.5);
        let y = ARROW + GRID_TOP + PITCH * (row as f32 + 0.5);
        self.click(x, y);
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    /// What the picker asked of the host since it showed.
    fn calls(&mut self) -> Vec<String> {
        self.host.0.borrow().iter().skip(1).cloned().collect()
    }
}

#[test]
fn it_opens_on_the_emoji_tab_for_an_emoji_and_the_icon_tab_otherwise() {
    assert_eq!(tab_for("🦊"), Tab::Emoji);
    assert_eq!(tab_for("glyph:star"), Tab::Icon);
    assert_eq!(tab_for(""), Tab::Icon);
    assert_eq!(tab_for("glyph:unicorn"), Tab::Icon);
}

#[gpui::test]
fn it_hangs_its_card_under_the_icon_with_the_arrow_at_it(cx: &mut TestAppContext) {
    let mut picker = Picker::new("🦊", cx);
    assert_eq!(picker.tab(), Some(Tab::Emoji));
    // The arrow's tip is 13 in from the window's left, and the card 25 under the icon's middle.
    assert_eq!(picker.host.0.borrow()[0], "show 10 78");
}

#[gpui::test]
fn a_glyph_click_gives_the_space_that_glyph_and_closes(cx: &mut TestAppContext) {
    let mut picker = Picker::new("", cx);
    assert_eq!(picker.tab(), Some(Tab::Icon));
    picker.click_glyph(9);
    assert_eq!(picker.calls(), ["hide", "picked \"glyph:shapes\""]);
    assert_eq!(picker.tab(), None);
}

#[gpui::test]
fn the_glyph_the_space_has_changes_nothing(cx: &mut TestAppContext) {
    let mut picker = Picker::new("glyph:star", cx);
    picker.click_glyph(0);
    assert_eq!(picker.calls(), ["hide"]);
}

#[gpui::test]
fn the_trash_takes_the_icon_away(cx: &mut TestAppContext) {
    let mut picker = Picker::new("glyph:moon", cx);
    // The trash is the 24 pt button 7 after Emoji | Icon, the two centred under the arrow.
    let left = (WIDTH - (2. * SEGMENT_WIDTH + TRASH_GAP + SEGMENTS_HEIGHT)) / 2.;
    let x = left + 2. * SEGMENT_WIDTH + TRASH_GAP + SEGMENTS_HEIGHT / 2.;
    picker.click(x, ARROW + SEGMENTS_TOP + SEGMENTS_HEIGHT / 2.);
    assert_eq!(picker.calls(), ["hide", "picked \"\""]);
}

#[gpui::test]
fn escape_closes_it_with_no_change(cx: &mut TestAppContext) {
    let mut picker = Picker::new("glyph:moon", cx);
    picker.keys("escape");
    assert_eq!(picker.calls(), ["hide"]);
    assert_eq!(picker.tab(), None);

    // The Emoji tab's field has the palette to pick from.
    let mut picker = Picker::new("🦊", cx);
    picker.keys("escape");
    assert_eq!(picker.calls(), ["palette", "hide"]);
}

#[gpui::test]
fn an_emoji_typed_or_picked_is_the_icon_at_once(cx: &mut TestAppContext) {
    let mut picker = Picker::new("", cx);
    // The Emoji segment, the first half of the control.
    let left = (WIDTH - (2. * SEGMENT_WIDTH + TRASH_GAP + SEGMENTS_HEIGHT)) / 2.;
    picker.click(left + SEGMENT_WIDTH / 2., ARROW + SEGMENTS_TOP + SEGMENTS_HEIGHT / 2.);
    assert_eq!(picker.tab(), Some(Tab::Emoji));
    picker.cx.simulate_input("🌱");
    picker.cx.run_until_parked();
    assert_eq!(picker.calls(), ["palette", "hide", "picked \"🌱\""]);
}

#[test]
fn icons_come_in_from_the_right_and_emoji_from_the_left() {
    let near = |(old, new): (f32, f32), (x, y): (f32, f32)| {
        assert!((old - x).abs() < 1e-3 && (new - y).abs() < 1e-3, "{old} {new}, not {x} {y}");
    };
    near(slid(Tab::Emoji, Tab::Icon, 0.), (0., WIDTH));
    let (old, new) = slid(Tab::Emoji, Tab::Icon, 0.5);
    assert!(old < 0. && new > 0. && (new - old - WIDTH).abs() < 1e-3, "{old} {new}");
    near(slid(Tab::Emoji, Tab::Icon, 1.), (-WIDTH, 0.));
    near(slid(Tab::Icon, Tab::Emoji, 0.), (0., -WIDTH));
    near(slid(Tab::Icon, Tab::Emoji, 1.), (WIDTH, 0.));
}
