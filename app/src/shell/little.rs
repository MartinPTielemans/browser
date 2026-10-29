//! Arc's Little Arc, which ilo calls a Mini Window (docs/ux-spec.md › Little Arc): one page in a
//! small window with no sidebar, under a bar in its Space's colour with the page's favicon, host and
//! title, a click on which opens the command bar on its URL, and the Space it's in, Copy Link and
//! Open in Main Window at its end.

use gpui::{
    Bounds, Context, Entity, MouseButton, Pixels, SharedString, div, point, prelude::*, px, size,
    svg,
};

use super::{
    Command, MenuEntry, NAV, NAV_HOVERED, NAV_RADIUS, SECONDARY, Shell, Space, Swapped, TITLE,
    URL_BUTTON_GLYPH, URL_BUTTON_SIZE, browser::Browser, favicon, host,
    swap::{CHECK, LINK},
};
use crate::tokens::shell::{CARD_INSET, LITTLE_BAR, LITTLE_TITLE_X, WINDOW_BUTTON_Y};
use crate::tokens::text;

/// The gap between the bar's parts (ours).
const GAP: f32 = 6.;
/// The Space button's dot.
const DOT: f32 = 10.;

/// A Mini Window's own: the user's Space it belongs to, whose colour and profile it has, and which
/// ⌘O moves its page to.
pub(super) struct Little {
    pub(super) home: u64,
}

impl Shell {
    /// A Mini Window's shell: it shows `space`, the window's own, holding its one page in
    /// `profile`, for the user's Space `home`, and knows the user's Spaces from `from` to offer
    /// the bar's Space menu. It hears nothing of the favorites or the Library, and offers none of
    /// them.
    pub fn little_window(
        browser: std::rc::Rc<dyn Browser>,
        space: u64,
        home: u64,
        profile: u64,
        from: Option<&Entity<Shell>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut shell = Self::new(browser, cx);
        shell.little = Some(Little { home });
        shell.space = space;
        shell.sidebar_hidden = true;
        let users: Vec<Space> = from
            .map(|from| from.read(cx).users_spaces().cloned().collect())
            .unwrap_or_default();
        shell.spaces = vec![Space {
            id: space,
            name: SharedString::default(),
            color: None,
            theme: None,
            icon: SharedString::default(),
            agent: SharedString::default(),
            profile,
            pinned: Vec::new(),
            today: Vec::new(),
            splits: Vec::new(),
            folders: Default::default(),
            shown: 0,
        }];
        shell.spaces.extend(users.into_iter().map(Space::without_tabs));
        shell.take_home_look();
        shell
    }

    /// The user's Space this Mini Window belongs to, or None for any other window.
    pub fn little(&self) -> Option<u64> {
        self.little.as_ref().map(|little| little.home)
    }

    /// The user's Spaces this shell knows, not an agent's or a window's own.
    fn users_spaces(&self) -> impl Iterator<Item = &Space> {
        let own = self.private || self.little.is_some();
        (self.spaces.iter())
            .skip(own as usize)
            .filter(|space| space.agent.is_empty())
    }

    /// The window's own Space takes its home's name, colour, theme and icon, as it shows them.
    fn take_home_look(&mut self) {
        let Some(home) = self.little() else {
            return;
        };
        let Some(from) = self.spaces.iter().skip(1).find(|space| space.id == home).cloned() else {
            return;
        };
        if let Some(own) = self.spaces.first_mut() {
            own.name = from.name;
            own.color = from.color;
            own.theme = from.theme;
            own.icon = from.icon;
        }
    }

    /// The Spaces, as `set_spaces` has them, for a Mini Window: the user's, for the Space menu,
    /// after its own, which keeps its tabs.
    pub(super) fn set_little_spaces(
        &mut self,
        spaces: Vec<super::SpaceEntry>,
        cx: &mut Context<Self>,
    ) {
        self.spaces.truncate(1);
        for (id, name, color, profile, icon, agent, theme) in spaces {
            if agent.is_empty() {
                self.spaces.push(Space::without_tabs(Space {
                    id,
                    name,
                    color,
                    theme: crate::theme::Theme::parse(&theme),
                    icon,
                    profile,
                    agent,
                    pinned: Vec::new(),
                    today: Vec::new(),
                    splits: Vec::new(),
                    folders: Default::default(),
                    shown: 0,
                }));
            }
        }
        self.take_home_look();
        cx.notify();
    }

    /// An Esc the page didn't use closes a Mini Window, once there's no load for it to stop and no
    /// page full screen for it to leave (Arc), and says whether it did.
    pub fn escape_unused(&mut self, cx: &mut Context<Self>) -> bool {
        let page = self.page(self.selected);
        if self.little.is_none() || page.loading || self.page_fullscreen {
            return false;
        }
        // check.sh reads this line.
        log!("browser: little window closed by escape");
        self.browser.close_window(cx);
        true
    }

