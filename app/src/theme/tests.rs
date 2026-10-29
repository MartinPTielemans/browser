use super::*;

/// A point in Arc's field, measured in the window at 2×, in the field's units: its centre is at
/// (437, 265) and its half width 170 pt.
fn field(x: f32, y: f32) -> (f32, f32) {
    ((x - 437.) / 170., (y - 265.) / 170.)
}

fn channels(color: u32) -> [i32; 3] {
    [(color >> 16) as i32, (color >> 8 & 0xff) as i32, (color & 0xff) as i32]
}

fn close(color: u32, expected: [i32; 3], within: i32) -> bool {
    channels(color).iter().zip(expected).all(|(a, b)| (a - b).abs() <= within)
}

/// The field gives the colours Arc's handles showed where they stood.
#[test]
fn the_field_gives_arcs_colours() {
    for (x, y, expected) in [
        (476., 355., [230, 255, 35]),
        (510.8, 351.2, [254, 224, 81]),
        (362.8, 178.2, [82, 112, 255]),
        (305.8, 252.2, [131, 244, 255]),
        (535.2, 307.2, [255, 137, 62]),
    ] {
        let (x, y) = field(x, y);
        let color = color_at(x, y, 1.);
        assert!(close(color, expected, 12), "{color:06x} for {expected:?}");
    }
}

/// A second colour is the first's complement; with a third, the two sit 150° and 210° round.
#[test]
fn more_colours_go_round_the_main_one() {
    let (x, y) = field(477., 356.);
    let two = Theme::at(x, y).with_count(2);
    let (cx, cy) = two.handle(1);
    assert!((cx + x).abs() < 1e-4 && (cy + y).abs() < 1e-4);
    // Arc's second handle at 18.12 s.
    let arc = field(397., 173.);
    assert!((cx - arc.0).abs() < 0.03 && (cy - arc.1).abs() < 0.03);
    let three = two.with_count(3);
    let angle = |(x, y): (f32, f32)| y.atan2(x).to_degrees().rem_euclid(360.);
    let main = angle(three.handle(0));
    assert!(((angle(three.handle(1)) - main).rem_euclid(360.) - 150.).abs() < 0.01);
    assert!(((angle(three.handle(2)) - main).rem_euclid(360.) - 210.).abs() < 0.01);
    assert_eq!(three.colors().len(), 3);
    assert_eq!(three.with_count(9).count, MAX_COLORS);
    assert_eq!(three.with_count(0).count, 1);
}

/// Dragging any colour carries the others with it, and the one dragged lands where it's let go.
#[test]
fn a_drag_moves_them_together() {
    let theme = Theme::at(0.5, 0.).with_count(3);
    let moved = theme.moved(1, 0., -0.6);
    let (x, y) = moved.handle(1);
    assert!(x.abs() < 1e-4 && (y + 0.6).abs() < 1e-4);
    assert!((moved.x.hypot(moved.y) - 0.6).abs() < 1e-4);
    // Past the edge, the colour stays on it, white.
    let out = theme.moved(0, 3., 4.);
    assert!((out.x - 0.6).abs() < 1e-4 && (out.y - 0.8).abs() < 1e-4);
    assert_eq!(out.main_color(), 0xffffff);
}

/// The centre is black, and lightness grows outward.
#[test]
fn the_field_runs_from_black_to_white() {
    assert_eq!(color_at(0., 0., 1.), 0x000000);
    assert_eq!(color_at(1., 0., 1.), 0xffffff);
    assert_eq!(color_at(0.5, 0., 1.), 0xff0000);
}

