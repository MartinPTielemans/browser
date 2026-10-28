use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, KeyDownEvent, Keystroke, Modifiers, Pixels, Point, TestAppContext,
    VisualTestContext, point, px, size,
};

use super::{
    CHECK, GAP, Key, Layout, Look, Outcome, PADDING, Pointer, ROW_X, Request, Swatch, Tracking, contrast,
    described, key_of, lay_out, lay_out_over, swatch_ring,
};
use crate::shell::MenuEntry;

/// Seven points a character, near enough to SF's 13 pt for where things go.
fn width_of(text: &str) -> f32 {
    7. * text.chars().count() as f32
}

fn area() -> Bounds<Pixels> {
    Bounds::new(point(px(0.), px(0.)), size(px(1280.), px(800.)))
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

fn item(title: &str) -> MenuEntry {
    MenuEntry::new(title.to_string(), true)
}

fn under(parent: usize, title: &str) -> MenuEntry {
    MenuEntry { parent: Some(parent), ..item(title) }
}

/// A tab's menu as the sidebar has it, with a dimmed item, lines and a submenu.
fn tab_menu() -> Vec<MenuEntry> {
    vec![
        item("Copy Link"),                        // 0
        MenuEntry::line(),                        // 1
        item("Rename…"),                          // 2
        MenuEntry::new("Mute".to_string(), false), // 3
        item("Duplicate"),                        // 4
        item("Move to"),                          // 5
        MenuEntry { checked: true, ..under(5, "Space 1") }, // 6
        under(5, "reading list"),                 // 7
        under(5, "New Space…"),                   // 8
        MenuEntry::line(),                        // 9
        item("Archive Tab"),                      // 10
        item("Archive Tabs Below"),               // 11
    ]
}

fn opened(entries: Vec<MenuEntry>, x: f32, y: f32) -> Tracking {
    let layout = lay_out(&entries, at(x, y), area(), &width_of);
    Tracking::new(entries, layout, at(x, y))
}

/// The middle of the row of `entry` in the open level holding it.
fn row_of(tracking: &Tracking, entry: usize) -> Point<Pixels> {
    let level = (tracking.layout.levels.iter()).find(|level| level.items.contains(&entry)).unwrap();
    let row = level.items.iter().position(|&each| each == entry).unwrap();
    level.row_bounds(row, &tracking.entries).center()
}

fn level_of(layout: &Layout, parent: Option<usize>) -> Bounds<Pixels> {
    layout.levels.iter().find(|level| level.parent == parent).unwrap().bounds
}

#[test]
fn the_arrows_walk_the_items_skipping_lines_and_dimmed_ones_and_stop_at_the_ends() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Down, now);
    assert_eq!(menu.highlighted(), ["Copy Link"]);
    menu.key(Key::Down, now);
    assert_eq!(menu.highlighted(), ["Rename…"]);
    menu.key(Key::Down, now);
    assert_eq!(menu.highlighted(), ["Duplicate"], "past the dimmed Mute");
    menu.key(Key::Last, now);
    menu.key(Key::Down, now);
    assert_eq!(menu.highlighted(), ["Archive Tabs Below"], "it stays at the end, as NSMenu's does");
    menu.key(Key::First, now);
    menu.key(Key::Up, now);
    assert_eq!(menu.highlighted(), ["Copy Link"]);

    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Up, now);
    assert_eq!(menu.highlighted(), ["Archive Tabs Below"], "up from nothing starts at the bottom");
}

#[test]
fn return_picks_the_highlighted_item_and_escape_closes() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.key(Key::Choose, now), Outcome::Stay, "nothing is highlighted yet");
    menu.key(Key::Down, now);
    assert_eq!(menu.key(Key::Choose, now), Outcome::Pick(0));
    assert_eq!(menu.key(Key::Escape, now), Outcome::Close);
}

