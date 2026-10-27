//! The Site Controls: the panel the URL field's Site Controls button opens, as Arc's Site
//! Control Center (docs/ux-spec.md › Extensions › Site Controls). For the site of the page shown:
//! the action tiles, the extensions that act on it, whether ads and trackers are blocked there and
//! its permissions as Chrome's page info lists them, each changeable, and its connection. GPUI can't draw over the page's native view, so
//! it's a window of its own that the host shows over the browser window, as its child, with the
//! keyboard, as the command bar is, over a blur in its shape with a shadow. What it asks of its
//! window and the core goes through `Host`.

use std::{
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, ClickEvent, Context, FocusHandle, Global, Image, KeyBinding, PathBuilder, Pixels,
    Point, SharedString, Size, Task, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, canvas, div, img,
    linear_color_stop, linear_gradient, point, prelude::*, px, rgba, size, svg,
};

use crate::{
    icon_picker,
    shell::{Command, MenuEntry, cubic_bezier, scaled::scaled},
    tooltip,
};

actions!(site_controls, [Close]);

// Arc 1.170.0 cap, in a dark Space, 1 pt a pixel, except where a comment says ours.
/// The body's width: a row of four tiles inside the padding.
pub const WIDTH: f32 = 4. * TILE_WIDTH + 3. * GAP + 2. * PADDING;
const PADDING: f32 = 11.;
const RADIUS: f32 = 17.;
const GAP: f32 = 8.;
/// The beak's sharp tip is this far over the body, its sides at 45°, rounded at the tip and where
/// they meet the top edge: 10.5 pt shows.
const BEAK: f32 = 12.5;
const BEAK_TIP_RADIUS: f32 = 5.;
const BEAK_FOOT_RADIUS: f32 = 3.;
/// The body's top under the URL field's bottom edge, which puts the beak's tip 1.5 pt into it.
const BELOW_FIELD: f32 = 9.;
/// Ours: how close to the browser window's edges the body comes.
const WINDOW_MARGIN: f32 = 8.;
/// The see-through fill over the host's blur, fitted over the sidebar and over the page.
const FILL: u32 = 0x7073874d;
/// A dark line around it, and a light one inside that.
const OUTLINE: u32 = 0x000000d9;
const RIM: u32 = 0xffffff1a;
const TEXT: u32 = 0xffffffcc;
const SECONDARY: u32 = 0xffffff73;
/// "Manage" and "More", which the headings show under the pointer.
const LINK: u32 = 0xffffff7a;
const TILE_WIDTH: f32 = 48.;
const TILE_HEIGHT: f32 = 32.;
const TILE_RADIUS: f32 = 8.;
/// An action tile is opaque, lighter at its top, with a light line along its top inside.
const TILE_TOP: u32 = 0x282828ff;
const TILE_BOTTOM: u32 = 0x161616ff;
const TILE_HIGHLIGHT: u32 = 0xffffff33;
/// The + tile's, an extension's and the lock button's plate.
const PLATE: u32 = 0xffffff0d;
/// Ours: a tile or the "…" button under the pointer, and a plate.
const HOVERED: u32 = 0xffffff0d;
const PLATE_HOVERED: u32 = 0xffffff1a;
/// Ours: an extension's icon, as the toolbar's is.
const EXTENSION_ICON: f32 = 16.;
const PLUS_GLYPH: f32 = 16.5;
const HEADING_SIZE: f32 = 14.;
const LINK_SIZE: f32 = 12.;
const NAME_SIZE: f32 = 14.;
const STATE_SIZE: f32 = 13.;
const CIRCLE: f32 = 32.;
const ROW_PITCH: f32 = 43.;
/// The text's left edge, past the circle.
const ROW_TEXT: f32 = PADDING + CIRCLE + 8.;
/// A permission's circle when it isn't allowed and when it is, with its glyph darker, as if cut
/// out of it.
const OFF: u32 = 0xffffff4d;
const ON: u32 = 0xffffffeb;
const OFF_GLYPH: u32 = 0x00000080;
const ON_GLYPH: u32 = 0x000000a8;
const ROW_GLYPH: f32 = 17.;
/// Ours: the reset row's glyph, as Chrome's page info's button has none.
const RESET: &str = "icons/rotate-ccw-clock.svg";
/// Ours: the row of a site the profile keeps for Safari's engine, first, as the URL field's mark
/// for it. Named for Safari, which people know, though the page plays here, in ilo's tab.
const ENGINE: &str = "icons/compass.svg";
const ENGINE_NAME: &str = "Safari's engine";

/// Ours: the ad blocking row's, which Arc's panel didn't have.
const AD_BLOCKING: &str = "icons/shield.svg";
/// Ours: a policy's row, which the user can't change.
const MANAGED_OPACITY: f32 = 0.5;
const DIVIDER: u32 = 0xffffff2b;
const FOOTER: f32 = 32.;
const LOCK_RADIUS: f32 = 6.;
const LOCK_LEFT: f32 = 10.;
const LOCK_SLOT: f32 = 10.;
const LOCK_GLYPH: f32 = 15.;
const LOCK_TEXT_GAP: f32 = 6.;
const LOCK_RIGHT: f32 = 8.;
/// The lock's words' cap top in its plate.
const LOCK_CAP: f32 = 11.5;
/// Ours: the lock button with no certificate to show.
const DISABLED_OPACITY: f32 = 0.5;
const MORE_FILL: u32 = 0x0000000a;
const MORE_GLYPH: f32 = 17.5;
// Down the body, each from the top of what's above it.
const HEADING_CAP: f32 = TILE_HEIGHT + 14.;
const HEADING_TO_TILES: f32 = 23.;
const TILES_TO_HEADING: f32 = TILE_HEIGHT + 18.;
const HEADING_TO_ROWS: f32 = 25.;
const ROWS_TO_DIVIDER: f32 = CIRCLE + 15.;
const DIVIDER_TO_FOOTER: f32 = 11.;
/// Ours: the extensions' rows are as far apart as their tiles.
const EXTENSIONS_PER_ROW: usize = 4;
// SF's ascent, descent and cap height for its size, which put a line's cap where it was measured.
const ASCENT: f32 = 0.967;
const DESCENT: f32 = 0.211;
const CAP: f32 = 0.705;
const LINE: f32 = 20.;
/// Ours, as before: it opens in 0.15 s and closes in 0.1 s, growing from the beak's tip and
/// shrinking back into it, from and to this much of its size, as it fades.
const OPEN: f32 = 0.15;
const CLOSE: f32 = 0.1;
const FROM_SCALE: f32 = 0.9;

