//! Battery saver (#686): Chromium's, which turns on by itself below 20 % on battery, as Chrome's
//! does, and which Settings › General turns off. The toast says when it turns on and off, as it
//! changes how pages run without showing it: quietly, its words alone, for a moment, as Copy URL's
//! does (docs/ux-principles.md › Quiet controls, loud thresholds). Arc's isn't recorded.

use std::time::Duration;

use gpui::App;

use super::commands::ShellHandle;

pub const ON: &str = "Battery saver on below 20%";
pub const OFF: &str = "Battery saver off";
const TIME: Duration = Duration::from_secs(3);

/// The battery saver turned on, or off again, which the key window's toast says.
pub fn changed(on: bool, cx: &mut App) {
    let shell = cx.try_global::<ShellHandle>().and_then(|handle| handle.0.upgrade());
    let Some(browser) = shell.map(|shell| shell.read(cx).browser.clone()) else {
        return;
    };
    browser.toast(if on { ON } else { OFF }, None, TIME, cx);
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::TestAppContext;

    use super::{OFF, ON, changed};
    use crate::shell::{Shell, recorder::Recorder};

    /// Turning on and off each shows its toast, in the window the shortcuts act on.
    #[gpui::test]
    fn the_toast_says_when_the_battery_saver_turns_on_and_off(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        let browser = host.clone();
        let (_shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        cx.update(|_, cx| changed(true, cx));
        cx.update(|_, cx| changed(false, cx));
        assert_eq!(*host.toasts.borrow(), [ON, OFF]);
    }
}
