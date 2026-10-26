//! The New Tab page's intro, drawn in the card under the command bar's box, which is a window of
//! its own: Dia's band rising in the Space's colour, and its halo running once around the box
//! (docs/ux-spec.md › New tab).

use std::{f32::consts::PI, time::Instant};

use gpui::{
    AnyElement, Bounds, Hsla, PathBuilder, Pixels, Rgba, Window, canvas, div, linear_color_stop,
    linear_gradient, point, prelude::*, px, rgb,
};

use super::{NEUTRAL, Shell};
use crate::command_bar;

// The band (Dia 1.50.1's binary and capture): the colour at 0.068 until 0.2 s, falling to 0.036
// by 0.4 s, held to 0.73 s, then fading to nothing by 3.4 s, when the intro ends.
const BAND_PEAK: f32 = 0.068;
const BAND_PLATEAU: f32 = 0.036;
const INTRO: f32 = 3.4;
// It's blurred 24, which softens its top as far either side of it, and it rises to 125 above
// the box. Its bottom fades out over the card's last 48, half gone 24 above it (Dia's: ≈ 20).
const BAND_BLUR: f32 = 24.;
const BAND_ABOVE: f32 = 125.;
// Its top starts 160 up, already over the card's bottom when the page appears, and slows as it
// goes, passing the box's middle at ≈ 0.3 s (ours, fitted to the capture).
const BAND_START: f32 = 160.;
const BAND_RISE: f32 = 5.2;
// A neutral Space's band and halo are grey 0.5 at 0.65.
const NEUTRAL_COLOR: Rgba = Rgba { r: 0.502, g: 0.502, b: 0.502, a: 0.65 };

// The halo (fitted to Dia 1.50.1's capture, the binary's shape): the middle of the lit arc runs
// from the box's bottom centre (0) to its top centre (1) and past it, at
// 1.25 · (1 − 2^(−11.2 (t − 0.225))), lighting the edge within 0.22 of it.
const HALO_START: f32 = 0.225;
const HALO_RATE: f32 = 11.2;
const HALO_SWEEP: f32 = 1.25;
const HALO_ARC: f32 = 0.22;
// Dia's line runs along the box's edge, half under the box, as far as 2.65 out at most, in the
// colour lightened by up to 0.4, at up to 0.4. Its glow goes out 10.6 to 21 at up to 0.1. Ours
// draws them as strokes centred on the edge, the glow's in steps: their widths, opacities, and
// how much they lighten the colour.
const HALO_STROKES: [(f32, f32, f32); 4] =
    [(28., 0.022, 0.), (18., 0.028, 0.), (10., 0.035, 0.), (3.2, 0.35, 0.35)];
// How far apart the points the strokes go through are, along the edge, and the steps the light
// along it is drawn in.
const HALO_STEP: f32 = 3.;
const HALO_LEVELS: f32 = 12.;

impl Shell {
    /// The intro, while it runs on the New Tab page, which it asks frames for. Once it has run,
    /// it's gone until the next ⌘T.
    pub(super) fn new_tab_intro(&mut self, window: &mut Window) -> Option<AnyElement> {
        let t = self.new_tab_intro?.elapsed().as_secs_f32();
        if t >= INTRO {
            self.new_tab_intro = None;
            // check.sh waits for this line.
            log!("browser: new tab intro done");
            return None;
        }
        window.request_animation_frame();
        let (card, _) = self.page_frame(self.viewport);
        let (bar, radius) = command_bar::empty_new_tab_box(card);
        let color = self.shown_space().and_then(|space| space.color).unwrap_or(self.accent_color);
        let color = rgb(color);
        let color = if Hsla::from(color).s < NEUTRAL { NEUTRAL_COLOR } else { color };
        let lit = halo_lit(t);
        let halo = canvas(
            |_, _, _| {},
            move |_, _, window, _| paint_halo(bar, radius, color, lit, window),
        )
        .absolute()
        .size_full();
        let band = band(card, bar.top().into(), color, t);
        Some(div().absolute().size_full().children(band).child(halo).into_any_element())
    }

    /// Starts the intro, unless the user asked for less motion, or stops it.
    pub(super) fn set_new_tab_intro(&mut self, shown: bool) {
        self.new_tab_intro = (shown && !self.browser.reduce_motion()).then(Instant::now);
    }
}

