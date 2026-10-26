//! The list's rows: a tab's, a split view's and a folder's, the Space's name, and the
//! divider with Clear and New Tab.

use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, ClickEvent, Context, Hsla, MouseButton, MouseDownEvent,
    SharedString, Transformation, div, percentage, prelude::*, px, rgb, rgba, svg,
};

use super::{
    CLEAR, CLEAR_ARROW, CLOSE_HOVERED, CLOSE_RIGHT, CLOSE_SIZE, CancelName, Closing, Command,
    DIVIDER, EMOJI_SIZE, FAVICON_DRAWN, FAVICON_SIZE, FAVICON_X, FOLDER_INDENT, FORM_TEXT, HOVERED,
    KeepName, LOADING_ROW_MIN, LOADING_SIZE, LOADING_TURN, NAV, NAV_HOVERED, NAV_RADIUS, Naming,
    PLATE_HEIGHT, PLATE_INSET, PRESSED, ROW_FILL, ROW_HEIGHT, Row, SECONDARY, SEGMENT_END,
    SEGMENT_FAVICON_X, SEGMENT_LINE, SEGMENT_TITLE_GAP, SELECTED, SLEEPING_FAVICON, SPEAKER_DRAWN,
    SPEAKER_GAP, SPEAKER_SIZE, Shell, Slot, Swapped, TITLE_END_INSET, clear_label, cubic_bezier,
    divider_box, favicon, folder_icon, name_box, new_tab_label, plate, row_box, row_spring,
    selected_plate, space_icon, split_plate,
    swap::{ARROW_DOWN, NONE, folder_glyph},
    title,
};

impl Shell {
    /// Closes every folder of the Space shown, all on one fold.
    pub(super) fn collapse_folders(&mut self, cx: &mut Context<Self>) {
        let Some(space) = self.shown_space() else {
            return;
        };
        let folders = space.folders.folders.keys().copied();
        let open: Vec<u64> = folders.filter(|&folder| space.is_open(folder)).collect();
        // The core reports each, and the first report has them all closed.
        self.folding = !open.is_empty();
        for folder in open {
            self.browser.set_folder_open(folder, false, cx);
        }
    }

    /// Opens or closes the folder, unless the click ended a drag or renames it.
    fn click_folder(&mut self, folder: u64, cx: &mut Context<Self>) {
        let renaming = self.renaming.as_ref().map(|renaming| renaming.naming);
        if self.dragged || self.dragging().is_some() || renaming == Some(Naming::Folder(folder)) {
            return;
        }
        let open = self.shown_space().is_some_and(|space| space.is_open(folder));
        log!("browser: command toggle-folder via sidebar");
        self.folding = true;
        self.browser.set_folder_open(folder, !open, cx);
    }

