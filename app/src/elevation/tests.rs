use super::*;

const LEVELS: [Level; 3] = [Level::Resting, Level::Popover, Level::Modal];

#[test]
fn each_level_floats_higher_than_the_last() {
    let host: Vec<HostShadow> = LEVELS.map(Level::host_shadow).to_vec();
    for pair in host.windows(2) {
        assert!(pair[0].shadow_opacity < pair[1].shadow_opacity);
        assert!(pair[0].shadow_radius < pair[1].shadow_radius);
        assert!(pair[0].shadow_drop < pair[1].shadow_drop);
    }
    let rims: Vec<f32> = LEVELS.iter().map(|level| level.rim().a).collect();
    assert!(rims.windows(2).all(|pair| pair[0] < pair[1]), "{rims:?}");
}

#[test]
fn every_level_has_the_same_outline() {
    let first = Level::Resting.host_shadow();
    for level in LEVELS {
        let shadow = level.host_shadow();
        assert_eq!(shadow.outline_opacity, first.outline_opacity);
        assert_eq!(shadow.outline_radius, first.outline_radius);
        // GPUI draws it as the shadow over the drop shadow, with no offset.
        let drawn = level.shadow();
        assert_eq!(drawn.len(), 2);
        assert_eq!(drawn[1].offset, point(px(0.), px(0.)));
        assert_eq!(drawn[1].blur_radius, px(1.));
    }
}

#[test]
fn the_host_takes_a_blur_as_half_its_radius() {
    // The popover's blur is 24 and its drop 8, a black at 0x59 of 0xff.
    let popover = Level::Popover.host_shadow();
    assert_eq!(popover.shadow_radius, 12.);
    assert_eq!(popover.shadow_drop, 8.);
    assert!((popover.shadow_opacity - 0x59 as f32 / 255.).abs() < 1e-6);
    // The outline, a blur of 1 at 0xb3.
    assert_eq!(popover.outline_radius, 0.5);
    assert!((popover.outline_opacity - 0xb3 as f32 / 255.).abs() < 1e-6);
}
