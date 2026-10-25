//! The browser's UI, drawn with GPUI. `Browser` (main.rs) runs it on the engine, through the core's
//! C API; `preview` runs it alone with made-up tabs.

pub mod command_bar;
pub mod dialog;
pub mod shell;
pub mod text_input;
pub mod toast;