#[test]
fn right_and_return_open_a_submenu_and_left_and_escape_close_only_it() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Char('m'), now);
    assert_eq!(menu.highlighted(), ["Move to"]);
    menu.key(Key::Right, now);
    assert_eq!(menu.highlighted(), ["Move to", "Space 1"], "with its first item highlighted");
    menu.key(Key::Down, now);
    assert_eq!(menu.highlighted(), ["Move to", "reading list"]);
    menu.key(Key::Left, now);
    assert_eq!(menu.highlighted(), ["Move to"]);
    assert_eq!(menu.key(Key::Choose, now), Outcome::Stay, "Return on it opens the submenu");
    assert_eq!(menu.highlighted(), ["Move to", "Space 1"]);
    assert_eq!(menu.key(Key::Escape, now), Outcome::Stay);
    assert_eq!(menu.highlighted(), ["Move to"]);
    menu.key(Key::Right, now);
    menu.key(Key::Down, now);
    assert_eq!(menu.key(Key::Choose, now), Outcome::Pick(7));
}

#[test]
fn typing_selects_by_the_titles_start_and_starts_over_after_a_second() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Char('A'), now);
    assert_eq!(menu.highlighted(), ["Archive Tab"], "case doesn't matter");
    menu.key(Key::Char('r'), now + Duration::from_millis(300));
    menu.key(Key::Char('c'), now + Duration::from_millis(600));
    menu.key(Key::Char('h'), now + Duration::from_millis(900));
    menu.key(Key::Char('i'), now + Duration::from_millis(1200));
    menu.key(Key::Char('v'), now + Duration::from_millis(1500));
    menu.key(Key::Char('e'), now + Duration::from_millis(1800));
    menu.key(Key::Char(' '), now + Duration::from_millis(2100));
    menu.key(Key::Char('t'), now + Duration::from_millis(2400));
    menu.key(Key::Char('a'), now + Duration::from_millis(2700));
    menu.key(Key::Char('b'), now + Duration::from_millis(3000));
    menu.key(Key::Char('s'), now + Duration::from_millis(3300));
    assert_eq!(menu.highlighted(), ["Archive Tabs Below"]);
    menu.key(Key::Char('d'), now + Duration::from_millis(4400));
    assert_eq!(menu.highlighted(), ["Duplicate"], "after a second, a key starts a new search");
    menu.key(Key::Char('x'), now + Duration::from_millis(4500));
    assert_eq!(menu.highlighted(), ["Duplicate"], "what matches nothing moves nothing");
}

#[test]
fn the_same_letter_again_goes_to_the_next_item_it_starts() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Char('a'), now);
    menu.key(Key::Char('a'), now + Duration::from_millis(200));
    assert_eq!(menu.highlighted(), ["Archive Tabs Below"]);
    menu.key(Key::Char('a'), now + Duration::from_millis(400));
    assert_eq!(menu.highlighted(), ["Archive Tab"], "round to the first again");
}

#[test]
fn typing_ignores_accents_and_finds_only_what_can_be_chosen() {
    let now = Instant::now();
    let entries = vec![item("Éditer"), MenuEntry::new("Mute".to_string(), false), item("Move")];
    let mut menu = opened(entries, 100., 100.);
    menu.key(Key::Char('e'), now);
    assert_eq!(menu.highlighted(), ["Éditer"]);
    menu.key(Key::Char('m'), now + Duration::from_secs(2));
    assert_eq!(menu.highlighted(), ["Move"], "past the dimmed Mute");
}

#[test]
fn typing_in_an_open_submenu_selects_there() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.key(Key::Char('m'), now);
    menu.key(Key::Right, now);
    menu.key(Key::Char('n'), now + Duration::from_secs(2));
    assert_eq!(menu.highlighted(), ["Move to", "New Space…"]);
}

#[test]
fn a_key_makes_a_character_and_the_arrows_their_moves() {
    let key = |key: &str, key_char: Option<&str>, alt: bool| {
        let keystroke = Keystroke {
            modifiers: Modifiers { alt, ..Modifiers::default() },
            key: key.into(),
            key_char: key_char.map(Into::into),
        };
        key_of(&KeyDownEvent { keystroke, is_held: false, prefer_character_input: false })
    };
    assert_eq!(key("down", None, false), Some(Key::Down));
    assert_eq!(key("up", None, true), Some(Key::First), "⌥↑ goes to the top, as in NSMenu");
    assert_eq!(key("enter", None, false), Some(Key::Choose));
    assert_eq!(key("escape", None, false), Some(Key::Escape));
    assert_eq!(key("r", Some("r"), false), Some(Key::Char('r')));
    assert_eq!(key("a", Some("あ"), false), Some(Key::Char('あ')), "kana selects as itself");
    assert_eq!(key("tab", None, false), None);
}

