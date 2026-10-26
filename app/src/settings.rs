//! The Settings window: Arc's, a standard window of its own titled with the tab it shows, with the
//! tabs across its toolbar and each tab's settings in boxes of rows under them (docs/ux-spec.md ›
//! Settings). It opens on ⌘, and hides when closed, keeping its place and its tab until the
//! browser quits.

use std::ffi::CString;
use std::time::Instant;

use browser::log;
use browser::shell::{self, CancelName, Command, KeepName, MenuEntry};
use browser::text_input::TextInput;
use gpui::{
    AnyElement, App, Bounds, Context, Entity, FocusHandle, Focusable, FontWeight, Global,
    MouseButton, MouseDownEvent, Rgba, SharedString, Subscription, TitlebarOptions, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, div, point, prelude::*, px, rgb,
    rgba, size, svg,
};

use crate::{bc, ns_view, platform, show_menu};

actions!(settings, [CloseSettings]);

// Arc's, measured in its dark Settings window, but for the Shortcuts tab, which isn't captured.
const WIDTH: f32 = 684.;
const HEIGHT: f32 = 650.;
/// The title is centred in the bar over the toolbar.
const TITLE_BAR: f32 = 32.;
/// The toolbar's tabs are 55 wide, or as wide as a long name needs, 1 apart, and the one shown
/// sits on a plate 44 tall, 2 down.
const TOOLBAR: f32 = 56.;
const TAB: f32 = 55.;
const TAB_PADDING: f32 = 6.;
const PLATE_TOP: f32 = 2.;
const PLATE: f32 = 44.;
const ICON: f32 = 20.;
const ICON_TOP: f32 = 4.;
const LABEL_TOP: f32 = 3.;
/// Boxes of rows are 64 in from the window's sides, the first 20 under the toolbar. A row's text
/// and its control are 10 in from the box's edges, and 10 from its top and bottom, and the lines
/// between rows 8 in.
const SIDE: f32 = 64.;
const FIRST: f32 = 20.;
const PADDING: f32 = 10.;
const LINE_INSET: f32 = 8.;
const RADIUS: f32 = 6.;
/// A row's icon is a tile of colour 20 across with a white glyph, 8 before the text.
const TILE: f32 = 20.;
const TILE_GLYPH: f32 = 14.;
const TILE_RADIUS: f32 = 4.5;
const TILE_GAP: f32 = 8.;
/// A switch is 36 by 16, with a knob 21 by 13, 1.5 in, that slides across over 150 ms (ours).
const SWITCH_WIDTH: f32 = 36.;
const SWITCH_HEIGHT: f32 = 16.;
const KNOB_WIDTH: f32 = 21.;
const KNOB_HEIGHT: f32 = 13.;
const KNOB_INSET: f32 = 1.5;
const SLIDE: f32 = 0.15;
/// Pop-up buttons and buttons are 24 tall with their text 12 in; a pop-up button is 155 wide,
/// with its chevrons 10 from its right edge, and ends 22.5 short of the switches and buttons.
const CONTROL_HEIGHT: f32 = 24.;
const CONTROL_PADDING: f32 = 12.;
const CONTROL_RADIUS: f32 = 5.;
const POPUP_WIDTH: f32 = 155.;
const POPUP_END: f32 = 22.5;
const CHEVRONS: f32 = 14.;
/// Buttons under the boxes are 11 below them.
const BUTTONS_TOP: f32 = 11.;
/// Ours, for the Shortcuts tab: a header 8 over each box, 20 below the box before, and keys in
/// chips as the command bar shows them.
const HEADER_GAP: f32 = 8.;
const SECTION_GAP: f32 = 20.;
const CHIP_HEIGHT: f32 = 20.;
const CHIP_PADDING: f32 = 6.;
/// Ours, for the Profiles tab: rows as tall as a button's, so they're even with and without one.
const PROFILE_ROW: f32 = 40.;

