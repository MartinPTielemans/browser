//! ⌘Z and ⇧⌘Z on a host that records what the shell asks of the core: each action the user can take
//! that loses or moves something is undone, and done again, at the place it was, and the core's
//! reports are played by the test, as the core plays them a moment after each call.

use std::{collections::HashMap, rc::Rc};

use gpui::{App, AppContext, Entity, TestAppContext};

use crate::shell::{
    Archived, Command, Folders, Shell, Split,
    recorder::Recorder,
    undo::{Prop, Value},
};

/// Two Spaces in one profile, the first shown, with the given tabs in the first.
fn window(
    cx: &mut TestAppContext,
    pinned: Vec<u64>,
    today: Vec<u64>,
    folders: Folders,
    splits: Vec<Split>,
) -> (Entity<Shell>, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::new(host.clone(), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(
            vec![
                (1, "One".into(), None, 1, "".into(), "".into()),
                (2, "Two".into(), None, 1, "".into(), "".into()),
                (3, "Three".into(), None, 2, "".into(), "".into()),
            ],
            cx,
        );
        shell.set_space(1, cx);
        shell.set_tabs(1, pinned, today, splits, folders, cx);
    });
    (shell, host)
}

fn plain(cx: &mut TestAppContext, today: Vec<u64>) -> (Entity<Shell>, Rc<Recorder>) {
    window(cx, Vec::new(), today, Folders::default(), Vec::new())
}

/// The core reports the first Space's tabs.
fn report(shell: &Entity<Shell>, cx: &mut TestAppContext, pinned: Vec<u64>, today: Vec<u64>) {
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, pinned, today, Vec::new(), Folders::default(), cx)
    });
}

fn undo(shell: &Entity<Shell>, cx: &mut TestAppContext) -> bool {
    shell.update(cx, |shell, cx| shell.undo(cx))
}

fn redo(shell: &Entity<Shell>, cx: &mut TestAppContext) -> bool {
    shell.update(cx, |shell, cx| shell.redo(cx))
}

fn titles(
    shell: &Entity<Shell>,
    cx: &mut TestAppContext,
) -> (Option<&'static str>, Option<&'static str>) {
    shell.read_with(cx, |shell, _| shell.history.titles())
}

#[gpui::test]
fn a_closed_tab_reopens_and_closes_again_under_its_new_id(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10, 11, 12]);
    shell.update(cx, |shell, cx| shell.close_tab(11, false, cx));
    assert_eq!(host.take_edits(), ["close 11"]);
    report(&shell, cx, Vec::new(), vec![10, 12]);

    assert!(undo(&shell, cx));
    assert_eq!(host.take_edits(), ["reopen 11 as 1001"]);
    report(&shell, cx, Vec::new(), vec![10, 1001, 12]);

    // Done again, it's the tab that came back it closes.
    assert!(redo(&shell, cx));
    assert_eq!(host.take_edits(), ["close 1001"]);
    report(&shell, cx, Vec::new(), vec![10, 12]);
    assert!(undo(&shell, cx));
    assert_eq!(host.take_edits(), ["reopen 1001 as 1002"]);
}

#[gpui::test]
fn the_edit_menu_names_what_it_would_undo_and_redo(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10, 11]);
    assert_eq!(*host.edit_titles.borrow(), (None, None));
    shell.update(cx, |shell, cx| shell.close_tab(11, false, cx));
    assert_eq!(*host.edit_titles.borrow(), (Some("Close Tab".into()), None));
    report(&shell, cx, Vec::new(), vec![10]);
    undo(&shell, cx);
    assert_eq!(*host.edit_titles.borrow(), (None, Some("Close Tab".into())));
    report(&shell, cx, Vec::new(), vec![10, 1001]);
    redo(&shell, cx);
    assert_eq!(*host.edit_titles.borrow(), (Some("Close Tab".into()), None));
}

