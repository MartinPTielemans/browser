//! The Site Controls' keys and clicks, on a host that records what the panel asks of its window
//! and the core.

use std::cell::{Cell, RefCell};

use gpui::{Modifiers, TestAppContext, VisualTestContext, hsla};

use super::*;

#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<String>>,
    /// The index the next menu answers with.
    answer: Cell<Option<usize>>,
}

impl Recorder {
    fn push(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }
}

impl Host for Recorder {
    fn show(&self, at: Point<Pixels>, size: Size<Pixels>) {
        let (x, y) = (f32::from(at.x), f32::from(at.y));
        self.push(format!("show {x} {y} {} {}", f32::from(size.width), f32::from(size.height)));
    }
    fn set_height(&self, height: f32) {
        self.push(format!("height {height}"));
    }
    // A frame of the motion, which tells nothing of what the user did.
    fn drew(&self) {}
    fn release(&self) {
        self.push("release".into());
    }
    fn hide(&self) {
        self.push("hide".into());
    }
    fn read(&self, tab: u64) {
        self.push(format!("read {tab}"));
    }
    fn stop_reading(&self) {
        self.push("stop".into());
    }
    fn set_permission(&self, tab: u64, kind: i32, value: Value) {
        self.push(format!("set {tab} {kind} {value:?}"));
    }
    fn reset_permissions(&self, tab: u64) {
        self.push(format!("reset {tab}"));
    }
    fn run(&self, tile: Tile, tab: u64, _: &mut App) {
        self.push(format!("run {tile:?} {tab}"));
    }
    fn menu(&self, items: &[MenuEntry], _: Point<Pixels>) -> Option<usize> {
        let items: Vec<String> = (items.iter())
            .map(|item| format!("{}{}", item.title, if item.checked { "*" } else { "" }))
            .collect();
        self.push(format!("menu {}", items.join(", ")));
        self.answer.get()
    }
}

const TAB: u64 = 7;
/// Window management, in Chrome's ContentSettingsType.
const WINDOWS: i32 = 36;

fn permission(kind: i32, name: &'static str, value: Value, choices: &[Value]) -> Permission {
    Permission { kind, name: name.into(), value, choices: choices.to_vec() }
}

/// A site that asks for where the user is, may manage windows, and runs JavaScript as a policy
/// says, which the user can't change.
fn site() -> Site {
    Site {
        site: "example.com".into(),
        connection: Connection::Secure,
        connection_summary: "Connection is secure".into(),
        permissions: vec![
            permission(5, "Location", Value::Ask, &Value::ALL),
            permission(WINDOWS, "Window management", Value::Allow, &Value::ALL),
            permission(2, "JavaScript", Value::Allow, &[]),
        ],
        reset: Some("Reset permissions".into()),
        picture_in_picture: PictureInPicture::None,
    }
}

/// The panel under the URL field, opened from its button, once the core has read the site.
struct Panel {
    cx: VisualTestContext,
    host: Rc<Recorder>,
}

impl Panel {
    fn new(cx: &mut TestAppContext, site: Site) -> Self {
        let host = Rc::new(Recorder::default());
        let window = cx.update(|cx| open(host.clone(), |_| hsla(0., 0., 0.2, 1.), cx));
        let window = window.expect("no window for the Site Controls");
        let cx = VisualTestContext::from_window(window.into(), cx);
        let mut panel = Panel { cx, host };
        panel.show();
        assert_eq!(panel.calls(), [format!("read {TAB}")]);
        panel.changed(site);
        panel
    }

    /// What the button's press does.
    fn show(&mut self) {
        let button = Bounds::new(point(px(191.), px(49.)), size(px(30.), px(30.)));
        self.cx.cx.update(|cx| show(TAB, point(px(8.), px(88.)), button, cx));
        self.cx.run_until_parked();
    }

