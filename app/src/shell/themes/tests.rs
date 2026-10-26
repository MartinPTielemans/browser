//! A Space's theme in the window: what the window's colours are with one, two or three colours,
//! and the picker's theme showing as it's made until the core has it.

use std::rc::Rc;

use gpui::{Entity, TestAppContext, VisualTestContext, px, size};

use super::*;
use crate::shell::recorder::Recorder;

const BLUE: u32 = 0x3d81f9;

/// The shell with one Space, with `color` and `theme`, in a window as big as Arc's recording.
fn shell(
    color: Option<u32>,
    theme: Option<Theme>,
    cx: &mut TestAppContext,
) -> (Entity<Shell>, &mut VisualTestContext) {
    let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(Rc::new(Recorder::default()), cx));
    shell.update(cx, |shell, cx| {
        shell.set_spaces(vec![entry(color, theme)], cx);
        shell.set_space(1, cx);
    });
    cx.run_until_parked();
    shell.update(cx, |shell, _| shell.viewport = size(px(1800.), px(1130.)));
    (shell, cx)
}

/// Space 1 as the core reports it, with `color` and `theme`.
fn entry(color: Option<u32>, theme: Option<Theme>) -> crate::shell::SpaceEntry {
    let text = theme.map_or(String::new(), |theme| theme.to_text());
    (1, "Space".into(), color, 1, "".into(), "".into(), text.into())
}

fn rgb_of(color: Rgba) -> u32 {
    theme::packed(color)
}

#[gpui::test]
fn a_theme_of_one_colour_at_its_start_looks_as_the_colour_alone(cx: &mut TestAppContext) {
    let theme = Theme::at(0.3, -0.5);
    let color = theme.main_color();
    let (alone, cx) = shell(Some(color), None, cx);
    let before: Vec<_> = alone.update(cx, |shell, _| {
        [0., 0.5, 1.].map(|y| shell.window_fill_at(y)).to_vec()
    });
    alone.update(cx, |shell, cx| shell.set_spaces(vec![entry(Some(color), Some(theme))], cx));
    let after: Vec<_> = alone.update(cx, |shell, _| {
        [0., 0.5, 1.].map(|y| shell.window_fill_at(y)).to_vec()
    });
    assert_eq!(before, after);
}

#[gpui::test]
fn two_colours_go_from_the_top_left_to_the_second_along_the_diagonal(cx: &mut TestAppContext) {
    let theme = Theme::at(0.6, 0.).with_count(2);
    let [main, second] = theme.colors()[..] else { panic!("two colours") };
    let (shell, cx) = shell(Some(main), Some(theme), cx);
    shell.update(cx, |shell, _| {
        assert_eq!(rgb_of(shell.theme_color_at(&theme, 0., 0.)), main);
        assert_eq!(rgb_of(shell.theme_color_at(&theme, 1., 1.)), second);
        // The right edge is the second colour from three quarters of the way down the diagonal.
        let (width, height) = (1800., 1130.);
        let y = (SECOND_AT * (width + height) - width) / height;
        assert_eq!(rgb_of(shell.theme_color_at(&theme, 1., y + 0.01)), second);
        assert_ne!(rgb_of(shell.theme_color_at(&theme, 1., y - 0.2)), second);
        // A third comes in down the window.
        let three = theme.with_count(3);
        let third = three.colors()[2];
        let bottom_left = shell.theme_color_at(&three, 0., 1.);
        assert_eq!(rgb_of(bottom_left), rgb_of(theme::mix_oklab(rgb(main), rgb(third), 1.)));
    });
}

#[gpui::test]
fn intensity_washes_the_colours_out(cx: &mut TestAppContext) {
    let theme = Theme::at(0.6, 0.);
    let (shell, cx) = shell(Some(theme.main_color()), Some(theme), cx);
    shell.update(cx, |shell, _| {
        let alpha =
            |intensity| shell.theme_color_at(&Theme { intensity, ..theme }, 0.5, 0.5).a;
        assert!(alpha(0.) < alpha(0.24) && alpha(0.24) < alpha(DEFAULT_INTENSITY));
        assert!(alpha(DEFAULT_INTENSITY) < alpha(1.));
        assert_eq!(alpha(DEFAULT_INTENSITY), GRADIENT_ALPHA);
    });
}

#[gpui::test]
fn the_picker_s_theme_shows_at_once_and_gives_way_to_the_kept_one(cx: &mut TestAppContext) {
    let (shell, cx) = shell(Some(BLUE), None, cx);
    let made = Theme::at(-0.4, 0.2).with_count(2);
    shell.update(cx, |shell, cx| {
        shell.theme_changed(1, made, false, cx);
        assert_eq!(shell.theme_of(1), Some(made));
        // The core's report of another theme, such as the colour kept ahead of it, leaves it.
        shell.set_spaces(vec![entry(Some(made.main_color()), None)], cx);
        assert_eq!(shell.theme_of(1), Some(made));
        shell.set_spaces(vec![entry(Some(made.main_color()), Some(made))], cx);
        assert_eq!(shell.theme_preview, None);
        assert_eq!(shell.theme_of(1).map(|theme| theme.to_text()), Some(made.to_text()));
    });
}

/// GPUI's shader, as gpui_apple's fill_color has it for a linear gradient on a square, `side`
/// across from the origin, with stops at `start` and `end`: how far from the first colour to the
/// second `point` is.
fn shader_t(angle: f32, side: f32, start: f32, end: f32, point: (f32, f32)) -> f32 {
    let radians = (angle - 90.).to_radians();
    let direction = (radians.cos(), radians.sin());
    let half = side / 2.;
    let to_point = (point.0 - half, point.1 - half);
    let dot = to_point.0 * direction.0 + to_point.1 * direction.1;
    let along = dot / direction.0.hypot(direction.1);
    let t = (along + half) / side;
    ((t - start) / (end - start)).clamp(0., 1.)
}

#[test]
fn the_diagonal_is_drawn_where_it_is_taken_for_what_covers_it() {
    for (width, height) in [(1800., 1130.), (800., 1200.), (1000., 1000.)] {
        let (side, start, end) = diagonal_stops(width, height);
        for (x, y) in [(0., 0.), (0.2, 0.7), (0.5, 0.5), (0.9, 0.1), (1., 1.), (0.6, 0.3)] {
            let drawn = shader_t(135., side, start, end, (x * width, y * height));
            let taken = (theme::diagonal(x, y, width, height) / SECOND_AT).min(1.);
            assert!((drawn - taken).abs() < 1e-4, "{width}×{height} at {x}, {y}: {drawn} {taken}");
        }
    }
}