/// A value of a site's permission, in Chrome's page info's words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Value {
    Allow,
    Block,
    Ask,
}

impl Value {
    pub const ALL: [Value; 3] = [Value::Allow, Value::Block, Value::Ask];

    fn label(self) -> &'static str {
        match self {
            Value::Allow => "Allow",
            Value::Block => "Block",
            Value::Ask => "Ask",
        }
    }
}

/// A permission of the site, as Chrome's page info lists it.
#[derive(Clone, PartialEq, Debug)]
pub struct Permission {
    /// Chrome's ContentSettingsType for it, which the core takes back.
    pub kind: i32,
    /// Chrome's lasting name for it, such as "geolocation", which picks its glyph.
    pub key: SharedString,
    /// Chrome's name for it, such as "Location".
    pub name: SharedString,
    pub value: Value,
    /// Chrome's words for its value, such as "Allowed".
    pub state: SharedString,
    /// What the user may set it to, none for one that isn't theirs to change, as a policy's isn't.
    pub choices: Vec<Value>,
}

/// What the connection to the page is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Connection {
    Secure,
    NotSecure,
    /// A file, or one of the browser's own pages.
    Internal,
}

/// The site of a tab's page, as the core reports it.
#[derive(Clone, PartialEq, Debug)]
pub struct Site {
    /// As the user should see it, such as "example.com".
    pub site: SharedString,
    pub connection: Connection,
    /// Chrome's words for the connection, such as "Connection is secure".
    pub connection_summary: SharedString,
    /// The page came with a certificate, which the lock button shows.
    pub certificate: bool,
    pub permissions: Vec<Permission>,
    /// Chrome's words for its button that sets the permissions back to their defaults, when it
    /// shows one.
    pub reset: Option<SharedString>,
    /// Its row for Safari's engine, which plays the protected video Chromium can't: on for a site
    /// the profile keeps for it, off for one it offers it for, where a page of the tab asked or the
    /// user closed the site's note, else none. The row turns it on or off.
    pub system_engine: Option<bool>,
    /// Whether ads and trackers are blocked on the site, none where that isn't the user's to change
    /// there: a page that isn't http(s), one Safari's engine shows, a private window, or with
    /// blocking off in Settings.
    pub ad_blocking: Option<bool>,
}

/// An extension acting on the tab's page, as the toolbar shows it.
#[derive(Clone)]
pub struct Extension {
    pub id: SharedString,
    pub title: SharedString,
    pub icon: Option<Arc<Image>>,
}

/// What the panel hangs from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Place {
    /// Its button in the URL field: the beak at the button's middle, the body under the field.
    Button { field: Bounds<Pixels>, button: Bounds<Pixels> },
    /// With the sidebar hidden, the page's top left corner, without a beak.
    Corner(Point<Pixels>),
}

/// What a click in the panel asks the host to do, once the panel has let go.
#[derive(Clone, PartialEq, Debug)]
pub enum Run {
    /// A shell command on the tab shown.
    Command(Command),
    /// The system's share menu for the page, hanging from this, in the browser window.
    Share(Bounds<Pixels>),
    /// Chrome's settings for the site.
    SiteSettings,
    /// The extension's action, as a click on its toolbar button.
    Extension(SharedString),
    /// The certificate the page came with, in the system's viewer.
    Certificate,
}

/// An action tile. Arc's row is Share, Boost, Capture a portion and Capture the full page; the
/// ones the browser doesn't have aren't here, and one is a line in TILES. Boost comes with Boosts.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tile {
    /// For the logs.
    pub name: &'static str,
    /// Its tooltip.
    pub label: &'static str,
    pub glyph: &'static str,
    /// The glyph's size, for its ink's measured size.
    pub glyph_size: f32,
    pub run: TileRun,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TileRun {
    Share,
    Command(Command),
}

/// Arc's order, without Boost. The captures' glyphs are Lucide's frame and page, *interim*: Arc's
/// aren't recorded.
pub const TILES: &[Tile] = &[
    Tile {
        name: "share",
        label: "Share",
        glyph: "icons/share.svg",
        glyph_size: 19.5,
        run: TileRun::Share,
    },
    Tile {
        name: "capture",
        label: "Capture a Portion of This Page",
        glyph: "icons/maximize.svg",
        glyph_size: 19.5,
        run: TileRun::Command(Command::CapturePortion),
    },
    Tile {
        name: "capture-full-page",
        label: "Capture Full Page",
        glyph: "icons/file.svg",
        glyph_size: 19.5,
        run: TileRun::Command(Command::CaptureFullPage),
    },
];

