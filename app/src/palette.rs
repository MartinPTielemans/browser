//! What the chrome draws with over a Space's theme, or with none (#320's rules): the window's
//! underlay and how strongly the theme lies over it, Ink or Paper for its text and glyphs, the
//! Space's accent and the label on it, and the opaque fills of what floats over the page.
//!
//! A theme is the user's, anything the picker makes, so what's drawn over it is chosen against
//! the window's every point rather than one colour of it: the theme's layer is held back (k) only
//! as far as the chrome's text needs, Ink or Paper is whichever lets it show more, and the accent
//! is the main colour moved in OKLab lightness, its hue kept, until it reads everywhere it's
//! drawn. It's worked out once when a Space's theme changes, never while drawing. The tests sweep
//! the picker's output and hold every component to WCAG's minimums.

use gpui::{
    ColorSpace, Div, Global, Hsla, Rgba, div, linear_color_stop, linear_gradient, prelude::*, px,
    rgb,
};

use crate::theme::{self, Appearance, GRAIN_STRENGTH, Theme};

pub use crate::tokens::palette::{INK, NO_THEME_ACCENT, OVERLAY_DARK, PAPER};
use crate::tokens::palette::{DARK_UNDERLAY, DIALOG_DARK, LIGHT_UNDERLAY};
/// The window's material is black 0.25 over Dia's (#131313 over grey 20); the dark underlay is
/// black 0.4 more.
const MATERIAL_TINT: f32 = 0.25;
const DARKER: f32 = 0.4;
/// Automatic appearance takes the light underlay over this mean OKLab lightness of the colours.
const LIGHT_OVER: f32 = 0.72;
pub use crate::tokens::palette::SECOND_AT;
use crate::tokens::palette::{INTENSITY_FLOOR, INTENSITY_SPAN, RING};
/// The ends of the scale each family's text runs toward, which the accent's search needs, and the
/// ground a swatch's colours are judged on.
const BLACK: Rgba = Rgba { r: 0., g: 0., b: 0., a: 1. };
const WHITE: Rgba = Rgba { r: 1., g: 1., b: 1., a: 1. };
/// k's steps, from 1 down.
const K_STEP: f32 = 0.05;
/// WCAG's minimums: text, and glyphs, boundaries and focus rings.
pub const TEXT: f32 = 4.5;
pub const GLYPH: f32 = 3.;
/// The accent's label aims past TEXT, so rounding to 8 bits can't take it under.
const LABEL: f32 = 4.6;
/// Grain at 1 moves the window toward the text's colour by half its strength on the whole, where
/// the text has least room.
const GRAIN_SHIFT: f32 = GRAIN_STRENGTH * 0.5;
/// The plates under rows, in Paper: the selected row's, one under the pointer, the URL field's,
/// a Library section's and a split target's. Ink's are lighter by INK_PLATES, as Ink weighs more
/// on a light window than Paper does on a dark one.
pub const SELECTED_PLATE: f32 = 0.2;
pub const HOVERED_PLATE: f32 = 0.12;
pub const FIELD_PLATE: f32 = 0.1;
pub const SECTION_PLATE: f32 = 0.09;
pub const TARGET_PLATE: f32 = 0.06;
const INK_PLATES: f32 = 0.65;
/// A split target lies the accent at this over its plate.
pub const TARGET_ACCENT: f32 = 0.15;
/// Where on the window the colours are checked: across the gradient's line and down it. The
/// tests check a finer grid.
const ALONG: [f32; 9] = [0., 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.];
const DOWN: [f32; 9] = ALONG;
/// The window between those points can dip a little under them, so what's checked at them has
/// this much to spare.
const BETWEEN: f32 = 0.05;