#[gpui::test]
fn closing_the_tabs_below_brings_back_the_top_one_first(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![1, 2, 3, 4]);
    shell.update(cx, |shell, cx| shell.close_tabs_below(2, cx));
    assert_eq!(host.take_edits(), ["close below 2"]);
    assert_eq!(host.undo_toasts.borrow().as_slice(), ["Tabs Closed"]);
    report(&shell, cx, Vec::new(), vec![1, 2]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["reopen 3 as 1001", "reopen 4 as 1002"]);
    report(&shell, cx, Vec::new(), vec![1, 2, 1001, 1002]);
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["close 1001", "close 1002"]);
}

#[gpui::test]
fn a_pinned_tab_keeps_its_row_so_closing_it_has_nothing_to_undo(cx: &mut TestAppContext) {
    let (shell, host) = window(cx, vec![5], vec![6], Folders::default(), Vec::new());
    shell.update(cx, |shell, cx| shell.close_tab(5, false, cx));
    host.take_edits();
    assert!(!undo(&shell, cx));
    assert!(host.take_edits().is_empty());
}

#[gpui::test]
fn closing_a_split_brings_all_its_tabs_back(cx: &mut TestAppContext) {
    let split = Split {
        tabs: vec![10, 11],
        ..Default::default()
    };
    let (shell, host) = window(
        cx,
        Vec::new(),
        vec![10, 11, 12],
        Folders::default(),
        vec![split],
    );
    shell.update(cx, |shell, cx| shell.close_tab(10, true, cx));
    // Both panes closed: the core keeps each with its partner, so each comes back.
    assert_eq!(host.take_edits(), ["close 10"]);
    report(&shell, cx, Vec::new(), vec![12]);
    undo(&shell, cx);
    assert_eq!(
        host.take_edits(),
        ["reopen 10 as 1001", "reopen 11 as 1002"]
    );
}

#[gpui::test]
fn removing_a_favorite_is_said_in_the_toast_and_undone_to_its_place_and_url(
    cx: &mut TestAppContext,
) {
    let (shell, host) = plain(cx, vec![30]);
    shell.update(cx, |shell, cx| {
        shell.set_favorites(vec![(20, 1), (21, 1), (22, 1)], cx)
    });
    host.pinned_urls
        .borrow_mut()
        .insert(21, "https://a.example/".into());
    shell.update(cx, |shell, cx| shell.move_tab_to(21, false, 0, 0, cx));
    assert_eq!(host.take_edits(), ["move 21 false 0 0"]);
    assert_eq!(
        host.undo_toasts.borrow().as_slice(),
        ["Removed from Favorites"]
    );
    assert_eq!(titles(&shell, cx).0, Some("Remove from Favorites"));

    // The core has made it the first Today tab, which forgot the URL.
    shell.update(cx, |shell, cx| {
        shell.set_favorites(vec![(20, 1), (22, 1)], cx)
    });
    report(&shell, cx, Vec::new(), vec![21, 30]);
    host.pinned_urls.borrow_mut().clear();
    assert!(undo(&shell, cx));
    assert_eq!(
        host.take_edits(),
        ["favorite 21 1", "pinned url 21 https://a.example/"]
    );

    shell.update(cx, |shell, cx| {
        shell.set_favorites(vec![(20, 1), (21, 1), (22, 1)], cx)
    });
    report(&shell, cx, Vec::new(), vec![30]);
    assert!(redo(&shell, cx));
    assert_eq!(host.take_edits(), ["move 21 false 0 0"]);
}

#[gpui::test]
fn the_toasts_undo_button_does_what_command_z_does_and_goes(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![30]);
    shell.update(cx, |shell, cx| {
        shell.set_favorites(vec![(20, 1), (21, 1)], cx)
    });
    shell.update(cx, |shell, cx| shell.move_tab_to(21, false, 0, 0, cx));
    host.take_edits();
    shell.update(cx, |shell, cx| shell.set_favorites(vec![(20, 1)], cx));
    report(&shell, cx, Vec::new(), vec![21, 30]);

    let button = host
        .undo_toast
        .borrow()
        .clone()
        .expect("the toast offers Undo");
    cx.update(|cx: &mut App| button(cx));
    assert_eq!(host.take_edits(), ["favorite 21 1"]);
    assert!(
        host.undo_toast.borrow().is_none(),
        "the offer goes once it's taken"
    );
}

