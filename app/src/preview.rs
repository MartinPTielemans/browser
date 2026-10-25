//! The UI alone, with made-up tabs, for working on how it looks: it needs no engine and rebuilds in
//! seconds. `cargo run --release --bin preview`. The browser's own rules for tabs live in the core,
//! so the ones here are only stand-ins.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use browser::shell::{self, Browser, MenuEntry, Page, Shell, WindowButton};
use gpui::{
    App, AppContext, Bounds, Focusable, Pixels, Point, SharedString, Size, TitlebarOptions,
    WeakEntity, WindowBounds, WindowOptions, point, px, size,
};

unsafe extern "C" {
    // platform/mac/host.mm: where a window goes, centered on the main display or, with
    // BROWSER_DISPLAY=builtin, on the Mac's own.
    fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
}

// Two Spaces, so the switcher shows. The tabs are in the first, the top two pinned.
const SPACES: [&str; 2] = ["Space 1", "Reading"];
const TABS: [(&str, &str); 5] = [
    (
        "Arc (web browser) - Wikipedia",
        "https://en.wikipedia.org/wiki/Arc_(web_browser)",
    ),
    ("Hacker News", "https://news.ycombinator.com/"),
    ("Zed — The editor for what's next", "https://zed.dev/"),
    (
        "Rust Programming Language",
        "https://www.rust-lang.org/",
    ),
    ("", "https://example.com/"),
];

struct Fake {
    shell: WeakEntity<Shell>,
    next_id: Cell<u64>,
    spaces: RefCell<Vec<(u64, SharedString)>>,
}

impl Fake {
    fn report_spaces(&self, cx: &mut App) {
        let spaces = self.spaces.borrow().clone();
        self.later(cx, move |shell, cx| shell.set_spaces(spaces, cx));
    }

    /// Changes the shell once the current update is over, as the engine reports back.
    fn later(&self, cx: &mut App, change: impl FnOnce(&mut Shell, &mut gpui::Context<Shell>) + 'static) {
        let shell = self.shell.clone();
        cx.defer(move |cx| {
            shell.update(cx, change).ok();
        });
    }
}

impl Browser for Fake {
    fn select(&self, tab: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| shell.set_selected(tab, cx));
    }

    fn close(&self, tab: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let Some(index) = today.iter().position(|&id| id == tab) else {
                return;
            };
            today.remove(index);
            let next = today.get(index).or(today.last()).copied().unwrap_or(0);
            shell.set_tabs(shell.space(), shell.pinned().to_vec(), today, cx);
            if shell.selected() == tab {
                shell.set_selected(next, cx);
            }
        });
    }

    fn close_below(&self, tab: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let Some(index) = today.iter().position(|&id| id == tab) else {
                return;
            };
            today.truncate(index + 1);
            if !today.contains(&shell.selected()) && !shell.pinned().contains(&shell.selected()) {
                shell.set_selected(tab, cx);
            }
            shell.set_tabs(shell.space(), shell.pinned().to_vec(), today, cx);
        });
    }

    // Under the tab, with its title and page, as the core does.
    fn duplicate(&self, tab: u64, cx: &mut App) {
        let copy = self.next_id.get();
        self.next_id.set(copy + 1);
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            let at = today.iter().position(|&id| id == tab).map_or(0, |index| index + 1);
            today.insert(at, copy);
            shell.set_title(copy, shell.title(tab), cx);
            shell.set_page(copy, shell.page(tab), cx);
            shell.set_tabs(shell.space(), shell.pinned().to_vec(), today, cx);
            shell.set_selected(copy, cx);
        });
    }

    // The made-up tabs stay in their Space.
    fn move_to_space(&self, _: u64, _: u64, _: &mut App) {}

    // As the core does: pinned at the bottom of the pinned tabs, unpinned at the top of Today.
    fn set_pinned(&self, tab: u64, pinned: bool, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut tabs = [shell.pinned().to_vec(), shell.today().to_vec()];
            let (from, to) = if pinned { (1, 0) } else { (0, 1) };
            let Some(index) = tabs[from].iter().position(|&id| id == tab) else {
                return;
            };
            tabs[from].remove(index);
            let at = if pinned { tabs[to].len() } else { 0 };
            tabs[to].insert(at, tab);
            let [pinned, today] = tabs;
            shell.set_tabs(shell.space(), pinned, today, cx);
        });
    }

    fn move_tab(&self, tab: u64, pinned: bool, index: usize, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            let mut tabs = [shell.pinned().to_vec(), shell.today().to_vec()];
            for list in &mut tabs {
                list.retain(|&id| id != tab);
            }
            let to = &mut tabs[if pinned { 0 } else { 1 }];
            to.insert(index.min(to.len()), tab);
            let [pinned, today] = tabs;
            shell.set_tabs(shell.space(), pinned, today, cx);
        });
    }

    // As the core does, without the undo.
    fn clear(&self, cx: &mut App) {
        self.later(cx, |shell, cx| {
            let pinned = shell.pinned().to_vec();
            if shell.today().contains(&shell.selected()) {
                shell.set_selected(pinned.last().copied().unwrap_or(0), cx);
            }
            shell.set_tabs(shell.space(), pinned, Vec::new(), cx);
        });
    }

    fn undo_clear(&self, _: &mut App) {}

    fn reopen_closed_tab(&self, _: &mut App) {}

    // The made-up pages have no history to go through.
    fn back_to_pinned_url(&self, _: u64, _: &mut App) {}

    fn go_back(&self, _: u64, _: &mut App) {}

    fn go_forward(&self, _: u64, _: &mut App) {}

    fn reload(&self, _: u64, _: bool, _: &mut App) {}

    fn stop(&self, _: u64, _: &mut App) {}

    fn command_bar(
        &self,
        editing: Option<(u64, SharedString)>,
        _: Option<Bounds<Pixels>>,
        cx: &mut App,
    ) {
        // No command bar here: a new tab opens at once.
        if editing.is_some() {
            return;
        }
        let tab = self.next_id.get();
        self.next_id.set(tab + 1);
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            today.insert(0, tab);
            shell.set_tabs(shell.space(), shell.pinned().to_vec(), today, cx);
            shell.set_selected(tab, cx);
        });
    }

    // The made-up pages have nothing to find.
    fn find(&self, _: u64, _: &mut App) {}

    fn find_next(&self, _: u64, _: bool, _: &mut App) {}

    // Each made-up Space shows its top tab.
    fn show_space(&self, space: u64, cx: &mut App) {
        self.later(cx, move |shell, cx| {
            shell.set_space(space, cx);
            let top = shell.today().first().or(shell.pinned().first()).copied().unwrap_or(0);
            shell.set_selected(top, cx);
        });
    }

    // The made-up Spaces' pages slide in without a selected row.
    fn space_tab(&self, _: u64) -> u64 {
        0
    }

    fn create_space(&self, name: &str, cx: &mut App) -> u64 {
        let space = self.next_id.get();
        self.next_id.set(space + 1);
        self.spaces.borrow_mut().push((space, name.to_string().into()));
        self.report_spaces(cx);
        space
    }

    fn rename_space(&self, space: u64, name: &str, cx: &mut App) {
        if let Some(entry) = self.spaces.borrow_mut().iter_mut().find(|(id, _)| *id == space) {
            entry.1 = name.to_string().into();
        }
        self.report_spaces(cx);
    }

    fn delete_space(&self, space: u64, cx: &mut App) {
        self.spaces.borrow_mut().retain(|(id, _)| *id != space);
        self.report_spaces(cx);
    }

    // The preview's window is GPUI's own, which has the keyboard already.
    fn focus_ui(&self, _: &mut App) {}

    // There's no page, and the window's buttons stay where GPUI put them.
    fn lay_out_page(&self, _: Size<Pixels>, _: f32, _: &mut App) {}

    fn show_window_buttons(&self, _: bool, _: &mut App) {}

    fn press_window_button(&self, _: WindowButton, _: &mut App) {}

    // The reveal's window is the app's, over the page, which the preview has neither of.
    fn show_reveal(&self, _: f32, _: &mut App) {}

    fn hide_reveal(&self, _: &mut App) {}

    fn saved_sidebar_width(&self) -> Option<f32> {
        None
    }

    fn set_sidebar_width(&self, _: f32, _: &mut App) {}

    fn detent(&self, _: &mut App) {}

    // macOS's default blue.
    fn accent_color(&self) -> u32 {
        0x007aff
    }

    // Menus and alerts are AppKit's, which the preview goes without.
    fn menu(
        &self,
        _: Vec<MenuEntry>,
        _: Point<Pixels>,
        _: bool,
        _: Box<dyn FnOnce(Option<usize>, &mut App)>,
        _: &mut App,
    ) {
    }

    fn confirm(
        &self,
        _: String,
        _: String,
        _: &'static str,
        _: Box<dyn FnOnce(&mut App)>,
        _: &mut App,
    ) {
    }

    fn quit(&self, cx: &mut App) {
        cx.quit();
    }
}