/// The colours the chrome draws with for one Space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// The window's underlay is the light one.
    pub light: bool,
    /// The underlay, opaque.
    pub underlay: Rgba,
    /// What goes over the window's material to make the underlay.
    pub tint: Rgba,
    /// How opaque the theme's colours lie over the underlay; 0 without a theme.
    pub layer_alpha: f32,
    /// The text and glyphs are Ink, on a light window, rather than Paper.
    pub ink_family: bool,
    /// Ink or Paper.
    pub ink: Rgba,
    /// The least alpha at which `ink` reads as text, and as a glyph, on the window and its
    /// plates everywhere.
    pub text_floor: f32,
    pub glyph_floor: f32,
    /// The Space's accent, for controls: filled rows and buttons, focus rings, toggles.
    pub accent: Rgba,
    /// Ink or Paper, whichever reads more on the accent.
    pub accent_label: Rgba,
    /// The opaque fill of what floats over the page: the command bar, menus, the theme picker's
    /// panel, toasts, the zoom HUD.
    pub overlay: Rgba,
    /// A dialog's fill.
    pub dialog: Rgba,
    /// The least alpha at which `ink` reads as text on the overlay and the dialog.
    pub overlay_floor: f32,
    /// The Space's mark.
    pub swatch: Swatch,
}

/// A Space's mark: its theme's colours as a small swatch (#320), drawn in the same gradient the
/// window is, with a hairline ring of Ink or Paper where its edge would fade into what's behind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swatch {
    /// The gradient's ends: the main colour and the second along 135°, reaching it SECOND_AT of
    /// the way, or a colour alone going lighter down.
    from: Rgba,
    to: Rgba,
    angle: f32,
    to_at: f32,
    /// A third colour coming in toward the bottom.
    third: Option<Rgba>,
    /// The relative luminance of its darkest and its lightest points.
    darkest: f32,
    lightest: f32,
}

impl Global for Palette {}

impl Default for Palette {
    fn default() -> Self {
        Palette::of(None)
    }
}

impl Palette {
    /// The palette for a Space with `theme`, or none.
    pub fn of(theme: Option<&Theme>) -> Palette {
        let Some(theme) = theme else {
            return Palette::untinted();
        };
        let window = Window::of(theme);
        let (ink_family, k) = window.family();
        let palette = Palette::with(window.light, ink_family, window.alpha * k, &window);
        let accent = palette.solve_accent(rgb(theme.main_color()), &window);
        let swatch = Swatch::of(Some(theme), &window);
        Palette { accent, accent_label: label_on(accent), swatch, ..palette }
    }

    /// No theme: the dark underlay alone, with Paper, and ilo green as the accent.
    fn untinted() -> Palette {
        let window = Window::of_none();
        let accent = rgb(NO_THEME_ACCENT);
        let swatch = Swatch::of(None, &window);
        let palette = Palette::with(false, false, 0., &window);
        Palette { accent, accent_label: label_on(accent), swatch, ..palette }
    }

    fn with(light: bool, ink_family: bool, layer_alpha: f32, window: &Window) -> Palette {
        let ink = rgb(if ink_family { INK } else { PAPER });
        let underlay = underlay(light);
        let mut palette = Palette {
            light,
            underlay,
            tint: tint(light),
            layer_alpha,
            ink_family,
            ink,
            text_floor: 0.,
            glyph_floor: 0.,
            accent: ink,
            accent_label: ink,
            overlay: rgb(if ink_family { PAPER } else { OVERLAY_DARK }),
            dialog: rgb(if ink_family { PAPER } else { DIALOG_DARK }),
            overlay_floor: 0.,
            swatch: Swatch::of(None, window),
        };
        palette.overlay_floor = least_alpha(ink, &[palette.overlay, palette.dialog], TEXT + BETWEEN);
        let grounds = palette.grounds(window);
        palette.text_floor = least_alpha(ink, &grounds, TEXT + BETWEEN);
        palette.glyph_floor = least_alpha(ink, &grounds, GLYPH + BETWEEN);
        palette
    }

    /// The window at every point checked, k applied, with grain moving it toward the ink; and
    /// under the selected, hovered and field plates.
    fn grounds(&self, window: &Window) -> Vec<Rgba> {
        let plain = window.samples(self.underlay, self.layer_alpha);
        let mut grounds: Vec<Rgba> =
            plain.iter().map(|&at| over(at, self.ink, GRAIN_SHIFT * window.grain)).collect();
        for plate in [SELECTED_PLATE, HOVERED_PLATE, FIELD_PLATE] {
            grounds.extend(plain.iter().map(|&at| over(at, self.ink, self.plate_alpha(plate))));
        }
        grounds
    }

