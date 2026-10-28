//! The bar in an agent's Space (#365): while the agent is at work there, what it did last, with
//! Take Over, which makes the tab the user's, and Stop; on a tab the user holds, Hand Back, which
//! gives it back. ⌘↩ is Take Over or Hand Back, with focus in the page too. It's a card at the
//! sidebar's foot, which GPUI draws, as the window's question cards are.

use gpui::{Context, FontWeight, SharedString, div, prelude::*, px, rgba};

use super::{
    FOOTER_HEIGHT, Shell, TITLE,
    permission::{BUTTONS_TOP, card_button, card_listener},
};
use crate::agents::Status;
use crate::tokens::shell::permission::{BUTTON_GAP, DETAIL, GAP};

/// What the bar shows.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Bar {
    /// The agent with the id `agent` is at work in `tab`, having last done `doing`.
    Working { agent: SharedString, name: SharedString, doing: SharedString, tab: u64 },
    /// The user holds the tab shown from the agent: handed to them for `why`, or taken over.
    Held { name: SharedString, why: SharedString, tab: u64 },
}

impl Shell {
    /// What the agents do and what the user holds from them, as the Agents tell every window.
    pub fn set_agent_status(&mut self, status: Vec<Status>, cx: &mut Context<Self>) {
        if self.agent_status != status {
            self.agent_status = status;
            cx.notify();
        }
    }

    /// The bar the Space shown has, if it's an agent's and the agent is at work in it or the user
    /// holds the tab shown.
    pub(super) fn agent_bar(&self) -> Option<Bar> {
        let space = self.shown_space().filter(|space| !space.agent.is_empty())?;
        let status = self.agent_status.iter().find(|status| status.space == space.id)?;
        let name = space.agent.clone();
        if let Some((tab, why)) = status.held.iter().find(|(tab, _)| *tab == self.selected) {
            return Some(Bar::Held { name, why: why.clone(), tab: *tab });
        }
        let tab = self.agents_at.get(&status.agent).copied()?;
        let doing = status.doing.clone();
        Some(Bar::Working { agent: status.agent.clone(), name, doing, tab })
    }

    /// Whether the user holds `tab` from an agent, which its row's mark shows in the agent's
    /// Space.
    pub(super) fn agent_holds(&self, tab: u64) -> bool {
        self.agents_space_shown()
            && self.agent_status.iter().any(|status| status.held.iter().any(|held| held.0 == tab))
    }

    /// Take Over, or Hand Back: the bar's first button and ⌘↩.
    pub(super) fn agent_bar_primary(&mut self, cx: &mut Context<Self>) {
        match self.agent_bar() {
            Some(Bar::Working { agent, tab, .. }) => {
                self.browser.take_over(agent, tab, cx);
                self.select(tab, cx);
            }
            Some(Bar::Held { tab, .. }) => self.browser.hand_back(tab, cx),
            None => {}
        }
    }