#[gpui::test]
fn the_next_thing_the_user_does_takes_the_offer_away(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![30, 31]);
    shell.update(cx, |shell, cx| {
        shell.set_favorites(vec![(20, 1), (21, 1)], cx)
    });
    shell.update(cx, |shell, cx| shell.move_tab_to(21, false, 0, 0, cx));
    assert!(host.undo_toast.borrow().is_some());
    // Its button would undo this instead, under words that name the other.
    shell.update(cx, |shell, cx| shell.close_tab(31, false, cx));
    assert!(host.undo_toast.borrow().is_none());
}

#[gpui::test]
fn pinning_and_unpinning_are_undone_to_the_same_row(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![9, 10, 11]);
    shell.update(cx, |shell, cx| shell.pin_tab(10, true, cx));
    assert_eq!(host.take_edits(), ["pinned 10 true"]);
    report(&shell, cx, vec![10], vec![9, 11]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["move 10 false 0 1"]);
    report(&shell, cx, Vec::new(), vec![9, 10, 11]);

    // Unpinned, it forgets the URL it went back to, which the undo gives it again.
    report(&shell, cx, vec![10], vec![9, 11]);
    host.pinned_urls
        .borrow_mut()
        .insert(10, "https://p.example/".into());
    shell.update(cx, |shell, cx| shell.pin_tab(10, false, cx));
    assert_eq!(host.take_edits(), ["pinned 10 false"]);
    report(&shell, cx, Vec::new(), vec![10, 9, 11]);
    host.pinned_urls.borrow_mut().clear();
    undo(&shell, cx);
    assert_eq!(
        host.take_edits(),
        ["move 10 true 0 0", "pinned url 10 https://p.example/"]
    );
}

#[gpui::test]
fn a_drag_is_undone_to_where_the_tab_was_in_its_folder(cx: &mut TestAppContext) {
    let folders = Folders {
        folders: HashMap::from([(5, ("Reading".into(), true))]),
        parents: HashMap::from([(6, 5), (7, 5), (8, 5)]),
    };
    let (shell, host) = window(cx, vec![4, 5, 6, 7, 8], vec![9], folders, Vec::new());
    // 8 goes out of the folder to the top of Today.
    shell.update(cx, |shell, cx| shell.move_tab_to(8, false, 0, 0, cx));
    host.take_edits();
    report(&shell, cx, vec![4, 5, 6, 7], vec![8, 9]);
    undo(&shell, cx);
    // Third of the folder's, counted without the folder's own row.
    assert_eq!(host.take_edits(), ["move 8 true 5 2"]);
}

#[gpui::test]
fn moving_a_tab_to_a_space_of_the_same_profile_is_undone_but_not_to_another_profile(
    cx: &mut TestAppContext,
) {
    let (shell, host) = plain(cx, vec![10, 11]);
    shell.update(cx, |shell, cx| shell.send_to_space(11, 2, cx));
    assert_eq!(host.take_edits(), ["space 11 2"]);
    assert_eq!(host.undo_toasts.borrow().as_slice(), ["Tab Moved to Space"]);
    report(&shell, cx, Vec::new(), vec![10]);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(2, Vec::new(), vec![11], Vec::new(), Folders::default(), cx)
    });
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["space 11 1", "move 11 false 0 1"]);

    // Another profile's Space opens it there as a new tab, which has nothing to give back.
    let (shell, host) = plain(cx, vec![10, 11]);
    shell.update(cx, |shell, cx| shell.send_to_space(11, 3, cx));
    host.take_edits();
    assert!(!undo(&shell, cx));
    assert!(host.take_edits().is_empty());
}

