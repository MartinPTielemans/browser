//! A Space's theme, as Arc's theme picker makes it: one to three colours placed in a square field,
//! how strongly the window takes them, its grain, and its appearance (docs/ux-spec.md › Themes).
//! A theme from Arc can also have another look for a dark window. The core keeps it with the
//! Space as text, and the main colour as the Space's colour.
//!
//! The field is a disc of HSL colours about its centre (Arc 1.170 rec at 2×, twelve handles): the
//! hue is the angle clockwise from the right, saturation is 1, and lightness is the distance from
//! the centre in half the field's width, black at the centre and white at the edge. A second colour
//! is the main one's complement; with a third the two sit 150° and 210° round from it. Dragging
//! moves them together. A swatch places the main colour at its hue and lightness and keeps its
//! saturation, the others' too, until a handle is dragged.

use std::{cell::OnceCell, sync::Arc};

use gpui::{Hsla, RenderImage, Rgba, hsla, rgb};
use image::{Frame, RgbaImage};

pub const MAX_COLORS: usize = 3;
pub use crate::tokens::theme::DEFAULT_INTENSITY;

/// The window's look with the theme: light, dark, or picked from the colours (Arc's sparkles).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Appearance {
    #[default]
    Automatic,
    Light,
    Dark,
}

impl Appearance {
    pub const ALL: [Appearance; 3] = [Appearance::Automatic, Appearance::Light, Appearance::Dark];

    pub fn name(self) -> &'static str {
        match self {
            Appearance::Automatic => "automatic",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// The main colour's place in the field, from its centre, in half the field's width: x to the
    /// right, y down. Never further out than 1, where the colour is white.
    pub x: f32,
    pub y: f32,
    /// The colours' HSL saturation: the field's 1, or a swatch's own.
    pub saturation: f32,
    /// How many colours, 1 to MAX_COLORS.
    pub count: usize,
    /// 0 to 1: how strongly the window takes the colours.
    pub intensity: f32,
    /// 0 to 1: how much grain lies over them.
    pub grain: f32,
    pub appearance: Appearance,
    /// The look a dark window draws instead, where the theme has two, as Arc's can; the picker
    /// makes themes of one.
    pub dark: Option<Look>,
}

/// A theme's colours, intensity and grain, without its appearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub x: f32,
    pub y: f32,
    pub saturation: f32,
    pub count: usize,
    pub intensity: f32,
    pub grain: f32,
}

impl Theme {
    /// One colour, at (x, y) in the field, with the picker's other settings as they start.
    pub fn at(x: f32, y: f32) -> Theme {
        let (x, y) = in_disc(x, y);
        Theme {
            x,
            y,
            saturation: 1.,
            count: 1,
            intensity: DEFAULT_INTENSITY,
            grain: 0.,
            appearance: Appearance::Automatic,
            dark: None,
        }
    }

    /// The theme's own look, the one a light window draws.
    pub fn look(&self) -> Look {
        let Theme { x, y, saturation, count, intensity, grain, .. } = *self;
        Look { x, y, saturation, count, intensity, grain }
    }

    /// The theme as a window draws it, `dark` or light: its dark look on a dark window where it
    /// has one, as a theme of one look.
    pub fn shown(self, dark: bool) -> Theme {
        let look = self.dark.filter(|_| dark).unwrap_or(self.look());
        let Look { x, y, saturation, count, intensity, grain } = look;
        Theme { x, y, saturation, count, intensity, grain, dark: None, ..self }
    }

    /// A theme of `color`, 0xRRGGBB, alone: in the field at its hue and HSL lightness, and at its
    /// saturation, so it is the colour itself, such as for a Space whose colour came before themes
    /// did, or a swatch.
    pub fn of_color(color: u32) -> Theme {
        let hsla = Hsla::from(rgb(color));
        let angle = hsla.h * std::f32::consts::TAU;
        let theme = Theme::at(hsla.l * angle.cos(), hsla.l * angle.sin());
        Theme { saturation: hsla.s, ..theme }
    }

