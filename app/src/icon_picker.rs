//! The popover that picks a Space's icon, Arc's: it hangs under the Space's name, its arrow at the
//! icon, with Emoji | Icon and a trash button at its top, and under them the glyphs, or a field for
//! an emoji (docs/ux-spec.md › Spaces › Icon). GPUI can't draw over the page's native view, so
//! it's a window of its own that the host shows over the browser window, as its child, with the
//! keyboard, translucent over a blur in the shape drawn here. What it asks of its window goes
//! through `Host`.

use std::{rc::Rc, time::Instant};

use gpui::{
    App, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, Global, KeyBinding,
    MouseButton, PathBuilder, Pixels, Point, SharedString, Subscription, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions,
    canvas, div, point, prelude::*, px, rgb, rgba, size, svg,
};

use crate::{
    command_bar,
    shell::{KEY_CONTEXT, cubic_bezier},
    space_icon::{self, GLYPHS, SpaceIcon},
    text_input::TextInput,
};

actions!(icon_picker, [Dismiss, Keep]);

// Arc's, measured at 2× (docs/ux-spec.md › Spaces › Icon). The card is square, and the window
// holds the arrow over it too.
pub const WIDTH: f32 = 273.;
const CARD: f32 = 273.;
/// The arrow's tip is this far over the card, and its sides go down at 45° from there.
const ARROW: f32 = 13.;
/// Its tip, from the card's left edge, which its left side goes down to.
const ARROW_X: f32 = 13.;
/// The arrow's tip in the window, which the picker grows in from as it opens.
pub const ARROW_TIP: (f32, f32) = (ARROW_X, 0.);
const ARROW_TIP_RADIUS: f32 = 5.;
const ARROW_FOOT_RADIUS: f32 = 3.5;
const RADIUS: f32 = 20.;
pub const HEIGHT: f32 = ARROW + CARD;
/// The card's top is this far under the middle of the icon the arrow points at.
const BELOW_ICON: f32 = 25.;
/// Grey at 0.565 over the blur: lighter than the command bar's fill, as Arc's is.
const FILL: u32 = 0x4848_4890;
/// Emoji | Icon, 14.5 under the card's top, with the trash button 7 after it, the two centred.
const SEGMENTS_TOP: f32 = 14.5;
const SEGMENT_WIDTH: f32 = 57.;
const SEGMENTS_HEIGHT: f32 = 24.;
const SEGMENTS_RADIUS: f32 = 6.;
const SEGMENTS_FILL: u32 = 0xffff_ff2e;
const TRASH_GAP: f32 = 7.;
const TRASH_GLYPH: f32 = 16.;
const LABEL: u32 = 0xededef;
/// The glyphs, eight to a row on a 31 pt pitch, the first 12.25 in and 47.75 under the card's top.
const COLUMNS: usize = 8;
const PITCH: f32 = 31.;
const GRID_X: f32 = 12.25;
const GRID_TOP: f32 = 47.75;
const GLYPH_SIZE: f32 = 16.;
const PLATE: f32 = 30.;
const PLATE_RADIUS: f32 = 4.;
const CHOSEN: u32 = 0xffff_ff40;
/// Arc's hover wasn't captured: half the chosen plate.
const HOVERED: u32 = 0xffff_ff1f;
/// The emoji's field, which Arc's popover hasn't: Arc's emoji grid isn't built.
const FIELD: f32 = 48.;
const FIELD_TEXT: f32 = 28.;
const HINT: u32 = 0xffff_ff87;
const HINT_TEXT: &str = "Type an emoji, or pick one";
/// Arc's switch between Emoji and Icon slides the content a card's width over 0.2 s, easing out
/// (Arc 1.170 rec at 2×).
const SLIDE: f32 = 0.2;