#[gpui::test]
fn a_deleted_folder_is_made_again_with_what_it_held_and_deleted_again_by_its_new_id(
    cx: &mut TestAppContext,
) {
    let folders = Folders {
        folders: HashMap::from([(5, ("Reading".into(), false))]),
        parents: HashMap::from([(6, 5), (7, 5)]),
    };
    let (shell, host) = window(cx, vec![4, 5, 6, 7], Vec::new(), folders, Vec::new());
    shell.update(cx, |shell, cx| shell.delete_folder_now(5, cx));
    assert_eq!(host.take_edits(), ["delete folder 5"]);
    assert_eq!(host.undo_toasts.borrow().as_slice(), ["Folder Deleted"]);
    report(&shell, cx, vec![4, 6, 7], Vec::new());

    undo(&shell, cx);
    assert_eq!(
        host.take_edits(),
        [
            "folder 1001 1 Reading 0 1",
            "move 6 true 1001 0",
            "move 7 true 1001 1"
        ]
    );
    let folders = Folders {
        folders: HashMap::from([(1001, ("Reading".into(), true))]),
        parents: HashMap::from([(6, 1001), (7, 1001)]),
    };
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, vec![4, 1001, 6, 7], Vec::new(), Vec::new(), folders, cx)
    });
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["delete folder 1001"]);
}

#[gpui::test]
fn leaving_a_split_is_undone_by_joining_the_tab_beside_it(cx: &mut TestAppContext) {
    let split = Split {
        tabs: vec![10, 11, 12],
        ..Default::default()
    };
    let (shell, host) = window(
        cx,
        Vec::new(),
        vec![10, 11, 12],
        Folders::default(),
        vec![split],
    );
    shell.update(cx, |shell, cx| shell.separate_tab(12, cx));
    assert_eq!(host.take_edits(), ["separate 12"]);
    report(&shell, cx, Vec::new(), vec![10, 11, 12]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["split 12 11 false"]);

    // The first one rejoins at the left of the one after it.
    shell.update(cx, |shell, cx| {
        shell.set_tabs(
            1,
            Vec::new(),
            vec![10, 11, 12],
            vec![Split {
                tabs: vec![10, 11],
                ..Default::default()
            }],
            Folders::default(),
            cx,
        )
    });
    shell.update(cx, |shell, cx| shell.separate_tab(10, cx));
    host.take_edits();
    report(&shell, cx, Vec::new(), vec![10, 11, 12]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["split 10 11 true"]);
}

#[gpui::test]
fn names_icons_and_colours_go_back_to_what_they_were(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10]);
    host.names.borrow_mut().insert(10, "Old".into());
    shell.update(cx, |shell, cx| {
        shell.change(Prop::TabName(10), Value::Text("New".into()), cx);
        shell.change(Prop::SpaceName(2), Value::Text("Work".into()), cx);
        shell.change(Prop::SpaceColor(2), Value::Color(Some(0xff0000)), cx);
        shell.change(Prop::SpaceIcon(2), Value::Text("🌱".into()), cx);
    });
    assert_eq!(
        host.take_edits(),
        [
            "rename tab 10 New",
            "rename space 2 Work",
            "color 2 Some(16711680)",
            "space icon 2 🌱"
        ]
    );
    host.names.borrow_mut().insert(10, "New".into());
    for _ in 0..4 {
        undo(&shell, cx);
    }
    assert_eq!(
        host.take_edits(),
        [
            "space icon 2 ",
            "color 2 None",
            "rename space 2 Two",
            "rename tab 10 Old"
        ]
    );
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["rename tab 10 New"]);
}

#[gpui::test]
fn naming_something_what_it_is_already_called_is_not_a_step(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10]);
    shell.update(cx, |shell, cx| {
        shell.change(Prop::SpaceName(1), Value::Text("One".into()), cx)
    });
    assert!(host.take_edits().is_empty());
    assert!(!undo(&shell, cx));
}

