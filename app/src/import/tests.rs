//! The import on made-up profiles: Arc's sidebar in `fixture/`, laid out as a Mac's home folder,
//! and each system's browsers in folders the tests make. Never a real browser's data.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use gpui::{App, AppContext, TestAppContext};

use super::arc::{self, ArcProfile, Node, Tab};
use super::plan::{self, Existing, ExistingSpace, Picked, Row};
use super::sources::{self, Os, Roots};
use super::{Host, ImportEvent, ImportView};
use crate::theme::{Appearance, Theme};

const ALL: Picked = Picked {
    spaces: true,
    favorites: true,
};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/import/fixture")
}

fn sidebar() -> arc::Sidebar {
    let path = fixture().join("Library/Application Support/Arc/StorableSidebar.json");
    arc::parse(&fs::read_to_string(path).unwrap()).unwrap()
}

fn tab(url: &str) -> Node {
    Node::Tab(Tab {
        url: url.into(),
        name: String::new(),
    })
}

/// Spaces, their pinned tabs and folders, their profiles and the favorites come in; Arc's own
/// pages, the welcome card, Today tabs and ids that name nothing don't. A split view's tabs take
/// its place, and a folder named inside itself comes in once.
#[test]
fn arcs_sidebar_reads_as_its_spaces_and_favorites() {
    let sidebar = sidebar();
    let [personal, work] = &sidebar.spaces[..] else {
        panic!("{:?}", sidebar.spaces)
    };
    assert_eq!(
        (personal.name.as_str(), &personal.profile),
        ("Personal", &ArcProfile::Default)
    );
    assert_eq!(personal.icon, "🏠");
    let named = Node::Tab(Tab {
        url: "https://news.ycombinator.com/".into(),
        name: "HN".into(),
    });
    let deep = Node::Folder {
        name: "Deep".into(),
        children: vec![tab("https://example.com/d")],
    };
    let reading = Node::Folder {
        name: "Reading".into(),
        children: vec![
            tab("https://example.com/a"),
            tab("https://example.com/b"),
            tab("https://example.com/c"),
            deep,
        ],
    };
    assert_eq!(personal.pinned, vec![named, reading]);
    assert_eq!(
        (work.name.as_str(), &work.profile),
        ("Work", &ArcProfile::Custom("Profile 1".into()))
    );
    assert_eq!(work.icon, "glyph:bolt");
    let code = Node::Tab(Tab {
        url: "https://github.com/".into(),
        name: "Code".into(),
    });
    assert_eq!(work.pinned, vec![code]);
    let urls = |tabs: &[Tab]| tabs.iter().map(|tab| tab.url.clone()).collect::<Vec<_>>();
    let [(default, mine), (custom, theirs)] = &sidebar.favorites[..] else {
        panic!()
    };
    assert_eq!(default, &ArcProfile::Default);
    assert_eq!(
        urls(mine),
        ["https://mail.example.com/", "https://calendar.example.com/"]
    );
    assert_eq!(custom, &ArcProfile::Custom("Profile 1".into()));
    assert_eq!(urls(theirs), ["https://linear.app/"]);
}

/// A gradient's first colour is the main one, its colours as many as ours go, with Arc's
/// intensity, grain and appearance; a colour outside sRGB is brought in.
#[test]
fn arcs_theme_becomes_the_spaces_theme() {
    let sidebar = sidebar();
    let personal = sidebar.spaces[0].theme.unwrap();
    assert_eq!(personal.count, 2);
    assert_eq!(
        personal.main_color(),
        Theme::of_color(0x3366cc).main_color()
    );
    assert_eq!((personal.intensity, personal.grain), (0.6, 0.3));
    assert_eq!(personal.appearance, Appearance::Dark);
    let work = sidebar.spaces[1].theme.unwrap();
    assert_eq!(work.count, 1);
    assert_eq!(work.main_color(), Theme::of_color(0xe699b3).main_color());
    assert_eq!(work.appearance, Appearance::Light);
}