    /// A sidebar row: the plate, with the favicon slot and the label.
    fn row(
        &self,
        row: Row,
        content: gpui::Div,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let selected = row == self.selected_row();
        let (id, slot) = match row {
            Row::NewTab => (("new-tab", 0), Slot::NewTab),
            Row::Tab(tab) => (("tab", tab), Slot::Tab(tab)),
        };
        // Full for a dragged row, which the pointer holds, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(dragged) => (dragged == slot).then_some(1.),
            None => self.hover_fill(row),
        };
        // A new tab's plate fades in, and rows that come together fade in whole.
        let opened = self.opened.as_ref().filter(|opened| {
            matches!(row, Row::Tab(tab) if opened.tabs.contains(&tab))
        });
        let (row_in, plate_in) = opened.map_or((1., 1.), |opened| {
            let done = opened.done().unwrap_or(1.).clamp(0., 1.);
            if opened.whole { (done, 1.) } else { (1., done) }
        });
        // In from its folders, or where a dragged row would go.
        let depth = match (row, &self.drag) {
            (_, Some(drag)) if dragging == Some(slot) => drag.depth,
            (Row::Tab(tab), _) => self.depth(tab),
            (Row::NewTab, _) => 0,
        };
        row_box()
            .pl(px(PLATE_INSET + depth as f32 * FOLDER_INDENT))
            .id(id)
            .group("row")
            .opacity(row_in)
            // A dragged row is drawn over the list, from `list`.
            .top(px(if dragging == Some(slot) { 0. } else { self.slide(slot) }))
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, window, cx| {
                shell.hover(row, *hovered, cx);
                shell.hover_tooltip(row, *hovered, window.mouse_position(), cx)
            }))
            .child(
                content
                    .when(selected, |plate| selected_plate(plate, plate_in))
                    .id("plate")
                    .when_some(hover.filter(|_| !selected), |plate, hover| {
                        plate.bg(Hsla::from(rgba(HOVERED)).opacity(hover))
                    })
                    .when(!selected && dragging != Some(slot), |plate| {
                        // GPUI keeps an element's pressed state only once a listener gives it a
                        // hitbox.
                        plate
                            .on_mouse_down(MouseButton::Left, |_, _, _| {})
                            .group_active("row", |style| style.bg(rgba(PRESSED)))
                    })
                    // A dragged row covers the rows it passes, in the colours it has in place.
                    .when(dragging == Some(slot), |plate| {
                        let fill = if selected { SELECTED } else { HOVERED };
                        plate.bg(self.window_fill().blend(rgba(fill).into()))
                    }),
            )
    }

    /// How full `row`'s hover fill is: full under the pointer or with its menu up, and fading once
    /// it's left. None for none.
    pub(super) fn hover_fill(&self, row: Row) -> Option<f32> {
        if self.hovered == Some(row) || self.menu_row.is_some_and(|tab| row == Row::Tab(tab)) {
            return Some(1.);
        }
        self.unhovered.iter().find(|&&(old, _)| old == row).map(|(_, start)| {
            let t = (start.elapsed().as_secs_f32() / ROW_FILL).min(1.);
            1. - cubic_bezier(0., 0., 0.58, 1., t)
        })
    }

    /// A closed tab's row where it was, without the selected fill it may have had, fading as the
    /// rows below slide over it.
    pub(super) fn closing_row(&self, row: &Closing, room: f32) -> impl IntoElement + use<> {
        let done = row_spring(row.start.elapsed().as_secs_f32()).unwrap_or(1.);
        div()
            .absolute()
            .top(px(room + row.top))
            .left_0()
            .w(px(self.width()))
            .h(px(ROW_HEIGHT))
            .px(px(PLATE_INSET))
            .pl(px(PLATE_INSET + row.indent))
            .py(px((ROW_HEIGHT - PLATE_HEIGHT) / 2.))
            .opacity((1. - done).clamp(0., 1.))
            .child(plate(
                match row.folder {
                    Some(open) => folder_icon(open),
                    None if !row.icon.is_empty() => emoji(row.icon.clone()),
                    None => favicon(row.favicon.clone()),
                },
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .pr(px(TITLE_END_INSET))
                    .child(title(row.title.clone(), false)),
            ))
    }

    /// A tab's row, its split view's for the split's first tab, or a folder's.
    pub(super) fn tab_row(&self, tab: u64, cx: &mut Context<Self>) -> AnyElement {
        if self.is_folder(tab) {
            return self.folder_row(tab, cx).into_any_element();
        }
        let panes = self.panes(tab);
        match panes.len() > 1 {
            true => self.split_row(panes, cx).into_any_element(),
            false => self.single_row(tab, cx).into_any_element(),
        }
    }

    /// A folder's row: its icon, open or closed, and its name. A click opens or closes it, a
    /// double-click renames it and a right-click shows its menu.
    fn folder_row(&self, folder: u64, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let open = self.shown_space().is_some_and(|space| space.is_open(folder));
        let renaming =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Folder(folder));
        let label = match renaming {
            Some(renaming) => self.naming_field(renaming, cx),
            None => title(self.folder_name(folder), false).pr(px(TITLE_END_INSET)),
        };
        let renaming = renaming.is_some();
        let icon = self.swaps.glyph(
            Swapped::Folder(folder),
            folder_glyph(open),
            FAVICON_DRAWN,
            rgba(SECONDARY).into(),
        );
        self.row(Row::Tab(folder), plate(icon, label), cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.browser.tooltip(None, cx);
                    // The first click opened or closed it.
                    if event.click_count == 2 {
                        return shell.rename_folder(folder, cx);
                    }
                    shell.press_row(folder, event.position)
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    if shell.dragging().is_none() {
                        shell.tab_menu(folder, event.position, cx)
                    }
                }),
            )
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_folder(folder, cx)))
        })
    }

    /// The button that closes `tabs`, which shows under the pointer at a row's end.
    fn close_button(&self, tabs: Vec<u64>, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        div()
            .id(("close", tabs[0]))
            .flex_none()
            .ml(px(4.))
            .mr(px(CLOSE_RIGHT))
            .size(px(CLOSE_SIZE))
            .rounded(px(NAV_RADIUS))
            .hover(|style| style.bg(rgba(CLOSE_HOVERED)))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(16.))
            .text_color(rgba(SECONDARY))
            .child("×")
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                log!("browser: command close-tab via sidebar");
                shell.browser.close(tabs[0], true, cx);
            }))
    }

    /// The speaker after `tab`'s favicon while its page plays sound, crossed out while it's muted,
    /// which a click mutes or unmutes (Dia).
    fn speaker(&self, tab: u64, cx: &mut Context<Self>) -> Option<gpui::Stateful<gpui::Div>> {
        let page = self.page(tab);
        let icon = speaker_glyph(page.muted);
        (page.audible || page.muted).then(|| {
            div()
                .id(("speaker", tab))
                .flex_none()
                .mr(px(SPEAKER_GAP))
                .size(px(SPEAKER_SIZE))
                .rounded(px(NAV_RADIUS))
                .hover(|style| style.bg(rgba(CLOSE_HOVERED)))
                .flex()
                .items_center()
                .justify_center()
                .child(self.swaps.glyph(
                    Swapped::Speaker(tab),
                    icon,
                    SPEAKER_DRAWN,
                    rgba(SECONDARY).into(),
                ))
                // Not a double-click's rename.
                .on_mouse_down(MouseButton::Left, |event: &MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        cx.stop_propagation();
                    }
                })
                .on_click(cx.listener(move |shell, _, _, cx| {
                    cx.stop_propagation();
                    log!("browser: command toggle-mute via sidebar");
                    shell.toggle_mute(tab, cx);
                }))
        })
    }

    /// Mutes `tab`, or unmutes it.
    pub(super) fn toggle_mute(&mut self, tab: u64, cx: &mut Context<Self>) {
        if tab != 0 {
            let muted = self.page(tab).muted;
            self.browser.set_muted(tab, !muted, cx);
        }
    }

    /// What stands for `tab`: the emoji the user gave it, else its page's favicon.
    pub(super) fn tab_icon(&self, tab: u64) -> AnyElement {
        match self.tabs.get(&tab) {
            Some(each) if !each.page.icon.is_empty() => emoji(each.page.icon.clone()),
            each => favicon(each.and_then(|each| each.favicon.clone())),
        }
    }

    /// A click on `tab`'s row, or its segment of a split view's, shows it, unless it ended a drag.
    fn click_row(&mut self, tab: u64, cx: &mut Context<Self>) {
        let renaming = self.renaming.as_ref().map(|renaming| renaming.naming);
        if self.dragged || self.dragging().is_some() || renaming == Some(Naming::Tab(tab)) {
            return;
        }
        log!("browser: command select via sidebar");
        self.select(tab, cx)
    }

    /// How wide each segment of a split view's row is, `count` of them `depth` folders in.
    pub(super) fn segment_width(&self, depth: usize, count: usize) -> f32 {
        let plate = self.width() - 2. * PLATE_INSET - depth as f32 * FOLDER_INDENT;
        (plate - (FAVICON_X - PLATE_INSET - SEGMENT_FAVICON_X)) / count as f32
    }

    /// A tab's segment of its split view's row, `width` wide: its favicon and its title, bright if
    /// `focused`, after a line unless it's the `first`.
    pub(super) fn segment(&self, tab: u64, width: f32, first: bool, focused: bool) -> gpui::Div {
        let page = self.page(tab);
        let icon = div()
            .flex_none()
            .size(px(FAVICON_SIZE))
            .mr(px(SEGMENT_TITLE_GAP))
            .flex()
            .items_center()
            .justify_center()
            .when(page.unloaded, |icon| icon.opacity(SLEEPING_FAVICON))
            .child(self.tab_icon(tab));
        let line = div()
            .absolute()
            .left_0()
            .top(px((PLATE_HEIGHT - SEGMENT_LINE) / 2.))
            .w(px(1.))
            .h(px(SEGMENT_LINE))
            .bg(rgba(DIVIDER));
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .items_center()
            .pl(px(SEGMENT_FAVICON_X))
            .pr(px(SEGMENT_END))
            .when(!first, |segment| segment.child(line))
            .child(icon)
            .child(title(self.title(tab), focused))
            .when(page.loading && width >= LOADING_ROW_MIN, |segment| {
                segment.child(self.loading_ring())
            })
    }

    /// `segment`, which a click shows focused, a double-click renames and a right-click shows the
    /// tab's menu for.
    fn split_segment(
        &self,
        tab: u64,
        width: f32,
        first: bool,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        self.segment(tab, width, first, focused)
            .id(("segment", tab))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click selected the tab (Dia).
                    if event.click_count == 2 {
                        cx.stop_propagation();
                        shell.rename_tab(tab, cx);
                    }
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
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_row(tab, cx)))
    }

    /// A split view's row, for its first tab: a segment per tab, left to right. A click on one
    /// shows the split with its page focused, and the close button closes them all.
    fn split_row(&self, panes: Vec<u64>, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let first = panes[0];
        let row_selected = self.selected_row() == Row::Tab(first);
        let hovered = self.hovered == Some(Row::Tab(first));
        // A pinned tab closed already has nothing to close.
        let pinned = self.pinned().contains(&first);
        let closable = panes.iter().any(|&tab| !(self.page(tab).unloaded && pinned));
        let renaming = self.renaming.as_ref().filter(|renaming| {
            matches!(renaming.naming, Naming::Tab(tab) if panes.contains(&tab))
        });
        let content = match renaming {
            Some(renaming) => {
                let Naming::Tab(tab) = renaming.naming else {
                    unreachable!()
                };
                plate(self.tab_icon(tab), self.naming_field(renaming, cx))
            }
            None => {
                let close = (hovered && closable).then(|| self.close_button(panes.clone(), cx));
                let width = self.segment_width(self.depth(first), panes.len());
                let segments: Vec<_> = panes
                    .iter()
                    .enumerate()
                    .map(|(index, &tab)| {
                        let focused = row_selected && tab == self.selected;
                        self.split_segment(tab, width, index == 0, focused, cx)
                    })
                    .collect();
                split_plate(segments).children(close)
            }
        };
        let renaming = renaming.is_some();
        self.row(Row::Tab(first), content, cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, _| {
                    shell.press_row(first, event.position)
                }),
            )
        })
    }

    /// Whether a tab's row has a ✕ to show: a pinned tab closed already has nothing to close.
    pub(super) fn closable(&self, tab: u64) -> bool {
        !(self.page(tab).unloaded && self.pinned().contains(&tab))
    }

    fn single_row(&self, tab: u64, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let page = self.page(tab);
        let loading = page.loading;
        let closable = self.closable(tab);
        let hovered = self.hovered == Some(Row::Tab(tab));
        // Away from its pinned URL, a pinned tab's favicon is a way back.
        let away = page.away_from_pinned_url;
        let back = away && self.hovered_favicon == Some(tab);
        let label = if back { "Back to Pinned URL".into() } else { self.title(tab) };
        let title = title(label, Row::Tab(tab) == self.selected_row());
        // The close button shows under the pointer, and the title truncates before it.
        let close = self.close_button(vec![tab], cx);
        let renaming =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Tab(tab));
        let label = match renaming {
            Some(renaming) => self.naming_field(renaming, cx),
            None => div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .when(!(hovered && closable), |label| label.pr(px(TITLE_END_INSET)))
                .children(self.speaker(tab, cx))
                .child(title)
                .when(hovered && closable, |label| label.child(close))
                .when(!hovered && loading, |label| label.child(self.loading_ring())),
        };
        let renaming = renaming.is_some();
        let icon = match back {
            true => svg()
                .path("icons/arrow-left.svg")
                .size(px(FAVICON_DRAWN))
                .text_color(rgba(NAV))
                .into_any_element(),
            false => self.tab_icon(tab),
        };
        let icon = div()
            .id(("favicon", tab))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .when(page.unloaded && !back, |icon| icon.opacity(SLEEPING_FAVICON))
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                if *hovered {
                    shell.hovered_favicon = Some(tab);
                } else if shell.hovered_favicon == Some(tab) {
                    shell.hovered_favicon = None;
                }
                cx.notify();
            }))
            // The row's click selects the tab after this one.
            .when(away, |icon| {
                icon.on_click(cx.listener(move |shell, _, _, cx| {
                    log!("browser: command back-to-pinned-url via sidebar");
                    shell.browser.back_to_pinned_url(tab, cx);
                }))
            })
            .child(icon);
        let changing =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Icon(tab));
        let icon = match changing {
            Some(changing) => self.icon_field(changing, cx).into_any_element(),
            None => icon.into_any_element(),
        };
        let renaming = renaming || changing.is_some();
        // The field takes the row's clicks while it's up.
        self.row(Row::Tab(tab), plate(icon, label), cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first click selected the tab (Dia).
                    if event.click_count == 2 {
                        return shell.rename_tab(tab, cx);
                    }
                    shell.press_row(tab, event.position)
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
            .on_click(cx.listener(move |shell, _, _, cx| shell.click_row(tab, cx)))
        })
    }

    pub(super) fn name_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let space = self.space;
        // Its icon, or the field changing it, before its name.
        let changing =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::SpaceIcon(space));
        let icon = match changing {
            Some(changing) => Some(space_icon(self.space_icon_field(changing, cx))),
            None => (self.shown_space())
                .filter(|each| !each.icon.is_empty())
                .map(|each| space_icon(each.icon.clone())),
        };
        let row = name_box()
            .id("space-name")
            .block_mouse_except_scroll()
            .when_some(icon, |row, icon| row.child(icon));
        if let Some(renaming) =
            self.renaming.as_ref().filter(|renaming| renaming.naming == Naming::Space(space))
        {
            return row
                .key_context("Naming")
                .on_action(
                    cx.listener(|shell, _: &KeepName, window, cx| shell.keep_name(window, cx)),
                )
                .on_action(
                    cx.listener(|shell, _: &CancelName, window, cx| shell.cancel_name(window, cx)),
                )
                .text_color(rgb(FORM_TEXT))
                .child(renaming.name.clone());
        }
        let name = self.shown_space().map(|space| space.name.clone()).unwrap_or_default();
        row.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    shell.rename_space(space, cx);
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                shell.space_menu(space, event.position, cx)
            }),
        )
        .child(div().truncate().child(name))
    }

    /// The line over Today, and Clear at its end, where the titles end.
    pub(super) fn divider(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let live = self.tools_live();
        let clear = self.tools_shown().then(|| {
            let arrow = if self.clears_last() { ARROW_DOWN } else { NONE };
            let what = Swapped::Clear(self.space);
            let room = self.swaps.toward(what, ARROW_DOWN);
            let room = room.unwrap_or(if arrow == NONE { 0. } else { 1. });
            let glyph = self.swaps.glyph(what, arrow, CLEAR_ARROW, rgba(CLEAR).into());
            // Fading out, it takes no click and doesn't light up.
            let clear = clear_label(glyph, room).id("clear").opacity(self.tools_opacity());
            clear.when(live, |clear| {
                clear
                    .block_mouse_except_scroll()
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    // check.sh waits for this line.
                    .on_hover(|hovered, _, _| log!("browser: clear hovered {}", *hovered as u8))
                    .on_click(
                        cx.listener(|shell, _, _, cx| shell.run(Command::Clear, "sidebar", cx)),
                    )
            })
        });
        divider_box(self.hairline(), clear).top(px(self.slide(Slot::Divider)))
    }

    /// Whether the Space shown shows only its Today tabs, which Clear then closes.
    pub(super) fn clears_last(&self) -> bool {
        self.shown_space().is_some_and(|space| space.clears_last(self.selected))
    }

    /// Starts the arrow coming or going as Clear starts or stops closing the last Today tabs, on
    /// the divider drawn before. Another Space's, or a divider just drawn, has its own at once.
    pub(super) fn step_clear_arrow(&mut self) {
        let arrow = (!self.today().is_empty()).then(|| (self.space, self.clears_last()));
        if arrow == self.drawn_arrow {
            return;
        }
        if let Some((space, is)) = arrow {
            match self.drawn_arrow {
                Some((was_in, was)) if was_in == space => {
                    let glyph = |arrow: bool| if arrow { ARROW_DOWN } else { NONE };
                    self.swaps.start(Swapped::Clear(space), glyph(was), glyph(is), Instant::now());
                }
                _ => self.swaps.end(Swapped::Clear(space)),
            }
        }
        let shows = |arrow: Option<(u64, bool)>| arrow.is_some_and(|(_, shows)| shows);
        if shows(arrow) != shows(self.drawn_arrow) {
            // check.sh waits for this line.
            log!("browser: clear arrow {}", shows(arrow) as u8);
        }
        self.drawn_arrow = arrow;
    }

    pub(super) fn new_tab_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let (plus, label) = new_tab_label();
        // ⌥-click opens the new tab beside the page, as Dia's does.
        self.row(Row::NewTab, plate(plus, label), cx).on_click(cx.listener(
            |shell, event: &ClickEvent, _, cx| {
                let command =
                    if event.modifiers().alt { Command::AddSplit } else { Command::NewTab };
                shell.run(command, "sidebar", cx)
            },
        ))
    }
}

