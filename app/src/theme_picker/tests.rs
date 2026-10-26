//! The theme picker's clicks, drags and keys, on a host that records what the picker asks of its
//! window and the themes it hands on.

use std::{cell::RefCell, rc::Rc};

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};

use super::*;

#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<String>>,
    themes: RefCell<Vec<(Theme, bool)>>,
}

impl Host for Recorder {
    fn show(&self, left: f32, top: f32, corners: &[(f32, f32, f32)]) {
        let (_, tip, _) = corners[0];
        self.calls.borrow_mut().push(format!("show {left:.1} {top:.1} tip {tip:.1}"));
    }

    fn hide(&self) {
        self.calls.borrow_mut().push("hide".into());
    }
}

/// The picker, shown for Space 7 with `theme`, its arrow at (262, 300) in a window 900 tall.
struct Picker {
    cx: VisualTestContext,
    window: WindowHandle<ThemePicker>,
    host: Rc<Recorder>,
}

impl Picker {
    fn new(theme: Option<Theme>, cx: &mut TestAppContext) -> Self {
        let host = Rc::new(Recorder::default());
        let window = cx.update(|cx| open(host.clone(), cx)).expect("no window for the picker");
        let recorded = host.clone();
        let changed: Changed =
            Rc::new(move |theme, done, _| recorded.themes.borrow_mut().push((theme, done)));
        cx.update(|cx| show(7, theme, point(262., 300.), 900., changed, cx));
        let cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        Picker { cx, window, host }
    }

    /// Where `x`, `y` in the card is in the window.
    fn at(x: f32, y: f32) -> Point<Pixels> {
        point(px(ARROW + x), px(y))
    }

    /// Where the field's point `x`, `y` is in the card.
    fn in_field(x: f32, y: f32) -> (f32, f32) {
        let middle = FIELD_INSET + FIELD / 2.;
        (middle + x * FIELD / 2., middle + y * FIELD / 2.)
    }

    fn click(&mut self, x: f32, y: f32) {
        self.cx.simulate_click(Self::at(x, y), Modifiers::none());
        self.cx.run_until_parked();
    }

    fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        let left = MouseButton::Left;
        self.cx.simulate_mouse_down(Self::at(from.0, from.1), left, Modifiers::none());
        self.cx.run_until_parked();
        self.cx.simulate_mouse_move(Self::at(to.0, to.1), left, Modifiers::none());
        self.cx.run_until_parked();
        self.cx.simulate_mouse_up(Self::at(to.0, to.1), left, Modifiers::none());
        self.cx.run_until_parked();
    }

    fn theme(&mut self) -> Option<Theme> {
        let editing = self.window.read_with(&self.cx.cx, |picker, _| picker.editing()).unwrap();
        editing.and_then(|(_, theme)| theme)
    }

    fn open(&mut self) -> bool {
        self.cx.cx.update(|cx| is_open(cx))
    }

    /// Whether each theme handed on was kept, since the last look.
    fn handed_on(&mut self) -> Vec<bool> {
        self.host.themes.borrow_mut().drain(..).map(|(_, done)| done).collect()
    }

    fn calls(&mut self) -> Vec<String> {
        self.host.calls.borrow().clone()
    }
}