#[test]
fn a_menu_opens_at_the_pointer_and_flips_where_the_screen_ends() {
    let menu = lay_out(&tab_menu(), at(100., 100.), area(), &width_of);
    let root = level_of(&menu, None);
    assert_eq!(root.origin, at(100., 100.));
    assert_eq!(f32::from(root.size.width), 200., "no narrower than the least");
    // Six rows and two lines, inside the padding.
    assert_eq!(f32::from(root.size.height), 4. + 7. * 24. + 2. * 9. + 4.);

    let menu = lay_out(&tab_menu(), at(1200., 780.), area(), &width_of);
    let root = level_of(&menu, None);
    assert_eq!(root.bottom_right(), at(1200., 780.), "up and to the left of the pointer");

    // A menu taller than the room above and below sits inside the screen, 8 from its edge.
    let short = Bounds::new(point(px(0.), px(0.)), size(px(1280.), px(150.)));
    let menu = lay_out(&tab_menu(), at(100., 60.), short, &width_of);
    assert_eq!(level_of(&menu, None).origin.y, px(8.));
}

#[test]
fn a_submenu_opens_beside_its_row_and_on_the_left_where_the_right_has_no_room() {
    let menu = lay_out(&tab_menu(), at(100., 100.), area(), &width_of);
    let root = level_of(&menu, None);
    let sub = level_of(&menu, Some(5));
    assert_eq!(sub.left(), root.right() - px(4.), "over its parent's edge by 4");
    // Its first row is level with the row that opened it.
    let row_top = 100. + 4. + 4. * 24. + 9.;
    assert_eq!(sub.top(), px(row_top - 4.));

    let menu = lay_out(&tab_menu(), at(1000., 100.), area(), &width_of);
    let root = level_of(&menu, None);
    let sub = level_of(&menu, Some(5));
    assert_eq!(root.left(), px(1000.), "the root fits");
    assert_eq!(sub.right(), root.left() + px(4.), "the submenu goes on the left");

    let menu = lay_out(&tab_menu(), at(100., 700.), area(), &width_of);
    assert!(level_of(&menu, Some(5)).bottom() <= px(792.), "and slides up to fit");
}

#[test]
fn a_long_title_widens_the_menu_to_the_most() {
    let title = "A very long page title that goes on and on and on, as page titles do";
    let menu = lay_out(&[item(title)], at(10., 10.), area(), &width_of);
    assert_eq!(f32::from(level_of(&menu, None).size.width), 320.);
}

#[test]
fn a_release_where_the_menu_opened_leaves_it_open_and_a_drag_to_an_item_picks_it() {
    let mut menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.released(at(102., 101.)), Outcome::Stay, "a right-click, not a drag");
    let rename = row_of(&menu, 2);
    menu.moved(rename, Instant::now());
    assert_eq!(menu.released(rename), Outcome::Stay, "the button is up: only a click picks now");
    assert!(menu.pressed(rename));
    assert_eq!(menu.released(rename), Outcome::Pick(2));

    let mut menu = opened(tab_menu(), 100., 100.);
    let duplicate = row_of(&menu, 4);
    menu.moved(duplicate, Instant::now());
    assert_eq!(menu.released(duplicate), Outcome::Pick(4), "press, drag and release");
}

#[test]
fn a_release_on_what_cant_be_chosen_leaves_the_menu_open() {
    let mut menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.released(row_of(&menu, 3)), Outcome::Stay, "the dimmed Mute");
    let mut menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.released(row_of(&menu, 1)), Outcome::Stay, "a line");
    let mut menu = opened(tab_menu(), 100., 100.);
    let move_to = row_of(&menu, 5);
    menu.moved(move_to, Instant::now());
    assert_eq!(menu.released(move_to), Outcome::Stay, "a submenu's row");
    let space = row_of(&menu, 7);
    menu.moved(space, Instant::now());
    assert!(menu.pressed(space));
    assert_eq!(menu.released(space), Outcome::Pick(7), "in the submenu");
}

