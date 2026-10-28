use super::*;
use crate::theme::Appearance;

/// A theme as the design sweep (#320) names one: `count` colours, the main one at `hue` degrees,
/// HSL `saturation` and `lightness` (its distance from the field's centre).
fn theme(count: usize, grain: f32, appearance: &str, hue: f32, saturation: f32, lightness: f32, intensity: f32) -> Theme {
    let angle = hue.to_radians();
    let at = Theme::at(lightness * angle.cos(), lightness * angle.sin());
    let appearance = *Appearance::ALL.iter().find(|a| a.name() == appearance).unwrap();
    Theme { saturation, count, intensity, grain, appearance, ..at }
}

fn hex(color: Rgba) -> u32 {
    theme::packed(color)
}

/// The window, finer than the palette checks it: 17 points along the gradient's line and 17 down,
/// the sidebar's and the page's alike, each over a black and a white backdrop.
fn fine_window(palette: &Palette, theme: Option<&Theme>) -> Vec<Rgba> {
    let steps = 16;
    (0..=steps)
        .flat_map(|along| (0..=steps).map(move |y| (along as f32 / steps as f32, y as f32 / steps as f32)))
        .flat_map(|(along, y)| palette.window_at(theme, along, y))
        .collect()
}

/// What fails: each of the chrome's components drawn as the shell draws it, with its alpha
/// from the token ladder, on every point of the window, below its WCAG minimum.
fn failures(theme: Option<&Theme>) -> Vec<String> {
    let palette = Palette::of(theme);
    let window = fine_window(&palette, theme);
    let grain = theme.map_or(0., |theme| theme.grain);
    let ink = palette.ink;
    let grained: Vec<Rgba> = window.iter().map(|&at| over(at, ink, GRAIN_SHIFT * grain)).collect();
    let plated = |plate: f32| -> Vec<Rgba> { window.iter().map(|&at| over(at, ink, plate)).collect() };
    let mut failed = Vec::new();
    let mut check = |name: &str, worst: f32, minimum: f32| {
        if worst < minimum - 1e-4 {
            failed.push(format!("{name} {worst:.2} < {minimum}"));
        }
    };
    let on = |color: Hsla, grounds: &[Rgba]| {
        let color = Rgba::from(color);
        grounds.iter().map(|&at| contrast(over(at, color, color.a), at)).fold(f32::MAX, f32::min)
    };
    // The sidebar's text and glyphs (#320 rule 5's ladder).
    check("selected title", on(palette.text(1.), &plated(SELECTED_PLATE)), TEXT);
    check("unselected title", on(palette.text(0.78), &grained), TEXT);
    check("title on hovered plate", on(palette.text(0.78), &plated(HOVERED_PLATE)), TEXT);
    check("URL host on field", on(palette.text(0.62), &plated(FIELD_PLATE)), TEXT);
    for name in ["Space name", "New Tab", "Clear", "secondary"] {
        check(name, on(palette.text(0.62), &grained), TEXT);
    }
    check("nav glyph", on(palette.glyph(0.6), &grained), GLYPH);
    check("New Tab +", on(palette.glyph(0.45), &grained), GLYPH);
    check("Space dot", on(palette.glyph(0.6), &grained), GLYPH);
    check("other Space dot", on(palette.glyph(0.35), &grained), GLYPH);
    // The accent, wherever it's drawn, and its label.
    let accent = palette.accent;
    let against = |grounds: &[Rgba]| grounds.iter().map(|&at| contrast(accent, at)).fold(f32::MAX, f32::min);
    check("focus ring, New Space circle", against(&window), GLYPH);
    check("label on accent", contrast(palette.accent_label, accent), TEXT);
    check("selected row vs command bar", contrast(accent, palette.overlay), GLYPH);
    check("dialog action vs dialog", contrast(accent, palette.dialog), GLYPH);
    check("Library section glyph", against(&plated(SECTION_PLATE)), GLYPH);
    let targets: Vec<Rgba> = plated(TARGET_PLATE).iter().map(|&at| over(at, accent, TARGET_ACCENT)).collect();
    check("split target dashes", against(&targets), GLYPH);
    // What floats over the page.
    check("overlay text", contrast(ink, palette.overlay), TEXT);
    check("dialog text", contrast(ink, palette.dialog), TEXT);
    // The pickers' card over the window, and the text on it.
    let cards: Vec<Rgba> = window.iter().map(|&at| over(at, palette.card, palette.card.a)).collect();
    let white = Hsla::from(WHITE);
    check("picker hint", on(white.opacity(rgba(CARD_HINT).a), &cards), TEXT);
    let segments: Vec<Rgba> = cards.iter().map(|&at| over(at, WHITE, rgba(CARD_SEGMENTS).a)).collect();
    let label = segments.iter().map(|&at| contrast(rgb(CARD_LABEL), at)).fold(f32::MAX, f32::min);
    check("picker segment label", label, TEXT);
    // The Space's swatch, ringed where its edge doesn't reach 3:1 on the window.
    let swatch = match theme {
        Some(theme) => Window::of(theme).swatch(),
        None => vec![rgb(NO_THEME_ACCENT)],
    };
    let edge = match Palette::swatch_ring_on(theme, &window) {
        Some(ring) => window.iter().map(|&at| contrast(ring, at)).fold(f32::MAX, f32::min),
        None => {
            let each = swatch.iter().flat_map(|&c| window.iter().map(move |&at| contrast(c, at)));
            each.fold(f32::MAX, f32::min)
        }
    };
    check("Space swatch edge", edge, GLYPH);
    failed
}

