//! Add Split View's targets (Dia, docs/ux-spec.md › Split view › Drag targets): a tab dragged out
//! of the sidebar over the page becomes a picture of a window, and a target slides in at each of
//! the card's sides. Let go over one, the tab joins the page shown in a split view on that side.
//! GPUI can't draw over the page's native view, so the app draws them in a window of its own over
//! the browser window, which the mouse goes through (`Browser::show_drop_targets`).

use web_time::Instant;

use gpui::{
    AnyElement, BoxShadow, Bounds, Context, FontWeight, Hsla, PathBuilder, Pixels, Point, Window,
    canvas, div, hsla, point, prelude::*, px, rgb, size,
};

use super::{FAVICON_DRAWN, Place, Shell, TITLE_GAP};

/// Each target's width and height as a fraction of the card's, and how far it is in from the
/// card's side: as it slides in, grown, and under the tab, where it leans TARGET_LEAN of the way
/// toward the pointer.
const TARGET_REST: (f32, f32, f32) = (0.132, 0.383, 37.5);
const TARGET_GROWN: (f32, f32, f32) = (0.215, 0.611, 30.);
const TARGET_UNDER: (f32, f32) = (0.241, 0.69);
const TARGET_LEAN: f32 = 0.43;
/// The targets slide in, then grow, and the one under the tab grows again, each on an ease-out
/// cubic with no overshoot.
const TARGET_SLIDE: f32 = 0.11;
const TARGET_GROW: f32 = 0.12;
use crate::tokens::shell::split_targets::TARGET_RADIUS;
/// A target's dashed line, TARGET_DASH_INSET inside its edge: its dashes, as long as its gaps, and
/// its width, away from the tab and under it.
use crate::tokens::shell::split_targets::TARGET_DASH_INSET;
const TARGET_DASH: (f32, f32) = (6., 1.75);
const TARGET_DASH_UNDER: (f32, f32) = (7., 2.);
use crate::tokens::shell::split_targets::TARGET_LIGHTER;
use crate::tokens::shell::split_targets::TARGET_EDGE;
use crate::tokens::shell::split_targets::TARGET_ACCENT;
use crate::tokens::shell::split_targets::TARGET_DASHES;
use crate::tokens::shell::split_targets::TARGET_TEXT;
/// Dia's targets came once the pointer was 26 pt into the page from its top strip. Ours: the same
/// past the card's left edge, until Arc's is measured.
const TARGET_TRIGGER: f32 = 26.;
/// The picture of a window the tab becomes over the page, centred on the pointer (Dia), and ours:
/// its corners, rim, traffic lights and the page in it.
const PICTURE: (f32, f32) = (209., 109.5);
use crate::tokens::shell::split_targets::PICTURE_RADIUS;
use crate::tokens::shell::split_targets::PICTURE_RIM;
use crate::tokens::shell::split_targets::PICTURE_LIGHTS;
use crate::tokens::shell::split_targets::PICTURE_LIGHT;
use crate::tokens::shell::split_targets::PICTURE_TOP;
use crate::tokens::shell::split_targets::PICTURE_INSET;
use crate::tokens::shell::split_targets::HALF_GLYPH_RADIUS;
use crate::tokens::shell::split_targets::PICTURE_SHADOW;

/// A dragged tab over the page, where it can join the page shown in a split view.
pub(super) struct OverPage {
    /// The tab it would join.
    with: u64,
    /// When the targets began sliding in.
    since: Instant,
    /// Where the pointer is in the window.
    pointer: Point<Pixels>,
    /// For the left target and the right: whether the tab is over it, since when, and how far it
    /// had grown toward its size under the tab then.
    under: [(bool, Instant, f32); 2],
}

impl OverPage {
    /// The target the tab is over: the left (true) or the right.
    fn side(&self) -> Option<bool> {
        let [(left, ..), (right, ..)] = self.under;
        (left || right).then_some(left)
    }
}

