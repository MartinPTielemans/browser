//! The command bar's keys and completion, on a host that records what the bar asks of it, with the
//! core's suggestions made up.

use std::{cell::RefCell, ops::Range, rc::Rc};

use gpui::{
    App, ClipboardItem, Modifiers, MouseButton, TestAppContext, VisualTestContext, WindowHandle,
};

use super::*;
use crate::text_input::{self, Chosen, FieldMenu};

/// A host that records what the bar asks of it, but its suggestions and its window's height and
/// shape. A text with a dot names a page there; any other is searched for.
#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl Recorder {
    fn record(&self, call: String) {
        self.0.borrow_mut().push(call);
    }
}

fn search(text: &str) -> String {
    format!("https://search.test/?q={text}")
}

impl Host for Recorder {
    fn suggest(&self, _: Option<&str>, _: &str, _: bool) {}

    fn remove_suggestion(&self, url: &str) {
        self.record(format!("remove {url}"));
    }

    fn typed_url(&self, text: &str) -> Option<String> {
        Some(if text.contains('.') { format!("https://{text}/") } else { search(text) })
    }

    fn search_url(&self, site: Option<&str>, text: &str) -> Option<String> {
        Some(match site {
            Some(site) => format!("https://{site}/?q={text}"),
            None => search(text),
        })
    }

    fn page_favicon(&self, _: &str) {}

    fn open_tab(&self, url: &str, behind: bool) {
        self.record(format!("open {url}{}", if behind { " behind" } else { "" }));
    }

    fn navigate(&self, tab: u64, url: &str) {
        self.record(format!("navigate {tab} {url}"));
    }

    fn select(&self, tab: u64) {
        self.record(format!("select {tab}"));
    }

    fn split(&self, tab: u64, with: u64, _: &mut App) {
        self.record(format!("split {tab} with {with}"));
    }

    fn open_split(&self, url: &str, with: u64, _: &mut App) {
        self.record(format!("open {url} beside {with}"));
    }

    fn copy(&self, text: &str, _: &mut App) {
        self.record(format!("copy {text}"));
    }

    fn show(&self, _: bool, _: f32, _: f32, _: f32) {}

    fn set_height(&self, _: f32) {}

    fn hide(&self) {
        self.record("hide".into());
    }

    fn dismiss(&self) {
        self.record("dismiss".into());
    }

}

/// The bar, opened for a new tab.
struct Bar {
    cx: VisualTestContext,
    window: WindowHandle<CommandBar>,
    host: Rc<Recorder>,
}

impl Bar {
    fn new(cx: &mut TestAppContext) -> Self {
        let host = Rc::new(Recorder::default());
        cx.update(text_input::bind_keys);
        let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the bar");
        cx.update(|cx| show(None, cx));
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        Bar { cx, window, host }
    }

    fn type_text(&mut self, text: &str) {
        self.cx.simulate_input(text);
        self.cx.run_until_parked();
    }

    fn keys(&mut self, keys: &str) {
        self.cx.simulate_keystrokes(keys);
        self.cx.run_until_parked();
    }

    /// The core's suggestions for `text`.
    fn suggest(&mut self, text: &str, rows: Vec<Suggestion>) {
        self.cx.cx.update(|cx| suggested(text.into(), rows, cx));
        self.cx.run_until_parked();
    }

    /// What the input shows, and what's selected in it.
    fn input(&mut self) -> (String, Range<usize>) {
        self.window
            .read_with(&self.cx.cx, |bar, cx| {
                let input = bar.input.read(cx);
                (input.text().to_string(), input.selection())
            })
            .unwrap()
    }

    /// What the bar asked of the host since the last time.
    fn calls(&self) -> Vec<String> {
        self.host.0.take()
    }

    /// A right-click in the input: the items of the menu it shows, " (disabled)" after those
    /// that are and "" for a separator, and what takes the one chosen.
    fn right_click(&mut self) -> (Vec<String>, Chosen) {
        let shown: Rc<RefCell<Option<(Vec<String>, Chosen)>>> = Rc::default();
        let menu = shown.clone();
        self.cx.cx.update(|cx| {
            cx.set_global(FieldMenu(Rc::new(move |items, _, _, chosen, _| {
                let items = (items.into_iter())
                    .map(|(title, enabled)| match enabled || title.is_empty() {
                        true => title.to_string(),
                        false => format!("{title} (disabled)"),
                    })
                    .collect();
                *menu.borrow_mut() = Some((items, chosen));
            })))
        });
        let at = point(px(200.), px(HEIGHT / 2.));
        self.cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
        self.cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
        self.cx.run_until_parked();
        shown.take().expect("a right-click in the input showed no menu")
    }

