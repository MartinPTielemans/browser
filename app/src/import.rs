//! Import from the other browsers on the machine (#453): one view, shown in Settings › Import and
//! as a step of the first launch, that lists the browsers found, the one used last first, and
//! brings in what the user picks from one. Arc's sidebar comes in so far: its Spaces with their
//! names, icons, themes and profiles, their pinned tabs and folders, and its favorites.
//!
//! It reads the user's data, so it's the user's alone: agents get no path to it. It only reads
//! the other browser's files, and it adds through the core's calls a user's actions make, so a
//! second run adds nothing the first did (plan.rs).

pub mod arc;
pub mod plan;
pub mod sources;
#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::time::Instant;

use gpui::{App, Context, EventEmitter, SharedString, div, prelude::*, px, rgb};

use crate::palette::NO_THEME_ACCENT;
use crate::settings_rows::{SLIDE, button, group, row, switch};
use crate::theme::Theme;
use crate::tokens::settings::{BOX_GAP, BUTTONS_TOP, DETAIL, PADDING};
use crate::tokens::text;
use arc::Tab;
use plan::{Add, Existing, Picked, Plan, ProfileRef, Target};
use sources::{Roots, Source};

/// What the import asks of the browser: the core's calls, which the tests stand in for.
pub trait Host {
    /// Where the other browsers' folders are: the user's, or a made-up home a check run gives.
    fn roots(&self) -> Roots;
    fn existing(&self) -> Existing;
    /// Makes a profile, which loads afterwards, and returns it.
    fn create_profile(&self, name: &str) -> u64;
    /// Makes a Space after the others, in the profile the browser started with, and gives it
    /// `icon` and `theme`, its colour the theme's main one.
    fn create_space(&self, name: &str, icon: &str, theme: Option<&Theme>) -> u64;
    /// Puts the empty `space` in `profile`, which may happen only once the profile has loaded.
    fn set_space_profile(&self, space: u64, profile: u64);
    fn space_profile(&self, space: u64) -> u64;
    /// Makes a folder named `name` at the end of `folder`, else of `space`'s pinned tabs.
    fn create_folder(&self, space: u64, folder: u64, name: &str) -> u64;
    /// Pins a tab at `tab`'s URL, with its name, at the end of `folder`, else of `space`'s pinned
    /// tabs, in `space`'s profile. Its page loads once it's first shown.
    fn add_pinned(&self, space: u64, folder: u64, tab: &Tab);
    /// Makes a tab at `tab`'s URL the last of the favorites `space` shows, in its profile.
    fn add_favorite(&self, space: u64, tab: &Tab);
    /// Calls `then` once, the next time the Spaces change, such as when a Space's profile has
    /// loaded.
    fn when_spaces_change(&self, then: Box<dyn FnOnce(&mut App)>);
}

/// A host with no browser behind it, for the preview and the shell's tests: it finds the browsers
/// under its roots, and an import there adds nothing.
pub struct Detached(pub Roots);

impl Host for Detached {
    fn roots(&self) -> Roots {
        self.0.clone()
    }
    fn existing(&self) -> Existing {
        Existing::default()
    }
    fn create_profile(&self, _: &str) -> u64 {
        0
    }
    fn create_space(&self, _: &str, _: &str, _: Option<&Theme>) -> u64 {
        0
    }
    fn set_space_profile(&self, _: u64, _: u64) {}
    fn space_profile(&self, _: u64) -> u64 {
        0
    }
    fn create_folder(&self, _: u64, _: u64, _: &str) -> u64 {
        0
    }
    fn add_pinned(&self, _: u64, _: u64, _: &Tab) {}
    fn add_favorite(&self, _: u64, _: &Tab) {}
    fn when_spaces_change(&self, _: Box<dyn FnOnce(&mut App)>) {}
}

/// An import under way: the profiles and Spaces it made, and what goes into them once each Space
/// is in its profile.
pub struct Job {
    plan: Plan,
    profiles: Vec<u64>,
    /// Each planned Space's id, with the profile it waits for.
    spaces: Vec<(u64, u64)>,
}