#[test]
fn a_file_that_isnt_arcs_sidebar_says_so() {
    assert!(arc::parse("not json").is_err());
    assert!(arc::parse(r#"{"sidebar": {"containers": [{"global": {}}]}}"#).is_err());
}

/// A folder the test makes, empty, for one test's browsers.
fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ilo-import-{}-{name}", std::process::id()));
    _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: PathBuf, text: &str) -> PathBuf {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

/// Marks a file used `ago` seconds ago.
fn used(path: &Path, ago: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(ago))
        .unwrap();
}

fn found(roots: &Roots) -> Vec<(&'static str, Vec<String>)> {
    let sources = sources::find(roots);
    let names = |source: &sources::Source| source.profiles.iter().map(|p| p.name.clone()).collect();
    sources
        .iter()
        .map(|source| (source.browser.name, names(source)))
        .collect()
}

/// On the Mac, Arc's profiles by the names its `Local State` gives them, with its sidebar.
#[test]
fn the_mac_finds_arc_with_its_profiles() {
    let roots = Roots::under(&fixture(), Os::Mac);
    assert_eq!(
        found(&roots),
        [("Arc", vec!["Personal".to_string(), "Work".to_string()])]
    );
    let arc = &sources::find(&roots)[0];
    assert!(arc.arc_sidebar().is_some());
}

/// On Windows, Arc in the folder the Store gives it, Chrome in Local AppData and Firefox's
/// profiles in Roaming, the one used last first; a folder with no profile in it is no browser.
#[test]
fn windows_finds_each_browser_where_it_keeps_its_profiles() {
    let home = home("windows");
    let arc = home
        .join("AppData/Local/Packages/TheBrowserCompany.Arc_ttt1ap7aakyb4/LocalCache/Local/Arc");
    write(arc.join("StorableSidebar.json"), "{}");
    let arc_used = write(arc.join("User Data/Default/History"), "");
    write(arc.join("User Data/Default/Preferences"), "{}");
    let chrome = home.join("AppData/Local/Google/Chrome/User Data");
    write(
        chrome.join("Local State"),
        r#"{"profile":{"info_cache":{"Profile 2":{"name":"Two"},"Profile 10":{"name":"Ten"},"Default":{"name":"Me"}}}}"#,
    );
    for profile in ["Default", "Profile 2", "Profile 10"] {
        used(&write(chrome.join(profile).join("Preferences"), "{}"), 100);
    }
    let firefox = home.join("AppData/Roaming/Mozilla/Firefox");
    write(
        firefox.join("profiles.ini"),
        "[Install1]\nDefault=Profiles/a.default\n\n[Profile0]\nName=default-release\nIsRelative=1\nPath=Profiles/a.default\n\n[Profile1]\nName=gone\nIsRelative=1\nPath=Profiles/b.gone\n",
    );
    used(
        &write(firefox.join("Profiles/a.default/places.sqlite"), ""),
        50,
    );
    fs::create_dir_all(home.join("AppData/Local/Microsoft/Edge/User Data")).unwrap();
    used(&arc_used, 10);
    let roots = Roots::under(&home, Os::Windows);
    let strings = |names: &[&str]| {
        names
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        found(&roots),
        [
            ("Arc", strings(&["Default"])),
            ("Firefox", strings(&["default-release"])),
            ("Chrome", strings(&["Me", "Two", "Ten"])),
        ]
    );
    fs::remove_dir_all(home).unwrap();
}

/// On Linux, Chrome in `~/.config` and Firefox in `~/.mozilla`; Opera keeps its one profile in
/// its folder itself. Arc and Safari aren't looked for.
#[test]
fn linux_finds_each_browser_where_it_keeps_its_profiles() {
    let home = home("linux");
    used(
        &write(home.join(".config/google-chrome/Default/Preferences"), "{}"),
        30,
    );
    used(&write(home.join(".config/opera/Preferences"), "{}"), 20);
    write(
        home.join(".mozilla/firefox/profiles.ini"),
        "[Profile0]\nName=default\nPath=x.default\n",
    );
    used(
        &write(home.join(".mozilla/firefox/x.default/places.sqlite"), ""),
        10,
    );
    write(
        home.join("Library/Application Support/Arc/User Data/Default/Preferences"),
        "{}",
    );
    let roots = Roots::under(&home, Os::Linux);
    let names: Vec<_> = found(&roots).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["Firefox", "Opera", "Chrome"]);
    fs::remove_dir_all(home).unwrap();
}

