//! The Site Controls' keys, clicks and hovers, on a host that records what the panel asks of its
//! window and the core.

use std::cell::{Cell, RefCell};

use gpui::{Modifiers, TestAppContext, VisualTestContext};

use super::*;

#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<String>>,
    /// The shape the panel last gave its window.
    corners: RefCell<Vec<(f32, f32, f32)>>,
    /// The tooltips asked for, apart from the calls, as each close takes one away.
    tips: RefCell<Vec<String>>,
    /// The index the next menu answers with.
    answer: Cell<Option<usize>>,
}

impl Recorder {
    fn push(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }
}

impl Host for Recorder {
    fn show(&self, at: Point<Pixels>, size: Size<Pixels>, corners: &[(f32, f32, f32)]) {
        let (x, y) = (f32::from(at.x), f32::from(at.y));
        self.push(format!("show {x} {y} {} {}", f32::from(size.width), f32::from(size.height)));
        self.corners.replace(corners.to_vec());
    }
    fn set_height(&self, height: f32, corners: &[(f32, f32, f32)]) {
        self.push(format!("height {height}"));
        self.corners.replace(corners.to_vec());
    }
    // A frame of the motion, which tells nothing of what the user did.
    fn drew(&self, _: f32, _: f32, _: Point<Pixels>) {}
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
    fn set_system_engine(&self, tab: u64, on: bool) {
        self.push(format!("system engine {tab} {on}"));
    }
    fn run(&self, run: Run, tab: u64, _: &mut App) {
        let run = match run {
            Run::Command(command) => format!("{command:?}"),
            Run::Share(frame) => {
                let (x, y) = (f32::from(frame.origin.x), f32::from(frame.origin.y));
                let (width, height) = (f32::from(frame.size.width), f32::from(frame.size.height));
                format!("share {x} {y} {width} {height}")
            }
            Run::SiteSettings => "site settings".into(),
            Run::Extension(id) => format!("extension {id}"),
            Run::Certificate => "certificate".into(),
        };
        self.push(format!("run {run} {tab}"));
    }
    fn menu(&self, items: &[MenuEntry], _: Point<Pixels>) -> Option<usize> {
        let items: Vec<String> = (items.iter())
            .map(|item| format!("{}{}", item.title, if item.checked { "*" } else { "" }))
            .collect();
        self.push(format!("menu {}", items.join(", ")));
        self.answer.get()
    }
    fn tooltip(&self, tip: Option<(SharedString, tooltip::Anchor)>, _: &mut App) {
        self.tips.borrow_mut().push(match tip {
            Some((text, anchor)) => format!("{text} at {} {}", anchor.x, anchor.bottom),
            None => "none".into(),
        });
    }
}

const TAB: u64 = 7;
/// Window management, in Chrome's ContentSettingsType.
const WINDOWS: i32 = 36;

fn permission(kind: i32, name: &'static str, value: Value, choices: &[Value]) -> Permission {
    let state = if value == Value::Allow { "Allowed" } else { "Ask" };
    Permission {
        kind,
        key: "".into(),
        name: name.into(),
        value,
        state: state.into(),
        choices: choices.to_vec(),
    }
}

/// A site that asks for where the user is, may manage windows, and runs JavaScript as a policy
/// says, which the user can't change.
fn site() -> Site {
    Site {
        site: "example.com".into(),
        connection: Connection::Secure,
        connection_summary: "Connection is secure".into(),
        certificate: true,
        permissions: vec![
            permission(5, "Location", Value::Ask, &Value::ALL),
            permission(WINDOWS, "Window management", Value::Allow, &Value::ALL),
            permission(2, "JavaScript", Value::Allow, &[]),
        ],
        reset: Some("Reset permissions".into()),
        system_engine: None,
    }
}

fn extension() -> Extension {
    Extension { id: "abc".into(), title: "Notes".into(), icon: None }
}

fn bounds(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
}

/// The URL field and its Site Controls button, as the shell lays them out in a 228 pt sidebar.
fn under_button() -> Place {
    Place::Button { field: bounds(8., 46., 212., 36.), button: bounds(191., 49., 30., 30.) }
}

fn window() -> Size<Pixels> {
    size(px(1200.), px(800.))
}

/// The panel opened from its button, once the core has read the site.
struct Panel {
    cx: VisualTestContext,
    host: Rc<Recorder>,
}

impl Panel {
    fn new(cx: &mut TestAppContext, site: Site) -> Self {
        Panel::at(cx, under_button(), window(), site)
    }