    /// This palette `t` of the way to `other`'s, as the window's colour goes from one Space's to
    /// the next while the sidebar pages between them; Ink or Paper and the underlay change
    /// halfway.
    pub fn toward(&self, other: &Palette, t: f32) -> Palette {
        if t <= 0. {
            return *self;
        }
        if t >= 1. {
            return *other;
        }
        let mix = |a: Rgba, b: Rgba| Rgba {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        };
        let (near, _) = if t < 0.5 { (self, other) } else { (other, self) };
        Palette {
            light: near.light,
            underlay: mix(self.underlay, other.underlay),
            tint: mix(self.tint, other.tint),
            layer_alpha: self.layer_alpha + (other.layer_alpha - self.layer_alpha) * t,
            ink_family: near.ink_family,
            ink: mix(self.ink, other.ink),
            text_floor: self.text_floor.max(other.text_floor),
            glyph_floor: self.glyph_floor.max(other.glyph_floor),
            accent: mix(self.accent, other.accent),
            accent_label: near.accent_label,
            overlay: near.overlay,
            dialog: near.dialog,
            overlay_floor: self.overlay_floor.max(other.overlay_floor),
            swatch: near.swatch,
        }
    }

    /// Ink or Paper at `alpha`, or as much more as text needs to read on any theme.
    pub fn text(&self, alpha: f32) -> Hsla {
        Hsla::from(self.ink).opacity(alpha.max(self.text_floor))
    }

    /// Ink or Paper at `alpha`, or as much more as a glyph or boundary needs.
    pub fn glyph(&self, alpha: f32) -> Hsla {
        Hsla::from(self.ink).opacity(alpha.max(self.glyph_floor))
    }

    /// Ink or Paper at `alpha` over the overlay or the dialog, or as much more as text needs.
    pub fn on_overlay(&self, alpha: f32) -> Hsla {
        Hsla::from(self.ink).opacity(alpha.max(self.overlay_floor))
    }

    /// Ink or Paper at `alpha`, as it is: for what needn't read, such as a disabled glyph or a
    /// shadow.
    pub fn faint(&self, alpha: f32) -> Hsla {
        Hsla::from(self.ink).opacity(alpha)
    }

    /// The alpha of a plate that's `alpha` in Paper.
    pub fn plate_alpha(&self, alpha: f32) -> f32 {
        if self.ink_family { alpha * INK_PLATES } else { alpha }
    }

    /// A plate under a row or a field, `alpha` in Paper's terms.
    pub fn plate(&self, alpha: f32) -> Hsla {
        Hsla::from(self.ink).opacity(self.plate_alpha(alpha))
    }

    /// The window's opaque colour at the sidebar, `y` down it, with `theme` (this palette's own).
    pub fn window_at(&self, theme: Option<&Theme>, along: f32, y: f32) -> Rgba {
        match theme {
            Some(theme) => Layer::of(theme, along, y).over(self.underlay, self.layer_alpha),
            None => self.underlay,
        }
    }

    /// The hairline ring a swatch of `theme`'s colours needs on `behind`, opaque: Ink or Paper,
    /// whichever stands out, where any of the swatch is under 3:1 against it; none where all of
    /// it reads.
    pub fn swatch_ring(theme: Option<&Theme>, behind: Rgba) -> Option<Rgba> {
        Palette::swatch_ring_on(theme, &[behind])
    }

    /// The ring a swatch of `theme`'s colours needs on any of `behind`.
    pub fn swatch_ring_on(theme: Option<&Theme>, behind: &[Rgba]) -> Option<Rgba> {
        Swatch::of_theme(theme).ring_on(behind)
    }

    /// Where a swatch ring is checked against this palette's window: every point checked.
    pub fn window_samples(&self, theme: Option<&Theme>) -> Vec<Rgba> {
        match theme {
            Some(theme) => Window::of(theme).samples(self.underlay, self.layer_alpha),
            None => vec![self.underlay],
        }
    }

