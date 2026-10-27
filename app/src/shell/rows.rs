//! The list's rows: a tab's, a split view's and a folder's, the Space's name, and the
//! divider with Clear and New Tab.

use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, ClickEvent, Context, Hsla, MouseButton, MouseDownEvent, Role, SharedString,
    Transformation, canvas, div, percentage, prelude::*, px, rgb, rgba, svg,
};

use super::{
    CLEAR, CLEAR_ARROW, CLEAR_HOVERED, CLOSE_GLYPH, CLOSE_HOVERED, CLOSE_RIGHT, CLOSE_SIZE,
    CancelName, Closing, Command, DIVIDER, EMOJI_SIZE, FAVICON_DRAWN, FAVICON_SIZE, FAVICON_X,
    FOLDER_INDENT, FOOTER_HEIGHT, FORM_TEXT, HOVERED, KeepName, LOADING_ROW_MIN, LOADING_SIZE,
    LOADING_TURN, NAME_HEIGHT, NAV, NAV_HOVERED, NAV_RADIUS, Naming, PLATE_HEIGHT, PLATE_INSET,
    ROW_HEIGHT, ROW_PITCH, Row, SECONDARY, SEGMENT_END, SEGMENT_FAVICON_X, SEGMENT_LINE, SEGMENT_TITLE_GAP, SELECTED,
    SELECTED_UNHOVER, SLEEPING_FAVICON, SPACE_ICON_PLATE, SPEAKER_DRAWN, SPEAKER_GAP, SPEAKER_SIZE,
    Shell, Slot, Swapped, TITLE_END_INSET, clear_label, cubic_bezier, divider_box, favicon,
    folder_icon, name_box, new_tab_label, plate, row_box, row_spring, scaled::scaled_from_middle,
    selected_plate, space_icon, split_plate, swap::{ARROW_DOWN, CLOSE, NONE, UNLOAD, folder_glyph},
    title,
};
use super::{
    access::{Accessible, NEW_TAB},
    drag::in_sidebar,
};
use crate::icon_picker::{self, Subject};

impl Shell {
    /// Closes every folder of the Space shown, all on one fold.
    pub(super) fn collapse_folders(&mut self, cx: &mut Context<Self>) {
        let Some(space) = self.shown_space() else {
            return;
        };
        let folders = space.folders.folders.keys().copied();
        let open: Vec<u64> = folders.filter(|&folder| space.is_open(folder)).collect();
        // The core reports each, and the first report has them all closed.
        self.folding = (!open.is_empty()).then_some(false);
        for folder in open {
            self.browser.set_folder_open(folder, false, cx);
        }
    }

    /// Opens or closes the folder as it's pressed, as Arc's does, even if the press becomes a drag.
    fn press_folder(&mut self, folder: u64, cx: &mut Context<Self>) {
        let open = self.shown_space().is_some_and(|space| space.is_open(folder));
        log!("browser: command toggle-folder via sidebar");
        self.folding = Some(!open);
        self.browser.set_folder_open(folder, !open, cx);
    }