/// The themes of a grid over the picker's output: colours, field, intensity, grain, appearance.
fn grid(hues: &[f32], saturations: &[f32], lightnesses: &[f32], intensities: &[f32]) -> Vec<Theme> {
    let mut themes = Vec::new();
    for count in 1..=3 {
        for &grain in &[0., 1.] {
            for appearance in Appearance::ALL {
                for &hue in hues {
                    for &saturation in saturations {
                        for &lightness in lightnesses {
                            for &intensity in intensities {
                                let theme = theme(count, grain, appearance.name(), hue, saturation, lightness, intensity);
                                themes.push(theme);
                            }
                        }
                    }
                }
            }
        }
    }
    themes
}

fn sweep(themes: &[Theme]) -> Vec<String> {
    themes
        .iter()
        .filter_map(|theme| {
            let failed = failures(Some(theme));
            (!failed.is_empty()).then(|| format!("{}: {}", theme.to_text(), failed.join(", ")))
        })
        .collect()
}

/// A Space with no theme has the measured window, black 0.25 over the material and nothing more,
/// and ilo green made to read with Ink on it, and everything passes.
#[test]
fn no_theme_is_the_measured_window_and_ilo_green_with_ink() {
    let palette = Palette::of(None);
    assert_eq!((palette.tint.a, palette.shade, palette.layer_alpha), (TINT, 0., 0.));
    // #289054 reads 2.7:1 on the window over a white backdrop, so ilo green, lighter, at its hue.
    assert_eq!(hex(palette.accent), 0x57b97a);
    assert_eq!(hex(palette.accent_label), INK);
    assert!(contrast(palette.accent_label, palette.accent) >= TEXT);
    // #0B8045 with Paper, the value #319 first had, is under 4.5.
    assert!(contrast(rgb(PAPER), rgb(0x0b8045)) < TEXT);
    assert_eq!(failures(None), Vec::<String>::new());
}