impl Job {
    /// Makes the profiles and Spaces `plan` needs.
    pub fn start(plan: Plan, host: &dyn Host) -> Job {
        let profiles: Vec<u64> = plan
            .new_profiles
            .iter()
            .map(|name| host.create_profile(name))
            .collect();
        let profile_id = |profile: ProfileRef| match profile {
            ProfileRef::Existing(id) => id,
            ProfileRef::New(at) => profiles[at],
        };
        let spaces = (plan.spaces.iter())
            .map(|space| match &space.into {
                Target::Existing(id) => (*id, host.space_profile(*id)),
                Target::New {
                    name,
                    profile,
                    icon,
                    theme,
                } => {
                    let id = host.create_space(name, icon, theme.as_ref());
                    let profile = profile_id(*profile);
                    if host.space_profile(id) != profile {
                        host.set_space_profile(id, profile);
                    }
                    (id, profile)
                }
            })
            .collect();
        Job {
            plan,
            profiles,
            spaces,
        }
    }

    /// Whether every Space is in its profile, so its tabs can go in.
    pub fn ready(&self, host: &dyn Host) -> bool {
        self.spaces
            .iter()
            .all(|&(space, profile)| host.space_profile(space) == profile)
    }

    /// Adds the pinned tabs, folders and favorites.
    pub fn finish(self, host: &dyn Host) {
        fn add(host: &dyn Host, space: u64, folder: u64, adds: &[Add]) {
            for each in adds {
                match each {
                    Add::Tab(tab) => host.add_pinned(space, folder, tab),
                    Add::Folder {
                        existing,
                        name,
                        adds,
                    } => {
                        let inner =
                            existing.unwrap_or_else(|| host.create_folder(space, folder, name));
                        add(host, space, inner, adds);
                    }
                }
            }
        }
        for (space, &(id, _)) in self.plan.spaces.iter().zip(&self.spaces) {
            add(host, id, 0, &space.pinned);
        }
        let existing = host.existing();
        for (profile, tab) in &self.plan.favorites {
            let profile = match *profile {
                ProfileRef::Existing(id) => id,
                ProfileRef::New(at) => self.profiles[at],
            };
            // A favorite goes in through a Space of its profile; one with no Space there stays out.
            let imported = self
                .spaces
                .iter()
                .find(|&&(_, each)| each == profile)
                .map(|&(id, _)| id);
            let any = existing
                .spaces
                .iter()
                .find(|space| space.profile == profile)
                .map(|space| space.id);
            if let Some(space) = imported.or(any) {
                host.add_favorite(space, tab);
            }
        }
    }
}

/// What the view tells whoever shows it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImportEvent {
    /// The user imported, and was told what came in.
    Done,
    /// The user went on without importing.
    Skipped,
}

enum Step {
    /// The browsers found.
    List,
    /// What can come from one, to pick from.
    Pick { source: usize, picked: Picked },
    /// Waiting for new profiles to load.
    Running { source: usize, job: Option<Job> },
    /// What came in, or why nothing did.
    Done(SharedString),
}

/// The import's view: Settings shows it as its Import tab, and the first launch as a step, with
/// Skip.
pub struct ImportView {
    host: Rc<dyn Host>,
    skip: bool,
    sources: Vec<Source>,
    step: Step,
    /// Which switch last changed, and when, for its knob's slide.
    switched: Option<(&'static str, Instant)>,
}

impl EventEmitter<ImportEvent> for ImportView {}

impl ImportView {
    /// Finds the browsers on the machine. With `skip`, it offers to go on without importing, as
    /// the first launch's step does.
    pub fn new(host: Rc<dyn Host>, skip: bool) -> ImportView {
        let sources: Vec<Source> = sources::find(&host.roots())
            .into_iter()
            .filter(|source| source.arc_sidebar().is_some())
            .collect();
        let names: Vec<&str> = sources.iter().map(|source| source.browser.name).collect();
        // check.sh waits for this line.
        log!("browser: import found {}", names.join(", "));
        ImportView {
            host,
            skip,
            sources,
            step: Step::List,
            switched: None,
        }
    }