#[test]
fn a_drag_released_outside_closes_the_menu_and_a_press_outside_is_not_on_it() {
    let mut menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.released(at(60., 60.)), Outcome::Close);
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.released(at(100., 100.));
    assert!(!menu.pressed(at(600., 600.)));
}

#[test]
fn the_pointer_opens_a_submenu_and_its_row_keeps_it_open() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.moved(row_of(&menu, 5), now);
    assert_eq!(menu.highlights(), [Some(5), None]);
    menu.moved(row_of(&menu, 5) + point(px(20.), px(0.)), now);
    assert_eq!(menu.highlights(), [Some(5), None], "anywhere on the row");
    menu.moved(row_of(&menu, 7), now);
    assert_eq!(menu.highlights(), [Some(5), Some(7)]);
}

#[test]
fn on_its_way_to_the_submenu_the_pointer_crosses_other_rows_without_closing_it() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    let parent = row_of(&menu, 5);
    menu.moved(parent, now);
    let sub = level_of(&menu.layout, Some(5));
    // Down and right, over Archive Tab's row, towards the submenu's lower half.
    let below = row_of(&menu, 10);
    let crossing = point(sub.left() - px(10.), below.y);
    assert!(menu.moved(crossing, now + Duration::from_millis(30)), "in the safe triangle");
    assert_eq!(menu.highlights(), [Some(5), None], "Move to stays highlighted, its submenu open");

    menu.rested(now + Duration::from_millis(100));
    assert_eq!(menu.highlights(), [Some(5), None], "not yet");
    menu.rested(now + Duration::from_millis(340));
    assert_eq!(menu.highlights(), [Some(10)], "resting 300 ms gives the row under it the highlight");
}

#[test]
fn moving_away_from_the_submenu_closes_it_at_once() {
    let now = Instant::now();
    let mut menu = opened(tab_menu(), 100., 100.);
    menu.moved(row_of(&menu, 5), now);
    // Straight down along the left edge, away from the submenu on the right.
    let away = point(px(110.), row_of(&menu, 10).y);
    assert!(!menu.moved(away, now + Duration::from_millis(30)));
    assert_eq!(menu.highlights(), [Some(10)]);
}

#[test]
fn the_driver_chooses_an_item_by_its_title_or_its_submenus() {
    let menu = opened(tab_menu(), 100., 100.);
    assert_eq!(menu.find("Duplicate"), Some(4));
    assert_eq!(menu.find("Move to › reading list"), Some(7));
    assert_eq!(menu.find("reading list"), Some(7));
    assert_eq!(menu.find("Mute"), None, "dimmed");
    assert_eq!(menu.find("Move to"), None, "a submenu's row");
}

#[test]
fn the_log_names_the_items_as_the_systems_menus_did() {
    assert_eq!(
        described(&tab_menu()),
        "Copy Link, Rename…, Mute (disabled), Duplicate, Move to › Space 1 (checked), \
         Move to › reading list, Move to › New Space…, Archive Tab, Archive Tabs Below"
    );
}

/// What the menu asked of its window, in order.
#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<&'static str>>,
}

impl super::Host for Recorder {
    fn show(&self, _: Bounds<Pixels>) {
        self.calls.borrow_mut().push("show");
    }

    fn hide(&self) {
        self.calls.borrow_mut().push("hide");
    }
}

/// Shows the tab menu at (100, 100) and returns what it chose once it closes. Each call is an
/// update of the app, as the host's calls are, not of the menu's window.
fn show_menu(cx: &mut VisualTestContext) -> Rc<Cell<Option<Option<usize>>>> {
    let chosen = Rc::new(Cell::new(None));
    let answer = chosen.clone();
    cx.cx.update(|cx: &mut App| {
        let request = Request {
            items: tab_menu(),
            at: at(100., 100.),
            area: area(),
            look: Look::Dark,
            chosen: Box::new(move |index, _| answer.set(Some(index))),
        };
        super::show(request, cx);
    });
    cx.run_until_parked();
    chosen
}

