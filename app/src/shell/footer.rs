//! The sidebar's footer: the Space switcher and the plus button beside it, and over them the row
//! that restarts into a downloaded update.

use std::cell::Cell;

use gpui::{Context, MouseButton, MouseDownEvent, Role, canvas, div, prelude::*, px, svg};

use super::{
    FAVICON_DRAWN, FOOTER_HEIGHT, HOVERED, NAV, NAV_GLYPH, NAV_HOVERED, NAV_RADIUS, NAV_SIZE,
    NEW_TAB, NEW_TAB_PLUS, PLATE_HEIGHT, PLATE_INSET, PLUS_INSET, ROW_PITCH, SPACE_GLYPH,
    SPACE_HEIGHT, SPACE_HOVERED, SPACE_ICON_SWITCHER, SPACE_OTHER, SPACE_PITCH,
    SPACE_SHOWN, SPACE_SWATCH, SPACE_SWATCH_HOVERED, SPACE_SWATCH_SHOWN, SWITCHER_INSET, Shell,
    access::Accessible, plate,
};
use crate::tokens::text;
use crate::space_icon::{GLYPHS, SpaceIcon};

/// How strong a Space's icon draws in the switcher, as its dot would: the Space shown at full
/// strength, the others dimmed by the dot's own ratio, and a hovered one by the dot's hovered
/// ratio.
pub(super) fn icon_opacity(shown: bool, hovered: bool) -> f32 {
    match (shown, hovered) {
        (true, _) => 1.,
        (false, false) => SPACE_OTHER / SPACE_SHOWN,
        (false, true) => SPACE_HOVERED / SPACE_SHOWN,
    }
}

thread_local! {
    /// Where the update's row was last drawn, in the window's points, as its centre.
    static UPDATE_ROW_AT: Cell<Option<(i32, i32)>> = const { Cell::new(None) };
}

/// Logs where the update's row draws, once for each place, for check.sh to click it.
fn drawn_at() -> impl IntoElement {
    canvas(
        |_, _, _| {},
        |bounds, _, _, _| {
            let at = (f32::from(bounds.center().x) as i32, f32::from(bounds.center().y) as i32);
            if UPDATE_ROW_AT.replace(Some(at)) != Some(at) {
                // check.sh waits for this line.
                log!("browser: update row at {} {}", at.0, at.1);
            }
        },
    )
    .absolute()
    .inset_0()
}

impl Shell {
    /// How far the tab list stops above the window's bottom: the footer, and the update's row over
    /// it while there is one.
    pub(super) fn foot(&self) -> f32 {
        FOOTER_HEIGHT + if self.update_shown() { ROW_PITCH } else { 0. }
    }

    /// A private window shows no update, as it shows no Library or plus button.
    fn update_shown(&self) -> bool {
        !self.private && self.browser.update_ready()
    }

    /// "Relaunch to update", a row over the footer once an update has downloaded. It stays until
    /// the update installs, at the click, at quit or while the Mac is idle.
    pub(super) fn update_row(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if !self.update_shown() {
            return None;
        }
        let icon = svg().path("icons/rotate-cw.svg").size(px(FAVICON_DRAWN));
        let label = div().text_size(px(text::BODY)).text_color(self.palette.text(NEW_TAB));
        let label = label.child("Relaunch to update");
        Some(
            div()
                .id("update")
                .when(self.a11y, |row| row.button_named("Relaunch to update"))
                .absolute()
                .left(px(PLATE_INSET))
                .w(px(self.width() - 2. * PLATE_INSET))
                .bottom(px(FOOTER_HEIGHT + (ROW_PITCH - PLATE_HEIGHT) / 2.))
                .h(px(PLATE_HEIGHT))
                .block_mouse_except_scroll()
                .child(
                    plate(icon.text_color(self.palette.glyph(NEW_TAB_PLUS)), label)
                        .hover(|style| style.bg(self.palette.plate(HOVERED))),
                )
                .child(drawn_at())
                .on_click(cx.listener(|shell, _, _, cx| {
                    log!("browser: update relaunch via sidebar");
                    shell.browser.install_update(cx);
                })),
        )
    }