    /// The main colour, its hue kept, at the OKLab lightness nearest its own at which it reads
    /// against the window, the overlays, a Library section's plate and a split target, and its
    /// label reads on it; with as much of its chroma as the gamut there allows. Grey, at worst.
    fn solve_accent(&self, main: Rgba, window: &Window) -> Rgba {
        let plain = window.samples(self.underlay, self.layer_alpha);
        let section: Vec<Rgba> =
            plain.iter().map(|&at| over(at, self.ink, self.plate_alpha(SECTION_PLATE))).collect();
        let target: Vec<Rgba> =
            plain.iter().map(|&at| over(at, self.ink, self.plate_alpha(TARGET_PLATE))).collect();
        let passes = |c: Rgba| {
            contrast(rgb(INK), c).max(contrast(rgb(PAPER), c)) >= LABEL
                && contrast(c, self.overlay) >= GLYPH
                && contrast(c, self.dialog) >= GLYPH
                && plain.iter().chain(&section).all(|&at| contrast(c, at) >= GLYPH + BETWEEN)
                && target.iter().all(|&at| contrast(c, over(at, c, TARGET_ACCENT)) >= GLYPH + BETWEEN)
        };
        let [l, a, b] = theme::oklab(main);
        // A hundredth at a time, either way from its own lightness, the nearest first: with all
        // its chroma the gamut allows, then half, then none.
        for chroma in [1., 0.5, 0.] {
            for step in 0..=100 {
                for sign in [1., -1.] {
                    let at = l + sign * step as f32 * 0.01;
                    if !(0. ..=1.).contains(&at) || (step == 0 && sign < 0.) {
                        continue;
                    }
                    let color = in_gamut(at, a * chroma, b * chroma);
                    if passes(color) {
                        return color;
                    }
                }
            }
        }
        self.ink
    }
}

impl Swatch {
    /// `theme`'s swatch, or ilo green's for none, without the rest of its palette.
    pub fn of_theme(theme: Option<&Theme>) -> Swatch {
        match theme {
            Some(theme) => Swatch::of(Some(theme), &Window::of(theme)),
            None => Swatch::of(None, &Window::of_none()),
        }
    }

    fn of(theme: Option<&Theme>, window: &Window) -> Swatch {
        let Some(theme) = theme else {
            let accent = rgb(NO_THEME_ACCENT);
            let lum = luminance(accent);
            return Swatch {
                from: accent,
                to: accent,
                angle: 180.,
                to_at: 1.,
                third: None,
                darkest: lum,
                lightest: lum,
            };
        };
        let colors: Vec<Rgba> = theme.colors().into_iter().map(rgb).collect();
        let (from, to, angle, to_at) = match colors[..] {
            [_] => (Layer::of(theme, 0., 0.).color, Layer::of(theme, 0., 1.).color, 180., 1.),
            _ => (colors[0], colors[1], 135., SECOND_AT),
        };
        let lums = window.swatch().into_iter().map(luminance);
        let (darkest, lightest) = lums.fold((f32::MAX, f32::MIN), |(d, l), y| (d.min(y), l.max(y)));
        Swatch { from, to, angle, to_at, third: colors.get(2).copied(), darkest, lightest }
    }

    /// Ink or Paper for a hairline ring where any of the swatch is under 3:1 against any of
    /// `behind`; none where it all reads. Its colours run between its darkest and lightest, so a
    /// ground whose luminance lies between them always takes a ring.
    pub fn ring_on(&self, behind: &[Rgba]) -> Option<Rgba> {
        let reads = |y: f32, at: Rgba| {
            let b = luminance(at);
            (y.max(b) + 0.05) / (y.min(b) + 0.05) >= GLYPH
        };
        let all_read = behind.iter().all(|&at| {
            let b = luminance(at);
            !(self.darkest..=self.lightest).contains(&b)
                && reads(self.darkest, at)
                && reads(self.lightest, at)
        });
        if all_read {
            return None;
        }
        let (ink, paper) = (rgb(INK), rgb(PAPER));
        let worst = |ring: Rgba| behind.iter().map(|&b| contrast(ring, b)).fold(f32::MAX, f32::min);
        Some(if worst(ink) >= worst(paper) { ink } else { paper })
    }

    /// The swatch, `size` across, on `behind`.
    pub fn draw(&self, size: f32, behind: Rgba) -> Div {
        let gradient = linear_gradient(
            self.angle,
            linear_color_stop(self.from, 0.),
            linear_color_stop(self.to, self.to_at),
        )
        .color_space(ColorSpace::Oklab);
        div()
            .relative()
            .flex_none()
            .size(px(size))
            .rounded_full()
            .overflow_hidden()
            .bg(gradient)
            .when_some(self.third, |swatch, third| {
                let third = Hsla::from(third);
                swatch.child(div().absolute().size_full().bg(linear_gradient(
                    180.,
                    linear_color_stop(third.opacity(0.), 0.),
                    linear_color_stop(third, 1.),
                )))
            })
            .when_some(self.ring_on(&[behind]), |swatch, ring| {
                // The ring is its own layer, over the colours, so it doesn't move them.
                swatch.child(
                    div().absolute().size_full().rounded_full().border(px(RING)).border_color(ring),
                )
            })
    }
}

