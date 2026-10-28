//! The browsers an import reads from, where each keeps its profiles on each system, and which of
//! them are on this machine. A table rather than code per system, so the tests find every
//! system's browsers in a made-up home folder.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// How a browser keeps its profiles, which decides the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Chrome's: a user data folder with `Local State` and a folder for each profile.
    Chromium,
    /// Firefox's: `profiles.ini` and the profile folders it names.
    Firefox,
    /// Safari's one profile, in `~/Library/Safari`.
    Safari,
}

/// The folders the browsers' paths start from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Base {
    /// `~/Library/Application Support` on the Mac.
    Support,
    /// `%LOCALAPPDATA%` on Windows.
    Local,
    /// `%APPDATA%` on Windows.
    Roaming,
    /// `~/.config`, or `$XDG_CONFIG_HOME`, on Linux.
    Config,
    /// The home folder.
    Home,
}

pub struct Browser {
    pub name: &'static str,
    pub family: Family,
    /// Where it keeps its profiles on each system it runs on. A `*` in a part matches any name
    /// starting with what comes before it, as the folder Windows gives a Store app does.
    paths: &'static [(Os, Base, &'static str)],
}

use Base::*;
use Family::*;
use Os::*;

/// The browsers, in the order they're listed when none has been used more recently than another.
pub const BROWSERS: &[Browser] = &[
    Browser {
        name: "Arc",
        family: Chromium,
        paths: &[
            (Mac, Support, "Arc/User Data"),
            (
                Windows,
                Local,
                "Packages/TheBrowserCompany.Arc*/LocalCache/Local/Arc/User Data",
            ),
        ],
    },
    Browser {
        name: "Chrome",
        family: Chromium,
        paths: &[
            (Mac, Support, "Google/Chrome"),
            (Windows, Local, "Google/Chrome/User Data"),
            (Linux, Config, "google-chrome"),
        ],
    },
    Browser {
        name: "Dia",
        family: Chromium,
        paths: &[(Mac, Support, "Dia/User Data")],
    },
    Browser {
        name: "Edge",
        family: Chromium,
        paths: &[
            (Mac, Support, "Microsoft Edge"),
            (Windows, Local, "Microsoft/Edge/User Data"),
            (Linux, Config, "microsoft-edge"),
        ],
    },
    Browser {
        name: "Brave",
        family: Chromium,
        paths: &[
            (Mac, Support, "BraveSoftware/Brave-Browser"),
            (Windows, Local, "BraveSoftware/Brave-Browser/User Data"),
            (Linux, Config, "BraveSoftware/Brave-Browser"),
        ],
    },
    Browser {
        name: "Vivaldi",
        family: Chromium,
        paths: &[
            (Mac, Support, "Vivaldi"),
            (Windows, Local, "Vivaldi/User Data"),
            (Linux, Config, "vivaldi"),
        ],
    },
    Browser {
        name: "Opera",
        family: Chromium,
        paths: &[
            (Mac, Support, "com.operasoftware.Opera"),
            (Windows, Roaming, "Opera Software/Opera Stable"),
            (Linux, Config, "opera"),
        ],
    },
    Browser {
        name: "Chromium",
        family: Chromium,
        paths: &[
            (Mac, Support, "Chromium"),
            (Windows, Local, "Chromium/User Data"),
            (Linux, Config, "chromium"),
        ],
    },
    Browser {
        name: "Helium",
        family: Chromium,
        paths: &[
            (Mac, Support, "net.imput.helium"),
            (Windows, Local, "imput/Helium/User Data"),
            (Linux, Config, "net.imput.helium"),
        ],
    },
    Browser {
        name: "Firefox",
        family: Firefox,
        paths: &[
            (Mac, Support, "Firefox"),
            (Windows, Roaming, "Mozilla/Firefox"),
            (Linux, Home, ".mozilla/firefox"),
        ],
    },
    Browser {
        name: "Zen",
        family: Firefox,
        paths: &[
            (Mac, Support, "zen"),
            (Windows, Roaming, "zen"),
            (Linux, Home, ".zen"),
        ],
    },
    Browser {
        name: "LibreWolf",
        family: Firefox,
        paths: &[
            (Mac, Support, "librewolf"),
            (Windows, Roaming, "librewolf"),
            (Linux, Home, ".librewolf"),
        ],
    },
    Browser {
        name: "Safari",
        family: Safari,
        paths: &[(Mac, Home, "Library/Safari")],
    },
];