    /// How far round from the main colour each colour is, in degrees.
    pub fn offsets(count: usize) -> &'static [f32] {
        match count {
            0 | 1 => &[0.],
            2 => &[0., 180.],
            _ => &[0., 150., 210.],
        }
    }

    /// Where colour `index` is in the field, as `x` and `y` are.
    pub fn handle(&self, index: usize) -> (f32, f32) {
        let offset = Self::offsets(self.count).get(index).copied().unwrap_or(0.);
        rotated(self.x, self.y, offset)
    }

    /// The colours, the main one first, 0xRRGGBB.
    pub fn colors(&self) -> Vec<u32> {
        (0..self.count)
            .map(|index| {
                let (x, y) = self.handle(index);
                color_at(x, y, self.saturation)
            })
            .collect()
    }

    /// The main colour, which the core keeps as the Space's colour.
    pub fn main_color(&self) -> u32 {
        color_at(self.x, self.y, self.saturation)
    }

    /// The theme with colour `index` dragged to (x, y): the others keep their places round it,
    /// and all take the field's colours there.
    pub fn moved(self, index: usize, x: f32, y: f32) -> Theme {
        let offset = Self::offsets(self.count).get(index).copied().unwrap_or(0.);
        let (x, y) = rotated(x, y, -offset);
        let (x, y) = in_disc(x, y);
        if (x, y) == (self.x, self.y) {
            return self;
        }
        Theme { x, y, saturation: 1., ..self }
    }

    /// The theme with `count` colours, from 1 to MAX_COLORS.
    pub fn with_count(self, count: usize) -> Theme {
        Theme { count: count.clamp(1, MAX_COLORS), ..self }
    }

    /// The text the core keeps; the saturation only once a swatch has set it, so a theme from the
    /// field reads as it did before swatches kept theirs, and a dark look as its place, count,
    /// intensity, grain and saturation.
    pub fn to_text(&self) -> String {
        let text = format!(
            "x={:.4} y={:.4} colors={} intensity={:.3} grain={:.3} appearance={}",
            self.x,
            self.y,
            self.count,
            self.intensity,
            self.grain,
            self.appearance.name()
        );
        let text = if self.saturation < 1. {
            format!("{text} saturation={:.4}", self.saturation)
        } else {
            text
        };
        match self.dark {
            Some(Look { x, y, saturation, count, intensity, grain }) => format!(
                "{text} dark={x:.4},{y:.4},{count},{intensity:.3},{grain:.3},{saturation:.4}"
            ),
            None => text,
        }
    }

    /// The theme `text` describes, as `to_text` writes it; None for empty text, or text without the
    /// main colour's place. What it doesn't say is as a new theme has it.
    pub fn parse(text: &str) -> Option<Theme> {
        let (mut x, mut y) = (None, None);
        let mut theme = Theme::at(0., 0.);
        for pair in text.split_whitespace() {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            let number = value.parse::<f32>().ok().filter(|value| value.is_finite());
            match key {
                "x" => x = number,
                "y" => y = number,
                "colors" => {
                    if let Ok(count) = value.parse::<usize>() {
                        theme.count = count.clamp(1, MAX_COLORS);
                    }
                }
                "intensity" => theme.intensity = number.unwrap_or(theme.intensity).clamp(0., 1.),
                "grain" => theme.grain = number.unwrap_or(theme.grain).clamp(0., 1.),
                "saturation" => {
                    theme.saturation = number.unwrap_or(theme.saturation).clamp(0., 1.)
                }
                "appearance" => {
                    if let Some(appearance) = Appearance::ALL.iter().find(|a| a.name() == value) {
                        theme.appearance = *appearance;
                    }
                }
                "dark" => theme.dark = parse_look(value),
                _ => {}
            }
        }
        let (x, y) = in_disc(x?, y?);
        Some(Theme { x, y, ..theme })
    }
}

/// A dark look as `to_text` writes it: x, y, count, intensity, grain and saturation.
fn parse_look(text: &str) -> Option<Look> {
    let numbers: Vec<f32> = (text.split(','))
        .map(|n| n.parse::<f32>().ok().filter(|n| n.is_finite()))
        .collect::<Option<_>>()?;
    let [x, y, count, intensity, grain, saturation] = numbers[..] else {
        return None;
    };
    let (x, y) = in_disc(x, y);
    let count = (count as usize).clamp(1, MAX_COLORS);
    let [intensity, grain, saturation] = [intensity, grain, saturation].map(|n| n.clamp(0., 1.));
    Some(Look { x, y, saturation, count, intensity, grain })
}