/// The speaker after a playing tab's favicon, crossed out while it's muted.
pub(super) fn speaker_glyph(muted: bool) -> &'static str {
    if muted { "icons/volume-x.svg" } else { "icons/volume-2.svg" }
}

// How often the rings are drawn. GPUI can't redraw only a ring, so each frame is the whole shell
// again; 30 ms lands on every second display frame at 60 Hz and every fourth at 120.
const LOADING_FRAME: Duration = Duration::from_millis(30);
// Rings left undrawn this long start upright again.
const LOADING_GAP: Duration = Duration::from_millis(200);

/// The loading rings' clock: every ring turns with it, and one timer redraws them all while one
/// is drawn, where an animation for each would ask for the shell at every display frame.
#[derive(Default)]
pub(super) struct Rings {
    /// When the turn began that the rings are on, and when a ring was last drawn.
    since: Cell<Option<Instant>>,
    last: Cell<Option<Instant>>,
    /// The frame being built draws a ring, and a timer is out to draw the next.
    drawn: Cell<bool>,
    pending: Cell<bool>,
    /// Motion is reduced, so the rings stand still.
    still: Cell<bool>,
}

impl Rings {
    /// How far through its turn a ring drawn now is, 0 to 1.
    fn turn(&self) -> f32 {
        self.drawn.set(true);
        if self.still.get() {
            return 0.;
        }
        let now = Instant::now();
        if self.last.replace(Some(now)).is_none_or(|last| now.duration_since(last) > LOADING_GAP) {
            self.since.set(Some(now));
        }
        let since = self.since.get().unwrap_or(now);
        (now.duration_since(since).as_secs_f32() / LOADING_TURN.as_secs_f32()).fract()
    }

