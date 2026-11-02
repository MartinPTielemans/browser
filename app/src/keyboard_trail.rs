//! The keyboard trail: a short local file of where the keyboard went, for a command bar that
//! stopped taking typing (#1314) to be read after the fact, when the user says when it happened.
//!
//! A line is a wall-clock time and the words of one `Event`, and an `Event` is made of enums
//! alone, so no URL, title, typed text, key, query, profile or path can get into the file: there
//! is no field to carry one. Nothing reads the file but the user, and nothing sends it anywhere.
//!
//! The UI thread only queues: `record` sends the event down a channel, and a thread of the trail's
//! own formats, writes and flushes what has come, waiting on the channel between, with no timer.

use std::{
    cell::{Cell, RefCell},
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender, channel},
    time::SystemTime,
};

/// How large the file grows before it's moved aside, to the one previous file kept.
pub const CAP: u64 = 256 * 1024;

/// The file's name in the folder `start` is given.
pub const FILE: &str = "keyboard.log";

/// The previous file's, which the next rotation replaces.
pub const PREVIOUS: &str = "keyboard.1.log";

/// A window by its kind, never its title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Browser,
    CommandBar,
    Menu,
    Settings,
}

/// Where the command bar opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opens {
    Centred,
    OverUrlField,
    SearchTabs,
    Split,
}

/// Why the command bar closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closes {
    Esc,
    /// ⌘T or ⌘L again.
    Shortcut,
    /// A row taken, or ↩.
    Choice,
    /// The host's word: a click outside it, another window or another app.
    Left,
    /// The window it showed over moved to another Space, which a new pane's bar doesn't follow.
    SpaceShown,
}

/// Where a key pressed while the command bar shows went. Which key, never.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyTo {
    Field,
    /// The bar's window, with its field not focused.
    BarNotField,
    Page,
}

/// What the trail records: keyboard, focus and window events, from a closed set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Starts,
    AppActive,
    AppInactive,
    BecomesKey(Window),
    ResignsKey(Window),
    BarOpens(Opens),
    BarCloses(Closes),
    /// The bar's window stopped being key while the bar shows and nothing closed it.
    BarLosesKeyboard,
    /// It asked the host for the keyboard back, which its window becoming key says it got.
    BarAsksForKeyboard,
    FieldLosesFocus,
    FieldTakesFocus,
    CompositionStarts,
    CompositionEnds,
    Key(KeyTo),
}

impl Event {
    /// The line's words, all of them written here.
    pub fn words(self) -> &'static str {
        use {Closes as C, Event as E, KeyTo as K, Opens as O, Window as W};
        match self {
            E::Starts => "app starts",
            E::AppActive => "app becomes active",
            E::AppInactive => "app resigns active",
            E::BecomesKey(W::Browser) => "browser window becomes key",
            E::BecomesKey(W::CommandBar) => "command bar window becomes key",
            E::BecomesKey(W::Menu) => "menu window becomes key",
            E::BecomesKey(W::Settings) => "settings window becomes key",
            E::ResignsKey(W::Browser) => "browser window resigns key",
            E::ResignsKey(W::CommandBar) => "command bar window resigns key",
            E::ResignsKey(W::Menu) => "menu window resigns key",
            E::ResignsKey(W::Settings) => "settings window resigns key",
            E::BarOpens(O::Centred) => "command bar opens centred",
            E::BarOpens(O::OverUrlField) => "command bar opens over the URL field",
            E::BarOpens(O::SearchTabs) => "command bar opens to search tabs",
            E::BarOpens(O::Split) => "command bar opens for a split view",
            E::BarCloses(C::Esc) => "command bar closes on Esc",
            E::BarCloses(C::Shortcut) => "command bar closes on its shortcut",
            E::BarCloses(C::Choice) => "command bar closes on a choice",
            E::BarCloses(C::Left) => "command bar closes as the user leaves it",
            E::BarCloses(C::SpaceShown) => "command bar closes as its window shows another Space",
            E::BarLosesKeyboard => "command bar loses the keyboard while it shows",
            E::BarAsksForKeyboard => "command bar asks for the keyboard back",
            E::FieldLosesFocus => "command bar field loses the focus while it shows",
            E::FieldTakesFocus => "command bar field takes the focus",
            E::CompositionStarts => "input method starts composing in the command bar",
            E::CompositionEnds => "input method stops composing in the command bar",
            E::Key(K::Field) => "a key reaches the command bar field",
            E::Key(K::BarNotField) => "a key reaches the command bar but not its field",
            E::Key(K::Page) => "a key reaches the page while the command bar shows",
        }
    }
}