/// (x, y) turned `degrees` clockwise about the field's centre.
fn rotated(x: f32, y: f32, degrees: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (x * cos - y * sin, x * sin + y * cos)
}

/// (x, y), brought in to the field's disc.
fn in_disc(x: f32, y: f32) -> (f32, f32) {
    let distance = x.hypot(y);
    if distance > 1. { (x / distance, y / distance) } else { (x, y) }
}

/// The colour at (x, y) in the field, at HSL `saturation` (the field's own is 1), 0xRRGGBB.
pub fn color_at(x: f32, y: f32, saturation: f32) -> u32 {
    let hue = y.atan2(x).to_degrees().rem_euclid(360.) / 360.;
    packed(hsla(hue, saturation, x.hypot(y).min(1.), 1.).into())
}

/// `color` as 0xRRGGBB.
pub fn packed(color: Rgba) -> u32 {
    let channel = |c: f32| (c.clamp(0., 1.) * 255.).round() as u32;
    channel(color.r) << 16 | channel(color.g) << 8 | channel(color.b)
}

/// `from` mixed toward `to` by `t`, in OKLab, as the window's gradient between two colours is.
pub fn mix_oklab(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let (a, b) = (oklab(from), oklab(to));
    let mixed = std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
    let [r, g, bl] = from_oklab(mixed);
    Rgba { r, g, b: bl, a: from.a + (to.a - from.a) * t }
}

pub(crate) fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn from_linear(c: f32) -> f32 {
    let c = c.clamp(0., 1.);
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1. / 2.4) - 0.055 }
}

pub(crate) fn oklab(color: Rgba) -> [f32; 3] {
    let (r, g, b) = (to_linear(color.r), to_linear(color.g), to_linear(color.b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

pub(crate) fn from_oklab(lab: [f32; 3]) -> [f32; 3] {
    oklab_to_linear(lab).map(from_linear)
}

/// The linear sRGB channels of OKLab `[l, a, b]`, unclamped: outside 0 to 1 where it's out of
/// gamut.
pub(crate) fn oklab_to_linear([l, a, b]: [f32; 3]) -> [f32; 3] {
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m_ = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s_ = (l - 0.0894841775 * a - 1.2914855480 * b).powi(3);
    [
        4.0767416621 * l_ - 3.3077118613 * m_ + 0.2309699292 * s_,
        -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
        -0.0041960863 * l_ - 0.7034186147 * m_ + 1.7076127010 * s_,
    ]
}

/// How far along the window's diagonal, from its top left (0) to its bottom right (1), the point
/// `x` and `y` of the way across and down is, in a window `width` by `height`: a 135° CSS
/// gradient's line, whose ends' perpendiculars pass through those corners.
pub fn diagonal(x: f32, y: f32, width: f32, height: f32) -> f32 {
    ((x * width + y * height) / (width + height).max(1.)).clamp(0., 1.)
}

pub use crate::tokens::theme::GRAIN_STRENGTH;

/// Grain's side, in device pixels: a tile the window repeats.
pub const GRAIN_TILE: usize = 256;

thread_local! {
    static GRAIN: OnceCell<Arc<RenderImage>> = const { OnceCell::new() };
}

/// Noise to lay over the window's colours: each pixel white or black at a random strength, so it
/// neither lightens nor darkens them on the whole. Made once, the same each time.
pub fn grain_image() -> Arc<RenderImage> {
    GRAIN.with(|grain| {
        grain
            .get_or_init(|| {
                let mut state: u32 = 0x9e37_79b9;
                let mut data = Vec::with_capacity(GRAIN_TILE * GRAIN_TILE * 4);
                for _ in 0..GRAIN_TILE * GRAIN_TILE {
                    // xorshift32
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    let value = (state >> 24) as i32 - 128;
                    let shade = if value >= 0 { 255 } else { 0 };
                    let alpha = (value.unsigned_abs() * 2).min(255) as u8;
                    data.extend([shade, shade, shade, alpha]);
                }
                let image = RgbaImage::from_raw(GRAIN_TILE as u32, GRAIN_TILE as u32, data)
                    .expect("sized to fit");
                Arc::new(RenderImage::new(vec![Frame::new(image)]))
            })
            .clone()
    })
}

#[cfg(test)]
mod tests;
