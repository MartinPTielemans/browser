//! Add Split View's targets (Dia, docs/ux-spec.md › Split view › Drag targets): a tab dragged out
//! of the sidebar over the page becomes a picture of a window, and a target slides in at each of
//! the card's sides, and at its top and bottom as Arc's stacks them. Let go over one, the tab joins
//! the page shown in a split view on that side.
//! GPUI can't draw over the page's native view, so the app draws them in a window of its own over
//! the browser window, which the mouse goes through (`Browser::show_drop_targets`).

use std::time::Instant;

use gpui::{
    AnyElement, Bounds, Context, FontWeight, Hsla, PathBuilder, Pixels, Point, Window,
    canvas, div, point, prelude::*, px, rgb, size,
};

use super::{FAVICON_DRAWN, Place, Shell, TITLE_GAP};
use crate::palette::{Palette, TARGET_ACCENT, TARGET_PLATE};

/// Each target's width and height as a fraction of the card's, and how far it is in from the
/// card's side: as it slides in, grown, and under the tab, where it leans TARGET_LEAN of the way
/// toward the pointer.
const TARGET_REST: (f32, f32, f32) = (0.132, 0.383, 37.5);
const TARGET_GROWN: (f32, f32, f32) = (0.215, 0.611, 30.);
const TARGET_UNDER: (f32, f32) = (0.241, 0.69);
const TARGET_LEAN: f32 = 0.43;
/// The top and bottom targets' height and width as a fraction of the card's, and their inset, the
/// sides' turned, but only as wide as the room between the side targets grown (ours, as Arc's
/// aren't measured).
const STACK_REST: (f32, f32, f32) = (0.132, 0.28, 37.5);
const STACK_GROWN: (f32, f32, f32) = (0.215, 0.45, 30.);
const STACK_UNDER: (f32, f32) = (0.241, 0.51);
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
use crate::tokens::shell::split_targets::TARGET_EDGE;
use crate::tokens::shell::split_targets::TARGET_DASHES;
use crate::tokens::shell::split_targets::TARGET_TEXT;
/// Dia's targets came once the pointer was 26 pt into the page from its top strip. Ours: the same
/// past the card's left edge, until Arc's is measured.
const TARGET_TRIGGER: f32 = 26.;
/// The picture of a window the tab becomes over the page, centred on the pointer (Dia), and ours:
/// its corners, rim, traffic lights and the page in it.
const PICTURE: (f32, f32) = (209., 109.5);
use crate::tokens::shell::split_targets::PICTURE_RADIUS;
use crate::elevation::Level;
use crate::tokens::shell::split_targets::PICTURE_LIGHTS;
use crate::tokens::shell::split_targets::PICTURE_LIGHT;
use crate::tokens::shell::split_targets::PICTURE_TOP;
use crate::tokens::shell::split_targets::PICTURE_INSET;
use crate::tokens::shell::split_targets::HALF_GLYPH_RADIUS;
use crate::tokens::text;

/// Where a target is on the card, and so where the tab joins the page.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    /// Whether the split it makes shows its pages top to bottom.
    fn stacked(self) -> bool {
        matches!(self, Edge::Top | Edge::Bottom)
    }

    /// Whether the tab comes before the page it joins.
    fn first(self) -> bool {
        matches!(self, Edge::Left | Edge::Top)
    }

    fn name(self) -> &'static str {
        match self {
            Edge::Left => "left",
            Edge::Right => "right",
            Edge::Top => "top",
            Edge::Bottom => "bottom",
        }
    }
}

/// A dragged tab over the page, where it can join the page shown in a split view.
pub(super) struct OverPage {
    /// The tab it would join.
    with: u64,
    /// When the targets began sliding in.
    since: Instant,
    /// Where the pointer is in the window.
    pointer: Point<Pixels>,
    /// Each target, as one split has one orientation: all four for a page alone, else the two
    /// along its split's. For each, whether the tab is over it, since when, and how far it had
    /// grown toward its size under the tab then.
    targets: Vec<(Edge, bool, Instant, f32)>,
}

impl OverPage {
    /// The target the tab is over.
    fn edge(&self) -> Option<Edge> {
        self.targets.iter().find(|(_, under, ..)| *under).map(|(edge, ..)| *edge)
    }
}

