//! The New Space form, which a new Space's page shows in place of its list: its name,
//! its profile and its theme (docs/ux-spec.md › Spaces).

use crate::tokens::text;
use std::time::Instant;

use gpui::{
    Context, FontWeight, MouseButton, MouseDownEvent, Pixels, Point, Window, div, point, prelude::*,
    px, svg,
};

use super::{
    CANCEL_BOTTOM, CANCEL_HEIGHT, CREATE_BOTTOM, CREATE_UNNAMED, CreateSpace, FORM_CHEVRON,
    FORM_CHEVRON_GAP, FORM_FIELD_TOP, FORM_GLYPH, FORM_GLYPH_COLOR, FORM_GLYPH_X, FORM_INSET,
    FORM_LABEL_X, FORM_LEAVE, FORM_LEAVE_FIELD, FORM_LEAVE_ROWS, FORM_LEAVE_TITLE, FORM_PILL,
    FORM_PILL_HEIGHT, FORM_PILL_LEFT, FORM_PILL_RIGHT, FORM_PILL_X, FORM_PLACEHOLDER, FORM_PLATE,
    FORM_PLATE_HEIGHT, FORM_PROFILE_TOP, FORM_RADIUS, FORM_SINK, FORM_TEXT, FORM_TEXT_X,
    FORM_THEME_TOP, FORM_TITLE_HEIGHT, FORM_TITLE_TOP, HOVERED, SECONDARY, Shell, SpaceForm,
};
use crate::{text_input::TextInput, theme_picker};

impl Shell {
    /// Makes a Space and shows it with the form for naming it, or shows the form already open.
    pub(super) fn new_space(&mut self, cx: &mut Context<Self>) {
        self.new_space_with(None, cx)
    }

    /// New Space…, and with `tab` Move to › New Space… in a tab's menu, which moves the tab into
    /// the Space once the form makes it, so Cancel leaves the tab where it was.
    pub(super) fn new_space_with(&mut self, tab: Option<u64>, cx: &mut Context<Self>) {
        if self.private {
            return;
        }
        let space = match &mut self.form {
            Some(form) => {
                form.tab = tab.or(form.tab);
                form.space
            }
            None => {
                // Arc's name for a Space the user doesn't name.
                let name = format!("Space {}", self.spaces.len() + 1);
                // In the profile of the Space it's made from (Arc).
                let profile = self.shown_space().map_or(0, |space| space.profile);
                let space = self.browser.create_space(&name, profile, cx);
                let name = cx.new(|cx| {
                    TextInput::new("Space name", cx).placeholder_color(self.palette.text(FORM_PLACEHOLDER))
                });
                self.form = Some(SpaceForm {
                    space,
                    from: self.space,
                    name,
                    tab,
                });
                space
            }
        };
        if space == self.space {
            self.focus_form(cx);
        } else {
            self.show_space(space, cx);
        }
    }

    /// The form, if the window shows its Space.
    pub(super) fn form(&self) -> Option<&SpaceForm> {
        self.form.as_ref().filter(|form| form.space == self.space)
    }

    pub(super) fn focus_form(&mut self, cx: &mut Context<Self>) {
        // The panel never takes the keyboard, and goes when the pointer leaves it.
        self.dock(cx);
        self.browser.focus_ui(cx);
        self.focus_form = true;
        cx.notify();
    }