impl Shell {
    /// The tab a dragged `tab` would join in a split view on the page: the one shown, or for the
    /// tab shown alone, the one shown before it (Dia). None where the page has no targets for it:
    /// a favorite, a folder, a split view's row, a tab of the split shown, or a split shown full.
    fn split_partner(&self, tab: u64) -> Option<u64> {
        if self.sidebar_hidden || self.is_folder(tab) || self.panes(tab).len() > 1 {
            return None;
        }
        let shown = self.panes(self.selected);
        if self.selected == 0 || shown.len() >= super::MAX_PANES {
            return None;
        }
        if shown != [tab] {
            return (!shown.contains(&tab)).then_some(self.selected);
        }
        let space = self.shown_space()?;
        let in_space = |other: &u64| space.pinned.contains(other) || space.today.contains(other);
        self.recent.iter().copied().find(|&other| other != tab && in_space(&other))
    }

    /// The dragged tab is at `position` in the window. Past TARGET_TRIGGER into the card, where it
    /// can join the page shown, the targets slide in and the tab becomes a picture of a window, and
    /// a target it's over grows toward it. Returns whether it's over the page. Back over the
    /// sidebar, it's a row again.
    pub(super) fn drag_over_page(
        &mut self,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let card = self.page_frame(self.viewport).0;
        let with = match drag.from {
            Place::List(..) => self.split_partner(drag.tab),
            Place::Favorite(_) => None,
        };
        let Some(with) = with.filter(|_| position.x > card.left() + px(TARGET_TRIGGER)) else {
            if let Some(drag) = self.drag.as_mut().filter(|drag| drag.over_page.is_some()) {
                drag.over_page = None;
                // check.sh reads this line.
                log!("browser: drop targets gone");
                cx.notify();
            }
            return false;
        };
        let now = Instant::now();
        let Some(drag) = &mut self.drag else {
            return false;
        };
        drag.moving = true;
        let over = drag.over_page.get_or_insert_with(|| {
            log!("browser: drop targets for {with}");
            OverPage {
                with,
                since: now,
                pointer: position,
                under: [(false, now, 0.); 2],
            }
        });
        over.pointer = position;
        let side = over.side();
        for (index, left) in [true, false].into_iter().enumerate() {
            let (under, since, from) = over.under[index];
            // Under once the pointer is in the grown target, until it leaves the one under it.
            let bounds = match under {
                true => under_bounds(card, left, position.x),
                false => target_bounds(card, left, TARGET_GROWN),
            };
            if bounds.contains(&position) != under {
                over.under[index] = (!under, now, grown(under, since, from));
            }
        }
        if over.side() != side {
            // check.sh reads this line.
            let side = match over.side() {
                Some(true) => "left",
                Some(false) => "right",
                None => "none",
            };
            log!("browser: drop target {side}");
        }
        cx.notify();
        true
    }

    /// The pointer let go of a tab over the page: over a target, it joins the page shown in a split
    /// view on that side and shows there, and anywhere else it goes back to its row. Returns
    /// whether it was over the page.
    pub(super) fn drop_over_page(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let Some(over) = &drag.over_page else {
            return false;
        };
        let (tab, with, side) = (drag.tab, over.with, over.side());
        // The release that ends the drag isn't a click.
        self.dragged = true;
        self.settle_drag();
        if let Some(left) = side {
            self.browser.split_with(tab, with, left, cx);
        }
        cx.notify();
        true
    }

    /// What the window over the page draws while a dragged tab is over it: the targets, clipped to
    /// the card, and the tab as a picture of a window under the pointer.
    pub fn drop_targets(&self, window: &mut Window) -> AnyElement {
        let Some(over) = self.drag.as_ref().and_then(|drag| drag.over_page.as_ref()) else {
            return div().into_any_element();
        };
        let Some(tab) = self.drag.as_ref().map(|drag| drag.tab) else {
            return div().into_any_element();
        };
        let card = self.page_frame(self.viewport).0;
        let accent = Hsla::from(rgb(self.focus_color.unwrap_or(self.accent_color)));
        let fill = self.window_fill().blend(hsla(0., 0., 1., TARGET_LIGHTER));
        let t = over.since.elapsed().as_secs_f32();
        let growing =
            over.under.iter().any(|(_, since, _)| since.elapsed().as_secs_f32() < TARGET_GROW);
        if growing || t < TARGET_SLIDE + TARGET_GROW {
            window.request_animation_frame();
        }
        let targets = [true, false].into_iter().enumerate().map(|(index, left)| {
            let (under, since, from) = over.under[index];
            let way = grown(under, since, from);
            let under = under_bounds(card, left, over.pointer.x);
            let bounds = mix_bounds(arriving(card, left, t), under, way);
            target(bounds, card, left, way, fill, accent)
        });
        let page = div()
            .absolute()
            .left(card.left())
            .top(card.top())
            .w(card.size.width)
            .h(card.size.height)
            .rounded(px(super::CARD_RADIUS))
            .overflow_hidden()
            .children(targets);
        let title = self.tabs.get(&tab).map(|tab| tab.title.clone()).unwrap_or_default();
        let icon = self.tab_icon(tab);
        div()
            .size_full()
            .relative()
            .font_family(".SystemUIFont")
            .child(page)
            .child(picture(over.pointer, self.window_fill(), icon, title))
            .into_any_element()
    }
}

