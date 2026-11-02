//! The trail's lines, its file's cap, and its writer, without the UI. The bar's events are tested
//! with the bar, in command_bar/tests.rs.

use std::time::Duration;

use super::*;

/// Every event the trail has, so a new one is checked here as it's added.
fn every_event() -> Vec<Event> {
    use {Closes as C, Event as E, KeyTo as K, Opens as O, Window as W};
    let windows = [W::Browser, W::CommandBar, W::Menu, W::Settings];
    let mut events = vec![
        E::Starts,
        E::AppActive,
        E::AppInactive,
        E::BarLosesKeyboard,
        E::BarAsksForKeyboard,
        E::FieldLosesFocus,
        E::FieldTakesFocus,
        E::CompositionStarts,
        E::CompositionEnds,
    ];
    events.extend(windows.map(E::BecomesKey));
    events.extend(windows.map(E::ResignsKey));
    events.extend([O::Centred, O::OverUrlField, O::SearchTabs, O::Split].map(E::BarOpens));
    events.extend([C::Esc, C::Shortcut, C::Choice, C::Left, C::SpaceShown].map(E::BarCloses));
    events.extend([K::Field, K::BarNotField, K::Page].map(E::Key));
    events
}

fn temp_folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("trail-{name}-{}", std::process::id()));
    _ = fs::remove_dir_all(&folder);
    folder
}

/// An event can hold nothing but the trail's own words: it's `Copy`, so it owns no text, and its
/// parts are enums. The words are plain, so none of them reads as a URL, a path or a time.
#[test]
fn a_line_holds_only_the_trails_own_words() {
    fn owns_no_text<T: Copy + 'static>() {}
    owns_no_text::<Event>();
    let words: Vec<_> = every_event().into_iter().map(Event::words).collect();
    for (index, word) in words.iter().enumerate() {
        assert!(
            word.chars().all(|c| c.is_ascii_alphabetic() || c == ' '),
            "{word:?} has more than words"
        );
        assert!(!words[..index].contains(word), "{word:?} is two events' words");
    }
}

#[test]
fn a_line_is_the_local_time_and_the_words() {
    // 2026-11-02 13:20:13.512 UTC.
    let at = SystemTime::UNIX_EPOCH + Duration::from_millis(1_793_625_613_512);
    let line = format_line(&Line { at, event: Event::BarOpens(Opens::Centred) });
    let offset = utc_offset(1_793_625_613);
    let (date, time) = civil(1_793_625_613 + offset);
    assert!(line.starts_with(&format!("{date} {time}.512 ")), "{line}");
    assert!(line.ends_with(" command bar opens centred\n"), "{line}");
    assert_eq!(civil(1_793_625_613), ("2026-11-02".into(), "13:20:13".into()));
    assert_eq!(civil(951_782_400), ("2000-02-29".into(), "00:00:00".into()));
    assert_eq!(civil(-1), ("1969-12-31".into(), "23:59:59".into()));
}

/// Recording only sends: the line is on the channel when it returns, and nothing waits for the
/// writer, which here never reads it.
#[test]
fn recording_only_queues() {
    let queued = capture();
    record(Event::AppActive);
    let line = queued.try_recv().expect("nothing was queued");
    assert_eq!(line.event, Event::AppActive);
}

/// Keys are recorded only while the bar shows, and a run to one place once.
#[test]
fn a_run_of_keys_is_one_line_and_only_while_the_bar_shows() {
    let queued = capture();
    record(Event::Key(KeyTo::Page));
    record(Event::BarOpens(Opens::Centred));
    for _ in 0..5 {
        record(Event::Key(KeyTo::Field));
    }
    record(Event::Key(KeyTo::Page));
    record(Event::Key(KeyTo::Field));
    record(Event::BarCloses(Closes::Esc));
    record(Event::Key(KeyTo::Page));
    let events: Vec<_> = queued.try_iter().map(|line| line.event).collect();
    assert_eq!(
        events,
        [
            Event::BarOpens(Opens::Centred),
            Event::Key(KeyTo::Field),
            Event::Key(KeyTo::Page),
            Event::Key(KeyTo::Field),
            Event::BarCloses(Closes::Esc),
        ]
    );
}

/// The writer writes what was queued once it has it, and ends with the channel.
#[test]
fn the_writer_appends_what_was_queued() {
    let folder = temp_folder("writer");
    let path = folder.join(FILE);
    fs::create_dir_all(&folder).unwrap();
    fs::write(&path, "from before\n").unwrap();
    let (sender, receiver) = channel();
    for event in [Event::Starts, Event::BarOpens(Opens::SearchTabs)] {
        sender.send(Line { at: SystemTime::now(), event }).unwrap();
    }
    drop(sender);
    write_from(receiver, Capped::new(path.clone(), CAP));
    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert_eq!(lines[0], "from before");
    assert!(lines[1].ends_with(" app starts"));
    assert!(lines[2].ends_with(" command bar opens to search tabs"));
    _ = fs::remove_dir_all(&folder);
}

/// Past the cap the file moves aside, replacing the previous one, so the trail keeps at most two
/// files, neither over the cap.
#[test]
fn the_file_moves_aside_at_its_cap_keeping_one_before_it() {
    let folder = temp_folder("cap");
    let path = folder.join(FILE);
    let line = "0123456789\n";
    let mut file = Capped::new(path.clone(), 50);
    for _ in 0..4 {
        file.push(line);
    }
    file.flush();
    assert_eq!(fs::read_to_string(&path).unwrap(), line.repeat(4));
    assert!(!previous(&path).exists());

    file.push(line);
    file.push("second\n");
    file.flush();
    assert_eq!(fs::read_to_string(previous(&path)).unwrap(), line.repeat(4));
    assert_eq!(fs::read_to_string(&path).unwrap(), format!("{line}second\n"));

    for _ in 0..4 {
        file.push(line);
    }
    file.flush();
    assert_eq!(fs::read_to_string(previous(&path)).unwrap(), format!("{line}second\n{line}{line}"));
    assert_eq!(fs::read_to_string(&path).unwrap(), line.repeat(2));
    let names: Vec<_> = fs::read_dir(&folder).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(names.len(), 2);

    // The next launch goes on from the file's size, not from nothing.
    let mut file = Capped::new(path.clone(), 50);
    file.push(&line.repeat(3));
    file.flush();
    assert_eq!(fs::read_to_string(&path).unwrap(), line.repeat(3));
    assert_eq!(fs::read_to_string(previous(&path)).unwrap(), line.repeat(2));
    _ = fs::remove_dir_all(&folder);
}

/// Deleting the file while the app runs is safe: the trail starts a new one.
#[test]
fn a_deleted_file_starts_again() {
    let folder = temp_folder("deleted");
    let path = folder.join(FILE);
    let mut file = Capped::new(path.clone(), CAP);
    file.push("first\n");
    file.flush();
    fs::remove_file(&path).unwrap();
    file.start_again_if_deleted();
    file.push("second\n");
    file.flush();
    assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
    _ = fs::remove_dir_all(&folder);
}
