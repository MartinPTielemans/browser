//! The Library, which slides in from the left over the sidebar and pushes the card right without
//! resizing it (Arc): a column of its sections, Downloads, Spaces and Archived Tabs so far, and
//! beside it the section's list, newest first under day headings, with a search field over it, or
//! for Spaces a card for each (docs/ux-spec.md › Archive and Downloads).

use std::{sync::Arc, time::Instant};

use gpui::{
    AnyElement, BoxShadow, ClickEvent, Context, DispatchPhase, Div, Entity, Focusable, FontWeight,
    Image, ListAlignment, ListState, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, SharedString, Stateful, Subscription, Window, canvas, div, hsla, list, point,
    prelude::*, px, relative, rgb, rgba, svg,
};

use super::{
    Command, DRAG_START, FAVICON_X, FOLDER_INDENT, FOOTER_HEIGHT, FORM_TEXT, HOVERED, MenuEntry,
    NAV, NAV_HOVERED, NAV_RADIUS, NAV_SIZE, PLATE_INSET, PRESSED, SECONDARY, SEGMENT_FAVICON_X,
    Shell, Space, Swapped, UNSELECTED_TITLE, clean_link, divider_box, favicon, folder_icon, host,
    name_box, plate, row_box, space_icon, spring_to, split_plate, still_clear, title,
    header::TipSide,
};
use crate::text_input::TextInput;

/// Where the card's left edge goes while the Library is open: the Library's right edge, which is
/// this for Downloads and Archived Tabs and `spaces_width` for Spaces.
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
/// column, so once the sidebar is gone it catches the card up as it stops. Arc's is a sheet that
/// covers the sidebar up to its edge. Ours has no fill, and cut there the sidebar looks sliced
/// along a line with nothing on it, so it fades in over the sidebar as the sidebar fades out.
/// (Arc 1.170 at 2×, a slow swipe open and one closed, the same both ways, in its Spaces section,
/// whose card goes 192 to where the swipe opens it.) The Spaces section moves as `receding` says.
const RECEDE_SPAN: f32 = 0.66;
const RECEDE_SCALE: f32 = 0.076;
pub(super) const RECEDE_X: f32 = 128.;
const LIBRARY_LAG: f32 = 0.29;
const LIBRARY_LAG_SPEED: f32 = 0.92;
/// Under the Spaces section the sidebar is gone by 0.48 of the way (Arc 1.170 rec, three Spaces,
/// both ways), where the older recording above had 0.66.
const SPACES_RECEDE_SPAN: f32 = 0.48;
/// Fingers going on past where the Library opens take it on with the card, a quarter of their way
/// at first and never 80 (ours, as Arc's isn't recorded): about as far as the Space pages' rubber
/// band on a short pull, so it gives, and half as far on a long one, so it pulls back.
const LIBRARY_BAND: f32 = 80.;
const LIBRARY_BAND_STIFFNESS: f32 = 0.25;
/// How near, in points, the Library's panel counts as at rest where it opens, for the window's
/// buttons: half a pixel at 2×, so their swap from the panel's copy can't be seen.
const LIBRARY_REST_SLACK: f32 = 0.25;

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

/// The Spaces section (Arc 1.170 cap, one Space, and rec, three): a card per Space starts 28 after
/// the column, 240 wide, 24 apart and as tall as the window less 48 above and below, with Arc's
/// blue + 33 after the last, 30 across and halfway down, and the Library ends 37 after it, so it's
/// 216 + 264 a Space wide. The card is the Space's gradient at radius 14, with its list drawn as
/// the sidebar draws it, and its name's row has a brush at its end, 28 square, 13 from the card's
/// right. Arc's card has no search field over it.
const SPACES_LEFT: f32 = 28.;
const SPACES_INSET: f32 = 48.;
const CARD_WIDTH: f32 = 240.;
const CARD_RADIUS: f32 = 14.;
const CARD_GAP: f32 = 24.;
/// Ours (*interim*, as Arc's recording never ran out of room): the section grows until the page
/// beside it would be narrower than this, and its cards scroll across past that.
const SPACES_PAGE: f32 = 400.;
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
/// Ours: under the pointer the + darkens to 0.9 of its fill, as the dialog's action does, and a
/// card lightens by the sections' hover plate (Arc's aren't recorded).
const PLUS_HOVERED: u32 = 0x1c26e6;
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
#[derive(Clone)]
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
    /// Where its file is once it's done, or empty until that's known.
    pub path: SharedString,
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
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// What an item of a download's menu does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum DownloadChoice {
    Act(DownloadAction),
    /// Moves the file to the Trash and takes it off the list.
    Trash,
    /// Takes the downloads that aren't running or paused off the list.
    Clear,
}

