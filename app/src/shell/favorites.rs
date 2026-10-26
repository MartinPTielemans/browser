//! The favorites' tiles over the list, which scroll with it once they're taller than
//! 40 % of the window (docs/ux-spec.md › Favorites).

use std::{collections::HashMap, time::Instant};

use gpui::{Context, MouseButton, MouseDownEvent, div, hsla, prelude::*, px, rgba};

use super::{
    Curve, FAVICON_SIZE, FOOTER_HEIGHT, HAIRLINE, LIST_TOP, PLATE_INSET, Place, Row, SELECTED,
    SLEEPING_FAVICON, Shell, Slot, TALL_FAVORITES, TILE, TILE_EDGE, TILE_GAP, TILE_HEIGHT,
    TILE_HOVERED, TILE_MIN, TILE_RADIUS, TILE_RING,
};

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
        let tile = self.tile(tab, cx).absolute().left(px(left));
        Some(tile.top(px(self.tiles_top() + top)).w(px(width)))
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

    /// A favorite's tile: its favicon on a plate, which the tab shown has filled and ringed. A
    /// click shows it, a double-click takes it back to its URL (Dia), and a right-click shows its
    /// menu.
    fn tile(&self, tab: u64, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let selected = tab == self.selected && !self.new_tab;
        // Full for a dragged tile, which covers what it passes, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(dragged) => (dragged == Slot::Tab(tab)).then_some(1.),
            None => self.hover_fill(Row::Tab(tab)),
        };
        let fill = hsla(0., 0., 1., TILE + (TILE_HOVERED - TILE) * hover.unwrap_or(0.));
        let fill = match dragging == Some(Slot::Tab(tab)) {
            true => self.window_fill().blend(if selected { rgba(SELECTED).into() } else { fill }),
            false if selected => rgba(SELECTED).into(),
            false => fill,
        };
        let icon = div()
            .size(px(FAVICON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .when(self.page(tab).unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(self.tab_icon(tab));
        div()
            .id(("tile", tab))
            .h(px(TILE_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(TILE_RADIUS))
            .border(px(HAIRLINE))
            .border_color(rgba(if selected { TILE_RING } else { TILE_EDGE }))
            .bg(fill)
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(Row::Tab(tab), *hovered, cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
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
            .child(icon)
    }
}