/// How far a target has grown toward its size under the tab: `from` at `since`, going to all the
/// way if `under`, else none.
fn grown(under: bool, since: Instant, from: f32) -> f32 {
    let to = if under { 1. } else { 0. };
    from + (to - from) * ease_out(since.elapsed().as_secs_f32() / TARGET_GROW)
}

fn ease_out(progress: f32) -> f32 {
    1. - (1. - progress.clamp(0., 1.)).powi(3)
}

/// The left target's bounds in `card`, or the right's: `(width, height, inset)`, its width and
/// height as fractions of the card's and how far in it is from the card's side, centred down it.
fn target_bounds(card: Bounds<Pixels>, left: bool, at: (f32, f32, f32)) -> Bounds<Pixels> {
    let (width, height, inset) = at;
    let size = size(card.size.width * width, card.size.height * height);
    let x = match left {
        true => card.left() + px(inset),
        false => card.right() - px(inset) - size.width,
    };
    Bounds::new(point(x, card.top() + (card.size.height - size.height) / 2.), size)
}

/// A target under the tab: bigger, its centre leaning toward the pointer at `x`.
fn under_bounds(card: Bounds<Pixels>, left: bool, x: Pixels) -> Bounds<Pixels> {
    let grown = target_bounds(card, left, TARGET_GROWN);
    let (width, height) = TARGET_UNDER;
    let size = size(card.size.width * width, card.size.height * height);
    let centre = grown.center().x + (x - grown.center().x) * TARGET_LEAN;
    Bounds::new(
        point(centre - size.width / 2., card.top() + (card.size.height - size.height) / 2.),
        size,
    )
}

/// Where a target is `t` seconds after the tab came over the page: sliding in at its first size
/// from outside the card's side, then growing.
fn arriving(card: Bounds<Pixels>, left: bool, t: f32) -> Bounds<Pixels> {
    let rest = target_bounds(card, left, TARGET_REST);
    if t < TARGET_SLIDE {
        let out = rest.size.width + px(TARGET_REST.2);
        let away = out * (1. - ease_out(t / TARGET_SLIDE));
        let x = if left { rest.left() - away } else { rest.left() + away };
        return Bounds::new(point(x, rest.top()), rest.size);
    }
    let grown = target_bounds(card, left, TARGET_GROWN);
    mix_bounds(rest, grown, ease_out((t - TARGET_SLIDE) / TARGET_GROW))
}

fn mix_bounds(from: Bounds<Pixels>, to: Bounds<Pixels>, way: f32) -> Bounds<Pixels> {
    let mix = |from: Pixels, to: Pixels| from + (to - from) * way;
    Bounds::new(
        point(mix(from.left(), to.left()), mix(from.top(), to.top())),
        size(mix(from.size.width, to.size.width), mix(from.size.height, to.size.height)),
    )
}