    /// Chooses `title` in the menu a right-click showed.
    fn choose(&mut self, (items, chosen): (Vec<String>, Chosen), title: &str) {
        let index = items.iter().position(|item| item == title).expect("no such item");
        self.cx.update(|window, cx| chosen(index, window, cx));
        self.cx.run_until_parked();
    }

    fn copy_text(&mut self, text: &str) {
        self.cx.cx.write_to_clipboard(ClipboardItem::new_string(text.into()));
    }
}

/// A row for a page from the profile's history, which ⇧⌦ can take out.
fn page(url: &str) -> Suggestion {
    Suggestion {
        url: url.into(),
        text: url.to_string().into(),
        detail: SharedString::default(),
        tab: 0,
        completion: String::new(),
        removable: true,
        ..Suggestion::default()
    }
}

/// The first row for what's typed, as the core makes it: the page `completion` completes the text
/// to.
fn completing(url: &str, completion: &str) -> Suggestion {
    Suggestion { completion: completion.into(), ..page(url) }
}

/// The search row for `text`.
fn searching(text: &str) -> Suggestion {
    Suggestion { removable: false, ..page(&search(text)) }
}

#[gpui::test]
fn return_goes_where_the_completion_goes(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example.c");
    let rows = vec![completing("https://example.com/", "om"), searching("example.c")];
    bar.suggest("example.c", rows);
    assert_eq!(bar.input(), ("example.com".into(), 9..11));
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://example.com/", "hide"]);
}

#[gpui::test]
fn backspace_takes_the_completion_back_until_the_next_key(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example.c");
    bar.suggest("example.c", vec![completing("https://example.com/", "om")]);
    bar.keys("backspace");
    assert_eq!(bar.input(), ("example.c".into(), 9..9));
    // The core's rows for the same text, even ones that would complete it, as a late answer to
    // the ask before ⌫ would.
    bar.suggest("example.c", vec![completing("https://example.com/", "om")]);
    assert_eq!(bar.input(), ("example.c".into(), 9..9));
    // The next key typed at the end completes again.
    bar.type_text("o");
    bar.suggest("example.co", vec![completing("https://example.com/", "m")]);
    assert_eq!(bar.input(), ("example.com".into(), 10..11));
}

#[gpui::test]
fn right_keeps_the_completion_as_typed(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example.c");
    bar.suggest("example.c", vec![completing("https://example.com/", "om")]);
    bar.keys("right");
    assert_eq!(bar.input(), ("example.com".into(), 11..11));
    // Before the rows for it come, ↩ goes where the text now names.
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://example.com/", "hide"]);
}

#[gpui::test]
fn return_before_the_suggestions_goes_where_the_text_names(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example.org");
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://example.org/", "hide"]);
}

#[gpui::test]
fn older_suggestions_are_dropped(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    bar.type_text("a");
    bar.suggest("ex", vec![completing("https://example.com/", "ample.com")]);
    assert_eq!(bar.input(), ("exa".into(), 3..3));
    bar.keys("enter");
    assert_eq!(bar.calls(), [format!("open {}", search("exa")), "hide".into()]);
}

#[gpui::test]
fn arrows_wrap_at_the_ends(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    let rows = || vec![searching("ex"), page("https://a.test/"), page("https://b.test/")];
    bar.suggest("ex", rows());
    bar.keys("up enter");
    assert_eq!(bar.calls(), ["open https://b.test/", "hide"]);

    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    bar.suggest("ex", rows());
    bar.keys("down down ctrl-n enter");
    assert_eq!(bar.calls(), [format!("open {}", search("ex")), "hide".into()]);
}

#[gpui::test]
fn return_on_a_tab_switches_to_it(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    bar.suggest("ex", vec![searching("ex"), Suggestion { tab: 7, ..page("https://a.test/") }]);
    bar.keys("down enter");
    assert_eq!(bar.calls(), ["select 7", "hide"]);
}

