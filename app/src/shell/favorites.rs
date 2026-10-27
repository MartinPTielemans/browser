//! The favorites' tiles over the list, which scroll with it once they're taller than
//! 40 % of the window (docs/ux-spec.md › Favorites).

use std::{collections::HashMap, rc::Rc, time::Instant};

use gpui::{
    Context, Hsla, MouseButton, MouseDownEvent, Role, canvas, div, hsla, img, prelude::*, px,
    rgba,
};

use crate::tooltip::{self, Anchor, Tip};

use super::{
    access::Accessible,
    Curve, FAVICON_DRAWN, FAVICON_SIZE, FAVICON_X, FOLDER_TITLE_X, FOOTER_HEIGHT, HAIRLINE,
    HOVERED, LIST_TOP,
    PLATE_INSET, Place, Row, SELECTED, SLEEPING_FAVICON, Shell, Slot, TALL_FAVORITES, TILE,
    TILE_EDGE, TILE_GAP, TILE_HEIGHT, TILE_HOVERED, TILE_MIN, TILE_RADIUS, TILE_RING,
    TILE_UNTHEMED, TITLE_END_INSET, TITLE_GAP, TURN_TITLE_FADE, TileTheme, tile_theme::RING_WIDTH,
    title,
};

/// Who shares the favorites, as Settings lists the choices. Each is the core's BC_FAVORITES_ value
/// of the same name, which the bin checks at compile time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FavoritesScope {
    /// The same sites in every Space, each profile opening them in its own tabs.
    Every,
    /// The Spaces of one profile share them, as in Arc.
    #[default]
    Profile,
    /// Every Space has its own.
    Space,
}

impl FavoritesScope {
    pub const ALL: [Self; 3] = [Self::Every, Self::Profile, Self::Space];

    pub fn title(self) -> &'static str {
        match self {
            Self::Every => "In Every Space",
            Self::Profile => "In Each Profile's Spaces",
            Self::Space => "In Each Space",
        }
    }

    pub fn to_core(self) -> i32 {
        match self {
            Self::Every => 0,
            Self::Profile => 1,
            Self::Space => 2,
        }
    }

    /// The scope the core reports. One it doesn't know is the default.
    pub fn from_core(scope: i32) -> Self {
        Self::ALL.into_iter().find(|each| each.to_core() == scope).unwrap_or_default()
    }
}

impl Shell {
    /// The favorites' tiles under the URL field, which stay put while the Spaces' pages move under
    /// them, and slide to their places as tiles come, go and make room for a dragged tab. Taller
    /// than TALL_FAVORITES of the window, they scroll with the list, under the URL field.
    pub(super) fn tiles(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let drawn = self.drawn_tiles();
        let dragged = self.tile_ghost().map(|(tab, _)| tab);
        let scrolled = self.tiles_scrolled();
        let tiles: Vec<_> = self
            .tile_tabs()
            .into_iter()
            .filter(|&tab| Some(tab) != dragged)
            .map(|tab| {
                let [left, top, width] = drawn[&tab];
                let tile = self.tile(tab, cx).absolute().left(px(left));
                tile.top(px(top - scrolled)).w(px(width))
            })
            .collect();
        (!tiles.is_empty()).then(|| {
            div()
                .id("favorites")
                .when(self.a11y, |tiles| tiles.role(Role::TabList).aria_label("Favorites"))
                .absolute()
                .top(px(LIST_TOP))
                .left_0()
                .w(px(self.width()))
                .when(self.tiles_scroll(), |tiles| {
                    tiles.bottom(px(FOOTER_HEIGHT)).overflow_hidden()
                })
                .children(tiles)
        })
    }

    /// A tab dragged over the favorites, as a tile where the pointer has it, over the rest of the
    /// sidebar.
    pub(super) fn dragged_tile(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let (tab, [left, top, width]) = self.tile_ghost()?;
        if self.turning_shape().is_some() {
            return None;
        }
        let tile = self.tile(tab, cx).absolute().left(px(left));
        Some(tile.top(px(self.tiles_top() + top)).w(px(width)))
    }