/// What the picker asks of its window.
pub trait Host {
    /// Shows the picker's window with its top left at `left`, `top` in the browser window, with
    /// the keyboard.
    fn show(&self, left: f32, top: f32);
    /// Hides it, and the browser window gets the keyboard back.
    fn hide(&self);
    /// Shows the system's character palette, for the emoji's field, which has the keyboard.
    fn show_character_palette(&self, window: &mut Window);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tab {
    Emoji,
    Icon,
}

/// The icon a Space is to have: what the core keeps, `""` for none.
pub type Picked = Rc<dyn Fn(&str, &mut App)>;

struct Picking {
    space: u64,
    icon: SharedString,
    tab: Tab,
    accent: u32,
    picked: Picked,
    /// The emoji's field wants the character palette, which shows once the window is on screen.
    palette: bool,
    /// While the content slides from one tab's to the other's: the tab it slides from, and when.
    slide: Option<(Tab, Instant)>,
    _edit: Option<Subscription>,
}

pub struct IconPicker {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    emoji: Entity<TextInput>,
    picking: Option<Picking>,
}

struct IconPickerWindow(WindowHandle<IconPicker>);

impl Global for IconPickerWindow {}

/// Opens the picker's window, hidden, for `host` to show over the browser window, ahead of time as
/// the command bar's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<IconPicker>> {
    cx.bind_keys([
        KeyBinding::new("escape", Dismiss, Some("IconPicker")),
        KeyBinding::new("enter", Keep, Some("IconPicker")),
    ]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(HEIGHT)),
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
        // The card's shape is drawn, so the window around it is clear.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        let emoji = cx.new(|cx| {
            TextInput::new("", cx).caret_color(rgb(0xffffff)).selection_color(rgba(HOVERED))
        });
        cx.new(|cx| IconPicker { host, focus: cx.focus_handle(), emoji, picking: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(IconPickerWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the icon picker's window: {error}");
            None
        }
    }
}

/// The card's corners, from the window's top left, clockwise from the arrow's tip, each with the
/// radius it's rounded by, for the host's blur and shadow and for the drawing here.
pub fn corners() -> [(f32, f32, f32); 6] {
    [
        (ARROW_X, 0., ARROW_TIP_RADIUS),
        (ARROW_X + ARROW, ARROW, ARROW_FOOT_RADIUS),
        (WIDTH, ARROW, RADIUS),
        (WIDTH, HEIGHT, RADIUS),
        (0., HEIGHT, RADIUS),
        (0., ARROW, 0.),
    ]
}

/// The tab the picker opens on for a Space with `icon`: the emoji's if it has one.
pub fn tab_for(icon: &str) -> Tab {
    match SpaceIcon::parse(icon) {
        SpaceIcon::Emoji(_) => Tab::Emoji,
        _ => Tab::Icon,
    }
}

/// Shows the picker for `space`, which has `icon`, its arrow at `anchor`, the middle of the icon's
/// slot in the browser window, with the chosen segment in `accent`, 0xRRGGBB. `picked` gets the
/// icon the user picks.
pub fn show(
    space: u64,
    icon: SharedString,
    anchor: Point<f32>,
    accent: u32,
    picked: Picked,
    cx: &mut App,
) {
    let Some(window) = cx.try_global::<IconPickerWindow>().map(|picker| picker.0) else {
        return;
    };
    window
        .update(cx, |picker, window, cx| {
            let tab = tab_for(&icon);
            // check.sh reads this line.
            log!("browser: icon picker for {space} on {}", tab.name());
            picker.picking = Some(Picking {
                space,
                icon,
                tab,
                accent,
                picked,
                palette: false,
                slide: None,
                _edit: None,
            });
            picker.show_tab(tab, window, cx);
            picker.host.show(anchor.x - ARROW_X, anchor.y + BELOW_ICON - ARROW);
            cx.notify();
        })
        .ok();
}

/// The user left the picker without picking, such as with a click outside it.
pub fn dismissed(cx: &mut App) {
    if let Some(window) = cx.try_global::<IconPickerWindow>().map(|picker| picker.0) {
        window
            .update(cx, |picker, _, cx| {
                picker.picking = None;
                cx.notify();
            })
            .ok();
    }
}

/// Whether the picker shows, for a click on the icon it points at to close it rather than open it
/// again.
pub fn is_open(cx: &App) -> bool {
    cx.try_global::<IconPickerWindow>()
        .and_then(|picker| picker.0.read(cx).ok())
        .is_some_and(|picker| picker.picking.is_some())
}

