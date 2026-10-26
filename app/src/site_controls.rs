//! The Site Controls: the panel the URL field's Site Controls button opens, as Arc's Site
//! Control Center (docs/ux-spec.md › Extensions › Site Controls). For the site of the page shown:
//! action tiles, the site's permissions as Chrome's page info lists them, each with its value and
//! changeable, and the connection in Chrome's words. GPUI can't draw over the page's native view,
//! so it's a window of its own that the host shows over the browser window, as its child, with
//! the keyboard, as the command bar is. What it asks of its window and the core goes through
//! `Host`.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, ClickEvent, Context, FocusHandle, Global, Hsla, KeyBinding, Pixels, Point,
    SharedString, Size, Task, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, actions, div, point, prelude::*, px, rgba, size, svg,
};

use crate::shell::{MenuEntry, cubic_bezier, scaled::scaled};

actions!(site_controls, [Close]);

// Ardia's grid (58 pt cells, 8 pt gaps, rows of 4) sets the panel's width, with Dia's styling where
// Arc's wasn't recorded: the page question's card, the URL field's plates, the header's glyph
// colour and hover plates. The rows and the footer are ours.
const CELL: f32 = 58.;
const GAP: f32 = 8.;
const PADDING: f32 = 12.;
pub const WIDTH: f32 = 4. * CELL + 3. * GAP + 2. * PADDING;
const RADIUS: f32 = 12.;
/// A line of white 0.1 inside its edge, as the page question's card has.
const EDGE: u32 = 0xffff_ff1a;
const TITLE: u32 = 0xffff_ffff;
const SECONDARY: u32 = 0xffff_ff99;
const DISABLED: u32 = 0xffff_ff42;
/// The URL field's plate, which a tile and the reset button sit on, and its hover.
const PLATE: u32 = 0xffff_ff1a;
const PLATE_HOVERED: u32 = 0xffff_ff1f;
/// A value's pop-up button has the header buttons' hover plate.
const VALUE_HOVERED: u32 = 0xffff_ff14;
const TILE_RADIUS: f32 = 10.;
const TILE_GLYPH: f32 = 16.;
const TILE_LINE: f32 = 13.;
const ROW: f32 = 32.;
const VALUE_HEIGHT: f32 = 24.;
const VALUE_RADIUS: f32 = 6.;
const VALUE_GLYPH: f32 = 12.;
const RESET_TOP: f32 = 6.;
const RESET_HEIGHT: f32 = 28.;
const RESET_RADIUS: f32 = 8.;
const SECTION_GAP: f32 = 12.;
const FOOTER: f32 = 16.;
/// Ardia's: it opens in 0.15 s and closes in 0.1 s, growing from its button and shrinking back
/// into it, from and to this much of its size, as it fades.
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
    /// Chrome's name for it, such as "Location".
    pub name: SharedString,
    pub value: Value,
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

/// What Picture in Picture can do for the page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PictureInPicture {
    #[default]
    None,
    /// Take its playing video.
    Enter,
    /// Bring the video it has back.
    Exit,
}

/// The site of a tab's page, as the core reports it.
#[derive(Clone, PartialEq, Debug)]
pub struct Site {
    /// As the user should see it, such as "example.com".
    pub site: SharedString,
    pub connection: Connection,
    /// Chrome's words for the connection, such as "Connection is secure".
    pub connection_summary: SharedString,
    pub permissions: Vec<Permission>,
    /// Chrome's words for its button that sets the permissions back to their defaults, when it
    /// shows one.
    pub reset: Option<SharedString>,
    pub picture_in_picture: PictureInPicture,
}

/// An action tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// Copies the page's URL without its tracking parameters, as Copy URL does.
    CopyLink,
    Find,
    PictureInPicture,
}

impl Tile {
    fn label(self, site: &Site) -> &'static str {
        match self {
            Tile::CopyLink => "Copy Clean Link",
            Tile::Find => "Find",
            Tile::PictureInPicture if site.picture_in_picture == PictureInPicture::Exit => {
                "Exit Picture in Picture"
            }
            Tile::PictureInPicture => "Picture in Picture",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Tile::CopyLink => "icons/link.svg",
            Tile::Find => "icons/text-search.svg",
            Tile::PictureInPicture => "icons/picture-in-picture-2.svg",
        }
    }

    /// For the logs.
    fn name(self) -> &'static str {
        match self {
            Tile::CopyLink => "copy-link",
            Tile::Find => "find",
            Tile::PictureInPicture => "picture-in-picture",
        }
    }
}