/// The band `t` s into the intro, in the card at `card` under the box whose top is at `top`: its
/// soft top, its body and its soft bottom.
fn band(card: Bounds<Pixels>, top: f32, color: Rgba, t: f32) -> Vec<gpui::Div> {
    let alpha = band_alpha(t);
    if alpha <= 0. {
        return Vec::new();
    }
    let color = Rgba { a: color.a * alpha, ..color };
    let clear = Rgba { a: 0., ..color };
    let (bottom, soft) = (f32::from(card.bottom()), 2. * BAND_BLUR);
    let head = band_top(bottom, top, t).min(bottom - 3. * BAND_BLUR);
    let strip = |top: f32, height: f32| {
        div().absolute().left(card.left()).w(card.size.width).top(px(top)).h(px(height))
    };
    let fade = |from: Rgba, to: Rgba| {
        linear_gradient(180., linear_color_stop(from, 0.), linear_color_stop(to, 1.))
    };
    vec![
        strip(head - BAND_BLUR, soft).bg(fade(clear, color)),
        strip(head + BAND_BLUR, bottom - soft - head - BAND_BLUR).bg(color),
        strip(bottom - soft, soft).bg(fade(color, clear)),
    ]
}

/// How much of its colour the band shows `t` s into the intro.
fn band_alpha(t: f32) -> f32 {
    match t {
        ..0.2 => BAND_PEAK,
        ..0.4 => BAND_PEAK + (BAND_PLATEAU - BAND_PEAK) * (t - 0.2) / 0.2,
        ..0.73 => BAND_PLATEAU,
        _ => BAND_PLATEAU * (1. - (t - 0.73) / (INTRO - 0.73)).max(0.),
    }
}

/// Where the band's top is `t` s into the intro, in a card whose bottom is at `bottom`, under a
/// box whose top is at `top`.
fn band_top(bottom: f32, top: f32, t: f32) -> f32 {
    let end = top - BAND_ABOVE;
    let start = (bottom - BAND_START).max(end);
    end + (start - end) * (-BAND_RISE * t).exp2()
}

/// How lit the halo is `t` s into the intro, at a point `around` the box from its bottom centre
/// (0) to its top centre (1).
fn halo_lit(t: f32) -> impl Fn(f32) -> f32 + 'static {
    let sweep = HALO_SWEEP * (1. - (-HALO_RATE * (t - HALO_START)).exp2());
    move |around| {
        let x = (1. - (around - sweep).abs() / HALO_ARC).clamp(0., 1.);
        x * x * (3. - 2. * x)
    }
}

/// Paints the halo around the box at `bounds`, lit by `lit`, as strokes through points along its
/// edge. The points lit to the same step make one path, so it's a few paths a frame.
fn paint_halo(
    bounds: Bounds<Pixels>,
    radius: f32,
    color: Rgba,
    lit: impl Fn(f32) -> f32,
    window: &mut Window,
) {
    let points = right_half(bounds, radius);
    let (cx, cy) = (f32::from(bounds.center().x), f32::from(bounds.center().y));
    let (hx, hy) = (f32::from(bounds.size.width) / 2., f32::from(bounds.size.height) / 2.);
    // Its two halves are lit alike, from the bottom up both sides, so one half's levels do.
    let levels: Vec<f32> = points
        .iter()
        .map(|&(x, y)| {
            let around = ((x - cx) / hx).atan2((y - cy) / hy) / PI;
            (lit(around) * HALO_LEVELS).round()
        })
        .collect();
    if levels.iter().all(|&level| level == 0.) {
        return;
    }
    for (width, alpha, lighten) in HALO_STROKES {
        let light = |c: f32| (c + lighten).min(1.);
        let stroke = Rgba { r: light(color.r), g: light(color.g), b: light(color.b), ..color };
        for level in 1..=HALO_LEVELS as usize {
            let mut path = PathBuilder::stroke(px(width));
            let mut drawn = false;
            // A run of points at this level, through the point after it, so runs meet.
            let mut start = 0;
            while start < levels.len() {
                let end = (start..levels.len()).find(|&i| levels[i] != levels[start]);
                let end = end.unwrap_or(levels.len());
                if levels[start] as usize == level {
                    let run = &points[start..(end + 1).min(points.len())];
                    for side in [1., -1.] {
                        let at = |&(x, y): &(f32, f32)| point(px(cx + side * (x - cx)), px(y));
                        path.move_to(at(&run[0]));
                        run[1..].iter().for_each(|each| path.line_to(at(each)));
                    }
                    drawn = true;
                }
                start = end;
            }
            if drawn && let Ok(path) = path.build() {
                let alpha = color.a * alpha * level as f32 / HALO_LEVELS;
                window.paint_path(path, Rgba { a: alpha, ..stroke });
            }
        }
    }
}