/// The browser as the import sees it: profiles, Spaces with their pinned rows, and favorites.
/// A Space's profile changes only once `load` stands in for the profile having loaded.
#[derive(Default)]
struct Browser {
    roots: Option<Roots>,
    state: RefCell<State>,
    waiting: RefCell<Vec<Box<dyn FnOnce(&mut App)>>>,
}

#[derive(Default)]
struct State {
    next: u64,
    profiles: Vec<(u64, String)>,
    spaces: Vec<Space>,
    favorites: Vec<(u64, Tab)>,
    /// Spaces put in a profile that hasn't loaded yet.
    loading: Vec<(u64, u64)>,
}

#[derive(Clone, Debug, PartialEq)]
struct Space {
    id: u64,
    name: String,
    profile: u64,
    icon: String,
    theme: Option<Theme>,
    rows: Vec<FakeRow>,
}

#[derive(Clone, Debug, PartialEq)]
enum FakeRow {
    Tab(Tab),
    Folder(u64, String, Vec<FakeRow>),
}

impl State {
    fn id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }

    fn space(&mut self, id: u64) -> &mut Space {
        self.spaces.iter_mut().find(|space| space.id == id).unwrap()
    }
}

fn folder_rows(rows: &mut Vec<FakeRow>, folder: u64) -> Option<&mut Vec<FakeRow>> {
    if folder == 0 {
        return Some(rows);
    }
    for row in rows {
        if let FakeRow::Folder(id, _, inner) = row {
            if *id == folder {
                return Some(inner);
            }
            if let Some(found) = folder_rows(inner, folder) {
                return Some(found);
            }
        }
    }
    None
}

impl Browser {
    /// A browser as a first launch leaves it: one profile and one empty Space.
    fn new(roots: Option<Roots>) -> Browser {
        let browser = Browser {
            roots,
            ..Browser::default()
        };
        {
            let mut state = browser.state.borrow_mut();
            state.profiles.push((1, "Default".into()));
            state.spaces.push(Space {
                id: 2,
                name: "Space".into(),
                profile: 1,
                icon: String::new(),
                theme: None,
                rows: Vec::new(),
            });
            state.next = 2;
        }
        browser
    }

    /// The new profiles have loaded: the Spaces waiting for them move in, and the Spaces changed.
    fn load(&self, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        for (space, profile) in std::mem::take(&mut state.loading) {
            let space = state.space(space);
            if space.rows.is_empty() {
                space.profile = profile;
            }
        }
        drop(state);
        for then in self.waiting.take() {
            then(cx);
        }
    }

    fn spaces(&self) -> Vec<Space> {
        self.state.borrow().spaces.clone()
    }
}

impl Host for Browser {
    fn roots(&self) -> Roots {
        self.roots
            .clone()
            .unwrap_or_else(|| Roots::under(&fixture(), Os::Mac))
    }

    fn existing(&self) -> Existing {
        fn rows(of: &[FakeRow]) -> Vec<Row> {
            of.iter()
                .map(|row| match row {
                    FakeRow::Tab(tab) => Row::Tab(tab.url.clone()),
                    FakeRow::Folder(id, name, inner) => Row::Folder {
                        id: *id,
                        name: name.clone(),
                        rows: rows(inner),
                    },
                })
                .collect()
        }
        let state = self.state.borrow();
        Existing {
            profiles: state.profiles.clone(),
            spaces: (state.spaces.iter())
                .map(|space| ExistingSpace {
                    id: space.id,
                    name: space.name.clone(),
                    profile: space.profile,
                    pinned: rows(&space.rows),
                })
                .collect(),
            favorites: state
                .favorites
                .iter()
                .map(|(profile, tab)| (*profile, tab.url.clone()))
                .collect(),
        }
    }

    fn create_profile(&self, name: &str) -> u64 {
        let mut state = self.state.borrow_mut();
        let id = state.id();
        state.profiles.push((id, name.into()));
        id
    }

    fn create_space(&self, name: &str, icon: &str, theme: Option<&Theme>) -> u64 {
        let mut state = self.state.borrow_mut();
        let id = state.id();
        let space = Space {
            id,
            name: name.into(),
            profile: 1,
            icon: icon.into(),
            theme: theme.copied(),
            rows: Vec::new(),
        };
        state.spaces.push(space);
        id
    }

    fn set_space_profile(&self, space: u64, profile: u64) {
        self.state.borrow_mut().loading.push((space, profile));
    }

