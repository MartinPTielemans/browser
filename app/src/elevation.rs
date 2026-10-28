//! The three elevation levels every overlay takes (#317). Each is one recipe: a rim inside the
//! surface's edge, the outline round the outside and a drop shadow, with the values in
//! `design/tokens.toml` under `[elevation]`. A surface GPUI draws takes `shadow()` and `rim()`;
//! one the host draws a shadow window for takes `host_shadow()`.

use gpui::{BoxShadow, Hsla, Rgba, point, px, rgba};

use crate::palette::Palette;
use crate::tokens::elevation as t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Sits on the window: favorite tiles, library cards, pictures of pages.
    Resting,
    /// Opens from something and closes on a click outside: menus, the Site Controls, the toast,
    /// the zoom HUD, tooltips, the link target, the find bar.
    Popover,
    /// Takes the keyboard: the command bar, the dialog, the icon and theme pickers.
    Modal,
}

/// How far a window that draws a popover's shadow itself reaches past what it holds.
pub const WINDOW_MARGIN: f32 = t::WINDOW_MARGIN;

/// What the host's shadow window draws, in Core Animation's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostShadow {
    pub outline_opacity: f32,
    pub outline_radius: f64,
    pub shadow_opacity: f32,
    pub shadow_radius: f64,
    pub shadow_drop: f64,
}

/// The black's alpha of an `rgba` colour.
fn alpha(color: u32) -> f32 {
    (color & 0xff) as f32 / 255.
}

impl Level {
    /// The white line inside the surface's edge.
    pub fn rim(self) -> Rgba {
        rgba(match self {
            Level::Resting => t::RESTING_RIM,
            Level::Popover => t::POPOVER_RIM,
            Level::Modal => t::MODAL_RIM,
        })
    }

    /// The rim in the palette's ink, Paper or Ink as the Space's theme has it, at the level's
    /// alpha, for a surface the theme colours.
    pub fn rim_in(self, palette: &Palette) -> Hsla {
        palette.faint(self.rim().a)
    }

    /// The drop shadow's colour, how far it falls and its blur.
    fn drop(self) -> (u32, f32, f32) {
        match self {
            Level::Resting => (t::RESTING_SHADOW, t::RESTING_SHADOW_Y, t::RESTING_SHADOW_BLUR),
            Level::Popover => (t::POPOVER_SHADOW, t::POPOVER_SHADOW_Y, t::POPOVER_SHADOW_BLUR),
            Level::Modal => (t::MODAL_SHADOW, t::MODAL_SHADOW_Y, t::MODAL_SHADOW_BLUR),
        }
    }

    /// For a surface GPUI draws, which is wholly inside its window: the drop shadow, and over it
    /// the outline.
    pub fn shadow(self) -> Vec<BoxShadow> {
        let (color, y, blur) = self.drop();
        let shadow = |color: u32, y: f32, blur: f32| BoxShadow {
            color: rgba(color).into(),
            offset: point(px(0.), px(y)),
            blur_radius: px(blur),
            spread_radius: px(0.),
            inset: false,
        };
        vec![shadow(color, y, blur), shadow(t::OUTLINE, 0., t::OUTLINE_BLUR)]
    }

    /// For a surface the host puts a shadow window under. A shadow layer's radius is the
    /// Gaussian's standard deviation, which is half CSS's blur.
    pub fn host_shadow(self) -> HostShadow {
        let (color, y, blur) = self.drop();
        HostShadow {
            outline_opacity: alpha(t::OUTLINE),
            outline_radius: f64::from(t::OUTLINE_BLUR) / 2.,
            shadow_opacity: alpha(color),
            shadow_radius: f64::from(blur) / 2.,
            shadow_drop: f64::from(y),
        }
    }
}

#[cfg(test)]
mod tests;