/// The folders a system's paths start from: the user's, or the same layout under a made-up home
/// folder, which the tests and the check runs read in place of the user's.
#[derive(Clone, Debug, PartialEq)]
pub struct Roots {
    pub os: Os,
    home: PathBuf,
    local: PathBuf,
    roaming: PathBuf,
    config: PathBuf,
}

impl Roots {
    /// The user's folders on this system.
    pub fn system() -> Roots {
        let var = |name| std::env::var_os(name).map(PathBuf::from);
        let home = var("HOME")
            .or_else(|| var("USERPROFILE"))
            .unwrap_or_default();
        Roots {
            os: Os::current(),
            local: var("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData/Local")),
            roaming: var("APPDATA").unwrap_or_else(|| home.join("AppData/Roaming")),
            config: var("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config")),
            home,
        }
    }

    /// `os`'s folders, laid out under `home` as that system lays them out under the user's.
    pub fn under(home: &Path, os: Os) -> Roots {
        Roots {
            os,
            home: home.into(),
            local: home.join("AppData/Local"),
            roaming: home.join("AppData/Roaming"),
            config: home.join(".config"),
        }
    }

    fn base(&self, base: Base) -> PathBuf {
        match base {
            Support => self.home.join("Library/Application Support"),
            Local => self.local.clone(),
            Roaming => self.roaming.clone(),
            Config => self.config.clone(),
            Home => self.home.clone(),
        }
    }
}

/// A browser found on the machine, with its profiles.
#[derive(Clone, Debug)]
pub struct Source {
    pub browser: &'static Browser,
    /// The folder its profiles are in.
    pub dir: PathBuf,
    pub profiles: Vec<Profile>,
    /// When it was last used, as far as its files tell, for the list's order.
    pub used: Option<SystemTime>,
}

impl std::fmt::Debug for Browser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

impl Source {
    /// Arc's sidebar, beside its user data folder, where its Spaces, pinned tabs and favorites are.
    pub fn arc_sidebar(&self) -> Option<PathBuf> {
        let path = self.dir.parent()?.join("StorableSidebar.json");
        (self.browser.name == "Arc" && path.is_file()).then_some(path)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// Its folder's name, as the browser's own files name it, such as "Profile 1".
    pub dir: String,
    /// The name the user knows it by.
    pub name: String,
    pub path: PathBuf,
}

/// The browsers found under `roots`, the one used last first. Reads only the files that list the
/// profiles; what's in a profile waits until the user picks it.
pub fn find(roots: &Roots) -> Vec<Source> {
    let mut found: Vec<Source> = BROWSERS
        .iter()
        .flat_map(|browser| {
            let paths = browser.paths.iter().filter(|(os, ..)| *os == roots.os);
            paths
                .flat_map(move |&(_, base, path)| expand(&roots.base(base), path))
                .filter_map(move |dir| source(browser, dir))
        })
        .collect();
    // Stable, so browsers with no time keep the table's order after the others.
    found.sort_by(|a, b| b.used.cmp(&a.used));
    found
}

/// The folders `path` names under `base`, each `*` matched against what's there.
fn expand(base: &Path, path: &str) -> Vec<PathBuf> {
    let mut dirs = vec![base.to_path_buf()];
    for part in path.split('/') {
        dirs = match part.split_once('*') {
            None => dirs
                .into_iter()
                .map(|dir| dir.join(part))
                .filter(|dir| dir.exists())
                .collect(),
            Some((start, _)) => dirs
                .into_iter()
                .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(start))
                .map(|entry| entry.path())
                .collect(),
        };
    }
    dirs.retain(|dir| dir.is_dir());
    dirs
}