#[gpui::test]
fn cmd_return_opens_behind_and_keeps_the_bar_open(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("exa");
    let rows = vec![
        completing("https://example.com/", "mple.com"),
        searching("exa"),
        page("https://example.org/"),
    ];
    bar.suggest("exa", rows);
    bar.keys("down down cmd-enter");
    assert_eq!(bar.calls(), ["open https://example.org/ behind"]);
    // Another, from the same rows.
    bar.keys("up up cmd-enter");
    assert_eq!(bar.calls(), ["open https://example.com/ behind"]);
}

#[gpui::test]
fn shift_cmd_return_searches_for_what_was_typed(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("exa");
    bar.suggest("exa", vec![completing("https://example.com/", "mple.com")]);
    bar.keys("shift-cmd-enter");
    assert_eq!(bar.calls(), [format!("open {}", search("exa")), "hide".into()]);
}

#[gpui::test]
fn ctrl_return_puts_www_and_com_around_a_word(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("exa");
    bar.suggest("exa", vec![completing("https://example.com/", "mple.com")]);
    bar.keys("ctrl-enter");
    assert_eq!(bar.calls(), ["open https://www.exa.com/", "hide"]);

    // More than a word goes where ↩ goes.
    let mut bar = Bar::new(cx);
    bar.type_text("two words");
    bar.keys("ctrl-enter");
    assert_eq!(bar.calls(), [format!("open {}", search("two words")), "hide".into()]);
}

#[gpui::test]
fn shift_delete_takes_a_page_out_of_history(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    bar.suggest("ex", vec![searching("ex"), page("https://a.test/"), page("https://b.test/")]);
    // The search isn't from history.
    bar.keys("shift-delete");
    assert_eq!(bar.calls(), Vec::<String>::new());
    bar.keys("down shift-delete");
    assert_eq!(bar.calls(), ["remove https://a.test/"]);
    // The core lists the rows again without it, and the row under it takes its place.
    bar.suggest("ex", vec![searching("ex"), page("https://b.test/")]);
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://b.test/", "hide"]);
}

#[gpui::test]
fn a_sum_comes_after_the_first_row_and_return_on_it_copies_its_result(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("12*7");
    bar.suggest("12*7", vec![searching("12*7"), page("https://a.test/12*7")]);
    // The search stays first, as the core put it, and the sum's result is the row after it.
    bar.keys("down enter");
    assert_eq!(bar.calls(), ["hide", "copy 84"]);
}

/// The first row for an engine's keyword typed, which hints its site search.
fn hinting(keyword: &str) -> Suggestion {
    let url = format!("https://{keyword}/");
    Suggestion { keyword: keyword.into(), site: "Duck".into(), ..page(&url) }
}

#[gpui::test]
fn tab_on_a_hinted_row_searches_its_site_until_backspace_leaves(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test");
    bar.suggest("duck.test", vec![hinting("duck.test"), page("https://a.test/")]);
    bar.keys("tab");
    assert_eq!(bar.input(), (String::new(), 0..0));
    // ⌫ in the middle of the words stays in the site search.
    bar.type_text("cats");
    bar.keys("backspace");
    assert_eq!(bar.input(), ("cat".into(), 3..3));
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://duck.test/?q=cat", "hide"]);

    // Left before anything is typed, the keyword comes back as it was, without a space.
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test");
    bar.suggest("duck.test", vec![hinting("duck.test")]);
    bar.keys("tab backspace");
    assert_eq!(bar.input(), ("duck.test".into(), 9..9));
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://duck.test/", "hide"]);
}

#[gpui::test]
fn tab_on_a_row_without_a_hint_selects_the_next(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("ex");
    bar.suggest("ex", vec![searching("ex"), hinting("duck.test"), page("https://a.test/")]);
    bar.keys("tab");
    // Now on the hinted row, Tab enters its site search.
    bar.keys("tab");
    assert_eq!(bar.input(), (String::new(), 0..0));

    // It keeps the words after the first, the keyword.
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test cats");
    bar.suggest("duck.test cats", vec![hinting("duck.test")]);
    bar.keys("tab");
    assert_eq!(bar.input(), ("cats".into(), 4..4));
}