/// A theme lies over the window as a Space's colour did before themes, and only a theme too light
/// for Paper over a white backdrop is shaded, by the least step that lets it read.
#[test]
fn the_window_is_shaded_only_where_paper_needs_it() {
    // check/window.sh's black Space, measured against Dia's composite: neutral, unshaded.
    let black = Palette::of(Some(&Theme::at(0., 0.)));
    assert_eq!((black.tint.a, black.shade, black.layer_alpha), (TINT, 0., NEUTRAL_GRADIENT_ALPHA));
    let violet = Palette::of(Some(&Theme::of_color(0x5a2a9a)));
    assert_eq!((violet.shade, violet.layer_alpha), (0., GRADIENT_ALPHA));
    // Blue lightens to #9AC3FF down the window, too light for Paper's plates over white.
    let pale = Theme::of_color(0x1a79ff);
    let palette = Palette::of(Some(&pale));
    assert!(palette.shade > 0., "the blue isn't shaded");
    let less = Palette { shade: palette.shade - SHADE_STEP, ..palette };
    let paper = rgb(PAPER);
    let fails = less.window_samples(Some(&pale)).iter().any(|&at| {
        contrast(paper, over(at, paper, SELECTED_PLATE)) < TEXT + BETWEEN
    });
    assert!(fails, "a step less shade would have read");
}

/// The pickers' card keeps Arc's measured grey where its text reads, and Capture's glyphs reach
/// 3:1 on its menu and a hovered row.
#[test]
fn the_pickers_card_and_captures_glyphs_read() {
    let black = Palette::of(Some(&Theme::at(0., 0.)));
    assert_eq!(hex(black.card), hex(rgba(CARD)), "the measured grey over the black Space");
    assert_eq!(Palette::of(None).card, rgba(CARD), "the measured grey with no theme");
    use crate::tokens::capture::{GLYPH_COLOR, MENU_FILL, ROW_HOVERED};
    for ground in [rgba(MENU_FILL), over(rgba(MENU_FILL), WHITE, rgba(ROW_HOVERED).a)] {
        assert!(contrast(over(ground, rgb(PAPER), GLYPH_COLOR), ground) >= GLYPH);
    }
}

/// The themes that came out worst for each component in the design sweep, today's and v3's.
#[test]
fn the_design_sweeps_worst_themes_pass() {
    let worst = [
        (3, 0., "dark", 340., 1., 0.5, 0.4),
        (3, 0., "dark", 340., 0.75, 0.9, 0.7),
        (1, 0., "automatic", 0., 0.05, 0.65, 0.9),
        (1, 0., "light", 350., 0.35, 0.4, 0.8),
        (3, 0., "automatic", 110., 0.05, 0.55, 1.),
        (3, 0., "automatic", 330., 1., 0.55, 0.4),
        (2, 1., "automatic", 60., 0.05, 0.35, 0.8),
        (2, 1., "automatic", 110., 0.1, 0.75, 0.5),
        (3, 1., "dark", 200., 0.2, 0.75, 0.2),
        (1, 1., "light", 320., 0.2, 0.5, 0.4),
        (1, 0., "light", 80., 0.35, 0.35, 0.9),
        (1, 0., "automatic", 90., 0.1, 0.15, 0.),
        (1, 0., "automatic", 280., 1., 0.95, 1.),
        (2, 0., "automatic", 270., 0.1, 0.15, 0.9),
        (2, 0., "dark", 300., 1., 0.45, 0.),
        (3, 1., "automatic", 180., 1., 0.95, 1.),
        (3, 1., "automatic", 300., 1., 0.95, 1.),
        (3, 1., "automatic", 60., 1., 0.95, 1.),
        (3, 0., "automatic", 270., 1., 0.15, 0.2),
        (1, 0., "automatic", 60., 1., 0.55, 0.),
        (1, 0., "automatic", 60., 1., 0.5, 0.),
        (1, 0., "automatic", 180., 0.05, 0.1, 0.),
        (1, 0., "automatic", 30., 0.5, 0.1, 0.),
        (2, 0., "automatic", 300., 0.35, 0.15, 0.3),
        (2, 0., "automatic", 90., 0.2, 0.15, 0.7),
    ];
    let themes: Vec<Theme> =
        worst.iter().map(|&(n, g, app, hue, s, l, i)| theme(n, g, app, hue, s, l, i)).collect();
    let mut themes = themes;
    // The picker's own swatches, and the field's extremes.
    for color in [0xf0eae5, 0xe59eba, 0x9e749b, 0xe1676f, 0xef8c63, 0xf2d46c, 0x73e49b, 0x7eb6d5, 0x656786] {
        themes.push(Theme::of_color(color));
    }
    for color in [0x000000, 0xffffff, 0x808080, 0x00ff6a, 0xffff00, 0x0000ff] {
        themes.push(Theme::of_color(color));
    }
    assert_eq!(sweep(&themes), Vec::<String>::new());
}