const BAR: u32 = 0x282a2a;
const BAR_LINE: u32 = 0x303131;
const BACKGROUND: u32 = 0x202222;
const BOX: u32 = 0x232525;
const BOX_LINE: u32 = 0x353737;
const PLATE_FILL: u32 = 0x444747;
const PLATE_EDGE: u32 = 0x525656;
const TITLE: u32 = 0xa4a6a6;
const TAB_TEXT: u32 = 0x9e9f9f;
const TEXT: u32 = 0xc2c2c2;
const CONTROL: u32 = 0x313333;
const CONTROL_PRESSED: u32 = 0x404242;
const SWITCH_OFF: u32 = 0x383939;
const KNOB: u32 = 0xe1e2e2;
/// The knob over the accent colour, which tints it.
const KNOB_ON: u32 = 0xe1ebfe;
const HAPTICS_TILE: u32 = 0x3f8ff7;
const PROFILE_TILE: u32 = 0x8e8e93;
/// A row's detail, such as a profile's Spaces, in the toolbar's grey.
const DETAIL: u32 = 0x9e9f9f;
const CHIP: u32 = 0xffff_ff14;

/// What Arc offers for how long a Today tab waits unseen before it's archived, in seconds.
const ARCHIVE_AFTER: [(&str, f64); 4] = [
    ("12 hours (default)", 12. * 3600.),
    ("24 hours", 24. * 3600.),
    ("7 days", 7. * 24. * 3600.),
    ("30 days", 30. * 24. * 3600.),
];

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    General,
    Profiles,
    Shortcuts,
    Advanced,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::General, Tab::Profiles, Tab::Shortcuts, Tab::Advanced];

    fn name(self) -> &'static str {
        match self {
            Tab::General => "General",
            Tab::Profiles => "Profiles",
            Tab::Shortcuts => "Shortcuts",
            Tab::Advanced => "Advanced",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Tab::General => "icons/settings.svg",
            Tab::Profiles => "icons/user.svg",
            Tab::Shortcuts => "icons/keyboard.svg",
            Tab::Advanced => "icons/sliders-horizontal.svg",
        }
    }
}

