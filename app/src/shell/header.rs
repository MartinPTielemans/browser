//! The sidebar's header: the buttons to hide the sidebar and go back, forward and
//! reload, and the URL field under them.

use std::time::Instant;

use crate::command_bar::PLACEHOLDER_TEXT;
use crate::site_controls::Place;

use gpui::{
    Bounds, Context, Div, Hsla, MouseButton, Pixels, SharedString, Stateful, Window, canvas, div,
    hsla, point, prelude::*, px, rgba, size, svg,
};

use super::{
    access::Accessible,
    BACK_X, Command, FORWARD_X, NAV, NAV_DISABLED, NAV_END, NAV_GLYPH, NAV_HOVERED, NAV_RADIUS,
    NAV_SIZE, NAV_TOP, PLATE_INSET, PLATE_RADIUS, RELOAD_X, SECONDARY, SIDEBAR_BUTTON_X, Shell,
    Swapped, TILE_HOVERED, TOOLS_FADE, URL_BUTTON_GLYPH, URL_BUTTON_INSET, URL_BUTTON_SIZE,
    URL_FIELD, URL_FIELD_HEIGHT, URL_FIELD_TEXT_X, URL_FIELD_TOP, commands::BAR_COMMANDS, fade_at,
    host,
    swap::{CHECK, LINK},
    user_keys,
};
use crate::tooltip::{Anchor, Tip};

/// Where a sidebar button's tooltip hangs: under it, as a tile's does, or right of it, as Arc's
/// does beside the footer's Library button.
#[derive(Clone, Copy)]
pub(super) enum TipSide {
    Under,
    Beside,
}

use crate::tokens::shell::header::SITE_CONTROLS_END;
use crate::tokens::shell::header::TOOL_PITCH;
use crate::tokens::shell::header::URL_PLACEHOLDER;

