//! The toolbar over the page (View › Show Toolbar, ⇧⌘D or the URL field's menu), off by default as
//! Arc's: back, forward, reload, Copy URL, the URL field with the pinned extensions at its end,
//! Site Controls and, in a split view, its controls leave the sidebar for a strip at the card's
//! top, in the page's colour, and the page starts under it (docs/ux-spec.md › Toolbar). The
//! controls are the sidebar's own, laid out on the strip.

use gpui::{
    AnyElement, App, Bounds, Context, Global, Hsla, MouseButton, MouseDownEvent, Pixels, Point,
    Rgba, SharedString, div, point, prelude::*, px, rgb, size,
};

use super::{
    CARD_RADIUS, Command, MenuAction, MenuEntry, NAV, NAV_HOVERED, NAV_RADIUS, NAV_SIZE, Shell,
    URL_BUTTON_GLYPH, clean_link, header::reload_glyph, user_keys,
};
use crate::palette::{Palette, Tone, contrast};
use crate::tokens::palette::{INK, PAPER};
use crate::tokens::shell::toolbar::{
    BUTTON_INSET, BUTTON_PITCH, DIVIDER, FIELD_GAP, FIELD_INSET, HEIGHT,
};

/// Whether the toolbar shows, as the user chose for every window. It's their view: agents have
/// no call to change it.
#[derive(Clone, Copy, Default)]
pub struct ShowToolbar(pub bool);

impl Global for ShowToolbar {}

/// Whether the user chose to show the toolbar.
pub fn toolbar_shown(cx: &App) -> bool {
    cx.try_global::<ShowToolbar>().is_some_and(|chosen| chosen.0)
}

/// Whether the toolbar's URL field shows the whole URL, not the host alone, as the user chose
/// (Arc's Show Full URL, its Settings' "Show full URL when Toolbar is enabled"). The sidebar's
/// field keeps the host, as Arc's does.
#[derive(Clone, Copy, Default)]
pub struct ShowFullUrl(pub bool);

impl Global for ShowFullUrl {}

/// Whether the user chose the whole URL in the toolbar.
pub fn full_url_shown(cx: &App) -> bool {
    cx.try_global::<ShowFullUrl>().is_some_and(|chosen| chosen.0)
}

/// The whole of `url` as the toolbar's field shows it, as Dia's Show Full URL does: the host
/// without "www.", then the path, query and fragment, with no "/" at the end. Arc's isn't
/// measured. A URL with no host is shown as it is.
pub fn full_url(url: &str) -> String {
    let host = super::host(url);
    let Some((_, rest)) = url.split_once("://").filter(|_| host != url) else {
        return url.to_string();
    };
    let rest = rest.strip_prefix("www.").filter(|rest| rest.starts_with(host)).unwrap_or(rest);
    rest.strip_suffix('/').unwrap_or(rest).to_string()
}

/// What's kept for the next launch, and what it reads back as: shown only if it says so, so a
/// missing or unreadable file leaves it off, as Arc's is at first.
pub fn toolbar_text(shown: bool) -> &'static str {
    if shown { "shown" } else { "hidden" }
}

pub fn toolbar_from(text: &str) -> bool {
    text.trim() == "shown"
}

/// Where the toolbar's parts go in the window, over a card at `card`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarLayout {
    pub strip: Bounds<Pixels>,
    /// The left edges of its buttons, and their top: back, forward, reload, Copy URL and Site
    /// Controls, and the split view's controls in one.
    pub back: f32,
    pub forward: f32,
    pub reload: f32,
    pub copy: f32,
    pub site_controls: f32,
    pub split: Option<f32>,
    pub top: f32,
    /// The URL field, between Copy URL and Site Controls.
    pub field: Bounds<Pixels>,
}

/// The toolbar's parts over a card at `card`, in a split view if `split`.
pub fn toolbar_layout(card: Bounds<Pixels>, split: bool) -> ToolbarLayout {
    let (left, top) = (f32::from(card.left()), f32::from(card.top()));
    let right = f32::from(card.right());
    let at = |index: f32| left + BUTTON_INSET + index * BUTTON_PITCH;
    let last = right - BUTTON_INSET - NAV_SIZE;
    let split = split.then_some(last);
    let site_controls = split.map_or(last, |split| split - BUTTON_PITCH);
    let field_left = at(3.) + NAV_SIZE + FIELD_GAP;
    let field_right = (site_controls - FIELD_GAP).max(field_left);
    ToolbarLayout {
        strip: Bounds::new(card.origin, size(card.size.width, px(HEIGHT))),
        back: at(0.),
        forward: at(1.),
        reload: at(2.),
        copy: at(3.),
        site_controls,
        split,
        top: top + (HEIGHT - NAV_SIZE) / 2.,
        field: Bounds::new(
            point(px(field_left), px(top + FIELD_INSET)),
            size(px(field_right - field_left), px(HEIGHT - 2. * FIELD_INSET)),
        ),
    }
}