/// The menu in its window, driven the ways the host drives it: keys through the window, pointer
/// events and the driver's choose and dismiss. Each closes it from inside the menu's update, which
/// must leave its window alone until the update is over, or drawing it finds the menu held.
#[gpui::test]
fn the_host_drives_the_menu_and_the_menu_closes_outside_its_update(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let window = cx.update(|cx| super::open(host.clone(), cx)).expect("no window for the menu");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);

    let chosen = show_menu(cx);
    cx.simulate_keystrokes("m right down enter");
    assert_eq!(chosen.get(), Some(Some(7)), "Move to › reading list");
    assert_eq!(*host.calls.borrow(), ["show", "hide"]);

    // Press and release on Duplicate, in the window's terms, which start 24 left of the menu
    // and 16 above it for its shadow.
    let chosen = show_menu(cx);
    let duplicate = point(px(150. - 76.), px(100. + 85. + 12. - 84.));
    cx.cx.update(|cx: &mut App| {
        super::pointer(Pointer::Released, point(px(26.), px(18.)), cx);
        assert!(super::pointer(Pointer::Moved, duplicate, cx));
        assert!(super::pointer(Pointer::Pressed, duplicate, cx));
        assert!(super::pointer(Pointer::Released, duplicate, cx));
    });
    cx.run_until_parked();
    assert_eq!(chosen.get(), Some(Some(4)));

    let chosen = show_menu(cx);
    assert!(cx.cx.update(|cx: &mut App| super::choose("Archive Tab", cx)));
    cx.run_until_parked();
    assert_eq!(chosen.get(), Some(Some(10)));

    let chosen = show_menu(cx);
    cx.cx.update(|cx: &mut App| super::dismissed(cx));
    cx.run_until_parked();
    assert_eq!(chosen.get(), Some(None));

    // A menu opened over one that shows takes its place and stays.
    show_menu(cx);
    let chosen = show_menu(cx);
    assert_eq!(host.calls.borrow().last(), Some(&"show"));
    cx.simulate_keystrokes("escape");
    assert_eq!(chosen.get(), Some(None));
    assert_eq!(host.calls.borrow().iter().filter(|call| **call == "hide").count(), 6);
}

/// A pop-up button's menu, such as the Space form's Profile: the current choice checked and over
/// the button's title.
fn pop_up() -> Vec<MenuEntry> {
    let checked = |title| MenuEntry { checked: true, over: true, ..item(title) };
    vec![item("Default"), checked("work"), MenuEntry::line(), item("New Profile")]
}

/// The current choice's title goes on the button's, highlighted, and a click on the button, which
/// lands on it, leaves the menu open, as a pop-up button's does.
#[test]
fn a_pop_ups_menu_opens_with_its_choice_over_the_buttons_title() {
    let entries = pop_up();
    let button = at(400., 300.);
    let layout = lay_out_over(&entries, button, area(), &width_of);
    let root = &layout.levels[0];
    let row = root.row_bounds(1, &entries);
    assert_eq!(row.origin.x + px(ROW_X + CHECK + GAP), button.x, "the title's left on the button's");
    assert_eq!(row.origin.y, button.y, "the row's top on the button's text");
    assert_eq!(root.bounds.origin.y, button.y - px(PADDING + super::ROW), "Default above it");

    let mut menu = Tracking::new(entries.clone(), layout, button);
    assert_eq!(menu.highlighted(), ["work"]);
    let release = button + point(px(10.), px(8.));
    assert_eq!(menu.released(release), Outcome::Stay, "a click on the button leaves it open");
    menu.moved(row_of(&menu, 3), Instant::now());
    menu.pressed(row_of(&menu, 3));
    assert_eq!(menu.released(row_of(&menu, 3)), Outcome::Pick(3));

    // Without a choice over the button, it opens at the point as any menu does.
    let plain: Vec<MenuEntry> = entries.into_iter().map(|entry| MenuEntry { over: false, ..entry }).collect();
    assert_eq!(lay_out_over(&plain, button, area(), &width_of), lay_out(&plain, button, area(), &width_of));
}