/// The Lucide glyph for a permission, by Chrome's name for its type.
pub fn glyph_for(key: &str) -> &'static str {
    match key {
        "geolocation" => "icons/map-pin.svg",
        "notifications" => "icons/bell.svg",
        "media-stream-camera" | "camera-pan-tilt-zoom" => "icons/video.svg",
        "media-stream-mic" => "icons/mic.svg",
        "javascript" => "icons/code.svg",
        "javascript-jit" | "javascript-optimizer" => "icons/zap.svg",
        "popups" => "icons/external-link.svg",
        "images" => "icons/image.svg",
        "cookies" | "storage-access" => "icons/cookie.svg",
        "automatic-downloads" => "icons/download.svg",
        "sound" => "icons/volume-2.svg",
        "clipboard" => "icons/clipboard.svg",
        "sensors" => "icons/activity.svg",
        "midi-sysex" => "icons/piano.svg",
        "usb-guard" => "icons/usb.svg",
        "serial-guard" => "icons/cable.svg",
        "hid-guard" => "icons/gamepad-2.svg",
        "bluetooth-guard" | "bluetooth-scanning" => "icons/bluetooth.svg",
        "nfc-devices" => "icons/nfc.svg",
        "window-placement" => "icons/app-window.svg",
        "local-fonts" => "icons/type.svg",
        "hand-tracking" => "icons/hand.svg",
        "ar" | "vr" => "icons/glasses.svg",
        "auto-picture-in-picture" => "icons/picture-in-picture-2.svg",
        "automatic-fullscreen" => "icons/maximize.svg",
        "keyboard-lock" => "icons/keyboard.svg",
        "pointer-lock" => "icons/mouse-pointer-2.svg",
        "payment-handler" => "icons/credit-card.svg",
        "web-printing" => "icons/printer.svg",
        "speaker-selection" => "icons/speaker.svg",
        "local-network-access" | "local-network" | "loopback-network" | "direct-sockets" => {
            "icons/network.svg"
        }
        "subresource-filter" => "icons/megaphone.svg",
        "file-system-write-guard" => "icons/file.svg",
        "durable-storage" => "icons/database.svg",
        "idle-detection" => "icons/clock.svg",
        _ => "icons/settings.svg",
    }
}

/// Chrome's settings page for the site of `url`, an http or https page's origin.
pub fn site_settings_url(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit_once('@').map_or(authority, |(_, host)| host);
    if host.is_empty() {
        return None;
    }
    let origin = format!("{}://{host}", scheme.to_ascii_lowercase());
    let site: String = (origin.bytes())
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect();
    Some(format!("chrome://settings/content/siteDetails?site={site}"))
}

/// What the panel asks of its window and the core.
pub trait Host {
    /// Shows the panel's window over the browser window, as it draws now, with the keyboard:
    /// `size`, its top left at `at` in the window's content, in the shape of `corners` (x, y and
    /// radius from its top left, clockwise), which its blur and shadow take.
    fn show(&self, at: Point<Pixels>, size: Size<Pixels>, corners: &[(f32, f32, f32)]);
    /// Makes it `height` tall, keeping its top edge, in its new shape.
    fn set_height(&self, height: f32, corners: &[(f32, f32, f32)]);
    /// It drew a frame of its motion, at `scale` about `origin` in its window and `opacity`,
    /// which its blur and shadow follow.
    fn drew(&self, scale: f32, opacity: f32, origin: Point<Pixels>);
    /// It stops taking the keyboard and clicks, which the browser window gets back, as it leaves.
    fn release(&self);
    /// Hides its window, once it has left.
    fn hide(&self);
    /// Reads the site of `tab`'s page, which comes in `changed` and again as it changes, until
    /// `stop_reading`.
    fn read(&self, tab: u64);
    fn stop_reading(&self);
    /// Gives the site the permission of `kind` at `value`. The user's: agents get none.
    fn set_permission(&self, tab: u64, kind: i32, value: Value);
    /// Sets the site's permissions back to their defaults. The user's, as above.
    fn reset_permissions(&self, tab: u64);
    /// Has the profile keep the site for Safari's engine or not, and the tab show its page there or
    /// in Chromium. The user's, as above.
    fn set_system_engine(&self, tab: u64, on: bool);
    /// Blocks ads and trackers on the site or lets them load, and reloads the page. The user's, as
    /// above.
    fn set_ad_blocking(&self, tab: u64, on: bool);
    fn run(&self, run: Run, tab: u64, cx: &mut App);
    /// Shows a menu of `items` at `position` in the panel's window, and returns the index of the
    /// one chosen once it closes. AppKit runs it in an event loop of its own, so it's called
    /// outside GPUI's updates.
    fn menu(&self, items: &[MenuEntry], position: Point<Pixels>) -> Option<usize>;
    /// Shows the app's tooltip, hanging from a tile in the browser window, or takes it away.
    fn tooltip(&self, tip: Option<(SharedString, tooltip::Anchor)>, cx: &mut App);
}

/// A heading whose link shows under the pointer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Heading {
    Extensions,
    Settings,
}

/// The panel for one tab, from the press that opens it until it has left.
struct Open {
    tab: u64,
    /// Its window's top left in the browser window.
    at: Point<Pixels>,
    /// The beak's tip across its top, none in the corner.
    beak: Option<f32>,
    /// The browser window's size, which tooltips stay inside.
    window: Size<Pixels>,
    extensions: Vec<Extension>,
    /// None until the core first reports, which is when it shows.
    site: Option<Site>,
    height: f32,
    hovered: Option<Heading>,
    /// The tile whose tooltip shows, as the pointer enters the next before it leaves the last.
    tip: Option<SharedString>,
    /// When its window was first on screen, where the motion starts.
    shown: Option<Instant>,
    settled: bool,
    /// Once it's closing: when, and how far it had opened.
    closing: Option<(Instant, f32)>,
    _leave: Option<Task<()>>,
}

impl Open {
    /// How far it has opened, from 0 to 1.
    fn progress(&self) -> f32 {
        match self.closing {
            Some((at, from)) => {
                let t = (at.elapsed().as_secs_f32() / CLOSE).min(1.);
                from * (1. - cubic_bezier(0.42, 0., 1., 1., t))
            }
            None => {
                let opened = self.shown.map_or(0., |shown| shown.elapsed().as_secs_f32());
                cubic_bezier(0., 0., 0.58, 1., (opened / OPEN).min(1.))
            }
        }
    }

    fn moving(&self) -> bool {
        self.closing.is_some()
            || self.shown.is_none_or(|shown| shown.elapsed().as_secs_f32() < OPEN)
    }

    /// Where it grows from in its window: the beak's tip, else its top left.
    fn origin(&self) -> Point<Pixels> {
        point(px(self.beak.unwrap_or(0.)), px(0.))
    }

    /// `frame`, in the panel's window, in the browser window.
    fn in_window(&self, frame: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds::new(self.at + frame.origin, frame.size)
    }
}