    /// A tab dragged across the favorites' edge, mid-turn between its row and its tile: the plate
    /// takes the tile's size, corners and colours, the title fades over the first part of the
    /// turn, and the favicon goes to the middle. A turn the other way reverses it.
    pub(super) fn turning_drag(&self) -> Option<impl IntoElement + use<>> {
        let (tab, [left, top, width, height, radius], turned) = self.turning_shape()?;
        let selected = tab == self.selected && !self.new_tab;
        let row_fill = rgba(if selected { SELECTED } else { HOVERED });
        let row_fill = self.window_fill().blend(row_fill.into());
        let theme = selected.then(|| self.tile_theme(tab)).flatten();
        let (shown, ring_width, ring) = shown_look(theme.as_deref());
        let tile_fill = match selected {
            true => shown,
            false => hsla(0., 0., 1., TILE_HOVERED),
        };
        let (ring_width, ring) = match selected {
            true => (ring_width, ring),
            false => (HAIRLINE, rgba(TILE_EDGE).into()),
        };
        let row_icon = FAVICON_X - PLATE_INSET;
        let icon_left = row_icon + ((width - FAVICON_SIZE) / 2. - row_icon) * turned;
        let row_width = self.width() - 2. * PLATE_INSET;
        let title_left = icon_left + FAVICON_SIZE + TITLE_GAP;
        let template = match theme.as_deref() {
            Some(TileTheme::Flat { template, .. }) => Some(template.clone()),
            _ => None,
        };
        // A one-colour icon's tile draws it as a white template, which fades in over it.
        let favicon = 1. - if template.is_some() { turned } else { 0. };
        let icon = div()
            .absolute()
            .left(px(icon_left))
            .top(px((height - FAVICON_SIZE) / 2.))
            .size(px(FAVICON_SIZE))
            .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(centred().opacity(favicon).child(self.tab_icon(tab)))
            .children(template.map(|template| {
                centred().opacity(turned).child(img(template).size(px(FAVICON_DRAWN)))
            }));
        Some(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width))
                .h(px(height))
                .rounded(px(radius))
                .overflow_hidden()
                .bg(self.window_fill().blend(tile_fill))
                .children(theme.as_deref().and_then(TileTheme::layers).map(|layers| {
                    div().absolute().size_full().opacity(turned).child(layers)
                }))
                .child(div().absolute().size_full().bg(row_fill).opacity(1. - turned))
                .child(
                    div()
                        .absolute()
                        .left(px(title_left))
                        .w(px((row_width - title_left - TITLE_END_INSET).max(0.)))
                        .h_full()
                        .flex()
                        .items_center()
                        .opacity((1. - turned / TURN_TITLE_FADE).max(0.))
                        .child(title(self.title(tab), selected)),
                )
                .child(icon)
                // The ring over the rest, as the tile's border would shift the favicon in by its
                // width.
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .rounded(px(radius))
                        .border(px(ring_width))
                        .border_color(ring.opacity(turned)),
                ),
        )
    }

    /// The tiles in their order: the favorites, with a dragged tab out of its place, and in the
    /// place it would go to among them.
    fn tile_tabs(&self) -> Vec<u64> {
        let mut tabs = self.favorites.clone();
        if let Some(drag) = self.drag.as_ref().filter(|drag| drag.moving) {
            tabs.retain(|&tab| tab != drag.tab);
            if let Place::Favorite(index) = drag.to {
                tabs.insert(index.min(tabs.len()), drag.tab);
            }
        }
        tabs
    }

    /// Where `count` tiles go, down from the favorites' top: each one's left, top and width. A row
    /// has as many as fit, each as wide as the others, and fewer tiles share it.
    pub(super) fn tile_places(&self, count: usize) -> Vec<[f32; 3]> {
        let room = self.width() - 2. * PLATE_INSET + TILE_GAP;
        let fit = (room / (TILE_MIN + TILE_GAP)).floor().max(1.) as usize;
        let columns = fit.min(count).max(1);
        let width = room / columns as f32 - TILE_GAP;
        (0..count)
            .map(|index| {
                let (column, row) = ((index % columns) as f32, (index / columns) as f32);
                [PLATE_INSET + column * (width + TILE_GAP), row * (TILE_HEIGHT + TILE_GAP), width]
            })
            .collect()
    }

    /// Where each tile is drawn, down from the favorites' top: at its place or on its way there,
    /// and a dragged one where the pointer has it.
    pub(super) fn drawn_tiles(&self) -> HashMap<u64, [f32; 3]> {
        let tabs = self.tile_tabs();
        let places = self.tile_places(tabs.len());
        let mut drawn: HashMap<u64, [f32; 3]> = tabs
            .into_iter()
            .zip(places)
            .map(|(tab, place)| {
                let way = self.tile_slides.get(&tab).map_or(0., |&(_, start, curve)| {
                    curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| 1. - done)
                });
                let from = self.tile_slides.get(&tab).map_or([0.; 3], |&(from, ..)| from);
                (tab, std::array::from_fn(|i| place[i] + from[i] * way))
            })
            .collect();
        drawn.extend(self.tile_ghost());
        drawn
    }

    /// A tab dragged over the favorites, and its tile where the pointer has it.
    fn tile_ghost(&self) -> Option<(u64, [f32; 3])> {
        let drag = self.drag.as_ref().filter(|drag| drag.moving)?;
        if !matches!(drag.to, Place::Favorite(_)) {
            return None;
        }
        let width = self.tile_places(self.tile_tabs().len()).first()?[2];
        let centre = drag.pointer - drag.grab.1;
        let top = centre.y - self.tiles_top() - TILE_HEIGHT / 2.;
        Some((drag.tab, [centre.x - width / 2., top, width]))
    }

    /// Where the tiles and the list's top are drawn, for motion from there.
    pub(super) fn drawn(&self) -> (HashMap<u64, [f32; 3]>, f32) {
        (self.drawn_tiles(), self.drawn_list_top())
    }

    /// Starts the tiles and the list's top from where they were drawn `before` to their places.
    pub(super) fn move_tiles(&mut self, before: (HashMap<u64, [f32; 3]>, f32), curve: Curve) {
        let now = Instant::now();
        let tabs = self.tile_tabs();
        let places = self.tile_places(tabs.len());
        self.tile_slides = tabs
            .into_iter()
            .zip(places)
            .filter_map(|(tab, place)| {
                let was = before.0.get(&tab)?;
                let from: [f32; 3] = std::array::from_fn(|i| was[i] - place[i]);
                (from != [0.; 3]).then_some((tab, (from, now, curve)))
            })
            .collect();
        let from = before.1 - self.list_top();
        self.list_slide = (from != 0.).then_some((from, now, curve));
    }

    /// Where the Space's pages start with `count` tiles: under them, else under the URL field.
    pub(super) fn list_top_with(&self, count: usize) -> f32 {
        let rows = self.tile_places(count).last().map_or(0., |&[_, top, _]| top + TILE_HEIGHT);
        LIST_TOP + if count > 0 { rows + TILE_GAP } else { 0. }
    }

    /// Where the Space's pages start: under the tiles.
    fn list_top(&self) -> f32 {
        self.list_top_with(self.tile_tabs().len())
    }

    /// Where the Space's pages start as drawn: at list_top, or on their way there.
    pub(super) fn drawn_list_top(&self) -> f32 {
        self.list_top()
            + self.list_slide.map_or(0., |(from, start, curve)| {
                curve.at(start.elapsed().as_secs_f32()).map_or(0., |done| from * (1. - done))
            })
    }

    /// Whether the tiles scroll with the list, as they do while they're taller than
    /// TALL_FAVORITES of the window.
    pub(super) fn tiles_scroll(&self) -> bool {
        self.list_top() - LIST_TOP > TALL_FAVORITES * f32::from(self.viewport.height)
    }

    /// How far the tiles have gone up with the list.
    pub(super) fn tiles_scrolled(&self) -> f32 {
        match self.tiles_scroll() {
            true => -f32::from(self.list_scroll.offset().y),
            false => 0.,
        }
    }

    /// Where the tiles' top is drawn.
    pub(super) fn tiles_top(&self) -> f32 {
        LIST_TOP - self.tiles_scrolled()
    }

    /// Where the Space's pages are drawn from: under the tiles, or under the URL field while the
    /// tiles scroll with them.
    pub(super) fn pages_top(&self) -> f32 {
        if self.tiles_scroll() { LIST_TOP } else { self.drawn_list_top() }
    }

    /// Where the sidebar under the tiles starts, as they're drawn.
    pub(super) fn under_tiles(&self) -> f32 {
        (self.list_top() - self.tiles_scrolled()).max(LIST_TOP)
    }

    /// The room a page leaves above its rows for the tiles that scroll with it.
    pub(super) fn tiles_room(&self) -> f32 {
        self.drawn_list_top() - self.pages_top()
    }

    /// How `tab`'s favicon themes its tile while it's shown: none for an emoji, which GPUI can't
    /// give the pixels of, and for a page without a favicon or one with no opaque pixel. Made the
    /// first time it's drawn, then kept with the tab until its favicon changes.
    fn tile_theme(&self, tab: u64) -> Option<Rc<TileTheme>> {
        let tab = self.tabs.get(&tab)?;
        let favicon = tab.favicon.as_ref().filter(|_| tab.page.icon.is_empty())?;
        let mut made = tab.theme.borrow_mut();
        match &*made {
            Some((id, theme)) if *id == favicon.id => theme.clone(),
            _ => {
                let theme = TileTheme::new(favicon).map(Rc::new);
                *made = Some((favicon.id, theme.clone()));
                theme
            }
        }
    }

    /// The tooltip over a favorite's tile, which has only an icon, or a folder's row: its name,
    /// after a moment, hung from the tile or row, wherever the pointer is on it: a tile's square
    /// corner is under its middle. The pointer leaving, or a drag or a menu, ends it; other rows have none (the hover card is Dia's).
    pub(super) fn hover_tooltip(&self, row: Row, hovered: bool, cx: &mut Context<Self>) {
        let Row::Tab(tab) = row else {
            return;
        };
        // A tile's square corner is under its middle, a folder's text over its title.
        let (name, title) = if self.is_folder(tab) {
            (self.folder_name(tab), Some(FOLDER_TITLE_X))
        } else if self.favorites.contains(&tab) {
            (self.title(tab), None)
        } else {
            return;
        };
        let busy = self.dragging().is_some() || self.menu_row.is_some();
        let frame = self.tip_frames.borrow().get(&tab).copied();
        let tip = (hovered && !busy)
            .then(|| tooltip::text_for(&name).map(Tip::named))
            .flatten()
            .zip(frame)
            .map(|(text, frame)| match title {
                Some(title) => (text, Anchor::titled(frame, title, self.viewport)),
                None => (text, Anchor::under(frame, self.viewport)),
            });
        self.hover_tip(tip, cx);
    }

    /// Asks for `tip` from a hover handler, or ends the tooltip with none. One the pointer has come
    /// onto is asked for after the move, since the handler of the tile it left, which ends the
    /// tooltip, may run after this one in it.
    pub(super) fn hover_tip(&self, tip: Option<(Tip, Anchor)>, cx: &mut Context<Self>) {
        match tip {
            Some(tip) => {
                let browser = self.browser.clone();
                cx.defer(move |cx| browser.tooltip(Some(tip), cx));
            }
            None => self.browser.tooltip(None, cx),
        }
    }

    /// Keeps the frame `tab`'s tile or row is drawn in, `outset` past its inside, for its tooltip
    /// to hang from, since GPUI gives a hover handler no bounds.
    pub(super) fn framed(&self, tab: u64, outset: f32) -> impl IntoElement + use<> {
        let frames = self.tip_frames.clone();
        canvas(
            move |bounds, _, _| {
                frames.borrow_mut().insert(tab, bounds);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset(px(-outset))
    }

    /// A favorite's tile: its favicon on a plate, which the tab shown has filled and ringed by its
    /// icon (tile_theme.rs). A click shows it, a double-click takes it back to its URL (Dia), and a
    /// right-click shows its menu.
    fn tile(&self, tab: u64, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let selected = tab == self.selected && !self.new_tab;
        // Full for a dragged tile, which covers what it passes, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(dragged) => (dragged == Slot::Tab(tab)).then_some(1.),
            None => self.hover_fill(Row::Tab(tab)),
        };
        let fill = hsla(0., 0., 1., TILE + (TILE_HOVERED - TILE) * hover.unwrap_or(0.));
        let theme = selected.then(|| self.tile_theme(tab)).flatten();
        let (shown, ring_width, ring) = shown_look(theme.as_deref());
        let fill = match dragging == Some(Slot::Tab(tab)) {
            true => self.window_fill().blend(if selected { shown } else { fill }),
            false if selected => shown,
            false => fill,
        };
        let icon = div()
            .size(px(FAVICON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(match theme.as_deref() {
                Some(TileTheme::Flat { template, .. }) => {
                    img(template.clone()).size(px(FAVICON_DRAWN)).into_any_element()
                }
                _ => self.tab_icon(tab),
            });
        div()
            .id(("tile", tab))
            .when(self.a11y, |tile| {
                tile.role(Role::Tab)
                    .accessibility_id(format!("tile-{tab}"))
                    .aria_label(self.tab_label(tab))
                    .aria_selected(selected)
                    .pressed_by_mouse_down(false, true)
            })
            .h(px(TILE_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(TILE_RADIUS))
            .border(px(if selected { ring_width } else { HAIRLINE }))
            .border_color(if selected { ring } else { rgba(TILE_EDGE).into() })
            .bg(fill)
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(Row::Tab(tab), *hovered, cx);
                shell.hover_tooltip(Row::Tab(tab), *hovered, cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.browser.tooltip(None, cx);
                    // The first click showed it.
                    if event.click_count == 2 {
                        log!("browser: command back-to-pinned-url via favorite");
                        return shell.browser.back_to_pinned_url(tab, cx);
                    }
                    shell.press_tile(tab, event.position)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(tab, event.position, cx)
                    }
                }),
            )
            // Unless it ended a drag.
            .on_click(cx.listener(move |shell, _, _, cx| {
                if shell.dragged || shell.dragging().is_some() {
                    return;
                }
                log!("browser: command select via favorite");
                shell.select(tab, cx)
            }))
            .children(theme.as_deref().and_then(TileTheme::layers))
            .child(icon)
            .child(self.framed(tab, if selected { ring_width } else { HAIRLINE }))
    }
}

/// The selected tile's fill, ring width and ring: a colourful icon's ring is drawn with the blurred
/// icon, a one-colour icon's is a border, and without a theme it's Dia's default.
fn shown_look(theme: Option<&TileTheme>) -> (Hsla, f32, Hsla) {
    match theme {
        Some(TileTheme::Blur { .. }) => (rgba(SELECTED).into(), 0., hsla(0., 0., 0., 0.)),
        Some(&TileTheme::Flat { fill, ring, .. }) => {
            (rgba(fill).into(), RING_WIDTH, rgba(ring).into())
        }
        None => (rgba(TILE_UNTHEMED).into(), HAIRLINE, rgba(TILE_RING).into()),
    }
}

/// A favicon's square, its icon in the middle.
fn centred() -> gpui::Div {
    div().absolute().size_full().flex().items_center().justify_center()
}

#[cfg(test)]
mod tests;