/// The theme's window, worked out once: its underlay, how opaque the theme's layer would be
/// at k = 1, and the layer's colours at every point checked.
struct Window {
    light: bool,
    alpha: f32,
    layers: Vec<Layer>,
    grain: f32,
}

/// The theme's colour at one point of the window: the gradient's, and a third colour over it at
/// a share of the layer's alpha.
#[derive(Clone, Copy)]
struct Layer {
    color: Rgba,
    third: Option<(Rgba, f32)>,
}

impl Layer {
    fn none() -> Layer {
        Layer { color: BLACK, third: None }
    }

    /// `theme`'s colour `along` the gradient's line (0 at the window's top left, 1 at its bottom
    /// right) and `y` down the window: one colour going to the same at HSL lightness + 0.25 down
    /// the window; two from the main one to the second SECOND_AT along; a third coming in toward
    /// the bottom.
    fn of(theme: &Theme, along: f32, y: f32) -> Layer {
        let colors: Vec<Rgba> = theme.colors().into_iter().map(rgb).collect();
        if colors.len() == 1 {
            let main = Hsla::from(colors[0]);
            return Layer { color: Hsla { l: (main.l + 0.25 * y).min(1.), ..main }.into(), third: None };
        }
        let along = (along / SECOND_AT).min(1.);
        let color = theme::mix_oklab(colors[0], colors[1], along);
        Layer { color, third: colors.get(2).map(|&third| (third, y)) }
    }

    fn over(&self, base: Rgba, alpha: f32) -> Rgba {
        let color = over(base, self.color, alpha);
        match self.third {
            Some((third, share)) => over(color, third, alpha * share),
            None => color,
        }
    }
}

impl Window {
    /// No theme: the underlay alone.
    fn of_none() -> Window {
        Window { light: false, alpha: 0., layers: vec![Layer::none()], grain: 0. }
    }

    fn of(theme: &Theme) -> Window {
        let colors: Vec<Rgba> = theme.colors().into_iter().map(rgb).collect();
        let light = match theme.appearance {
            Appearance::Light => true,
            Appearance::Dark => false,
            Appearance::Automatic => {
                let mean = colors.iter().map(|&c| theme::oklab(c)[0]).sum::<f32>();
                mean / colors.len() as f32 > LIGHT_OVER
            }
        };
        let alongs: &[f32] = if colors.len() == 1 { &[0.] } else { &ALONG };
        let layers = alongs
            .iter()
            .flat_map(|&along| DOWN.iter().map(move |&y| Layer::of(theme, along, y)))
            .collect();
        let alpha = INTENSITY_FLOOR + INTENSITY_SPAN * theme.intensity;
        Window { light, alpha, layers, grain: theme.grain }
    }

    fn samples(&self, underlay: Rgba, alpha: f32) -> Vec<Rgba> {
        self.layers.iter().map(|layer| layer.over(underlay, alpha)).collect()
    }

    /// The theme's colours as a swatch shows them, opaque.
    fn swatch(&self) -> Vec<Rgba> {
        self.layers.iter().map(|layer| layer.over(BLACK, 1.)).collect()
    }