    /// Lists the browsers found again, as Settings does each time it shows the tab.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if matches!(self.step, Step::List | Step::Done(_)) {
            *self = ImportView::new(self.host.clone(), self.skip);
            cx.notify();
        }
    }

    fn pick(&mut self, source: usize, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        log!("browser: import pick {}", self.sources[source].browser.name);
        self.step = Step::Pick {
            source,
            picked: Picked {
                spaces: true,
                favorites: true,
            },
        };
        cx.notify();
    }

    /// Reads the source and starts bringing in what was picked.
    fn import(&mut self, cx: &mut Context<Self>) {
        let Step::Pick { source, picked } = self.step else {
            return;
        };
        let from = &self.sources[source];
        let name = from.browser.name;
        let read = from
            .arc_sidebar()
            .ok_or_else(|| "its sidebar is gone".to_string())
            .and_then(|path| {
                let json = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
                arc::parse(&json)
            });
        let sidebar = match read {
            Ok(sidebar) => sidebar,
            Err(error) => {
                log!("browser: import from {name} failed: {error}");
                self.step = Step::Done(format!("Couldn't read {name}'s data: {error}.").into());
                cx.notify();
                return;
            }
        };
        let names: Vec<(String, String)> = from
            .profiles
            .iter()
            .map(|profile| (profile.dir.clone(), profile.name.clone()))
            .collect();
        let plan = plan::plan(&sidebar, &names, &self.host.existing(), picked);
        let (spaces, pinned, favorites) = plan.counts();
        // check.sh waits for this line.
        log!(
            "browser: import from {name}: {spaces} Spaces, {pinned} pinned tabs, {favorites} favorites"
        );
        if plan.is_empty() {
            self.step = Step::Done(format!("Everything from {name} is here already.").into());
            cx.notify();
            return;
        }
        let summary = summary(name, spaces, pinned, favorites);
        let job = Job::start(plan, &*self.host);
        self.step = Step::Running {
            source,
            job: Some(job),
        };
        self.advance(summary, cx);
    }

    /// Finishes the job once its Spaces are in their profiles, else waits for the next change.
    fn advance(&mut self, summary: SharedString, cx: &mut Context<Self>) {
        let Step::Running { job, .. } = &mut self.step else {
            return;
        };
        if job.as_ref().is_some_and(|job| !job.ready(&*self.host)) {
            let view = cx.entity().downgrade();
            self.host.when_spaces_change(Box::new(move |cx| {
                view.update(cx, |view, cx| view.advance(summary, cx)).ok();
            }));
            return;
        }
        if let Some(job) = job.take() {
            job.finish(&*self.host);
        }
        // check.sh waits for this line.
        log!("browser: import done");
        self.step = Step::Done(summary);
        cx.notify();
    }

    fn done(&mut self, cx: &mut Context<Self>) {
        self.step = Step::List;
        cx.emit(ImportEvent::Done);
        cx.notify();
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        self.step = Step::List;
        cx.notify();
    }

    fn list(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.sources.is_empty() {
            return line("No browser to import from was found on this computer.")
                .into_any_element();
        }
        let rows = self.sources.iter().enumerate().map(|(index, source)| {
            let profiles: Vec<&str> = source
                .profiles
                .iter()
                .map(|profile| profile.name.as_str())
                .collect();
            let choose = button(("import-from", index), "Import…")
                .on_click(cx.listener(move |view, _, _, cx| view.pick(index, cx)));
            detailed(
                source.browser.name.into(),
                profiles.join(", ").into(),
                choose.into_any_element(),
            )
        });
        group(rows).into_any_element()
    }

    fn picking(&self, source: usize, picked: Picked, cx: &mut Context<Self>) -> gpui::AnyElement {
        let name = self.sources[source].browser.name;
        let toggle = |key: &'static str, on: bool, cx: &mut Context<Self>| {
            let slid = match self.switched {
                Some((switched, at)) if switched == key => {
                    (at.elapsed().as_secs_f32() / SLIDE).min(1.)
                }
                _ => 1.,
            };
            div()
                .id(key)
                .on_click(cx.listener(move |view, _, _, cx| {
                    if let Step::Pick { picked, .. } = &mut view.step {
                        let flag = if key == "spaces" {
                            &mut picked.spaces
                        } else {
                            &mut picked.favorites
                        };
                        *flag = !*flag;
                        view.switched = Some((key, Instant::now()));
                        cx.notify();
                    }
                }))
                .child(switch(on, slid, rgb(NO_THEME_ACCENT)))
                .into_any_element()
        };
        let rows = [
            row(
                None,
                "Spaces, with their pinned tabs and folders",
                toggle("spaces", picked.spaces, cx),
            ),
            row(None, "Favorites", toggle("favorites", picked.favorites, cx)),
        ];
        let any = picked.spaces || picked.favorites;
        let import = button("import", "Import")
            .when(!any, |button| button.opacity(0.5))
            .when(any, |button| {
                button.on_click(cx.listener(|view, _, _, cx| view.import(cx)))
            });
        let back =
            button("import-back", "Back").on_click(cx.listener(|view, _, _, cx| view.back(cx)));
        div()
            .flex()
            .flex_col()
            .gap(px(BOX_GAP))
            .child(line(format!("From {name}")))
            .child(group(rows))
            .child(
                div()
                    .mt(px(BUTTONS_TOP - BOX_GAP))
                    .flex()
                    .justify_end()
                    .gap(px(PADDING))
                    .child(back)
                    .child(import),
            )
            .into_any_element()
    }
}

