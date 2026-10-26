//! The Library, which slides in from the left over the sidebar and pushes the card right without
//! resizing it (Arc): a column of its sections, Downloads, Spaces and Archived Tabs so far, and
//! beside it the section's list, newest first under day headings, with a search field over it, or
//! for Spaces a card for each (docs/ux-spec.md › Archive and Downloads).

use std::{sync::Arc, time::Instant};

use gpui::{
    AnyElement, BoxShadow, ClickEvent, Context, Entity, Focusable, FontWeight, Image, ListAlignment,
    ListState, MouseButton, MouseDownEvent, Pixels, Point, SharedString, Subscription, Window, div,
    hsla, list, point, prelude::*, px, relative, rgb, rgba, svg,
};

use super::{
    Command, FAVICON_X, FOLDER_INDENT, FOOTER_HEIGHT, FORM_TEXT, HOVERED, MenuEntry, NAV,
    NAV_HOVERED, NAV_RADIUS, NAV_SIZE, PLATE_INSET, PRESSED, SECONDARY, SEGMENT_FAVICON_X, Shell,
    Space, UNSELECTED_TITLE, clean_link, clear_label, divider_box, favicon, folder_icon, host,
    name_box, plate, row_box, spring_to, split_plate, title,
};
use crate::text_input::TextInput;

/// Where the card's left edge goes while the Library is open: the Library's right edge, which is
/// this for Downloads and Archived Tabs and `SPACES_WIDTH` for Spaces.
const LIBRARY_WIDTH: f32 = 417.;
/// The push, on Arc's measured spring (1 run, rms 1.0 pt). The Library comes with it from its own
/// width to the left, so they meet as they stop.
const LIBRARY_RESPONSE: f32 = 0.245;
const LIBRARY_DAMPING: f32 = 0.92;
/// Opened or closed by a click, the sidebar under the Library fades out in 2 frames at 60 fps as
/// it comes, and back in over 130 ms from 80 ms after it starts to go (Arc on video, 2 opens and 2
/// closes).
const SIDEBAR_FADE_OUT: f32 = 0.033;
const SIDEBAR_FADE_IN_DELAY: f32 = 0.08;
const SIDEBAR_FADE_IN: f32 = 0.13;
/// Moved by fingers, and on after they let go, the sidebar recedes as the card goes: it fades out
/// by 0.66 of the way, and shrinks by 0.076 of its size over the way, about a point at the window's
/// middle 128 right of its edge, so it goes down and right at the top and up and right at the
/// bottom. The Library comes late, from 0.29 of the way, at 0.92 of its width over the way, so
/// the sidebar is seen receding. That's Arc's at twice the card's speed. Ours is wider, with its
/// column, so once the sidebar is gone it catches the card up as it stops. (Arc 1.170 at 2×, a
/// slow swipe open and one closed, the same both ways, in its Spaces section, whose card goes 192
/// to where the swipe opens it.)
const RECEDE_SPAN: f32 = 0.66;
const RECEDE_SCALE: f32 = 0.076;
pub(super) const RECEDE_X: f32 = 128.;
const LIBRARY_LAG: f32 = 0.29;
const LIBRARY_LAG_SPEED: f32 = 0.92;

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
/// Ours: the glyph of a section not shown, and the plate under one hovered.
const SECTION_GLYPH_UNSELECTED: u32 = 0xffffff99;
const SECTION_HOVERED: u32 = 0xffffff0d;
/// Between two sections' buttons, 100 apart.
const SECTION_GAP: f32 = 20.;
/// The plate under the selected section and the search field.
const LIBRARY_PLATE: u32 = 0xffffff17;
/// The back button, at the column's bottom.
pub(super) const LIBRARY_BACK_INSET: f32 = 8.;

/// The Spaces section (Arc 1.170 cap, one Space): the Library is 480 wide, and a card per Space
/// starts 28 after the column, 240 wide and as tall as the window less 48 above and below, with
/// Arc's blue + 33 after the last, 30 across and halfway down. The card is the Space's gradient at
/// radius 14, with its list drawn as the sidebar draws it, and its name's row has a brush at its
/// end, 28 square, 13 from the card's right. Arc's card has no search field over it.
const SPACES_WIDTH: f32 = 480.;
const SPACES_LEFT: f32 = 28.;
const SPACES_INSET: f32 = 48.;
const CARD_WIDTH: f32 = 240.;
const CARD_RADIUS: f32 = 14.;
/// Ours: Arc's capture had one card, so how far apart they are isn't known.
const CARD_GAP: f32 = 16.;
/// The name's row in the card, 33 down to its middle as in Arc, with the rows 52 down.
const CARD_NAME_TOP: f32 = 14.;
const CARD_NAME_HEIGHT: f32 = 38.;
const CARD_BRUSH: f32 = 28.;
const CARD_BRUSH_GLYPH: f32 = 14.;
const CARD_BRUSH_RIGHT: f32 = 13.;
const CARD_BRUSH_COLOR: u32 = 0xffffff59;
/// Ours: the shadow round a card, which Arc has, soft and dark.
const CARD_SHADOW: f32 = 0.3;
const CARD_SHADOW_BLUR: f32 = 16.;
const PLUS_GAP: f32 = 33.;
const PLUS_SIZE: f32 = 30.;
const PLUS_FILL: u32 = 0x1f2aff;
const PLUS_GLYPH: f32 = 14.;
/// The room after the +, to the Library's edge.
const PLUS_AFTER: f32 = 37.;

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
/// Ours: a running download's bar, under its text, and the footer button's, under its glyph.
const PROGRESS_HEIGHT: f32 = 3.;
const PROGRESS_BOTTOM: f32 = 6.;
const PROGRESS_TRACK: u32 = 0xffffff1f;
const PROGRESS_FILL: u32 = 0xffffff99;
const BUTTON_PROGRESS_WIDTH: f32 = 16.;
const BUTTON_PROGRESS_HEIGHT: f32 = 2.;
const BUTTON_PROGRESS_BOTTOM: f32 = 4.;

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