impl Shell {
    /// The tab a dragged `tab` would join in a split view on the page: the one shown, or for the
    /// tab shown alone, the one shown before it (Dia). None where the page has no targets for it:
    /// a favorite, a folder, a split view's row, a tab of the split shown, or a split shown full.
    pub(super) fn split_partner(&self, tab: u64) -> Option<u64> {
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
        let edges = match (self.panes(with).len() > 1, self.stacked(with)) {
            (false, _) => vec![Edge::Left, Edge::Right, Edge::Top, Edge::Bottom],
            (true, false) => vec![Edge::Left, Edge::Right],
            (true, true) => vec![Edge::Top, Edge::Bottom],
        };
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
                targets: edges.into_iter().map(|edge| (edge, false, now, 0.)).collect(),
            }
        });
        over.pointer = position;
        let edge = over.edge();
        for target in &mut over.targets {
            let (at, under, since, from) = *target;
            // Under once the pointer is in the grown target, until it leaves the one under it.
            let bounds = match under {
                true => under_bounds(card, at, position),
                false => target_bounds(card, at, grown_at(at)),
            };
            if bounds.contains(&position) != under {
                *target = (at, !under, now, grown(under, since, from));
            }
        }
        if over.edge() != edge {
            // check.sh reads this line.
            log!("browser: drop target {}", over.edge().map_or("none", Edge::name));
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
        let (tab, with, edge) = (drag.tab, over.with, over.edge());
        let alone = self.panes(with).len() == 1;
        // The release that ends the drag isn't a click.
        self.dragged = true;
        self.settle_drag();
        if let Some(edge) = edge {
            self.browser.split_with(tab, with, edge.first(), cx);
            // A page alone has every target, and the top and bottom ones stack its new split.
            if alone && edge.stacked() {
                self.browser.set_split_stacked(with, true, cx);
            }
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
        let palette = self.palette;
        let fill = self.window_fill().blend(palette.plate(TARGET_PLATE));
        let t = over.since.elapsed().as_secs_f32();
        let growing =
            over.targets.iter().any(|(_, _, since, _)| since.elapsed().as_secs_f32() < TARGET_GROW);
        if growing || t < TARGET_SLIDE + TARGET_GROW {
            window.request_animation_frame();
        }
        let mut targets: Vec<_> = over
            .targets
            .iter()
            .map(|&(edge, under, since, from)| (edge, grown(under, since, from)))
            .collect();
        // The one growing under the tab goes over the others, which it can reach.
        targets.sort_by(|a, b| a.1.total_cmp(&b.1));
        let targets = targets.into_iter().map(|(edge, way)| {
            let under = under_bounds(card, edge, over.pointer);
            let bounds = mix_bounds(arriving(card, edge, t), under, way);
            target(bounds, card, edge, way, fill, palette)
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
            .font_family(crate::face::UI)
            .child(page)
            .child(picture(over.pointer, self.window_fill(), self.palette, icon, title))
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

/// The target at `edge`'s rest and grown sizes: `(across, along, inset)`.
fn rest_at(edge: Edge) -> (f32, f32, f32) {
    if edge.stacked() { STACK_REST } else { TARGET_REST }
}

fn grown_at(edge: Edge) -> (f32, f32, f32) {
    if edge.stacked() { STACK_GROWN } else { TARGET_GROWN }
}

/// The target at `edge`'s bounds in `card`: `(across, along, inset)`, its size across the card's
/// edge and along it as fractions of the card's, and how far in it is from that edge, centred
/// along it.
fn target_bounds(card: Bounds<Pixels>, edge: Edge, at: (f32, f32, f32)) -> Bounds<Pixels> {
    let (across, along, inset) = at;
    let (width, height) = if edge.stacked() { (along, across) } else { (across, along) };
    let size = size(card.size.width * width, card.size.height * height);
    let centred = card.center() - point(size.width / 2., size.height / 2.);
    let origin = match edge {
        Edge::Left => point(card.left() + px(inset), centred.y),
        Edge::Right => point(card.right() - px(inset) - size.width, centred.y),
        Edge::Top => point(centred.x, card.top() + px(inset)),
        Edge::Bottom => point(centred.x, card.bottom() - px(inset) - size.height),
    };
    Bounds::new(origin, size)
}

/// A target under the tab: bigger, its centre leaning across the card toward the pointer.
fn under_bounds(card: Bounds<Pixels>, edge: Edge, pointer: Point<Pixels>) -> Bounds<Pixels> {
    let grown = target_bounds(card, edge, grown_at(edge));
    let (across, along) = if edge.stacked() { STACK_UNDER } else { TARGET_UNDER };
    let (width, height) = if edge.stacked() { (along, across) } else { (across, along) };
    let size = size(card.size.width * width, card.size.height * height);
    let lean = |from: Pixels, to: Pixels| from + (to - from) * TARGET_LEAN;
    let mut centre = card.center();
    if edge.stacked() {
        centre.y = lean(grown.center().y, pointer.y);
    } else {
        centre.x = lean(grown.center().x, pointer.x);
    }
    Bounds::new(centre - point(size.width / 2., size.height / 2.), size)
}

/// Where a target is `t` seconds after the tab came over the page: sliding in at its first size
/// from outside the card's edge, then growing.
fn arriving(card: Bounds<Pixels>, edge: Edge, t: f32) -> Bounds<Pixels> {
    let rest = target_bounds(card, edge, rest_at(edge));
    if t < TARGET_SLIDE {
        let away = 1. - ease_out(t / TARGET_SLIDE);
        let inset = px(rest_at(edge).2);
        let origin = match edge {
            Edge::Left => rest.origin - point((rest.size.width + inset) * away, px(0.)),
            Edge::Right => rest.origin + point((rest.size.width + inset) * away, px(0.)),
            Edge::Top => rest.origin - point(px(0.), (rest.size.height + inset) * away),
            Edge::Bottom => rest.origin + point(px(0.), (rest.size.height + inset) * away),
        };
        return Bounds::new(origin, rest.size);
    }
    let grown = target_bounds(card, edge, grown_at(edge));
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
    edge: Edge,
    way: f32,
    fill: Hsla,
    palette: Palette,
) -> impl IntoElement {
    let accent = Hsla::from(palette.accent);
    let fill = fill.blend(accent.opacity(TARGET_ACCENT * way));
    let mix = |from: Hsla, to: Hsla| from.opacity(1. - way).blend(to.opacity(way));
    let text = mix(palette.text(TARGET_TEXT), palette.text(1.));
    let dashes = mix(palette.faint(TARGET_DASHES), accent);
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
        .border_color(palette.faint(TARGET_EDGE))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(TITLE_GAP))
        .child(line)
        .child(half_glyph(edge, text))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .child(match edge {
                    Edge::Left => "Add left split",
                    Edge::Right => "Add right split",
                    Edge::Top => "Add top split",
                    Edge::Bottom => "Add bottom split",
                }),
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

/// A ≈ 19 pt rectangle with the half at `edge` filled.
fn half_glyph(edge: Edge, color: Hsla) -> impl IntoElement {
    let half = div().absolute().bg(color);
    let half = match edge {
        Edge::Left => half.top_0().bottom_0().w(px(8.)).left_0(),
        Edge::Right => half.top_0().bottom_0().w(px(8.)).right_0(),
        Edge::Top => half.left_0().right_0().h(px(6.)).top_0(),
        Edge::Bottom => half.left_0().right_0().h(px(6.)).bottom_0(),
    };
    div()
        .relative()
        .w(px(19.))
        .h(px(15.))
        .rounded(px(HALF_GLYPH_RADIUS))
        .border(px(1.5))
        .border_color(color)
        .overflow_hidden()
        .child(half)
}

/// The dragged tab over the page: a picture of a window centred on `pointer`, its page showing the
/// tab's favicon and title.
fn picture(
    pointer: Point<Pixels>,
    fill: Hsla,
    palette: Palette,
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
        .bg(fill.blend(palette.plate(TARGET_PLATE)))
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
                .text_size(px(text::SMALL))
                .text_color(palette.text(TARGET_TEXT))
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
        .border_color(Level::Popover.rim_in(&palette))
        .shadow(Level::Popover.shadow())
        .child(div().absolute().left(px(8.)).top(px(6.)).flex().gap(px(4.)).children(lights))
        .child(page)
}
