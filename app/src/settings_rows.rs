//! The Settings window's boxes of rows and their controls, which the views it shows from the
//! library, such as the import, draw with too.

use gpui::{Rgba, SharedString, div, prelude::*, px, rgb, svg};

use crate::tokens::settings::{
    BOX, BOX_LINE, CONTROL, CONTROL_HEIGHT, CONTROL_PADDING, CONTROL_PRESSED, CONTROL_RADIUS, KNOB,
    KNOB_HEIGHT, KNOB_INSET, KNOB_ON, KNOB_WIDTH, LINE_INSET, PADDING, RADIUS, SWITCH_HEIGHT,
    SWITCH_OFF, SWITCH_WIDTH, TILE, TILE_GAP, TILE_GLYPH, TILE_GLYPH_COLOR, TILE_RADIUS,
};

/// A switch is 36 by 16, with a knob 21 by 13, 1.5 in, that slides across over 150 ms (ours).
pub const SLIDE: f32 = 0.15;

/// A box of `rows`, with lines between them.
pub fn group(rows: impl IntoIterator<Item = gpui::Div>) -> gpui::Div {
    let rows = rows.into_iter().enumerate().map(|(index, row)| {
        div()
            .when(index > 0, |below| {
                below.child(div().mx(px(LINE_INSET)).h(px(1.)).bg(rgb(BOX_LINE)))
            })
            .child(row)
    });
    div()
        .flex()
        .flex_col()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(rgb(BOX_LINE))
        .bg(rgb(BOX))
        .children(rows)
}

/// A row: its icon if any, its name, and its control at the right.
pub fn row(
    icon: Option<(&'static str, u32)>,
    name: impl Into<SharedString>,
    control: gpui::AnyElement,
) -> gpui::Div {
    div()
        .px(px(PADDING))
        .py(px(PADDING))
        .flex()
        .items_center()
        .children(icon.map(|(glyph, color)| tile(glyph, color)))
        .child(div().flex_1().min_w_0().truncate().child(name.into()))
        .child(div().flex_none().ml(px(PADDING)).child(control))
}

/// A row's icon: a white glyph on a tile of colour.
pub fn tile(glyph: &'static str, color: u32) -> gpui::Div {
    div()
        .flex_none()
        .size(px(TILE))
        .mr(px(TILE_GAP))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(TILE_RADIUS))
        .bg(rgb(color))
        .child(
            svg()
                .path(glyph)
                .size(px(TILE_GLYPH))
                .text_color(rgb(TILE_GLYPH_COLOR)),
        )
}

/// A push button, darker while pressed.
pub fn button(
    id: impl Into<gpui::ElementId>,
    title: impl Into<SharedString>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .h(px(CONTROL_HEIGHT))
        .px(px(CONTROL_PADDING))
        .flex()
        .items_center()
        .rounded(px(CONTROL_RADIUS))
        .bg(rgb(CONTROL))
        .active(|button| button.bg(rgb(CONTROL_PRESSED)))
        .child(title.into())
}

/// A switch, on or off, with its knob `slid` of the way to that side.
pub fn switch(on: bool, slid: f32, accent: Rgba) -> impl IntoElement {
    let travel = SWITCH_WIDTH - KNOB_WIDTH - 2. * KNOB_INSET;
    let way = if on { slid } else { 1. - slid };
    div()
        .relative()
        .w(px(SWITCH_WIDTH))
        .h(px(SWITCH_HEIGHT))
        .rounded_full()
        .bg(if on { accent } else { rgb(SWITCH_OFF) })
        .child(
            div()
                .absolute()
                .top(px((SWITCH_HEIGHT - KNOB_HEIGHT) / 2.))
                .left(px(KNOB_INSET + travel * way))
                .w(px(KNOB_WIDTH))
                .h(px(KNOB_HEIGHT))
                .rounded_full()
                .bg(rgb(if on { KNOB_ON } else { KNOB })),
        )
}