/// The menu a right-click on `download`'s row shows, what can be done with it where it is, with
/// Clear List enabled if any download is `finished`.
fn download_menu_items(
    download: &Download,
    finished: bool,
) -> Vec<(MenuEntry, Option<DownloadChoice>)> {
    let item = |title: &'static str, choice| (MenuEntry::new(title, true), Some(choice));
    let act = |title: &'static str, action| item(title, DownloadChoice::Act(action));
    let mut items = match download.state {
        _ if download.held => {
            vec![act("Keep", DownloadAction::Keep), act("Delete", DownloadAction::Remove)]
        }
        DownloadState::Running | DownloadState::Paused => vec![
            match download.state {
                DownloadState::Paused => act("Resume", DownloadAction::Resume),
                _ => act("Pause", DownloadAction::Pause),
            },
            act("Cancel", DownloadAction::Cancel),
            (
                MenuEntry::new("Open When Done", true).checked(download.open_when_done),
                Some(DownloadChoice::Act(DownloadAction::Open)),
            ),
        ],
        DownloadState::Done => vec![
            act("Open", DownloadAction::Open),
            act("Show in Finder", DownloadAction::Show),
            item("Move to Trash", DownloadChoice::Trash),
        ],
        DownloadState::Cancelled | DownloadState::Failed => Vec::new(),
    };
    if !download.held {
        if !items.is_empty() {
            items.push((MenuEntry::line(), None));
        }
        items.push(act("Remove from List", DownloadAction::Remove));
    }
    items.push((MenuEntry::new("Clear List", finished), Some(DownloadChoice::Clear)));
    items
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
        let running = |downloads: &[Download]| {
            library_glyph(downloads.iter().any(|each| each.state == DownloadState::Running))
        };
        let (from, to) = (running(&self.downloads), running(&downloads));
        self.swaps.start(Swapped::Library, from, to, Instant::now());
        self.download_icons.retain(|id, _| downloads.iter().any(|each| each.id == *id));
        self.download_previews.retain(|id, _| downloads.iter().any(|each| each.id == *id));
        let bar = bar_drawn(&self.downloads);
        let flying = self.flight.as_ref().map(|flight| flight.id);
        self.fly_new_download(&downloads, cx);
        self.downloads = downloads;
        self.recent_downloads_changed(cx);
        if self.section == Section::Downloads {
            self.filter(true, cx);
        }
        // A download changes about twice a second, and shows only in its list, the footer's bar
        // and the icon flying in.
        let listed = self.library.is_some() && self.section == Section::Downloads;
        let flying = self.flight.as_ref().map(|flight| flight.id) != flying;
        if listed || flying || bar_drawn(&self.downloads) != bar {
            cx.notify();
        }
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

    /// How wide the Library is with `section` in it: Spaces as wide as its cards, in this window.
    pub(super) fn section_width(&self, section: Section) -> f32 {
        match section {
            Section::Spaces => spaces_width(self.spaces.len(), f32::from(self.viewport.width)),
            _ => LIBRARY_WIDTH,
        }
    }

    /// How wide the Library is: its section's, and on its way to another's after a click on it, or
    /// as Spaces come and go.
    pub(super) fn library_width(&self) -> f32 {
        let to = self.section_width(self.section);
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
            return self.receding().sidebar;
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

    /// Where the Library is as the sidebar recedes under it with the card.
    fn receding(&self) -> Receding {
        receding(self.section, self.library_out(), self.library_width(), self.card_left())
    }

    /// How far the Library's right edge has come to the card's: 0 at the window's edge and 1 there,
    /// where it stops, to go on past only with the card (`library_past`). After a click it comes
    /// with the card; with the sidebar receding, as `receding` says.
    pub(super) fn library_edge(&self) -> f32 {
        match self.clicked_fade() {
            Some(_) => self.library_out().min(1.),
            None => self.receding().edge,
        }
    }

    /// How far, in points, fingers past where the Library opens have taken it on with the card.
    fn library_past(&self) -> f32 {
        match self.clicked_fade() {
            Some(_) => 0.,
            None => self.receding().past,
        }
    }

    /// Where the Library cuts the sidebar off, after a click: at its edge, which comes with the
    /// card. Receding, the sidebar fades under it uncut.
    pub(super) fn sidebar_cut(&self) -> Option<f32> {
        self.clicked_fade().map(|_| self.library_edge() * self.library_width())
    }

    /// The Library's opacity: whole after a click, and as the sidebar recedes, as `receding` says.
    fn library_opacity(&self) -> f32 {
        match self.clicked_fade() {
            Some(_) => 1.,
            None => self.receding().opacity,
        }
    }

    /// Whether the Library's panel is at rest where it opens, or past it with the card, to within
    /// a pixel's half. Its column, and the window's buttons on it, then sit where the window's
    /// own do.
    pub(super) fn library_at_rest(&self) -> bool {
        panel_at_rest(self.library_edge(), self.library_width())
    }

    /// Opens the Library, or closes it, from wherever the card is, the sidebar fading.
    pub(super) fn set_library(&mut self, open: bool, cx: &mut Context<Self>) {
        // A private window has no Library: what it shows is the regular windows'.
        if open == self.library_open || (open && (self.page_fullscreen || self.private)) {
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
            self.recent_downloads_gone(cx);
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
        if self.library.is_some() && self.section_width(section) != self.library_width() {
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
        // check.sh waits for these lines.
        log!("browser: library section {}", section.name());
        if section == Section::Spaces {
            let (count, width) = (self.spaces.len(), self.section_width(section));
            log!("browser: library spaces {count} wide {width}");
        }
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
            let (t, to) = (start.elapsed().as_secs_f32(), self.section_width(self.section));
            match spring_to(t, from, to, 0., LIBRARY_RESPONSE, LIBRARY_DAMPING) {
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
    pub(super) fn library_panel(
        &self,
        buttons: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
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
        // Taken on past where it opens, the column's fill reaches back to the window's edge, so
        // the window's own background never shows left of it.
        let past = self.library_past();
        let back = div()
            .id("library-back")
            .absolute()
            .left(px(LIBRARY_BACK_INSET + past))
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
            .left(px(-past))
            .top_0()
            .bottom_0()
            .w(px(COLUMN_WIDTH + past))
            .bg(rgba(COLUMN))
            .flex()
            .items_center()
            .pl(px(SECTION_X + past))
            .child(sections)
            .child(back)
            // Where the window's own sit, so they slide in with the panel, and stay put on the
            // column when fingers take it past its end.
            .children(buttons.then(|| self.window_buttons(cx)));
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
            .left(px((self.library_edge() - 1.) * width + past))
            .top_0()
            .bottom_0()
            .w(px(width))
            .opacity(self.library_opacity())
            .child(column)
            .child(edge);
        if spaces {
            return panel.child(self.space_cards(cx));
        }
        panel
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
            .children(self.download_press_listeners(cx))
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
            .hover(|style| style.bg(rgb(PLUS_HOVERED)))
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
                let clear = still_clear(space.clears_last(space.shown));
                divider_box(self.hairline(), Some(clear)).into_any_element()
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
            .group("space-card")
            // Over the gradient and under the rows.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(CARD_RADIUS))
                    .group_hover("space-card", |style| style.bg(rgba(SECTION_HOVERED))),
            )
            .child(
                name_box()
                    .mt(px(CARD_NAME_TOP))
                    .h(px(CARD_NAME_HEIGHT))
                    .children(space_icon(&space.icon))
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
                self.download_row(div().id(("download", id)), id, "row", cx)
                    .block_mouse_except_scroll()
                    .size_full()
                    .relative()
                    .rounded(px(ARCHIVED_RADIUS))
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

    /// What a download's row does, in the Library or the Library button's recent downloads, with
    /// its plate lit under the pointer: a press that can drag the file out, a click that opens a
    /// finished file or shows a held one's menu, and a right-click that shows the menu.
    pub(super) fn download_row(
        &self,
        row: Stateful<Div>,
        id: u64,
        source: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(download) = self.downloads.iter().find(|each| each.id == id) else {
            return row;
        };
        let (state, held) = (download.state, download.held);
        // GPUI never hears the release that ends the system's drag, and would keep the row
        // pressed.
        let dragged = matches!(self.download_press, Some((pressed, _, true)) if pressed == id);
        row.hover(|style| style.bg(rgba(HOVERED)))
            .when(!dragged, |row| row.active(|style| style.bg(rgba(PRESSED))))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.download_press = Some((id, event.position, false));
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                // A press that became a drag isn't a click.
                if shell.download_press.take().is_some_and(|(_, _, dragged)| dragged) {
                    return;
                }
                // A held file's only choices are in its menu, so a click shows them.
                if held {
                    shell.download_menu(id, event.position(), cx);
                } else if state == DownloadState::Done {
                    shell.act_on_download(id, DownloadAction::Open, source, cx);
                    shell.hide_recent_downloads(cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.download_menu(id, event.position, cx)
                }),
            )
    }

    /// The menu a right-click on a download's row shows: what can be done with it where it is.
    fn download_menu(&mut self, id: u64, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(download) = self.downloads.iter().find(|each| each.id == id) else {
            return;
        };
        let finished = self.downloads.iter().any(|each| {
            !matches!(each.state, DownloadState::Running | DownloadState::Paused)
        });
        let items = download_menu_items(download, finished);
        let entries = items.iter().map(|(entry, _)| entry.clone()).collect();
        let shell = cx.weak_entity();
        // The recent downloads' rows are in the reveal's panel while the sidebar is hidden.
        let in_reveal = self.library.is_none() && self.sidebar_hidden;
        self.browser.menu(
            entries,
            position,
            None,
            in_reveal,
            Box::new(move |chosen, cx| {
                let Some((_, Some(choice))) = chosen.and_then(|at| items.get(at)) else {
                    return;
                };
                shell
                    .update(cx, |shell, cx| match *choice {
                        DownloadChoice::Act(action) => {
                            shell.act_on_download(id, action, "download menu", cx)
                        }
                        DownloadChoice::Trash => shell.trash_download(id, cx),
                        DownloadChoice::Clear => {
                            log!("browser: command clear-downloads via download menu");
                            shell.browser.clear_downloads(cx);
                        }
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

    /// Moves a finished download's file to the Trash and takes it off the list, or leaves both
    /// if the file is there and can't go.
    fn trash_download(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(path) = self.downloads.iter().find(|each| each.id == id).map(|d| d.path.clone())
        else {
            return;
        };
        // For check.sh.
        log!("browser: download {id} Trash via download menu");
        if path.is_empty() || self.browser.trash(&path, cx) {
            self.act_on_download(id, DownloadAction::Remove, "download menu", cx);
        }
    }

    /// The pointer pressing a download's row moved to `position`, the left button held. Past
    /// DRAG_START a finished file goes into the system's drag, for Finder, other apps, the page
    /// or the tab list, and the Library closes so the tab list under it shows.
    fn drag_download(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some((id, press, false)) = self.download_press else {
            return;
        };
        let moved = position - press;
        if moved.x.abs().max(moved.y.abs()) < px(DRAG_START) {
            return;
        }
        let Some(download) = self.downloads.iter().find(|each| each.id == id) else {
            return;
        };
        // One with no file yet keeps its press a click, as a sidebar row's does.
        if download.state != DownloadState::Done || download.held || download.path.is_empty() {
            return;
        }
        let path = download.path.clone();
        self.download_press = Some((id, press, true));
        if self.browser.drag_file(&path, cx) {
            // For check.sh.
            log!("browser: download {id} drag");
            // A row of the recent downloads leaves the Library closed, and the rows go.
            self.library_dragged = self.library.is_some();
            match self.library_dragged {
                true => self.set_library(false, cx),
                false => self.hide_recent_downloads(cx),
            }
        }
        cx.notify();
    }

    /// The system's drag of a download's file was let go. If nothing took it, the Library comes
    /// back with the row it came from, as a sidebar row dropped nowhere goes back.
    pub fn file_drag_ended(&mut self, taken: bool, cx: &mut Context<Self>) {
        // For check.sh.
        log!("browser: file drag ended {}", if taken { "taken" } else { "nowhere" });
        if std::mem::take(&mut self.library_dragged) && !taken {
            self.set_library(true, cx);
        }
    }

    /// The pointer's presses, moves and release anywhere in the window while a download's row is
    /// pressed, which only hears the pointer while it's over it.
    pub(super) fn download_press_listeners(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        self.download_press.filter(|(_, _, dragged)| !dragged)?;
        let shell = cx.weak_entity();
        Some(
            canvas(
                |_, _, _| {},
                move |_, _, window, _| {
                    let pressed = shell.clone();
                    window.on_mouse_event(move |_: &MouseDownEvent, phase, _, cx| {
                        // Before the row pressed, if it's one, hears it.
                        if phase == DispatchPhase::Capture {
                            pressed.update(cx, |shell, _| shell.download_press = None).ok();
                        }
                    });
                    let moved = shell.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture && event.dragging() {
                            let position = event.position;
                            moved.update(cx, |shell, cx| shell.drag_download(position, cx)).ok();
                        }
                    });
                    window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                        // Before the row's click, which hears no press then and opens.
                        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                            shell.update(cx, |shell, _| shell.download_press = None).ok();
                        }
                    });
                },
            )
            .absolute()
            .size_0(),
        )
    }

    /// The footer's button that opens the Library, at its left (Arc). While downloads run, it shows
    /// the downloads' glyph over a bar of how far they've got, and opens the Library on them. The
    /// pointer resting on it shows the recent downloads over it.
    pub(super) fn library_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let progress = downloads_progress(&self.downloads);
        let downloading = progress.is_some();
        let glyph = library_glyph(downloading);
        let command = if downloading { Command::ViewDownloads } else { Command::ViewLibrary };
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
            .map(|button| self.with_tooltip(button, command, TipSide::Beside, cx))
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            // A button has one hover handler, so this one also carries the tooltip's.
            .on_hover(cx.listener(move |shell, hovered: &bool, window, cx| {
                shell.hover_button(command, TipSide::Beside, *hovered, cx);
                shell.hover_recent_downloads(false, *hovered, window, cx)
            }))
            .on_click(cx.listener(move |shell, _, _, cx| match downloading {
                true => shell.run(Command::ViewDownloads, "sidebar", cx),
                false => shell.run(Command::ViewLibrary, "sidebar", cx),
            }))
            .child(self.swaps.glyph(Swapped::Library, glyph, 16., rgba(NAV).into()))
            .when(downloading, |button| {
                let done = progress.unwrap_or(0.);
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

/// How far the running downloads have got in all, 0 to 1, or none while none runs. One of a size
/// not yet known counts for nothing, so its bar stays empty.
fn downloads_progress(downloads: &[Download]) -> Option<f32> {
    let mut running = downloads.iter().filter(|each| each.state == DownloadState::Running);
    let first = running.next()?;
    let running = std::iter::once(first).chain(running);
    let (received, total) = running.fold((0, 0), |(received, total), each| {
        (received + each.received.min(each.total.max(0)), total + each.total.max(0))
    });
    Some(if total > 0 { received as f32 / total as f32 } else { 0. })
}

/// What the footer's button draws of `downloads`: its bar's length in quarter points, if it has
/// one. Smaller steps don't show.
fn bar_drawn(downloads: &[Download]) -> Option<u32> {
    downloads_progress(downloads).map(|done| (done * BUTTON_PROGRESS_WIDTH * 4.).round() as u32)
}

/// A row of a Space's card, `depth` folders in.
fn card_row(depth: usize, plate: gpui::Div) -> AnyElement {
    row_box().pl(px(PLATE_INSET + depth as f32 * FOLDER_INDENT)).child(plate).into_any_element()
}

/// Where a card's left edge is in the Spaces section, the `index`th from the Library's left.
fn card_x(index: usize) -> f32 {
    COLUMN_WIDTH + SPACES_LEFT + index as f32 * (CARD_WIDTH + CARD_GAP)
}

/// Where the +'s left edge is after `cards` cards.
fn plus_x(cards: usize) -> f32 {
    card_x(cards.max(1) - 1) + CARD_WIDTH + PLUS_GAP
}

/// How wide the Spaces section is with `cards` of them in a window `window` wide: to the room
/// after the +, but leaving the page `SPACES_PAGE`, and never narrower than with one card.
fn spaces_width(cards: usize, window: f32) -> f32 {
    let width = |cards| plus_x(cards) + PLUS_SIZE + PLUS_AFTER;
    width(cards).min(window - SPACES_PAGE).max(width(1))
}

/// Where the Library is as the sidebar recedes under it.
struct Receding {
    /// How far its right edge has come to where it opens (`library_edge`).
    edge: f32,
    opacity: f32,
    /// How far, in points, it has gone on past where it opens.
    past: f32,
    /// The sidebar's opacity.
    sidebar: f32,
}

/// Where the Library with `section` in it, `width` wide, is with the card `out` of its way there
/// (`library_out`) from `card_left`. Spaces is one opaque sheet at the card's pace all the way,
/// so its cards pass over the sidebar, and on past where it opens too, 1.4 times the card's way
/// with three Spaces (Arc 1.170 rec, three Spaces). The others come late and fade in over the
/// sidebar, and past where they open go the card's way (the older recording, `LIBRARY_LAG`).
fn receding(section: Section, out: f32, width: f32, card_left: f32) -> Receding {
    let past = (out - 1.).max(0.);
    match section {
        Section::Spaces => Receding {
            edge: out.min(1.),
            opacity: 1.,
            past: width * past,
            sidebar: receded_opacity(out, SPACES_RECEDE_SPAN),
        },
        _ => Receding {
            edge: receding_edge(out),
            opacity: 1. - receded_opacity(out, RECEDE_SPAN),
            past: (width - card_left) * past,
            sidebar: receded_opacity(out, RECEDE_SPAN),
        },
    }
}

/// The sidebar's opacity as it recedes with the Library `out` (`library_out`), gone by `span`.
fn receded_opacity(out: f32, span: f32) -> f32 {
    (1. - out / span).clamp(0., 1.)
}

/// Whether the Library's panel, `width` wide with its edge `edge` of the way (`library_edge`), is
/// at rest, to within `LIBRARY_REST_SLACK`.
fn panel_at_rest(edge: f32, width: f32) -> bool {
    (1. - edge) * width < LIBRARY_REST_SLACK
}

/// How far the Library's edge has come (`library_edge`) as the sidebar recedes with it `out`.
fn receding_edge(out: f32) -> f32 {
    let lagged = |out: f32| LIBRARY_LAG_SPEED * (out - LIBRARY_LAG);
    let catch_up = (1. - lagged(RECEDE_SPAN)) / (1. - RECEDE_SPAN);
    lagged(out).max(1. - catch_up * (1. - out)).clamp(0., 1.)
}

/// How far the card goes, in points, for fingers' `way` past where the Library opens.
pub(super) fn library_band(way: f32) -> f32 {
    super::band(way, LIBRARY_BAND, LIBRARY_BAND_STIFFNESS)
}

/// A line of a row's text, cut short with an ellipsis.
/// The Library button's glyph, the downloads' while they run.
fn library_glyph(downloading: bool) -> &'static str {
    if downloading { "icons/arrow-down-to-line.svg" } else { "icons/archive.svg" }
}

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
    use super::{
        Download, DownloadAction, DownloadChoice, DownloadState, LIBRARY_BAND, LIBRARY_LAG,
        LIBRARY_WIDTH, Line, RECEDE_SPAN, Section, Space, bar_drawn, card_x, download_menu_items,
        downloads_progress, library_band, panel_at_rest, plus_x, receding, receding_edge,
        spaces_width,
    };
    use crate::shell::sidebar::window_buttons_at;
    use crate::shell::{Folders, Split, rubber_band};

    fn space(pinned: &[u64], today: &[u64], folders: Folders, splits: Vec<Split>) -> Space {
        Space {
            id: 1,
            name: "Space 1".into(),
            color: None,
            theme: None,
            icon: Default::default(),
            profile: 0,
            agent: Default::default(),
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

    /// The Library's edge comes from the window's edge as fingers bring it out, and stops where it
    /// opens whatever their way: past that, it goes on only as the card does.
    #[test]
    fn the_library_edge_stops_where_it_opens() {
        for out in [1., 1.01, 1.2, 3.] {
            assert_eq!(receding_edge(out), 1., "the Library's edge went on at {out}");
        }
        assert_eq!(receding_edge(0.), 0.);
        assert!(receding_edge(0.5) > 0. && receding_edge(0.5) < 1.);
    }

    /// Past its end, the Library gives about as far as the Space pages do past theirs on a short
    /// pull, so it's felt, and pulls back harder on a long one, never as far as the band's limit.
    #[test]
    fn past_its_end_the_library_gives_and_pulls_back() {
        for way in [20., 100., 300., 600., 2000.] {
            let (library, page) = (library_band(way), rubber_band(way, 0., 0.));
            let gives = if way <= 300. { library > 0.9 * page } else { library < 0.8 * page };
            assert!(gives, "{library} past the Library, {page} past a page");
        }
        assert!(library_band(600.) < library_band(2000.) && library_band(2000.) < LIBRARY_BAND);
    }

    /// Fingers bringing out Downloads or Archived Tabs fade the Library in over the sidebar as the
    /// sidebar fades out, so neither is cut: faint as its edge comes, and whole once the sidebar is
    /// gone.
    #[test]
    fn the_library_fades_in_as_the_sidebar_recedes() {
        let at = |out| receding(Section::Archive, out, LIBRARY_WIDTH, 288.);
        assert!(at(LIBRARY_LAG).opacity < 0.5 && at(LIBRARY_LAG).edge == 0.);
        assert_eq!(at(RECEDE_SPAN).opacity, 1.);
        assert_eq!(at(RECEDE_SPAN).sidebar, 0.);
        let mut last = 0.;
        for step in 0..=100 {
            let out = step as f32 / 100.;
            assert!(at(out).opacity >= last, "the Library faded back at {out}");
            last = at(out).opacity;
        }
    }

    /// The Spaces section is as wide as Arc's: 480 with one Space and 1008 with three, the cards
    /// 264 apart from 140 and the + 33 after the last, until the page beside it would get too
    /// narrow, and never narrower than with one card.
    #[test]
    fn the_spaces_section_grows_with_its_spaces() {
        assert_eq!(spaces_width(1, 1800.), 480.);
        assert_eq!(spaces_width(3, 1800.), 1008.);
        assert_eq!([card_x(0), card_x(1), card_x(2)], [140., 404., 668.]);
        assert_eq!(plus_x(3), 941.);
        assert_eq!(spaces_width(10, 1800.), 1400.);
        assert_eq!(spaces_width(3, 1200.), 800.);
        assert_eq!(spaces_width(3, 700.), 480.);
        assert_eq!(spaces_width(0, 1800.), 480.);
    }

    /// With three Spaces in Arc's 1800-wide window, the card going from 288 to 1008, the Library
    /// is one opaque sheet whose edge keeps the card's pace, as measured frame by frame, the
    /// sidebar under it gone by 0.48 of the way, and past where it opens it goes on 1.4 times as
    /// far as the card.
    #[test]
    fn the_spaces_section_comes_in_with_the_card() {
        let at = |out| receding(Section::Spaces, out, 1008., 288.);
        // The page card's left and the second card's, measured, both ways.
        let cards = [(565., -216.), (630., -125.5), (734.5, 21.), (905., 260.), (947.5, 320.)];
        for (page, red) in cards {
            let receding = at((page - 288.) / 720.);
            assert_eq!(receding.opacity, 1., "the Library faded with the card at {page}");
            let left = (receding.edge - 1.) * 1008. + card_x(1);
            assert!((left - red).abs() < 2., "the card at {left}, not {red}");
        }
        // How far the card has gone and the sidebar's opacity, measured, both ways.
        let sidebars = [(0.17, 0.66), (0.32, 0.36), (0.43, 0.05), (0.16, 0.68), (0.38, 0.22)];
        for (out, sidebar) in sidebars {
            let faded = at(out).sidebar;
            assert!((faded - sidebar).abs() < 0.06, "the sidebar at {faded} at {out}");
        }
        assert_eq!(at(0.48).sidebar, 0.);
        assert_eq!(at(1.).edge, 1.);
        let past = at(1. + 72. / 720.);
        assert_eq!(past.edge, 1.);
        assert!((past.past / 72. - 1.4).abs() < 0.001, "{} past its end", past.past);
    }

    /// How the window's buttons are drawn with the Library `out` of the way (`library_out`),
    /// moving, as (its own, the panel's copy, the sidebar's copy).
    fn buttons(open: bool, out: f32, docked: bool) -> (bool, bool, bool) {
        let at_rest = panel_at_rest(receding_edge(out), 417.);
        let drawn = window_buttons_at(open, true, at_rest, docked);
        (drawn.window, drawn.library, drawn.sidebar)
    }

    /// Opening, the panel's copy slides in with it and the window's own take over once the panel
    /// has stopped, within a pixel's half of where its copy was, and stay.
    #[test]
    fn the_windows_buttons_slide_in_with_the_library_and_take_over_at_rest() {
        let mut real = false;
        for step in 0..=1200 {
            let out = step as f32 / 1000.;
            let (window, panel, sidebar) = buttons(true, out, true);
            assert!(!(window && (panel || sidebar)), "drawn twice at {out}");
            assert!(window || panel, "missing at {out}");
            assert!(!real || window, "the window's own went again at {out}");
            if window && !real {
                let offset = (1. - receding_edge(out)) * 417.;
                assert!(offset < 0.25, "swapped {offset} pt from the copy's place");
            }
            real |= window;
        }
        assert!(real);
    }

    /// Closing is the same the other way, whether the sidebar is docked or hidden, and the sidebar
    /// draws its copy only once the window's own have gone.
    #[test]
    fn the_windows_buttons_leave_with_the_library() {
        for docked in [true, false] {
            let mut gone = false;
            for step in (0..=1200).rev() {
                let out = step as f32 / 1000.;
                let (window, panel, sidebar) = buttons(false, out, docked);
                assert!(!(window && (panel || sidebar)), "drawn twice at {out}");
                assert!(window || panel, "missing at {out}");
                assert!(!(gone && window), "the window's own came back at {out}");
                gone |= !window;
            }
            assert!(gone);
        }
    }

    /// Fingers let go short of halfway take the Library back: the window's own never show, and the
    /// copies are all there is.
    #[test]
    fn a_swipe_let_go_short_keeps_the_windows_own_buttons_away() {
        for step in 0..=500 {
            let (window, panel, sidebar) = buttons(false, step as f32 / 1000., true);
            assert!(!window && panel && sidebar, "at {step}");
        }
    }

    /// At rest the window's own show over the open Library and the docked sidebar, and with
    /// neither, nobody draws any.
    #[test]
    fn the_windows_buttons_at_rest() {
        let rest = |open, docked| window_buttons_at(open, false, true, docked);
        assert!(rest(true, false).window && !rest(true, false).library);
        assert!(rest(false, true).window && !rest(false, true).library);
        let none = rest(false, false);
        assert!(!none.window && !none.library && !none.sidebar);
        // Taken past where it opens, the panel is still at rest, its column under them.
        assert!(panel_at_rest(1., 417.) && !panel_at_rest(receding_edge(0.99), 417.));
    }

    fn download(state: DownloadState, held: bool) -> Download {
        Download {
            id: 1,
            name: "report.pdf".into(),
            url: "https://example.com/report.pdf".into(),
            path: "/Users/someone/Downloads/report.pdf".into(),
            received: 0,
            total: 0,
            state,
            open_when_done: false,
            held,
            started: 0.,
        }
    }

    /// A finished file's menu has Move to Trash after Open and Show in Finder; one still coming,
    /// held, cancelled or failed has no file of its own to move.
    #[test]
    fn only_a_finished_file_goes_to_the_trash() {
        use DownloadState::{Cancelled, Done, Failed, Paused, Running};
        let done = download_menu_items(&download(Done, false), true);
        let titles: Vec<_> = done.iter().map(|(entry, _)| entry.title.as_ref()).collect();
        assert_eq!(
            titles,
            ["Open", "Show in Finder", "Move to Trash", "", "Remove from List", "Clear List"]
        );
        assert_eq!(done[2].1, Some(DownloadChoice::Trash));
        assert_eq!(done[4].1, Some(DownloadChoice::Act(DownloadAction::Remove)));
        let trashes = |state, held| {
            download_menu_items(&download(state, held), true)
                .iter()
                .any(|(_, choice)| *choice == Some(DownloadChoice::Trash))
        };
        for state in [Running, Paused, Cancelled, Failed] {
            assert!(!trashes(state, false));
            assert!(!trashes(state, true));
        }
        assert!(!trashes(Done, true));
    }

    fn running(received: i64, total: i64) -> Download {
        Download { received, total, ..download(DownloadState::Running, false) }
    }

    /// The footer's bar is all the running downloads' progress together, and only they have one.
    #[test]
    fn the_footer_bar_follows_the_running_downloads() {
        let done = download(DownloadState::Done, false);
        assert_eq!(downloads_progress(&[done.clone()]), None);
        assert_eq!(downloads_progress(&[]), None);
        assert_eq!(downloads_progress(&[running(50, 100), done.clone()]), Some(0.5));
        assert_eq!(downloads_progress(&[running(50, 100), running(0, 100)]), Some(0.25));
        // A size not yet known shows an empty bar, and a download is never past its end.
        assert_eq!(downloads_progress(&[running(7, 0)]), Some(0.));
        assert_eq!(downloads_progress(&[running(300, 100)]), Some(1.));
    }

    /// A download's report that moves the bar less than it can draw is not worth a frame, while
    /// one that finishes it, or starts another, is.
    #[test]
    fn the_bar_draws_a_frame_only_when_it_moves() {
        let gigabyte = 1_000_000_000;
        let bar = |received| bar_drawn(&[running(received, gigabyte)]);
        assert_eq!(bar(1_000), bar(2_000));
        assert_ne!(bar(0), bar(gigabyte / 4));
        assert_eq!(bar(gigabyte), Some(64));
        assert_eq!(bar_drawn(&[download(DownloadState::Done, false)]), None);
        assert_ne!(bar_drawn(&[running(0, 0)]), None);
    }
}