#[gpui::test]
fn a_space_after_the_hinted_keyword_searches_its_site(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test");
    bar.suggest("duck.test", vec![hinting("duck.test")]);
    bar.type_text(" ");
    assert_eq!(bar.input(), (String::new(), 0..0));
    // The site's searches only, from the core, which names them by the keyword and the words.
    bar.type_text("cats");
    bar.suggest("duck.test cats", vec![page("https://duck.test/?q=cats&ia=web")]);
    bar.keys("enter");
    assert_eq!(bar.calls(), ["open https://duck.test/?q=cats&ia=web", "hide"]);

    // So does Japanese input's space.
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test");
    bar.suggest("duck.test", vec![hinting("duck.test")]);
    bar.type_text("\u{3000}");
    assert_eq!(bar.input(), (String::new(), 0..0));

    // Esc leaves, keeping the space, and then closes the bar.
    let mut bar = Bar::new(cx);
    bar.type_text("duck.test");
    bar.suggest("duck.test", vec![hinting("duck.test")]);
    bar.type_text(" cats");
    bar.keys("escape");
    assert_eq!(bar.input(), ("duck.test cats".into(), 10..10));
    bar.keys("escape");
    assert_eq!(bar.calls(), ["dismiss"]);
}

#[gpui::test]
fn a_space_after_other_text_stays_in_it(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("duck");
    bar.suggest("duck", vec![Suggestion { completion: ".test".into(), ..hinting("duck.test") }]);
    bar.type_text(" ");
    assert_eq!(bar.input(), ("duck ".into(), 5..5));
    bar.keys("enter");
    assert_eq!(bar.calls(), [format!("open {}", search("duck")), "hide".into()]);
}

#[gpui::test]
fn the_inputs_menu_pastes_and_goes_where_return_would(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("something else");
    let menu = bar.right_click();
    let items = [
        "Cut (disabled)",
        "Copy (disabled)",
        "Paste",
        "Paste and Go",
        "Paste and Search",
        "",
        "Select All",
    ];
    assert_eq!(menu.0, items);
    // What was typed doesn't count, and a copied line break is a space.
    bar.copy_text("  example.org\n");
    bar.choose(menu, "Paste and Go");
    assert_eq!(bar.calls(), ["open https://example.org/", "hide"]);

    let mut bar = Bar::new(cx);
    bar.copy_text("example.org");
    let menu = bar.right_click();
    bar.choose(menu, "Paste and Search");
    assert_eq!(bar.calls(), [format!("open {}", search("example.org")), "hide".into()]);

    // Nothing to paste, nowhere to go.
    let mut bar = Bar::new(cx);
    bar.copy_text(" \n");
    let menu = bar.right_click();
    bar.choose(menu, "Paste and Go");
    assert_eq!(bar.calls(), Vec::<String>::new());
}

#[gpui::test]
fn a_right_click_in_the_selection_keeps_it_for_copy(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example");
    bar.keys("cmd-a");
    let menu = bar.right_click();
    assert_eq!(menu.0[..2], ["Cut", "Copy"]);
    bar.choose(menu, "Copy");
    let copied = bar.cx.cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("example"));
    assert_eq!(bar.input(), ("example".into(), 0..7));
}

#[test]
fn the_window_fits_up_to_five_rows_of_arcs_size() {
    assert_eq!(height(0), 64.);
    assert_eq!(height(1), 128.5);
    // Arc's panel with five rows is 328.5 tall, and more rows scroll inside it.
    assert_eq!(height(5), 328.5);
    assert_eq!(height(11), 328.5);
}

#[gpui::test]
fn the_rows_that_go_to_a_search_are_marked_for_their_icon(cx: &mut TestAppContext) {
    let mut bar = Bar::new(cx);
    bar.type_text("example");
    let typed = Suggestion {
        url: search("example"),
        text: "example".into(),
        ..Suggestion::default()
    };
    bar.suggest("example", vec![page("https://example.com/"), typed]);
    let kinds = bar
        .window
        .read_with(&bar.cx.cx, |bar, _| {
            bar.rows.iter().map(|row| matches!(row.kind(), Kind::Search)).collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(kinds, [false, true]);
}
