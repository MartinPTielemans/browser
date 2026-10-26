//! The sidebar's header: the buttons to hide the sidebar and go back, forward and
//! reload, and the URL field under them.

use gpui::{Bounds, Context, Pixels, div, point, prelude::*, px, rgba, size, svg};

use super::{
    BACK_X, Command, FORWARD_X, NAV, NAV_DISABLED, NAV_END, NAV_GLYPH, NAV_HOVERED, NAV_RADIUS,
    NAV_SIZE, NAV_TOP, PLATE_INSET, PLATE_RADIUS, RELOAD_X, SECONDARY, SIDEBAR_BUTTON_X, Shell,
    URL_BUTTON_GLYPH, URL_BUTTON_INSET, URL_BUTTON_SIZE, URL_FIELD, URL_FIELD_HEIGHT,
    URL_FIELD_TEXT_X, URL_FIELD_TOP, host,
};

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
        div()
            .id(command.name())
            .absolute()
            .left(px(x))
            .top(px(NAV_TOP))
            .size(px(NAV_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .when(enabled, |button| {
                button
                    .hover(|style| style.bg(rgba(NAV_HOVERED)))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.run(command, "sidebar", cx)
                    }))
            })
            .child(
                svg()
                    .path(icon)
                    .size(px(NAV_GLYPH))
                    .text_color(rgba(if enabled { NAV } else { NAV_DISABLED })),
            )
    }

    /// The sidebar button, back, forward, and reload, which is stop while the page loads, as many
    /// as fit.
    pub(super) fn nav_buttons(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let page = self.page(self.selected);
        let reload = match page.loading {
            true => (RELOAD_X, "icons/nav-stop.svg", Command::Stop, true),
            false => (RELOAD_X, "icons/nav-reload.svg", Command::Reload, self.selected != 0),
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

    pub(super) fn url_field(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let url = self.url(self.selected);
        // The pinned extensions' actions, at the field's end, and Copy URL before them, where the
        // host stops short of them.
        let actions = self.action_buttons(cx);
        let buttons_width = actions.len() as f32 * URL_BUTTON_SIZE;
        let copy = (self.tools_shown() && !url.is_empty()).then(|| {
            div()
                .id("copy-url")
                .absolute()
                .top(px(URL_BUTTON_INSET))
                .right(px(URL_BUTTON_INSET + buttons_width))
                .size(px(URL_BUTTON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .block_mouse_except_scroll()
                .hover(|style| style.bg(rgba(NAV_HOVERED)))
                .on_click(cx.listener(|shell, _, _, cx| shell.copy_url_from_button(cx)))
                .child(
                    svg()
                        .path(if self.url_copied.is_some() {
                            "icons/check.svg"
                        } else {
                            "icons/link.svg"
                        })
                        .size(px(URL_BUTTON_GLYPH))
                        .text_color(rgba(NAV)),
                )
        });
        let text_end = match copy {
            Some(_) => URL_BUTTON_INSET + buttons_width + URL_BUTTON_SIZE,
            None if actions.is_empty() => URL_FIELD_TEXT_X - PLATE_INSET,
            None => URL_BUTTON_INSET + buttons_width,
        };
        div()
            .id("url-field")
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
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::EditUrl, "sidebar", cx)))
            .bg(rgba(URL_FIELD))
            .text_size(px(13.))
            .text_color(rgba(SECONDARY))
            .child(div().truncate().child(
                (self.extension_name(&url)).unwrap_or_else(|| host(&url).to_string().into()),
            ))
            .children(copy)
            .children(actions)
    }

    /// Whether the sidebar shows Clear and the URL field's button, which it does while the pointer
    /// is over it (Arc). The reveal's panel shows only under the pointer.
    pub(super) fn tools_shown(&self) -> bool {
        self.sidebar_hovered || self.sidebar_hidden
    }
}