fn close(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

#[gpui::test]
fn a_click_on_the_empty_field_places_the_first_colour_there(cx: &mut TestAppContext) {
    let mut picker = Picker::new(None, cx);
    let (x, y) = Picker::in_field(0.2, 0.45);
    picker.click(x, y);
    let theme = picker.theme().expect("no colour placed");
    assert!(close((theme.x, theme.y), (0.2, 0.45)), "at {}, {}", theme.x, theme.y);
    assert_eq!(theme.count, 1);
    assert_eq!(theme.intensity, DEFAULT_INTENSITY);
    // Handed on as it lands, and kept as the button comes up.
    assert_eq!(picker.handed_on(), vec![false, true]);
}

#[gpui::test]
fn dragging_a_handle_moves_every_colour_and_keeps_them_on_letting_go(cx: &mut TestAppContext) {
    let mut picker = Picker::new(Some(Theme::at(0.5, 0.).with_count(3)), cx);
    let complement = picker.theme().unwrap().handle(1);
    // Taken off its middle, the handle keeps where the pointer took it.
    let (x, y) = Picker::in_field(0.5 + 0.05, 0.);
    let (to_x, to_y) = Picker::in_field(0.05, 0.3);
    picker.drag((x, y), (to_x, to_y));
    let theme = picker.theme().unwrap();
    assert!(close((theme.x, theme.y), (0., 0.3)), "at {}, {}", theme.x, theme.y);
    assert_eq!(theme.count, 3);
    assert_ne!(theme.handle(1), complement);
    assert_eq!(picker.handed_on(), vec![false, true]);
}

#[gpui::test]
fn a_secondary_handle_drags_the_set_round(cx: &mut TestAppContext) {
    let mut picker = Picker::new(Some(Theme::at(0.5, 0.).with_count(2)), cx);
    // The complement is at (-0.5, 0); dragged to (0, -0.5), the main colour goes to (0, 0.5).
    let (x, y) = Picker::in_field(-0.5, 0.);
    let (to_x, to_y) = Picker::in_field(0., -0.5);
    picker.drag((x, y), (to_x, to_y));
    let theme = picker.theme().unwrap();
    assert!(close((theme.x, theme.y), (0., 0.5)), "at {}, {}", theme.x, theme.y);
}

#[gpui::test]
fn plus_and_minus_add_and_take_colours_from_one_to_three(cx: &mut TestAppContext) {
    let mut picker = Picker::new(Some(Theme::at(0.5, 0.)), cx);
    picker.click(REMOVE_X, COUNT_Y);
    assert_eq!(picker.theme().unwrap().count, 1, "− with one colour");
    for count in [2, 3, 3] {
        picker.click(ADD_X, COUNT_Y);
        assert_eq!(picker.theme().unwrap().count, count);
    }
    picker.click(REMOVE_X, COUNT_Y);
    assert_eq!(picker.theme().unwrap().count, 2);
    // Each change that took is kept at once.
    assert_eq!(picker.handed_on(), vec![true, true, true]);
}

#[gpui::test]
fn a_swatch_gives_its_colour_and_keeps_the_rest(cx: &mut TestAppContext) {
    let theme = Theme { intensity: 0.4, grain: 0.3, ..Theme::at(0.5, 0.).with_count(2) };
    let mut picker = Picker::new(Some(theme), cx);
    picker.click(SWATCH_X + SWATCH_PITCH * 3., SWATCH_Y);
    let picked = picker.theme().unwrap();
    assert_eq!(picked, Theme { x: picked.x, y: picked.y, ..theme });
    let swatch = Theme::of_color(SWATCHES[3]);
    assert_eq!(picked, Theme { count: 2, intensity: 0.4, grain: 0.3, ..swatch });
    // Without a colour yet, a swatch is the first.
    let mut picker = Picker::new(None, cx);
    picker.click(SWATCH_X, SWATCH_Y);
    assert_eq!(picker.theme(), Some(Theme::of_color(SWATCHES[0])));
}

#[gpui::test]
fn the_wave_sets_the_intensity_and_the_dial_the_grain(cx: &mut TestAppContext) {
    let mut picker = Picker::new(Some(Theme::at(0.5, 0.)), cx);
    let knob = knob_x(DEFAULT_INTENSITY);
    picker.drag((knob, SLIDER_Y), (knob_x(0.25), SLIDER_Y + 10.));
    assert!((picker.theme().unwrap().intensity - 0.25).abs() < 0.001);
    // Handed on as the knob is taken and moved, and kept as it's let go.
    assert_eq!(picker.handed_on(), vec![false, false, true]);
    // A quarter turn from the top, then on past the top, where it stops.
    let right = (DIAL.0 + DIAL_RING, DIAL.1);
    picker.drag((DIAL.0, DIAL.1 - DIAL_RING), right);
    assert!((picker.theme().unwrap().grain - 0.25).abs() < 0.001);
    let left = (DIAL.0 - DIAL_RING, DIAL.1);
    let past_top = (DIAL.0 + 5., DIAL.1 - DIAL_RING);
    let (button, none) = (MouseButton::Left, Modifiers::none());
    picker.cx.simulate_mouse_down(Picker::at(left.0, left.1), button, none);
    for (x, y) in [(DIAL.0 - 5., DIAL.1 - DIAL_RING), past_top] {
        picker.cx.simulate_mouse_move(Picker::at(x, y), button, none);
        picker.cx.run_until_parked();
    }
    picker.cx.simulate_mouse_up(Picker::at(past_top.0, past_top.1), button, none);
    picker.cx.run_until_parked();
    assert_eq!(picker.theme().unwrap().grain, 1.);
}

#[gpui::test]
fn without_a_colour_only_the_field_swatches_and_appearance_answer(cx: &mut TestAppContext) {
    let mut picker = Picker::new(None, cx);
    picker.click(ADD_X, COUNT_Y);
    picker.click(knob_x(0.5), SLIDER_Y);
    picker.click(DIAL.0 + DIAL_RING, DIAL.1);
    assert_eq!(picker.theme(), None);
    assert!(picker.handed_on().is_empty());
    // The appearance chosen first goes with the first colour.
    picker.click(APPEARANCE_X[2], APPEARANCE_Y);
    let (x, y) = Picker::in_field(0., 0.5);
    picker.click(x, y);
    assert_eq!(picker.theme().unwrap().appearance, Appearance::Dark);
}

#[gpui::test]
fn escape_closes_it_and_a_click_outside_leaves_it_closed(cx: &mut TestAppContext) {
    let mut picker = Picker::new(Some(Theme::at(0.5, 0.)), cx);
    assert!(picker.open());
    picker.cx.simulate_keystrokes("escape");
    picker.cx.run_until_parked();
    assert!(!picker.open());
    assert_eq!(picker.calls().last().map(String::as_str), Some("hide"));
    // The host's dismissal after it changes nothing more.
    picker.cx.cx.update(dismissed);
    assert!(!picker.open());
}

#[test]
fn the_card_centres_on_its_arrow_inside_the_window() {
    // Room above and below: centred, the tip at the card's middle.
    let (left, top, arrow_y) = placed(point(250., 400.), 1000.);
    assert!((left - (250. - TIP_SHORT)).abs() < 0.01);
    assert_eq!((top, arrow_y), (400. - HEIGHT / 2., HEIGHT / 2.));
    // Near the top, the card stays in and the arrow moves up it, clear of the corner.
    let (_, top, arrow_y) = placed(point(250., 60.), 1000.);
    assert_eq!((top, arrow_y), (MARGIN, 52.));
    let (_, top, arrow_y) = placed(point(250., 10.), 1000.);
    assert_eq!((top, arrow_y), (MARGIN, RADIUS + ARROW + ARROW_FOOT_RADIUS));
    // Near the bottom, the same.
    let (_, top, arrow_y) = placed(point(250., 950.), 1000.);
    assert_eq!((top, arrow_y), (1000. - MARGIN - HEIGHT, 950. - top));
}

#[test]
fn presses_land_on_what_is_drawn_there() {
    let theme = Theme::at(0.5, 0.).with_count(2);
    let (x, y) = Picker::in_field(0.5, 0.);
    assert_eq!(target(x, y, Some(&theme)), Some(Target::Handle(0)));
    let (x, y) = Picker::in_field(-0.5, 0.);
    assert_eq!(target(x, y, Some(&theme)), Some(Target::Handle(1)));
    let (x, y) = Picker::in_field(0., 0.5);
    assert_eq!(target(x, y, Some(&theme)), Some(Target::Field));
    assert_eq!(
        target(APPEARANCE_X[1], APPEARANCE_Y, Some(&theme)),
        Some(Target::Appearance(Appearance::Light))
    );
    assert_eq!(target(SWATCH_X + SWATCH_PITCH * 8., SWATCH_Y, None), Some(Target::Swatch(8)));
    // Between two swatches, nothing.
    assert_eq!(target(SWATCH_X + SWATCH_PITCH / 2., SWATCH_Y, None), None);
    assert_eq!(target(TRACK_LEFT + 1., SLIDER_Y, None), Some(Target::Intensity));
    assert_eq!(target(DIAL.0, DIAL.1 + DIAL_RING, None), Some(Target::Grain));
}