    fn at(cx: &mut TestAppContext, place: Place, window: Size<Pixels>, site: Site) -> Self {
        let host = Rc::new(Recorder::default());
        let panel = cx.update(|cx| open(host.clone(), cx));
        let panel = panel.expect("no window for the Site Controls");
        let cx = VisualTestContext::from_window(panel.into(), cx);
        // As tall as the host makes it for the tallest panel here, so every part takes a click.
        cx.simulate_resize(size(px(WIDTH), px(400.)));
        let mut panel = Panel { cx, host };
        panel.show(place, window);
        assert_eq!(panel.calls(), [format!("read {TAB}")]);
        panel.changed(site);
        panel
    }

    /// What the button's press does.
    fn show(&mut self, place: Place, window: Size<Pixels>) {
        self.cx.cx.update(|cx| show(TAB, place, window, vec![extension()], cx));
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

    fn click(&mut self, (x, y): (f32, f32)) {
        self.cx.simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    fn hover(&mut self, (x, y): (f32, f32)) {
        self.cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::none());
        self.cx.run_until_parked();
    }

    /// What the panel asked since the last time, once it has had time to leave.
    fn calls(&mut self) -> Vec<String> {
        self.cx.executor().advance_clock(Duration::from_secs_f32(CLOSE));
        self.cx.run_until_parked();
        self.host.calls.take()
    }

    /// What a click closing the panel to run `run` asks.
    fn ran(&mut self, run: &str) -> Vec<String> {
        let calls = self.calls();
        assert_eq!(calls, ["release", "stop", &format!("run {run} {TAB}"), "hide"]);
        calls
    }
}

// The panel's places in its window, which is 238 pt wide under a beak 12.5 pt tall: the Share tile
// 11 pt in at y 23.5 and the capture tiles 56 pt apart after it, the extensions' tiles from y
// 92.5, the Settings heading's cap at y 142.5, the rows 43 pt apart from y 167.5, and the footer at
// y 354.5.
const SHARE: (f32, f32) = (35., 39.5);
const CAPTURE: (f32, f32) = (91., 39.5);
const CAPTURE_FULL_PAGE: (f32, f32) = (147., 39.5);
const EXTENSION: (f32, f32) = (35., 108.5);
const ADD: (f32, f32) = (91., 108.5);
const EXTENSIONS_HEADING: (f32, f32) = (40., 74.);
const MANAGE: (f32, f32) = (215., 74.);
const SETTINGS_HEADING: (f32, f32) = (40., 147.);
const MORE_LINK: (f32, f32) = (220., 147.);
fn row(index: usize) -> (f32, f32) {
    (100., 167.5 + index as f32 * ROW_PITCH + 16.)
}
const LOCK: (f32, f32) = (30., 370.5);
const MORE: (f32, f32) = (211., 370.5);

/// Centred under its button at x 206, its top 9 pt under the URL field's bottom, which its beak
/// reaches over.
#[gpui::test]
fn it_hangs_from_its_button_once_the_site_is_read(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    assert_eq!(panel.calls(), ["show 87 78.5 238 397.5"]);
    let corners = panel.host.corners.take();
    assert_eq!(corners.first(), Some(&(119., 0., BEAK_TIP_RADIUS)));
    assert_eq!(corners.len(), 7);
}

/// It stays inside the window, and its beak at its button as far as the body's corner allows.
#[gpui::test]
fn it_keeps_inside_the_window(cx: &mut TestAppContext) {
    let field = bounds(8., 46., 284., 36.);
    let place = Place::Button { field, button: bounds(265., 49., 30., 30.) };
    let mut panel = Panel::at(cx, place, size(px(300.), px(800.)), site());
    assert_eq!(panel.calls(), ["show 54 78.5 238 397.5"]);
    assert_eq!(panel.host.corners.take()[0].0, WIDTH - (RADIUS + BEAK + BEAK_FOOT_RADIUS));
}

/// With the sidebar hidden it opens at the page's corner, without a beak.
#[gpui::test]
fn in_the_corner_it_has_no_beak(cx: &mut TestAppContext) {
    let place = Place::Corner(point(px(18.), px(8.)));
    let mut panel = Panel::at(cx, place, window(), site());
    assert_eq!(panel.calls(), ["show 18 8 238 385"]);
    assert_eq!(panel.host.corners.take().len(), 4);
}

/// Measured on Arc's: with two rows it's 299 pt tall, under its beak.
#[gpui::test]
fn two_rows_make_arcs_height(_: &mut TestAppContext) {
    let two = Site { permissions: site().permissions[..2].to_vec(), reset: None, ..site() };
    let at = layout(&two, 0, false);
    let found = (at.tiles, at.extensions_cap, at.extensions, at.settings_cap, at.rows);
    assert_eq!(found, (11., 57., 80., Some(130.), 155.));
    assert_eq!((at.divider, at.footer, at.height), (245., 256., 299.));
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
    panel.show(under_button(), window());
    assert_eq!(panel.calls(), ["release", "stop", "hide"]);
    panel.show(under_button(), window());
    panel.changed(site());
    assert_eq!(panel.calls(), [format!("read {TAB}"), "show 87 78.5 238 397.5".into()]);
}

/// Share closes the panel first, then the share menu hangs from it, in the browser window.
#[gpui::test]
fn share_hangs_its_menu_from_its_tile(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(SHARE);
    panel.ran("share 98 102 48 32");
}

/// The capture tiles close the panel first, so the picture has none of it, then ask for what
/// Capture's commands do.
#[gpui::test]
fn the_capture_tile_closes_the_panel_and_starts_a_capture(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(CAPTURE);
    panel.ran("CapturePortion");
}

#[gpui::test]
fn the_full_page_tile_closes_the_panel_and_captures_the_page(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(CAPTURE_FULL_PAGE);
    panel.ran("CaptureFullPage");
}

#[gpui::test]
fn a_tile_names_itself_in_a_tooltip(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.hover(SHARE);
    panel.hover(EXTENSION);
    panel.hover((120., 60.));
    // Under each tile's middle in the browser window, and Share's leave, after Notes came, leaves
    // Notes'.
    let tips = panel.host.tips.take();
    assert_eq!(tips, ["Share at 122 134", "Notes at 122 203", "none"]);
}

#[gpui::test]
fn an_extension_runs_as_its_toolbar_button(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(EXTENSION);
    panel.ran("extension abc");
}

/// The browser has no add path of its own, so + opens Chrome's extensions page, which links the
/// store.
#[gpui::test]
fn the_add_tile_opens_the_extensions_page(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(ADD);
    panel.ran("ManageExtensions");
}

/// "Manage" shows on the Extensions heading under the pointer, and opens the extensions page.
#[gpui::test]
fn manage_shows_on_the_extensions_heading(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(MANAGE);
    assert!(panel.calls().is_empty());
    panel.hover(EXTENSIONS_HEADING);
    panel.click(MANAGE);
    panel.ran("ManageExtensions");
}

/// "More" shows on the Settings heading under the pointer, and opens Chrome's settings for the
/// site.
#[gpui::test]
fn more_shows_on_the_settings_heading(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(MORE_LINK);
    assert!(panel.calls().is_empty());
    panel.hover(SETTINGS_HEADING);
    // Not the other heading's.
    panel.click(MANAGE);
    assert!(panel.calls().is_empty());
    panel.hover(SETTINGS_HEADING);
    panel.click(MORE_LINK);
    panel.ran("site settings");
}

#[gpui::test]
fn a_value_chosen_sets_the_permission(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(row(1));
    let set = format!("set {TAB} {WINDOWS} Block");
    assert_eq!(panel.calls(), ["menu Allow*, Block, Ask", &set]);
    // The value it has sets nothing, nor does a menu closed without a choice.
    panel.host.answer.set(Some(0));
    panel.click(row(1));
    panel.host.answer.set(None);
    panel.click(row(1));
    assert_eq!(panel.calls(), ["menu Allow*, Block, Ask", "menu Allow*, Block, Ask"]);
}

/// A site kept for Safari's engine lists it first, and its menu's Off turns it off; On, which it
/// is, does nothing.
#[gpui::test]
fn the_engine_row_turns_safaris_engine_off(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, Site { system_engine: Some(true), ..site() });
    panel.calls();
    panel.host.answer.set(Some(0));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On*, Off"]);
    panel.host.answer.set(Some(1));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On*, Off", &format!("system engine {TAB} false")]);
    // The permissions follow it, a row down.
    panel.host.answer.set(Some(1));
    panel.click(row(2));
    assert_eq!(panel.calls(), ["menu Allow*, Block, Ask", &format!("set {TAB} {WINDOWS} Block")]);
}