#[gpui::test]
fn a_renamed_tab_that_closed_and_came_back_is_renamed_back_under_its_new_id(
    cx: &mut TestAppContext,
) {
    let (shell, host) = plain(cx, vec![10, 11]);
    host.names.borrow_mut().insert(10, "Old".into());
    shell.update(cx, |shell, cx| {
        shell.change(Prop::TabName(10), Value::Text("New".into()), cx);
        shell.close_tab(10, false, cx);
    });
    host.take_edits();
    report(&shell, cx, Vec::new(), vec![11]);
    undo(&shell, cx);
    report(&shell, cx, Vec::new(), vec![1001, 11]);
    host.names.borrow_mut().insert(1001, "New".into());
    undo(&shell, cx);
    assert_eq!(
        host.take_edits(),
        ["reopen 10 as 1001", "rename tab 1001 Old"]
    );
}

#[gpui::test]
fn a_step_that_cant_be_done_any_more_is_dropped_and_the_one_before_it_undone(
    cx: &mut TestAppContext,
) {
    let (shell, host) = plain(cx, vec![10, 11]);
    shell.update(cx, |shell, cx| {
        shell.change(Prop::SpaceName(1), Value::Text("Work".into()), cx);
        shell.pin_tab(11, true, cx);
    });
    host.take_edits();
    // The pinned tab is gone, closed by a page or an agent.
    report(&shell, cx, Vec::new(), vec![10]);
    assert!(undo(&shell, cx));
    assert_eq!(host.take_edits(), ["rename space 1 One"]);
}

#[gpui::test]
fn a_new_step_ends_what_could_be_done_again(cx: &mut TestAppContext) {
    let (shell, _) = plain(cx, vec![10, 11]);
    shell.update(cx, |shell, cx| {
        shell.change(Prop::SpaceName(1), Value::Text("Work".into()), cx)
    });
    undo(&shell, cx);
    assert_eq!(titles(&shell, cx), (None, Some("Rename Space")));
    shell.update(cx, |shell, cx| {
        shell.change(Prop::SpaceName(1), Value::Text("Play".into()), cx)
    });
    assert!(!redo(&shell, cx));
}

#[gpui::test]
fn clearing_and_deleting_a_space_are_undone_and_done_again(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10, 11, 12]);
    shell.update(cx, |shell, cx| {
        shell.set_selected(10, cx);
        shell.run(Command::Clear, "ui", cx);
    });
    assert_eq!(host.take_edits(), ["clear"]);
    assert!(
        host.undo_toasts.borrow().is_empty(),
        "Clear says nothing (Arc)"
    );
    report(&shell, cx, Vec::new(), vec![10]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["undo clear"]);
    report(&shell, cx, Vec::new(), vec![10, 1001, 1002]);
    assert!(redo(&shell, cx));
    assert_eq!(host.take_edits(), ["clear"]);

    shell.update(cx, |shell, cx| shell.delete_space_now(2, cx));
    assert_eq!(host.take_edits(), ["delete space 2"]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["undo delete space"]);
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["delete space 2"]);
}

#[gpui::test]
fn the_archives_delete_and_clear_are_undone_and_done_again(cx: &mut TestAppContext) {
    let (shell, host) = plain(cx, vec![10]);
    let archived = |id| Archived {
        id,
        url: "https://a.example/".into(),
        title: "A".into(),
        archived: 0.,
    };
    shell.update(cx, |shell, cx| {
        shell.set_archive(vec![archived(7), archived(8)], cx)
    });
    shell.update(cx, |shell, cx| shell.delete_archived(7, cx));
    assert_eq!(host.take_edits(), ["archive remove 7"]);
    assert_eq!(
        host.undo_toasts.borrow().as_slice(),
        ["Deleted from Archive"]
    );
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["undo archive remove"]);
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["archive remove 7"]);

    shell.update(cx, |shell, cx| shell.empty_archive(cx));
    assert_eq!(host.take_edits(), ["archive clear"]);
    undo(&shell, cx);
    assert_eq!(host.take_edits(), ["undo archive remove"]);
    redo(&shell, cx);
    assert_eq!(host.take_edits(), ["archive clear"]);
}