    /// ⌘O: its page goes, live, to its Space in the main window, and the Mini Window closes.
    pub(super) fn open_in_main_window(&mut self, cx: &mut Context<Self>) {
        if let Some(home) = self.little()
            && self.selected != 0
        {
            self.browser.open_in_main_window(self.selected, home, cx);
        }
    }

    /// The card's top in a Mini Window, under its bar.
    pub(super) fn little_card(&self, card: Bounds<Pixels>) -> Bounds<Pixels> {
        if self.little.is_none() || self.page_fullscreen {
            return card;
        }
        let top = px(LITTLE_BAR);
        Bounds::new(point(card.origin.x, top), size(card.size.width, card.bottom() - top))
    }

    /// Where the bar's title goes, which a click opens the command bar over, as the URL field's
    /// does in the sidebar.
    fn little_title_bounds(&self) -> Bounds<Pixels> {
        let right = self.viewport.width - px(self.little_buttons_width() + CARD_INSET);
        Bounds::new(
            point(px(LITTLE_TITLE_X), px(WINDOW_BUTTON_Y - URL_BUTTON_SIZE / 2.)),
            size(right - px(LITTLE_TITLE_X), px(URL_BUTTON_SIZE)),
        )
    }

    /// Where ⌘L's compact command bar opens in a Mini Window: over its title.
    pub(super) fn little_field_bounds(&self) -> Option<Bounds<Pixels>> {
        self.little.as_ref().map(|_| self.little_title_bounds())
    }

    /// How wide the bar's buttons at its end are, with the gaps between them.
    fn little_buttons_width(&self) -> f32 {
        let name = self.spaces.first().map_or(0, |space| space.name.chars().count()) as f32;
        // The Space button: its dot and its name, which the text's width rounds up to.
        let space = DOT + GAP + name * text::BODY * 0.6 + 2. * GAP;
        space + 2. * (URL_BUTTON_SIZE + GAP)
    }