/// A target at `bounds` in the window, drawn in `card`, `way` of the way to its look under the
/// tab: its fill, edge and dashed line, with a glyph of the half it adds over its title.
fn target(
    bounds: Bounds<Pixels>,
    card: Bounds<Pixels>,
    left: bool,
    way: f32,
    fill: Hsla,
    accent: Hsla,
) -> impl IntoElement {
    let fill = fill.blend(accent.opacity(TARGET_ACCENT * way));
    let white = |alpha: f32| hsla(0., 0., 1., alpha);
    let mix = |from: Hsla, to: Hsla| from.opacity(1. - way).blend(to.opacity(way));
    let text = mix(white(TARGET_TEXT), accent);
    let dashes = mix(white(TARGET_DASHES), accent);
    let dash = TARGET_DASH.0 + (TARGET_DASH_UNDER.0 - TARGET_DASH.0) * way;
    let width = TARGET_DASH.1 + (TARGET_DASH_UNDER.1 - TARGET_DASH.1) * way;
    let line = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let inset = px(TARGET_DASH_INSET);
            let inner = Bounds::new(
                bounds.origin + point(inset, inset),
                size(bounds.size.width - inset * 2., bounds.size.height - inset * 2.),
            );
            let mut path = PathBuilder::stroke(px(width)).dash_array(&[px(dash), px(dash)]);
            rounded_rect(&mut path, inner, px(TARGET_RADIUS - TARGET_DASH_INSET));
            if let Ok(path) = path.build() {
                window.paint_path(path, dashes);
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    div()
        .absolute()
        .left(bounds.left() - card.left())
        .top(bounds.top() - card.top())
        .w(bounds.size.width)
        .h(bounds.size.height)
        .rounded(px(TARGET_RADIUS))
        .bg(fill)
        .border(px(0.5))
        .border_color(white(TARGET_EDGE))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(TITLE_GAP))
        .child(line)
        .child(half_glyph(left, text))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .child(if left { "Add left split" } else { "Add right split" }),
        )
}

/// A rounded rectangle at `bounds` with corners of `radius`, clockwise from its top left.
pub(super) fn rounded_rect(path: &mut PathBuilder, bounds: Bounds<Pixels>, radius: Pixels) {
    let (left, top, right, bottom) = (bounds.left(), bounds.top(), bounds.right(), bounds.bottom());
    let radii = point(radius, radius);
    path.move_to(point(left + radius, top));
    path.line_to(point(right - radius, top));
    path.arc_to(radii, px(0.), false, true, point(right, top + radius));
    path.line_to(point(right, bottom - radius));
    path.arc_to(radii, px(0.), false, true, point(right - radius, bottom));
    path.line_to(point(left + radius, bottom));
    path.arc_to(radii, px(0.), false, true, point(left, bottom - radius));
    path.line_to(point(left, top + radius));
    path.arc_to(radii, px(0.), false, true, point(left + radius, top));
    path.close();
}

/// A ≈ 19 pt rectangle with its left half filled, or its right.
fn half_glyph(left: bool, color: Hsla) -> impl IntoElement {
    let half = div().absolute().top_0().bottom_0().w(px(8.)).bg(color);
    div()
        .relative()
        .w(px(19.))
        .h(px(15.))
        .rounded(px(HALF_GLYPH_RADIUS))
        .border(px(1.5))
        .border_color(color)
        .overflow_hidden()
        .child(if left { half.left_0() } else { half.right_0() })
}

/// The dragged tab over the page: a picture of a window centred on `pointer`, its page showing the
/// tab's favicon and title.
fn picture(
    pointer: Point<Pixels>,
    fill: Hsla,
    icon: AnyElement,
    title: gpui::SharedString,
) -> impl IntoElement {
    let (width, height) = PICTURE;
    let lights = PICTURE_LIGHTS.map(|color| {
        div().size(px(PICTURE_LIGHT)).rounded(px(PICTURE_LIGHT / 2.)).bg(rgb(color))
    });
    let page = div()
        .absolute()
        .left(px(PICTURE_INSET))
        .right(px(PICTURE_INSET))
        .top(px(PICTURE_TOP))
        .bottom(px(PICTURE_INSET))
        .rounded(px(PICTURE_RADIUS - PICTURE_INSET))
        .bg(fill.blend(hsla(0., 0., 1., TARGET_LIGHTER)))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(TITLE_GAP))
        .px(px(FAVICON_DRAWN))
        .child(div().flex_none().child(icon))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(12.))
                .text_color(hsla(0., 0., 1., TARGET_TEXT))
                .child(title),
        );
    div()
        .absolute()
        .left(pointer.x - px(width / 2.))
        .top(pointer.y - px(height / 2.))
        .w(px(width))
        .h(px(height))
        .rounded(px(PICTURE_RADIUS))
        .bg(fill)
        .border(px(0.5))
        .border_color(hsla(0., 0., 1., PICTURE_RIM))
        .shadow(vec![BoxShadow {
            color: hsla(0., 0., 0., PICTURE_SHADOW),
            offset: point(px(0.), px(8.)),
            blur_radius: px(24.),
            spread_radius: px(0.),
            inset: false,
        }])
        .child(div().absolute().left(px(8.)).top(px(6.)).flex().gap(px(4.)).children(lights))
        .child(page)
}