    /// A dot for each Space, in the footer. Arc draws none while there's only one.
    pub(super) fn switcher(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.spaces.len() < 2 {
            return None;
        }
        let span = self.width() - 2. * SWITCHER_INSET;
        let pitch = SPACE_PITCH.min(span / self.spaces.len() as f32);
        // What the swatches lie on: the window at the footer, over whatever is behind it.
        let window = f32::from(self.viewport.height).max(1.);
        let at = self.gradient_color((window - FOOTER_HEIGHT / 2.) / window);
        let behind = self.palette.over_backdrops(at);
        let dots = self.spaces.iter().map(|space| {
            let (id, shown) = (space.id, space.id == self.space);
            div()
                .id(("space", id))
                .when(self.a11y, |dot| {
                    dot.role(Role::Tab)
                        .accessibility_id(format!("space-{id}"))
                        .aria_label(space.name.clone())
                        .aria_selected(shown)
                        .pressed_by_mouse_down(false, true)
                })
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
                .child(match SpaceIcon::parse(&space.icon) {
                    SpaceIcon::Dot => {
                        let size = if shown { SPACE_SWATCH_SHOWN } else { SPACE_SWATCH };
                        (self.palette_of(id).swatch.draw(size, &behind))
                            .when(!shown, |swatch| {
                                swatch.group_hover("space", |style| {
                                    style.size(px(SPACE_SWATCH_HOVERED))
                                })
                            })
                            .into_any_element()
                    }
                    // The Space shown's in its accent, as its dot is in its colour, or the ink
                    // without one; the others' in the dots' greys.
                    SpaceIcon::Glyph(index) => svg()
                        .path(GLYPHS[index].1)
                        .size(px(SPACE_GLYPH))
                        .text_color(match (shown, self.space_theme(id).is_some()) {
                            (true, true) => self.palette_of(id).accent.into(),
                            (true, false) => self.palette.glyph(1.),
                            (false, _) => self.palette.glyph(SPACE_OTHER),
                        })
                        .when(!shown, |glyph| {
                            glyph.group_hover("space", |style| {
                                style.text_color(self.palette.glyph(SPACE_HOVERED))
                            })
                        })
                        .into_any_element(),
                    // The item's box is the dot's, so the strip doesn't move with an icon.
                    SpaceIcon::Emoji(emoji) => div()
                        .text_size(px(SPACE_ICON_SWITCHER))
                        .opacity(icon_opacity(shown, false))
                        .when(!shown, |icon| {
                            icon.group_hover("space", |style| {
                                style.opacity(icon_opacity(false, true))
                            })
                        })
                        .child(emoji.to_string())
                        .into_any_element(),
                })
        });
        Some(
            div()
                .id("switcher")
                .when(self.a11y, |switcher| switcher.role(Role::TabList).aria_label("Spaces"))
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
            .when(self.a11y, |plus| {
                plus.button_named("New Space or Folder").pressed_by_mouse_down(true, false)
            })
            .absolute()
            .right(px(PLUS_INSET))
            .bottom(px((FOOTER_HEIGHT - NAV_SIZE) / 2.))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(self.palette.plate(NAV_HOVERED)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, event: &MouseDownEvent, _, cx| {
                    shell.plus_menu(event.position, cx)
                }),
            )
            .child(svg().path("icons/nav-plus.svg").size(px(NAV_GLYPH)).text_color(self.palette.glyph(NAV)))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{AppContext as _, Modifiers, TestAppContext, point, px};

    use super::icon_opacity;
    use crate::shell::{FOOTER_HEIGHT, Folders, Page, ROW_PITCH, Shell, recorder::Recorder};

    /// A page playing holds an update installing while the Mac is idle, muted too, as a video
    /// can play muted, and stops holding it once the sound stops.
    #[gpui::test]
    fn a_page_playing_holds_the_idle_install_muted_too(cx: &mut TestAppContext) {
        let shell = cx.new(|cx| Shell::new(Rc::new(Recorder::default()), cx));
        let plays = |page: Page, cx: &mut TestAppContext| {
            shell.update(cx, |shell, cx| {
                let space = (1, "Space".into(), None, 0, "".into(), "".into(), "".into());
                shell.set_spaces(vec![space], cx);
                shell.set_tabs(1, Vec::new(), vec![1], Vec::new(), Folders::default(), cx);
                shell.set_page(1, page, cx);
                shell.plays_sound()
            })
        };
        assert!(!plays(Page::default(), cx));
        assert!(plays(Page { audible: true, ..Page::default() }, cx));
        assert!(plays(Page { audible: true, muted: true, ..Page::default() }, cx));
        assert!(!plays(Page { muted: true, ..Page::default() }, cx));
    }

    /// Once an update has downloaded, its row stands over the footer and the list stops above it,
    /// and a click on it installs the update. A private window shows none.
    #[gpui::test]
    fn a_ready_update_takes_a_row_over_the_footer(cx: &mut TestAppContext) {
        let host = Rc::new(Recorder::default());
        host.update_ready.set(true);
        let private = cx.new(|cx| Shell::private_window(host.clone(), 9, "Private".into(), 1, cx));
        assert_eq!(private.read_with(cx, |shell, _| shell.foot()), FOOTER_HEIGHT);

        let browser = host.clone();
        let (shell, cx) = cx.add_window_view(|_, cx| Shell::new(browser, cx));
        shell.update(cx, |shell, cx| {
            let space = (1, "Space".into(), None, 0, "".into(), "".into(), "".into());
            shell.set_spaces(vec![space], cx);
            shell.set_space(1, cx);
            shell.set_tabs(1, Vec::new(), vec![1, 2], Vec::new(), Folders::default(), cx);
        });
        cx.run_until_parked();
        let (foot, at) = shell.read_with(cx, |shell, _| {
            let y = f32::from(shell.viewport.height) - FOOTER_HEIGHT - ROW_PITCH / 2.;
            (shell.foot(), point(px(shell.width() / 2.), px(y)))
        });
        assert_eq!(foot, FOOTER_HEIGHT + ROW_PITCH);
        host.take();
        cx.simulate_click(at, Modifiers::none());
        assert!(host.take().contains(&"install update".to_string()));
    }

    /// An icon is dimmed as the dot it replaces is: the Space shown at full strength, the others
    /// by the dots' ratio of 0.35 to 0.6, and a hovered one by the dots' 0.5 to 0.6 (#320's
    /// ladder).
    #[test]
    fn an_icon_dims_as_the_dot_does() {
        assert_eq!(icon_opacity(true, false), 1.);
        assert_eq!(icon_opacity(true, true), 1.);
        let others = icon_opacity(false, false);
        assert!((others - 0.35 / 0.6).abs() < 0.02, "the others are at {others}");
        let hovered = icon_opacity(false, true);
        assert!((hovered - 0.5 / 0.6).abs() < 0.02, "a hovered one is at {hovered}");
        assert!(others < hovered && hovered < 1.);
    }
}