/// Where a download is.
#[derive(Clone, Copy, PartialEq)]
pub enum DownloadState {
    Running,
    Paused,
    Done,
    Cancelled,
    Failed,
}

impl DownloadState {
    /// For check.sh's log lines.
    fn name(self) -> &'static str {
        match self {
            DownloadState::Running => "running",
            DownloadState::Paused => "paused",
            DownloadState::Done => "done",
            DownloadState::Cancelled => "cancelled",
            DownloadState::Failed => "failed",
        }
    }
}

/// One of the profile's downloads, as the browser reports it.
#[derive(Clone)]
pub struct Download {
    pub id: u64,
    pub name: SharedString,
    pub url: SharedString,
    /// The bytes it has, and the bytes it will have in all, or 0 while that isn't known.
    pub received: i64,
    pub total: i64,
    pub state: DownloadState,
    pub open_when_done: bool,
    /// The browser holds it as possibly harmful until it's kept.
    pub held: bool,
    /// When it started, in milliseconds since the Unix epoch.
    pub started: f64,
}

/// What `Browser::act_on_download` does.
#[derive(Clone, Copy, Debug)]
pub enum DownloadAction {
    /// Opens the file, or for one still running, opens it once it's done, or no longer.
    Open,
    /// Shows the file in Finder.
    Show,
    Pause,
    Resume,
    Cancel,
    /// Takes it off the list, keeping a finished file and deleting any other.
    Remove,
    /// Keeps one the browser held.
    Keep,
}

/// A section of the Library.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Section {
    Downloads,
    Spaces,
    Archive,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::Downloads => "downloads",
            Section::Spaces => "spaces",
            Section::Archive => "archive",
        }
    }

    /// How wide the Library is with this section in it.
    fn width(self) -> f32 {
        match self {
            Section::Spaces => SPACES_WIDTH,
            _ => LIBRARY_WIDTH,
        }
    }
}

/// A line of a Space's card.
#[derive(Debug, PartialEq)]
enum Line {
    /// A folder, in this many folders.
    Folder(u64, usize),
    /// A tab, or a split view by its first tab, in this many folders.
    Tab(u64, usize),
    /// The line over the Today tabs, with Clear at its end.
    Divider,
}

impl Space {
    /// What its card lists: the sidebar's rows, pinned tabs and folders, then the Today tabs
    /// under their line, without the New Tab row between (Arc).
    fn card_lines(&self) -> Vec<Line> {
        let (pinned, today) = self.rows();
        let line = |row: u64| match self.is_folder(row) {
            true => Line::Folder(row, self.depth(row)),
            false => Line::Tab(row, self.depth(row)),
        };
        let divider = (!today.is_empty()).then_some(Line::Divider);
        let lines = pinned.into_iter().map(line).chain(divider);
        lines.chain(today.into_iter().map(line)).collect()
    }
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

/// A line of the section's list.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Heading(&'static str),
    /// The archived tab at this index of `Shell::archive`.
    Tab(usize),
    /// The download at this index of `Shell::downloads`.
    Download(usize),
}

impl Shell {
    /// The archive changed, all of it, newest first.
    pub fn set_archive(&mut self, archive: Vec<Archived>, cx: &mut Context<Self>) {
        self.archive = archive;
        let archive = &self.archive;
        self.page_favicons.retain(|url, _| archive.iter().any(|tab| tab.url == *url));
        if self.section == Section::Archive {
            self.filter(true, cx);
        }
    }

    /// The downloads changed, all of them, newest first.
    pub fn set_downloads(&mut self, downloads: Vec<Download>, cx: &mut Context<Self>) {
        // check.sh waits for these lines: a download's state, and how many there are.
        for download in &downloads {
            let before = self.downloads.iter().find(|each| each.id == download.id);
            if before.is_none_or(|before| before.state != download.state) {
                let state = download.state.name();
                log!("browser: download {} {state} {}", download.id, download.name);
            }
        }
        if downloads.len() != self.downloads.len() {
            log!("browser: downloads {}", downloads.len());
        }
        self.download_icons.retain(|id, _| downloads.iter().any(|each| each.id == *id));
        self.fly_new_download(&downloads, cx);
        self.downloads = downloads;
        if self.section == Section::Downloads {
            self.filter(true, cx);
        }
        cx.notify();
    }