/// Where each part of the panel goes, down its window.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Layout {
    /// The body's top: under the beak, if it has one.
    body: f32,
    tiles: f32,
    extensions_cap: f32,
    extensions: f32,
    /// The Settings heading's cap and the first row's top, if there are rows.
    settings_cap: Option<f32>,
    rows: f32,
    divider: f32,
    footer: f32,
    height: f32,
}

/// The panel's layout for `site` with `extensions` tiles, under a beak or not.
fn layout(site: &Site, extensions: usize, beak: bool) -> Layout {
    let body = if beak { BEAK } else { 0. };
    let tiles = body + PADDING;
    let extensions_cap = tiles + HEADING_CAP;
    let grid = extensions_cap + HEADING_TO_TILES;
    let lines = (extensions + 1).div_ceil(EXTENSIONS_PER_ROW) as f32;
    let grid_bottom = grid + lines * TILE_HEIGHT + (lines - 1.) * GAP;
    let rows = rows(site);
    let (settings_cap, first, divider) = match rows {
        // Ours: with nothing to list, the heading goes, and the line follows the tiles.
        0 => (None, grid_bottom, grid_bottom + ROWS_TO_DIVIDER - CIRCLE),
        rows => {
            let cap = grid_bottom - TILE_HEIGHT + TILES_TO_HEADING;
            let first = cap + HEADING_TO_ROWS;
            (Some(cap), first, first + (rows - 1) as f32 * ROW_PITCH + ROWS_TO_DIVIDER)
        }
    };
    let footer = divider + DIVIDER_TO_FOOTER;
    Layout {
        body,
        tiles,
        extensions_cap,
        extensions: grid,
        settings_cap,
        rows: first,
        divider,
        footer,
        height: footer + FOOTER + PADDING,
    }
}

/// The Settings heading's rows: ad blocking, the permissions, then Reset.
fn rows(site: &Site) -> usize {
    let firsts = site.system_engine.is_some() as usize + site.ad_blocking.is_some() as usize;
    firsts + site.permissions.len() + site.reset.is_some() as usize
}

/// The panel's corners in its window, clockwise from the beak's tip, or its top left without one.
fn corners(beak: Option<f32>, height: f32) -> Vec<(f32, f32, f32)> {
    let Some(tip) = beak else {
        let corner = |x: f32, y: f32| (x, y, RADIUS);
        return vec![corner(0., 0.), corner(WIDTH, 0.), corner(WIDTH, height), corner(0., height)];
    };
    vec![
        (tip, 0., BEAK_TIP_RADIUS),
        (tip + BEAK, BEAK, BEAK_FOOT_RADIUS),
        (WIDTH, BEAK, RADIUS),
        (WIDTH, height, RADIUS),
        (0., height, RADIUS),
        (0., BEAK, RADIUS),
        (tip - BEAK, BEAK, BEAK_FOOT_RADIUS),
    ]
}

/// Where a line of text `size` tall goes for its cap's top to be at `cap`, in a LINE-tall box.
fn text_top(cap: f32, size: f32) -> f32 {
    cap - ((LINE - size * (ASCENT + DESCENT)) / 2. + size * (ASCENT - CAP))
}

/// The panel's window's top left in the browser window and its beak's tip, from `place`.
fn placed(place: Place, window: Size<Pixels>) -> (Point<Pixels>, Option<f32>) {
    match place {
        Place::Button { field, button } => {
            let middle = f32::from(button.center().x);
            let right = f32::from(window.width) - WINDOW_MARGIN - WIDTH;
            let left = (middle - WIDTH / 2.).min(right).max(WINDOW_MARGIN);
            // The beak stays clear of the body's rounded corners.
            let clear = RADIUS + BEAK + BEAK_FOOT_RADIUS;
            let tip = (middle - left).clamp(clear, WIDTH - clear);
            let top = f32::from(field.bottom()) + BELOW_FIELD - BEAK;
            (point(px(left), px(top)), Some(tip))
        }
        Place::Corner(corner) => (corner, None),
    }
}

pub struct SiteControls {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    open: Option<Open>,
}

struct SiteControlsWindow(WindowHandle<SiteControls>);

impl Global for SiteControlsWindow {}

/// Opens the panel's window, hidden, for `host` to show over the browser window, ahead of time as
/// the command bar's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<SiteControls>> {
    cx.bind_keys([KeyBinding::new("escape", Close, Some("SiteControls"))]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(WIDTH)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Floating,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The panel's shape is drawn, over the host's blur in it.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| SiteControls { host, focus: cx.focus_handle(), open: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(SiteControlsWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the Site Controls' window: {error}");
            None
        }
    }
}

fn update(
    cx: &mut App,
    change: impl FnOnce(&mut SiteControls, &mut Window, &mut Context<SiteControls>),
) {
    if let Some(window) = cx.try_global::<SiteControlsWindow>().map(|panel| panel.0) {
        window.update(cx, change).ok();
    }
}

/// Opens the panel for the site of `tab`'s page, hanging from `place` in a browser window of
/// `window`'s size, with the tab's `extensions`, once the core has read the site. If it shows,
/// closes it instead, as its button's second press does.
pub fn show(
    tab: u64,
    place: Place,
    window: Size<Pixels>,
    extensions: Vec<Extension>,
    cx: &mut App,
) {
    update(cx, |panel, _, cx| {
        if panel.open.as_ref().is_some_and(|open| open.closing.is_none()) {
            return panel.close("toggle", cx);
        }
        let (at, beak) = placed(place, window);
        panel.open = Some(Open {
            tab,
            at,
            beak,
            window,
            extensions,
            site: None,
            height: 0.,
            hovered: None,
            tip: None,
            shown: None,
            settled: false,
            closing: None,
            _leave: None,
        });
        panel.host.read(tab);
        cx.notify();
    });
}