    /// Ink or Paper, and the largest k, in steps, at which it reads on the window and the
    /// selected and hovered plates, and the family's end, black or white, stands out on the
    /// whole window, so an accent always exists. k = 0 is the underlay alone, where one of
    /// them always reads.
    fn family(&self) -> (bool, f32) {
        let underlay = underlay(self.light);
        let largest_k = |ink_family: bool| {
            let ink = rgb(if ink_family { INK } else { PAPER });
            let end = if ink_family { BLACK } else { WHITE };
            let plates = if ink_family { INK_PLATES } else { 1. };
            let steps = (1. / K_STEP).round() as i32;
            (0..=steps).rev().map(|step| step as f32 * K_STEP).find(|&k| {
                self.samples(underlay, self.alpha * k).iter().all(|&at| {
                    let grained = over(at, ink, GRAIN_SHIFT * self.grain);
                    contrast(end, at) >= GLYPH + BETWEEN
                        && contrast(ink, grained) >= TEXT + BETWEEN
                        && [SELECTED_PLATE, HOVERED_PLATE].iter().all(|&plate| {
                            contrast(ink, over(at, ink, plate * plates)) >= TEXT + BETWEEN
                        })
                })
            })
        };
        let (ink, paper) = (largest_k(true), largest_k(false));
        match (ink, paper) {
            (Some(ink), Some(paper)) if ink != paper => (ink > paper, ink.max(paper)),
            (Some(k), Some(_)) => {
                // Level: the one the window's middle, at k = 1, is closer to.
                let middle = self.layers[self.layers.len() / 2].over(underlay, self.alpha);
                (luminance(middle) > 0.18, k)
            }
            (Some(k), None) => (true, k),
            (None, Some(k)) => (false, k),
            (None, None) => (self.light, 0.),
        }
    }
}

fn underlay(light: bool) -> Rgba {
    rgb(if light { LIGHT_UNDERLAY } else { DARK_UNDERLAY })
}

/// What over the window's material makes the underlay: black 0.4 more over its tint, which
/// takes MATERIAL to DARK_UNDERLAY, or white so opaque nothing shows through.
fn tint(light: bool) -> Rgba {
    match light {
        true => rgb(LIGHT_UNDERLAY),
        false => Rgba { r: 0., g: 0., b: 0., a: 1. - (1. - MATERIAL_TINT) * (1. - DARKER) },
    }
}

/// Ink or Paper, whichever reads more on `color`.
fn label_on(color: Rgba) -> Rgba {
    let (ink, paper) = (rgb(INK), rgb(PAPER));
    if contrast(ink, color) >= contrast(paper, color) { ink } else { paper }
}

/// The least alpha, in 25ths, at which `ink` over each of `grounds` reaches `minimum`; 1 where
/// none does.
fn least_alpha(ink: Rgba, grounds: &[Rgba], minimum: f32) -> f32 {
    (0..=25)
        .map(|step| step as f32 / 25.)
        .find(|&alpha| grounds.iter().all(|&at| contrast(over(at, ink, alpha), at) >= minimum))
        .unwrap_or(1.)
}

/// OKLab `[l, a, b]` with its chroma cut to the gamut's edge, as 8-bit sRGB.
fn in_gamut(l: f32, a: f32, b: f32) -> Rgba {
    let inside = |share: f32| {
        theme::oklab_to_linear([l, a * share, b * share])
            .iter()
            .all(|&channel| (-1e-5..=1. + 1e-5).contains(&channel))
    };
    let (mut low, mut high) = (0., 1.);
    if inside(1.) {
        low = 1.;
    } else {
        for _ in 0..14 {
            let middle = (low + high) / 2.;
            if inside(middle) { low = middle } else { high = middle }
        }
    }
    let [r, g, b] = theme::from_oklab([l, a * low, b * low]);
    quantized(Rgba { r, g, b, a: 1. })
}

/// `color` as it is once it's 8 bits a channel.
pub fn quantized(color: Rgba) -> Rgba {
    rgb(theme::packed(color))
}

/// `color` at `alpha` over opaque `base`.
pub fn over(base: Rgba, color: Rgba, alpha: f32) -> Rgba {
    let mix = |b: f32, c: f32| b + (c - b) * alpha;
    Rgba { r: mix(base.r, color.r), g: mix(base.g, color.g), b: mix(base.b, color.b), a: 1. }
}

/// WCAG's relative luminance.
pub fn luminance(color: Rgba) -> f32 {
    let [r, g, b] = [color.r, color.g, color.b].map(theme::to_linear);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG's contrast ratio between two opaque colours.
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The palette of the Space shown, for what floats over the page: the command bar, menus, a
/// dialog, the theme picker.
pub fn current(cx: &gpui::App) -> Palette {
    cx.try_global::<Palette>().copied().unwrap_or_default()
}

/// The Space shown changed its palette.
pub fn set_current(palette: Palette, cx: &mut gpui::App) {
    if cx.try_global::<Palette>() != Some(&palette) {
        cx.set_global(palette);
    }
}

#[cfg(test)]
mod tests;