    /// Names the form's Space, if it has a name, and closes the form.
    fn create_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form() else {
            return;
        };
        let name = form.name.read(cx).text().trim().to_string();
        if name.is_empty() {
            return;
        }
        // check.sh waits for these lines.
        log!("browser: create space {} {name}", form.space);
        let (space, tab) = (form.space, form.tab);
        self.browser.rename_space(space, &name, cx);
        if let Some(tab) = tab {
            self.move_to_space(tab, space, cx);
        }
        self.leaving_form = self.form.take().map(|form| (form, Instant::now()));
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    /// Shows the Space the form was opened from and deletes the form's.
    pub(super) fn cancel_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(form) = self.close_form(window, cx) {
            self.browser.delete_space(form.space, cx);
        }
    }

    /// Shows the Space the form was opened from, and returns the form, whose Space is left for
    /// the caller to delete.
    pub(super) fn close_form(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<SpaceForm> {
        let form = self.form.take_if(|form| form.space == self.space)?;
        log!("browser: cancel space {}", form.space);
        self.show_space(form.from, cx);
        window.focus(&self.focus_handle, cx);
        cx.notify();
        Some(form)
    }

    /// The form shown, or the one a swipe cancelled, and where its page is while fingers or a
    /// settle move it off its place.
    pub(super) fn paged_form(&self) -> Option<(&SpaceForm, f32)> {
        let form = self.form().or(self.form_going.as_ref())?;
        let (_, x) = self.pages().into_iter().find(|&(space, _)| space == form.space)?;
        (x != 0.).then_some((form, x))
    }

    /// The URL field and the favorites on the page of the Space beside the form's, which comes in
    /// or goes out with it as its rows do. None with no page beside, as the form rubber-bands.
    pub(super) fn beside_form(
        &self,
        form: u64,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let (_, x) = self.pages().into_iter().find(|&(space, _)| space != form)?;
        let page = div().absolute().top_0().bottom_0().left(px(x)).w(px(self.width()));
        let page = page
            .child(self.url_field(cx))
            .children(self.scroll_edge_line())
            .children(self.tiles(cx));
        Some(div().absolute().size_full().overflow_hidden().child(page))
    }

    /// The Create a Space form, in place of the URL field and the Space's tabs, and as it leaves,
    /// `left` seconds after Create.
    pub(super) fn space_form(
        &self,
        form: &SpaceForm,
        left: Option<f32>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let named = !form.name.read(cx).text().trim().is_empty();
        // How far a part has gone, from 0 to 1, if it starts to leave `delay` after Create. It
        // fades and sinks slowly at first, as Arc's do.
        let gone = |delay: f32| left.map_or(0., |left| ((left - delay) / FORM_LEAVE).clamp(0., 1.));
        let sinking = |delay: f32| {
            let gone = gone(delay);
            (1. - gone.powi(3), FORM_SINK * gone * gone)
        };
        let (title_opacity, title_sink) = sinking(FORM_LEAVE_TITLE);
        let (field_opacity, field_sink) = sinking(FORM_LEAVE_FIELD);
        let (rows_opacity, rows_sink) = sinking(FORM_LEAVE_ROWS);
        let buttons_opacity = 1. - gone(0.);
        let plate = |id: &'static str| {
            div()
                .id(id)
                .absolute()
                .left(px(FORM_INSET))
                .w(px(self.width() - 2. * FORM_INSET))
                .h(px(FORM_PLATE_HEIGHT))
                .flex()
                .items_center()
                .rounded(px(FORM_RADIUS))
                .block_mouse_except_scroll()
        };
        let title = div()
            .absolute()
            .top(px(FORM_TITLE_TOP + title_sink))
            .opacity(title_opacity)
            .w_full()
            .h(px(FORM_TITLE_HEIGHT))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(text::HEADING))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.palette.text(FORM_TEXT))
            .child("New Space");
        let field = plate("space-name-field")
            .top(px(FORM_FIELD_TOP + field_sink))
            .opacity(field_opacity)
            .px(px(FORM_TEXT_X - FORM_INSET))
            .bg(self.palette.plate(FORM_PLATE))
            .text_size(px(text::BODY))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.palette.text(FORM_TEXT))
            .child(form.name.clone());
        // A row with a glyph and a label, whose menu a press shows while the form is up.
        let row = |id: &'static str, top: f32, glyph: &'static str, label: &'static str| {
            plate(id)
                .top(px(top + rows_sink))
                .opacity(rows_opacity)
                .pl(px(FORM_GLYPH_X - FORM_GLYPH / 2. - FORM_INSET))
                .bg(self.palette.plate(FORM_PLATE))
                .text_size(px(text::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(self.palette.text(FORM_TEXT))
                .child(svg().path(glyph).size(px(FORM_GLYPH)).text_color(self.palette.glyph(FORM_GLYPH_COLOR)))
                .child(div().ml(px(FORM_LABEL_X - FORM_GLYPH_X - FORM_GLYPH / 2.)).child(label))
        };
        let profile = (self.spaces.iter().find(|each| each.id == form.space))
            .and_then(|space| self.profiles.iter().find(|(id, _)| *id == space.profile))
            .map(|(_, name)| name.clone())
            .unwrap_or_default();
        let pill = div()
            .absolute()
            .left(px(FORM_PILL_X - FORM_INSET))
            .h(px(FORM_PILL_HEIGHT))
            .flex()
            .items_center()
            .pl(px(FORM_PILL_LEFT))
            .pr(px(FORM_PILL_RIGHT))
            .rounded(px(FORM_PILL_HEIGHT / 2.))
            .bg(self.palette.plate(FORM_PILL))
            .text_size(px(text::SMALL))
            .font_weight(FontWeight::SEMIBOLD)
            .child(profile)
            .child(
                svg()
                    .path("icons/chevron-down.svg")
                    .ml(px(FORM_CHEVRON_GAP))
                    .size(px(FORM_CHEVRON))
                    .text_color(self.palette.text(FORM_TEXT)),
            );
        let menu = |show: fn(&mut Self, Point<Pixels>, &mut Context<Self>)| {
            cx.listener(move |shell, event: &MouseDownEvent, _, cx| show(shell, event.position, cx))
        };
        // Under the pointer a row takes the list's hover fill, while a press still shows its menu.
        let profile_row = row("space-profile", FORM_PROFILE_TOP, "icons/user.svg", "Profile")
            .child(pill)
            .when(left.is_none(), |row| {
                row.hover(|style| style.bg(self.palette.plate(HOVERED)))
                    .on_mouse_down(MouseButton::Left, menu(Self::form_profile_menu))
            });
        let theme_row = row("space-theme", FORM_THEME_TOP, "icons/paintbrush.svg", "Choose a Theme")
            .when(left.is_none(), |row| {
                row.hover(|style| style.bg(self.palette.plate(HOVERED)))
                    .on_mouse_down(MouseButton::Left, menu(Self::form_theme_picker))
            });
        let create = plate("create-space")
            .bottom(px(CREATE_BOTTOM))
            .justify_center()
            .opacity(buttons_opacity * if named { 1. } else { CREATE_UNNAMED })
            .bg(self.palette.accent)
            .text_size(px(text::BODY))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(self.palette.accent_label)
            .child("Create Space")
            .on_click(cx.listener(|shell, _, window, cx| shell.create_space(window, cx)));
        let cancel = div()
            .absolute()
            .bottom(px(CANCEL_BOTTOM))
            .w_full()
            .h(px(CANCEL_HEIGHT))
            .flex()
            .justify_center()
            .opacity(buttons_opacity)
            .child(
                div()
                    .id("cancel-space")
                    .h_full()
                    .flex()
                    .items_center()
                    .block_mouse_except_scroll()
                    .text_size(px(text::SMALL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.palette.text(SECONDARY))
                    // It brightens to the form's text under the pointer (ours: Arc's isn't
                    // recorded). Create Space doesn't change, as Arc's doesn't.
                    .hover(|style| style.text_color(self.palette.text(FORM_TEXT)))
                    .child("Cancel")
                    .on_click(cx.listener(|shell, _, window, cx| shell.cancel_space(window, cx))),
            );
        div()
            .key_context("SpaceForm")
            .on_action(cx.listener(|shell, _: &CreateSpace, window, cx| {
                shell.create_space(window, cx)
            }))
            .absolute()
            .top_0()
            .w(px(self.width()))
            .bottom_0()
            .child(title)
            .child(field)
            .child(profile_row)
            .child(theme_row)
            .child(create)
            .child(cancel)
    }

    /// The form's Profile row's menu: the profiles its Space can be in, the Space's on the pill's
    /// name, as a pop-up button's menu is, and a new one.
    fn form_profile_menu(&mut self, _: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(space) = self.form().map(|form| form.space) {
            let items = self.profile_items(space);
            let over = items.iter().position(|(entry, _)| entry.checked);
            let name = point(
                px(FORM_PILL_X + FORM_PILL_LEFT),
                px(FORM_PROFILE_TOP + (FORM_PLATE_HEIGHT - FORM_PILL_HEIGHT) / 2.),
            );
            self.show_menu(items, name, over, cx);
        }
    }

    /// The form's Choose a Theme row's picker, pointing at the row's right end, as Arc's does. A
    /// press on the row while it shows closes it, as a click outside it does, without opening it
    /// again.
    fn form_theme_picker(&mut self, _: Point<Pixels>, cx: &mut Context<Self>) {
        if theme_picker::is_open(cx) {
            return;
        }
        if let Some(space) = self.form().map(|form| form.space) {
            let right = self.width() - FORM_INSET;
            let anchor = point(right, FORM_THEME_TOP + FORM_PLATE_HEIGHT / 2.);
            self.show_theme_picker(space, anchor, cx);
        }
    }
}
