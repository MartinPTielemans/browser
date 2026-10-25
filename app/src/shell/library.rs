//! The Library, which takes the sidebar's place and pushes the card right without resizing it
//! (Arc): a column of its sections, of which there's one so far, Archived Tabs, and beside it the
//! archive, newest first under day headings, with a search field over it (docs/ux-spec.md ›
//! Archive).

use std::{sync::Arc, time::Instant};

use gpui::{
    AnyElement, Context, Entity, Focusable, FontWeight, Image, ListAlignment, ListState,
    MouseButton, SharedString, Subscription, Window, div, list, prelude::*, px, rgb, rgba, svg,
};

use super::{
    Command, FOOTER_HEIGHT, FORM_TEXT, HOVERED, NAV, NAV_HOVERED, NAV_RADIUS, NAV_SIZE, PRESSED,
    SECONDARY, Shell, UNSELECTED_TITLE, favicon, host, spring,
};
use crate::text_input::TextInput;

/// Where the card's left edge goes while the Library is open: the Library's right edge.
const LIBRARY_WIDTH: f32 = 417.;
/// The push, on Arc's measured spring (1 run, rms 1.0 pt).
const LIBRARY_RESPONSE: f32 = 0.245;
const LIBRARY_DAMPING: f32 = 0.92;

/// The column of sections, and the line at its edge.
const COLUMN_WIDTH: f32 = 112.;
const COLUMN: u32 = 0xffffff0f;
const COLUMN_EDGE: u32 = 0xffffff0a;
/// A section's button, its glyph centred 31 below its top and its label 56 below.
const SECTION_X: f32 = 8.;
const SECTION_WIDTH: f32 = 97.;
const SECTION_HEIGHT: f32 = 80.;
const SECTION_RADIUS: f32 = 12.;
const SECTION_GLYPH: f32 = 20.;
const SECTION_GLYPH_Y: f32 = 31.;
const SECTION_LABEL_Y: f32 = 56.;
const SECTION_LABEL_HEIGHT: f32 = 15.;
const SECTION_LABEL: u32 = 0xffffffd9;
/// The plate under the selected section and the search field.
const LIBRARY_PLATE: u32 = 0xffffff17;
/// The back button, at the column's bottom.
const LIBRARY_BACK_INSET: f32 = 8.;

/// The search field, on the window buttons' row. Arc's Filter button is left out, and the field
/// takes its place.
const SEARCH_X: f32 = 120.;
const SEARCH_TOP: f32 = 10.;
const SEARCH_WIDTH: f32 = 289.;
const SEARCH_HEIGHT: f32 = 36.;
const SEARCH_RADIUS: f32 = 10.;
const SEARCH_GLYPH: f32 = 15.;
const SEARCH_GLYPH_X: f32 = 14.5;
const SEARCH_TEXT_X: f32 = 44.;
const SEARCH_PLACEHOLDER: u32 = 0xffffff8f;
// The command bar's.
const SEARCH_CARET: u32 = 0x4a77d4;
const SEARCH_SELECTION: u32 = 0x787d_86b3;

/// The archive's list, beside the column.
const ARCHIVE_TOP: f32 = 56.;
const ARCHIVE_WIDTH: f32 = 304.;
const HEADING_X: f32 = 12.;
const HEADING_TOP: f32 = 8.;
const HEADING_HEIGHT: f32 = 15.;
const HEADING_BOTTOM: f32 = 4.;
/// Over a heading after rows. Ours: Arc's capture had one heading.
const HEADING_GAP: f32 = 12.;
const ARCHIVED_PITCH: f32 = 56.;
const ARCHIVED_PLATE: f32 = 50.;
const ARCHIVED_LEFT: f32 = 8.;
const ARCHIVED_RIGHT: f32 = 3.;
const ARCHIVED_FAVICON_X: f32 = 14.;
const ARCHIVED_TEXT_X: f32 = 44.;
const ARCHIVED_RADIUS: f32 = 12.;
/// Rows laid out past the list's ends, so a scroll doesn't show them late.
const ARCHIVE_OVERDRAW: f32 = 300.;

/// The headings, by how long ago a tab was archived: the starts of `Browser::day_starts`, then
/// before them.
const HEADINGS: [&str; 5] = ["Today", "Yesterday", "Earlier This Week", "Last Week", "Older"];

/// A tab in the archive, as the browser reports it.
pub struct Archived {
    pub id: u64,
    pub url: SharedString,
    pub title: SharedString,
    /// When it was archived, in milliseconds since the Unix epoch.
    pub archived: f64,
}

