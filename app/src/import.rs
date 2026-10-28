//! Import from the other browsers on the machine (#453): one view, shown in Settings › Import and
//! as a step of the first launch, that lists the browsers found, the one used last first, and
//! brings in what the user picks from one. From Chromium's family so far: Arc's Spaces with their
//! names, icons, themes and profiles, their pinned tabs and folders, and its favorites; the
//! others' bookmarks, as a Space named after the browser; and every one's history.
//!
//! It reads the user's data, so it's the user's alone: agents get no path to it. It only reads
//! the other browser's files, and it adds through the core's calls a user's actions make, so a
//! second run adds nothing the first did (plan.rs).

pub mod arc;
pub mod chromium;
pub mod found;
pub mod plan;
pub mod sources;
#[cfg(test)]
mod tests;

use std::rc::Rc;
use std::time::{Instant, SystemTime};

use gpui::{App, Context, EventEmitter, SharedString, div, prelude::*, px, rgb};

use crate::palette::NO_THEME_ACCENT;
pub use crate::settings_rows::Look;
use crate::settings_rows::{SLIDE, button, group, row, switch};
use crate::theme::Theme;
use crate::tokens::settings::{BOX_GAP, BUTTONS_TOP, PADDING};
use crate::tokens::text;
use chromium::Kind;
use found::{Found, Page, Tab};
use plan::{Add, Counts, Existing, Plan, ProfileRef, Target};
use sources::{Family, Roots, Source};

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
    /// Whether `profile` has loaded, so its history can take pages.
    fn profile_loaded(&self, profile: u64) -> bool;
    /// Adds `pages` to `profile`'s history but for those it has, and returns whether it will. If
    /// it will, `then` gets how many it added, later.
    fn add_history(
        &self,
        profile: u64,
        pages: &[Page],
        then: Box<dyn FnOnce(usize, &mut App)>,
    ) -> bool;
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
    fn profile_loaded(&self, _: u64) -> bool {
        true
    }
    fn add_history(&self, _: u64, _: &[Page], _: Box<dyn FnOnce(usize, &mut App)>) -> bool {
        false
    }
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

    fn profile(&self, profile: ProfileRef) -> u64 {
        match profile {
            ProfileRef::Existing(id) => id,
            ProfileRef::New(at) => self.profiles[at],
        }
    }

    /// Whether every Space is in its profile, so its tabs can go in, and every profile history
    /// goes to has loaded.
    pub fn ready(&self, host: &dyn Host) -> bool {
        let spaces =
            (self.spaces.iter()).all(|&(space, profile)| host.space_profile(space) == profile);
        let history = (self.plan.history.iter())
            .all(|(profile, _)| host.profile_loaded(self.profile(*profile)));
        spaces && history
    }

    /// Adds the pinned tabs, folders and favorites, and returns the history for each profile, which
    /// the core adds away from the UI.
    pub fn finish(mut self, host: &dyn Host) -> Vec<(u64, Vec<Page>)> {
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
            let profile = self.profile(*profile);
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
        let history = std::mem::take(&mut self.plan.history);
        (history.into_iter())
            .map(|(profile, pages)| (self.profile(profile), pages))
            .collect()
    }
}

/// What the view tells whoever shows it.
#[derive(Clone, Debug, PartialEq)]
pub enum ImportEvent {
    /// An import began, which goes on while the view isn't shown; running() says so.
    Started,
    /// The import that began is over, and this says what came in, or why nothing did.
    Finished(SharedString),
    /// The user read what came in, and pressed Done.
    Done,
    /// The user went on without importing.
    Skipped,
}

enum Step {
    /// The browsers found.
    List,
    /// What can come from one, to pick from.
    Pick { source: usize, picked: Vec<Kind> },
    /// Reading the browser's files, then waiting for new profiles to load and for the history to
    /// go in.
    Running { source: usize, run: Run },
    /// What came in, or why nothing did.
    Done(SharedString),
}

/// An import under way.
#[derive(Default)]
struct Run {
    counts: Counts,
    /// Made once the browser's files are read.
    job: Option<Job>,
    /// The profiles whose history the core is still adding.
    waiting: usize,
    /// Pages the core added, which leaves out those that were here.
    pages: usize,
}

/// The import's view: Settings shows it as its Import tab, and the first launch as a step, with
/// Skip.
pub struct ImportView {
    host: Rc<dyn Host>,
    skip: bool,
    look: Look,
    sources: Vec<Source>,
    step: Step,
    /// Which switch last changed, and when, for its knob's slide.
    switched: Option<(Kind, Instant)>,
}

impl EventEmitter<ImportEvent> for ImportView {}

/// The browsers under `roots` that have something this build can bring in, the one used last
/// first.
pub fn importable(roots: &Roots) -> Vec<Source> {
    sources::find(roots)
        .into_iter()
        .filter(|source| source.browser.family == Family::Chromium && chromium::has_any(source))
        .collect()
}