/// The keys a row of the Shortcuts tab shows: a command's shortcut, or keys the app binds itself.
enum Keys {
    Of(Command),
    Text(&'static str),
}

/// The Shortcuts tab, in the menus' order and names.
const SHORTCUTS: &[(&str, &[(&str, Keys)])] = &[
    (
        "Tabs",
        &[
            ("New Tab", Keys::Of(Command::NewTab)),
            ("Open Command Bar", Keys::Of(Command::EditUrl)),
            ("Archive Tab", Keys::Of(Command::CloseTab)),
            ("Restore Last Closed Tab", Keys::Of(Command::ReopenClosedTab)),
            ("Next Tab", Keys::Of(Command::NextTab)),
            ("Previous Tab", Keys::Of(Command::PreviousTab)),
            ("Last Used Tab", Keys::Of(Command::RecentTab)),
            ("Tabs 1 to 8", Keys::Text("⌘1 – ⌘8")),
            ("Last Tab", Keys::Of(Command::SelectLastTab)),
            ("Pin/Unpin Tab", Keys::Of(Command::TogglePin)),
            ("Back to Pinned URL", Keys::Of(Command::BackToPinnedUrl)),
            ("Clear Today", Keys::Of(Command::Clear)),
            ("Copy URL", Keys::Of(Command::CopyUrl)),
            ("Copy URL as Markdown", Keys::Of(Command::CopyMarkdownLink)),
        ],
    ),
    (
        "Page",
        &[
            ("Go Back", Keys::Of(Command::Back)),
            ("Go Forward", Keys::Of(Command::Forward)),
            ("Refresh the Page", Keys::Of(Command::Reload)),
            ("Force Refresh the Page", Keys::Of(Command::ForceReload)),
            ("Stop Loading", Keys::Of(Command::Stop)),
            ("Find", Keys::Of(Command::Find)),
            ("Find Next", Keys::Of(Command::FindNext)),
            ("Find Previous", Keys::Of(Command::FindPrevious)),
            ("Jump to Selection", Keys::Of(Command::JumpToSelection)),
            ("Zoom In", Keys::Of(Command::ZoomIn)),
            ("Zoom Out", Keys::Of(Command::ZoomOut)),
            ("Actual Size", Keys::Of(Command::ActualSize)),
            ("Save Page As", Keys::Of(Command::SavePage)),
            ("Print", Keys::Of(Command::Print)),
        ],
    ),
    (
        "Spaces and Sidebar",
        &[
            ("Next Space", Keys::Of(Command::NextSpace)),
            ("Previous Space", Keys::Of(Command::PreviousSpace)),
            ("Spaces 1 to 9", Keys::Text("⌃1 – ⌃9")),
            ("Show/Hide Sidebar", Keys::Of(Command::ToggleSidebar)),
            ("View Library", Keys::Of(Command::ViewLibrary)),
            ("View Downloads", Keys::Of(Command::ViewDownloads)),
        ],
    ),
    (
        "Split View",
        &[
            ("Add Split View", Keys::Of(Command::AddSplit)),
            ("Close this Split Pane", Keys::Of(Command::CloseSplitPane)),
            ("Next Pane", Keys::Of(Command::NextPane)),
            ("Previous Pane", Keys::Of(Command::PreviousPane)),
        ],
    ),
    (
        "Developer",
        &[
            ("View Source", Keys::Of(Command::ViewSource)),
            ("Developer Tools", Keys::Of(Command::DevTools)),
            ("Inspect Elements", Keys::Of(Command::InspectElements)),
            ("JavaScript Console", Keys::Of(Command::JavaScriptConsole)),
        ],
    ),
    (
        "Browser",
        &[
            ("Settings", Keys::Of(Command::Settings)),
            // main.rs binds these.
            ("Hide Browser", Keys::Text("⌘H")),
            ("Hide Others", Keys::Text("⌥⌘H")),
            ("Minimize", Keys::Text("⌘M")),
            ("Quit Browser", Keys::Of(Command::Quit)),
        ],
    ),
];

pub struct Settings {
    tab: Tab,
    focus: FocusHandle,
    /// Which switch last changed, and when, for its knob's slide.
    switched: Option<(&'static str, Instant)>,
    /// Whether the archive's pop-up menu is open, when its button shows pressed.
    choosing: bool,
    /// The profile whose name is in a field for renaming.
    renaming: Option<Renaming>,
    /// Whether the field takes the keyboard once its row is drawn.
    focus_renaming: bool,
}

struct Renaming {
    profile: u64,
    name: Entity<TextInput>,
    /// Keeps the name once the field loses the keyboard, as the sidebar's renaming does.
    _blur: Option<Subscription>,
}

/// A profile as the Profiles tab lists it.
struct Profile {
    id: u64,
    name: SharedString,
    spaces: usize,
    /// As the core has it: not the one it started with, nor one a Space or a favorite is in.
    deletable: bool,
}

/// The profiles, the one the browser started with first.
fn profiles() -> Vec<Profile> {
    let in_space = |space| unsafe { bc::bc_space_profile(space) };
    let spaces: Vec<u64> = crate::ids(bc::bc_spaces).into_iter().map(in_space).collect();
    let of_tab = |tab| unsafe { bc::bc_tab_profile(tab) };
    let favorites: Vec<u64> = crate::ids(bc::bc_favorites).into_iter().map(of_tab).collect();
    (crate::ids(bc::bc_profiles).into_iter().enumerate())
        .map(|(index, id)| {
            let count = spaces.iter().filter(|&&each| each == id).count();
            Profile {
                id,
                name: crate::name(bc::bc_profile_name, id),
                spaces: count,
                deletable: index > 0 && count == 0 && !favorites.contains(&id),
            }
        })
        .collect()
}

struct SettingsWindow(WindowHandle<Settings>);

impl Global for SettingsWindow {}

/// Shows the Settings window, opening it the first time, or brings it to the front.
pub fn show(cx: &mut App) {
    if !cx.has_global::<SettingsWindow>() {
        open(cx);
    }
    let Some(window) = cx.try_global::<SettingsWindow>().map(|settings| settings.0) else {
        return;
    };
    window
        .update(cx, |settings, window, cx| {
            window.focus(&settings.focus, cx);
            cx.notify();
        })
        .ok();
    unsafe { platform::host_show_settings() };
}

fn open(cx: &mut App) {
    cx.bind_keys([gpui::KeyBinding::new("cmd-w", CloseSettings, Some("Settings"))]);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.), px(0.)),
            size(px(WIDTH), px(HEIGHT)),
        ))),
        // The UI draws the title and the toolbar under the window's buttons.
        titlebar: Some(TitlebarOptions {
            title: Some(Tab::General.name().into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::Normal,
        show: false,
        focus: false,
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };
    let window = cx.open_window(options, |window, cx| {
        cx.new(|cx| {
            // Closing it hides it, so it opens again where the user left it.
            window.on_window_should_close(cx, |_, cx| {
                hide(cx);
                false
            });
            Settings {
                tab: Tab::General,
                focus: cx.focus_handle(),
                switched: None,
                choosing: false,
                renaming: None,
                focus_renaming: false,
            }
        })
    });
    match window {
        Ok(window) => {
            if let Ok(view) = window.update(cx, |_, window, _| ns_view(window)) {
                unsafe { platform::host_add_settings(view) };
                cx.set_global(SettingsWindow(window));
            }
        }
        Err(error) => log!("couldn't open the Settings window: {error}"),
    }
}

/// Shows what changed in the core, such as a profile made, renamed or deleted, if it's open.
pub fn refresh(cx: &mut App) {
    if let Some(window) = cx.try_global::<SettingsWindow>().map(|settings| settings.0) {
        window.update(cx, |_, _, cx| cx.notify()).ok();
    }
}

fn hide(_: &mut App) {
    // check.sh waits for this line.
    log!("browser: settings hidden");
    unsafe { platform::host_hide_settings() };
}

impl Settings {
    fn show_tab(&mut self, tab: Tab, window: &mut Window, cx: &mut Context<Self>) {
        if tab == self.tab {
            return;
        }
        self.keep_name(window, cx);
        // check.sh waits for this line.
        log!("browser: settings tab {}", tab.name());
        self.tab = tab;
        window.set_window_title(tab.name());
        cx.notify();
    }

    /// The title, and the toolbar's tabs. Dragging either moves the window, as a toolbar does.
    fn bar(&self, accent: Rgba, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let tabs = Tab::ALL.map(|tab| {
            let shown = tab == self.tab;
            let color = if shown { accent } else { rgb(TAB_TEXT) };
            div()
                .id(tab.name())
                .min_w(px(TAB))
                .h(px(PLATE))
                .mt(px(PLATE_TOP))
                .px(px(TAB_PADDING))
                .flex()
                .flex_col()
                .items_center()
                .rounded(px(RADIUS))
                .when(shown, |plate| {
                    plate.bg(rgb(PLATE_FILL)).border(px(0.5)).border_color(rgb(PLATE_EDGE))
                })
                // A press here picks the tab, and doesn't move the window.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |settings, _, window, cx| {
                    settings.show_tab(tab, window, cx)
                }))
                .child(svg().path(tab.icon()).size(px(ICON)).mt(px(ICON_TOP)).text_color(color))
                .child(
                    div().mt(px(LABEL_TOP)).text_size(px(11.)).text_color(color).child(tab.name()),
                )
        });
        div()
            .flex_none()
            .h(px(TITLE_BAR + TOOLBAR))
            .bg(rgb(BAR))
            .border_b(px(0.5))
            .border_color(rgb(BAR_LINE))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .child(
                div()
                    .h(px(TITLE_BAR))
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(TITLE))
                    .child(self.tab.name()),
            )
            .child(div().h(px(TOOLBAR)).flex().justify_center().gap(px(1.)).children(tabs))
    }

    fn general(&self, accent: Rgba, cx: &mut Context<Self>) -> AnyElement {
        let after = unsafe { bc::bc_archive_after() };
        let chosen = ARCHIVE_AFTER.iter().position(|&(_, seconds)| seconds == after);
        let label: SharedString = match chosen {
            Some(index) => ARCHIVE_AFTER[index].0.into(),
            None => format!("{:.0} hours", after / 3600.).into(),
        };
        // It opens on a press, as a pop-up button does, and shows pressed while its menu is open.
        let popup = div()
            .flex_none()
            .w(px(POPUP_WIDTH))
            .mr(px(POPUP_END))
            .h(px(CONTROL_HEIGHT))
            .pl(px(CONTROL_PADDING))
            .pr(px(PADDING - (CHEVRONS - 8.) / 2.))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(CONTROL_RADIUS))
            .bg(rgb(if self.choosing { CONTROL_PRESSED } else { CONTROL }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |settings, _, _, cx| settings.choose_archive_after(chosen, cx)),
            )
            .child(label)
            .child(
                svg().path("icons/chevrons-up-down.svg").size(px(CHEVRONS)).text_color(rgb(TEXT)),
            );
        let warn = unsafe { bc::bc_warn_before_quitting() };
        let warn = self.toggle("warn before quitting", warn, set_warn_before_quitting, accent, cx);
        group([
            row(None, "Warn before quitting", warn),
            row(None, "Archive tabs after", popup.into_any_element()),
        ])
        .into_any_element()
    }

    /// The switch `name`, `on` now, that calls `set` with the other way when clicked.
    fn toggle(
        &self,
        name: &'static str,
        on: bool,
        set: fn(bool),
        accent: Rgba,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let slid = match self.switched {
            Some((switched, at)) if switched == name => {
                (at.elapsed().as_secs_f32() / SLIDE).min(1.)
            }
            _ => 1.,
        };
        div()
            .id(name)
            .on_click(cx.listener(move |settings, _, _, cx| {
                // check.sh waits for this line.
                log!("browser: {name} {}", if on { "off" } else { "on" });
                set(!on);
                settings.switched = Some((name, Instant::now()));
                cx.notify();
            }))
            .child(switch(on, slid, accent))
            .into_any_element()
    }

    /// Opens the archive's pop-up menu over its button, with the time chosen now, `chosen`, over
    /// the button's text, as a pop-up button does, and sets the time picked.
    fn choose_archive_after(&mut self, chosen: Option<usize>, cx: &mut Context<Self>) {
        let items: Vec<MenuEntry> = ARCHIVE_AFTER
            .iter()
            .enumerate()
            .map(|(index, &(title, _))| MenuEntry {
                title: title.into(),
                enabled: true,
                shortcut: None,
                parent: None,
                checked: Some(index) == chosen,
                swatch: None,
            })
            .collect();
        self.choosing = true;
        cx.notify();
        cx.spawn(async move |settings, cx| {
            let picked = show_menu(&items, popup_text(), platform::IN_SETTINGS, chosen);
            if let Some((title, seconds)) = picked.map(|index| ARCHIVE_AFTER[index]) {
                // check.sh waits for this line.
                log!("browser: archive tabs after {title}");
                unsafe { bc::bc_set_archive_after(seconds) };
            }
            settings
                .update(cx, |settings, cx| {
                    settings.choosing = false;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    /// A row for each profile, with how many Spaces are in it and Delete for one nothing is in, and
    /// New Profile under them. A double-click on a profile's name renames it, as on a Space's.
    fn profiles(&self, profiles: &[Profile], cx: &mut Context<Self>) -> AnyElement {
        let rows = profiles.iter().map(|profile| {
            let (id, name) = (profile.id, profile.name.clone());
            let label = match &self.renaming {
                Some(renaming) if renaming.profile == id => div()
                    .flex_1()
                    .min_w_0()
                    .key_context("Naming")
                    .on_action(cx.listener(|settings, _: &KeepName, window, cx| {
                        settings.keep_name(window, cx)
                    }))
                    .on_action(cx.listener(|settings, _: &CancelName, window, cx| {
                        settings.cancel_name(window, cx)
                    }))
                    .child(renaming.name.clone()),
                _ => div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |settings, event: &MouseDownEvent, _, cx| {
                            if event.click_count == 2 {
                                settings.rename(id, name.clone(), cx);
                            }
                        }),
                    )
                    .child(profile.name.clone()),
            };
            let spaces: SharedString = match profile.spaces {
                0 => "No Spaces".into(),
                1 => "1 Space".into(),
                count => format!("{count} Spaces").into(),
            };
            let name = profile.name.clone();
            let delete = profile.deletable.then(|| {
                button(("delete-profile", id), "Delete").on_click(
                    cx.listener(move |settings, _, _, cx| settings.delete(id, name.clone(), cx)),
                )
            });
            div()
                .h(px(PROFILE_ROW))
                .px(px(PADDING))
                .flex()
                .items_center()
                .child(tile("icons/user.svg", PROFILE_TILE))
                .child(label)
                .child(div().flex_none().ml(px(PADDING)).text_color(rgb(DETAIL)).child(spaces))
                .children(delete.map(|delete| delete.ml(px(PADDING))))
        });
        let new = button("new-profile", "New Profile")
            .on_click(cx.listener(|settings, _, _, cx| settings.new_profile(cx)));
        div()
            .child(group(rows))
            .child(div().mt(px(BUTTONS_TOP)).flex().child(new))
            .into_any_element()
    }

    /// Shows the profile's name in a field, all selected, to type a new one over.
    fn rename(&mut self, profile: u64, name: SharedString, cx: &mut Context<Self>) {
        let name = cx.new(|cx| {
            TextInput::new("Profile name", cx)
                .placeholder_color(rgb(DETAIL))
                .caret_color(rgb(TEXT))
                .with_text(name)
        });
        self.renaming = Some(Renaming {
            profile,
            name,
            _blur: None,
        });
        self.focus_renaming = true;
        cx.notify();
    }

    /// Names the profile what the field holds, unless it's empty, and closes the field.
    fn keep_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = self.renaming.take() else {
            return;
        };
        let name = renaming.name.read(cx).text().trim().to_string();
        if !name.is_empty() {
            let name = CString::new(name).unwrap_or_default();
            unsafe { bc::bc_profile_rename(renaming.profile, name.as_ptr()) };
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn cancel_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.renaming.take().is_some() {
            // check.sh waits for this line.
            log!("browser: rename cancelled");
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// Makes a profile, named as Chrome numbers them, with its name in a field once it's listed,
    /// to name it at once, as Finder's New Folder does.
    fn new_profile(&mut self, cx: &mut Context<Self>) {
        let names: Vec<SharedString> = profiles().into_iter().map(|profile| profile.name).collect();
        let name = shell::new_profile_name(&names);
        let c_name = CString::new(name.as_str()).unwrap_or_default();
        let profile = unsafe { bc::bc_profile_create(c_name.as_ptr()) };
        // check.sh waits for this line.
        log!("browser: create profile {profile} {name}");
        if profile != 0 {
            self.rename(profile, name.into(), cx);
        }
    }

    /// Deletes the profile once the user confirms, in an alert, as the window has no room for the
    /// browser window's dialog.
    fn delete(&mut self, profile: u64, name: SharedString, cx: &mut Context<Self>) {
        cx.spawn(async move |_, _| {
            let message = format!("Delete “{name}”?");
            let detail = "Its history, cookies and logins go with it.";
            if crate::ask(&message, detail, &["Delete", "Cancel"]) == 0 {
                unsafe { bc::bc_profile_delete(profile) };
            }
        })
        .detach();
    }

    fn advanced(&self, accent: Rgba, cx: &mut Context<Self>) -> AnyElement {
        let haptics = unsafe { bc::bc_haptics() };
        let haptics = self.toggle("haptics", haptics, set_haptics, accent, cx);
        let more = button("more-settings", "More Settings…").on_click(|_, _, cx| more_settings(cx));
        div()
            .child(group([row(
                Some(("icons/vibrate.svg", HAPTICS_TILE)),
                "Haptic feedback when swiping between Spaces",
                haptics,
            )]))
            .child(div().mt(px(BUTTONS_TOP)).flex().child(more))
            .into_any_element()
    }

    /// Gives the renaming field the keyboard once its profile is listed, which a new one is once
    /// Chrome has made it.
    fn focus_field(&mut self, profiles: &[Profile], window: &mut Window, cx: &mut Context<Self>) {
        let Some(renaming) = &mut self.renaming else {
            return;
        };
        if !profiles.iter().any(|profile| profile.id == renaming.profile)
            || !std::mem::take(&mut self.focus_renaming)
        {
            return;
        }
        let field = renaming.name.focus_handle(cx);
        window.focus(&field, cx);
        // check.sh waits for this line, which says the field has the keyboard.
        log!("browser: rename profile {}", renaming.profile);
        renaming._blur =
            Some(cx.on_blur(&field, window, |settings, window, cx| settings.keep_name(window, cx)));
    }

    fn shortcuts(&self) -> AnyElement {
        let sections = SHORTCUTS.iter().enumerate().map(|(index, (section, rows))| {
            let rows = rows.iter().map(|(name, keys)| {
                let keys: SharedString = match keys {
                    Keys::Of(command) => shell::shortcut_label(*command).unwrap_or_default(),
                    Keys::Text(text) => text.to_string(),
                }
                .into();
                row(None, name, chip(keys).into_any_element())
            });
            div()
                .when(index > 0, |section| section.mt(px(SECTION_GAP)))
                .child(
                    div()
                        .mb(px(HEADER_GAP))
                        .pl(px(PADDING))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(*section),
                )
                .child(group(rows))
        });
        div().children(sections).into_any_element()
    }
}

/// Where the archive's pop-up button's text starts in the window, in the first box's second row,
/// under the switch's.
fn popup_text() -> gpui::Point<gpui::Pixels> {
    let switch_row = PADDING + SWITCH_HEIGHT + PADDING + 1.;
    point(
        px(WIDTH - SIDE - PADDING - POPUP_END - POPUP_WIDTH + CONTROL_PADDING),
        px(TITLE_BAR + TOOLBAR + FIRST + 1. + switch_row + PADDING),
    )
}

fn set_warn_before_quitting(warn: bool) {
    unsafe { bc::bc_set_warn_before_quitting(warn) };
}

fn set_haptics(haptics: bool) {
    unsafe { bc::bc_set_haptics(haptics) };
}

/// Opens Chromium's settings in a new tab, for what this window doesn't offer yet, and goes to it.
fn more_settings(cx: &mut App) {
    // check.sh waits for this line.
    log!("browser: more settings");
    crate::open_tab("chrome://settings");
    hide(cx);
}

/// A box of `rows`, with lines between them.
fn group(rows: impl IntoIterator<Item = gpui::Div>) -> gpui::Div {
    let rows = rows.into_iter().enumerate().map(|(index, row)| {
        div().when(index > 0, |below| {
            below.child(div().mx(px(LINE_INSET)).h(px(1.)).bg(rgb(BOX_LINE)))
        })
        .child(row)
    });
    div()
        .flex()
        .flex_col()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(rgb(BOX_LINE))
        .bg(rgb(BOX))
        .children(rows)
}

/// A row: its icon if any, its name, and its control at the right.
fn row(icon: Option<(&'static str, u32)>, name: &'static str, control: AnyElement) -> gpui::Div {
    div()
        .px(px(PADDING))
        .py(px(PADDING))
        .flex()
        .items_center()
        .children(icon.map(|(glyph, color)| tile(glyph, color)))
        .child(div().flex_1().min_w_0().truncate().child(name))
        .child(div().flex_none().ml(px(PADDING)).child(control))
}

/// A row's icon: a white glyph on a tile of colour.
fn tile(glyph: &'static str, color: u32) -> gpui::Div {
    div()
        .flex_none()
        .size(px(TILE))
        .mr(px(TILE_GAP))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(TILE_RADIUS))
        .bg(rgb(color))
        .child(svg().path(glyph).size(px(TILE_GLYPH)).text_color(rgb(0xffffff)))
}

/// A push button, darker while pressed.
fn button(id: impl Into<gpui::ElementId>, title: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .h(px(CONTROL_HEIGHT))
        .px(px(CONTROL_PADDING))
        .flex()
        .items_center()
        .rounded(px(CONTROL_RADIUS))
        .bg(rgb(CONTROL))
        .active(|button| button.bg(rgb(CONTROL_PRESSED)))
        .child(title)
}

/// A switch, on or off, with its knob `slid` of the way to that side.
fn switch(on: bool, slid: f32, accent: Rgba) -> impl IntoElement {
    let travel = SWITCH_WIDTH - KNOB_WIDTH - 2. * KNOB_INSET;
    let way = if on { slid } else { 1. - slid };
    div()
        .relative()
        .w(px(SWITCH_WIDTH))
        .h(px(SWITCH_HEIGHT))
        .rounded_full()
        .bg(if on { accent } else { rgb(SWITCH_OFF) })
        .child(
            div()
                .absolute()
                .top(px((SWITCH_HEIGHT - KNOB_HEIGHT) / 2.))
                .left(px(KNOB_INSET + travel * way))
                .w(px(KNOB_WIDTH))
                .h(px(KNOB_HEIGHT))
                .rounded_full()
                .bg(rgb(if on { KNOB_ON } else { KNOB })),
        )
}

/// Keys as a chip, as the command bar shows a command's.
fn chip(keys: SharedString) -> impl IntoElement {
    div()
        .h(px(CHIP_HEIGHT))
        .px(px(CHIP_PADDING))
        .flex()
        .items_center()
        .rounded(px(4.))
        .bg(rgba(CHIP))
        .text_size(px(12.))
        .child(keys)
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let accent = rgb(unsafe { platform::host_accent_color() });
        if self.switched.is_some_and(|(_, at)| at.elapsed().as_secs_f32() < SLIDE) {
            window.request_animation_frame();
        }
        let content = match self.tab {
            Tab::General => self.general(accent, cx),
            Tab::Profiles => {
                let profiles = profiles();
                self.focus_field(&profiles, window, cx);
                self.profiles(&profiles, cx)
            }
            Tab::Shortcuts => self.shortcuts(),
            Tab::Advanced => self.advanced(accent, cx),
        };
        div()
            .key_context("Settings")
            .track_focus(&self.focus)
            .on_action(|_: &CloseSettings, _, cx| hide(cx))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BACKGROUND))
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(rgb(TEXT))
            .child(self.bar(accent, cx))
            .child(
                div()
                    .id("content")
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(SIDE))
                    .py(px(FIRST))
                    .child(content),
            )
    }
}