/// A press-drag from a pop-up button onto another choice picks it, as AppKit's do.
#[test]
fn a_drag_from_a_pop_up_button_picks_where_it_ends() {
    let entries = pop_up();
    let button = at(400., 300.);
    let layout = lay_out_over(&entries, button, area(), &width_of);
    let mut menu = Tracking::new(entries, layout, button);
    let default = row_of(&menu, 0);
    menu.moved(default, Instant::now());
    assert_eq!(menu.released(default), Outcome::Pick(0));
}

/// A Space's swatch gets a ring only where an end of it would fade into the panel (#320): a dark
/// one on the dark panel, a pale one on the light panel, Paper and Ink each reading 3:1 there.
#[test]
fn a_swatch_is_ringed_only_where_its_edge_fades_into_the_panel() {
    let ink_like = Swatch { from: 0x1a1a1a, to: 0x303030 };
    let vivid = Swatch { from: 0x4f94ff, to: 0xff7a3c };
    let pale = Swatch { from: 0xfaf6ea, to: 0xffd9a0 };
    assert_eq!(swatch_ring(vivid, Look::Dark), None);
    assert_eq!(swatch_ring(ink_like, Look::Dark), Some(super::DARK_SWATCH_RING));
    assert_eq!(swatch_ring(pale, Look::Light), Some(super::LIGHT_SWATCH_RING));
    assert_eq!(swatch_ring(ink_like, Look::Light), None);
    for (ring, fill) in [(super::DARK_SWATCH_RING, super::DARK_FILL), (super::LIGHT_SWATCH_RING, super::LIGHT_FILL)] {
        assert!(contrast(ring >> 8, fill >> 8) >= super::SWATCH_CONTRAST, "the ring itself reads");
    }
}

/// Swatches take room before the titles, so a submenu with them is wider by one and a gap.
#[test]
fn swatches_widen_their_submenu() {
    let mut entries = tab_menu();
    // Wider than the narrowest a panel is.
    entries[7].title = "a reading list with a long name".into();
    let plain = level_of(&lay_out(&entries, at(100., 100.), area(), &width_of), Some(5));
    for entry in &mut entries[6..8] {
        entry.swatch = Some(Swatch { from: 0x4f94ff, to: 0xff7a3c });
    }
    let with = level_of(&lay_out(&entries, at(100., 100.), area(), &width_of), Some(5));
    assert_eq!(with.size.width - plain.size.width, px(super::SWATCH + GAP));
}