/// The open Library, until it has closed.
pub(super) struct Library {
    search: Entity<TextInput>,
    /// The search the list shows, to skip notes from the field that leave its text as it was.
    query: String,
    items: Vec<Item>,
    list: ListState,
    _search: Subscription,
}

/// A line of the archive's list.
#[derive(Clone, Copy)]
enum Item {
    Heading(&'static str),
    /// The archived tab at this index of `Shell::archive`.
    Tab(usize),
}

impl Shell {
    /// The archive changed, all of it, newest first.
    pub fn set_archive(&mut self, archive: Vec<Archived>, cx: &mut Context<Self>) {
        self.archive = archive;
        let archive = &self.archive;
        self.page_favicons.retain(|url, _| archive.iter().any(|tab| tab.url == *url));
        self.filter_archive(true, cx);
    }

    /// The favicon `Browser::page_favicon` asked for.
    pub fn set_page_favicon(
        &mut self,
        url: SharedString,
        favicon: Option<Arc<Image>>,
        cx: &mut Context<Self>,
    ) {
        if let Some(entry) = self.page_favicons.get_mut(&url) {
            *entry = favicon;
            cx.notify();
        }
    }

    /// How far the Library pushes the card right of `card_left`, at the same width.
    pub fn card_push(&self) -> f32 {
        if self.page_fullscreen {
            return 0.;
        }
        (LIBRARY_WIDTH - self.card_left()) * self.library_out()
    }

    /// How far the card has gone to the Library's edge: 0 not at all and 1 there. None once it's
    /// at rest.
    fn library_step(&self) -> Option<f32> {
        let (from, start) = self.library_motion?;
        let to = if self.library_open { 1. } else { 0. };
        spring(start.elapsed().as_secs_f32(), LIBRARY_RESPONSE, LIBRARY_DAMPING)
            .map(|eased| from + (to - from) * eased)
    }

    fn library_out(&self) -> f32 {
        self.library_step().unwrap_or(if self.library_open { 1. } else { 0. })
    }

    /// Opens the Library, or closes it, from wherever the card is.
    pub(super) fn set_library(&mut self, open: bool, cx: &mut Context<Self>) {
        if open == self.library_open || (open && self.page_fullscreen) {
            return;
        }
        self.library_motion = Some((self.library_out(), Instant::now()));
        self.library_open = open;
        if open {
            self.conceal_now(cx);
            if self.library.is_none() {
                let search = cx.new(|cx| {
                    TextInput::new("Search Archive…", cx)
                        .placeholder_color(rgba(SEARCH_PLACEHOLDER))
                        .caret_color(rgb(SEARCH_CARET))
                        .selection_color(rgba(SEARCH_SELECTION))
                });
                let observe = cx.observe(&search, |shell, _, cx| shell.filter_archive(false, cx));
                self.library = Some(Library {
                    search,
                    query: String::new(),
                    items: Vec::new(),
                    list: ListState::new(0, ListAlignment::Top, px(ARCHIVE_OVERDRAW)),
                    _search: observe,
                });
                self.filter_archive(true, cx);
            }
            self.browser.show_window_buttons(true, cx);
        }
        cx.notify();
    }

    /// Closes the Library at once, as a page goes full screen.
    pub(super) fn close_library_now(&mut self) {
        self.library = None;
        self.library_open = false;
        self.library_motion = None;
    }

    /// Steps the Library's motion for a frame, and once it's done, says so and lets the closed
    /// Library go.
    pub(super) fn step_library(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The search field goes with the Library, so the keyboard goes back to the shell.
        if !self.library_open
            && let Some(library) = &self.library
            && library.search.focus_handle(cx).is_focused(window)
        {
            window.focus(&self.focus_handle, cx);
        }
        if self.library_motion.is_none() {
            return;
        }
        if self.library_step().is_some() {
            window.request_animation_frame();
            return;
        }
        self.library_motion = None;
        // check.sh waits for these lines.
        if self.library_open {
            eprintln!("browser: library shown");
        } else {
            eprintln!("browser: library hidden");
            self.library = None;
            if self.sidebar_hidden {
                self.browser.show_window_buttons(false, cx);
            }
        }
    }