/// A site the core offers Safari's engine for, as a page asked for what it plays, or the user closed
/// the site's note or turned it off, lists it off, and its menu's On turns it on, as the note's button does; Off,
/// which it is, does nothing. The core then reports the site kept, and the row says On.
#[gpui::test]
fn the_engine_row_offers_safaris_engine(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, Site { system_engine: Some(false), ..site() });
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On, Off*"]);
    panel.host.answer.set(Some(0));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On, Off*", &format!("system engine {TAB} true")]);
    panel.changed(Site { system_engine: Some(true), ..site() });
    panel.calls();
    panel.host.answer.set(None);
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On*, Off"]);
}

/// A site turned off stays offered, as the core reports it after Off, so On brings it back.
#[gpui::test]
fn after_off_the_engine_row_offers_on(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, Site { system_engine: Some(true), ..site() });
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On*, Off", &format!("system engine {TAB} false")]);
    panel.changed(Site { system_engine: Some(false), ..site() });
    panel.calls();
    panel.host.answer.set(Some(0));
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu On, Off*", &format!("system engine {TAB} true")]);
}

/// A site the core neither keeps nor offers Safari's engine for has no row for it: its first row is
/// the first permission's.
#[gpui::test]
fn without_the_offer_there_is_no_engine_row(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.host.answer.set(None);
    panel.click(row(0));
    assert_eq!(panel.calls(), ["menu Allow, Block, Ask*"]);
}

