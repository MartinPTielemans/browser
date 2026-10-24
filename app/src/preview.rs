//! The UI alone, with made-up tabs, for working on how it looks: it needs no engine and rebuilds in
//! seconds. `cargo run --release --bin preview`. The browser's own rules for tabs live in the core,
//! so the ones here are only stand-ins.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use browser::shell::{self, Browser, Page, Shell};
use gpui::{
    App, AppContext, Bounds, Focusable, SharedString, TitlebarOptions, WeakEntity, WindowBounds,
    WindowOptions, point, px, size,
};

unsafe extern "C" {
    // platform/mac/host.mm: where a window goes, centered on the main display or, with
    // BROWSER_DISPLAY=builtin, on the Mac's own.
    fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
}

// Two Spaces, so the switcher shows. The tabs are in the first.
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
            shell.set_today(shell.space(), today, cx);
            if shell.selected() == tab {
                shell.set_selected(next, cx);
            }
        });
    }

    // The made-up pages have no history to go through.
    fn go_back(&self, _: u64, _: &mut App) {}

    fn go_forward(&self, _: u64, _: &mut App) {}

    fn reload(&self, _: u64, _: bool, _: &mut App) {}

    fn stop(&self, _: u64, _: &mut App) {}

    fn command_bar(&self, editing: Option<(u64, SharedString)>, cx: &mut App) {
        // No command bar here: a new tab opens at once.
        if editing.is_some() {
            return;
        }
        let tab = self.next_id.get();
        self.next_id.set(tab + 1);
        self.later(cx, move |shell, cx| {
            let mut today = shell.today().to_vec();
            today.insert(0, tab);
            shell.set_today(shell.space(), today, cx);
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
            let top = shell.today().first().copied().unwrap_or(0);
            shell.set_selected(top, cx);
        });
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
                        // The first tab has been somewhere before, so back shows bright, and the
                        // third is still loading.
                        can_go_back: index == 0,
                        loading: index == 2,
                        ..Default::default()
                    };
                    shell.set_page(tab, page, cx);
                }
                shell.set_today(first_space, (1..=TABS.len() as u64).collect(), cx);
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