/// The core read the site of `tab`'s page, or it changed.
pub fn changed(tab: u64, site: Site, cx: &mut App) {
    update(cx, |panel, window, cx| {
        let open = panel.open.as_mut();
        let Some(open) = open.filter(|open| open.tab == tab && open.closing.is_none()) else {
            return;
        };
        // check.sh reads this line.
        log!("browser: site controls {}", describe(&site));
        let height = layout(&site, open.extensions.len(), open.beak.is_some()).height;
        let first = open.site.is_none();
        open.site = Some(site);
        let shape = corners(open.beak, height);
        if first {
            open.height = height;
            window.focus(&panel.focus, cx);
            panel.host.show(open.at, size(px(WIDTH), px(height)), &shape);
        } else if height != open.height {
            open.height = height;
            panel.host.set_height(height, &shape);
        }
        cx.notify();
    });
}

/// The user left the panel: a click outside it, another window or another app. The host has
/// released it already.
pub fn dismissed(cx: &mut App) {
    update(cx, |panel, _, cx| panel.close("dismissed", cx));
}

/// The tab closed, and its panel with it.
pub fn tab_closed(tab: u64, cx: &mut App) {
    update(cx, |panel, _, cx| {
        if panel.open.as_ref().is_some_and(|open| open.tab == tab) {
            panel.close("tab closed", cx);
        }
    });
}

/// The site for the log: Chrome's words for the connection, then each row and its value, in their
/// order, which check.sh clicks by.
fn describe(site: &Site) -> String {
    let ad_blocking = (site.ad_blocking.iter())
        .map(|&on| format!("Ad blocking {}", if on { "On" } else { "Off" }));
    let permissions: Vec<String> = ad_blocking
        .chain(
            (site.permissions.iter())
                .map(|permission| format!("{} {}", permission.name, permission.value.label())),
        )
        .collect();
    format!(
        "{}: {}; {}{}",
        site.site,
        site.connection_summary,
        if permissions.is_empty() { "no permissions".into() } else { permissions.join(", ") },
        match site.system_engine {
            Some(on) => format!("; {ENGINE_NAME} {}", if on { "on" } else { "off" }),
            None => String::new(),
        }
    )
}

/// The lock button's words: Chrome's short ones for the connection, the page's own for a file or
/// one of the browser's pages, which Chrome gives none.
fn lock_label(site: &Site) -> SharedString {
    match site.connection {
        Connection::Secure => "Secure".into(),
        Connection::NotSecure => "Not secure".into(),
        Connection::Internal => site.site.clone(),
    }
}

/// The "…" menu: Arc's, without Boost's items.
fn more_items() -> Vec<(MenuEntry, Option<Run>)> {
    let item = |title: &str, run: Run| (MenuEntry::new(title.to_string(), true), Some(run));
    vec![
        item("Clear Cache", Run::Command(Command::ClearCacheAndRefresh)),
        item("Clear Cookies", Run::Command(Command::ClearCookiesAndRefresh)),
        (MenuEntry::line(), None),
        item("Manage Extensions…", Run::Command(Command::ManageExtensions)),
        // The browser has no add path of its own: Chrome's extensions page links its store.
        item("Add Extension…", Run::Command(Command::ManageExtensions)),
        (MenuEntry::line(), None),
        item("All Site Settings…", Run::SiteSettings),
    ]
}

impl SiteControls {
    /// Closes the panel back into its button, giving the browser window the keyboard and the
    /// clicks at once.
    fn close(&mut self, why: &str, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_mut().filter(|open| open.closing.is_none()) else {
            return;
        };
        open.closing = Some((Instant::now(), open.progress()));
        self.host.release();
        self.host.stop_reading();
        self.host.tooltip(None, cx);
        // check.sh reads this line.
        log!("browser: site controls closing ({why})");
        open._leave = Some(cx.spawn(async move |panel, cx| {
            cx.background_executor().timer(Duration::from_secs_f32(CLOSE)).await;
            panel
                .update(cx, |panel, cx| {
                    // Unless it opened again meanwhile.
                    if panel.open.as_ref().is_some_and(|open| open.closing.is_some()) {
                        panel.open = None;
                        panel.host.hide();
                        cx.notify();
                    }
                })
                .ok();
        }));
        cx.notify();
    }

    /// Closes the panel, then has the host do `run`, so what it opens gets the keyboard from the
    /// browser window.
    fn run(&mut self, name: &str, run: Run, cx: &mut Context<Self>) {
        let Some(tab) = self.open.as_ref().filter(|open| open.closing.is_none()).map(|o| o.tab)
        else {
            return;
        };
        // check.sh reads this line.
        log!("browser: site controls run {name}");
        self.close(name, cx);
        self.host.run(run, tab, cx);
    }

    fn run_tile(&mut self, tile: Tile, frame: Bounds<Pixels>, cx: &mut Context<Self>) {
        let Some(open) = self.open.as_ref() else {
            return;
        };
        let run = match tile.run {
            TileRun::Share => Run::Share(open.in_window(frame)),
            TileRun::Command(command) => Run::Command(command),
        };
        self.run(tile.name, run, cx);
    }

