//! The popover that makes a Space's theme, Arc's "Choose a Theme": beside the sidebar, its arrow
//! at the row or Space it was opened from, with the appearance buttons over a square field where
//! the colours are placed, − and + for how many, a row of swatches, and a wave slider for the
//! intensity and a dial for the grain (docs/ux-spec.md › Themes). The window's colour follows
//! each move live. Like the icon picker, it's a window of its own that the host shows over the
//! browser window, as its child; what it asks of that window goes through `Host`.

use std::{cell::OnceCell, f32::consts::TAU, rc::Rc, sync::Arc, time::Instant};

use gpui::{
    App, Bounds, Context, DispatchPhase, FocusHandle, Focusable, FontWeight, Global, Hsla,
    KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels,
    Point, RenderImage, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, actions, canvas, div, fill, point, prelude::*, px,
    quad, rgb, rgba, size, svg,
};
use image::{Frame, RgbaImage};

use crate::{
    icon_picker,
    shell::{KEY_CONTEXT, cubic_bezier},
    theme::{self, Appearance, DEFAULT_INTENSITY, MAX_COLORS, Theme},
};

actions!(theme_picker, [Dismiss]);

// Arc 1.170's, measured at 2× (docs/ux-spec.md › Themes), in points from the card's top left.
// The window holds the arrow left of the card too.
const CARD_WIDTH: f32 = 357.;
pub const HEIGHT: f32 = 509.;
/// The arrow's sides meet this far left of the card, at 45°, as the icon picker's do.
const ARROW: f32 = 13.;
const ARROW_TIP_RADIUS: f32 = 5.;
const ARROW_FOOT_RADIUS: f32 = 3.5;
/// How far inside where its sides meet the rounded tip is.
const TIP_SHORT: f32 = ARROW_TIP_RADIUS * (std::f32::consts::SQRT_2 - 1.);
pub const WIDTH: f32 = ARROW + CARD_WIDTH;
const RADIUS: f32 = 20.;
/// The arrow's tip is this far right of the row it points at.
const GAP: f32 = 2.5;
/// Kept off the browser window's top and bottom, when the card can't centre on its arrow (ours).
const MARGIN: f32 = 8.;
/// The square field, 8.5 inside the card's top and sides.
const FIELD_INSET: f32 = 8.5;
const FIELD: f32 = 340.;
const FIELD_RADIUS: f32 = 12.;
const FIELD_FILL: u32 = 0xffff_ff08;
const FIELD_RIM: u32 = 0xffff_ff33;
/// Its grid of 1 pt dots.
const DOT_PITCH: f32 = 4.3;
const DOT_ALPHA: u8 = 0x40;
const HINT: &str = "Tap to pick a color for this space";
/// The appearance buttons, across the field's top: automatic, light and dark.
const APPEARANCE_Y: f32 = 49.;
const APPEARANCE_X: [f32; 3] = [137.9, 178.3, 218.7];
const APPEARANCE_PLATE: f32 = 31.5;
const APPEARANCE_RADIUS: f32 = 6.;
const APPEARANCE_CHOSEN: u32 = 0xffff_ff14;
const APPEARANCE_GLYPH: f32 = 16.;
const APPEARANCE_COLOR: u32 = 0xffff_ffa6;
/// − and +, at the field's foot.
const COUNT_Y: f32 = 316.5;
const REMOVE_X: f32 = 158.5;
const ADD_X: f32 = 198.5;
const COUNT_GLYPH: f32 = 12.;
const COUNT_COLOR: u32 = 0xffff_ff59;
const COUNT_PRESSED: u32 = 0xffff_ffa6;
/// Ours: Arc's with one colour or three wasn't recorded.
const COUNT_DISABLED: u32 = 0xffff_ff1f;
const COUNT_TARGET: f32 = 24.;
/// The swatches: 24 pt discs on a 29.5 pitch, each with a ring a shade darker than its fill.
const SWATCH_Y: f32 = 380.;
const SWATCH_X: f32 = 60.5;
const SWATCH_PITCH: f32 = 29.5;
const SWATCH: f32 = 24.;
const SWATCH_RING: f32 = 1.5;
const SWATCH_RING_SHADE: f32 = 0.88;
pub const SWATCHES: [u32; 9] = [
    0xf0eae5, 0xe59eba, 0x9e749b, 0xe1676f, 0xef8c63, 0xf2d46c, 0x73e49b, 0x7eb6d5, 0x656786,
];
/// The intensity's wave slider: a pill with a sine over it, brighter left of the knob.
const SLIDER_Y: f32 = 452.;
const TRACK_LEFT: f32 = 18.5;
const TRACK_RIGHT: f32 = 242.5;
const TRACK_HEIGHT: f32 = 16.;
const TRACK_FILL: u32 = 0xffff_ff1a;
const WAVE_LEFT: f32 = 26.5;
const WAVE_RIGHT: f32 = 235.5;
const WAVE_PERIODS: f32 = 7.;
const WAVE_STROKE: f32 = 4.;
/// Half the wave's height at full intensity: 12.25 at 0.81.
const WAVE_AMPLITUDE: f32 = 15.;
const WAVE_ON: u32 = 0xffff_ff8c;
const WAVE_OFF: u32 = 0xffff_ff40;
const KNOB_WIDTH: f32 = 23.;
const KNOB_HEIGHT: f32 = 55.5;
/// The knob's middle at intensity 0 and at 1.
const KNOB_LEFT: f32 = 30.;
const KNOB_RIGHT: f32 = 231.;
/// The grain's dial: 24 dots round a disc, those the knob has swept bright.
const DIAL: (f32, f32) = (298.5, 452.);
const DIAL_DOTS: usize = 24;
const DIAL_RING: f32 = 32.;
const DIAL_DOT: f32 = 4.;
const DIAL_SWEPT: u32 = 0xffff_ff8a;
const DIAL_DIM: u32 = 0xffff_ff29;
const DIAL_DISC: f32 = 20.;
const DIAL_DISC_FILL: u32 = 0x0000_001a;
const DIAL_DISC_RIM: u32 = 0xffff_ff4d;
const DIAL_KNOB: (f32, f32) = (7., 15.);
/// How far from the dial's middle a press turns it (ours).
const DIAL_REACH: f32 = 44.;
/// The handles: the main colour's 44 pt with a 3 pt white ring, the others 26 pt.
const MAIN_HANDLE: f32 = 44.;
const MAIN_RING: f32 = 3.;
const HANDLE: f32 = 26.;
/// Ours: the small handles' ring isn't measured; the main one's, scaled.
const HANDLE_RING: f32 = 2.;
/// Pressed, a handle grows by this in 60 ms, and shrinks back as long on release.
const PRESSED: f32 = 48. / 44.;
const PRESS: f32 = 0.06;
/// With a third colour, the second glides from the main one's complement to 150° round in 0.2 s.
const GLIDE: f32 = 0.2;