/// How far right the old tab's content and the new one's are, `progress` (0 to 1) into a switch
/// from `from` to `to`: the content moves the way the segment chosen is from the other, so Icon's
/// comes in from the right.
fn slid(from: Tab, to: Tab, progress: f32) -> (f32, f32) {
    let way = match (from, to) {
        (Tab::Emoji, Tab::Icon) => -1.,
        (Tab::Icon, Tab::Emoji) => 1.,
        _ => 0.,
    };
    let t = cubic_bezier(0., 0., 0.58, 1., progress.clamp(0., 1.));
    (way * WIDTH * t, way * WIDTH * (t - 1.))
}

impl Tab {
    fn name(self) -> &'static str {
        match self {
            Tab::Emoji => "emoji",
            Tab::Icon => "icon",
        }
    }
}

impl IconPicker {
    fn show_tab(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picking) = &mut self.picking else {
            return;
        };
        if picking.tab != tab {
            picking.slide = Some((picking.tab, Instant::now()));
        }
        picking.tab = tab;
        picking._edit = None;
        if tab == Tab::Icon {
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        // The Space's emoji, all selected, for a pick to replace.
        let start = match SpaceIcon::parse(&picking.icon) {
            SpaceIcon::Emoji(emoji) => SharedString::from(emoji.to_string()),
            _ => SharedString::default(),
        };
        self.emoji.update(cx, |field, cx| field.reset(start.clone(), window, cx));
        // A pick from the palette, or a key, is the icon at once, as no ↩ is needed for one
        // character.
        picking._edit = Some(cx.observe_in(&self.emoji, window, move |picker, field, _, cx| {
            let field = field.read(cx);
            let text = field.text();
            if !field.composing() && !text.trim().is_empty() && text != start.as_ref() {
                let icon = space_icon::first_character(text).to_string();
                picker.pick(&icon, cx);
            }
        }));
        window.focus(&self.emoji.focus_handle(cx), cx);
        picking.palette = true;
        cx.notify();
    }

    /// Gives the Space `icon`, `""` for none, if it hasn't it already, and closes.
    fn pick(&mut self, icon: &str, cx: &mut Context<Self>) {
        let Some(picking) = self.picking.take() else {
            return;
        };
        self.host.hide();
        if icon != picking.icon.as_ref() {
            (picking.picked)(icon, cx);
        }
        cx.notify();
    }

    fn dismiss(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
        if self.picking.take().is_some() {
            // check.sh reads this line.
            log!("browser: icon picker dismissed");
            self.host.hide();
            cx.notify();
        }
    }

    /// ↩ in the emoji's field keeps what it holds, and none takes the icon away.
    fn keep(&mut self, _: &Keep, _: &mut Window, cx: &mut Context<Self>) {
        match self.picking.as_ref().map(|picking| picking.tab) {
            Some(Tab::Emoji) => {
                let icon = space_icon::first_character(self.emoji.read(cx).text()).to_string();
                self.pick(&icon, cx);
            }
            Some(Tab::Icon) | None => {}
        }
    }