    /// Lists the archived tabs the search finds, under their headings. With `changed` false, only
    /// if the search did.
    fn filter_archive(&mut self, changed: bool, cx: &mut Context<Self>) {
        let Some(library) = &mut self.library else {
            return;
        };
        let query = library.search.read(cx).text().trim().to_lowercase();
        if !changed && query == library.query {
            return;
        }
        let starts = self.browser.day_starts();
        let mut items = Vec::new();
        let mut heading = None;
        for (index, tab) in self.archive.iter().enumerate() {
            let found = |text: &SharedString| text.to_lowercase().contains(&query);
            if !query.is_empty() && !found(&tab.title) && !found(&tab.url) {
                continue;
            }
            let day = starts.iter().position(|&start| tab.archived >= start).unwrap_or(4);
            if heading != Some(day) {
                heading = Some(day);
                items.push(Item::Heading(HEADINGS[day]));
            }
            items.push(Item::Tab(index));
        }
        library.list.reset(items.len());
        library.items = items;
        library.query = query;
        cx.notify();
    }

    /// Opens the archived tab `id` at the top of the Space's Today tabs and closes the Library.
    fn restore_archived(&mut self, id: u64, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        eprintln!("browser: restore archived {id}");
        self.browser.restore_archived(id, cx);
        self.set_library(false, cx);
    }

    /// Empties the archive, once the user says so.
    pub(super) fn clear_archive(&mut self, cx: &mut Context<Self>) {
        let count = self.archive.len();
        if count == 0 {
            return;
        }
        let browser = self.browser.clone();
        self.browser.confirm(
            "Clear the archive?".into(),
            if count == 1 {
                "Its tab can't be reopened from it after this.".into()
            } else {
                format!("Its {count} tabs can't be reopened from it after this.")
            },
            "Clear Archive",
            Box::new(move |cx| browser.clear_archive(cx)),
            cx,
        );
    }

