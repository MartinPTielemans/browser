//! What an import of Arc's sidebar adds, worked out against what the browser has, so that running
//! it again adds nothing: a Space joins the Space here with its name and profile, a folder the one
//! with its name in the same place, and a tab or favorite whose URL is already there is skipped.

use super::arc::{ArcProfile, Node, Sidebar, Tab};
use crate::theme::Theme;

/// What the browser has, as far as an import needs to know.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Existing {
    /// The profiles, by id and name, the one the browser started with first.
    pub profiles: Vec<(u64, String)>,
    /// The user's Spaces, never an agent's.
    pub spaces: Vec<ExistingSpace>,
    /// Each favorite's profile and URL.
    pub favorites: Vec<(u64, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExistingSpace {
    pub id: u64,
    pub name: String,
    pub profile: u64,
    pub pinned: Vec<Row>,
}

/// A pinned row: a tab by the URL it's pinned at, or a folder with what's in it.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Tab(String),
    Folder {
        id: u64,
        name: String,
        rows: Vec<Row>,
    },
}

/// A profile an import puts things in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProfileRef {
    Existing(u64),
    /// The plan's `new_profiles` at this index, made by the import.
    New(usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub new_profiles: Vec<String>,
    pub spaces: Vec<SpacePlan>,
    /// Favorites to add, each with the profile it's for.
    pub favorites: Vec<(ProfileRef, Tab)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpacePlan {
    pub into: Target,
    /// What to add to its pinned tabs, at their ends.
    pub pinned: Vec<Add>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Existing(u64),
    New {
        name: String,
        profile: ProfileRef,
        icon: String,
        theme: Option<Theme>,
    },
}

/// A pinned tab to add, or a folder: one to make, or one that's here, and what goes in it.
#[derive(Clone, Debug, PartialEq)]
pub enum Add {
    Tab(Tab),
    Folder {
        existing: Option<u64>,
        name: String,
        adds: Vec<Add>,
    },
}

/// What the user picked to bring in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Picked {
    /// Spaces, with their pinned tabs and folders.
    pub spaces: bool,
    pub favorites: bool,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.new_profiles.is_empty() && self.spaces.is_empty() && self.favorites.is_empty()
    }

    /// The Spaces it makes, the pinned tabs and favorites it adds.
    pub fn counts(&self) -> (usize, usize, usize) {
        fn tabs(adds: &[Add]) -> usize {
            adds.iter()
                .map(|add| match add {
                    Add::Tab(_) => 1,
                    Add::Folder { adds, .. } => tabs(adds),
                })
                .sum()
        }
        let made = self
            .spaces
            .iter()
            .filter(|space| matches!(space.into, Target::New { .. }));
        let pinned = self.spaces.iter().map(|space| tabs(&space.pinned)).sum();
        (made.count(), pinned, self.favorites.len())
    }
}