fn main() {
    gpui_platform::application().with_assets(shell::Assets).run(|cx| {
        shell::bind_keys(cx);
        browser::text_input::bind_keys(cx);
        let (width, height) = (1280., 800.);
        let (mut x, mut y) = (0., 0.);
        unsafe { host_centered(width, height, &mut x, &mut y) };
        let bounds = Bounds::new(
            point(px(x as f32), px(y as f32)),
            size(px(width as f32), px(height as f32)),
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                // Arc's: 16 pt buttons centred at (20, 24), 23 apart.
                traffic_light_position: Some(point(px(12.), px(16.))),
            }),
            ..Default::default()
        };
        let window = cx.open_window(options, |window, cx| {
            let shell = cx.new(|cx| {
                let first_space = TABS.len() as u64 + 1;
                let spaces: Vec<(u64, SharedString)> = (first_space..)
                    .zip(SPACES)
                    .map(|(id, name)| (id, name.into()))
                    .collect();
                let browser = Rc::new(Fake {
                    shell: cx.weak_entity(),
                    next_id: Cell::new(first_space + SPACES.len() as u64),
                    spaces: RefCell::new(spaces.clone()),
                });
                let mut shell = Shell::new(browser, cx);
                shell.set_spaces(spaces, cx);
                shell.set_space(first_space, cx);
                for (index, (title, url)) in TABS.iter().enumerate() {
                    let tab = index as u64 + 1;
                    shell.set_title(tab, (*title).into(), cx);
                    let page = Page {
                        url: (*url).into(),
                        // The first tab has been somewhere before, so back shows bright, and it's
                        // pinned somewhere else. The second is a pinned tab closed, and the third
                        // is still loading.
                        can_go_back: index == 0,
                        away_from_pinned_url: index == 0,
                        unloaded: index == 1,
                        loading: index == 2,
                        ..Default::default()
                    };
                    shell.set_page(tab, page, cx);
                }
                let pinned = vec![1, 2];
                shell.set_tabs(first_space, pinned, (3..=TABS.len() as u64).collect(), cx);
                shell.set_selected(1, cx);
                shell
            });
            window.focus(&shell.focus_handle(cx), cx);
            shell
        });
        if let Err(error) = window {
            eprintln!("couldn't open the window: {error}");
            cx.quit();
        }
        cx.activate(true);
    });
}
