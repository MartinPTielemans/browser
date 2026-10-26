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

/// Milliseconds on macOS's uptime clock, the one Chromium's `TimeTicks` and the host's lines read,
/// so a step timed across the core, the host and the UI is one subtraction.
pub fn uptime_ms() -> f64 {
    unsafe extern "C" {
        fn clock_gettime_nsec_np(clock_id: u32) -> u64;
    }
    const CLOCK_UPTIME_RAW: u32 = 8;
    unsafe { clock_gettime_nsec_np(CLOCK_UPTIME_RAW) as f64 / 1e6 }
}

pub mod capture;
pub mod command_bar;
pub mod default_browser;
pub mod dialog;
pub mod dock_dot;
pub mod icon_picker;
pub mod link_target;
pub mod offer;
pub mod shell;
pub mod site_controls;
pub mod space_icon;
pub mod text_input;
pub mod theme;
pub mod toast;
pub mod tooltip;
pub mod window_frames;
pub mod zoom_hud;