    /// Once a frame is built: if it drew a ring, draws the next in a moment.
    pub(super) fn arm(&self, cx: &mut Context<Shell>) {
        self.still.set(cx.reduce_motion());
        if !self.drawn.take() || self.still.get() || self.pending.replace(true) {
            return;
        }
        cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(LOADING_FRAME).await;
            shell
                .update(cx, |shell, cx| {
                    shell.rings.pending.set(false);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}

impl Shell {
    /// A loading tab's ring, at the row's trailing edge, turning.
    fn loading_ring(&self) -> impl IntoElement + use<> {
        svg()
            .path("icons/loading.svg")
            .flex_none()
            .ml(px(4.))
            .size(px(LOADING_SIZE))
            .text_color(rgba(SECONDARY))
            .with_transformation(Transformation::rotate(percentage(self.rings.turn())))
    }
}

/// An emoji in a favicon's place.
pub(super) fn emoji(icon: SharedString) -> AnyElement {
    div()
        .text_size(px(EMOJI_SIZE))
        .line_height(px(FAVICON_SIZE))
        .child(icon)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::{
        rc::Rc,
        time::{Duration, Instant},
    };

    use gpui::{Entity, TestAppContext, VisualTestContext};

    use super::{
        super::{Folders, Shell, Swapped, commands::bind_keys, recorder::Recorder},
        LOADING_GAP, LOADING_TURN, Rings,
    };

    /// Rings begin upright and are half way round half a turn later, together; left undrawn for a
    /// while they start upright again, and with reduced motion they never turn.
    #[test]
    fn the_rings_turn_together_and_start_upright() {
        let rings = Rings::default();
        assert!(rings.turn() < 0.01);
        rings.since.set(Some(Instant::now() - LOADING_TURN / 2));
        let (first, second) = (rings.turn(), rings.turn());
        assert!((0.49..0.52).contains(&first), "half a turn in, {first}");
        assert!((first..first + 0.01).contains(&second));

        rings.last.set(Some(Instant::now() - LOADING_GAP - Duration::from_millis(1)));
        assert!(rings.turn() < 0.01, "undrawn for a while, it starts over");

        rings.since.set(Some(Instant::now() - LOADING_TURN / 2));
        rings.still.set(true);
        assert_eq!(rings.turn(), 0.);
    }

    /// The window, showing a Space whose Today tabs the core reports as `today`, with `shown`.
    fn report(shell: &Entity<Shell>, cx: &mut VisualTestContext, today: Vec<u64>, shown: u64) {
        shell.update(cx, |shell, cx| {
            shell.set_tabs(1, Vec::new(), today, Vec::new(), Folders::default(), cx);
            shell.set_selected(shown, cx);
        });
        cx.run_until_parked();
    }

    /// Whether the window drew Clear with its arrow, without it, or no Clear, and whether the
    /// arrow was coming or going.
    fn arrow(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> (Option<bool>, bool) {
        shell.read_with(cx, |shell, _| {
            let drawn = shell.drawn_arrow.map(|(_, arrow)| arrow);
            (drawn, shell.swaps.changing(Swapped::Clear(1)))
        })
    }

    /// With two Today tabs, Clear keeps the one shown and has no arrow. Once one is left, the arrow
    /// comes before Clear, and Clear closes that one too. ⌘Z brings it back with its arrow, shown
    /// at once on the divider drawn again, and a second tab takes it away until it closes.
    #[gpui::test]
    fn clear_shows_its_arrow_when_one_today_tab_is_left(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        cx.update(bind_keys);
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into())], cx);
            shell.set_space(1, cx);
        });
        // The keys reach the shell as they do with the UI focused.
        cx.update(|window, cx| window.focus(&shell.read(cx).focus_handle.clone(), cx));
        report(&shell, cx, vec![2, 3], 2);
        assert_eq!(arrow(&shell, cx), (Some(false), false));