    /// Emoji | Icon, and the trash button after it.
    fn top(&self, picking: &Picking, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let segment = |tab: Tab, label: &'static str| {
            let chosen = picking.tab == tab;
            div()
                .id(label)
                .w(px(SEGMENT_WIDTH))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(SEGMENTS_RADIUS))
                .when(chosen, |segment| {
                    segment.bg(rgb(picking.accent)).text_color(rgb(0xffffff))
                })
                .child(label)
                .on_click(cx.listener(move |picker, _, window, cx| {
                    if picker.picking.as_ref().is_some_and(|picking| picking.tab != tab) {
                        // check.sh reads this line.
                        log!("browser: icon picker on {}", tab.name());
                        picker.show_tab(tab, window, cx);
                    }
                }))
        };
        let trash = div()
            .id("icon-picker-trash")
            .size(px(SEGMENTS_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(SEGMENTS_RADIUS))
            .hover(|trash| trash.bg(rgba(HOVERED)))
            .child(svg().path("icons/trash.svg").size(px(TRASH_GLYPH)).text_color(rgb(LABEL)))
            .on_click(cx.listener(|picker, _, _, cx| picker.pick("", cx)));
        div()
            .absolute()
            .top(px(ARROW + SEGMENTS_TOP))
            .left_0()
            .w(px(WIDTH))
            .h(px(SEGMENTS_HEIGHT))
            .flex()
            .justify_center()
            .gap(px(TRASH_GAP))
            .child(
                div()
                    .h_full()
                    .flex()
                    .rounded(px(SEGMENTS_RADIUS))
                    .bg(rgba(SEGMENTS_FILL))
                    .child(segment(Tab::Emoji, "Emoji"))
                    .child(segment(Tab::Icon, "Icon")),
            )
            .child(trash)
    }

    /// The glyphs, the Space's on a plate.
    fn glyphs(&self, picking: &Picking, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let chosen = match SpaceIcon::parse(&picking.icon) {
            SpaceIcon::Glyph(index) => Some(index),
            _ => None,
        };
        let cells = GLYPHS.iter().enumerate().map(|(index, &(_, path))| {
            div()
                .id(("glyph", index))
                .size(px(PITCH))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(PLATE))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(PLATE_RADIUS))
                        .when(chosen == Some(index), |plate| plate.bg(rgba(CHOSEN)))
                        .when(chosen != Some(index), |plate| {
                            plate.hover(|plate| plate.bg(rgba(HOVERED)))
                        })
                        .child(svg().path(path).size(px(GLYPH_SIZE)).text_color(rgb(LABEL))),
                )
                .on_click(cx.listener(move |picker, _, _, cx| {
                    picker.pick(&space_icon::glyph(index), cx)
                }))
        });
        div()
            .absolute()
            .top(px(ARROW + GRID_TOP))
            .left(px(GRID_X))
            .w(px(PITCH * COLUMNS as f32))
            .flex()
            .flex_wrap()
            .children(cells)
    }

    /// The emoji's field, and the palette to pick one from.
    fn emoji_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .absolute()
            .top(px(ARROW + GRID_TOP))
            .left_0()
            .w(px(WIDTH))
            .bottom_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .child(
                div()
                    .id("icon-picker-emoji")
                    .size(px(FIELD))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .bg(rgba(HOVERED))
                    .text_size(px(FIELD_TEXT))
                    .child(self.emoji.clone())
                    .on_click(cx.listener(|picker, _, window, _| {
                        picker.host.show_character_palette(window)
                    })),
            )
            .child(div().text_size(px(12.)).text_color(rgba(HINT)).child(HINT_TEXT))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|picker, _, window, cx| {
                    window.focus(&picker.emoji.focus_handle(cx), cx)
                }),
            )
    }
}

/// The card's outline `inset` inside its edge: each corner moved in along both its sides, and its
/// radius smaller where it bulges out and larger where it bends in.
fn inset_corners(inset: f32) -> Vec<(Point<f32>, f32, bool)> {
    let corners = corners();
    let count = corners.len();
    let at = |index: usize| {
        let (x, y, _) = corners[index % count];
        point(x, y)
    };
    let unit = |from: Point<f32>, to: Point<f32>| {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let length = (dx * dx + dy * dy).sqrt();
        point(dx / length, dy / length)
    };
    (0..count)
        .map(|index| {
            let (before, here, after) = (at(index + count - 1), at(index), at(index + 1));
            let (along_in, along_out) = (unit(before, here), unit(here, after));
            // Clockwise on screen, each side's inside is to its right.
            let normal_in = point(-along_in.y, along_in.x);
            let normal_out = point(-along_out.y, along_out.x);
            let scale = inset / (1. + normal_in.x * normal_out.x + normal_in.y * normal_out.y);
            let moved = point(
                here.x + (normal_in.x + normal_out.x) * scale,
                here.y + (normal_in.y + normal_out.y) * scale,
            );
            let convex = along_in.x * along_out.y - along_in.y * along_out.x > 0.;
            let radius = corners[index].2;
            let radius = if radius == 0. {
                0.
            } else if convex {
                (radius - inset).max(0.)
            } else {
                radius + inset
            };
            (moved, radius, convex)
        })
        .collect()
}

