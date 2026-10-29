//! Several sidebar rows chosen at once, as Arc's ⌘-click and ⇧-click choose them, for what's
//! done to them together: ⌘W closes them and ⇧⌘C copies their links. The tab shown stays the
//! one shown; the others are drawn with the secondary fill. A plain click, Esc or another Space
//! lets them go.

use gpui::{Context, MouseDownEvent};

use super::{Edit, Shell, What, clean_link};

impl Shell {
    /// The rows that can be chosen, in the sidebar's order: the shown Space's pinned tabs, outside
    /// closed folders, then its Today tabs. A split view is one row, by its first tab.
    fn choosable_rows(&self) -> Vec<u64> {
        let (pinned, today) = self.rows();
        let space = self.shown_space();
        let shut = |row: u64| space.is_some_and(|space| space.shut(row));
        let pinned = pinned.into_iter().filter(|&row| !self.is_folder(row) && !shut(row));
        pinned.chain(today).collect()
    }

    /// A left press on `tab`'s row or segment: ⌘ chooses it and ⇧ the rows through it, and
    /// returns true, as the press neither shows the tab nor drags it. A plain press lets the
    /// chosen rows go, and is the row's own.
    pub(super) fn choose_by_press(
        &mut self,
        tab: u64,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let (command, shift) = (event.modifiers.secondary(cx), event.modifiers.shift);
        match (command, shift) {
            (true, false) => self.choose_row(tab, cx),
            (false, true) => self.choose_through(tab, cx),
            _ => {
                self.unchoose(cx);
                return false;
            }
        }
        true
    }

    /// ⌘-click on `tab`'s row: chooses it, with the row shown, or lets it go if it's chosen.
    pub(super) fn choose_row(&mut self, tab: u64, cx: &mut Context<Self>) {
        let rows = self.choosable_rows();
        let row = self.row_of(tab);
        if !rows.contains(&row) {
            return;
        }
        let shown = self.row_of(self.selected);
        if self.chosen.is_empty() && shown != row && rows.contains(&shown) {
            self.chosen.push(shown);
        }
        match self.chosen.iter().position(|&each| each == row) {
            Some(at) => {
                self.chosen.remove(at);
            }
            None => self.chosen.push(row),
        }
        self.chosen_from = row;
        self.chosen_changed(cx);
    }

    /// ⇧-click on `tab`'s row: chooses the rows from the one last ⌘-clicked, else the one shown,
    /// through it, keeping those chosen already.
    pub(super) fn choose_through(&mut self, tab: u64, cx: &mut Context<Self>) {
        let rows = self.choosable_rows();
        let from = match self.chosen_from {
            0 => self.row_of(self.selected),
            from => from,
        };
        let (Some(from), Some(to)) = (
            rows.iter().position(|&row| row == from),
            rows.iter().position(|&row| row == self.row_of(tab)),
        ) else {
            return self.choose_row(tab, cx);
        };
        for &row in &rows[from.min(to)..=from.max(to)] {
            if !self.chosen.contains(&row) {
                self.chosen.push(row);
            }
        }
        self.chosen_from = rows[from];
        self.chosen_changed(cx);
    }

    /// Lets the chosen rows go, and returns whether there were any.
    pub(super) fn unchoose(&mut self, cx: &mut Context<Self>) -> bool {
        if self.chosen.is_empty() {
            return false;
        }
        self.chosen.clear();
        self.chosen_from = 0;
        self.chosen_changed(cx);
        true
    }

    /// Keeps only the chosen rows still in the sidebar, after the core reports the lists.
    pub(super) fn keep_chosen(&mut self, cx: &mut Context<Self>) {
        if self.chosen.is_empty() {
            return;
        }
        let rows = self.choosable_rows();
        let before = self.chosen.len();
        self.chosen.retain(|row| rows.contains(row));
        if self.chosen.len() != before {
            self.chosen_changed(cx);
        }
    }

    fn chosen_changed(&mut self, cx: &mut Context<Self>) {
        // A choice of the row shown alone is no choice.
        if self.chosen == [self.row_of(self.selected)] {
            self.chosen.clear();
        }
        let rows: Vec<String> = self.chosen_in_order().iter().map(u64::to_string).collect();
        // check.sh reads this line.
        log!("browser: chosen {}", rows.join(" "));
        cx.notify();
    }

    /// The chosen rows in the sidebar's order.
    pub(super) fn chosen_in_order(&self) -> Vec<u64> {
        let rows = self.choosable_rows();
        rows.into_iter().filter(|row| self.chosen.contains(row)).collect()
    }

    /// ⌘W with rows chosen closes them all, as Arc's does: Today's archive, as one step to undo,
    /// and pinned tabs unload and keep their rows, as ⌘W does to one.
    pub(super) fn close_chosen(&mut self, cx: &mut Context<Self>) {
        let rows = self.chosen_in_order();
        let today = self.today();
        let tabs: Vec<u64> = (rows.iter().filter(|row| today.contains(row)))
            .flat_map(|&row| self.panes(row))
            .collect();
        let show = self.shown_among(&tabs);
        // Bottom up, as Close Tabs Below closes them, so each reopens at its place.
        for &row in rows.iter().rev() {
            self.browser.close(row, true, cx);
        }
        self.chosen.clear();
        self.chosen_from = 0;
        let what = if tabs.len() > 1 { What::CLOSE_TABS } else { What::CLOSE_TAB };
        self.keep(what, (!tabs.is_empty()).then_some(Edit::Reopen { tabs, show }), cx);
    }

    /// ⇧⌘C with rows chosen copies each of their pages' links, one a line, without tracking
    /// parameters, as Arc's Copy URLs does.
    pub(super) fn copy_chosen(&mut self, cx: &mut Context<Self>) {
        let rows = self.chosen_in_order();
        let links: Vec<String> = (rows.iter().flat_map(|&row| self.panes(row)))
            .map(|tab| clean_link(&self.url(tab)))
            .filter(|link| !link.is_empty())
            .collect();
        if !links.is_empty() {
            self.copy(&links.join("\n"), "Copied URLs", cx);
        }
    }
}