    /// The Library, where the sidebar was.
    pub(super) fn library_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let Some(library) = &self.library else {
            return div();
        };
        // The selected section's glyph has the Space's colour, as in Arc.
        let color = self.shown_space().and_then(|space| space.color).unwrap_or(self.accent_color);
        let section = div()
            .relative()
            .w(px(SECTION_WIDTH))
            .h(px(SECTION_HEIGHT))
            // Arc's buttons sit 5 above the column's middle.
            .mb(px(10.))
            .rounded(px(SECTION_RADIUS))
            .bg(rgba(LIBRARY_PLATE))
            .child(
                svg()
                    .path("icons/archive.svg")
                    .absolute()
                    .left(px((SECTION_WIDTH - SECTION_GLYPH) / 2.))
                    .top(px(SECTION_GLYPH_Y - SECTION_GLYPH / 2.))
                    .size(px(SECTION_GLYPH))
                    .text_color(rgb(color)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(SECTION_LABEL_Y - SECTION_LABEL_HEIGHT / 2.))
                    .w_full()
                    .h(px(SECTION_LABEL_HEIGHT))
                    .flex()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgba(SECTION_LABEL))
                    .child("Archived Tabs"),
            );
        let back = div()
            .id("library-back")
            .absolute()
            .left(px(LIBRARY_BACK_INSET))
            .bottom(px(LIBRARY_BACK_INSET))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.run(Command::ViewLibrary, "library", cx)
            }))
            .child(svg().path("icons/arrow-left.svg").size(px(16.)).text_color(rgba(NAV)));
        let column = div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .w(px(COLUMN_WIDTH))
            .bg(rgba(COLUMN))
            .flex()
            .items_center()
            .pl(px(SECTION_X))
            .child(section)
            .child(back);
        let edge = div()
            .absolute()
            .left(px(COLUMN_WIDTH))
            .top_0()
            .bottom_0()
            .w(px(1.))
            .bg(rgba(COLUMN_EDGE));
        let search = library.search.clone();
        let field = div()
            .id("library-search")
            .absolute()
            .left(px(SEARCH_X))
            .top(px(SEARCH_TOP))
            .w(px(SEARCH_WIDTH))
            .h(px(SEARCH_HEIGHT))
            .flex()
            .items_center()
            .pl(px(SEARCH_TEXT_X))
            .pr(px(12.))
            .rounded(px(SEARCH_RADIUS))
            .bg(rgba(LIBRARY_PLATE))
            .block_mouse_except_scroll()
            // The page may have the keyboard.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, _, window, cx| {
                    shell.browser.focus_ui(cx);
                    window.focus(&search.focus_handle(cx), cx);
                }),
            )
            .text_size(px(14.))
            .text_color(rgb(FORM_TEXT))
            .child(
                svg()
                    .path("icons/search.svg")
                    .absolute()
                    .left(px(SEARCH_GLYPH_X))
                    .top(px((SEARCH_HEIGHT - SEARCH_GLYPH) / 2.))
                    .size(px(SEARCH_GLYPH))
                    .text_color(rgba(SEARCH_PLACEHOLDER)),
            )
            .child(library.search.clone());
        let archive = if library.items.is_empty() {
            let empty = match library.query.is_empty() {
                true => "Today tabs you close are kept here for 30 days.",
                false => "No archived tabs match.",
            };
            div()
                .size_full()
                .flex()
                .justify_center()
                .pt(px(HEADING_TOP + 40.))
                .px(px(HEADING_X))
                .text_size(px(12.))
                .text_color(rgba(SECONDARY))
                .child(empty)
                .into_any_element()
        } else {
            let item = cx.processor(|shell, index, _, cx| shell.archive_item(index, cx));
            list(library.list.clone(), item).size_full().into_any_element()
        };
        div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .w(px(LIBRARY_WIDTH))
            .child(column)
            .child(edge)
            .child(field)
            .child(
                div()
                    .absolute()
                    .left(px(COLUMN_WIDTH))
                    .top(px(ARCHIVE_TOP))
                    .bottom_0()
                    .w(px(ARCHIVE_WIDTH))
                    .child(archive),
            )
    }

    /// A line of the archive's list: a heading, or an archived tab, which reopens on a click.
    fn archive_item(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(item) = self.library.as_ref().and_then(|library| library.items.get(index)) else {
            return div().into_any_element();
        };
        let tab = match *item {
            Item::Heading(heading) => {
                return div()
                    .pt(px(if index == 0 { HEADING_TOP } else { HEADING_TOP + HEADING_GAP }))
                    .pb(px(HEADING_BOTTOM))
                    .pl(px(HEADING_X))
                    .child(
                        div()
                            .h(px(HEADING_HEIGHT))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgba(SECONDARY))
                            .child(heading),
                    )
                    .into_any_element();
            }
            Item::Tab(tab) => tab,
        };
        let Some(tab) = self.archive.get(tab) else {
            return div().into_any_element();
        };
        let (id, url) = (tab.id, tab.url.clone());
        let title = if tab.title.is_empty() { url.clone() } else { tab.title.clone() };
        // Asked for once a row first shows, so a long archive costs nothing until it's scrolled.
        if !self.page_favicons.contains_key(&url) {
            self.page_favicons.insert(url.clone(), None);
            self.browser.page_favicon(&url, cx);
        }
        let icon = favicon(self.page_favicons.get(&url).cloned().flatten());
        let line = |text: SharedString, size: f32, height: f32, color: u32| {
            div()
                .h(px(height))
                .flex()
                .items_center()
                .text_size(px(size))
                .text_color(rgba(color))
                .child(div().truncate().child(text))
        };
        let host = SharedString::from(host(&url).to_string());
        div()
            .h(px(ARCHIVED_PITCH))
            .pl(px(ARCHIVED_LEFT))
            .pr(px(ARCHIVED_RIGHT))
            .py(px((ARCHIVED_PITCH - ARCHIVED_PLATE) / 2.))
            .child(
                div()
                    .id(("archived", id))
                    .size_full()
                    .relative()
                    .rounded(px(ARCHIVED_RADIUS))
                    .block_mouse_except_scroll()
                    .hover(|style| style.bg(rgba(HOVERED)))
                    .active(|style| style.bg(rgba(PRESSED)))
                    .on_click(cx.listener(move |shell, _, _, cx| shell.restore_archived(id, cx)))
                    .child(
                        div()
                            .absolute()
                            .left(px(ARCHIVED_FAVICON_X))
                            .top_0()
                            .bottom_0()
                            .flex()
                            .items_center()
                            .child(icon),
                    )
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .pl(px(ARCHIVED_TEXT_X))
                            .pr(px(12.))
                            .child(line(title, 14., 17., UNSELECTED_TITLE))
                            .child(line(host, 12., 15., SECONDARY)),
                    ),
            )
            .into_any_element()
    }

    /// The footer's button that opens the Library, at its left (Arc).
    pub(super) fn library_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id("library")
            .absolute()
            .left(px(LIBRARY_BACK_INSET))
            .bottom(px((FOOTER_HEIGHT - NAV_SIZE) / 2.))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.run(Command::ViewLibrary, "sidebar", cx)
            }))
            .child(svg().path("icons/archive.svg").size(px(16.)).text_color(rgba(NAV)))
    }
}