        cx.simulate_keystrokes("shift-cmd-k");
        assert!(host.take().contains(&"clear".to_string()));
        assert_eq!(shell.read_with(cx, |shell, _| shell.clearing.clone()), vec![3]);
        report(&shell, cx, vec![2], 2);
        assert_eq!(arrow(&shell, cx), (Some(true), true), "the arrow comes on the same Clear");

        cx.simulate_keystrokes("shift-cmd-k");
        assert!(host.take().contains(&"clear".to_string()));
        assert_eq!(shell.read_with(cx, |shell, _| shell.clearing.clone()), vec![2]);
        report(&shell, cx, Vec::new(), 0);
        assert_eq!(arrow(&shell, cx).0, None);

        cx.simulate_keystrokes("cmd-z");
        assert!(host.take().contains(&"undo clear".to_string()));
        report(&shell, cx, vec![2], 2);
        assert_eq!(arrow(&shell, cx), (Some(true), false), "a divider drawn again has it at once");

        // A tab that opens takes it away again, on the same Clear, and closing it brings it back.
        report(&shell, cx, vec![4, 2], 4);
        assert_eq!(arrow(&shell, cx), (Some(false), true));
        report(&shell, cx, vec![2], 2);
        assert_eq!(arrow(&shell, cx), (Some(true), true));
    }
}