/// The card's outline `inset` inside its edge, at `origin`, into `path`.
fn outline(path: &mut PathBuilder, origin: Point<Pixels>, inset: f32) {
    let corners = inset_corners(inset);
    let count = corners.len();
    let to = |at: Point<f32>| origin + point(px(at.x), px(at.y));
    for index in 0..count {
        let (here, radius, convex) = corners[index];
        let before = corners[(index + count - 1) % count].0;
        let after = corners[(index + 1) % count].0;
        let toward = |other: Point<f32>, by: f32| {
            let (dx, dy) = (other.x - here.x, other.y - here.y);
            let length = (dx * dx + dy * dy).sqrt();
            point(here.x + dx / length * by, here.y + dy / length * by)
        };
        // Where the corner's arc meets its sides: as far along each as the radius over the tangent
        // of half the angle between them.
        let (dx0, dy0) = (before.x - here.x, before.y - here.y);
        let (dx1, dy1) = (after.x - here.x, after.y - here.y);
        let lengths = ((dx0 * dx0 + dy0 * dy0) * (dx1 * dx1 + dy1 * dy1)).sqrt();
        let cos = (dx0 * dx1 + dy0 * dy1) / lengths;
        let half = cos.clamp(-1., 1.).acos() / 2.;
        let along = if radius > 0. { radius / half.tan() } else { 0. };
        let (start, end) = (toward(before, along), toward(after, along));
        if index == 0 {
            path.move_to(to(start));
        } else {
            path.line_to(to(start));
        }
        if radius > 0. {
            path.arc_to(point(px(radius), px(radius)), px(0.), false, convex, to(end));
        }
    }
    path.close();
}

impl Focusable for IconPicker {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for IconPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div()
            .key_context(KEY_CONTEXT)
            .size_full()
            .font_family(".SystemUIFont");
        // AppKit puts the window on screen some frames after it's asked to, and the palette goes
        // with the key window then.
        if let Some(picking) = self.picking.as_mut().filter(|picking| picking.palette) {
            if window.is_visible() {
                picking.palette = false;
                self.host.show_character_palette(window);
            } else {
                window.request_animation_frame();
            }
        }
        // The slide's progress, while the content slides.
        let slide = self.picking.as_mut().and_then(|picking| {
            let (from, start) = picking.slide?;
            let progress = start.elapsed().as_secs_f32() / SLIDE;
            if progress >= 1. {
                picking.slide = None;
                return None;
            }
            window.request_animation_frame();
            Some((from, progress))
        });
        let Some(picking) = &self.picking else {
            return root;
        };
        // One device pixel, which a 0.5 pt line isn't on a 1× display, where it doesn't draw.
        let hairline = 1. / window.scale_factor();
        let card = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let mut fill = PathBuilder::fill();
                outline(&mut fill, bounds.origin, 0.);
                if let Ok(path) = fill.build() {
                    window.paint_path(path, rgba(FILL));
                }
                // The command bar's edge: a 0.5 pt outline, and inside it a 1 pt grey one.
                for (inset, width, color) in [
                    (hairline / 2., hairline, command_bar::OUTLINE),
                    (hairline + 0.5, 1., command_bar::EDGE),
                ] {
                    let mut edge = PathBuilder::stroke(px(width));
                    outline(&mut edge, bounds.origin, inset);
                    if let Ok(path) = edge.build() {
                        window.paint_path(path, rgb(color));
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let mut content = |tab, x: f32| {
            let pane = match tab {
                Tab::Icon => self.glyphs(picking, cx).into_any_element(),
                Tab::Emoji => self.emoji_field(cx).into_any_element(),
            };
            div().absolute().top_0().left(px(x)).size_full().child(pane)
        };
        let content = match slide {
            Some((from, progress)) => {
                let (old, new) = slid(from, picking.tab, progress);
                vec![content(from, old), content(picking.tab, new)]
            }
            None => vec![content(picking.tab, 0.)],
        };
        let top = self.top(picking, cx);
        root.child(
            div()
                .key_context("IconPicker")
                .track_focus(&self.focus)
                .on_action(cx.listener(Self::dismiss))
                .on_action(cx.listener(Self::keep))
                .relative()
                .size_full()
                .text_color(rgb(LABEL))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .child(card)
                .child(top)
                .children(content),
        )
    }
}

impl IconPicker {
    /// The Space it's picking for, and the tab it shows, while it shows.
    pub fn picking(&self) -> Option<(u64, Tab)> {
        self.picking.as_ref().map(|picking| (picking.space, picking.tab))
    }
}

#[cfg(test)]
mod tests;