/// What the picker asks of its window.
pub trait Host {
    /// Shows the picker's window with its top left at `left`, `top` in the browser window, with
    /// the keyboard, in the shape of `corners` (x, y and radius, clockwise from the arrow's tip,
    /// which it grows in from).
    fn show(&self, left: f32, top: f32, corners: &[(f32, f32, f32)]);
    /// Hides it, and the browser window gets the keyboard back.
    fn hide(&self);
}

/// The theme as the user makes it, and whether they're done: false while they drag, true once
/// they let go or click, for it to be kept.
pub type Changed = Rc<dyn Fn(Theme, bool, &mut App)>;

/// What a press in the card lands on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Appearance(Appearance),
    Remove,
    Add,
    Swatch(usize),
    Intensity,
    Grain,
    /// A colour's handle, the main one 0.
    Handle(usize),
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// A handle, and where in it the pointer took it, from its middle, in the field's units.
    Handle(usize, f32, f32),
    Intensity,
    Grain,
}

struct Editing {
    space: u64,
    theme: Option<Theme>,
    /// The appearance chosen before there's a colour, for the first one to take.
    appearance: Appearance,
    changed: Changed,
    /// The arrow's tip, down the window.
    arrow_y: f32,
    /// False once it's closing: what it shows stays, for the host's shrink out.
    open: bool,
    drag: Option<Drag>,
    /// A handle's growth: which, since when, and whether it's pressed or let go.
    press: Option<(usize, Instant, bool)>,
    /// The second handle's glide between offsets: from, to and since when.
    glide: Option<(f32, f32, Instant)>,
    /// − or + while it's pressed.
    pressed: Option<Target>,
    /// Whether a frame of it has been drawn on screen since it opened.
    drawn: bool,
}

pub struct ThemePicker {
    host: Rc<dyn Host>,
    focus: FocusHandle,
    editing: Option<Editing>,
}

struct ThemePickerWindow(WindowHandle<ThemePicker>);

impl Global for ThemePickerWindow {}