impl ImportView {
    /// Finds the browsers on the machine. With `skip`, it offers to go on without importing, as
    /// the first launch's step does. It's drawn in Settings' look until `look` gives another.
    pub fn new(host: Rc<dyn Host>, skip: bool) -> ImportView {
        let sources = importable(&host.roots());
        let names: Vec<&str> = sources.iter().map(|source| source.browser.name).collect();
        // check.sh waits for this line.
        log!("browser: import found {}", names.join(", "));
        ImportView {
            host,
            skip,
            look: Look::settings(),
            sources,
            step: Step::List,
            switched: None,
        }
    }

    /// Draws it in `look`, the ground and text of the view it's shown in.
    pub fn look(mut self, look: Look) -> ImportView {
        self.look = look;
        self
    }

    pub fn set_look(&mut self, look: Look, cx: &mut Context<Self>) {
        if self.look != look {
            self.look = look;
            cx.notify();
        }
    }

    /// Whether no browser to import from was found, so a first launch can leave its step out.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// What an import under way says, such as "Importing from Chrome…", for a host to show while
    /// the view isn't.
    pub fn running(&self) -> Option<SharedString> {
        match &self.step {
            Step::Running { source, .. } => {
                Some(format!("Importing from {}…", self.sources[*source].browser.name).into())
            }
            _ => None,
        }
    }