    /// Shows the permission's values in a menu at `position`, and sets the one chosen.
    fn choose_value(
        &mut self,
        permission: Permission,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.open.as_ref().map(|open| open.tab) else {
            return;
        };
        let items: Vec<MenuEntry> = (permission.choices.iter())
            .map(|&value| MenuEntry {
                checked: value == permission.value,
                ..MenuEntry::new(value.label(), true)
            })
            .collect();
        let host = self.host.clone();
        cx.spawn(async move |panel, cx| {
            let chosen = host.menu(&items, position);
            let Some(&value) = chosen.and_then(|index| permission.choices.get(index)) else {
                return;
            };
            panel
                .update(cx, |panel, _| {
                    // Unless the panel closed or went to another tab while the menu showed.
                    let open = panel.open.as_ref();
                    let still = open.is_some_and(|open| open.tab == tab && open.closing.is_none());
                    if still && value != permission.value {
                        // check.sh reads this line.
                        log!("browser: site controls set {} {}", permission.name, value.label());
                        panel.host.set_permission(tab, permission.kind, value);
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Shows the "…" menu at `position`, and runs the item chosen.
    fn more(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let (items, runs): (Vec<MenuEntry>, Vec<Option<Run>>) = more_items().into_iter().unzip();
        let host = self.host.clone();
        cx.spawn(async move |panel, cx| {
            let Some(index) = host.menu(&items, position) else {
                return;
            };
            let Some(Some(run)) = runs.get(index).cloned() else {
                return;
            };
            let name = items[index].title.to_string();
            panel.update(cx, |panel, cx| panel.run(&name, run, cx)).ok();
        })
        .detach();
    }

    /// Shows the engine row's menu at `position`: On and Off, the one it is checked, and the other
    /// turns it so.
    fn choose_engine(&mut self, on: bool, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(tab) = self.open.as_ref().map(|open| open.tab) else {
            return;
        };
        let items = [
            MenuEntry { checked: on, ..MenuEntry::new("On".to_string(), true) },
            MenuEntry { checked: !on, ..MenuEntry::new("Off".to_string(), true) },
        ];
        let host = self.host.clone();
        cx.spawn(async move |panel, cx| {
            let Some(chosen) = host.menu(&items, position) else {
                return;
            };
            let turn_on = chosen == 0;
            if turn_on == on {
                return;
            }
            panel
                .update(cx, |panel, _| {
                    let open = panel.open.as_ref();
                    if open.is_some_and(|open| open.tab == tab && open.closing.is_none()) {
                        // check.sh reads this line.
                        log!("browser: site controls set {ENGINE_NAME} {}", items[chosen].title);
                        panel.host.set_system_engine(tab, turn_on);
                    }
                })
                .ok();
        })
        .detach();
    }

    /// Turns blocking on the site the other way. The core reloads the page and reports the site
    /// again, which redraws the row.
    fn toggle_ad_blocking(&mut self, on: bool, cx: &mut Context<Self>) {
        if let Some(open) = self.open.as_ref().filter(|open| open.closing.is_none()) {
            // check.sh reads this line.
            log!("browser: site controls set ad blocking {}", if on { "on" } else { "off" });
            self.host.set_ad_blocking(open.tab, on);
            cx.notify();
        }
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        if let Some(open) = self.open.as_ref().filter(|open| open.closing.is_none()) {
            log!("browser: site controls reset");
            self.host.reset_permissions(open.tab);
            cx.notify();
        }
    }

    /// Hangs the tooltip saying `text` from `frame` in the panel's window, or takes it away.
    fn tooltip(&mut self, text: SharedString, frame: Bounds<Pixels>, hovered: bool, cx: &mut App) {
        let Some(open) = self.open.as_mut().filter(|open| open.closing.is_none()) else {
            return;
        };
        if hovered {
            let anchor = tooltip::Anchor::under(open.in_window(frame), open.window);
            open.tip = Some(text.clone());
            self.host.tooltip(Some((text, anchor)), cx);
        } else if open.tip.as_ref() == Some(&text) {
            open.tip = None;
            self.host.tooltip(None, cx);
        }
    }

    fn set_hovered(&mut self, heading: Heading, hovered: bool, cx: &mut Context<Self>) {
        if let Some(open) = self.open.as_mut() {
            match hovered {
                true => open.hovered = Some(heading),
                false if open.hovered == Some(heading) => open.hovered = None,
                false => return,
            }
            cx.notify();
        }
    }

    /// A tile `TILE_WIDTH` by `TILE_HEIGHT` at (`x`, `y`), named by its tooltip.
    fn tile_frame(x: f32, y: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(TILE_WIDTH), px(TILE_HEIGHT)))
    }

    fn tile(&self, index: usize, tile: Tile, y: f32, cx: &mut Context<Self>) -> gpui::AnyElement {
        let frame = Self::tile_frame(PADDING + index as f32 * (TILE_WIDTH + GAP), y);
        div()
            .id(tile.name)
            .absolute()
            .left(frame.origin.x)
            .top(frame.origin.y)
            .w(frame.size.width).h(frame.size.height)
            .rounded(px(TILE_RADIUS))
            .bg(linear_gradient(
                180.,
                linear_color_stop(rgba(TILE_TOP), 0.),
                linear_color_stop(rgba(TILE_BOTTOM), 1.),
            ))
            .border_t_1()
            .border_color(rgba(TILE_HIGHLIGHT))
            .flex()
            .items_center()
            .justify_center()
            .on_hover(cx.listener(move |panel, hovered: &bool, _, cx| {
                panel.tooltip(tile.label.into(), frame, *hovered, cx)
            }))
            .on_click(cx.listener(move |panel, _, _, cx| panel.run_tile(tile, frame, cx)))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .rounded(px(TILE_RADIUS))
                    .hover(|style| style.bg(rgba(HOVERED))),
            )
            .child(svg().path(tile.glyph).size(px(tile.glyph_size)).text_color(rgba(TEXT)))
            .into_any_element()
    }

    /// The extensions' tiles, then the + tile, four to a row.
    fn extensions(&self, open: &Open, y: f32, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let plate = |id: SharedString, index: usize| {
            let (column, line) = (index % EXTENSIONS_PER_ROW, index / EXTENSIONS_PER_ROW);
            let x = PADDING + column as f32 * (TILE_WIDTH + GAP);
            let frame = Self::tile_frame(x, y + line as f32 * (TILE_HEIGHT + GAP));
            let element = div()
                .id(id)
                .absolute()
                .left(frame.origin.x)
                .top(frame.origin.y)
                .w(frame.size.width).h(frame.size.height)
                .rounded(px(TILE_RADIUS))
                .bg(rgba(PLATE))
                .hover(|style| style.bg(rgba(PLATE_HOVERED)))
                .flex()
                .items_center()
                .justify_center();
            (element, frame)
        };
        let mut tiles: Vec<gpui::AnyElement> = (open.extensions.iter().enumerate())
            .map(|(index, extension)| {
                let (element, frame) = plate(format!("extension-{}", extension.id).into(), index);
                let (id, title) = (extension.id.clone(), extension.title.clone());
                element
                    .on_hover(cx.listener(move |panel, hovered: &bool, _, cx| {
                        panel.tooltip(title.clone(), frame, *hovered, cx)
                    }))
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.run("extension", Run::Extension(id.clone()), cx)
                    }))
                    .children(
                        (extension.icon.clone()).map(|icon| img(icon).size(px(EXTENSION_ICON))),
                    )
                    .into_any_element()
            })
            .collect();
        let (plus, frame) = plate("add-extension".into(), open.extensions.len());
        tiles.push(
            plus.on_hover(cx.listener(move |panel, hovered: &bool, _, cx| {
                panel.tooltip("Add Extension".into(), frame, *hovered, cx)
            }))
            .on_click(cx.listener(|panel, _, _, cx| {
                panel.run("add-extension", Run::Command(Command::ManageExtensions), cx)
            }))
            .child(svg().path("icons/plus.svg").size(px(PLUS_GLYPH)).text_color(rgba(TEXT)))
            .into_any_element(),
        );
        tiles
    }