/// Opens the picker's window, hidden, for `host` to show over the browser window, ahead of time as
/// the icon picker's is.
pub fn open(host: Rc<dyn Host>, cx: &mut App) -> Option<WindowHandle<ThemePicker>> {
    cx.bind_keys([KeyBinding::new("escape", Dismiss, Some("ThemePicker"))]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(HEIGHT)),
        ))),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Floating,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        // The card's shape is drawn, so the window around it is clear.
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let window = cx.open_window(options, |_, cx| {
        cx.new(|cx| ThemePicker { host, focus: cx.focus_handle(), editing: None })
    });
    match window {
        Ok(window) => {
            cx.set_global(ThemePickerWindow(window));
            Some(window)
        }
        Err(error) => {
            log!("couldn't open the theme picker's window: {error}");
            None
        }
    }
}

/// The card's corners with the arrow's tip `arrow_y` down the window, from the window's top left,
/// clockwise from the tip, each with the radius it's rounded by.
pub fn corners(arrow_y: f32) -> [(f32, f32, f32); 7] {
    [
        (0., arrow_y, ARROW_TIP_RADIUS),
        (ARROW, arrow_y - ARROW, ARROW_FOOT_RADIUS),
        (ARROW, 0., RADIUS),
        (WIDTH, 0., RADIUS),
        (WIDTH, HEIGHT, RADIUS),
        (ARROW, HEIGHT, RADIUS),
        (ARROW, arrow_y + ARROW, ARROW_FOOT_RADIUS),
    ]
}

/// Where the window goes for its arrow to point at `anchor`, the middle of a row's right end, in a
/// browser window `height` tall: its top left, and the arrow's tip down it. The card centres on
/// the arrow, as Arc's does, unless that takes it past the browser window's top or bottom.
pub fn placed(anchor: Point<f32>, height: f32) -> (f32, f32, f32) {
    let top = (anchor.y - HEIGHT / 2.).min(height - MARGIN - HEIGHT).max(MARGIN);
    // The tip keeps clear of the card's rounded corners.
    let reach = RADIUS + ARROW + ARROW_FOOT_RADIUS;
    let arrow_y = (anchor.y - top).clamp(reach, HEIGHT - reach);
    (anchor.x + GAP - TIP_SHORT, top, arrow_y)
}

/// Shows the picker for `space`, which has `theme` or none yet, pointing at `anchor`, the middle
/// of a row's right end, in a browser window `height` tall. `changed` gets the theme as the user
/// makes it.
pub fn show(
    space: u64,
    theme: Option<Theme>,
    anchor: Point<f32>,
    height: f32,
    changed: Changed,
    cx: &mut App,
) {
    let Some(window) = cx.try_global::<ThemePickerWindow>().map(|picker| picker.0) else {
        return;
    };
    window
        .update(cx, |picker, window, cx| {
            // check.sh reads this line.
            log!("browser: theme picker for {space}");
            let (left, top, arrow_y) = placed(anchor, height);
            picker.editing = Some(Editing {
                space,
                theme,
                appearance: theme.map_or(Appearance::default(), |theme| theme.appearance),
                changed,
                arrow_y,
                open: true,
                drag: None,
                press: None,
                glide: None,
                pressed: None,
                drawn: false,
            });
            window.focus(&picker.focus, cx);
            picker.host.show(left, top, &corners(arrow_y));
            cx.notify();
        })
        .ok();
}

/// The user left the picker, such as with a click outside it.
pub fn dismissed(cx: &mut App) {
    if let Some(window) = cx.try_global::<ThemePickerWindow>().map(|picker| picker.0) {
        window.update(cx, |picker, _, cx| picker.close(cx)).ok();
    }
}

/// Whether the picker shows, for a click on what opened it to close it rather than open it again.
pub fn is_open(cx: &App) -> bool {
    cx.try_global::<ThemePickerWindow>()
        .and_then(|picker| picker.0.read(cx).ok())
        .is_some_and(|picker| picker.editing.as_ref().is_some_and(|editing| editing.open))
}