    /// Lists the browsers found again, as Settings does each time it shows the tab.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if matches!(self.step, Step::List | Step::Done(_)) {
            *self = ImportView::new(self.host.clone(), self.skip).look(self.look);
            cx.notify();
        }
    }

    fn pick(&mut self, source: usize, cx: &mut Context<Self>) {
        // check.sh waits for this line.
        log!("browser: import pick {}", self.sources[source].browser.name);
        let picked = chromium::kinds(&self.sources[source]).to_vec();
        self.step = Step::Pick { source, picked };
        cx.notify();
    }

    fn toggle(&mut self, kind: Kind, cx: &mut Context<Self>) {
        if let Step::Pick { picked, .. } = &mut self.step {
            match picked.iter().position(|each| *each == kind) {
                Some(at) => {
                    picked.remove(at);
                }
                None => picked.push(kind),
            }
            self.switched = Some((kind, Instant::now()));
            cx.notify();
        }
    }

    /// Reads what was picked from the browser, away from the UI, then brings it in.
    fn import(&mut self, cx: &mut Context<Self>) {
        let Step::Pick { source, picked } = &self.step else {
            return;
        };
        let (source, picked) = (*source, picked.clone());
        let from = self.sources[source].clone();
        self.step = Step::Running {
            source,
            run: Run::default(),
        };
        cx.emit(ImportEvent::Started);
        cx.notify();
        let read = cx.background_spawn(async move {
            chromium::read(&from, &picked, SystemTime::now()).map(|found| (from, found))
        });
        cx.spawn(async move |view, cx| {
            let read = read.await;
            view.update(cx, |view, cx| view.read(read, cx)).ok();
        })
        .detach();
    }

    /// Plans what's read against what's here, and starts it.
    fn read(&mut self, read: Result<(Source, Found), String>, cx: &mut Context<Self>) {
        let Step::Running { source, run } = &mut self.step else {
            return;
        };
        let name = self.sources[*source].browser.name;
        let (from, found) = match read {
            Ok(read) => read,
            Err(error) => {
                log!("browser: import from {name} failed: {error}");
                return self.finish(format!("Couldn't read {name}'s data: {error}."), cx);
            }
        };
        let names: Vec<(String, String)> = (from.profiles.iter())
            .map(|profile| (profile.dir.clone(), profile.name.clone()))
            .collect();
        let plan = plan::plan(&found, &names, &self.host.existing());
        let counts = plan.counts();
        // check.sh waits for this line.
        log!(
            "browser: import from {name}: {} Spaces, {} pinned tabs, {} favorites, {} pages",
            counts.spaces,
            counts.pinned,
            counts.favorites,
            counts.pages
        );
        if plan.is_empty() {
            return self.finish(format!("Everything from {name} is here already."), cx);
        }
        run.counts = counts;
        run.job = Some(Job::start(plan, &*self.host));
        self.advance(cx);
    }

    /// Adds what goes in once the Spaces are in their profiles, else waits for the next change.
    fn advance(&mut self, cx: &mut Context<Self>) {
        let Step::Running { run, .. } = &mut self.step else {
            return;
        };
        let Some(job) = run.job.take_if(|job| job.ready(&*self.host)) else {
            let view = cx.entity().downgrade();
            self.host.when_spaces_change(Box::new(move |cx| {
                view.update(cx, |view, cx| view.advance(cx)).ok();
            }));
            return;
        };
        for (profile, pages) in job.finish(&*self.host) {
            let view = cx.entity().downgrade();
            let then = Box::new(move |added, cx: &mut App| {
                view.update(cx, |view, cx| view.history_added(added, cx))
                    .ok();
            });
            if self.host.add_history(profile, &pages, then) {
                run.waiting += 1;
            }
        }
        if run.waiting == 0 {
            self.complete(cx);
        }
    }

    fn history_added(&mut self, added: usize, cx: &mut Context<Self>) {
        let Step::Running { run, .. } = &mut self.step else {
            return;
        };
        run.pages += added;
        run.waiting = run.waiting.saturating_sub(1);
        if run.waiting == 0 {
            self.complete(cx);
        }
    }

    fn complete(&mut self, cx: &mut Context<Self>) {
        let Step::Running { source, run } = &self.step else {
            return;
        };
        let name = self.sources[*source].browser.name;
        let counts = Counts {
            pages: run.pages,
            ..run.counts
        };
        self.finish(summary(name, counts), cx);
    }

    fn finish(&mut self, said: impl Into<SharedString>, cx: &mut Context<Self>) {
        let said = said.into();
        // check.sh waits for this line.
        log!("browser: import done: {said}");
        self.step = Step::Done(said.clone());
        cx.emit(ImportEvent::Finished(said));
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
        let look = &self.look;
        let rows = self.sources.iter().enumerate().map(|(index, source)| {
            let profiles: Vec<&str> = (source.profiles.iter())
                .map(|profile| profile.name.as_str())
                .collect();
            let choose = button(look, ("import-from", index), "Import…")
                .on_click(cx.listener(move |view, _, _, cx| view.pick(index, cx)));
            detailed(
                look,
                source.browser.name.into(),
                profiles.join(", ").into(),
                choose.into_any_element(),
            )
        });
        group(look, rows).into_any_element()
    }

    fn picking(&self, source: usize, picked: &[Kind], cx: &mut Context<Self>) -> gpui::AnyElement {
        let look = &self.look;
        let name = self.sources[source].browser.name;
        let rows = chromium::kinds(&self.sources[source]).iter().map(|&kind| {
            let slid = match self.switched {
                Some((switched, at)) if switched == kind => {
                    (at.elapsed().as_secs_f32() / SLIDE).min(1.)
                }
                _ => 1.,
            };
            let on = picked.contains(&kind);
            let toggle = div()
                .id(("import-kind", kind as usize))
                .on_click(cx.listener(move |view, _, _, cx| view.toggle(kind, cx)))
                .child(switch(look, on, slid, rgb(NO_THEME_ACCENT)));
            row(None, kind.label(), toggle.into_any_element())
        });
        let any = !picked.is_empty();
        let import = button(look, "import", "Import")
            .when(!any, |button| button.opacity(0.5))
            .when(any, |button| {
                button.on_click(cx.listener(|view, _, _, cx| view.import(cx)))
            });
        let back = button(look, "import-back", "Back")
            .on_click(cx.listener(|view, _, _, cx| view.back(cx)));
        div()
            .flex()
            .flex_col()
            .gap(px(BOX_GAP))
            .child(line(format!("From {name}")))
            .child(group(look, rows))
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
fn detailed(
    look: &Look,
    name: SharedString,
    detail: SharedString,
    control: gpui::AnyElement,
) -> gpui::Div {
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
                        .text_color(look.detail)
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
fn summary(name: &str, counts: Counts) -> SharedString {
    let count = |count: usize, one: &str, many: &str| match count {
        0 => None,
        1 => Some(format!("1 {one}")),
        count => Some(format!("{count} {many}")),
    };
    let parts: Vec<String> = [
        count(counts.spaces, "Space", "Spaces"),
        count(counts.pinned, "pinned tab", "pinned tabs"),
        count(counts.favorites, "favorite", "favorites"),
        count(counts.pages, "page of history", "pages of history"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let what = match parts.as_slice() {
        [] => return format!("Everything from {name} is here already.").into(),
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
        let look = self.look;
        let content = match &self.step {
            Step::List => self.list(cx),
            Step::Pick { source, picked } => self.picking(*source, &picked.clone(), cx),
            Step::Running { .. } => line(self.running().unwrap_or_default()).into_any_element(),
            Step::Done(said) => {
                let done = button(&look, "import-done", "Done")
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
            button(&look, "import-skip", "Skip")
                .on_click(cx.listener(|_, _, _, cx| cx.emit(ImportEvent::Skipped)))
        });
        div()
            .flex()
            .flex_col()
            .text_color(look.ink)
            .child(content)
            .children(skip.map(|skip| div().mt(px(BUTTONS_TOP)).flex().justify_end().child(skip)))
    }
}