    fn space_profile(&self, space: u64) -> u64 {
        self.state.borrow_mut().space(space).profile
    }

    fn create_folder(&self, space: u64, folder: u64, name: &str) -> u64 {
        let mut state = self.state.borrow_mut();
        let id = state.id();
        let rows = folder_rows(&mut state.space(space).rows, folder).unwrap();
        rows.push(FakeRow::Folder(id, name.into(), Vec::new()));
        id
    }

    fn add_pinned(&self, space: u64, folder: u64, tab: &Tab) {
        let mut state = self.state.borrow_mut();
        folder_rows(&mut state.space(space).rows, folder)
            .unwrap()
            .push(FakeRow::Tab(tab.clone()));
    }

    fn add_favorite(&self, space: u64, tab: &Tab) {
        let mut state = self.state.borrow_mut();
        let profile = state.space(space).profile;
        state.favorites.push((profile, tab.clone()));
    }

    fn when_spaces_change(&self, then: Box<dyn FnOnce(&mut App)>) {
        self.waiting.borrow_mut().push(then);
    }
}

/// Rows as text: a tab's URL, after its name if it has one, and a folder's name with its rows.
fn shape(rows: &[FakeRow]) -> String {
    let rows: Vec<String> = rows
        .iter()
        .map(|row| match row {
            FakeRow::Tab(tab) if tab.name.is_empty() => tab.url.clone(),
            FakeRow::Tab(tab) => format!("{}={}", tab.name, tab.url),
            FakeRow::Folder(_, name, rows) => format!("{name}[{}]", shape(rows)),
        })
        .collect();
    rows.join(", ")
}

fn names(browser: &Browser) -> Vec<String> {
    browser
        .spaces()
        .into_iter()
        .map(|space| space.name)
        .collect()
}

fn profile_names() -> Vec<(String, String)> {
    vec![
        ("Default".into(), "Personal".into()),
        ("Profile 1".into(), "Work".into()),
    ]
}

/// Runs a whole import of the fixture into `browser`, its profiles loading at once.
fn import(browser: &Browser, picked: Picked, cx: &mut App) {
    let plan = plan::plan(&sidebar(), &profile_names(), &browser.existing(), picked);
    let job = super::Job::start(plan, browser);
    browser.load(cx);
    assert!(job.ready(browser));
    job.finish(browser);
}

/// Arc's default profile is the one the browser started with, and its other profile a new one
/// named as Arc named it. Each Space comes in after the others, with its icon and theme, and its
/// pinned tabs and folders only once it's in its profile. Each profile's favorites go to it.
#[gpui::test]
fn an_import_brings_in_spaces_folders_and_favorites(cx: &mut TestAppContext) {
    let browser = Browser::new(None);
    cx.update(|cx| import(&browser, ALL, cx));
    assert_eq!(names(&browser), ["Space", "Personal", "Work"]);
    let state = browser.state.borrow();
    assert_eq!(
        state.profiles,
        [(1, "Default".to_string()), (3, "Work".to_string())]
    );
    let personal = &state.spaces[1];
    assert_eq!((personal.profile, personal.icon.as_str()), (1, "🏠"));
    assert_eq!(personal.theme.unwrap().count, 2);
    assert_eq!(
        shape(&personal.rows),
        "HN=https://news.ycombinator.com/, Reading[https://example.com/a, https://example.com/b, \
         https://example.com/c, Deep[https://example.com/d]]"
    );
    let work = &state.spaces[2];
    assert_eq!((work.profile, work.icon.as_str()), (3, "glyph:bolt"));
    let favorites: Vec<_> = state
        .favorites
        .iter()
        .map(|(profile, tab)| (*profile, tab.url.as_str()))
        .collect();
    assert_eq!(
        favorites,
        [
            (1, "https://mail.example.com/"),
            (1, "https://calendar.example.com/"),
            (3, "https://linear.app/")
        ]
    );
}

/// Running it again finds everything there, so it adds nothing: no Space, profile, folder, tab
/// or favorite twice.
#[gpui::test]
fn running_it_again_adds_nothing(cx: &mut TestAppContext) {
    let browser = Browser::new(None);
    cx.update(|cx| import(&browser, ALL, cx));
    let plan = plan::plan(&sidebar(), &profile_names(), &browser.existing(), ALL);
    assert!(plan.is_empty(), "{plan:?}");
}