/// An event and when it happened.
pub struct Line {
    pub at: SystemTime,
    pub event: Event,
}

thread_local! {
    /// The writer's end of the channel, on the UI thread, where every event is recorded.
    static SINK: RefCell<Option<Sender<Line>>> = const { RefCell::new(None) };
    /// The last event recorded, so a run of keys to one place is one line, and whether the bar
    /// shows, outside which keys aren't recorded at all.
    static LAST: Cell<Option<Event>> = const { Cell::new(None) };
    static BAR_SHOWS: Cell<bool> = const { Cell::new(false) };
}

/// Starts the trail in `folder`, with its writer on a thread of its own, from the UI thread.
pub fn start(folder: PathBuf) {
    let (sender, receiver) = channel();
    let spawned = std::thread::Builder::new()
        .name("keyboard trail".into())
        .spawn(move || write_from(receiver, Capped::new(folder.join(FILE), CAP)));
    if let Err(error) = spawned {
        crate::log!("browser: no keyboard trail: {error}");
        return;
    }
    SINK.set(Some(sender));
    record(Event::Starts);
}

/// Queues `event` for the writer, and returns. Keys are recorded only while the bar shows, and
/// one in a run that goes to one place.
pub fn record(event: Event) {
    match event {
        Event::BarOpens(_) => BAR_SHOWS.set(true),
        Event::BarCloses(_) => BAR_SHOWS.set(false),
        Event::Key(_) if !BAR_SHOWS.get() || LAST.get() == Some(event) => return,
        _ => {}
    }
    LAST.set(Some(event));
    SINK.with_borrow(|sink| {
        if let Some(sink) = sink {
            _ = sink.send(Line { at: SystemTime::now(), event });
        }
    });
}

/// Records the window of `kind` that `window` is becoming and resigning key, for its view.
pub fn watch<V: 'static>(kind: Window, window: &mut gpui::Window, cx: &mut gpui::Context<V>) {
    cx.observe_window_activation(window, move |_, window, _| {
        record(match window.is_window_active() {
            true => Event::BecomesKey(kind),
            false => Event::ResignsKey(kind),
        })
    })
    .detach();
}

/// Records what this thread's events are, in place of a writer, for the tests.
#[cfg(test)]
pub fn capture() -> Receiver<Line> {
    let (sender, receiver) = channel();
    SINK.set(Some(sender));
    LAST.set(None);
    BAR_SHOWS.set(false);
    receiver
}

/// The writer's loop: it waits for a line, takes every other that has come meanwhile, and writes
/// them at once.
fn write_from(lines: Receiver<Line>, mut file: Capped) {
    while let Ok(line) = lines.recv() {
        file.start_again_if_deleted();
        file.push(&format_line(&line));
        while let Ok(line) = lines.try_recv() {
            file.push(&format_line(&line));
        }
        file.flush();
    }
}

/// "2026-11-02 14:20:13.512 +01:00 command bar opens centred", in the system's time zone.
pub fn format_line(line: &Line) -> String {
    let since = line.at.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let utc = since.as_secs() as i64;
    let offset = utc_offset(utc);
    let (date, time) = civil(utc + offset);
    let sign = if offset < 0 { '-' } else { '+' };
    let offset = offset.unsigned_abs() / 60;
    let ms = since.subsec_millis();
    let words = line.event.words();
    format!("{date} {time}.{ms:03} {sign}{:02}:{:02} {words}\n", offset / 60, offset % 60)
}