/// What the panel asks of its window and the core.
pub trait Host {
    /// Shows the panel's window over the browser window, as it draws now, with the keyboard:
    /// `size`, its top left at `at` in the window's content.
    fn show(&self, at: Point<Pixels>, size: Size<Pixels>);
    /// Makes it `height` tall, keeping its top edge.
    fn set_height(&self, height: f32);
    /// It drew a frame of its motion, which its shadow follows.
    fn drew(&self);
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
    fn run(&self, tile: Tile, tab: u64, cx: &mut App);
    /// Shows a menu of `items` at `position` in the panel's window, and returns the index of the
    /// one chosen once it closes. AppKit runs it in an event loop of its own, so it's called
    /// outside GPUI's updates.
    fn menu(&self, items: &[MenuEntry], position: Point<Pixels>) -> Option<usize>;
}

/// The panel for one tab, from the press that opens it until it has left.
struct Open {
    tab: u64,
    at: Point<Pixels>,
    /// Where across its top edge it grows from and shrinks back to: its button's middle.
    origin: f32,
    /// None until the core first reports, which is when it shows.
    site: Option<Site>,
    height: f32,
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
}

pub struct SiteControls {
    host: Rc<dyn Host>,
    fill: Box<dyn Fn(&App) -> Hsla>,
    focus: FocusHandle,
    open: Option<Open>,
}

struct SiteControlsWindow(WindowHandle<SiteControls>);

impl Global for SiteControlsWindow {}

/// Opens the panel's window, hidden, for `host` to show over the browser window, ahead of time as
/// the command bar's is. The panel takes `fill`'s colour, the window's.
pub fn open(
    host: Rc<dyn Host>,
    fill: impl Fn(&App) -> Hsla + 'static,
    cx: &mut App,
) -> Option<WindowHandle<SiteControls>> {
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
        // The panel's rounded shape is drawn, and the host's shadow takes it.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| SiteControls {
            host,
            fill: Box::new(fill),
            focus: cx.focus_handle(),
            open: None,
        })
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

