//! The browser's UI, drawn with GPUI. `Browser` (main.rs) runs it on the engine, through the core's
//! C API; `preview` runs it alone with made-up tabs.

/// Writes a line to stderr, as `eprintln!` does, but in one write where `eprintln!` makes one for
/// each piece, so a line Chromium logs at the same time can't land inside it, where the checks
/// wouldn't find it.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        _ = std::io::stderr().write_all(format!("{}\n", format_args!($($arg)*)).as_bytes());
    }};
}

pub mod command_bar;
pub mod dialog;
pub mod icon_picker;
pub mod shell;
pub mod site_controls;
pub mod space_icon;
pub mod text_input;
pub mod toast;
pub mod tooltip;
pub mod webkit;