    /// The icon `Browser::download_icon` asked for.
    pub fn set_download_icon(&mut self, id: u64, icon: Option<Arc<Image>>, cx: &mut Context<Self>) {
        if let Some(entry) = self.download_icons.get_mut(&id) {
            *entry = icon;
            cx.notify();
        }
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
        (self.library_width() - self.card_left()) * self.library_out()
    }

    /// How wide the Library is: its section's, and on its way to another's after a click on it.
    pub(super) fn library_width(&self) -> f32 {
        let to = self.section.width();
        let Some((from, start)) = self.library_resize else {
            return to;
        };
        let t = start.elapsed().as_secs_f32();
        spring_to(t, from, to, 0., LIBRARY_RESPONSE, LIBRARY_DAMPING).map_or(to, |(width, _)| width)
    }

    /// How far the card has gone to the Library's edge, 0 not at all and 1 there, and how fast,
    /// while it moves on its own. None once it's at rest.
    fn library_step(&self) -> Option<(f32, f32)> {
        let (from, speed, start) = self.library_motion?;
        let to = if self.library_open { 1. } else { 0. };
        let t = start.elapsed().as_secs_f32();
        spring_to(t, from, to, speed, LIBRARY_RESPONSE, LIBRARY_DAMPING)
    }

    /// How far the card has gone to the Library's edge: 0 not at all and 1 there. Fingers that
    /// move the Library hold it where they have it.
    pub(super) fn library_out(&self) -> f32 {
        if let Some(out) = self.swipe.as_ref().and_then(|swipe| self.library_swiped(swipe)) {
            return out;
        }
        match self.library_step() {
            Some((out, _)) => out,
            None => f32::from(u8::from(self.library_open)),
        }
    }

    /// Whether fingers have the Library, out of the sidebar's edge.
    fn library_swiping(&self) -> bool {
        self.swipe.as_ref().and_then(|swipe| self.library_swiped(swipe)).is_some()
    }

    /// Whether the Library is on its way open or closed, on its own or with the fingers.
    pub(super) fn library_moving(&self) -> bool {
        self.library_motion.is_some() || self.library_swiping()
    }

    /// The click's fade the sidebar is in, if it is: what it faded from, and when.
    fn clicked_fade(&self) -> Option<(f32, Instant)> {
        self.sidebar_fade.filter(|_| !self.library_swiping())
    }

    /// The sidebar's opacity under the Library: receding with the card, or, after a click, gone
    /// fast as the Library comes and back late as it goes.
    pub(super) fn sidebar_opacity(&self) -> f32 {
        let Some((from, start)) = self.clicked_fade() else {
            return receded_opacity(self.library_out());
        };
        let shown = if self.library_open { 0. } else { 1. };
        let t = start.elapsed().as_secs_f32();
        let done = match self.library_open {
            true => t / SIDEBAR_FADE_OUT,
            false => (t - SIDEBAR_FADE_IN_DELAY) / SIDEBAR_FADE_IN,
        };
        if done >= 1. { shown } else { from + (shown - from) * done.max(0.) }
    }

    /// The sidebar's size under the Library, about `RECEDE_X` right of its edge: receding with the
    /// card, or full while a click fades it.
    pub(super) fn sidebar_scale(&self) -> f32 {
        match self.clicked_fade() {
            Some(_) => 1.,
            None => 1. - RECEDE_SCALE * self.library_out(),
        }
    }

    /// How far the Library's right edge has come to the card's: 0 at the window's edge and 1 there.
    /// After a click it comes with the card; with the sidebar receding, late (`LIBRARY_LAG`).
    pub(super) fn library_edge(&self) -> f32 {
        let out = self.library_out();
        if self.clicked_fade().is_some() {
            return out;
        }
        let lagged = |out: f32| LIBRARY_LAG_SPEED * (out - LIBRARY_LAG);
        let catch_up = (1. - lagged(RECEDE_SPAN)) / (1. - RECEDE_SPAN);
        lagged(out).max(1. - catch_up * (1. - out)).max(0.)
    }

    /// Opens the Library, or closes it, from wherever the card is, the sidebar fading.
    pub(super) fn set_library(&mut self, open: bool, cx: &mut Context<Self>) {
        if open == self.library_open || (open && self.page_fullscreen) {
            return;
        }
        let speed = self.library_step().map_or(0., |(_, speed)| speed);
        let opacity = Some(self.sidebar_opacity());
        self.move_library(open, self.library_out(), speed, opacity, cx);
    }

    /// Sends the Library open or closed from `out` (`library_out`), going at `speed` a second, and
    /// the sidebar under it fading from `opacity`, or receding with the card without one.
    pub(super) fn move_library(
        &mut self,
        open: bool,
        out: f32,
        speed: f32,
        opacity: Option<f32>,
        cx: &mut Context<Self>,
    ) {
        let now = Instant::now();
        self.library_motion = Some((out, speed, now));
        self.sidebar_fade = opacity.map(|opacity| (opacity, now));
        self.library_open = open;
        if open {
            self.conceal_now(cx);
            if self.library.is_none() {
                self.show_section(self.section, cx);
            }
        }
        cx.notify();
    }

