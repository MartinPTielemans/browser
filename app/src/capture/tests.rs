//! Capture mode's keys and clicks, on a host that records what it asks of its window, the core
//! and the system.

use std::{cell::RefCell, io::Cursor};

use gpui::{Modifiers, TestAppContext, VisualTestContext};

use super::*;

const TAB: u64 = 7;

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Recorder {
    fn record(&self, call: String) {
        self.0.borrow_mut().push(call);
    }
}

impl Host for Recorder {
    fn show(&self, frame: Bounds<Pixels>) {
        self.record(format!("show {}", describe(frame)));
    }

    fn hide(&self) {
        self.record("hide".into());
    }

    fn camera(&self, camera: bool) {
        self.record(format!("camera {camera}"));
    }

    fn element_at(&self, tab: u64, at: Point<Pixels>) {
        self.record(format!("element at {tab} {} {}", f32::from(at.x), f32::from(at.y)));
    }

    fn capture(&self, tab: u64, request: u64, full_page: bool, rect: Option<Bounds<Pixels>>) {
        let what = match (full_page, rect) {
            (true, _) => "full page".to_string(),
            (false, Some(rect)) => describe(rect),
            (false, None) => "view".to_string(),
        };
        self.record(format!("capture {tab} {request} {what}"));
    }

    fn end(&self, tab: u64) {
        self.record(format!("end {tab}"));
    }

    fn copy(&self, png: &[u8]) {
        self.record(format!("copy {}", png.len()));
    }

    fn save(&self, png: &[u8]) {
        self.record(format!("save {}", png.len()));
    }

    fn share(&self, png: &[u8], share: Share) {
        self.record(format!("share {} {share:?}", png.len()));
    }
}

/// A picture as the core sends one.
fn png() -> Vec<u8> {
    let mut png = Vec::new();
    RgbaImage::new(8, 6).write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    png
}

/// Capture mode's window over a page that fills it.
struct Mode {
    cx: VisualTestContext,
    host: Rc<Recorder>,
}

impl Mode {
    fn new(cx: &mut TestAppContext) -> Self {
        let host = Rc::new(Recorder::default());
        let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for Capture");
        let cx = VisualTestContext::from_window(window.into(), cx);
        Mode { cx, host }
    }

    /// What ⇧⌘2 does: the shell's `start`.
    fn start(&mut self) {
        let frame = Bounds::new(point(px(0.), px(0.)), size(px(800.), px(600.)));
        self.cx.cx.update(|cx| start(TAB, frame, 8., cx));
        self.cx.run_until_parked();
    }

    fn captured(&mut self, request: u64) {
        self.cx.cx.update(|cx| captured(TAB, request, Some(png()), cx));
        self.cx.run_until_parked();
    }

    /// ⇧⌘2, and the picture of the page the window shows with.
    fn shown(cx: &mut TestAppContext) -> Self {
        let mut mode = Mode::new(cx);
        mode.start();
        mode.captured(1);
        assert_eq!(mode.calls(), [format!("capture {TAB} 1 view"), "show 0 0 800 600".into()]);
        mode
    }

    /// The pointer over an element, and the box the core answers with.
    fn hover(&mut self, x: f32, y: f32, element: Bounds<Pixels>) {
        let at = point(px(x), px(y));
        self.cx.simulate_mouse_move(at, None, Modifiers::none());
        self.cx.run_until_parked();
        assert_eq!(self.calls(), [format!("element at {TAB} {x} {y}")]);
        self.cx.cx.update(|cx| element_box(TAB, at, Some(element), cx));
        self.cx.run_until_parked();
    }

    fn click(&mut self, x: f32, y: f32) {
        self.cx.simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    fn calls(&self) -> Vec<String> {
        self.host.0.take()
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
}

/// A row's middle in the menu, from the menu's corner: the field, then 40 a row and 9 a line.
fn row(action: Action) -> f32 {
    let mut top = FIELD_TOP + FIELD_HEIGHT + ROWS_TOP;
    for row in ROWS {
        match row {
            Some(row) if *row == action => return top + ROW / 2.,
            Some(_) => top += ROW,
            None => top += SEPARATOR,
        }
    }
    panic!("no row for {action:?}");
}

#[gpui::test]
fn escape_leaves_and_shift_cmd_2_comes_back(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.keys("escape");
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}")]);
    // Esc again has nothing to leave.
    mode.keys("escape");
    assert!(mode.calls().is_empty());
    mode.start();
    assert_eq!(mode.calls(), [format!("capture {TAB} 2 view")]);
}

#[gpui::test]
fn the_second_shift_cmd_2_leaves(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.start();
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}")]);
    // Also before the picture of the page has come, which then has nowhere to show.
    mode.start();
    mode.start();
    mode.captured(2);
    let left = ["hide".to_string(), format!("end {TAB}")];
    assert_eq!(mode.calls(), [&[format!("capture {TAB} 2 view")], &left[..]].concat());
}

