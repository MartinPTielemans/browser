//! A Space's theme in the window: opening the theme picker for it, showing what the picker makes
//! as it's made and keeping it, and the window's colours and grain from it (docs/ux-spec.md ›
//! Themes).

use std::rc::Rc;

use gpui::{
    ColorSpace, Context, Hsla, Point, Rgba, canvas, div, linear_color_stop, linear_gradient, point,
    prelude::*, px, rgb, size,
};

use super::{GRADIENT_ALPHA, NEUTRAL, NEUTRAL_GRADIENT_ALPHA, Shell};
use crate::{
    theme::{self, DEFAULT_INTENSITY, GRAIN_STRENGTH, GRAIN_TILE, Theme},
    theme_picker::{self, Changed},
};

/// Two colours go from the main one at the window's top left to the second this far along its
/// diagonal (Arc 1.170 rec at 2×, with Ardia's account of Arc's rule: OKLab, 45°, to 75 %).
const SECOND_AT: f32 = 0.75;
/// How strongly the window takes a theme's colours at intensity `i`: in proportion to
/// `INTENSITY_FLOOR + INTENSITY_SPAN * i`, so the slider's start is the colour as before themes
/// and its left end washes most of it out (ours, fitted to Arc's window at 0.81 and 0.24).
const INTENSITY_FLOOR: f32 = 0.2;
const INTENSITY_SPAN: f32 = 0.55;

impl Shell {
    /// The theme `space` shows: the one the picker is making for it, else its own.
    pub(super) fn theme_of(&self, space: u64) -> Option<Theme> {
        match self.theme_preview {
            Some((previewed, theme)) if previewed == space => Some(theme),
            _ => self.spaces.iter().find(|each| each.id == space).and_then(|each| each.theme),
        }
    }

    /// Shows the theme picker for `space` pointing at `anchor`, the middle of a row's right end
    /// in the window, with the Space's theme, or its colour alone if it has no theme.
    pub(super) fn show_theme_picker(
        &mut self,
        space: u64,
        anchor: Point<f32>,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.spaces.iter().find(|each| each.id == space) else {
            return;
        };
        let theme = entry.theme.or_else(|| entry.color.map(Theme::of_color));
        let shell = cx.weak_entity();
        let changed: Changed = Rc::new(move |theme, done, cx| {
            shell.update(cx, |shell, cx| shell.theme_changed(space, theme, done, cx)).ok();
        });
        let height = f32::from(self.viewport.height);
        theme_picker::show(space, theme, anchor, height, changed, cx);
    }

    /// The picker made `theme` for `space`: the window shows it at once, and once the user is
    /// `done` the core keeps it, with its main colour as the Space's colour.
    fn theme_changed(&mut self, space: u64, theme: Theme, done: bool, cx: &mut Context<Self>) {
        self.theme_preview = Some((space, theme));
        if done {
            self.browser.set_space_color(space, Some(theme.main_color()), cx);
            self.browser.set_space_theme(space, &theme.to_text(), cx);
        }
        cx.notify();
    }

    /// Takes `space`'s theme and colour away, so it shows the system's accent again.
    pub(super) fn remove_space_theme(&mut self, space: u64, cx: &mut Context<Self>) {
        if self.theme_preview.is_some_and(|(previewed, _)| previewed == space) {
            self.theme_preview = None;
        }
        self.browser.set_space_theme(space, "", cx);
        self.browser.set_space_color(space, None, cx);
        cx.notify();
    }

    /// The Spaces came from the core: the theme being made gives way to the Space's own once
    /// the core has it, or the Space has gone.
    pub(super) fn settle_theme_preview(&mut self) {
        let Some((space, theme)) = self.theme_preview else {
            return;
        };
        let kept = self.spaces.iter().find(|each| each.id == space).map(|each| each.theme);
        if kept.is_none_or(|kept| kept.map(|kept| kept.to_text()) == Some(theme.to_text())) {
            self.theme_preview = None;
        }
    }

    /// `theme`'s colour `x` of the way across the window and `y` of the way down: one colour going
    /// to the same at HSL lightness + 0.25 down the window, as a Space's colour does; two from the
    /// main one at the top left to the second along the diagonal; and a third coming in toward
    /// the bottom.
    pub(super) fn theme_color_at(&self, theme: &Theme, x: f32, y: f32) -> Rgba {
        let colors: Vec<Rgba> = theme.colors().into_iter().map(rgb).collect();
        let main = Hsla::from(colors[0]);
        let color = match colors.len() {
            1 => Rgba::from(Hsla { l: (main.l + 0.25 * y).min(1.), ..main }),
            count => {
                let (width, height) = (self.viewport.width.into(), self.viewport.height.into());
                let along = (theme::diagonal(x, y, width, height) / SECOND_AT).min(1.);
                let two = theme::mix_oklab(colors[0], colors[1], along);
                if count > 2 { theme::mix_oklab(two, colors[2], y) } else { two }
            }
        };
        Rgba { a: theme_alpha(theme, main.s), ..color }
    }