    /// The bar, over the window's tint: the page's favicon, host and title, then the Space it's
    /// in, Copy Link and Open in Main Window. None while the page is full screen.
    pub(super) fn little_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.little.is_none() || self.page_fullscreen {
            return None;
        }
        let tab = self.selected;
        let url = self.url(tab);
        let title_bounds = self.little_title_bounds();
        let favicon = favicon(&self.palette, self.tabs.get(&tab).and_then(|tab| tab.favicon.clone()));
        let title = (tab != 0).then(|| self.title(tab)).unwrap_or_default();
        let site: SharedString = host(&url).to_string().into();
        let title = div()
            .id("little-title")
            .absolute()
            .left(title_bounds.origin.x)
            .top(title_bounds.origin.y)
            .w(title_bounds.size.width)
            .h(title_bounds.size.height)
            .px(px(GAP))
            .flex()
            .items_center()
            .gap(px(GAP))
            .rounded(px(NAV_RADIUS))
            .block_mouse_except_scroll()
            .hover(|style| style.bg(self.palette.plate(NAV_HOVERED)))
            .text_size(px(text::BODY))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| shell.pressed_bar_target(cx)),
            )
            .on_click(cx.listener(|shell, _, _, cx| {
                shell.clicked_bar_target(Command::EditUrl, "little bar", cx)
            }))
            .child(div().flex_none().child(favicon))
            .child(div().flex_none().text_color(self.palette.text(SECONDARY)).child(site))
            .child(div().min_w(px(0.)).truncate().text_color(self.palette.text(TITLE)).child(title));
        let button = |id: &'static str| {
            div()
                .id(id)
                .flex_none()
                .h(px(URL_BUTTON_SIZE))
                .min_w(px(URL_BUTTON_SIZE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(NAV_RADIUS))
                .block_mouse_except_scroll()
                .hover(|style| style.bg(self.palette.plate(NAV_HOVERED)))
        };
        let home = self.spaces.first();
        let dot = div()
            .size(px(DOT))
            .rounded_full()
            .bg(self.palette.accent);
        let space = button("little-space")
            .px(px(GAP))
            .gap(px(GAP))
            .text_size(px(text::BODY))
            .text_color(self.palette.text(SECONDARY))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, _, cx| {
                    cx.stop_propagation();
                    shell.little_space_menu(cx);
                }),
            )
            .child(dot)
            .child(home.map(|home| home.name.clone()).unwrap_or_default());
        let copied = self.url_copied.is_some();
        let copy = button("little-copy")
            .on_click(cx.listener(|shell, _, _, cx| shell.copy_url_from_button(cx)))
            .child(self.swaps.glyph(
                Swapped::CopyUrl,
                if copied { CHECK } else { LINK },
                URL_BUTTON_GLYPH,
                self.palette.glyph(NAV).into(),
            ));
        let main = button("little-main")
            .on_click(cx.listener(|shell, _, _, cx| shell.run(Command::OpenInMainWindow, "little bar", cx)))
            .child(
                svg()
                    .path("icons/external-link.svg")
                    .size(px(URL_BUTTON_GLYPH))
                    .text_color(self.palette.glyph(NAV)),
            );
        let buttons = div()
            .absolute()
            .right(px(CARD_INSET))
            .top(px(WINDOW_BUTTON_Y - URL_BUTTON_SIZE / 2.))
            .flex()
            .items_center()
            .gap(px(GAP))
            .child(space)
            .child(copy)
            .child(main);
        Some(div().absolute().top_0().left_0().right_0().h(px(LITTLE_BAR)).child(title).child(buttons))
    }

    /// The bar's Space button's menu: the user's Spaces, the one it's in checked. Picking another
    /// keeps it for the page's site, whose links from other apps open there from now on (Arc).
    fn little_space_menu(&mut self, cx: &mut Context<Self>) {
        let Some(home) = self.little() else {
            return;
        };
        let spaces: Vec<(u64, SharedString)> =
            self.users_spaces().map(|space| (space.id, space.name.clone())).collect();
        let entries = spaces
            .iter()
            .map(|(space, name)| {
                MenuEntry::named(name.clone(), true)
                    .checked(*space == home)
                    .swatch(self.space_swatch(*space))
            })
            .collect();
        let at = point(
            self.viewport.width - px(self.little_buttons_width() + CARD_INSET),
            px(WINDOW_BUTTON_Y + URL_BUTTON_SIZE / 2.),
        );
        self.context_menu_in(
            entries,
            at,
            false,
            Box::new(move |index, shell, cx| {
                let Some(&(space, _)) = index.and_then(|index| spaces.get(index)) else {
                    return;
                };
                // check.sh reads this line.
                log!("browser: little space picked {space}");
                shell.pick_little_space(space, cx);
            }),
            cx,
        );
    }

    /// The user picked `space` in the bar's Space menu.
    pub(super) fn pick_little_space(&mut self, space: u64, cx: &mut Context<Self>) {
        let url = self.url(self.selected);
        let Some(little) = self.little.as_mut().filter(|_| !url.is_empty()) else {
            return;
        };
        let profile = self.spaces.first().map_or(0, |own| own.profile);
        let same = self.spaces.iter().skip(1).any(|each| each.id == space && each.profile == profile);
        if same {
            little.home = space;
            self.take_home_look();
            self.update_title(cx);
            cx.notify();
        }
        self.browser.pick_little_space(&url, space, cx);
    }
}

impl Space {
    /// The Space, without its tabs, as a Mini Window knows the user's.
    fn without_tabs(self) -> Space {
        Space {
            pinned: Vec::new(),
            today: Vec::new(),
            splits: Vec::new(),
            folders: Default::default(),
            shown: 0,
            ..self
        }
    }
}

/// Whether a Mini Window runs `command`: it has one page and no sidebar, so what works on the
/// Spaces, the sidebar, the Library or other tabs isn't its.
pub(super) fn little_runs(command: Command) -> bool {
    !matches!(
        command,
        Command::NewTab
            | Command::SearchTabs
            | Command::ReopenClosedTab
            | Command::NextTab
            | Command::PreviousTab
            | Command::RecentTab
            | Command::RecentTabBack
            | Command::SelectTab(_)
            | Command::SelectLastTab
            | Command::TogglePin
            | Command::FavoriteTab
            | Command::DuplicateTab
            | Command::RenameTab
            | Command::NewFolder
            | Command::CollapseFolders
            | Command::RevealTab
            | Command::BackToPinnedUrl
            | Command::ResetSpaceTabs
            | Command::Clear
            | Command::NewSpace
            | Command::RenameSpace
            | Command::ChangeProfile(_)
            | Command::SelectSpace(_)
            | Command::NextSpace
            | Command::PreviousSpace
            | Command::ToggleSidebar
            | Command::ViewLibrary
            | Command::ViewArchive
            | Command::ClearArchive
            | Command::ViewDownloads
            | Command::ViewHistory
            | Command::SiteSearchSettings
            | Command::ManageExtensions
            | Command::AddExtension
            | Command::AddSplit
            | Command::CloseSplitPane
            | Command::SeparatePane
            | Command::SeparateSplit
            | Command::FlipSplit
            | Command::ChangeOrientation
            | Command::ExpandSplit
            | Command::MovePaneLeft
            | Command::MovePaneRight
            | Command::NextPane
            | Command::PreviousPane
            | Command::Welcome
    )
}