/// What a press at `x`, `y` from the card's top left lands on, with `theme` in the field.
pub fn target(x: f32, y: f32, theme: Option<&Theme>) -> Option<Target> {
    let near = |cx: f32, cy: f32, reach: f32| (x - cx).abs() <= reach && (y - cy).abs() <= reach;
    if let Some(index) = APPEARANCE_X.iter().position(|&ax| near(ax, APPEARANCE_Y, 15.75)) {
        return Some(Target::Appearance(Appearance::ALL[index]));
    }
    if near(REMOVE_X, COUNT_Y, COUNT_TARGET / 2.) {
        return Some(Target::Remove);
    }
    if near(ADD_X, COUNT_Y, COUNT_TARGET / 2.) {
        return Some(Target::Add);
    }
    let in_field = (FIELD_INSET..=FIELD_INSET + FIELD).contains(&x)
        && (FIELD_INSET..=FIELD_INSET + FIELD).contains(&y);
    if in_field {
        // The main handle lies over the others.
        if let Some(theme) = theme {
            let (fx, fy) = field_point(x, y);
            let mut order: Vec<usize> = (0..theme.count).collect();
            order.rotate_left(1);
            for index in order.into_iter().rev() {
                let (hx, hy) = theme.handle(index);
                let reach = handle_size(index) / 2. / (FIELD / 2.);
                if (fx - hx).hypot(fy - hy) <= reach {
                    return Some(Target::Handle(index));
                }
            }
        }
        return Some(Target::Field);
    }
    let swatch = ((x - SWATCH_X) / SWATCH_PITCH).round();
    if (0. ..SWATCHES.len() as f32).contains(&swatch)
        && (x - SWATCH_X - swatch * SWATCH_PITCH).hypot(y - SWATCH_Y) <= SWATCH / 2.
    {
        return Some(Target::Swatch(swatch as usize));
    }
    if (TRACK_LEFT..=TRACK_RIGHT).contains(&x) && (y - SLIDER_Y).abs() <= KNOB_HEIGHT / 2. {
        return Some(Target::Intensity);
    }
    if (x - DIAL.0).hypot(y - DIAL.1) <= DIAL_REACH {
        return Some(Target::Grain);
    }
    None
}

/// The field's point under `x`, `y` in the card, from its middle in half its width.
fn field_point(x: f32, y: f32) -> (f32, f32) {
    let middle = FIELD_INSET + FIELD / 2.;
    ((x - middle) / (FIELD / 2.), (y - middle) / (FIELD / 2.))
}

fn handle_size(index: usize) -> f32 {
    if index == 0 { MAIN_HANDLE } else { HANDLE }
}

/// The intensity knob's middle across the card.
fn knob_x(intensity: f32) -> f32 {
    KNOB_LEFT + (KNOB_RIGHT - KNOB_LEFT) * intensity
}

/// The intensity for the knob's middle at `x` in the card.
fn intensity_at(x: f32) -> f32 {
    ((x - KNOB_LEFT) / (KNOB_RIGHT - KNOB_LEFT)).clamp(0., 1.)
}

/// The grain for the pointer at `x`, `y` in the card: how far round the dial from its top,
/// clockwise. A turn past the top stops there rather than going round, from `before`.
fn grain_at(x: f32, y: f32, before: f32) -> f32 {
    let turn = (x - DIAL.0).atan2(DIAL.1 - y).rem_euclid(TAU) / TAU;
    match (before, turn) {
        (before, turn) if before > 0.75 && turn < 0.25 => 1.,
        (before, turn) if before < 0.25 && turn > 0.75 => 0.,
        _ => turn,
    }
}

impl ThemePicker {
    fn close(&mut self, cx: &mut Context<Self>) {
        if let Some(editing) = self.editing.as_mut().filter(|editing| editing.open) {
            editing.open = false;
            editing.drag = None;
            cx.notify();
        }
    }

    fn dismiss(&mut self, _: &Dismiss, _: &mut Window, cx: &mut Context<Self>) {
        if self.editing.as_ref().is_some_and(|editing| editing.open) {
            // check.sh reads this line.
            log!("browser: theme picker dismissed");
            self.close(cx);
            self.host.hide();
        }
    }

    /// Hands the theme on: kept, once `done`.
    fn changed(&mut self, done: bool, cx: &mut Context<Self>) {
        let Some(editing) = &self.editing else {
            return;
        };
        if let Some(theme) = editing.theme {
            if done {
                // check.sh reads this line.
                log!("browser: theme picker set {} {}", editing.space, theme.to_text());
            }
            (editing.changed.clone())(theme, done, cx);
        }
        cx.notify();
    }