/// Opens the panel for the site of `tab`'s page, its top left at `at` in the browser window,
/// growing from `anchor`, once the core has read the site. If it shows, closes it instead, as its
/// button's second press does.
pub fn show(tab: u64, at: Point<Pixels>, anchor: Bounds<Pixels>, cx: &mut App) {
    update(cx, |panel, _, cx| {
        if panel.open.as_ref().is_some_and(|open| open.closing.is_none()) {
            return panel.close("toggle", cx);
        }
        let origin = (anchor.center().x - at.x).clamp(px(0.), px(WIDTH));
        panel.open = Some(Open {
            tab,
            at,
            origin: origin.into(),
            site: None,
            height: 0.,
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
        let height = height(&site);
        let first = open.site.is_none();
        open.site = Some(site);
        if first {
            open.height = height;
            window.focus(&panel.focus, cx);
            panel.host.show(open.at, size(px(WIDTH), px(height)));
        } else if height != open.height {
            open.height = height;
            panel.host.set_height(height);
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

/// The site for the log: Chrome's words for the connection, then each permission and its value.
fn describe(site: &Site) -> String {
    let permissions: Vec<String> = site
        .permissions
        .iter()
        .map(|permission| format!("{} {}", permission.name, permission.value.label()))
        .collect();
    format!(
        "{}: {}; {}",
        site.site,
        site.connection_summary,
        if permissions.is_empty() { "no permissions".into() } else { permissions.join(", ") }
    )
}

/// The tiles the site's page offers, in Ardia's order without Share and Cast, which aren't built.
fn tiles(site: &Site) -> Vec<Tile> {
    let mut tiles = vec![Tile::CopyLink, Tile::Find];
    if site.picture_in_picture != PictureInPicture::None {
        tiles.push(Tile::PictureInPicture);
    }
    tiles
}

/// How tall the panel is for `site`, as it lays out below.
fn height(site: &Site) -> f32 {
    let mut height = PADDING + CELL;
    if !site.permissions.is_empty() {
        height += SECTION_GAP + site.permissions.len() as f32 * ROW;
        if site.reset.is_some() {
            height += RESET_TOP + RESET_HEIGHT;
        }
    }
    // The rule over the footer, and the card's edge, which is inside its size.
    height + SECTION_GAP + 1. + SECTION_GAP + FOOTER + PADDING + 2.
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

    fn run(&mut self, tile: Tile, cx: &mut Context<Self>) {
        let Some(tab) = self.open.as_ref().map(|open| open.tab) else {
            return;
        };
        log!("browser: site controls run {}", tile.name());
        // It closes first, so the find bar takes the keyboard from the browser window.
        self.close("tile", cx);
        self.host.run(tile, tab, cx);
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

    fn reset(&mut self, cx: &mut Context<Self>) {
        if let Some(open) = self.open.as_ref().filter(|open| open.closing.is_none()) {
            log!("browser: site controls reset");
            self.host.reset_permissions(open.tab);
            cx.notify();
        }
    }

    fn tile(&self, tile: Tile, site: &Site, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id(tile.name())
            .flex_1()
            .h(px(CELL))
            .px(px(4.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .rounded(px(TILE_RADIUS))
            .bg(rgba(PLATE))
            .hover(|style| style.bg(rgba(PLATE_HOVERED)))
            .on_click(cx.listener(move |panel, _, _, cx| panel.run(tile, cx)))
            .child(svg().path(tile.glyph()).size(px(TILE_GLYPH)).text_color(rgba(TITLE)))
            .child(
                div()
                    .text_size(px(11.))
                    .line_height(px(TILE_LINE))
                    .text_center()
                    .line_clamp(2)
                    .text_color(rgba(TITLE))
                    .child(tile.label(site)),
            )
    }

    fn permission(
        &self,
        index: usize,
        permission: &Permission,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let changeable = !permission.choices.is_empty();
        let value = div()
            .id(("site-permission", index))
            .h(px(VALUE_HEIGHT))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(4.))
            .rounded(px(VALUE_RADIUS))
            .text_color(rgba(if changeable { SECONDARY } else { DISABLED }))
            .child(permission.value.label())
            .when(changeable, |button| {
                let permission = permission.clone();
                button
                    .hover(|style| style.bg(rgba(VALUE_HOVERED)))
                    .on_click(cx.listener(move |panel, event: &ClickEvent, _, cx| {
                        panel.choose_value(permission.clone(), event.position(), cx)
                    }))
                    .child(
                        svg()
                            .path("icons/chevrons-up-down.svg")
                            .size(px(VALUE_GLYPH))
                            .text_color(rgba(SECONDARY)),
                    )
            });
        div()
            .h(px(ROW))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(GAP))
            .child(
                div().flex_1().truncate().text_color(rgba(TITLE)).child(permission.name.clone()),
            )
            .child(value)
    }
}

impl Render for SiteControls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .key_context("SiteControls")
            .track_focus(&self.focus)
            .on_action(cx.listener(|panel, _: &Close, _, cx| panel.close("escape", cx)))
            .size_full()
            .font_family(".SystemUIFont")
            .text_size(px(13.));
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
        let origin = point(px(open.origin), px(0.));
        let moving = open.moving();
        let settling = !moving && !open.settled;
        open.settled |= settling;
        if moving {
            window.request_animation_frame();
        }
        if moving || settling {
            self.host.drew();
        }
        if settling {
            // check.sh reads this line.
            log!("browser: site controls settled");
        }
        let tiles: Vec<_> = (tiles(&site).into_iter())
            .map(|tile| self.tile(tile, &site, cx).into_any_element())
            .collect();
        let permissions = (!site.permissions.is_empty()).then(|| {
            let rows: Vec<_> = (site.permissions.iter().enumerate())
                .map(|(index, permission)| {
                    self.permission(index, permission, cx).into_any_element()
                })
                .collect();
            let reset = site.reset.clone().map(|label| {
                div()
                    .id("site-permissions-reset")
                    .mt(px(RESET_TOP))
                    .h(px(RESET_HEIGHT))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(RESET_RADIUS))
                    .bg(rgba(PLATE))
                    .hover(|style| style.bg(rgba(PLATE_HOVERED)))
                    .text_color(rgba(TITLE))
                    .on_click(cx.listener(|panel, _, _, cx| panel.reset(cx)))
                    .child(label)
            });
            div()
                .mt(px(SECTION_GAP))
                .flex()
                .flex_col()
                .items_start()
                .child(div().w_full().flex().flex_col().children(rows))
                .children(reset)
        });
        let footer = div()
            .mt(px(SECTION_GAP))
            .h(px(FOOTER))
            .line_height(px(FOOTER))
            .text_size(px(12.))
            .truncate()
            .text_color(rgba(SECONDARY))
            .child(site.connection_summary.clone());
        let card = div()
            .id("site-controls")
            .size_full()
            .p(px(PADDING))
            .flex()
            .flex_col()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(rgba(EDGE))
            .bg((self.fill)(cx))
            .opacity(progress)
            .child(div().flex().gap(px(GAP)).children(tiles))
            .children(permissions)
            .child(div().mt(px(SECTION_GAP)).h(px(1.)).bg(rgba(EDGE)))
            .child(footer);
        root.child(scaled(card, FROM_SCALE + (1. - FROM_SCALE) * progress, origin))
    }
}

#[cfg(test)]
mod tests;