    /// A sidebar row: the plate, with the favicon slot and the label. The `ghost` is a dragged
    /// row's copy the pointer holds, drawn over the list while the row stays in its place.
    fn row(
        &self,
        row: Row,
        content: gpui::Div,
        ghost: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let selected = row == self.selected_row();
        let (id, slot) = match row {
            Row::NewTab => (("new-tab", 0), Slot::NewTab),
            Row::Tab(tab) => (("tab", tab), Slot::Tab(tab)),
        };
        let folder = matches!(row, Row::Tab(tab) if self.is_folder(tab));
        // Full for a dragged row's ghost, which the pointer holds, and none for the others then.
        let dragging = self.dragging();
        let hover = match dragging {
            Some(_) => ghost.then_some(1.),
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
            (_, Some(drag)) if ghost => drag.depth,
            (Row::Tab(tab), _) => self.depth(tab),
            (Row::NewTab, _) => 0,
        };
        let plate = content
            .when(selected, |plate| selected_plate(plate, plate_in, hover.unwrap_or(0.)))
            .id("plate")
            .when_some(hover.filter(|_| !selected), |plate, hover| {
                plate.bg(Hsla::from(rgba(HOVERED)).opacity(hover))
            })
            // Pressed, a tab's row has the selected fill before the core answers its press, and a
            // folder's keeps its hover fill, as Arc's do.
            .when(!selected && dragging != Some(slot) && !folder, |plate| {
                // GPUI keeps an element's pressed state only once a listener gives it a hitbox.
                plate
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .group_active("row", |style| style.bg(rgba(SELECTED)))
            })
            // A ghost covers the rows it passes, in the colours its row has in place.
            .when(ghost, |plate| {
                let fill = if selected { SELECTED } else { HOVERED };
                plate.bg(self.window_fill().blend(rgba(fill).into()))
            });
        let scale = match row {
            Row::Tab(tab) => self.press_scale(tab),
            Row::NewTab => 1.,
        };
        row_box()
            .pl(px(PLATE_INSET + depth as f32 * FOLDER_INDENT))
            .id(id)
            .group("row")
            .opacity(row_in)
            // A dragged row stays in its place, and its ghost is drawn over the list, from `list`.
            .top(px(if dragging == Some(slot) { 0. } else { self.slide(slot) }))
            .block_mouse_except_scroll()
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover(row, *hovered, cx);
                shell.hover_tooltip(row, *hovered, cx)
            }))
            // The ghost is a picture of a row that stays in the list.
            .when(self.a11y && !ghost, |element| self.row_access(element, row, selected))
            .child(scaled_from_middle(plate, scale))
    }

    /// What assistive apps read of a row: a tab, selected or not, a folder, open or closed, or New
    /// Tab. A split view's row is its segments, each a tab.
    fn row_access(
        &self,
        element: gpui::Stateful<gpui::Div>,
        row: Row,
        selected: bool,
    ) -> gpui::Stateful<gpui::Div> {
        match row {
            Row::NewTab => element.accessibility_id(NEW_TAB).button_named("New Tab"),
            Row::Tab(folder) if self.is_folder(folder) => {
                let open = self.shown_space().is_some_and(|space| space.is_open(folder));
                element
                    .role(Role::Button)
                    .accessibility_id(Self::folder_access_id(folder))
                    .aria_label(self.folder_name(folder))
                    .aria_expanded(open)
                    .a11y_synthetic_children(|builder| {
                        let node = builder.parent_node();
                        node.set_role_description("folder");
                        node.add_action(gpui::AccessibleAction::Click);
                        node.add_action(gpui::AccessibleAction::ShowContextMenu);
                    })
            }
            Row::Tab(tab) if self.panes(tab).len() > 1 => element,
            Row::Tab(tab) => element
                .role(Role::Tab)
                .accessibility_id(Self::tab_access_id(tab))
                .aria_label(self.tab_label(tab))
                .aria_selected(selected)
                .pressed_by_mouse_down(true, true),
        }
    }

    /// How full `row`'s hover fill is: full under the pointer or with its menu up, and for the
    /// selected row, easing back once it's left. None for none.
    pub(super) fn hover_fill(&self, row: Row) -> Option<f32> {
        if self.hovered == Some(row) || self.menu_row.is_some_and(|tab| row == Row::Tab(tab)) {
            return Some(1.);
        }
        let selected = row == self.selected_row();
        let left = self.unhovered.iter().find(|&&(old, _)| old == row).filter(|_| selected);
        left.map(|(_, start)| {
            let t = (start.elapsed().as_secs_f32() / SELECTED_UNHOVER).min(1.);
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

    /// A tab's row, its split view's for the split's first tab, or a folder's, or its `ghost`.
    pub(super) fn tab_row(&self, tab: u64, ghost: bool, cx: &mut Context<Self>) -> AnyElement {
        if self.is_folder(tab) {
            return self.folder_row(tab, ghost, cx).into_any_element();
        }
        let panes = self.panes(tab);
        match panes.len() > 1 {
            true => self.split_row(panes, ghost, cx).into_any_element(),
            false => self.single_row(tab, ghost, cx).into_any_element(),
        }
    }

    /// A folder's row: its icon, open or closed, and its name. A press opens or closes it, a
    /// double-click renames it and a right-click shows its menu.
    fn folder_row(
        &self,
        folder: u64,
        ghost: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
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
        let plate = plate(icon, label).child(self.framed(folder, 0.));
        self.row(Row::Tab(folder), plate, ghost, cx).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.browser.tooltip(None, cx);
                    // The first press opened or closed it.
                    if event.click_count == 2 {
                        return shell.rename_folder(folder, cx);
                    }
                    shell.press_folder(folder, cx);
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
        })
    }

    /// The button that closes `tabs`, which shows under the pointer at a row's end: −, which
    /// unloads them and keeps the row, on the selected pinned row, else ✕, as Arc's.
    fn close_button(&self, tabs: Vec<u64>, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let glyph = self.close_glyph(tabs[0]);
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
            .child(self.swaps.glyph(
                Swapped::Close(tabs[0]),
                glyph,
                CLOSE_GLYPH,
                rgba(SECONDARY).into(),
            ))
            // Not the row's press, which would show the tab before closing it.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            // A pinned tab closed unloads, which the core does for the same call.
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                log!("browser: command close-tab via sidebar");
                shell.close_tab(tabs[0], true, cx);
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
                // Not the row's press, which would show the tab, nor a double-click's rename.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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

    /// Shows `tab` for a press on its row, or its segment of a split view's, at once, as Arc's
    /// does, even if the press becomes a drag.
    fn show_row(&mut self, tab: u64, cx: &mut Context<Self>) {
        log!("browser: command select via sidebar");
        self.select(tab, cx)
    }

    /// A press on the page that slides in after a Space switch, where the page isn't yet: the
    /// pointer is over the outgoing page, which takes no input, or over nothing. It acts on the
    /// row that will be there, as the press that it is (docs/ux-principles.md: act on the press),
    /// the Space having switched already. Returns whether it took the press; a press on the
    /// sliding page's drawn part is the rows' own.
    pub(super) fn press_at_rest(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) -> bool {
        let sliding = self.settle.is_some() && self.swipe.is_none() && self.drag.is_none();
        if !sliding || self.form().is_some() || self.renaming.is_some() {
            return false;
        }
        let Some(&(_, page)) = self.pages().iter().find(|&&(space, _)| space == self.space) else {
            return false;
        };
        let at = in_sidebar(event.position, self.sidebar_origin.get());
        let width = self.width();
        let rows = self.drawn_list_top()..f32::from(self.viewport.height) - FOOTER_HEIGHT;
        let on_page = (page..page + width).contains(&at.x);
        if on_page || !(0. ..width).contains(&at.x) || !rows.contains(&at.y) {
            return false;
        }
        let y = self.list_y(at.y);
        let tab = self.layout().into_iter().find_map(|(slot, top)| match slot {
            Slot::Tab(tab) if (top..top + ROW_PITCH).contains(&y) => Some(tab),
            _ => None,
        });
        let Some(tab) = tab.filter(|&tab| self.panes(tab).len() == 1) else {
            return false;
        };
        match event.button {
            MouseButton::Right => self.tab_menu(tab, event.position, cx),
            MouseButton::Left if event.click_count == 2 && self.is_folder(tab) => {
                self.rename_folder(tab, cx)
            }
            MouseButton::Left if event.click_count == 2 => self.rename_tab(tab, cx),
            MouseButton::Left => {
                self.browser.tooltip(None, cx);
                if self.is_folder(tab) {
                    self.press_folder(tab, cx);
                } else {
                    self.show_row(tab, cx);
                }
                self.press_row(tab, event.position);
            }
            _ => return false,
        }
        true
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

    /// `segment`, which a press shows focused, a double-click renames and a right-click shows the
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
            .when(self.a11y, |segment| {
                segment
                    .role(Role::Tab)
                    .accessibility_id(Self::tab_access_id(tab))
                    .aria_label(format!("{}, split view", self.tab_label(tab)))
                    .aria_selected(focused)
                    .pressed_by_mouse_down(true, true)
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first press selected the tab (Dia); the row's press may start a drag.
                    if event.click_count == 2 {
                        cx.stop_propagation();
                        return shell.rename_tab(tab, cx);
                    }
                    shell.show_row(tab, cx)
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
    }

    /// A split view's row, for its first tab: a segment per tab, left to right. A press on one
    /// shows the split with its page focused, and the close button closes them all.
    fn split_row(
        &self,
        panes: Vec<u64>,
        ghost: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
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
        self.row(Row::Tab(first), content, ghost, cx).when(!renaming, |row| {
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

    /// The close button's glyph on the row whose first tab is `row`: − on the selected pinned
    /// row, else ✕.
    pub(super) fn close_glyph(&self, row: u64) -> &'static str {
        let selected = self.selected_row() == Row::Tab(row);
        if selected && self.pinned().contains(&row) { UNLOAD } else { CLOSE }
    }

    /// Turns the ✕ under the pointer into − as its pinned row becomes the selected one, and back
    /// as it stops being, from what the rows showed before the selection moved. Others show their
    /// own at once.
    pub(super) fn selection_moved(&mut self, before: Row) {
        let after = self.selected_row();
        let Some(Row::Tab(row)) = self.hovered else {
            return;
        };
        let moved = before != after && (before == Row::Tab(row) || after == Row::Tab(row));
        if !moved || !self.pinned().contains(&row) {
            return;
        }
        let to = self.close_glyph(row);
        let from = if to == UNLOAD { CLOSE } else { UNLOAD };
        self.swaps.start(Swapped::Close(row), from, to, Instant::now());
    }

    fn single_row(
        &self,
        tab: u64,
        ghost: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
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
                // The mark takes the speaker's place while the page has died or hung.
                .children(match self.trouble_mark(tab) {
                    Some(mark) => Some(mark.into_any_element()),
                    None => self.speaker(tab, cx).map(IntoElement::into_any_element),
                })
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
            // It shows the tab after going back, which the row's press leaves to it, so the old
            // page never loads.
            .when(away, |icon| {
                icon.on_click(cx.listener(move |shell, _, _, cx| {
                    if shell.dragged {
                        return;
                    }
                    log!("browser: command back-to-pinned-url via sidebar");
                    shell.browser.back_to_pinned_url(tab, cx);
                    shell.show_row(tab, cx)
                }))
            })
            .child(icon);
        // Where the row is, for the icon picker to hang under while it waits for the row to settle.
        let bounds = (self.picker_pending.map(|(subject, _)| subject) == Some(Subject::Tab(tab)))
            .then(|| {
                let drawn = self.tab_row_drawn.clone();
                canvas(move |bounds, _, _| drawn.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
            });
        // The field of a rename takes the row's clicks while it's up.
        let row = self.row(Row::Tab(tab), plate(icon.into_any_element(), label), ghost, cx);
        row.children(bounds).when(!renaming, |row| {
            row.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    // The first press selected the tab (Dia).
                    if event.click_count == 2 {
                        return shell.rename_tab(tab, cx);
                    }
                    // On the way back to the pinned URL, the favicon's click shows the tab.
                    if !back {
                        shell.show_row(tab, cx);
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
        })
    }

    pub(super) fn name_row(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let space = self.space;
        let icon = (self.shown_space()).and_then(|each| space_icon(&each.icon));
        // A click on the plate under its icon opens the icon picker, or closes it: the click
        // outside the picker has closed it by now, but the picker hears so only after this event.
        let icon_plate = icon.is_some().then(|| {
            div()
                .id("space-icon")
                .absolute()
                .left(px(FAVICON_X + FAVICON_SIZE / 2. - SPACE_ICON_PLATE / 2.))
                .top(px((NAME_HEIGHT - SPACE_ICON_PLATE) / 2.))
                .size(px(SPACE_ICON_PLATE))
                .rounded(px(NAV_RADIUS))
                .hover(|style| style.bg(rgba(NAV_HOVERED)))
                .when(self.a11y, |plate| {
                    plate.button_named("Change Icon").pressed_by_mouse_down(true, false)
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |shell, _: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        if !icon_picker::is_open(cx) {
                            shell.change_space_icon(space, cx);
                        }
                    }),
                )
        });
        let drawn = self.name_row_drawn.clone();
        // Where the row is, for the icon picker to hang under.
        let bounds = canvas(move |bounds, _, _| drawn.set(Some(bounds)), |_, _, _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full();
        let name = self.shown_space().map(|space| space.name.clone()).unwrap_or_default();
        // A heading VoiceOver can jump to, whose menu is the Space's.
        let row = name_box()
            .id("space-name")
            .when(self.a11y, |row| {
                row.role(Role::Heading)
                    .accessibility_id("space-name")
                    .aria_label(name.clone())
                    .aria_level(2)
                    .pressed_by_mouse_down(false, true)
            })
            .block_mouse_except_scroll()
            .child(bounds)
            .children(icon_plate)
            .children(icon);
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
            // Fading out, it takes no click and doesn't light up.
            let color = if live && self.clear_hovered { CLEAR_HOVERED } else { CLEAR };
            let glyph = self.swaps.glyph(what, arrow, CLEAR_ARROW, rgba(color).into());
            let clear = clear_label(glyph, room, color).id("clear").opacity(self.tools_opacity());
            let clear = clear.when(self.a11y, |clear| {
                clear.accessibility_id("clear").button_named("Clear Today")
            });
            clear.when(live, |clear| {
                clear
                    .block_mouse_except_scroll()
                    .on_hover(cx.listener(|shell, hovered: &bool, _, cx| {
                        shell.clear_hovered = *hovered;
                        // check.sh waits for this line.
                        log!("browser: clear hovered {}", *hovered as u8);
                        cx.notify();
                    }))
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
        self.row(Row::NewTab, plate(plus, label), false, cx)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.pressed_bar_target(cx)),
            )
            .on_click(cx.listener(|shell, event: &ClickEvent, _, cx| {
                let command =
                    if event.modifiers().alt { Command::AddSplit } else { Command::NewTab };
                shell.clicked_bar_target(command, "sidebar", cx)
            }))
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

    use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

    use super::{
        super::{
            CLOSE_RIGHT, CLOSE_SIZE, FAVICON_SIZE, FAVICON_X, Folders, NAME_HEIGHT, PLATE_INSET,
            PRESS_SCALE, PRESS_TIME, Page, ROW_HEIGHT, ROW_PITCH, Row, SELECTED_UNHOVER,
            SPACE_ICON_PLATE, Shell, Slot, Swapped, commands::bind_keys, drag::detents::shell_with,
            recorder::Recorder,
        },
        CLOSE, LOADING_GAP, LOADING_TURN, Rings, Subject, UNLOAD,
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

    /// A row's hover fill comes and goes at once, and only the selected row's eases back, from
    /// full to none over SELECTED_UNHOVER, as Arc's does.
    #[gpui::test]
    fn only_the_selected_rows_hover_eases_away(cx: &mut TestAppContext) {
        let (shell, _) = shell_with(cx, &[], &[1, 2], Folders::default());
        let fill = |shell: &Entity<Shell>, cx: &mut TestAppContext, tab| {
            shell.update(cx, |shell, _| shell.hover_fill(Row::Tab(tab)))
        };
        shell.update(cx, |shell, cx| shell.set_selected(1, cx));
        shell.update(cx, |shell, cx| shell.hover(Row::Tab(2), true, cx));
        assert_eq!(fill(&shell, cx, 2), Some(1.));
        shell.update(cx, |shell, cx| shell.hover(Row::Tab(2), false, cx));
        assert_eq!(fill(&shell, cx, 2), None);

        shell.update(cx, |shell, cx| shell.hover(Row::Tab(1), true, cx));
        assert_eq!(fill(&shell, cx, 1), Some(1.));
        shell.update(cx, |shell, cx| shell.hover(Row::Tab(1), false, cx));
        let easing = fill(&shell, cx, 1).unwrap_or(0.);
        assert!(easing > 0.9, "just left, the selected row's fill is {easing}");
        shell.update(cx, |shell, _| {
            for (_, start) in &mut shell.unhovered {
                *start -= Duration::from_secs_f32(SELECTED_UNHOVER);
            }
        });
        assert_eq!(fill(&shell, cx, 1), Some(0.));
    }

    /// The Space's icon answers the pointer on its plate, Arc's 26 pt frame, as a header button
    /// does: a press anywhere on the plate opens the picker, even past the glyph, and one above
    /// it, still in the name's row, doesn't.
    #[gpui::test]
    fn the_space_icon_opens_the_picker_from_its_plate(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space".into(), None, 1, "🌱".into(), "".into())], cx);
            shell.set_space(1, cx);
        });
        cx.run_until_parked();
        let row = shell.read_with(cx, |shell, _| shell.name_row_drawn.get()).expect("a name row");
        let middle = point(row.left() + px(FAVICON_X + FAVICON_SIZE / 2.), row.center().y);
        // Whether a press at `at` asked for the picker.
        let press = |cx: &mut VisualTestContext, at| {
            cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
            let asked = shell.update(cx, |shell, _| shell.picker_pending.take().is_some());
            cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
            cx.run_until_parked();
            asked
        };
        let edge = SPACE_ICON_PLATE / 2. - 1.5;
        assert!(press(cx, middle + point(px(edge), px(0.))), "on the plate, past the glyph");
        let above = (SPACE_ICON_PLATE / 2. + NAME_HEIGHT / 2.) / 2.;
        assert!(!press(cx, middle - point(px(0.), px(above))), "above the plate");
    }

    /// Change Icon… on a tab asks for the icon picker under that tab's row, which draws where the
    /// picker is to hang: only the row asked for tells where it is.
    #[gpui::test]
    fn change_icon_has_the_tabs_row_say_where_it_is(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into(), "".into())], cx);
            shell.set_space(1, cx);
        });
        report(&shell, cx, vec![2, 3], 2);
        assert_eq!(shell.read_with(cx, |shell, _| shell.tab_row_drawn.get()), None);

        shell.update(cx, |shell, cx| shell.change_icon(3, cx));
        cx.run_until_parked();
        let (pending, row) = shell.read_with(cx, |shell, _| {
            (shell.picker_pending.map(|(subject, _)| subject), shell.tab_row_drawn.get())
        });
        assert_eq!(pending, Some(Subject::Tab(3)));
        let row = row.expect("the tab's row didn't say where it is");
        // The other row is a row's pitch off.
        shell.update(cx, |shell, cx| shell.change_icon(2, cx));
        cx.run_until_parked();
        let first = shell.read_with(cx, |shell, _| shell.tab_row_drawn.get()).unwrap();
        assert_eq!(f32::from(row.center().y - first.center().y).abs(), ROW_PITCH);
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
            shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into(), "".into())], cx);
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

    /// A window showing one Space with `pinned` and `today` tabs and `shown`.
    fn window(
        cx: &mut TestAppContext,
        pinned: Vec<u64>,
        today: Vec<u64>,
        shown: u64,
    ) -> (Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
        window_with(cx, pinned, today, shown, Folders::default())
    }

    /// `window`, with `folders` among the pinned tabs.
    fn window_with(
        cx: &mut TestAppContext,
        pinned: Vec<u64>,
        today: Vec<u64>,
        shown: u64,
        folders: Folders,
    ) -> (Entity<Shell>, &mut VisualTestContext, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(vec![(1, "Space".into(), None, 1, "".into(), "".into())], cx);
            shell.set_space(1, cx);
            shell.set_tabs(1, pinned, today, Vec::new(), folders, cx);
            shell.set_selected(shown, cx);
        });
        cx.run_until_parked();
        (shell, cx, host)
    }

    /// Where `tab`'s row's close button is in the window, and its row's middle.
    fn close_button(shell: &Entity<Shell>, cx: &mut VisualTestContext, tab: u64) -> (f32, f32) {
        shell.read_with(cx, |shell, _| {
            let top = shell.drawn_list_top() + shell.layout()[&Slot::Tab(tab)];
            let x = shell.width() - PLATE_INSET - CLOSE_RIGHT - CLOSE_SIZE / 2.;
            (x, top + ROW_HEIGHT / 2.)
        })
    }

    fn move_to(cx: &mut VisualTestContext, (x, y): (f32, f32)) {
        cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::none());
        cx.run_until_parked();
    }

    /// The glyph the close button of the row under the pointer shows.
    fn hovered_glyph(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Option<&'static str> {
        shell.read_with(cx, |shell, _| match shell.hovered {
            Some(Row::Tab(row)) => Some(shell.close_glyph(row)),
            _ => None,
        })
    }

    /// Under the pointer, the selected pinned row's button is −, and a click on it unloads the tab
    /// with the call that closes one, without showing it again. A pinned row not selected and a
    /// Today row, selected or not, keep ✕.
    #[gpui::test]
    fn the_selected_pinned_row_shows_minus_which_unloads_it(cx: &mut TestAppContext) {
        let (shell, cx, host) = window(cx, vec![2, 3], vec![4, 5], 2);
        let minus = close_button(&shell, cx, 2);
        move_to(cx, minus);
        assert!(shell.read_with(cx, |shell, _| shell.hovered == Some(Row::Tab(2))));
        assert_eq!(hovered_glyph(&shell, cx), Some(UNLOAD));

        host.take();
        cx.simulate_click(point(px(minus.0), px(minus.1)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.take(), ["close 2"]);

        let pinned = close_button(&shell, cx, 3);
        move_to(cx, pinned);
        assert_eq!(hovered_glyph(&shell, cx), Some(CLOSE), "a pinned row not selected");
        let today = close_button(&shell, cx, 5);
        move_to(cx, today);
        assert_eq!(hovered_glyph(&shell, cx), Some(CLOSE), "a Today row");
        shell.update(cx, |shell, cx| shell.set_selected(5, cx));
        assert_eq!(hovered_glyph(&shell, cx), Some(CLOSE), "a selected Today row");
    }

    /// A pinned row selected under the pointer turns its ✕ into −, and back as another row is
    /// selected. A Today row's ✕ stays still, and a pinned row the pointer isn't on shows its
    /// glyph at once.
    #[gpui::test]
    fn the_cross_turns_into_minus_as_its_pinned_row_is_selected(cx: &mut TestAppContext) {
        let (shell, cx, _) = window(cx, vec![2, 3], vec![4], 4);
        let turning = |shell: &Entity<Shell>, cx: &mut VisualTestContext, row| {
            shell.read_with(cx, |shell, _| shell.swaps.changing(Swapped::Close(row)))
        };
        let pinned = close_button(&shell, cx, 2);
        move_to(cx, pinned);
        shell.update(cx, |shell, cx| shell.set_selected(2, cx));
        assert!(turning(&shell, cx, 2));
        assert_eq!(hovered_glyph(&shell, cx), Some(UNLOAD));

        shell.update(cx, |shell, _| shell.swaps = Default::default());
        shell.update(cx, |shell, cx| shell.set_selected(4, cx));
        assert!(turning(&shell, cx, 2), "back to ✕ as it stops being selected");
        assert_eq!(hovered_glyph(&shell, cx), Some(CLOSE));

        shell.update(cx, |shell, _| shell.swaps = Default::default());
        shell.update(cx, |shell, cx| shell.set_selected(3, cx));
        assert!(!turning(&shell, cx, 3), "not under the pointer");
        let today = close_button(&shell, cx, 4);
        move_to(cx, today);
        shell.update(cx, |shell, cx| shell.set_selected(4, cx));
        assert!(!turning(&shell, cx, 4), "a Today row");
    }

    /// A pressed row's plate shrinks to PRESS_SCALE over PRESS_TIME while it's held and grows
    /// back over as long once it's let go. Once it comes away to be dragged, it's full size.
    #[gpui::test]
    fn a_pressed_row_shrinks_a_little_until_it_is_let_go(cx: &mut TestAppContext) {
        let (shell, cx, _) = window(cx, Vec::new(), vec![2, 3], 2);
        let (x, y) = close_button(&shell, cx, 3);
        let middle = point(px(x - 100.), px(y));
        let scale = |shell: &Entity<Shell>, cx: &mut VisualTestContext| {
            shell.read_with(cx, |shell, _| shell.press_scale(3))
        };
        // As if the press were `ago` seconds back, and the release `let_go` seconds back.
        let back = |shell: &Entity<Shell>, cx: &mut VisualTestContext, ago: f32, let_go: f32| {
            shell.update(cx, |shell, _| {
                let held = shell.held_row.as_mut().unwrap();
                let now = Instant::now();
                held.pressed = now - Duration::from_secs_f32(ago);
                held.let_go = held.let_go.map(|_| now - Duration::from_secs_f32(let_go));
            })
        };
        assert_eq!(scale(&shell, cx), 1.);
        cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        back(&shell, cx, PRESS_TIME / 2., 0.);
        let half = scale(&shell, cx);
        assert!(half < 1. && half > PRESS_SCALE, "half way in, {half}");
        back(&shell, cx, PRESS_TIME * 2., 0.);
        assert_eq!(scale(&shell, cx), PRESS_SCALE);

        cx.simulate_mouse_up(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        back(&shell, cx, PRESS_TIME * 3., PRESS_TIME / 2.);
        let out = scale(&shell, cx);
        assert!(out < 1. && out > PRESS_SCALE, "half way out, {out}");
        back(&shell, cx, PRESS_TIME * 3., PRESS_TIME);
        assert_eq!(scale(&shell, cx), 1.);

        cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        back(&shell, cx, PRESS_TIME * 2., 0.);
        assert_eq!(scale(&shell, cx), PRESS_SCALE);
        let away = point(middle.x, middle.y + px(30.));
        cx.simulate_mouse_move(away, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(shell.read_with(cx, |shell, _| shell.dragging().is_some()));
        assert_eq!(scale(&shell, cx), 1.);
    }

    /// A press on a row shows its tab at once, as Arc's does, before it's let go, and letting go
    /// asks for nothing more. A row's ✕ closes its tab without showing it, and a right-click
    /// shows the row's menu without showing its tab.
    #[gpui::test]
    fn a_press_shows_its_tab_before_it_is_let_go(cx: &mut TestAppContext) {
        let (shell, cx, host) = window(cx, vec![2], vec![3, 4], 3);
        let (x, y) = close_button(&shell, cx, 4);
        let middle = point(px(x - 100.), px(y));
        host.take();
        cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.take(), ["select 4"]);
        cx.simulate_mouse_up(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(host.take().is_empty(), "letting go");

        let close = close_button(&shell, cx, 2);
        move_to(cx, close);
        cx.simulate_click(point(px(close.0), px(close.1)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.take(), ["close 2"], "the ✕ of a row not shown");

        let middle = point(px(close.0 - 100.), px(close.1));
        cx.simulate_mouse_down(middle, MouseButton::Right, Modifiers::none());
        cx.run_until_parked();
        assert!(!host.take().contains(&"select 2".to_string()), "a right-click");
    }

    /// A press that becomes a drag has shown its tab, and still drags it: let go two rows up, the
    /// tab moves there, and nothing is shown again.
    #[gpui::test]
    fn a_press_that_becomes_a_drag_still_drags(cx: &mut TestAppContext) {
        let (shell, cx, host) = window(cx, Vec::new(), vec![2, 3, 4], 2);
        let (x, y) = close_button(&shell, cx, 4);
        let from = point(px(x - 100.), px(y));
        host.take();
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.take(), ["select 4"]);
        let to = point(from.x, from.y - px(2. * ROW_PITCH));
        cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(shell.read_with(cx, |shell, _| shell.dragging() == Some(Slot::Tab(4))));
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.moves.get(), 1);
        assert!(!host.take().iter().any(|call| call.starts_with("select")));
    }

    /// A press on a folder's row closes it at once, as Arc's does, and a press again opens it.
    #[gpui::test]
    fn a_press_opens_or_closes_a_folder(cx: &mut TestAppContext) {
        let docs = [(20, ("Docs".into(), true))];
        let folders = Folders { folders: docs.into(), parents: Default::default() };
        let (shell, cx, host) = window_with(cx, vec![20, 21], vec![4], 4, folders);
        let (x, y) = close_button(&shell, cx, 20);
        let middle = point(px(x - 100.), px(y));
        host.take();
        cx.simulate_mouse_down(middle, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(shell.read_with(cx, |shell, _| shell.folding), Some(false));
        let calls = host.take();
        assert!(!calls.iter().any(|call| call.starts_with("select")), "{calls:?}");
    }

    /// Presses in the first frames after ⌃2, while the second Space's rows still slide in from the
    /// right, act on its row at the pointer, as they do once it has settled (docs/ux-principles.md:
    /// act on the press): a right-click shows its menu, and a left-click shows its tab.
    #[gpui::test]
    fn a_press_while_a_space_slides_in_acts_on_its_row(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            let space = |id: u64| (id, format!("Space {id}").into(), None, 1, "".into(), "".into());
            shell.set_spaces(vec![space(1), space(2)], cx);
            shell.set_space(1, cx);
            shell.set_tabs(1, Vec::new(), vec![2, 3], Vec::new(), Folders::default(), cx);
            shell.set_tabs(2, Vec::new(), vec![4, 5], Vec::new(), Folders::default(), cx);
            shell.set_selected(2, cx);
        });
        cx.run_until_parked();
        shell.update(cx, |shell, cx| {
            shell.set_space(2, cx);
            shell.set_selected(4, cx);
        });
        cx.run_until_parked();
        assert!(shell.read_with(cx, |shell, _| shell.settle.is_some()), "the Space didn't slide");

        let (x, y) = close_button(&shell, cx, 5);
        let at = point(px(x - 100.), px(y));
        host.take();
        host.menus.borrow_mut().clear();
        cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.menus.borrow().len(), 1, "a right-click mid-slide showed no menu");
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        let calls = host.take();
        assert_eq!(calls.iter().filter(|call| call.starts_with("select")).count(), 1, "{calls:?}");
        assert!(calls.contains(&"select 5".to_string()), "a left-click mid-slide: {calls:?}");

        // Where the page has come, its drawn row takes the press, and only once.
        shell.update(cx, |shell, cx| {
            if let Some(settle) = shell.settle.as_mut() {
                settle.start -= Duration::from_millis(60);
            }
            cx.notify();
        });
        cx.run_until_parked();
        let page = shell.read_with(cx, |shell, _| shell.pages()[0].1);
        assert!(page > 0. && page < 100., "the page is {page} from its place");
        host.menus.borrow_mut().clear();
        let drawn = point(px(x + page - 100.), px(y));
        cx.simulate_mouse_down(drawn, MouseButton::Right, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.menus.borrow().len(), 1, "a right-click on the drawn row");
    }

    /// A press on an away pinned tab's favicon leaves showing the tab to its click, which takes
    /// the tab back to its pinned URL first, so its old page never loads.
    #[gpui::test]
    fn the_way_back_shows_the_tab_once_it_is_back(cx: &mut TestAppContext) {
        let (shell, cx, host) = window(cx, vec![2], vec![3], 3);
        shell.update(cx, |shell, cx| {
            let page = Page { away_from_pinned_url: true, ..shell.page(2) };
            shell.set_page(2, page, cx)
        });
        let (_, y) = close_button(&shell, cx, 2);
        let favicon = (FAVICON_X + FAVICON_SIZE / 2., y);
        move_to(cx, favicon);
        assert_eq!(shell.read_with(cx, |shell, _| shell.hovered_favicon), Some(2));
        host.take();
        let at = point(px(favicon.0), px(favicon.1));
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(host.take().is_empty(), "the press");
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(host.take(), ["back to pinned url 2", "select 2"]);
    }
}
