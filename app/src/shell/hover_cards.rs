//! What the shell asks of a sidebar tab's hover card (crate::hover_card): which tab it's for, what
//! it shows of it and which buttons it has, the calls they make, and when the tabs' pictures are
//! taken, which is after a run of switches is over, never on the switch.

use std::rc::Rc;

use gpui::{App, Bounds, Context, px, size};

use crate::dock_dot::QUIET;
use crate::hover_card::{Action, Anchor, Card, Event, Hover, Icon, Rest};
use crate::space_icon::has_icon;

use super::{PLATE_INSET, Row, Shell, host};

impl Shell {
    /// The pointer came onto `row` or left it: a tab's row or tile rests for its card, and leaving
    /// it lets the card go. Folders, split views' rows and New Tab have none, and none shows while
    /// a row is dragged or has its menu up.
    pub(super) fn hover_card(&self, row: Row, hovered: bool, cx: &mut Context<Self>) {
        let Row::Tab(tab) = row else {
            return;
        };
        if !hovered {
            return self.browser.hover_card(Hover::Left(tab), cx);
        }
        let busy = self.dragging().is_some() || self.menu_row.is_some();
        let Some(card) = self.card(tab).filter(|_| !busy) else {
            return;
        };
        let Some(frame) = self.tip_frames.borrow().get(&tab).copied() else {
            return;
        };
        // A tile's card starts where a row's would, past the sidebar's rows, not over the tiles
        // beside it.
        let right = self.sidebar_origin.get().x + px(self.width() - PLATE_INSET);
        let row = Bounds::new(frame.origin, size(right.max(frame.right()) - frame.origin.x, frame.size.height));
        let shell = cx.weak_entity();
        // It may tell of the card inside this update, so what it tells waits for the update's end.
        let events = Rc::new(move |tab: u64, event: Event, cx: &mut App| {
            let shell = shell.clone();
            cx.defer(move |cx| {
                shell.update(cx, |shell, cx| shell.card_event(tab, event, cx)).ok();
            });
        });
        let anchor = Anchor { row, window: self.viewport };
        self.browser.hover_card(Hover::Rest(Rest { card, anchor, events }), cx);
    }

    /// What `tab`'s card shows, if it has one.
    pub(super) fn card(&self, tab: u64) -> Option<Card> {
        if tab == 0 || self.is_folder(tab) || self.panes(tab).len() > 1 || !self.tabs.contains_key(&tab)
        {
            return None;
        }
        let each = &self.tabs[&tab];
        let icon = match has_icon(&each.page.icon) {
            true => Icon::Emoji(each.page.icon.clone()),
            false => Icon::Favicon(each.favicon.clone()),
        };
        Some(Card {
            tab,
            title: self.title(tab),
            host: host(&each.page.url).to_string().into(),
            icon,
            // The page shown is on screen beside it.
            pictured: !self.on_screen(tab),
            actions: self.card_actions(tab),
        })
    }

    /// The buttons on `tab`'s card, each the call the row's menu or its drag makes: a Today tab
    /// pins and a pinned one unpins, either becomes a favorite, and a favorite leaves the
    /// favorites, unless the Space shown is an agent's, as its menu has it, or there's no room for
    /// it among them (MOST_FAVORITES), where its menu dims Add to Favorites. A private window
    /// keeps no tab, so it has none of those. Split puts the tab beside the page shown, where its
    /// drag onto the page could, which a favorite's can't.
    pub(super) fn card_actions(&self, tab: u64) -> Vec<Action> {
        let mut actions = Vec::new();
        let favorite = self.favorites.contains(&tab);
        if !self.private {
            match favorite {
                true if !self.agents_space_shown() => actions.push(Action::Unfavorite),
                true => {}
                false if self.pinned().contains(&tab) => actions.push(Action::Unpin),
                false => actions.push(Action::Pin),
            }
            if !favorite && self.has_room_for(tab) {
                actions.push(Action::Favorite);
            }
        }
        if !favorite && self.split_partner(tab).is_some() {
            actions.push(Action::Split);
        }
        // Chrome offers it on every tab but those the share is from and of.
        if self.page(tab).share_instead {
            actions.push(Action::ShareInstead);
        }
        actions
    }

    /// Whether `tab`'s page is on screen in this window, alone or in the split view shown.
    pub(super) fn on_screen(&self, tab: u64) -> bool {
        self.selected != 0 && !self.new_tab && self.panes(self.selected).contains(&tab)
    }

    fn card_event(&mut self, tab: u64, event: Event, cx: &mut Context<Self>) {
        match event {
            // A loaded tab's picture is taken again, as the page is now.
            Event::Shown => {
                if self.tabs.contains_key(&tab) && !self.on_screen(tab) && !self.page(tab).unloaded {
                    self.browser.card_picture(tab, cx);
                }
            }
            Event::Act(action) => {
                log!("browser: command {} via hover card", action.label());
                match action {
                    Action::Pin => self.pin_tab(tab, true, cx),
                    Action::Unpin => self.pin_tab(tab, false, cx),
                    Action::Favorite => self.favorite_tab(tab, usize::MAX, cx),
                    Action::Unfavorite => self.move_tab_to(tab, false, 0, 0, cx),
                    Action::Split => {
                        if let Some(with) = self.split_partner(tab) {
                            self.browser.split_with(tab, with, false, cx);
                        }
                    }
                    Action::ShareInstead => self.browser.share_instead(tab, cx),
                }
            }
        }
    }

    /// A press, a scroll, a menu or a Space switch ends the tooltip and the card.
    pub(super) fn end_hovers(&self, cx: &mut App) {
        self.browser.tooltip(None, cx);
        self.browser.hover_card(Hover::Close, cx);
    }

    /// The tab shown was `before`'s pages and is now another's: those it hid are pictured once
    /// the switches have been quiet for QUIET, as the Dock's dot waits, so nothing lands on the
    /// switch, and a run of switches pictures only what it left hidden.
    pub(super) fn hid_for_cards(&mut self, before: Vec<u64>, cx: &mut Context<Self>) {
        let shown = self.panes(self.selected);
        self.card_hidden.extend(before.into_iter().filter(|tab| *tab != 0 && !shown.contains(tab)));
        if self.card_hidden.is_empty() {
            return;
        }
        self.card_wait = Some(cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(QUIET).await;
            shell.update(cx, |shell, cx| shell.picture_hidden(cx)).ok();
        }));
    }

    fn picture_hidden(&mut self, cx: &mut Context<Self>) {
        self.card_wait = None;
        let mut hidden = std::mem::take(&mut self.card_hidden);
        hidden.dedup();
        for tab in hidden {
            let open = self.tabs.get(&tab).is_some_and(|each| !each.page.unloaded);
            if open && !self.on_screen(tab) {
                self.browser.card_picture(tab, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests;