/// Points about `HALO_STEP` apart along the right half of the edge of the box at `bounds`, with
/// corners of `radius`, from its bottom centre round to its top centre.
fn right_half(bounds: Bounds<Pixels>, radius: f32) -> Vec<(f32, f32)> {
    let (left, top) = (f32::from(bounds.left()), f32::from(bounds.top()));
    let (right, bottom) = (f32::from(bounds.right()), f32::from(bounds.bottom()));
    let mut points = Vec::new();
    let line = |points: &mut Vec<_>, (x0, y0): (f32, f32), (x1, y1): (f32, f32)| {
        let steps = ((x1 - x0).hypot(y1 - y0) / HALO_STEP).ceil().max(1.);
        for i in 0..steps as usize {
            let f = i as f32 / steps;
            points.push((x0 + (x1 - x0) * f, y0 + (y1 - y0) * f));
        }
    };
    let corner = |points: &mut Vec<_>, (x, y): (f32, f32), from: f32| {
        let steps = (radius * PI / 2. / HALO_STEP).ceil().max(1.);
        for i in 0..steps as usize {
            let angle = from - PI / 2. * i as f32 / steps;
            points.push((x + radius * angle.cos(), y + radius * angle.sin()));
        }
    };
    let centre = (left + right) / 2.;
    line(&mut points, (centre, bottom), (right - radius, bottom));
    corner(&mut points, (right - radius, bottom - radius), PI / 2.);
    line(&mut points, (right, bottom - radius), (right, top + radius));
    corner(&mut points, (right - radius, top + radius), 0.);
    line(&mut points, (right - radius, top), (centre, top));
    points.push((centre, top));
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The halo lights the box's bottom centre at 225 ms, its top at 376 ms and leaves the top by
    /// 529 ms, where Dia's capture saw its line pass a third of its brightness.
    #[test]
    fn the_halo_runs_round_the_box_when_dias_does() {
        let (bottom, top) = (0., 1.);
        assert_eq!(halo_lit(0.1)(bottom), 0.);
        assert!(halo_lit(0.2)(bottom) < 1. / 3.);
        assert!(halo_lit(HALO_START)(bottom) > 1. / 3.);
        assert!(halo_lit(0.36)(top) < 1. / 3.);
        assert!(halo_lit(0.376)(top) < 1. / 3. && halo_lit(0.39)(top) > 1. / 3.);
        assert!(halo_lit(0.425)(top) > 0.95);
        assert!(halo_lit(0.51)(top) > 1. / 3. && halo_lit(0.529)(top) < 0.4);
        assert!((0..=100).all(|i| halo_lit(0.71)(i as f32 / 100.) == 0.));
    }

    /// In the window Dia's capture was of, the band passes the box's middle at about 300 ms and
    /// is about 125 above the box by 1.2 s, and by 3.25 s it adds less than half a level of
    /// Dia's pink to the page, so it's gone.
    #[test]
    fn the_band_rises_and_fades_as_dias() {
        let (bottom, top, middle) = (942., 375.5, 431.5);
        assert!(band_top(bottom, top, 0.27) > middle && band_top(bottom, top, 0.33) < middle);
        assert!((band_top(bottom, top, 1.2) - (top - BAND_ABOVE)).abs() < 10.);
        assert_eq!(band_alpha(0.15), BAND_PEAK);
        assert_eq!(band_alpha(0.6), BAND_PLATEAU);
        let (pink, page) = (181., 44.);
        assert!(band_alpha(3.25) * (pink - page) < 0.5);
        assert_eq!(band_alpha(INTRO), 0.);
    }
}