    /// A heading with its cap's top at `cap`, and its link while the pointer is on it.
    fn heading(
        &self,
        heading: Heading,
        cap: f32,
        hovered: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (title, link, run) = match heading {
            Heading::Extensions => {
                ("Extensions", "Manage", Run::Command(Command::ManageExtensions))
            }
            Heading::Settings => ("Settings", "More", Run::SiteSettings),
        };
        // The heading's line, from the gap above it to the gap below.
        let (top, height) = (cap - 9., HEADING_SIZE * CAP + 18.);
        let text = |cap: f32, size: f32| {
            div()
                .absolute()
                .top(px(text_top(cap, size) - top))
                .h(px(LINE))
                .line_height(px(LINE))
                .text_size(px(size))
        };
        let link_cap = cap + (HEADING_SIZE - LINK_SIZE) * CAP / 2.;
        let name = link.to_lowercase();
        div()
            .id(SharedString::from(format!("heading-{}", title.to_lowercase())))
            .absolute()
            .left_0()
            .top(px(top))
            .w(px(WIDTH))
            .h(px(height))
            .on_hover(cx.listener(move |panel, hovered: &bool, _, cx| {
                panel.set_hovered(heading, *hovered, cx)
            }))
            .child(
                text(cap, HEADING_SIZE)
                    .left(px(PADDING))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgba(TEXT))
                    .child(title),
            )
            .when(hovered, |row| {
                row.child(
                    text(link_cap, LINK_SIZE)
                        .id(SharedString::from(format!("link-{name}")))
                        .right(px(PADDING))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgba(LINK))
                        .on_click(cx.listener(move |panel, _, _, cx| {
                            panel.run(&name, run.clone(), cx)
                        }))
                        .child(link),
                )
            })
            .into_any_element()
    }

    /// A row at `y` with a circle holding `glyph`, filled if `on`, `name` and `state` under it.
    fn row(
        id: SharedString,
        y: f32,
        glyph: &'static str,
        on: bool,
        name: SharedString,
        state: Option<SharedString>,
    ) -> gpui::Stateful<gpui::Div> {
        let text = |cap: f32, size: f32| {
            div()
                .absolute()
                .left(px(ROW_TEXT - PADDING))
                .w(px(WIDTH - ROW_TEXT - PADDING))
                .top(px(text_top(cap, size)))
                .h(px(LINE))
                .line_height(px(LINE))
                .text_size(px(size))
                .truncate()
        };
        let middle = CIRCLE / 2.;
        // The name's cap is 8 pt over the circle's middle and the state's 8 under, or the name's
        // on it alone.
        let name_cap = match state {
            Some(_) => middle - 8. - NAME_SIZE * CAP / 2.,
            None => middle - NAME_SIZE * CAP / 2.,
        };
        div()
            .id(id)
            .absolute()
            .left(px(PADDING))
            .top(px(y))
            .w(px(WIDTH - 2. * PADDING))
            .h(px(CIRCLE))
            .child(
                div()
                    .absolute()
                    .size(px(CIRCLE))
                    .rounded_full()
                    .bg(rgba(if on { ON } else { OFF }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(glyph)
                            .size(px(ROW_GLYPH))
                            .text_color(rgba(if on { ON_GLYPH } else { OFF_GLYPH })),
                    ),
            )
            .child(
                text(name_cap, NAME_SIZE)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgba(TEXT))
                    .child(name),
            )
            .children(state.map(|state| {
                text(middle + 8. - STATE_SIZE * CAP / 2., STATE_SIZE)
                    .text_color(rgba(SECONDARY))
                    .child(state)
            }))
    }

    fn permission(
        &self,
        index: usize,
        permission: &Permission,
        y: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let row = Self::row(
            format!("site-permission-{index}").into(),
            y,
            glyph_for(&permission.key),
            permission.value == Value::Allow,
            permission.name.clone(),
            Some(permission.state.clone()),
        );
        match permission.choices.is_empty() {
            true => row.opacity(MANAGED_OPACITY).into_any_element(),
            false => {
                let permission = permission.clone();
                row.on_click(cx.listener(move |panel, event: &ClickEvent, _, cx| {
                    panel.choose_value(permission.clone(), event.position(), cx)
                }))
                .into_any_element()
            }
        }
    }

    fn footer(&self, site: &Site, y: f32, cx: &mut Context<Self>) -> [gpui::AnyElement; 2] {
        let lock = match site.connection {
            Connection::Secure => "icons/lock-simple-fill.svg",
            _ => "icons/lock-simple-open-fill.svg",
        };
        let button = div()
            .id("site-lock")
            .absolute()
            .left(px(PADDING))
            .top(px(y))
            .h(px(FOOTER))
            .max_w(px(WIDTH - 2. * PADDING - FOOTER - GAP))
            .rounded(px(LOCK_RADIUS))
            .bg(rgba(PLATE))
            .flex()
            .items_start()
            .pl(px(LOCK_LEFT))
            .pr(px(LOCK_RIGHT))
            .child(
                div()
                    .flex_none()
                    .w(px(LOCK_SLOT))
                    .h(px(FOOTER))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().path(lock).size(px(LOCK_GLYPH)).text_color(rgba(TEXT))),
            )
            .child(
                div()
                    .min_w_0()
                    .ml(px(LOCK_TEXT_GAP))
                    .mt(px(text_top(LOCK_CAP, NAME_SIZE)))
                    .h(px(LINE))
                    .line_height(px(LINE))
                    .text_size(px(NAME_SIZE))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(rgba(TEXT))
                    .truncate()
                    .child(lock_label(site)),
            )
            .map(|button| match site.certificate {
                true => button
                    .hover(|style| style.bg(rgba(PLATE_HOVERED)))
                    .on_click(cx.listener(|panel, _, _, cx| {
                        panel.run("certificate", Run::Certificate, cx)
                    })),
                false => button.opacity(DISABLED_OPACITY),
            });
        let more = div()
            .id("site-more")
            .absolute()
            .left(px(WIDTH - PADDING - FOOTER))
            .top(px(y))
            .size(px(FOOTER))
            .rounded_full()
            .bg(rgba(MORE_FILL))
            .hover(|style| style.bg(rgba(HOVERED)))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|panel, event: &ClickEvent, _, cx| {
                panel.more(event.position(), cx)
            }))
            .child(svg().path("icons/ellipsis.svg").size(px(MORE_GLYPH)).text_color(rgba(TEXT)));
        [button.into_any_element(), more.into_any_element()]
    }
}