#[gpui::test]
fn a_page_without_text_focus_gets_command_z_only_while_there_is_something_to_undo(
    cx: &mut TestAppContext,
) {
    let (shell, host) = plain(cx, vec![10, 11]);
    // Nothing to undo: the key goes on to the Edit menu, for the page's own undo.
    assert!(!shell.update(cx, |shell, cx| shell.run_for_page(Command::Undo, cx)));
    assert!(!shell.update(cx, |shell, cx| shell.run_for_page(Command::Redo, cx)));
    shell.update(cx, |shell, cx| shell.close_tab(11, false, cx));
    host.take_edits();
    assert!(shell.update(cx, |shell, cx| shell.run_for_page(Command::Undo, cx)));
    assert_eq!(host.take_edits(), ["reopen 11 as 1001"]);
    report(&shell, cx, Vec::new(), vec![10, 1001]);
    assert!(shell.update(cx, |shell, cx| shell.run_for_page(Command::Redo, cx)));
}

#[gpui::test]
fn a_private_window_keeps_nothing_to_undo(cx: &mut TestAppContext) {
    let host = Rc::new(Recorder::default());
    let shell = cx.new(|cx| Shell::private_window(host.clone(), 9, "Private".into(), 1, cx));
    shell.update(cx, |shell, cx| {
        shell.set_tabs(9, Vec::new(), vec![10], Vec::new(), Folders::default(), cx)
    });
    shell.update(cx, |shell, cx| shell.close_tab(10, false, cx));
    assert_eq!(titles(&shell, cx), (None, None));
    assert!(!undo(&shell, cx));
}

/// With the UI focused, ⌘Z and ⇧⌘Z are the shell's own, and the Edit menu's Undo and Redo reach it
/// too, since the menu sends them to whatever is focused.
#[gpui::test]
fn command_z_and_the_edit_menu_undo_and_redo_with_the_ui_focused(cx: &mut TestAppContext) {
    use gpui::VisualTestContext;

    let host = Rc::new(Recorder::default());
    let browser = host.clone();
    cx.update(crate::shell::bind_keys);
    let (shell, cx): (Entity<Shell>, &mut VisualTestContext) =
        cx.add_window_view(|_, cx| Shell::new(browser, cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![(1, "One".into(), None, 1, "".into(), "".into())], cx);
        shell.set_space(1, cx);
        shell.set_tabs(
            1,
            Vec::new(),
            vec![10, 11],
            Vec::new(),
            Folders::default(),
            cx,
        );
        shell.set_selected(10, cx);
    });
    cx.update(|window, cx| window.focus(&shell.read(cx).focus_handle.clone(), cx));

    cx.simulate_keystrokes("cmd-d");
    assert_eq!(host.take_edits(), ["pinned 10 true"]);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, vec![10], vec![11], Vec::new(), Folders::default(), cx)
    });
    cx.simulate_keystrokes("cmd-z");
    assert_eq!(host.take_edits(), ["move 10 false 0 0"]);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(
            1,
            Vec::new(),
            vec![10, 11],
            Vec::new(),
            Folders::default(),
            cx,
        )
    });
    cx.simulate_keystrokes("shift-cmd-z");
    assert_eq!(host.take_edits(), ["move 10 true 0 0"]);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(1, vec![10], vec![11], Vec::new(), Folders::default(), cx)
    });

    // The menu's items are the actions a field would take for itself.
    cx.dispatch_action(crate::text_input::Undo);
    assert_eq!(host.take_edits(), ["move 10 false 0 0"]);
    shell.update(cx, |shell, cx| {
        shell.set_tabs(
            1,
            Vec::new(),
            vec![10, 11],
            Vec::new(),
            Folders::default(),
            cx,
        )
    });
    cx.dispatch_action(crate::text_input::Redo);
    assert_eq!(host.take_edits(), ["move 10 true 0 0"]);
}