    fn press(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let (x, y) = (f32::from(event.position.x) - ARROW, f32::from(event.position.y));
        let Some(editing) = self.editing.as_mut().filter(|editing| editing.open) else {
            return;
        };
        let Some(target) = target(x, y, editing.theme.as_ref()) else {
            return;
        };
        let now = Instant::now();
        match (target, editing.theme) {
            (Target::Appearance(appearance), theme) => {
                editing.appearance = appearance;
                editing.theme = theme.map(|theme| Theme { appearance, ..theme });
                self.changed(true, cx);
            }
            (Target::Remove, Some(theme)) if theme.count > 1 => {
                if theme.count == 3 {
                    editing.glide = Some((150., 180., now));
                }
                editing.theme = Some(theme.with_count(theme.count - 1));
                editing.pressed = Some(target);
                self.changed(true, cx);
            }
            (Target::Add, Some(theme)) if theme.count < MAX_COLORS => {
                if theme.count == 2 {
                    editing.glide = Some((180., 150., now));
                }
                editing.theme = Some(theme.with_count(theme.count + 1));
                editing.pressed = Some(target);
                self.changed(true, cx);
            }
            (Target::Swatch(index), theme) => {
                let picked = Theme::of_color(SWATCHES[index]);
                editing.theme = Some(match theme {
                    Some(theme) => {
                        Theme { x: picked.x, y: picked.y, saturation: picked.saturation, ..theme }
                    }
                    None => Theme { appearance: editing.appearance, ..picked },
                });
                self.changed(true, cx);
            }
            (Target::Intensity, Some(theme)) => {
                editing.theme = Some(Theme { intensity: intensity_at(x), ..theme });
                editing.drag = Some(Drag::Intensity);
                self.changed(false, cx);
            }
            (Target::Grain, Some(theme)) => {
                editing.theme = Some(Theme { grain: grain_at(x, y, theme.grain), ..theme });
                editing.drag = Some(Drag::Grain);
                self.changed(false, cx);
            }
            (Target::Handle(index), Some(theme)) => {
                let (fx, fy) = field_point(x, y);
                let (hx, hy) = theme.handle(index);
                editing.drag = Some(Drag::Handle(index, fx - hx, fy - hy));
                editing.press = Some((index, now, true));
                cx.notify();
            }
            // The first colour goes where the field is clicked, and follows the pointer from there.
            (Target::Field, None) => {
                let (fx, fy) = field_point(x, y);
                let appearance = editing.appearance;
                editing.theme = Some(Theme { appearance, ..Theme::at(fx, fy) });
                editing.drag = Some(Drag::Handle(0, 0., 0.));
                editing.press = Some((0, now, true));
                self.changed(false, cx);
            }
            _ => {}
        }
    }

    fn drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let (x, y) = (f32::from(position.x) - ARROW, f32::from(position.y));
        let Some(editing) = self.editing.as_mut() else {
            return;
        };
        let (Some(drag), Some(theme)) = (editing.drag, editing.theme) else {
            return;
        };
        let moved = match drag {
            Drag::Handle(index, grab_x, grab_y) => {
                let (fx, fy) = field_point(x, y);
                theme.moved(index, fx - grab_x, fy - grab_y)
            }
            Drag::Intensity => Theme { intensity: intensity_at(x), ..theme },
            Drag::Grain => Theme { grain: grain_at(x, y, theme.grain), ..theme },
        };
        if moved != theme {
            editing.theme = Some(moved);
            self.changed(false, cx);
        }
    }

    fn release(&mut self, cx: &mut Context<Self>) {
        let Some(editing) = self.editing.as_mut() else {
            return;
        };
        let pressed = editing.pressed.take().is_some();
        let Some(drag) = editing.drag.take() else {
            if pressed {
                cx.notify();
            }
            return;
        };
        if let Drag::Handle(index, ..) = drag {
            editing.press = Some((index, Instant::now(), false));
        }
        self.changed(true, cx);
    }

    /// The Space it's making a theme for, and the theme as it is, while it shows.
    pub fn editing(&self) -> Option<(u64, Option<Theme>)> {
        let editing = self.editing.as_ref().filter(|editing| editing.open)?;
        Some((editing.space, editing.theme))
    }
}

impl Editing {
    /// How far round from the main colour handle `index` shows, as the second glides.
    fn offset(&self, index: usize, theme: &Theme) -> f32 {
        let resting = Theme::offsets(theme.count).get(index).copied().unwrap_or(0.);
        match self.glide {
            Some((from, to, start)) if index == 1 => {
                let t = (start.elapsed().as_secs_f32() / GLIDE).min(1.);
                from + (to - from) * cubic_bezier(0.25, 0.1, 0.25, 1., t)
            }
            _ => resting,
        }
    }

    /// How much bigger than at rest handle `index` is, as it's pressed or let go.
    fn growth(&self, index: usize) -> f32 {
        match self.press {
            Some((pressed, start, down)) if pressed == index => {
                let t = (start.elapsed().as_secs_f32() / PRESS).min(1.);
                let t = if down { t } else { 1. - t };
                1. + (PRESSED - 1.) * t
            }
            _ => 1.,
        }
    }

    /// Whether a handle's growth or the second's glide is still under way.
    fn moving(&self) -> bool {
        let press = self.press.is_some_and(|(.., start, _)| start.elapsed().as_secs_f32() < PRESS);
        let glide = self.glide.is_some_and(|(.., start)| start.elapsed().as_secs_f32() < GLIDE);
        press || glide
    }
}