/// What VoiceOver reads of the tab menu: a menu with the keyboard, its items in order with where
/// each stands, the lines between them, the dimmed one flagged, and the submenu's row saying it
/// opens one. The highlight is what VoiceOver reads as the keys move it, into the submenu too.
#[gpui::test]
fn voiceover_reads_the_menu_and_follows_the_keys(cx: &mut TestAppContext) {
    use gpui::accesskit::{HasPopup, NodeId, Role, Toggled, TreeUpdate};
    let host = Rc::new(Recorder::default());
    let window = cx.update(|cx| super::open(host.clone(), cx)).expect("no window for the menu");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_a11y_activation();
    show_menu(cx);
    let named = |tree: &TreeUpdate, role: Role, label: &str| -> NodeId {
        let found = tree.nodes.iter().find(|(_, node)| node.role() == role && node.label() == Some(label));
        found.unwrap_or_else(|| panic!("no {role:?} called {label}")).0
    };
    let node = |tree: &TreeUpdate, id: NodeId| tree.nodes.iter().find(|(each, _)| *each == id).unwrap().1.clone();
    let focused = |tree: &TreeUpdate| node(tree, tree.focus).label().map(str::to_owned);

    let tree = cx.a11y_tree().expect("GPUI built a tree");
    let menu = named(&tree, Role::Menu, "Menu");
    assert_eq!(tree.focus, menu, "the menu has the keyboard, nothing highlighted yet");
    let rows: Vec<_> = (node(&tree, menu).children().iter())
        .map(|&child| node(&tree, child))
        .map(|row| (row.role(), row.label().unwrap_or("").to_owned()))
        .collect();
    let item = |title: &str| (Role::MenuItem, title.to_owned());
    let line = (Role::Splitter, String::new());
    assert_eq!(
        rows,
        [
            item("Copy Link"),
            line.clone(),
            item("Rename…"),
            item("Mute"),
            item("Duplicate"),
            item("Move to"),
            line,
            item("Archive Tab"),
            item("Archive Tabs Below"),
        ]
    );
    let archive = node(&tree, named(&tree, Role::MenuItem, "Archive Tab"));
    assert_eq!((archive.position_in_set(), archive.size_of_set()), (Some(6), Some(7)));
    assert!(node(&tree, named(&tree, Role::MenuItem, "Mute")).is_disabled());
    let move_to = node(&tree, named(&tree, Role::MenuItem, "Move to"));
    assert_eq!((move_to.has_popup(), move_to.is_expanded()), (Some(HasPopup::Menu), Some(false)));

    cx.simulate_keystrokes("down");
    assert_eq!(focused(&cx.a11y_tree().unwrap()).as_deref(), Some("Copy Link"));
    cx.simulate_keystrokes("down down");
    assert_eq!(focused(&cx.a11y_tree().unwrap()).as_deref(), Some("Duplicate"), "past the line and Mute");

    cx.simulate_keystrokes("m right");
    let tree = cx.a11y_tree().unwrap();
    named(&tree, Role::Menu, "Move to");
    assert_eq!(node(&tree, named(&tree, Role::MenuItem, "Move to")).is_expanded(), Some(true));
    let space = node(&tree, named(&tree, Role::MenuItemRadio, "Space 1"));
    assert_eq!(space.toggled(), Some(Toggled::True));
    assert_eq!(focused(&tree).as_deref(), Some("Space 1"));
}

/// 0xRRGGBBAA `top` over the opaque 0xRRGGBBAA `under`, as 0xRRGGBB.
fn over(top: u32, under: u32) -> u32 {
    let alpha = (top & 0xff) as f32 / 255.;
    let channel = |shift: u32| {
        let (t, u) = (((top >> shift) & 0xff) as f32, ((under >> shift) & 0xff) as f32);
        ((t * alpha + u * (1. - alpha)).round() as u32) << (shift - 8)
    };
    channel(24) | channel(16) | channel(8)
}

/// Every label a menu draws reads at 4.5:1 against what it's on, in both looks, which are all a
/// theme picks of the menu, as the panel is opaque over it. Dimmed items are exempt, as WCAG's
/// rule exempts what can't be used. Once the palette (#335) gives the highlight each Space's
/// accent, this sweeps the accents too.
#[test]
fn every_label_reads_at_4_5_to_1_in_both_looks() {
    use super::{DESTRUCTIVE_FILL, DESTRUCTIVE_LABEL, PRESSED, highlight};
    let (fill, label) = highlight();
    for look in [Look::Dark, Look::Light] {
        let palette = look.palette();
        let panel = palette.fill >> 8;
        for (what, color) in
            [("label", palette.label), ("shortcut", palette.secondary), ("destructive", palette.destructive)]
        {
            let ratio = contrast(over(color, palette.fill), panel);
            assert!(ratio >= 4.5, "{look:?}'s {what} is {ratio:.2}:1");
        }
        for (what, on, text) in [("highlight", fill, label), ("delete", DESTRUCTIVE_FILL, DESTRUCTIVE_LABEL)] {
            let ratio = contrast(over(text, on), over(on, palette.fill));
            assert!(ratio >= 4.5, "{look:?}'s {what} label is {ratio:.2}:1");
        }
        // The press darkens the highlight for as long as the button is down, under 4.5:1 with
        // #324's Ink 0.18, so it's held to 3:1 until the palette (#335) settles it.
        let pressed = over(PRESSED, fill);
        let ratio = contrast(over(label, fill), pressed);
        assert!(ratio >= 3., "{look:?}'s pressed label is {ratio:.2}:1");
    }
}