    fn changed(&mut self, site: Site) {
        self.cx.cx.update(|cx| changed(TAB, site, cx));
        self.cx.run_until_parked();
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    fn click(&mut self, x: f32, y: f32) {
        self.cx.simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// What the panel asked since the last time, once it has had time to leave.
    fn calls(&mut self) -> Vec<String> {
        self.cx.executor().advance_clock(Duration::from_secs_f32(CLOSE));
        self.cx.run_until_parked();
        self.host.calls.take()
    }
}

/// The panel's places, as it lays out at 280 pt: the two tiles side by side, then a row a
/// permission from y 83, then the reset button.
const FIND: (f32, f32) = (205., 42.);
const VALUE_X: f32 = 260.;
fn row_y(index: usize) -> f32 {
    83. + (index as f32 + 0.5) * ROW
}
const RESET: (f32, f32) = (20., 199.);

#[gpui::test]
fn it_shows_where_asked_once_the_site_is_read(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    assert_eq!(panel.calls(), ["show 8 88 280 267"]);
}

#[gpui::test]
fn escape_closes_it(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.keys("escape");
    // The browser window gets the keyboard back at once, and the window goes once it has left.
    assert_eq!(panel.host.calls.borrow().as_slice(), ["release", "stop"]);
    assert_eq!(panel.calls(), ["release", "stop", "hide"]);
}

#[gpui::test]
fn a_click_outside_closes_it(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.cx.cx.update(|cx| dismissed(cx));
    assert_eq!(panel.calls(), ["release", "stop", "hide"]);
}

/// The button's second press closes the panel, and a third opens it again.
#[gpui::test]
fn its_button_toggles_it(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.show();
    assert_eq!(panel.calls(), ["release", "stop", "hide"]);
    panel.show();
    panel.changed(site());
    assert_eq!(panel.calls(), [format!("read {TAB}"), "show 8 88 280 267".into()]);
}

#[gpui::test]
fn a_value_chosen_sets_the_permission(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(VALUE_X, row_y(1));
    let set = format!("set {TAB} {WINDOWS} Block");
    assert_eq!(panel.calls(), ["menu Allow*, Block, Ask", &set]);
    // The value it has sets nothing, nor does a menu closed without a choice.
    panel.host.answer.set(Some(0));
    panel.click(VALUE_X, row_y(1));
    panel.host.answer.set(None);
    panel.click(VALUE_X, row_y(1));
    assert_eq!(panel.calls(), ["menu Allow*, Block, Ask", "menu Allow*, Block, Ask"]);
}

/// A value a policy set shows, but isn't the user's to change.
#[gpui::test]
fn a_managed_value_has_no_menu(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(VALUE_X, row_y(2));
    assert!(panel.calls().is_empty());
}

#[gpui::test]
fn reset_sets_the_permissions_back(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(RESET.0, RESET.1);
    assert_eq!(panel.calls(), [format!("reset {TAB}")]);
}

/// A tile closes the panel before it runs, so what it opens, such as the find bar, gets the
/// keyboard from the browser window.
#[gpui::test]
fn a_tile_runs_once_the_panel_lets_go(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(FIND.0, FIND.1);
    assert_eq!(panel.calls(), ["release", "stop", &format!("run Find {TAB}"), "hide"]);
}

/// A change the core reports, such as a reset that leaves nothing to reset, resizes the panel.
#[gpui::test]
fn a_change_resizes_it(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.changed(Site { reset: None, ..site() });
    assert_eq!(panel.calls(), ["height 233"]);
    // A report for another tab isn't this panel's.
    panel.cx.cx.update(|cx| changed(TAB + 1, site(), cx));
    assert!(panel.calls().is_empty());
}

#[gpui::test]
fn picture_in_picture_shows_for_a_playing_video(cx: &mut TestAppContext) {
    let playing = Site { picture_in_picture: PictureInPicture::Enter, ..site() };
    let mut panel = Panel::new(cx, playing);
    panel.calls();
    // Three tiles across, the third at the right.
    panel.click(230., FIND.1);
    let run = format!("run PictureInPicture {TAB}");
    assert_eq!(panel.calls(), ["release", "stop", &run, "hide"]);
}

#[gpui::test]
fn the_log_names_each_permission_and_its_value(_: &mut TestAppContext) {
    let described = describe(&site());
    let permissions = "Location Ask, Window management Allow, JavaScript Allow";
    assert_eq!(described, format!("example.com: Connection is secure; {permissions}"));
    let bare = Site { permissions: Vec::new(), reset: None, ..site() };
    assert_eq!(describe(&bare), "example.com: Connection is secure; no permissions");
}