    /// Opens the Library on `section`, or shows it there, or closes it if it's there already.
    pub(super) fn view_section(&mut self, section: Section, cx: &mut Context<Self>) {
        if !self.library_open {
            self.section = section;
            return self.set_library(true, cx);
        }
        if section == self.section {
            return self.set_library(false, cx);
        }
        self.show_section(section, cx);
    }

    /// Opens the Library on its Spaces, for the preview, which has no pointer to pick it with.
    pub fn show_spaces(&mut self, cx: &mut Context<Self>) {
        self.view_section(Section::Spaces, cx);
    }

    /// Shows the Library's Downloads for Chrome, which asks after the user stopped a quit to let
    /// downloads finish. Unlike ⇧⌘J, it leaves the Library open when Downloads are shown already.
    pub fn show_downloads(&mut self, cx: &mut Context<Self>) {
        if !self.library_open || self.section != Section::Downloads {
            self.run(Command::ViewDownloads, "chrome", cx);
        }
    }

    /// Lists `section` in the Library, with an empty search field of its own.
    pub(super) fn show_section(&mut self, section: Section, cx: &mut Context<Self>) {
        // Ours: the Library goes to the new section's width on the push's spring, as Arc's
        // isn't recorded.
        if self.library.is_some() && section.width() != self.section.width() {
            self.library_resize = Some((self.library_width(), Instant::now()));
        }
        self.section = section;
        let placeholder = match section {
            Section::Downloads => "Search Downloads…",
            Section::Spaces => "",
            Section::Archive => "Search Archive…",
        };
        let search = cx.new(|cx| {
            TextInput::new(placeholder, cx)
                .placeholder_color(rgba(SEARCH_PLACEHOLDER))
                .caret_color(rgb(SEARCH_CARET))
                .selection_color(rgba(SEARCH_SELECTION))
        });
        let observe = cx.observe(&search, |shell, _, cx| shell.filter(false, cx));
        self.library = Some(Library {
            search,
            query: String::new(),
            items: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(ARCHIVE_OVERDRAW)),
            _search: observe,
        });
        // check.sh waits for this line.
        log!("browser: library section {}", section.name());
        self.filter(true, cx);
    }

    /// Closes the Library at once, as a page goes full screen.
    pub(super) fn close_library_now(&mut self) {
        self.library = None;
        self.library_open = false;
        self.library_resize = None;
        self.library_motion = None;
        self.sidebar_fade = None;
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
        if self.library.is_none() {
            self.library_resize = None;
        }
        if let Some((from, start)) = self.library_resize {
            let t = start.elapsed().as_secs_f32();
            match spring_to(t, from, self.section.width(), 0., LIBRARY_RESPONSE, LIBRARY_DAMPING) {
                Some(_) => window.request_animation_frame(),
                None => self.library_resize = None,
            }
        }
        if self.library_motion.is_none() {
            // Fingers that took the Library out a little, then paged instead, let it go.
            if !self.library_open && !self.library_swiping() {
                self.library = None;
            }
            return;
        }
        if self.library_step().is_some() {
            window.request_animation_frame();
            return;
        }
        self.library_motion = None;
        self.sidebar_fade = None;
        // check.sh waits for these lines.
        if self.library_open {
            log!("browser: library shown");
        } else {
            log!("browser: library hidden");
            self.library = None;
        }
    }

    /// Lists the archived tabs or downloads the search finds, under their headings. With `changed`
    /// false, only if the search did.
    fn filter(&mut self, changed: bool, cx: &mut Context<Self>) {
        let Some(library) = &mut self.library else {
            return;
        };
        let query = library.search.read(cx).text().trim().to_lowercase();
        if !changed && query == library.query {
            return;
        }
        let found = |text: &SharedString| text.to_lowercase().contains(&query);
        // Each with the time it's listed by, newest first.
        let found: Vec<(Item, f64)> = match self.section {
            Section::Spaces => Vec::new(),
            Section::Archive => (self.archive.iter().enumerate())
                .filter(|(_, tab)| query.is_empty() || found(&tab.title) || found(&tab.url))
                .map(|(index, tab)| (Item::Tab(index), tab.archived))
                .collect(),
            Section::Downloads => (self.downloads.iter().enumerate())
                .filter(|(_, each)| query.is_empty() || found(&each.name) || found(&each.url))
                .map(|(index, download)| (Item::Download(index), download.started))
                .collect(),
        };
        let starts = self.browser.day_starts();
        let mut items = Vec::new();
        let mut heading = None;
        for (item, time) in found {
            let day = starts.iter().position(|&start| time >= start).unwrap_or(4);
            if heading != Some(day) {
                heading = Some(day);
                items.push(Item::Heading(HEADINGS[day]));
            }
            items.push(item);
        }
        // Reset only for other lines, as it scrolls back to the top, and a running download
        // reports twice a second.
        if items != library.items {
            library.list.reset(items.len());
            library.items = items;
        }
        library.query = query;
        cx.notify();
    }

    /// Opens the archived tab `id` at the top of the Space's Today tabs and closes the Library.
    fn restore_archived(&mut self, id: u64, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        log!("browser: restore archived {id}");
        self.browser.restore_archived(id, cx);
        self.set_library(false, cx);
    }

    /// The menu a right-click on an archived tab's row shows: reopen it, copy its URL, or take it
    /// out of the archive.
    fn archived_menu(&mut self, id: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(url) = self.archive.iter().find(|tab| tab.id == id).map(|tab| tab.url.clone())
        else {
            return;
        };
        let entries = vec![
            MenuEntry::new("Restore Tab", true),
            MenuEntry::new("Copy URL", true),
            MenuEntry::line(),
            MenuEntry::new("Delete from Archive", true),
        ];
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            None,
            false,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| match chosen {
                        Some(0) => shell.restore_archived(id, cx),
                        Some(1) => shell.copy(&clean_link(&url), "Copied URL", cx),
                        Some(3) => {
                            // check.sh waits for this line.
                            log!("browser: remove archived {id}");
                            shell.browser.remove_archived(id, cx);
                        }
                        _ => {}
                    })
                    .ok();
            }),
            cx,
        );
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
        let sections = div()
            .flex()
            .flex_col()
            .gap(px(SECTION_GAP))
            // Arc's buttons sit 5 above the column's middle.
            .mb(px(10.))
            .child(self.section_button(Section::Downloads, color, cx))
            .child(self.section_button(Section::Spaces, color, cx))
            .child(self.section_button(Section::Archive, color, cx));
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
            .child(sections)
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
        let spaces = self.section == Section::Spaces;
        let archive = if library.items.is_empty() {
            let empty = match (self.section, library.query.is_empty()) {
                (Section::Spaces, _) => "",
                (Section::Archive, true) => "Today tabs you close are kept here for 30 days.",
                (Section::Archive, false) => "No archived tabs match.",
                (Section::Downloads, true) => "Files you download show here.",
                (Section::Downloads, false) => "No downloads match.",
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
        let width = self.library_width();
        let panel = div()
            .absolute()
            .left(px((self.library_edge() - 1.) * width))
            .top_0()
            .bottom_0()
            .w(px(width))
            .child(column)
            .child(edge);
        if spaces {
            return panel.child(self.space_cards(cx));
        }
        panel.child(field).child(
            div()
                .absolute()
                .left(px(COLUMN_WIDTH))
                .top(px(ARCHIVE_TOP))
                .bottom_0()
                .w(px(ARCHIVE_WIDTH))
                .child(archive),
        )
    }

    /// The Spaces section: a card for each Space of the window's profile set, in the switcher's
    /// order, and the + after them, which scroll across together if they don't fit.
    fn space_cards(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let cards: Vec<_> = self.spaces.iter().map(|space| self.space_card(space, cx)).collect();
        let plus = div()
            .id("library-new-space")
            .flex_none()
            .ml(px(PLUS_GAP - CARD_GAP))
            .mr(px(PLUS_AFTER))
            .size(px(PLUS_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(rgb(PLUS_FILL))
            .block_mouse_except_scroll()
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.run(Command::NewSpace, "library", cx);
                shell.set_library(false, cx);
            }))
            .child(
                svg().path("icons/nav-plus.svg").size(px(PLUS_GLYPH)).text_color(rgb(FORM_TEXT)),
            );
        div()
            .id("library-spaces")
            .absolute()
            .left(px(COLUMN_WIDTH))
            .top_0()
            .bottom_0()
            .right_0()
            .overflow_x_scroll()
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(CARD_GAP))
                    .pl(px(SPACES_LEFT))
                    .py(px(SPACES_INSET))
                    .children(cards)
                    .child(plus),
            )
    }

    /// A Space's card: its name over the rows the sidebar would show, in its colour. A click shows
    /// the Space, and the brush shows its menu, for what the Space can be changed in.
    fn space_card(&self, space: &Space, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let id = space.id;
        let rows = space.card_lines().into_iter().map(|line| match line {
            Line::Divider => {
                divider_box(self.hairline(), Some(clear_label(true))).into_any_element()
            }
            Line::Folder(folder, depth) => {
                let name = space.folders.folders.get(&folder).map(|(name, _)| name.clone());
                let name = title(name.unwrap_or_default(), false).pr(px(PLATE_INSET));
                let icon = folder_icon(space.is_open(folder));
                card_row(depth, plate(icon, name))
            }
            Line::Tab(tab, depth) => {
                let panes = space.panes(tab);
                if panes.len() == 1 {
                    let label = title(self.title(tab), false).pr(px(PLATE_INSET));
                    return card_row(depth, plate(self.tab_icon(tab), label));
                }
                let plate = CARD_WIDTH - 2. * PLATE_INSET - depth as f32 * FOLDER_INDENT;
                let plate = plate - (FAVICON_X - PLATE_INSET - SEGMENT_FAVICON_X);
                let width = plate / panes.len() as f32;
                let segments = panes.iter().enumerate().map(|(index, &pane)| {
                    self.segment(pane, width, index == 0, false)
                });
                card_row(depth, split_plate(segments))
            }
        });
        let brush = div()
            .id(("space-brush", id))
            .absolute()
            .top(px(CARD_NAME_TOP + (CARD_NAME_HEIGHT - CARD_BRUSH) / 2.))
            .right(px(CARD_BRUSH_RIGHT))
            .size(px(CARD_BRUSH))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                cx.stop_propagation();
                shell.space_menu(id, event.position(), cx);
            }))
            .child(
                svg()
                    .path("icons/paintbrush.svg")
                    .size(px(CARD_BRUSH_GLYPH))
                    .text_color(rgba(CARD_BRUSH_COLOR)),
            );
        let shadow = BoxShadow {
            color: hsla(0., 0., 0., CARD_SHADOW),
            offset: point(px(0.), px(CARD_SHADOW_BLUR / 4.)),
            blur_radius: px(CARD_SHADOW_BLUR),
            spread_radius: px(0.),
            inset: false,
        };
        div()
            .id(("space-card", id))
            .flex_none()
            .w(px(CARD_WIDTH))
            .h_full()
            .overflow_hidden()
            .rounded(px(CARD_RADIUS))
            .bg(self.space_gradient(id))
            .shadow(vec![shadow])
            .block_mouse_except_scroll()
            .on_click(cx.listener(move |shell, _, _, cx| shell.pick_space(id, cx)))
            .child(
                name_box()
                    .mt(px(CARD_NAME_TOP))
                    .h(px(CARD_NAME_HEIGHT))
                    .child(div().truncate().child(space.name.clone())),
            )
            .child(div().children(rows))
            .child(brush)
    }

    /// Shows `space`, from its card, and closes the Library.
    fn pick_space(&mut self, space: u64, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        log!("browser: library space {space}");
        self.show_space(space, cx);
        self.set_library(false, cx);
    }

    /// A line of the archive's list: a heading, or an archived tab, which reopens on a click and
    /// shows its menu on a right-click.
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
            Item::Download(download) => return self.download_item(download, cx),
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
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                            shell.archived_menu(id, event.position, cx)
                        }),
                    )
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
                            .child(text_line(title, 14., 17., UNSELECTED_TITLE))
                            .child(text_line(host, 12., 15., SECONDARY)),
                    ),
            )
            .into_any_element()
    }

    /// A section's button in the column, which shows the section.
    fn section_button(
        &self,
        section: Section,
        color: u32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let selected = section == self.section;
        let (glyph, label) = match section {
            Section::Downloads => ("icons/arrow-down-to-line.svg", "Downloads"),
            Section::Spaces => ("icons/spaces.svg", "Spaces"),
            Section::Archive => ("icons/archive.svg", "Archived Tabs"),
        };
        div()
            .id(label)
            .relative()
            .w(px(SECTION_WIDTH))
            .h(px(SECTION_HEIGHT))
            .rounded(px(SECTION_RADIUS))
            .block_mouse_except_scroll()
            .when(selected, |button| button.bg(rgba(LIBRARY_PLATE)))
            .when(!selected, |button| button.hover(|style| style.bg(rgba(SECTION_HOVERED))))
            .on_click(cx.listener(move |shell, _, _, cx| {
                if section != shell.section {
                    shell.show_section(section, cx);
                }
            }))
            .child(
                svg()
                    .path(glyph)
                    .absolute()
                    .left(px((SECTION_WIDTH - SECTION_GLYPH) / 2.))
                    .top(px(SECTION_GLYPH_Y - SECTION_GLYPH / 2.))
                    .size(px(SECTION_GLYPH))
                    .text_color(if selected { rgb(color) } else { rgba(SECTION_GLYPH_UNSELECTED) }),
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
                    .child(label),
            )
    }

    /// A download's row: its file's icon, its name over where it is, and while it runs, a bar. A
    /// click opens a finished file or shows a held one's menu, and a right-click shows the menu.
    fn download_item(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(download) = self.downloads.get(index) else {
            return div().into_any_element();
        };
        let id = download.id;
        let (state, held) = (download.state, download.held);
        let site = host(&download.url);
        let status: SharedString = match state {
            _ if held => "May harm your computer".into(),
            DownloadState::Running => of(download.received, download.total).into(),
            DownloadState::Paused => {
                format!("Paused · {}", of(download.received, download.total)).into()
            }
            DownloadState::Done => format!("{} · {site}", bytes(download.received)).into(),
            DownloadState::Cancelled => format!("Cancelled · {site}").into(),
            DownloadState::Failed => format!("Failed · {site}").into(),
        };
        let progress = match state {
            DownloadState::Running | DownloadState::Paused if download.total > 0 && !held => {
                Some((download.received as f32 / download.total as f32).clamp(0., 1.))
            }
            _ => None,
        };
        let name = download.name.clone();
        // Asked for once a row first shows, as the archive's favicons are.
        if !self.download_icons.contains_key(&id) {
            self.download_icons.insert(id, None);
            self.browser.download_icon(id, cx);
        }
        let icon = match self.download_icons.get(&id).cloned().flatten() {
            Some(icon) => favicon(Some(icon)).into_any_element(),
            None => (svg().path("icons/file.svg").size(px(16.)).text_color(rgba(SECONDARY)))
                .into_any_element(),
        };
        div()
            .h(px(ARCHIVED_PITCH))
            .pl(px(ARCHIVED_LEFT))
            .pr(px(ARCHIVED_RIGHT))
            .py(px((ARCHIVED_PITCH - ARCHIVED_PLATE) / 2.))
            .child(
                div()
                    .id(("download", id))
                    .size_full()
                    .relative()
                    .rounded(px(ARCHIVED_RADIUS))
                    .block_mouse_except_scroll()
                    .hover(|style| style.bg(rgba(HOVERED)))
                    .active(|style| style.bg(rgba(PRESSED)))
                    .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                        // A held file's only choices are in its menu, so a click shows them.
                        if held {
                            shell.download_menu(id, event.position(), cx);
                        } else if state == DownloadState::Done {
                            shell.act_on_download(id, DownloadAction::Open, "row", cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                            shell.download_menu(id, event.position, cx)
                        }),
                    )
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
                            .child(text_line(name, 14., 17., UNSELECTED_TITLE))
                            .child(text_line(status, 12., 15., SECONDARY)),
                    )
                    .children(progress.map(|done| {
                        div()
                            .absolute()
                            .left(px(ARCHIVED_TEXT_X))
                            .right(px(12.))
                            .bottom(px(PROGRESS_BOTTOM))
                            .h(px(PROGRESS_HEIGHT))
                            .rounded(px(PROGRESS_HEIGHT / 2.))
                            .bg(rgba(PROGRESS_TRACK))
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(done))
                                    .rounded(px(PROGRESS_HEIGHT / 2.))
                                    .bg(rgba(PROGRESS_FILL)),
                            )
                    })),
            )
            .into_any_element()
    }

    /// The menu a right-click on a download's row shows: what can be done with it where it is.
    fn download_menu(&mut self, id: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(download) = self.downloads.iter().find(|each| each.id == id) else {
            return;
        };
        let item = |title: &'static str, action| (MenuEntry::new(title, true), Some(action));
        let mut items: Vec<(MenuEntry, Option<DownloadAction>)> = match download.state {
            _ if download.held => {
                vec![item("Keep", DownloadAction::Keep), item("Delete", DownloadAction::Remove)]
            }
            DownloadState::Running | DownloadState::Paused => vec![
                match download.state {
                    DownloadState::Paused => item("Resume", DownloadAction::Resume),
                    _ => item("Pause", DownloadAction::Pause),
                },
                item("Cancel", DownloadAction::Cancel),
                (
                    MenuEntry::new("Open When Done", true).checked(download.open_when_done),
                    Some(DownloadAction::Open),
                ),
            ],
            DownloadState::Done => vec![
                item("Open", DownloadAction::Open),
                item("Show in Finder", DownloadAction::Show),
            ],
            DownloadState::Cancelled | DownloadState::Failed => Vec::new(),
        };
        if !download.held {
            if !items.is_empty() {
                items.push((MenuEntry::line(), None));
            }
            items.push(item("Remove from List", DownloadAction::Remove));
        }
        let finished = self.downloads.iter().any(|each| {
            !matches!(each.state, DownloadState::Running | DownloadState::Paused)
        });
        items.push((MenuEntry::new("Clear List", finished), None));
        let clear = items.len() - 1;
        let entries = items.iter().map(|(entry, _)| entry.clone()).collect();
        let shell = cx.weak_entity();
        self.browser.menu(
            entries,
            position,
            None,
            false,
            Box::new(move |chosen, cx| {
                shell
                    .update(cx, |shell, cx| match chosen {
                        Some(at) if at == clear => {
                            log!("browser: command clear-downloads via download menu");
                            shell.browser.clear_downloads(cx);
                        }
                        Some(at) => {
                            if let Some((_, Some(action))) = items.get(at) {
                                shell.act_on_download(id, *action, "download menu", cx);
                            }
                        }
                        None => {}
                    })
                    .ok();
            }),
            cx,
        );
    }

    fn act_on_download(
        &mut self,
        id: u64,
        action: DownloadAction,
        source: &str,
        cx: &mut Context<Self>,
    ) {
        // check.sh waits for this line.
        log!("browser: download {id} {action:?} via {source}");
        self.browser.act_on_download(id, action, cx);
    }

    /// The footer's button that opens the Library, at its left (Arc). While downloads run, it shows
    /// the downloads' glyph over a bar of how far they've got, and opens the Library on them.
    pub(super) fn library_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let running = self.downloads.iter().filter(|each| each.state == DownloadState::Running);
        let (received, total) = running.fold((0, 0), |(received, total), each| {
            (received + each.received.min(each.total.max(0)), total + each.total.max(0))
        });
        let downloading = self.downloads.iter().any(|each| each.state == DownloadState::Running);
        let glyph = if downloading { "icons/arrow-down-to-line.svg" } else { "icons/archive.svg" };
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
            .on_click(cx.listener(move |shell, _, _, cx| match downloading {
                true => shell.run(Command::ViewDownloads, "sidebar", cx),
                false => shell.run(Command::ViewLibrary, "sidebar", cx),
            }))
            .child(svg().path(glyph).size(px(16.)).text_color(rgba(NAV)))
            .when(downloading, |button| {
                // A download of a size not yet known shows an empty bar.
                let done = if total > 0 { received as f32 / total as f32 } else { 0. };
                button.child(
                    div()
                        .absolute()
                        .left(px((NAV_SIZE - BUTTON_PROGRESS_WIDTH) / 2.))
                        .bottom(px(BUTTON_PROGRESS_BOTTOM))
                        .w(px(BUTTON_PROGRESS_WIDTH))
                        .h(px(BUTTON_PROGRESS_HEIGHT))
                        .rounded(px(BUTTON_PROGRESS_HEIGHT / 2.))
                        .bg(rgba(PROGRESS_TRACK))
                        .child(
                            div()
                                .h_full()
                                .w(relative(done))
                                .rounded(px(BUTTON_PROGRESS_HEIGHT / 2.))
                                .bg(rgba(PROGRESS_FILL)),
                        ),
                )
            })
    }
}