impl Render for SiteControls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .key_context("SiteControls")
            .track_focus(&self.focus)
            .on_action(cx.listener(|panel, _: &Close, _, cx| panel.close("escape", cx)))
            .size_full()
            .font_family(".SystemUIFont");
        let Some(open) = self.open.as_mut() else {
            return root;
        };
        let Some(site) = open.site.clone() else {
            return root;
        };
        // AppKit puts the window on screen some frames after it's asked to, and only then does
        // GPUI draw it, so the motion starts there rather than skip its start.
        if open.shown.is_none() && window.is_visible() {
            open.shown = Some(Instant::now());
        }
        let progress = open.progress();
        let scale = FROM_SCALE + (1. - FROM_SCALE) * progress;
        let origin = open.origin();
        let moving = open.moving();
        let settling = !moving && !open.settled;
        open.settled |= settling;
        if moving {
            window.request_animation_frame();
        }
        if moving || settling {
            self.host.drew(scale, progress, origin);
        }
        if settling {
            // check.sh reads this line.
            log!("browser: site controls settled");
        }
        let Some(open) = self.open.as_ref() else {
            return root;
        };
        let at = layout(&site, open.extensions.len(), open.beak.is_some());
        let shape = corners(open.beak, at.height);
        // One device pixel, which a 0.5 pt line isn't on a 1× display, where it doesn't draw.
        let hairline = 1. / window.scale_factor();
        let card = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let mut fill = PathBuilder::fill();
                icon_picker::outline(&mut fill, bounds.origin, &shape, 0.);
                if let Ok(path) = fill.build() {
                    window.paint_path(path, rgba(FILL));
                }
                for (inset, width, color) in
                    [(hairline / 2., hairline, OUTLINE), (hairline + 0.5, 1., RIM)]
                {
                    let mut edge = PathBuilder::stroke(px(width));
                    icon_picker::outline(&mut edge, bounds.origin, &shape, inset);
                    if let Ok(path) = edge.build() {
                        window.paint_path(path, rgba(color));
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let tiles: Vec<_> = (TILES.iter().enumerate())
            .map(|(index, &tile)| self.tile(index, tile, at.tiles, cx))
            .collect();
        let extensions = self.extensions(open, at.extensions, cx);
        let extensions_heading = self.heading(
            Heading::Extensions,
            at.extensions_cap,
            open.hovered == Some(Heading::Extensions),
            cx,
        );
        let settings = at.settings_cap.map(|cap| {
            let hovered = open.hovered == Some(Heading::Settings);
            let mut rows = vec![self.heading(Heading::Settings, cap, hovered, cx)];
            let mut y = at.rows;
            if let Some(on) = site.system_engine {
                let state = if on { "On" } else { "Off" };
                let row = Self::row(
                    "site-engine".into(),
                    y,
                    ENGINE,
                    on,
                    ENGINE_NAME.into(),
                    Some(state.into()),
                );
                let row = row.on_click(cx.listener(move |panel, event: &ClickEvent, _, cx| {
                    panel.choose_engine(on, event.position(), cx)
                }));
                rows.push(row.into_any_element());
                y += ROW_PITCH;
            }
            // Before the permissions, as the row a page that doesn't work needs.
            if let Some(on) = site.ad_blocking {
                let state = if on { "On" } else { "Off for this site" };
                let name = "Block ads and trackers".into();
                let state = Some(state.into());
                let row = Self::row("site-ad-blocking".into(), y, AD_BLOCKING, on, name, state);
                let row = row.on_click(
                    cx.listener(move |panel, _, _, cx| panel.toggle_ad_blocking(!on, cx)),
                );
                rows.push(row.into_any_element());
                y += ROW_PITCH;
            }
            for (index, permission) in site.permissions.iter().enumerate() {
                rows.push(self.permission(index, permission, y, cx));
                y += ROW_PITCH;
            }
            if let Some(label) = site.reset.clone() {
                let row = Self::row("site-permissions-reset".into(), y, RESET, false, label, None);
                let row = row.on_click(cx.listener(|panel, _, _, cx| panel.reset(cx)));
                rows.push(row.into_any_element());
            }
            rows
        });
        let divider = div()
            .absolute()
            .left(px(PADDING))
            .top(px(at.divider))
            .w(px(WIDTH - 2. * PADDING))
            .h(px(1.))
            .bg(rgba(DIVIDER));
        let card = div()
            .id("site-controls")
            .relative()
            .size_full()
            .opacity(progress)
            .child(card)
            .children(tiles)
            .child(extensions_heading)
            .children(extensions)
            .children(settings.into_iter().flatten())
            .child(divider)
            .children(self.footer(&site, at.footer, cx));
        root.child(scaled(card, scale, origin))
    }
}

#[cfg(test)]
mod tests;
