//! The sidebar's footer: the Space switcher and the plus button beside it.

use gpui::{Context, MouseButton, MouseDownEvent, div, prelude::*, px, rgba, svg};

use super::{
    FOOTER_HEIGHT, NAV, NAV_GLYPH, NAV_HOVERED, NAV_RADIUS, NAV_SIZE, PLUS_INSET, SPACE_DOT,
    SPACE_HEIGHT, SPACE_HOVERED, SPACE_ICON_SWITCHER, SPACE_OTHER, SPACE_PITCH, SPACE_SHOWN,
    SWITCHER_INSET, Shell,
};

/// How strong a Space's icon draws in the switcher, as its dot would: the Space shown at full
/// strength, the others dimmed by the dot's own ratio, and a hovered one by the dot's hovered
/// ratio.
pub(super) fn icon_opacity(shown: bool, hovered: bool) -> f32 {
    let alpha = |color: u32| (color & 0xff) as f32;
    match (shown, hovered) {
        (true, _) => 1.,
        (false, false) => alpha(SPACE_OTHER) / alpha(SPACE_SHOWN),
        (false, true) => alpha(SPACE_HOVERED) / alpha(SPACE_SHOWN),
    }
}

impl Shell {
    /// A dot for each Space, in the footer. Arc draws none while there's only one.
    pub(super) fn switcher(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.spaces.len() < 2 {
            return None;
        }
        let span = self.width() - 2. * SWITCHER_INSET;
        let pitch = SPACE_PITCH.min(span / self.spaces.len() as f32);
        let dots = self.spaces.iter().map(|space| {
            let (id, shown) = (space.id, space.id == self.space);
            div()
                .id(("space", id))
                .group("space")
                .w(px(pitch))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .block_mouse_except_scroll()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    log!("browser: command select-space via sidebar");
                    shell.show_space(id, cx)
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                        shell.space_menu(id, event.position, cx)
                    }),
                )
                .child(if space.icon.is_empty() {
                    div()
                        .size(px(SPACE_DOT))
                        .rounded_full()
                        .bg(rgba(if shown { SPACE_SHOWN } else { SPACE_OTHER }))
                        .when(!shown, |dot| {
                            dot.group_hover("space", |style| style.bg(rgba(SPACE_HOVERED)))
                        })
                } else {
                    // The item's box is the dot's, so the strip doesn't move with an icon.
                    div()
                        .text_size(px(SPACE_ICON_SWITCHER))
                        .opacity(icon_opacity(shown, false))
                        .when(!shown, |icon| {
                            icon.group_hover("space", |style| {
                                style.opacity(icon_opacity(false, true))
                            })
                        })
                        .child(space.icon.clone())
                })
        });
        Some(
            div()
                .absolute()
                .left(px(SWITCHER_INSET))
                .w(px(span))
                .bottom(px((FOOTER_HEIGHT - SPACE_HEIGHT) / 2.))
                .h(px(SPACE_HEIGHT))
                .flex()
                .justify_center()
                .children(dots),
        )
    }

    /// The footer's + at its right (Arc), whose menu makes a Space or a folder.
    pub(super) fn plus_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id("plus")
            .absolute()
            .right(px(PLUS_INSET))
            .bottom(px((FOOTER_HEIGHT - NAV_SIZE) / 2.))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(rgba(NAV_HOVERED)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseDownEvent, _, cx| {
                    shell.plus_menu(event.position, cx)
                }),
            )
            .child(svg().path("icons/nav-plus.svg").size(px(NAV_GLYPH)).text_color(rgba(NAV)))
    }
}

#[cfg(test)]
mod tests {
    use super::icon_opacity;
    use crate::shell::{SPACE_OTHER, SPACE_SHOWN};

    /// An icon is dimmed as the dot it replaces is: the Space shown at full strength, the others
    /// by the dots' ratio of 0.19 to 0.50, and a hovered one by the dots' 0.30 to 0.50.
    #[test]
    fn an_icon_dims_as_the_dot_does() {
        let alpha = |color: u32| (color & 0xff) as f32 / 255.;
        assert_eq!(icon_opacity(true, false), 1.);
        assert_eq!(icon_opacity(true, true), 1.);
        let others = icon_opacity(false, false);
        assert!((others - alpha(SPACE_OTHER) / alpha(SPACE_SHOWN)).abs() < 1e-6);
        assert!((others - 0.19 / 0.5).abs() < 0.02, "the others are at {others}");
        let hovered = icon_opacity(false, true);
        assert!((hovered - 0.30 / 0.5).abs() < 0.02, "a hovered one is at {hovered}");
        assert!(others < hovered && hovered < 1.);
    }
}