/// A value a policy set shows, but isn't the user's to change.
#[gpui::test]
fn a_managed_value_has_no_menu(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.host.answer.set(Some(1));
    panel.click(row(2));
    assert!(panel.calls().is_empty());
}

#[gpui::test]
fn reset_sets_the_permissions_back(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(row(3));
    assert_eq!(panel.calls(), [format!("reset {TAB}")]);
}

/// Arc's "…" menu without Boost's items, each the browser's own command.
#[gpui::test]
fn the_more_menu_runs_what_it_lists(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    let menu = "menu Clear Cache, Clear Cookies, , Manage Extensions…, Add Extension…, , \
                All Site Settings…";
    for (index, run) in [
        (0, "ClearCacheAndRefresh"),
        (1, "ClearCookiesAndRefresh"),
        (3, "ManageExtensions"),
        (4, "ManageExtensions"),
        (6, "site settings"),
    ] {
        panel.host.answer.set(Some(index));
        panel.click(MORE);
        assert_eq!(panel.calls(), [menu, "release", "stop", &format!("run {run} {TAB}"), "hide"]);
        panel.show(under_button(), window());
        panel.changed(site());
        panel.calls();
    }
    // Closed without a choice, it leaves the panel be.
    panel.host.answer.set(None);
    panel.click(MORE);
    assert_eq!(panel.calls(), [menu]);
}

#[gpui::test]
fn the_lock_shows_the_certificate(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.click(LOCK);
    panel.ran("certificate");
}

/// A page with no certificate, as an http one, has nothing to show.
#[gpui::test]
fn without_a_certificate_the_lock_does_nothing(cx: &mut TestAppContext) {
    let plain = Site { connection: Connection::NotSecure, certificate: false, ..site() };
    let mut panel = Panel::new(cx, plain.clone());
    panel.calls();
    panel.click(LOCK);
    assert!(panel.calls().is_empty());
    assert_eq!(lock_label(&plain), "Not secure");
}

/// A change the core reports, such as a reset that leaves nothing to reset, resizes the panel.
#[gpui::test]
fn a_change_resizes_it(cx: &mut TestAppContext) {
    let mut panel = Panel::new(cx, site());
    panel.calls();
    panel.changed(Site { reset: None, ..site() });
    assert_eq!(panel.calls(), ["height 354.5"]);
    assert_eq!(panel.host.corners.take()[3], (WIDTH, 354.5, RADIUS));
    // A report for another tab isn't this panel's.
    panel.cx.cx.update(|cx| changed(TAB + 1, site(), cx));
    assert!(panel.calls().is_empty());
}

/// With no permissions to list, the Settings heading goes, and the line follows the extensions.
#[gpui::test]
fn without_permissions_there_is_no_settings_heading(cx: &mut TestAppContext) {
    let bare = Site { permissions: Vec::new(), reset: None, ..site() };
    let mut panel = Panel::new(cx, bare);
    assert_eq!(panel.calls(), ["show 87 78.5 238 193.5"]);
}

#[gpui::test]
fn the_log_names_each_permission_and_its_value(_: &mut TestAppContext) {
    let described = describe(&site());
    let permissions = "Location Ask, Window management Allow, JavaScript Allow";
    assert_eq!(described, format!("example.com: Connection is secure; {permissions}"));
    let bare = Site { permissions: Vec::new(), reset: None, ..site() };
    assert_eq!(describe(&bare), "example.com: Connection is secure; no permissions");
}

#[gpui::test]
fn site_settings_are_for_the_pages_origin(_: &mut TestAppContext) {
    let settings = "chrome://settings/content/siteDetails?site=";
    let url = site_settings_url("https://me:secret@example.com:8443/path?q=1#top");
    assert_eq!(url, Some(format!("{settings}https%3A%2F%2Fexample.com%3A8443")));
    let url = site_settings_url("http://localhost/");
    assert_eq!(url, Some(format!("{settings}http%3A%2F%2Flocalhost")));
    assert_eq!(site_settings_url("chrome://settings"), None);
    assert_eq!(site_settings_url("file:///Users/me/a.html"), None);
}

#[gpui::test]
fn each_permission_has_its_glyph(_: &mut TestAppContext) {
    assert_eq!(glyph_for("geolocation"), "icons/map-pin.svg");
    assert_eq!(glyph_for("media-stream-camera"), "icons/video.svg");
    assert_eq!(glyph_for("window-placement"), "icons/app-window.svg");
    // A type Chrome adds later.
    assert_eq!(glyph_for("something-new"), "icons/settings.svg");
}