/// A Space of Arc's name in the same profile is the one it joins: what it has already stays as it
/// is, a folder of the same name gets what it lacks, and the Space keeps its own look.
#[gpui::test]
fn a_space_already_here_gets_only_what_it_lacks(cx: &mut TestAppContext) {
    let browser = Browser::new(None);
    {
        let mut state = browser.state.borrow_mut();
        let space = state.space(2);
        space.name = "Personal".into();
        space.rows = vec![
            FakeRow::Tab(Tab {
                url: "https://news.ycombinator.com/".into(),
                name: String::new(),
            }),
            FakeRow::Folder(
                9,
                "Reading".into(),
                vec![FakeRow::Tab(Tab {
                    url: "https://example.com/a".into(),
                    name: String::new(),
                })],
            ),
        ];
        state.next = 9;
    }
    let picked = Picked {
        spaces: true,
        favorites: false,
    };
    let plan = plan::plan(&sidebar(), &profile_names(), &browser.existing(), picked);
    assert_eq!(plan.counts(), (1, 4, 0));
    cx.update(|cx| import(&browser, picked, cx));
    assert_eq!(names(&browser), ["Personal", "Work"]);
    let personal = &browser.spaces()[0];
    assert_eq!(personal.icon, "");
    assert_eq!(
        shape(&personal.rows),
        "https://news.ycombinator.com/, Reading[https://example.com/a, https://example.com/b, \
         https://example.com/c, Deep[https://example.com/d]]"
    );
}

/// A profile of Arc's name here already is the one its Spaces and favorites go to.
#[test]
fn a_profile_of_arcs_name_is_the_one_used() {
    let existing = Existing {
        profiles: vec![(1, "Me".into()), (7, "Work".into())],
        ..Existing::default()
    };
    let plan = plan::plan(&sidebar(), &profile_names(), &existing, ALL);
    assert!(plan.new_profiles.is_empty());
    let work = plan.spaces.iter().find_map(|space| match &space.into {
        plan::Target::New { name, profile, .. } if name == "Work" => Some(*profile),
        _ => None,
    });
    assert_eq!(work, Some(plan::ProfileRef::Existing(7)));
}

/// The view, as the first launch shows it: the browsers found, Arc's choices, the import, which
/// waits for the new profile, then what came in, and Done. Skip goes on without importing.
#[gpui::test]
fn the_view_imports_what_the_user_picks(cx: &mut TestAppContext) {
    let browser = Rc::new(Browser::new(None));
    let host: Rc<dyn Host> = browser.clone();
    let view = cx.new(|_| ImportView::new(host, true));
    let events = Rc::new(RefCell::new(Vec::new()));
    let _events = cx.update(|cx| {
        let events = events.clone();
        cx.subscribe(&view, move |_, event: &ImportEvent, _| {
            events.borrow_mut().push(*event)
        })
    });
    view.read_with(cx, |view, _| {
        let names: Vec<_> = view
            .sources
            .iter()
            .map(|source| source.browser.name)
            .collect();
        assert_eq!(names, ["Arc"]);
    });
    view.update(cx, |view, cx| {
        view.pick(0, cx);
        view.import(cx);
    });
    // The Work Space waits for its profile, empty.
    view.read_with(cx, |view, _| {
        assert!(matches!(view.step, super::Step::Running { .. }))
    });
    assert!(browser.spaces()[2].rows.is_empty());
    cx.update(|cx| browser.load(cx));
    view.read_with(cx, |view, _| {
        let super::Step::Done(said) = &view.step else {
            panic!()
        };
        assert_eq!(
            said.as_ref(),
            "2 Spaces, 6 pinned tabs and 3 favorites came in from Arc."
        );
    });
    assert_eq!(browser.spaces()[2].rows.len(), 1);
    view.update(cx, |view, cx| view.done(cx));
    cx.run_until_parked();
    assert_eq!(*events.borrow(), [ImportEvent::Done]);
    // Again, there's nothing new.
    view.update(cx, |view, cx| {
        view.pick(0, cx);
        view.import(cx);
    });
    view.read_with(cx, |view, _| {
        let super::Step::Done(said) = &view.step else {
            panic!()
        };
        assert_eq!(said.as_ref(), "Everything from Arc is here already.");
    });
}