/// The date and the time of day at `seconds` since 1970 in some zone, by Howard Hinnant's
/// days-to-civil algorithm.
fn civil(seconds: i64) -> (String, String) {
    let (days, secs) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = format!("{year:04}-{month:02}-{day:02}");
    let time = format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60);
    (date, time)
}

/// The system time zone's offset from UTC at `utc`, in seconds.
#[cfg(unix)]
fn utc_offset(utc: i64) -> i64 {
    use std::ffi::{c_char, c_int, c_long};
    // The fields every Unix's struct tm starts with, and the BSDs' and glibc's offset after them.
    #[repr(C)]
    struct Tm {
        fields: [c_int; 9],
        gmtoff: c_long,
        zone: *const c_char,
    }
    unsafe extern "C" {
        fn localtime_r(time: *const i64, tm: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm { fields: [0; 9], gmtoff: 0, zone: std::ptr::null() };
    match unsafe { localtime_r(&utc, &mut tm) }.is_null() {
        true => 0,
        false => tm.gmtoff as i64,
    }
}

/// The system time zone's offset from UTC now, which is near enough at `_utc`, written a moment
/// after it.
#[cfg(windows)]
fn utc_offset(_utc: i64) -> i64 {
    #[repr(C)]
    #[derive(Default)]
    struct WinTime {
        year: u16,
        month: u16,
        weekday: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        ms: u16,
    }
    unsafe extern "system" {
        fn GetSystemTime(time: *mut WinTime);
        fn GetLocalTime(time: *mut WinTime);
    }
    let (mut utc, mut local) = (WinTime::default(), WinTime::default());
    unsafe {
        GetSystemTime(&mut utc);
        GetLocalTime(&mut local);
    }
    // Days since 1970 by the same algorithm `civil` inverts, to the minute.
    let minutes = |t: &WinTime| {
        let (y, m, d) = (i64::from(t.year), i64::from(t.month), i64::from(t.day));
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        days * 1440 + i64::from(t.hour) * 60 + i64::from(t.minute)
    };
    (minutes(&local) - minutes(&utc)) * 60
}

/// A file that grows to `cap` bytes, then moves to the previous file's name, replacing it, and
/// starts again: the last 256 KB to 512 KB of the trail are kept.
pub struct Capped {
    path: PathBuf,
    cap: u64,
    file: Option<fs::File>,
    size: u64,
    pending: Vec<u8>,
}

impl Capped {
    pub fn new(path: PathBuf, cap: u64) -> Self {
        Capped { path, cap, file: None, size: 0, pending: Vec::new() }
    }

    /// Adds `line`, to be written at the next `flush`, moving the file aside first if it would
    /// grow past the cap.
    pub fn push(&mut self, line: &str) {
        self.open();
        let size = self.size + self.pending.len() as u64;
        if size > 0 && size + line.len() as u64 > self.cap {
            self.flush();
            self.rotate();
        }
        self.pending.extend_from_slice(line.as_bytes());
    }

    /// Writes what was pushed. A trail that can't be written is lost, never the app's trouble.
    pub fn flush(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        if let Some(file) = self.open()
            && file.write_all(&pending).is_ok()
        {
            self.size += pending.len() as u64;
        }
    }

    /// A file the user deleted is written again from nothing, rather than to the deleted file.
    pub fn start_again_if_deleted(&mut self) {
        if self.file.is_some() && !self.path.exists() {
            self.file = None;
        }
    }

    fn open(&mut self) -> Option<&mut fs::File> {
        if self.file.is_none() {
            if let Some(folder) = self.path.parent() {
                _ = fs::create_dir_all(folder);
            }
            let file = fs::OpenOptions::new().create(true).append(true).open(&self.path).ok()?;
            self.size = file.metadata().map_or(0, |metadata| metadata.len());
            self.file = Some(file);
        }
        self.file.as_mut()
    }

    fn rotate(&mut self) {
        self.file = None;
        _ = fs::rename(&self.path, previous(&self.path));
        self.size = 0;
    }
}

/// The previous file beside the trail at `path`.
pub fn previous(path: &Path) -> PathBuf {
    path.with_file_name(PREVIOUS)
}

#[cfg(test)]
mod tests;