thread_local! {
    static DOTS: OnceCell<Arc<RenderImage>> = const { OnceCell::new() };
}

/// The field's dot grid, drawn once at 2×.
fn dots_image() -> Arc<RenderImage> {
    DOTS.with(|dots| {
        dots.get_or_init(|| {
            let side = (FIELD * 2.) as usize;
            let mut data = vec![0u8; side * side * 4];
            let middle = FIELD / 2.;
            let count = (middle / DOT_PITCH) as i32;
            for row in -count..=count {
                for column in -count..=count {
                    let x = ((middle + column as f32 * DOT_PITCH) * 2.).round() as usize - 1;
                    let y = ((middle + row as f32 * DOT_PITCH) * 2.).round() as usize - 1;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let at = ((y + dy) * side + x + dx) * 4;
                        data[at..at + 4].copy_from_slice(&[255, 255, 255, DOT_ALPHA]);
                    }
                }
            }
            let image =
                RgbaImage::from_raw(side as u32, side as u32, data).expect("sized to fit");
            Arc::new(RenderImage::new(vec![Frame::new(image)]))
        })
        .clone()
    })
}

/// A white capsule `width` by `length` with its middle at `at`, its length along `angle` (radians
/// clockwise from up), into a path to fill.
fn capsule(at: Point<Pixels>, width: f32, length: f32, angle: f32) -> PathBuilder {
    let (sin, cos) = angle.sin_cos();
    let along = (sin, -cos);
    let radius = width / 2.;
    let reach = length / 2. - radius;
    let mut path = PathBuilder::fill();
    // Each end's half circle in eight steps, from one side round to the other.
    let ends = [(reach, 0.), (-reach, std::f32::consts::PI)];
    let mut first = true;
    for (offset, start) in ends {
        let (cx, cy) = (along.0 * offset, along.1 * offset);
        for step in 0..=8 {
            let theta = angle + start - std::f32::consts::FRAC_PI_2
                + std::f32::consts::PI * step as f32 / 8.;
            let (s, c) = theta.sin_cos();
            let point = at + point(px(cx + radius * s), px(cy - radius * c));
            if first {
                path.move_to(point);
                first = false;
            } else {
                path.line_to(point);
            }
        }
    }
    path.close();
    path
}