    pub(super) fn agent_bar_card(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let Some(bar) = self.agent_bar() else {
            // Drawn again, it's logged again.
            self.agent_bar_drawn.set(None);
            return None;
        };
        let (title, detail, tab) = match &bar {
            Bar::Working { name, doing, tab, .. } => {
                (format!("{name} is at work"), doing.to_string(), *tab)
            }
            Bar::Held { name, why, tab } if why.is_empty() => (
                "You have this tab".to_string(),
                format!("{name} can't act in it until you hand it back."),
                *tab,
            ),
            Bar::Held { name, why, tab } => (format!("{name} needs you"), why.to_string(), *tab),
        };
        let primary = match bar {
            Bar::Working { .. } => card_button(&self.palette, "agent-take-over", "Take Over"),
            Bar::Held { .. } => card_button(&self.palette, "agent-hand-back", "Hand Back"),
        };
        let primary = primary.on_click(cx.listener(|shell, _, _, cx| shell.agent_bar_primary(cx)));
        let stop = match &bar {
            Bar::Working { agent, .. } => {
                let agent = agent.clone();
                let stop = card_button(&self.palette, "agent-stop", "Stop");
                Some(stop.on_click(cx.listener(move |shell, _, _, cx| {
                    shell.browser.stop_agent(agent.clone(), cx)
                })))
            }
            Bar::Held { .. } => None,
        };
        let listener = card_listener("agent bar", &self.agent_bar_drawn, tab);
        Some(
            self.plate("agent-bar")
                .top(gpui::auto())
                .bottom(px(FOOTER_HEIGHT + GAP))
                .child(listener)
                .child(
                    div()
                        .text_color(self.palette.text(TITLE))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(div().truncate().text_color(rgba(DETAIL)).child(detail))
                .child(
                    div()
                        .mt(px(BUTTONS_TOP))
                        .flex()
                        .gap(px(BUTTON_GAP))
                        .children(stop)
                        .child(primary),
                ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{AppContext, Entity, SharedString, TestAppContext};

    use super::Bar;
    use crate::agents::Status;
    use crate::shell::{Command, Folders, Shell, recorder::Recorder};

    /// The user's Space 1 with tab 10, and Claude Code's Space 2 with tabs 20 and 21, shown.
    fn shell_in_agents_space(cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Recorder>) {
        let host = Rc::new(Recorder::default());
        let shell = cx.new(|cx| Shell::new(host.clone(), cx));
        shell.update(cx, |shell, cx| {
            shell.set_spaces(
                vec![
                    (1, "Space 1".into(), None, 0, "".into(), "".into(), "".into()),
                    (2, "Research".into(), None, 0, "".into(), "Claude Code".into(), "".into()),
                ],
                cx,
            );
            shell.set_tabs(1, Vec::new(), vec![10], Vec::new(), Folders::default(), cx);
            shell.set_tabs(2, Vec::new(), vec![20, 21], Vec::new(), Folders::default(), cx);
            shell.set_space(2, cx);
            shell.set_selected(21, cx);
        });
        (shell, host)
    }

    fn status(held: Vec<(u64, SharedString)>) -> Vec<Status> {
        let doing = "Clicked · example.com".into();
        vec![Status { agent: "claude-code".into(), space: 2, doing, held }]
    }

    /// While the agent is at work in its Space the bar says what it did, and Take Over, or ⌘↩,
    /// takes its tab and shows it; Stop stops it. With no agent at work there's no bar.
    #[gpui::test]
    fn the_bar_shows_the_agent_at_work_and_takes_over(cx: &mut TestAppContext) {
        let (shell, host) = shell_in_agents_space(cx);
        shell.update(cx, |shell, cx| shell.set_agent_status(status(Vec::new()), cx));
        assert_eq!(shell.read_with(cx, |shell, _| shell.agent_bar()), None);
        shell.update(cx, |shell, cx| shell.agent_acted("claude-code".into(), 20, cx));
        let bar = shell.read_with(cx, |shell, _| shell.agent_bar());
        let working = Bar::Working {
            agent: "claude-code".into(),
            name: "Claude Code".into(),
            doing: "Clicked · example.com".into(),
            tab: 20,
        };
        assert_eq!(bar, Some(working));
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::BackToPinnedUrl, "page", cx));
        assert_eq!(host.take(), ["take over claude-code 20", "select 20"]);
        shell.update(cx, |shell, cx| shell.set_space(1, cx));
        assert_eq!(shell.read_with(cx, |shell, _| shell.agent_bar()), None, "the user's Space");
    }

    /// On a tab the user holds the bar says so, the row's mark is a hollow ring, and Hand Back, or
    /// ⌘↩, gives the tab back.
    #[gpui::test]
    fn a_tab_the_user_holds_has_hand_back(cx: &mut TestAppContext) {
        let (shell, host) = shell_in_agents_space(cx);
        let held = vec![(21, "Sign in to example.com".into())];
        shell.update(cx, |shell, cx| shell.set_agent_status(status(held), cx));
        let bar = shell.read_with(cx, |shell, _| shell.agent_bar());
        let why = "Sign in to example.com".into();
        assert_eq!(bar, Some(Bar::Held { name: "Claude Code".into(), why, tab: 21 }));
        assert!(shell.read_with(cx, |shell, _| shell.agent_holds(21)));
        assert!(!shell.read_with(cx, |shell, _| shell.agent_holds(20)));
        host.take();
        shell.update(cx, |shell, cx| shell.run(Command::BackToPinnedUrl, "page", cx));
        assert_eq!(host.take(), ["hand back 21"]);
    }
}