#[gpui::test]
fn a_click_takes_the_element_under_the_pointer(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.hover(100., 100., rect(50., 60., 200., 80.));
    mode.click(100., 100.);
    assert_eq!(mode.calls(), ["camera false"]);
    // The menu is 10 in from the ring's left, level with its top; Copy copies the element's box.
    let menu = point(50. - RING + MENU_LEFT, 60. - RING);
    mode.click(menu.x + MENU_WIDTH / 2., menu.y + row(Action::Copy));
    assert_eq!(mode.calls(), [format!("capture {TAB} 2 50 60 200 80")]);
    mode.captured(2);
    let len = png().len();
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}"), format!("copy {len}")]);
}

#[gpui::test]
fn a_drag_takes_the_rectangle_dragged(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.hover(100., 100., rect(50., 60., 200., 80.));
    let (left, button) = (Modifiers::none(), MouseButton::Left);
    mode.cx.simulate_mouse_down(point(px(100.), px(100.)), button, left);
    // Under 4 pt it's still a click; past it, a drag, which asks the core nothing as it goes.
    mode.cx.simulate_mouse_move(point(px(102.), px(101.)), button, left);
    mode.cx.simulate_mouse_move(point(px(300.), px(250.)), button, left);
    mode.cx.simulate_mouse_up(point(px(300.), px(250.)), button, left);
    mode.cx.run_until_parked();
    assert_eq!(mode.calls(), ["camera false"]);
    let menu = point(100. - RING + MENU_LEFT, 100. - RING);
    mode.click(menu.x + MENU_WIDTH / 2., menu.y + row(Action::Save));
    assert_eq!(mode.calls(), [format!("capture {TAB} 2 100 100 200 150")]);
    mode.captured(2);
    let len = png().len();
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}"), format!("save {len}")]);
}

#[gpui::test]
fn a_press_that_barely_moves_is_a_click(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.hover(100., 100., rect(50., 60., 200., 80.));
    let (left, button) = (Modifiers::none(), MouseButton::Left);
    mode.cx.simulate_mouse_down(point(px(100.), px(100.)), button, left);
    mode.cx.simulate_mouse_move(point(px(102.), px(101.)), button, left);
    mode.cx.simulate_mouse_up(point(px(102.), px(101.)), button, left);
    mode.cx.run_until_parked();
    assert_eq!(mode.calls(), ["camera false"]);
    let menu = point(50. - RING + MENU_LEFT, 60. - RING);
    mode.click(menu.x + MENU_WIDTH / 2., menu.y + row(Action::Copy));
    assert_eq!(mode.calls(), [format!("capture {TAB} 2 50 60 200 80")]);
}

#[gpui::test]
fn retake_chooses_again(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.hover(100., 100., rect(50., 60., 200., 80.));
    mode.click(100., 100.);
    assert_eq!(mode.calls(), ["camera false"]);
    let menu = point(50. - RING + MENU_LEFT, 60. - RING);
    mode.click(menu.x + MENU_WIDTH / 2., menu.y + row(Action::Retake));
    // No picture is taken: the pointer is the camera again, and asks about what it's over.
    assert_eq!(mode.calls(), ["camera true"]);
    mode.hover(400., 300., rect(380., 280., 100., 50.));
    mode.click(400., 300.);
    assert_eq!(mode.calls(), ["camera false"]);
    let menu = point(380. - RING + MENU_LEFT, 280. - RING);
    mode.click(menu.x + MENU_WIDTH / 2., menu.y + row(Action::Copy));
    assert_eq!(mode.calls(), [format!("capture {TAB} 2 380 280 100 50")]);
}

#[gpui::test]
fn a_click_away_from_the_menu_leaves(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.hover(100., 100., rect(50., 60., 200., 80.));
    mode.click(100., 100.);
    assert_eq!(mode.calls(), ["camera false"]);
    mode.click(700., 500.);
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}")]);
}

#[gpui::test]
fn a_click_over_nothing_chooses_nothing(cx: &mut TestAppContext) {
    let mut mode = Mode::shown(cx);
    mode.click(100., 100.);
    assert!(mode.calls().is_empty());
    mode.keys("escape");
    assert_eq!(mode.calls(), ["hide".to_string(), format!("end {TAB}")]);
}