/// What the core keeps reads back as the same theme, and text without a place is no theme.
#[test]
fn a_theme_survives_its_text() {
    let theme = Theme {
        x: -0.25,
        y: 0.5,
        saturation: 0.6,
        count: 3,
        intensity: 0.4,
        grain: 0.7,
        appearance: Appearance::Dark,
        dark: None,
    };
    let back = Theme::parse(&theme.to_text()).unwrap();
    assert!((back.x - theme.x).abs() < 1e-3 && (back.y - theme.y).abs() < 1e-3);
    assert_eq!((back.count, back.appearance), (3, Appearance::Dark));
    assert!((back.intensity - 0.4).abs() < 1e-3 && (back.grain - 0.7).abs() < 1e-3);
    assert!((back.saturation - 0.6).abs() < 1e-3);
    // The field's own saturation goes unsaid, as before swatches kept theirs.
    assert!(!Theme::at(0.5, 0.).to_text().contains("saturation"));
    assert_eq!(Theme::parse("x=0.5 y=0").unwrap().saturation, 1.);
    assert_eq!(Theme::parse(""), None);
    // A dark look comes back with it, and a theme of one look says none.
    let dark = Look { x: 0.1, y: -0.2, saturation: 0.5, count: 2, intensity: 0.3, grain: 0.9 };
    let back = Theme::parse(&Theme { dark: Some(dark), ..theme }.to_text()).unwrap();
    let got = back.dark.unwrap();
    assert!((got.x - 0.1).abs() < 1e-3 && (got.y + 0.2).abs() < 1e-3 && got.count == 2);
    assert!((got.intensity - 0.3).abs() < 1e-3 && (got.grain - 0.9).abs() < 1e-3 && (got.saturation - 0.5).abs() < 1e-3);
    assert!(!theme.to_text().contains("dark="));
    assert_eq!(Theme::parse("colors=2"), None);
    let partial = Theme::parse("x=0.1 y=0.2 colors=7 grain=bad").unwrap();
    assert_eq!((partial.count, partial.grain, partial.intensity), (3, 0., DEFAULT_INTENSITY));
}

/// A colour from before themes opens the picker at its hue and lightness.
#[test]
fn a_colour_finds_its_place() {
    let theme = Theme::of_color(0xff0000);
    assert!((theme.x - 0.5).abs() < 1e-3 && theme.y.abs() < 1e-3);
    let pink = Theme::of_color(0xbf556a);
    let hsla = Hsla::from(rgb(0xbf556a));
    assert!((pink.x.hypot(pink.y) - hsla.l).abs() < 1e-3);
}

/// A swatch's theme is the swatch's own colour, and its other colours keep its saturation; a
/// handle dragged takes the field's colour.
#[test]
fn a_swatch_keeps_its_own_colour_until_a_drag() {
    for swatch in crate::theme_picker::SWATCHES {
        assert_eq!(Theme::of_color(swatch).main_color(), swatch, "{swatch:06x}");
    }
    let slate = Theme::of_color(0x656786).with_count(2);
    let second = Hsla::from(rgb(slate.colors()[1]));
    assert!((second.s - Hsla::from(rgb(0x656786)).s).abs() < 0.02, "{second:?}");
    let dragged = slate.moved(0, slate.x + 0.1, slate.y);
    assert_eq!(dragged.saturation, 1.);
    assert_eq!(dragged.main_color(), color_at(dragged.x, dragged.y, 1.));
    // A press that doesn't move it leaves the swatch's colour.
    assert_eq!(slate.moved(0, slate.x, slate.y), slate);
}

/// The diagonal runs from the window's top left to its bottom right, its middle at the centre.
#[test]
fn the_diagonal_spans_the_window() {
    assert_eq!(diagonal(0., 0., 1800., 1130.), 0.);
    assert_eq!(diagonal(1., 1., 1800., 1130.), 1.);
    assert!((diagonal(0.5, 0.5, 1800., 1130.) - 0.5).abs() < 1e-6);
}

/// OKLab mixing keeps its ends and stays between them.
#[test]
fn oklab_mixing_keeps_its_ends() {
    let (a, b) = (Rgba::from(rgb(0xff8800)), Rgba::from(rgb(0x0044ff)));
    assert_eq!(packed(mix_oklab(a, b, 0.)), 0xff8800);
    assert_eq!(packed(mix_oklab(a, b, 1.)), 0x0044ff);
}