/// What importing `sidebar` adds to `existing`. `names` are Arc's profiles' names by folder, from
/// its `Local State`. Arc's default profile is the one the browser started with, and each other
/// joins the profile here with its name, or a new one.
pub fn plan(
    sidebar: &Sidebar,
    names: &[(String, String)],
    existing: &Existing,
    picked: Picked,
) -> Plan {
    let mut plan = Plan::default();
    let start = existing.profiles.first().map_or(0, |(id, _)| *id);
    let resolve = |profile: &ArcProfile, plan: &mut Plan| -> ProfileRef {
        let ArcProfile::Custom(dir) = profile else {
            return ProfileRef::Existing(start);
        };
        let name = names
            .iter()
            .find(|(each, _)| each == dir)
            .map_or(dir.as_str(), |(_, name)| name);
        if let Some((id, _)) = existing.profiles.iter().find(|(_, each)| each == name) {
            return ProfileRef::Existing(*id);
        }
        let at = plan
            .new_profiles
            .iter()
            .position(|each| each == name)
            .unwrap_or_else(|| {
                plan.new_profiles.push(name.into());
                plan.new_profiles.len() - 1
            });
        ProfileRef::New(at)
    };
    if picked.spaces {
        for space in &sidebar.spaces {
            let profile = resolve(&space.profile, &mut plan);
            let here = existing.spaces.iter().find(|each| {
                each.name == space.name && profile == ProfileRef::Existing(each.profile)
            });
            // Two Arc Spaces of one name and profile become one here, as a rerun would merge them.
            let planned = plan.spaces.iter().position(|each| match &each.into {
                Target::New {
                    name,
                    profile: each,
                    ..
                } => *name == space.name && *each == profile,
                Target::Existing(id) => here.is_some_and(|here| here.id == *id),
            });
            let rows = here.map_or(&[][..], |here| &here.pinned[..]);
            let mut adds = Vec::new();
            add(&space.pinned, rows, &mut adds);
            match planned {
                Some(at) => merge(&mut plan.spaces[at].pinned, adds),
                None if here.is_some() && adds.is_empty() => {}
                None => plan.spaces.push(SpacePlan {
                    into: match here {
                        Some(here) => Target::Existing(here.id),
                        None => Target::New {
                            name: space.name.clone(),
                            profile,
                            icon: space.icon.clone(),
                            theme: space.theme,
                        },
                    },
                    pinned: adds,
                }),
            }
        }
    }
    if picked.favorites {
        for (profile, tabs) in &sidebar.favorites {
            let profile = resolve(profile, &mut plan);
            for tab in tabs {
                let here = existing
                    .favorites
                    .iter()
                    .any(|(each, url)| ProfileRef::Existing(*each) == profile && *url == tab.url);
                let planned = plan
                    .favorites
                    .iter()
                    .any(|(each, planned)| *each == profile && planned.url == tab.url);
                if !here && !planned {
                    plan.favorites.push((profile, tab.clone()));
                }
            }
        }
    }
    plan
}

/// Adds to `adds` what of `nodes` isn't among `rows`, the rows already in that place.
fn add(nodes: &[Node], rows: &[Row], adds: &mut Vec<Add>) {
    for node in nodes {
        match node {
            Node::Tab(tab) => {
                let here = rows
                    .iter()
                    .any(|row| matches!(row, Row::Tab(url) if *url == tab.url));
                let planned = adds
                    .iter()
                    .any(|each| matches!(each, Add::Tab(each) if each.url == tab.url));
                if !here && !planned {
                    adds.push(Add::Tab(tab.clone()));
                }
            }
            Node::Folder { name, children } => {
                let here = rows.iter().find_map(|row| match row {
                    Row::Folder {
                        id,
                        name: each,
                        rows,
                    } if each == name => Some((*id, &rows[..])),
                    _ => None,
                });
                let mut inner = Vec::new();
                add(children, here.map_or(&[], |(_, rows)| rows), &mut inner);
                let planned = adds.iter_mut().find_map(|each| match each {
                    Add::Folder {
                        name: each, adds, ..
                    } if each == name => Some(adds),
                    _ => None,
                });
                match (planned, here) {
                    (Some(planned), _) => merge(planned, inner),
                    // A folder that's here and gets nothing new is left as it is.
                    (None, Some(_)) if inner.is_empty() => {}
                    (None, here) => adds.push(Add::Folder {
                        existing: here.map(|(id, _)| id),
                        name: name.clone(),
                        adds: inner,
                    }),
                }
            }
        }
    }
}

/// Puts `more` into `adds`, skipping what's there already.
fn merge(adds: &mut Vec<Add>, more: Vec<Add>) {
    for each in more {
        match each {
            Add::Tab(tab) => {
                if !adds
                    .iter()
                    .any(|add| matches!(add, Add::Tab(each) if each.url == tab.url))
                {
                    adds.push(Add::Tab(tab));
                }
            }
            Add::Folder {
                existing,
                name,
                adds: inner,
            } => {
                let same = adds.iter_mut().find_map(|add| match add {
                    Add::Folder {
                        name: each, adds, ..
                    } if *each == name => Some(adds),
                    _ => None,
                });
                match same {
                    Some(same) => merge(same, inner),
                    None => adds.push(Add::Folder {
                        existing,
                        name,
                        adds: inner,
                    }),
                }
            }
        }
    }
}