impl Focusable for ThemePicker {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ThemePicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().key_context(KEY_CONTEXT).size_full().font_family(".SystemUIFont");
        // The pointer's moves anywhere in the window, while something is dragged, and its release
        // always: a press and its release can both come before the next frame, which would
        // otherwise be drawn without a listener for the release.
        let dragging = self
            .editing
            .as_ref()
            .is_some_and(|editing| editing.drag.is_some() || editing.pressed.is_some());
        let entity = cx.weak_entity();
        let listener = canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                if dragging {
                    let moved = entity.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                        if phase == DispatchPhase::Capture {
                            moved.update(cx, |picker, cx| picker.drag(event.position, cx)).ok();
                        }
                    });
                }
                let released = entity.clone();
                window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
                        released.update(cx, |picker, cx| picker.release(cx)).ok();
                    }
                });
            },
        )
        .absolute()
        .size_0();
        // GPUI hands a press to what the last frame drew, which before an opening's first frame is
        // the empty one drawn as the window was made. So every frame takes the keys, the press and
        // the release, the press read against the picker as it is now, and a click as the picker
        // appears isn't lost.
        let picker = div()
            .key_context("ThemePicker")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::dismiss))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|picker, event: &MouseDownEvent, _, cx| picker.press(event, cx)),
            )
            .relative()
            .size_full()
            .child(listener);
        let Some(editing) = &mut self.editing else {
            return root.child(picker);
        };
        if editing.moving() {
            window.request_animation_frame();
        }
        // AppKit puts the window on screen some frames after it's asked to, and only then does
        // GPUI draw it.
        if editing.open && !editing.drawn {
            if window.is_visible() {
                editing.drawn = true;
                // check.sh waits for this line.
                log!("browser: theme picker drawn");
            } else {
                window.request_animation_frame();
            }
        }
        let editing = &*editing;
        let theme = editing.theme;
        let card_x = |x: f32| px(ARROW + x);
        let arrow_y = editing.arrow_y;
        let card = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                icon_picker::paint_card(bounds.origin, &corners(arrow_y), window)
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        // The field, its dots, and the handles over it.
        let dots = canvas(
            |_, _, _| {},
            |bounds, _, window, _| {
                window
                    .paint_image(bounds, bounds, px(FIELD_RADIUS).into(), dots_image(), 0, false)
                    .ok();
            },
        )
        .absolute()
        .size_full();
        let mut field = div()
            .absolute()
            .left(card_x(FIELD_INSET))
            .top(px(FIELD_INSET))
            .size(px(FIELD))
            .rounded(px(FIELD_RADIUS))
            .bg(rgba(FIELD_FILL))
            .child(dots)
            .child(
                div()
                    .absolute()
                    .size_full()
                    .rounded(px(FIELD_RADIUS))
                    .border_1()
                    .border_color(rgba(FIELD_RIM)),
            );
        match theme {
            None => {
                field = field.child(
                    div()
                        .absolute()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0xffffff))
                        .child(HINT),
                );
            }
            Some(theme) => {
                // The main handle last, over the others.
                for index in (1..theme.count).chain([0]) {
                    let offset = editing.offset(index, &theme);
                    let (hx, hy) = theme.handle(0);
                    let (sin, cos) = offset.to_radians().sin_cos();
                    let (hx, hy) = (hx * cos - hy * sin, hx * sin + hy * cos);
                    let color = theme::color_at(hx, hy, theme.saturation);
                    let size = handle_size(index) * editing.growth(index);
                    let ring = if index == 0 { MAIN_RING } else { HANDLE_RING };
                    let (mx, my) = (FIELD / 2. * (1. + hx), FIELD / 2. * (1. + hy));
                    field = field.child(
                        div()
                            .absolute()
                            .left(px(mx - size / 2.))
                            .top(px(my - size / 2.))
                            .size(px(size))
                            .rounded_full()
                            .bg(rgb(color))
                            .border(px(ring))
                            .border_color(rgb(0xffffff)),
                    );
                }
            }
        }

        let chosen = theme.map_or(editing.appearance, |theme| theme.appearance);
        let appearances = Appearance::ALL.iter().zip(APPEARANCE_X).map(|(&appearance, x)| {
            let glyph = match appearance {
                Appearance::Automatic => "icons/sparkles.svg",
                Appearance::Light => "icons/sun.svg",
                Appearance::Dark => "icons/moon-star.svg",
            };
            div()
                .absolute()
                .left(card_x(x - APPEARANCE_PLATE / 2.))
                .top(px(APPEARANCE_Y - APPEARANCE_PLATE / 2.))
                .size(px(APPEARANCE_PLATE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(APPEARANCE_RADIUS))
                .when(appearance == chosen, |plate| plate.bg(rgba(APPEARANCE_CHOSEN)))
                .child(
                    svg()
                        .path(glyph)
                        .size(px(APPEARANCE_GLYPH))
                        .text_color(rgba(APPEARANCE_COLOR)),
                )
        });

        let count = theme.map_or(0, |theme| theme.count);
        let count_button = |target: Target, x: f32, glyph: &'static str, enabled: bool| {
            let color = match (enabled, editing.pressed == Some(target)) {
                (false, _) => COUNT_DISABLED,
                (true, true) => COUNT_PRESSED,
                (true, false) => COUNT_COLOR,
            };
            div()
                .absolute()
                .left(card_x(x - COUNT_TARGET / 2.))
                .top(px(COUNT_Y - COUNT_TARGET / 2.))
                .size(px(COUNT_TARGET))
                .flex()
                .items_center()
                .justify_center()
                .child(svg().path(glyph).size(px(COUNT_GLYPH)).text_color(rgba(color)))
        };
        let remove = count_button(Target::Remove, REMOVE_X, "icons/minus.svg", count > 1);
        let add = count_button(
            Target::Add,
            ADD_X,
            "icons/plus.svg",
            (1..MAX_COLORS).contains(&count),
        );

        let swatches = SWATCHES.iter().enumerate().map(|(index, &color)| {
            let ring = Hsla::from(rgb(color));
            let ring = Hsla { l: ring.l * SWATCH_RING_SHADE, ..ring };
            div()
                .absolute()
                .left(card_x(SWATCH_X + SWATCH_PITCH * index as f32 - SWATCH / 2.))
                .top(px(SWATCH_Y - SWATCH / 2.))
                .size(px(SWATCH))
                .rounded_full()
                .bg(rgb(color))
                .border(px(SWATCH_RING))
                .border_color(ring)
        });

        // The wave slider and the dial, which show without knobs, dimmer, until there's a colour.
        let controls = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let at = |x: f32, y: f32| bounds.origin + point(px(ARROW + x), px(y));
                let track = Bounds::new(
                    at(TRACK_LEFT, SLIDER_Y - TRACK_HEIGHT / 2.),
                    size(px(TRACK_RIGHT - TRACK_LEFT), px(TRACK_HEIGHT)),
                );
                let track = fill(track, rgba(TRACK_FILL)).corner_radii(px(TRACK_HEIGHT / 2.));
                window.paint_quad(track);
                let intensity = theme.map_or(DEFAULT_INTENSITY, |theme| theme.intensity);
                let split = if theme.is_some() { knob_x(intensity) } else { WAVE_LEFT };
                let amplitude = WAVE_AMPLITUDE * intensity;
                let wave_y = |x: f32| {
                    let phase = (x - WAVE_LEFT) / (WAVE_RIGHT - WAVE_LEFT) * WAVE_PERIODS * TAU;
                    SLIDER_Y - amplitude * phase.sin()
                };
                for (from, to, color) in
                    [(WAVE_LEFT, split, WAVE_ON), (split, WAVE_RIGHT, WAVE_OFF)]
                {
                    if to - from < 0.5 {
                        continue;
                    }
                    let mut wave = PathBuilder::stroke(px(WAVE_STROKE));
                    let steps = ((to - from) / 1.5).ceil() as usize;
                    for step in 0..=steps {
                        let x = from + (to - from) * step as f32 / steps as f32;
                        if step == 0 {
                            wave.move_to(at(x, wave_y(x)));
                        } else {
                            wave.line_to(at(x, wave_y(x)));
                        }
                    }
                    if let Ok(path) = wave.build() {
                        window.paint_path(path, rgba(color));
                    }
                }
                let grain = theme.map(|theme| theme.grain);
                let disc = Bounds::new(
                    at(DIAL.0 - DIAL_DISC, DIAL.1 - DIAL_DISC),
                    size(px(DIAL_DISC * 2.), px(DIAL_DISC * 2.)),
                );
                window.paint_quad(quad(
                    disc,
                    px(DIAL_DISC),
                    rgba(if grain.is_some() { DIAL_DISC_FILL } else { 0 }),
                    px(1.),
                    rgba(DIAL_DISC_RIM),
                    Default::default(),
                ));
                for dot in 0..DIAL_DOTS {
                    let turn = dot as f32 / DIAL_DOTS as f32;
                    let (sin, cos) = (turn * TAU).sin_cos();
                    let (x, y) = (DIAL.0 + DIAL_RING * sin, DIAL.1 - DIAL_RING * cos);
                    let swept = grain.is_some_and(|grain| turn <= grain + 1e-4);
                    let bounds = Bounds::new(
                        at(x - DIAL_DOT / 2., y - DIAL_DOT / 2.),
                        size(px(DIAL_DOT), px(DIAL_DOT)),
                    );
                    let color = if swept { DIAL_SWEPT } else { DIAL_DIM };
                    window.paint_quad(fill(bounds, rgba(color)).corner_radii(px(DIAL_DOT / 2.)));
                }
                if let Some(grain) = grain {
                    let (sin, cos) = (grain * TAU).sin_cos();
                    let middle = at(DIAL.0 + DIAL_RING * sin, DIAL.1 - DIAL_RING * cos);
                    let knob = capsule(middle, DIAL_KNOB.0, DIAL_KNOB.1, grain * TAU);
                    if let Ok(path) = knob.build() {
                        window.paint_path(path, rgb(0xffffff));
                    }
                    let knob = Bounds::new(
                        at(knob_x(intensity) - KNOB_WIDTH / 2., SLIDER_Y - KNOB_HEIGHT / 2.),
                        size(px(KNOB_WIDTH), px(KNOB_HEIGHT)),
                    );
                    window.paint_quad(
                        fill(knob, rgb(0xffffff)).corner_radii(px(KNOB_WIDTH / 2.)),
                    );
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        // The grain in the dial's disc, as it lies over the window.
        let dial_grain = theme.filter(|theme| theme.grain > 0.).map(|theme| {
            div()
                .absolute()
                .left(card_x(DIAL.0 - DIAL_DISC))
                .top(px(DIAL.1 - DIAL_DISC))
                .size(px(DIAL_DISC * 2.))
                .opacity(theme.grain * theme::GRAIN_STRENGTH)
                .child(
                    canvas(
                        |_, _, _| {},
                        |bounds, _, window, _| {
                            window
                                .paint_image(
                                    bounds,
                                    bounds,
                                    px(DIAL_DISC).into(),
                                    theme::grain_image(),
                                    0,
                                    false,
                                )
                                .ok();
                        },
                    )
                    .size_full(),
                )
        });

        root.child(
            picker
                .child(card)
                .child(field)
                .children(appearances)
                .child(remove)
                .child(add)
                .children(swatches)
                .child(controls)
                .children(dial_grain),
        )
    }
}

#[cfg(test)]
mod tests;