/// A row of a Space's card, `depth` folders in.
fn card_row(depth: usize, plate: gpui::Div) -> AnyElement {
    row_box().pl(px(PLATE_INSET + depth as f32 * FOLDER_INDENT)).child(plate).into_any_element()
}

/// The sidebar's opacity as it recedes with the Library `out` (`library_out`).
fn receded_opacity(out: f32) -> f32 {
    (1. - out / RECEDE_SPAN).clamp(0., 1.)
}

/// A line of a row's text, cut short with an ellipsis.
fn text_line(text: SharedString, size: f32, height: f32, color: u32) -> gpui::Div {
    div()
        .h(px(height))
        .flex()
        .items_center()
        .text_size(px(size))
        .text_color(rgba(color))
        .child(div().truncate().child(text))
}

/// A size as Finder writes it, in powers of 1000.
fn bytes(bytes: i64) -> String {
    let bytes = bytes.max(0) as f64;
    match bytes {
        b if b < 1e3 => format!("{b} bytes"),
        b if b < 1e6 => format!("{:.0} KB", b / 1e3),
        b if b < 1e9 => format!("{:.1} MB", b / 1e6),
        b => format!("{:.2} GB", b / 1e9),
    }
}

/// How much of a download has come, as Safari writes it.
fn of(received: i64, total: i64) -> String {
    match total > 0 {
        true => format!("{} of {}", bytes(received), bytes(total)),
        false => bytes(received),
    }
}