/// Where the page goes in a card at `card` under the toolbar.
pub fn under_toolbar(card: Bounds<Pixels>) -> Bounds<Pixels> {
    let height = px(HEIGHT).min(card.size.height);
    Bounds::new(
        point(card.left(), card.top() + height),
        size(card.size.width, card.size.height - height),
    )
}

/// The ink that reads on `fill`: Ink on a light page, Paper on a dark one.
pub fn tone_on(fill: Rgba) -> Tone {
    match contrast(rgb(INK), fill) >= contrast(rgb(PAPER), fill) {
        true => Tone::Light,
        false => Tone::Dark,
    }
}

impl Shell {
    /// Whether the toolbar shows over the page: as the user chose, but not while a page is full
    /// screen, which takes the window.
    pub(super) fn toolbar_shows(&self) -> bool {
        self.toolbar && self.card_left() != 0.
    }

    /// The toolbar's parts in the window, while it shows.
    pub(super) fn toolbar_layout(&self) -> Option<ToolbarLayout> {
        self.toolbar_shows().then(|| {
            let (card, _) = self.card_frames(self.viewport);
            toolbar_layout(card, self.panes(self.selected).len() > 1)
        })
    }

    /// Shows the toolbar in every window, or hides it, and keeps that for the next launch.
    pub(super) fn toggle_toolbar(&mut self, cx: &mut Context<Self>) {
        self.set_toolbar(!self.toolbar, cx);
        let shown = self.toolbar;
        cx.set_global(ShowToolbar(shown));
        self.browser.save_toolbar(shown, cx);
        // The other windows take it as they draw.
        cx.refresh_windows();
    }

    /// Takes the user's choice of toolbar, as this window draws.
    pub(super) fn set_toolbar(&mut self, shown: bool, cx: &mut Context<Self>) {
        if self.toolbar == shown {
            return;
        }
        self.toolbar = shown;
        // check.sh waits for these lines.
        log!("browser: toolbar {}", if shown { "shown" } else { "hidden" });
        self.update_menus(cx);
        cx.notify();
    }