impl Shell {
    /// A header button at `x`, with the glyph `icon`, that runs `command`. One that can't dims, as
    /// back does on a page with no history.
    fn nav_button(
        &self,
        x: f32,
        icon: &'static str,
        command: Command,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let color = Hsla::from(rgba(if enabled { NAV } else { NAV_DISABLED }));
        // Reload and stop are one button, which changes with the tab shown's page.
        let glyph = match x == RELOAD_X {
            true => self.swaps.glyph(Swapped::Reload(self.selected), icon, NAV_GLYPH, color),
            false => svg().path(icon).size(px(NAV_GLYPH)).text_color(color).into_any_element(),
        };
        let button = div()
            .id(command.name())
            .absolute()
            .left(px(x))
            .top(px(NAV_TOP))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll();
        let name = self.button_name(command).unwrap_or(command.name());
        let button = button.when(self.a11y, |button| {
            button.button_named(name).a11y_synthetic_children(move |builder| {
                if !enabled {
                    builder.parent_node().set_disabled();
                }
            })
        });
        // A dimmed one names itself too, as the system's tooltips do.
        self.with_tooltip(button, command, TipSide::Under, cx)
            .when(enabled, |button| {
                button
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.run(command, "sidebar", cx)
                    }))
            })
            .child(glyph)
    }

    /// Gives `button`, a sidebar button's plate that runs `command`, a tooltip with the command's
    /// name and the keys of the shortcut the menus show for it, as Arc's Library button has, hung
    /// `side` of the plate. A press ends it.
    pub(super) fn with_tooltip(
        &self,
        button: Stateful<Div>,
        command: Command,
        side: TipSide,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        button
            .on_hover(cx.listener(move |shell, hovered: &bool, _, cx| {
                shell.hover_button(command, side, *hovered, cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.browser.tooltip(None, cx)),
            )
            .child(self.button_frame(command.name()))
    }

    /// Keeps the frame a button is drawn in as `key`, the name of the command it runs if it runs
    /// one, for its tooltip to hang from, since GPUI gives a hover handler no bounds.
    pub(super) fn button_frame(&self, key: &'static str) -> impl IntoElement + use<> {
        let frames = self.button_frames.clone();
        canvas(
            move |bounds, _, _| {
                frames.borrow_mut().insert(key, bounds);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    /// The pointer has come onto or left the button that runs `command`.
    pub(super) fn hover_button(
        &self,
        command: Command,
        side: TipSide,
        hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let busy = self.dragging().is_some() || self.menu_row.is_some();
        let frame = self.button_frames.borrow().get(command.name()).copied();
        let tip = (hovered && !busy)
            .then(|| self.button_name(command))
            .flatten()
            .zip(frame)
            .map(|(name, frame)| {
                let keys = user_keys(cx).chips(command).into_iter().map(Into::into).collect();
                let anchor = match side {
                    TipSide::Under => Anchor::under(frame, self.viewport),
                    TipSide::Beside => Anchor::beside(frame, self.viewport),
                };
                (Tip { text: name.into(), keys }, anchor)
            });
        self.hover_tip(tip, cx);
    }

    /// What a sidebar button's tooltip calls `command`: the menus' name, as Arc's Library button
    /// says "View Library" where its command bar says "Open Library…", else the bar's. The sidebar
    /// button's is what Arc labels it, what a click does now.
    pub(super) fn button_name(&self, command: Command) -> Option<&'static str> {
        match command {
            Command::ToggleSidebar if self.sidebar_hidden => Some("Lock Sidebar"),
            Command::ToggleSidebar => Some("Hide Sidebar"),
            Command::ViewLibrary => Some("View Library"),
            Command::ViewDownloads => Some("View Downloads"),
            _ => (BAR_COMMANDS.iter())
                .find(|&&(_, each, _)| each == command)
                .map(|&(name, ..)| name),
        }
    }

    /// The sidebar button, back, forward, and reload, which is stop while the page loads, as many
    /// as fit.
    pub(super) fn nav_buttons(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let page = self.page(self.selected);
        let reload = match page.loading {
            true => (RELOAD_X, reload_glyph(true), Command::Stop, true),
            false => (RELOAD_X, reload_glyph(false), Command::Reload, self.selected != 0),
        };
        let buttons = [
            (SIDEBAR_BUTTON_X, "icons/nav-sidebar.svg", Command::ToggleSidebar, true),
            (BACK_X, "icons/nav-back.svg", Command::Back, page.can_go_back),
            (FORWARD_X, "icons/nav-forward.svg", Command::Forward, page.can_go_forward),
            reload,
        ];
        let width = self.width();
        buttons
            .into_iter()
            .filter(|&(x, ..)| x + NAV_SIZE <= width - NAV_END)
            .map(|(x, icon, command, enabled)| {
                self.nav_button(x, icon, command, enabled, cx).into_any_element()
            })
            .collect()
    }

    /// The URL field's box in the window, while the docked sidebar shows it.
    pub(super) fn url_field_bounds(&self) -> Option<Bounds<Pixels>> {
        let docked = !self.page_fullscreen && !self.sidebar_hidden && self.sidebar_motion.is_none();
        (docked && self.form().is_none() && self.library.is_none()).then(|| {
            Bounds::new(
                point(px(PLATE_INSET), px(URL_FIELD_TOP)),
                size(px(self.width() - 2. * PLATE_INSET), px(URL_FIELD_HEIGHT)),
            )
        })
    }

    /// Where the Site Controls button ends, in from the URL field's end: Arc's place, or before
    /// the pinned extensions' buttons.
    fn site_controls_end(&self) -> f32 {
        let (shown, overflow) = self.shown_actions();
        match shown + overflow as usize {
            0 => SITE_CONTROLS_END,
            places => URL_BUTTON_INSET + places as f32 * URL_BUTTON_SIZE,
        }
    }

    /// Opens the Site Controls under the URL field, hanging from their button, or with the field
    /// hidden at the page's top left corner, as an extension's popup is; or closes them. They list
    /// every turned-on extension, the pinned ones first, which the core reports once they show.
    pub(super) fn site_controls(&mut self, cx: &mut Context<Self>) {
        if self.selected == 0 {
            return;
        }
        let place = match self.url_field_bounds() {
            Some(field) => {
                let left = field.right() - px(self.site_controls_end() + URL_BUTTON_SIZE);
                let button = Bounds::new(
                    point(left, field.top() + px(URL_BUTTON_INSET)),
                    size(px(URL_BUTTON_SIZE), px(URL_BUTTON_SIZE)),
                );
                Place::Button { field, button }
            }
            None => Place::Corner(point(px(self.card_left() + PLATE_INSET), px(PLATE_INSET))),
        };
        let extensions = self.site_controls_extensions();
        self.browser.site_controls(self.selected, place, self.viewport, extensions, cx);
    }

    /// A button at the URL field's end, `end` in from it, shown while the pointer is over the
    /// sidebar and taking clicks only then.
    fn field_tool(&self, id: &'static str, end: f32) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .opacity(self.tools_opacity())
            .absolute()
            .top(px(URL_BUTTON_INSET))
            .right(px(end))
            .size(px(URL_BUTTON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
    }

    /// What the URL field says, in its colour: the page's host, or with no page shown, the
    /// command bar's placeholder, there from the frame the card's mark is.
    pub(super) fn url_field_words(&self, url: &str) -> (SharedString, Hsla) {
        if self.selected == 0 {
            let mut color = Hsla::from(rgba(URL_PLACEHOLDER));
            color.a *= self.empty_opacity(Instant::now());
            return (PLACEHOLDER_TEXT.into(), color);
        }
        let words = self.extension_name(url).unwrap_or_else(|| host(url).to_string().into());
        (words, rgba(SECONDARY).into())
    }

    pub(super) fn url_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let url = self.url(self.selected);
        let (words, color) = self.url_field_words(&url);
        // The pinned extensions' actions, at the field's end, and Copy URL and Site Controls
        // before them, where the host stops short of them.
        let actions = self.action_buttons(cx);
        let buttons_width = actions.len() as f32 * URL_BUTTON_SIZE;
        let live = self.tools_live();
        let tools = self.tools_shown() && !url.is_empty();
        let end = self.site_controls_end();
        let copy = tools.then(|| {
            let button = self.field_tool("copy-url", end + TOOL_PITCH);
            let button = button.when(self.a11y, |button| button.button_named("Copy URL"));
            self.with_tooltip(button, Command::CopyUrl, TipSide::Under, cx)
                // Fading out, it takes no click.
                .when(live, |button| {
                    button
                        .block_mouse_except_scroll()
                        .hover(|style| style.bg(rgba(NAV_HOVERED)))
                        .on_click(cx.listener(|shell, _, _, cx| shell.copy_url_from_button(cx)))
                })
                .child(self.swaps.glyph(
                    Swapped::CopyUrl,
                    if self.url_copied.is_some() { CHECK } else { LINK },
                    URL_BUTTON_GLYPH,
                    rgba(NAV).into(),
                ))
        });
        let site_controls = tools.then(|| {
            self.field_tool("site-controls", end)
                .when(self.a11y, |button| {
                    button.button_named("Site Controls").pressed_by_mouse_down(true, false)
                })
                .when(live, |button| {
                    button
                        .block_mouse_except_scroll()
                        .hover(|style| style.bg(rgba(NAV_HOVERED)))
                        // On the press, as a pop-up button opens: a click while the panel shows
                        // closes it first, and its press then toggles rather than opens it again.
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|shell, _, _, cx| {
                                cx.stop_propagation();
                                shell.run(Command::SiteControls, "sidebar", cx);
                            }),
                        )
                })
                .child(
                    svg()
                        .path("icons/sliders-horizontal.svg")
                        .size(px(URL_BUTTON_GLYPH))
                        .text_color(rgba(NAV)),
                )
        });
        // A tab showing its page in Safari's engine says so after the host, quietly (ours): the
        // Site Controls name it and turn it off.
        let engine = (self.selected != 0 && self.page(self.selected).system_engine).then(|| {
            let mut quiet = Hsla::from(rgba(SECONDARY));
            quiet.a *= 0.6;
            svg().path("icons/compass.svg").flex_none().ml(px(5.)).size(px(12.)).text_color(quiet)
        });
        let text_end = match tools {
            true => end + TOOL_PITCH + URL_BUTTON_SIZE,
            false if actions.is_empty() => URL_FIELD_TEXT_X - PLATE_INSET,
            false => URL_BUTTON_INSET + buttons_width,
        };
        // A button, as it is: a press opens the command bar on the URL, where it's edited. It's
        // called what it shows, the host.
        let field = div().id("url-field").when(self.a11y, |field| {
            field.button_named(words.clone()).a11y_synthetic_children(|builder| {
                builder.parent_node().set_role_description("address")
            })
        });
        field
            .absolute()
            .top(px(URL_FIELD_TOP))
            .left(px(PLATE_INSET))
            .w(px(self.width() - 2. * PLATE_INSET))
            .h(px(URL_FIELD_HEIGHT))
            .flex()
            .items_center()
            .pl(px(URL_FIELD_TEXT_X - PLATE_INSET))
            .pr(px(text_end))
            .rounded(px(PLATE_RADIUS))
            .block_mouse_except_scroll()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.pressed_bar_target(cx)),
            )
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.clicked_bar_target(Command::EditUrl, "sidebar", cx)
            }))
            .bg(rgba(URL_FIELD))
            // It brightens under the pointer as a tile does, from the same white 0.10 (ours).
            .hover(|style| style.bg(hsla(0., 0., 1., TILE_HOVERED)))
            .text_size(px(13.))
            .child(div().min_w(px(0.)).truncate().text_color(color).child(words))
            .children(engine)
            .children(copy)
            .children(site_controls)
            .children(actions)
    }

    /// Whether the sidebar draws Clear and the URL field's button, which it does from the pointer
    /// coming over it until they've faded out again (Arc).
    pub(super) fn tools_shown(&self) -> bool {
        self.tools_opacity() > 0.
    }

    /// Whether they take clicks: while the pointer is over the sidebar, and not while they fade
    /// out. The reveal's panel shows only under the pointer. VoiceOver's cursor is no pointer,
    /// so they stay for it.
    pub(super) fn tools_live(&self) -> bool {
        self.sidebar_hovered || self.sidebar_hidden || self.a11y
    }

    /// How far they've faded in. The reveal's panel shows them at once, as the sidebar does while
    /// an assistive app reads it.
    pub(super) fn tools_opacity(&self) -> f32 {
        if self.sidebar_hidden || self.a11y {
            return 1.;
        }
        let to = if self.tools_on { 1. } else { 0. };
        self.tools_fade.map_or(to, |(start, from)| {
            fade_at(from, to, start.elapsed().as_secs_f32(), TOOLS_FADE)
        })
    }

    /// Starts the fade as the pointer comes or goes, from where it is, and runs frames only while
    /// it does.
    pub(super) fn step_tools(&mut self, window: &mut Window) {
        if self.sidebar_hidden {
            self.tools_fade = None;
            self.tools_on = self.sidebar_hovered;
        } else if self.sidebar_hovered != self.tools_on {
            self.tools_fade = Some((Instant::now(), self.tools_opacity()));
            self.tools_on = self.sidebar_hovered;
        }
        if let Some((start, _)) = self.tools_fade {
            if start.elapsed().as_secs_f32() < TOOLS_FADE {
                window.request_animation_frame();
                return;
            }
            self.tools_fade = None;
        }
        let shown = self.tools_shown();
        if shown != self.drawn_tools {
            self.drawn_tools = shown;
            // check.sh waits for these lines, so a capture after one shows that state.
            log!("browser: sidebar tools {}", if shown { "shown" } else { "hidden" });
        }
    }
}

/// The header's reload, which is stop while the page loads.
pub(super) fn reload_glyph(loading: bool) -> &'static str {
    if loading { "icons/nav-stop.svg" } else { "icons/nav-reload.svg" }
}

#[cfg(test)]
mod tests;