/// Every theme of a coarse grid over the picker's output keeps every component readable.
#[test]
fn every_theme_of_a_coarse_sweep_reads() {
    let themes = grid(
        &[0., 60., 120., 180., 240., 300.],
        &[0., 0.4, 1.],
        &[0., 0.2, 0.45, 0.6, 0.8, 1.],
        &[0., crate::theme::DEFAULT_INTENSITY, 1.],
    );
    assert_eq!(sweep(&themes), Vec::<String>::new());
}


/// The design sweep's grid, 1,197,504 themes: `cargo test --release --lib -- --ignored
/// full_sweep`, a few minutes.
#[test]
#[ignore]
fn full_sweep() {
    let hues: Vec<f32> = (0..36).map(|i| i as f32 * 10.).collect();
    let saturations = [0., 0.05, 0.1, 0.2, 0.35, 0.5, 0.75, 1.];
    let lightnesses: Vec<f32> = (0..=20).map(|i| i as f32 / 20.).collect();
    let intensities: Vec<f32> = (0..=10).map(|i| i as f32 / 10.).collect();
    let themes = grid(&hues, &saturations, &lightnesses, &intensities);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let failed: Vec<String> = std::thread::scope(|scope| {
        let chunks: Vec<_> = themes
            .chunks(themes.len().div_ceil(threads))
            .map(|chunk| scope.spawn(move || sweep(chunk)))
            .collect();
        chunks.into_iter().flat_map(|chunk| chunk.join().unwrap()).collect()
    });
    let palettes: Vec<Palette> = themes.iter().map(|t| Palette::of(Some(t))).collect();
    let shaded = palettes.iter().filter(|p| p.shade > 0.).count();
    let most = palettes.iter().map(|p| p.shade).fold(0., f32::max);
    let card = palettes.iter().filter(|p| p.card != rgba(CARD)).count();
    eprintln!(
        "{} themes, {shaded} shaded (at most {most}), {card} with a darker card, {} failing",
        themes.len(),
        failed.len()
    );
    assert_eq!(failed.iter().take(20).collect::<Vec<_>>(), Vec::<&String>::new());
}

/// A swatch that reads on its ground has no ring; one that doesn't takes Ink or Paper, whichever
/// stands out there.
#[test]
fn a_swatch_is_ringed_only_where_it_fades_into_its_ground() {
    let garden = Theme::of_color(0x00ff6a);
    assert_eq!(Palette::swatch_ring(Some(&garden), rgb(0x0b0b0b)), None);
    assert_eq!(Palette::swatch_ring(Some(&garden), rgb(0xfafaf9)).map(hex), Some(INK));
    let night = Theme::of_color(0x110066);
    assert_eq!(Palette::swatch_ring(Some(&night), rgb(0x0b0b0b)).map(hex), Some(PAPER));
}

/// The palette is worked out once a theme changes, not while drawing, so it has to be quick.
#[test]
fn a_palette_takes_well_under_a_frame() {
    let theme = theme(3, 1., "automatic", 300., 1., 0.95, 1.);
    let start = std::time::Instant::now();
    for _ in 0..10 {
        std::hint::black_box(Palette::of(Some(&theme)));
    }
    let each = start.elapsed() / 10;
    assert!(each < std::time::Duration::from_millis(4), "{each:?}");
}