    /// The View menu's item for ⇧⌘D, by what it does now, as Arc's.
    pub(super) fn toolbar_title(&self) -> &'static str {
        if self.toolbar { "Hide Toolbar" } else { "Show Toolbar" }
    }

    /// Shows the whole URL in every window's toolbar, or the host alone, and keeps that for the
    /// next launch.
    pub(super) fn toggle_full_url(&mut self, cx: &mut Context<Self>) {
        self.set_full_url(!self.full_url, cx);
        let shown = self.full_url;
        cx.set_global(ShowFullUrl(shown));
        self.browser.save_full_url(shown, cx);
        cx.refresh_windows();
    }

    /// Takes the user's choice of the whole URL in the toolbar, as this window draws.
    pub(super) fn set_full_url(&mut self, shown: bool, cx: &mut Context<Self>) {
        if self.full_url == shown {
            return;
        }
        self.full_url = shown;
        // check.sh waits for these lines.
        log!("browser: full url {}", if shown { "shown" } else { "hidden" });
        cx.notify();
    }

    /// The toolbar field's menu item and the command bar's row, by what it does now, as Arc's.
    pub(super) fn full_url_title(&self) -> &'static str {
        if self.full_url { "Hide Full URL" } else { "Show Full URL" }
    }

    /// What the URL field says for `url`, in its colour: the whole URL on the toolbar if the user
    /// chose it, else the host.
    pub(super) fn field_words(&self, url: &str, in_toolbar: bool) -> (SharedString, Hsla) {
        let (words, color) = self.url_field_words(url);
        let whole = in_toolbar && self.full_url && self.selected != 0;
        match whole && self.extension_name(url).is_none() {
            true => (full_url(url).into(), color),
            false => (words, color),
        }
    }

    /// The strip with the line under it, and the sidebar's controls on it, while it shows: in the
    /// page's colour where it has one, else the card's, with the ink that reads on it.
    pub(super) fn toolbar(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(layout) = self.toolbar_layout() else {
            return Vec::new();
        };
        let (fill, palette) = match self.toolbar_look() {
            (Some(color), tone) => (Hsla::from(rgb(color)), Palette::of(None, tone)),
            (None, _) => (self.window_fill().blend(self.palette.page.into()), self.palette),
        };
        // The controls draw in the window's palette, so they take the strip's while they're made.
        let window = std::mem::replace(&mut self.palette, palette);
        let parts = self.toolbar_parts(layout, fill, cx);
        self.palette = window;
        parts
    }

    /// The page's colour, which the strip takes if it has one, else the card's, and the tone of
    /// the ink on the strip.
    pub(super) fn toolbar_look(&self) -> (Option<u32>, Tone) {
        match self.page(self.selected).color {
            Some(color) => (Some(color), tone_on(rgb(color))),
            None => (None, self.palette.tone),
        }
    }

    fn toolbar_parts(
        &self,
        layout: ToolbarLayout,
        fill: Hsla,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let strip = layout.strip;
        // No clicks of its own: like a title bar, it moves the window, as the window's background
        // does.
        let strip = div()
            .absolute()
            .left(strip.left())
            .top(strip.top())
            .w(strip.size.width)
            .h(strip.size.height)
            .rounded_t(px(CARD_RADIUS))
            .bg(fill)
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(self.hairline())
                    .bg(self.palette.faint(DIVIDER)),
            );
        let page = self.page(self.selected);
        let reload = match page.loading {
            true => (reload_glyph(true), Command::Stop, true),
            false => (reload_glyph(false), Command::Reload, self.selected != 0),
        };
        let buttons = [
            (layout.back, "icons/nav-back.svg", Command::Back, page.can_go_back),
            (layout.forward, "icons/nav-forward.svg", Command::Forward, page.can_go_forward),
            (layout.reload, reload.0, reload.1, reload.2),
        ];
        let mut parts = vec![strip.into_any_element()];
        parts.extend(buttons.into_iter().map(|(x, icon, command, enabled)| {
            self.nav_button(x, layout.top, icon, command, enabled, cx).into_any_element()
        }));
        let url = !self.url(self.selected).is_empty();
        let tool = |id: &'static str, x: f32| {
            div()
                .id(id)
                .absolute()
                .left(px(x))
                .top(px(layout.top))
                .size(px(NAV_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .block_mouse_except_scroll()
                .hover(|style| style.bg(self.palette.plate(NAV_HOVERED)))
        };
        if url {
            parts.push(self.copy_url_tool(tool("copy-url", layout.copy), "toolbar", true, cx));
            let controls = tool("site-controls", layout.site_controls);
            parts.push(self.site_controls_tool(controls, "toolbar", true, cx));
        }
        parts.push(self.url_field_at(layout.field, true, cx).into_any_element());
        if let Some(x) = layout.split {
            let tab = self.selected;
            let split = tool("toolbar-split-controls", x)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        shell.split_controls(tab, event.position, cx)
                    }),
                )
                .child(
                    gpui::svg()
                        .path("icons/columns-2.svg")
                        .size(px(URL_BUTTON_GLYPH))
                        .text_color(self.palette.glyph(NAV)),
                );
            parts.push(split.into_any_element());
        }
        parts
    }

    /// The menu a right-click on the URL field shows, in the sidebar or the toolbar: Copy URL,
    /// Share as the system can, Show Toolbar or Hide Toolbar, and on the toolbar Show Full URL or
    /// Hide Full URL, as Arc's.
    pub(super) fn url_field_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let keys = user_keys(cx);
        let tab = self.selected;
        let link = clean_link(&self.url(tab));
        let mut items: Vec<(MenuEntry, &'static str, MenuAction)> = vec![(
            MenuEntry::new("Copy URL", !link.is_empty()).shortcut(Command::CopyUrl, keys),
            "copy-url",
            Box::new(|shell, cx| shell.run(Command::CopyUrl, "url field menu", cx)),
        )];
        if tab != 0 {
            self.push_share(&mut items, tab, &link, position);
        }
        items.push((MenuEntry::line(), "", Box::new(|_, _| {})));
        items.push((
            MenuEntry::new(self.toolbar_title(), true).shortcut(Command::ToggleToolbar, keys),
            "toggle-toolbar",
            Box::new(|shell, cx| shell.run(Command::ToggleToolbar, "url field menu", cx)),
        ));
        // Arc's toolbar field offers it; the sidebar's always shows the host.
        if self.toolbar {
            items.push((
                MenuEntry::new(self.full_url_title(), true),
                "toggle-full-url",
                Box::new(|shell, cx| shell.run(Command::ToggleFullUrl, "url field menu", cx)),
            ));
        }
        let entries = items.iter().map(|(entry, ..)| entry.clone()).collect();
        // Over the card from the toolbar, in the hidden sidebar's reveal from its field.
        let in_reveal = !self.toolbar && self.sidebar_hidden;
        self.context_menu_in(
            entries,
            position,
            in_reveal,
            Box::new(move |chosen, shell, cx| {
                let item = chosen.and_then(|at| items.into_iter().nth(at));
                if let Some((_, name, action)) = item {
                    // check.sh waits for this line.
                    log!("browser: command {name} via url field menu");
                    action(shell, cx)
                }
            }),
            cx,
        );
    }
}

#[cfg(test)]
#[path = "toolbar/tests.rs"]
mod tests;
