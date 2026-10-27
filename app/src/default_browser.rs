//! Whether the browser is the system's default, for Settings › General: until it is, its row offers
//! to make it the default, and then says it is. The system keeps the setting, and the way back is
//! its own, in System Settings › Desktop & Dock, so the row offers none. It reads the setting again
//! when Settings shows and when the app becomes active, as the user may have changed it there,
//! never on a timer, which would wake an idle browser. Agents get no way to change it.

use std::rc::Rc;

use gpui::{Context, SharedString};

/// The system's default browser, as the row reads and asks for it.
pub trait System {
    /// Whether the system opens http and https links with this app.
    fn is_default(&self) -> bool;
    /// Asks the system to, which asks the user first.
    fn make_default(&self);
}

pub struct DefaultBrowser {
    system: Rc<dyn System>,
    default: bool,
}

impl DefaultBrowser {
    pub fn new(system: Rc<dyn System>) -> Self {
        let default = system.is_default();
        Self { system, default }
    }

    /// Reads the system's setting again.
    pub fn check(&mut self, cx: &mut Context<Self>) {
        let default = self.system.is_default();
        if default != self.default {
            self.default = default;
            cx.notify();
        }
    }

    pub fn is_default(&self) -> bool {
        self.default
    }

    /// What the row says: what it offers, or that the browser is the default.
    pub fn title(&self) -> SharedString {
        match self.default {
            false => concat!("Make ", env!("APP_NAME"), " your default browser").into(),
            true => concat!(env!("APP_NAME"), " is your default browser").into(),
        }
    }

    /// The way back, under the title.
    pub fn detail(&self) -> SharedString {
        let detail = match self.default {
            false => {
                "macOS asks you first. You can change it back in System Settings › Desktop & Dock."
            }
            true => "You can change it in System Settings › Desktop & Dock.",
        };
        detail.into()
    }

    /// Asks the system to make the browser the default, unless it is already.
    pub fn make_default(&self) {
        if !self.default {
            self.system.make_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use gpui::{AppContext, Entity, TestAppContext};

    use super::{DefaultBrowser, System};

    /// The system's setting, which a test changes as the user would in System Settings.
    #[derive(Default)]
    struct Setting {
        default: Cell<bool>,
        asked: Cell<usize>,
    }

    impl System for Setting {
        fn is_default(&self) -> bool {
            self.default.get()
        }

        fn make_default(&self) {
            self.asked.set(self.asked.get() + 1);
        }
    }

    fn row(setting: &Rc<Setting>, cx: &mut TestAppContext) -> Entity<DefaultBrowser> {
        let system = setting.clone();
        cx.new(|_| DefaultBrowser::new(system))
    }

    /// Not the default, the row offers to make it so and asks the system, which asks the user. It
    /// says nothing changed until the user has said yes and the app reads the setting again.
    #[gpui::test]
    fn the_row_offers_until_the_user_says_yes(cx: &mut TestAppContext) {
        let setting = Rc::new(Setting::default());
        let row = row(&setting, cx);
        row.read_with(cx, |row, _| {
            assert!(!row.is_default());
            assert!(row.title().starts_with("Make "));
            row.make_default();
        });
        assert_eq!(setting.asked.get(), 1);
        row.update(cx, |row, cx| row.check(cx));
        row.read_with(cx, |row, _| assert!(!row.is_default()));
        setting.default.set(true);
        row.update(cx, |row, cx| row.check(cx));
        row.read_with(cx, |row, _| {
            assert!(row.is_default());
            assert!(row.title().ends_with(" is your default browser"));
            // The default already, it doesn't ask again.
            row.make_default();
        });
        assert_eq!(setting.asked.get(), 1);
    }

    /// The user picked another browser in System Settings: back in the app, the row offers again,
    /// and Settings, which watches it, draws it so.
    #[gpui::test]
    fn the_row_follows_a_change_made_in_system_settings(cx: &mut TestAppContext) {
        let setting = Rc::new(Setting { default: Cell::new(true), ..Setting::default() });
        let row = row(&setting, cx);
        let redrawn = Rc::new(Cell::new(0));
        let _watch = cx.update(|cx| {
            let redrawn = redrawn.clone();
            cx.observe(&row, move |_, _| redrawn.set(redrawn.get() + 1))
        });
        row.update(cx, |row, cx| row.check(cx));
        assert_eq!(redrawn.get(), 0);
        setting.default.set(false);
        row.update(cx, |row, cx| row.check(cx));
        cx.run_until_parked();
        assert_eq!(redrawn.get(), 1);
        row.read_with(cx, |row, _| assert!(!row.is_default()));
    }
}