/// A row with a grey line under its name.
fn detailed(name: SharedString, detail: SharedString, control: gpui::AnyElement) -> gpui::Div {
    div()
        .px(px(PADDING))
        .py(px(PADDING))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().truncate().child(name))
                .child(
                    div()
                        .truncate()
                        .text_size(px(text::SMALL))
                        .text_color(rgb(DETAIL))
                        .child(detail),
                ),
        )
        .child(div().flex_none().ml(px(PADDING)).child(control))
}

/// A line of text over or between the boxes.
fn line(text: impl Into<SharedString>) -> gpui::Div {
    div().px(px(PADDING)).child(text.into())
}

/// What an import brought in, as the view says it.
fn summary(name: &str, spaces: usize, pinned: usize, favorites: usize) -> SharedString {
    let count = |count: usize, one: &str, many: &str| match count {
        0 => None,
        1 => Some(format!("1 {one}")),
        count => Some(format!("{count} {many}")),
    };
    let parts: Vec<String> = [
        count(spaces, "Space", "Spaces"),
        count(pinned, "pinned tab", "pinned tabs"),
        count(favorites, "favorite", "favorites"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let what = match parts.as_slice() {
        [] => "Folders".to_string(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    format!("{what} came in from {name}.").into()
}

impl Render for ImportView {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self
            .switched
            .is_some_and(|(_, at)| at.elapsed().as_secs_f32() < SLIDE)
        {
            window.request_animation_frame();
        }
        let content = match &self.step {
            Step::List => self.list(cx),
            &Step::Pick { source, picked } => self.picking(source, picked, cx),
            Step::Running { source, .. } => line(format!(
                "Importing from {}…",
                self.sources[*source].browser.name
            ))
            .into_any_element(),
            Step::Done(said) => {
                let done = button("import-done", "Done")
                    .on_click(cx.listener(|view, _, _, cx| view.done(cx)));
                div()
                    .flex()
                    .flex_col()
                    .gap(px(BOX_GAP))
                    .child(line(said.clone()))
                    .child(div().flex().justify_end().child(done))
                    .into_any_element()
            }
        };
        let skip = (self.skip && matches!(self.step, Step::List | Step::Pick { .. })).then(|| {
            button("import-skip", "Skip")
                .on_click(cx.listener(|_, _, _, cx| cx.emit(ImportEvent::Skipped)))
        });
        div()
            .flex()
            .flex_col()
            .child(content)
            .children(skip.map(|skip| div().mt(px(BUTTONS_TOP)).flex().justify_end().child(skip)))
    }
}