fn source(browser: &'static Browser, dir: PathBuf) -> Option<Source> {
    let profiles = match browser.family {
        Chromium => chromium_profiles(&dir),
        Firefox => firefox_profiles(&dir),
        Safari => vec![Profile {
            dir: String::new(),
            name: browser.name.into(),
            path: dir.clone(),
        }],
    };
    if profiles.is_empty() {
        return None;
    }
    // The files each family writes as it's used.
    let marks: &[&str] = match browser.family {
        Chromium => &["History", "Preferences"],
        Firefox => &["places.sqlite"],
        Safari => &["History.db"],
    };
    let mut files: Vec<PathBuf> = profiles
        .iter()
        .flat_map(|profile| marks.iter().map(|mark| profile.path.join(mark)))
        .collect();
    let mut source = Source {
        browser,
        dir,
        profiles,
        used: None,
    };
    files.extend(source.arc_sidebar());
    source.used = files
        .iter()
        .filter_map(|file| file.metadata().ok()?.modified().ok())
        .max();
    Some(source)
}

/// A Chromium user data folder's profiles: those `Local State` lists that are there, by the names
/// it gives them, else `Default`. Opera keeps its one profile in the folder itself.
fn chromium_profiles(dir: &Path) -> Vec<Profile> {
    let state = std::fs::read_to_string(dir.join("Local State")).unwrap_or_default();
    let mut named = profile_names(&state);
    if named.is_empty() {
        named.push(("Default".into(), "Default".into()));
    }
    let mut profiles: Vec<Profile> = named
        .into_iter()
        .map(|(folder, name)| Profile {
            path: dir.join(&folder),
            dir: folder,
            name,
        })
        .filter(|profile| profile.path.join("Preferences").is_file())
        .collect();
    if profiles.is_empty() && dir.join("Preferences").is_file() {
        let name = "Default".to_string();
        profiles.push(Profile {
            dir: String::new(),
            name,
            path: dir.to_path_buf(),
        });
    }
    profiles
}

/// The profiles a Chromium `Local State` lists, as their folders and names, in the order it
/// keeps them, which is the order they were made.
pub fn profile_names(local_state: &str) -> Vec<(String, String)> {
    let Ok(state) = serde_json::from_str::<serde_json::Value>(local_state) else {
        return Vec::new();
    };
    let Some(cache) = state
        .pointer("/profile/info_cache")
        .and_then(|cache| cache.as_object())
    else {
        return Vec::new();
    };
    let mut profiles: Vec<(String, String)> = cache
        .iter()
        .map(|(dir, info)| {
            let name = info
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or(dir);
            (dir.clone(), name.to_string())
        })
        .collect();
    // serde_json's map is sorted by key, which puts "Profile 10" before "Profile 2".
    profiles.sort_by_key(|(dir, _)| (dir != "Default", dir.len(), dir.clone()));
    profiles
}

/// A Firefox folder's profiles, as `profiles.ini` names them, that are there.
fn firefox_profiles(dir: &Path) -> Vec<Profile> {
    let ini = std::fs::read_to_string(dir.join("profiles.ini")).unwrap_or_default();
    let mut profiles = Vec::new();
    let (mut name, mut path, mut relative) = (None::<String>, None::<String>, true);
    let mut done = |name: &mut Option<String>, path: &mut Option<String>, relative: bool| {
        if let Some(folder) = path.take() {
            let full = if relative {
                dir.join(&folder)
            } else {
                PathBuf::from(&folder)
            };
            let name = name.take().unwrap_or_else(|| folder.clone());
            if full.is_dir() {
                profiles.push(Profile {
                    dir: folder,
                    name,
                    path: full,
                });
            }
        }
        *name = None;
    };
    for line in ini.lines().map(str::trim) {
        if line.starts_with('[') {
            done(&mut name, &mut path, relative);
            relative = true;
        } else if let Some((key, value)) = line.split_once('=') {
            match key {
                "Name" => name = Some(value.into()),
                "Path" => path = Some(value.into()),
                "IsRelative" => relative = value != "0",
                _ => {}
            }
        }
    }
    done(&mut name, &mut path, relative);
    profiles
}