#[cfg(test)]
mod tests {
    use super::{Line, Space};
    use crate::shell::{Folders, Split};

    fn space(pinned: &[u64], today: &[u64], folders: Folders, splits: Vec<Split>) -> Space {
        Space {
            id: 1,
            name: "Space 1".into(),
            color: None,
            profile: 0,
            pinned: pinned.to_vec(),
            today: today.to_vec(),
            splits,
            folders,
            shown: 0,
        }
    }

    /// A card lists what the sidebar does: folders with what's in them an open folder's depth in,
    /// a split view by its first tab, and a line over the Today tabs only if there are any.
    #[test]
    fn a_card_lists_the_rows_of_the_sidebar() {
        let folders = Folders {
            folders: [(10, ("Docs".into(), true))].into(),
            parents: [(2, 10)].into(),
        };
        let split = Split { tabs: vec![5, 6], ..Split::default() };
        let card = space(&[1, 10, 2], &[4, 5, 6], folders, vec![split]);
        assert_eq!(
            card.card_lines(),
            [
                Line::Tab(1, 0),
                Line::Folder(10, 0),
                Line::Tab(2, 1),
                Line::Divider,
                Line::Tab(4, 0),
                Line::Tab(5, 0),
            ]
        );
        let one = space(&[1], &[], Folders::default(), Vec::new());
        assert_eq!(one.card_lines(), [Line::Tab(1, 0)]);
    }

    /// What's in a closed folder is left out of its card, as of the sidebar, except the tab the
    /// Space shows.
    #[test]
    fn a_closed_folder_hides_its_tabs_but_the_one_shown() {
        let folders = Folders {
            folders: [(10, ("Docs".into(), false))].into(),
            parents: [(2, 10), (3, 10)].into(),
        };
        let mut card = space(&[10, 2, 3], &[], folders, Vec::new());
        assert_eq!(card.card_lines(), [Line::Folder(10, 0)]);
        card.shown = 3;
        assert_eq!(card.card_lines(), [Line::Folder(10, 0), Line::Tab(3, 1)]);
    }
}
