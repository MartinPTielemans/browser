//! The UI alone, with made-up tabs, for working on how it looks: it needs no engine and rebuilds in
//! seconds. `cargo run --release --bin preview`. The browser's own rules for tabs live in the core,
//! so the ones here are only stand-ins.

use std::{cell::Cell, rc::Rc};

use browser::shell::{self, Browser, Shell};
use gpui::{
    App, AppContext, Bounds, SharedString, TitlebarOptions, WeakEntity, WindowBounds,
    WindowOptions, point, px, size,
};

unsafe extern "C" {
    // platform/mac/host.mm: where a window goes, centered on the main display or, with
    // BROWSER_DISPLAY=builtin, on the Mac's own.
    fn host_centered(width: f64, height: f64, x: *mut f64, y: *mut f64);
}

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
}

impl Fake {
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
            shell.set_today(today, cx);
            if shell.selected() == tab {
                shell.set_selected(next, cx);
            }
        });
    }

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
            shell.set_today(today, cx);
            shell.set_selected(tab, cx);
        });
    }

    fn quit(&self, cx: &mut App) {
        cx.quit();
    }
}

fn main() {
    gpui_platform::application().run(|cx| {
        shell::bind_keys(cx);
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
        let window = cx.open_window(options, |_, cx| {
            cx.new(|cx| {
                let browser = Rc::new(Fake {
                    shell: cx.weak_entity(),
                    next_id: Cell::new(TABS.len() as u64 + 1),
                });
                let mut shell = Shell::new(browser, cx);
                for (index, (title, url)) in TABS.iter().enumerate() {
                    let tab = index as u64 + 1;
                    shell.set_title(tab, (*title).into(), cx);
                    shell.set_url(tab, (*url).into(), cx);
                }
                shell.set_today((1..=TABS.len() as u64).collect(), cx);
                shell.set_selected(1, cx);
                shell
            })
        });
        if let Err(error) = window {
            eprintln!("couldn't open the window: {error}");
            cx.quit();
        }
        cx.activate(true);
    });
}