    /// Where the sidebar's middle is across the window, 0 to 1, where the window's colour is taken
    /// for what covers it.
    pub(super) fn sidebar_x(&self) -> f32 {
        self.width() / 2. / f32::from(self.viewport.width).max(1.)
    }

    /// The window's colour behind everything: the gradient down it, or with a Space of more than
    /// one colour shown, its colours across it, one Space's over the other's as the sidebar pages
    /// between them; and the grain over them.
    pub(super) fn window_background(&self) -> gpui::Div {
        let (space, beside) = self.colored_pages();
        // Each Space shown, and how much of the window's colour is its.
        let pages = match beside {
            Some((beside, t)) => vec![(space, 1. - t), (beside, t)],
            None => vec![(space, 1.)],
        };
        let themes: Vec<_> = pages
            .into_iter()
            .map(|(space, weight)| {
                let theme = self.theme_of(space).filter(|_| !self.private);
                (space, theme, weight)
            })
            .collect();
        let layered = themes.iter().any(|(_, theme, _)| theme.is_some_and(|theme| theme.count > 1));
        let background = div().absolute().size_full();
        let background = if layered {
            background.children(themes.iter().map(|&(space, theme, weight)| {
                self.theme_layer(theme, space).opacity(weight)
            }))
        } else {
            background.bg(self.gradient(None))
        };
        let grain: f32 = themes
            .iter()
            .map(|(_, theme, weight)| theme.map_or(0., |theme| theme.grain) * weight)
            .sum();
        background.when(grain > 0., |background| background.child(grain_layer(grain)))
    }

    /// `space`'s colours over the whole window, with `theme`.
    fn theme_layer(&self, theme: Option<Theme>, space: u64) -> gpui::Div {
        let layer = div().absolute().top_0().left_0();
        let Some(theme) = theme.filter(|theme| theme.count > 1) else {
            return layer.size_full().bg(self.space_gradient(space));
        };
        let colors: Vec<Rgba> = theme.colors().into_iter().map(rgb).collect();
        let alpha = theme_alpha(&theme, Hsla::from(colors[0]).s);
        let stop = |color: Rgba, at: f32| linear_color_stop(Rgba { a: alpha, ..color }, at);
        let (width, height) = (f32::from(self.viewport.width), f32::from(self.viewport.height));
        let (side, start, end) = diagonal_stops(width, height);
        let diagonal = linear_gradient(135., stop(colors[0], start), stop(colors[1], end))
            .color_space(ColorSpace::Oklab);
        let layer = layer.size(px(side)).bg(diagonal);
        match colors.get(2) {
            Some(&third) => layer.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(width))
                    .h(px(height))
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(Rgba { a: 0., ..third }, 0.),
                        stop(third, 1.),
                    )
                    .color_space(ColorSpace::Oklab)),
            ),
            None => layer,
        }
    }
}

/// The square the diagonal is drawn on, from the window's top left, and its stops, for a window
/// `width` by `height`. GPUI's 135° gradient on a square runs along its diagonal from -0.207 at
/// its top left corner (0.5 - √2 / 2) to 1.207 at its bottom right, so on one as big as the
/// window's longer side, these stops put the main colour at the window's top left and the second
/// SECOND_AT of the way along the window's own 135° line, as CSS draws it.
fn diagonal_stops(width: f32, height: f32) -> (f32, f32, f32) {
    let side = width.max(height).max(1.);
    let start = 0.5 - std::f32::consts::FRAC_1_SQRT_2;
    let end = start + std::f32::consts::FRAC_1_SQRT_2 * SECOND_AT * (width + height) / side;
    (side, start, end)
}

/// How opaque `theme`'s colours lie over the window, its main colour `saturation` saturated: as a
/// Space's colour alone at the intensity a theme starts with.
fn theme_alpha(theme: &Theme, saturation: f32) -> f32 {
    let alpha = if saturation < NEUTRAL { NEUTRAL_GRADIENT_ALPHA } else { GRADIENT_ALPHA };
    let at = |intensity: f32| INTENSITY_FLOOR + INTENSITY_SPAN * intensity;
    alpha * at(theme.intensity) / at(DEFAULT_INTENSITY)
}

/// Noise over the whole window, `grain` strong (0 to 1), in tiles of the grain image at one device
/// pixel to a pixel.
fn grain_layer(grain: f32) -> impl IntoElement {
    div().absolute().size_full().opacity(grain * GRAIN_STRENGTH).child(
        canvas(
            |_, _, _| {},
            |bounds, _, window, _| {
                let image = theme::grain_image();
                let tile = GRAIN_TILE as f32 / window.scale_factor();
                let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                let (columns, rows) = ((width / tile).ceil() as i32, (height / tile).ceil() as i32);
                for row in 0..rows {
                    for column in 0..columns {
                        let origin =
                            bounds.origin + point(px(column as f32 * tile), px(row as f32 * tile));
                        let tile = gpui::Bounds::new(origin, size(px(tile), px(tile)));
                        window.paint_image(tile, tile, px(0.).into(), image.clone(), 0, false).ok();
                    }
                }
            },
        )
        .size_full(),
    )
}

#[cfg(test)]
mod tests;
