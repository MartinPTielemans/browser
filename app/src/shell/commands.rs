//! The browser's commands: their names, their keys and where the command bar offers
//! them, and what each does, run from a key, a menu, the command bar or the page.

use gpui::{Action, App, Context, Global, KeyBinding, MenuItem, SharedString, WeakEntity, actions};

use super::{COPIED_TIME, Section, Shell, Undo, clean_link, extensions};

/// A browser command, from a key or the sidebar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Command {
    NewTab,
    EditUrl,
    CloseTab,
    /// Reopens the Today tab closed last.
    ReopenClosedTab,
    NextTab,
    PreviousTab,
    /// The tab shown before this one, and on while ⌃ is held.
    RecentTab,
    /// The same walk, backwards.
    RecentTabBack,
    /// The tab at this index in sidebar order.
    SelectTab(usize),
    SelectLastTab,
    /// Pins the selected tab, or unpins it.
    TogglePin,
    /// Makes a folder at the bottom of the pinned tabs, and names it.
    NewFolder,
    /// Closes every folder of the Space shown.
    CollapseFolders,
    /// Loads the selected pinned tab's pinned URL.
    BackToPinnedUrl,
    /// Mutes the selected tab, or unmutes it.
    ToggleMute,
    /// Closes the Today tabs of the Space shown, but the one shown.
    Clear,
    /// Brings back the tabs the last Clear closed, or the Space deleted last, whichever was last.
    Undo,
    /// Copies the selected tab's URL.
    CopyUrl,
    /// Copies a Markdown link to the selected tab: its title, then its URL.
    CopyMarkdownLink,
    Find,
    FindNext,
    FindPrevious,
    /// Scrolls the page to its selection.
    JumpToSelection,
    Back,
    Forward,
    Reload,
    /// Reloads without the cache.
    ForceReload,
    /// Deletes the page's site's cache, then reloads without the cache.
    ClearCacheAndRefresh,
    /// Deletes the page's site's cookies, then reloads.
    ClearCookiesAndRefresh,
    Stop,
    /// Zooms the page's site to 100 %.
    ActualSize,
    ZoomIn,
    ZoomOut,
    /// Opens the page's source in a tab under it.
    ViewSource,
    /// Asks where to save the page, and saves it.
    SavePage,
    /// Shows print preview for the page.
    Print,
    /// Opens the page's DevTools, or closes them.
    DevTools,
    /// Opens the page's DevTools picking an element.
    InspectElements,
    /// Opens the page's DevTools at the console, or closes them there.
    JavaScriptConsole,
    /// Opens Chrome's Task Manager.
    TaskManager,
    /// Opens Chrome's extensions page in a new tab.
    ManageExtensions,
    NewSpace,
    /// Shows the Space shown with its name in a field.
    RenameSpace,
    /// The Space at this index in the switcher.
    SelectSpace(usize),
    NextSpace,
    PreviousSpace,
    /// Hides the docked sidebar, or shows it.
    ToggleSidebar,
    /// Opens the Library, or closes it.
    ViewLibrary,
    /// Opens the Library on the archive.
    ViewArchive,
    ClearArchive,
    /// Opens the Library on the downloads, or closes it there.
    ViewDownloads,
    /// Opens the command bar over a new pane beside the pages shown, for a split view.
    AddSplit,
    /// Closes the split view's focused page.
    CloseSplitPane,
    /// Takes the split view's focused page out of it, to show alone.
    SeparatePane,
    /// Takes every page of the split view out of it, and shows the focused one alone.
    SeparateSplit,
    /// Puts the split view's pages in the opposite order.
    FlipSplit,
    /// Stacks the split view's pages top to bottom, or puts them back side by side.
    ChangeOrientation,
    /// Swaps the split view's focused page with the one on its left, or on its right.
    MovePaneLeft,
    MovePaneRight,
    /// Focuses the split view's page right of the focused one, or left of it.
    NextPane,
    PreviousPane,
    /// Opens Chrome's history page in a new tab.
    ViewHistory,
    /// Opens the Settings window, or brings it to the front.
    Settings,
    /// Opens the Settings window on its Shortcuts tab.
    KeyboardShortcuts,
    Quit,
}

/// The browser's shortcuts (docs/ux-spec.md › Keyboard): the keystroke, with modifiers in the order
/// ctrl, alt, shift, cmd, its command, and whether it's reserved. As in Chrome, reserved ones reach
/// the browser before the page sees them, and the others only if the page doesn't keep them.
pub(super) const SHORTCUTS: &[(&str, Command, bool)] = &[
    ("cmd-t", Command::NewTab, true),
    ("cmd-w", Command::CloseTab, true),
    ("shift-cmd-t", Command::ReopenClosedTab, true),
    ("cmd-q", Command::Quit, true),
    ("cmd-,", Command::Settings, false),
    ("alt-cmd-down", Command::NextTab, true),
    ("alt-cmd-up", Command::PreviousTab, true),
    ("ctrl-tab", Command::RecentTab, true),
    ("ctrl-shift-tab", Command::RecentTabBack, true),
    ("alt-cmd-right", Command::NextSpace, true),
    ("alt-cmd-left", Command::PreviousSpace, true),
    ("cmd-l", Command::EditUrl, false),
    ("cmd-d", Command::TogglePin, false),
    ("cmd-enter", Command::BackToPinnedUrl, false),
    ("shift-cmd-k", Command::Clear, false),
    ("cmd-z", Command::Undo, false),
    ("shift-cmd-c", Command::CopyUrl, false),
    ("alt-shift-cmd-c", Command::CopyMarkdownLink, false),
    ("cmd-f", Command::Find, false),
    ("cmd-g", Command::FindNext, false),
    ("shift-cmd-g", Command::FindPrevious, false),
    ("cmd-j", Command::JumpToSelection, false),
    ("cmd-1", Command::SelectTab(0), false),
    ("cmd-2", Command::SelectTab(1), false),
    ("cmd-3", Command::SelectTab(2), false),
    ("cmd-4", Command::SelectTab(3), false),
    ("cmd-5", Command::SelectTab(4), false),
    ("cmd-6", Command::SelectTab(5), false),
    ("cmd-7", Command::SelectTab(6), false),
    ("cmd-8", Command::SelectTab(7), false),
    ("cmd-9", Command::SelectLastTab, false),
    ("cmd-[", Command::Back, false),
    ("cmd-]", Command::Forward, false),
    ("cmd-left", Command::Back, false),
    ("cmd-right", Command::Forward, false),
    ("cmd-r", Command::Reload, false),
    ("shift-cmd-r", Command::ForceReload, false),
    ("cmd-.", Command::Stop, false),
    ("cmd-0", Command::ActualSize, false),
    // The first is the one the menu shows. ⌘+ is ⇧⌘= on most layouts, which a page names either
    // way, and ⌘= is Chrome's hidden second ⌘+.
    ("cmd-+", Command::ZoomIn, false),
    ("cmd-=", Command::ZoomIn, false),
    ("shift-cmd-=", Command::ZoomIn, false),
    ("shift-cmd-+", Command::ZoomIn, false),
    ("cmd--", Command::ZoomOut, false),
    ("alt-cmd-u", Command::ViewSource, false),
    ("shift-cmd-s", Command::SavePage, false),
    ("cmd-p", Command::Print, false),
    ("alt-cmd-i", Command::DevTools, false),
    ("f12", Command::DevTools, false),
    ("alt-cmd-c", Command::InspectElements, false),
    ("alt-cmd-j", Command::JavaScriptConsole, false),
    ("cmd-s", Command::ToggleSidebar, false),
    ("shift-cmd-l", Command::ViewLibrary, false),
    ("shift-cmd-j", Command::ViewDownloads, false),
    ("cmd-y", Command::ViewHistory, false),
    // ⌃+ is ⌃⇧= on most layouts, as ⌘+ is.
    ("ctrl-+", Command::AddSplit, false),
    ("ctrl-shift-=", Command::AddSplit, false),
    ("ctrl-shift-+", Command::AddSplit, false),
    ("ctrl-shift--", Command::CloseSplitPane, false),
    ("ctrl-shift-_", Command::CloseSplitPane, false),
    ("ctrl-shift-]", Command::NextPane, false),
    ("ctrl-shift-}", Command::NextPane, false),
    ("ctrl-shift-[", Command::PreviousPane, false),
    ("ctrl-shift-{", Command::PreviousPane, false),
    ("escape", Command::Stop, false),
    ("ctrl-1", Command::SelectSpace(0), false),
    ("ctrl-2", Command::SelectSpace(1), false),
    ("ctrl-3", Command::SelectSpace(2), false),
    ("ctrl-4", Command::SelectSpace(3), false),
    ("ctrl-5", Command::SelectSpace(4), false),
    ("ctrl-6", Command::SelectSpace(5), false),
    ("ctrl-7", Command::SelectSpace(6), false),
    ("ctrl-8", Command::SelectSpace(7), false),
    ("ctrl-9", Command::SelectSpace(8), false),
];

/// The commands the command bar finds by name: Arc's bar's names where they're known, else its
/// menus' (docs/ux-spec.md › Command bar › Rows).
const BAR_COMMANDS: &[(&str, Command)] = &[
    ("Clear Today", Command::Clear),
    ("Clear Cache and Refresh", Command::ClearCacheAndRefresh),
    ("Clear Archive", Command::ClearArchive),
    ("Clear Cookies and Refresh", Command::ClearCookiesAndRefresh),
    ("Collapse All Folders", Command::CollapseFolders),
    ("New Folder", Command::NewFolder),
    ("Pin/Unpin Tab", Command::TogglePin),
    ("Back to Pinned URL", Command::BackToPinnedUrl),
    ("Mute/Unmute Tab", Command::ToggleMute),
    ("Archive Tab", Command::CloseTab),
    ("Restore Last Closed Tab", Command::ReopenClosedTab),
    ("Copy URL", Command::CopyUrl),
    ("Copy URL as Markdown", Command::CopyMarkdownLink),
    ("Find", Command::Find),
    ("Go Back", Command::Back),
    ("Go Forward", Command::Forward),
    ("Refresh the Page", Command::Reload),
    ("Force Refresh the Page", Command::ForceReload),
    ("Stop Loading", Command::Stop),
    ("Zoom In", Command::ZoomIn),
    ("Zoom Out", Command::ZoomOut),
    ("Zoom to Actual Size", Command::ActualSize),
    ("Close this Split Pane", Command::CloseSplitPane),
    ("Separate Page from Split View", Command::SeparatePane),
    // Dia's (docs/ux-spec.md › Split view).
    ("Flip Split View", Command::FlipSplit),
    ("Change Orientation", Command::ChangeOrientation),
    ("Move Pane Left", Command::MovePaneLeft),
    ("Move Pane Right", Command::MovePaneRight),
    ("Separate All Pages", Command::SeparateSplit),
    ("Show/Hide Sidebar", Command::ToggleSidebar),
    ("Save Page As", Command::SavePage),
    ("Print", Command::Print),
    ("View Source", Command::ViewSource),
    ("Developer Tools", Command::DevTools),
    ("Inspect Elements", Command::InspectElements),
    ("JavaScript Console", Command::JavaScriptConsole),
    ("Open Task Manager", Command::TaskManager),
    ("Manage Extensions", Command::ManageExtensions),
    ("New Space", Command::NewSpace),
    ("Rename Space", Command::RenameSpace),
    ("Next Space", Command::NextSpace),
    ("Previous Space", Command::PreviousSpace),
    ("Next Tab", Command::NextTab),
    ("Previous Tab", Command::PreviousTab),
    ("View Library", Command::ViewLibrary),
    ("View Archive", Command::ViewArchive),
    ("View Downloads", Command::ViewDownloads),
    ("View History", Command::ViewHistory),
    ("Settings", Command::Settings),
    ("Essential Keyboard Shortcuts", Command::KeyboardShortcuts),
];

/// The most commands the command bar lists at once.
const BAR_COMMANDS_SHOWN: usize = 4;

/// A command the command bar offers for what's typed.
pub struct BarCommand {
    pub name: SharedString,
    pub command: Command,
    /// The shortcut that runs it, as a menu shows it.
    pub shortcut: Option<SharedString>,
    /// Whether its name starts with what's typed, three letters or more, which puts it above
    /// the core's suggestions.
    pub strong: bool,
}

/// The commands that do something now whose names match `typed`, best first, as Arc's bar
/// matches: the name starting with it, then a word of it, then its letters in order from a word's
/// start. Nothing for under two letters.
pub fn bar_commands(typed: &str, cx: &App) -> Vec<BarCommand> {
    let typed = typed.trim().to_lowercase();
    let shell = cx.try_global::<ShellHandle>().and_then(|handle| handle.0.upgrade());
    let Some(shell) = shell.filter(|_| typed.chars().count() >= 2) else {
        return Vec::new();
    };
    let shell = shell.read(cx);
    let mut found: Vec<(u8, BarCommand)> = BAR_COMMANDS
        .iter()
        .filter(|&&(_, command)| shell.offers(command))
        .filter_map(|&(name, command)| {
            let score = match_score(name, &typed)?;
            let shortcut = SHORTCUTS.iter().find(|&&(_, each, _)| each == command);
            Some((
                score,
                BarCommand {
                    name: name.into(),
                    command,
                    shortcut: shortcut.map(|&(key, ..)| keystroke_label(key).into()),
                    strong: score == 3 && typed.chars().count() >= 3,
                },
            ))
        })
        .collect();
    found.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    found.into_iter().take(BAR_COMMANDS_SHOWN).map(|(_, command)| command).collect()
}

/// How well `name` matches `typed`, which is lowercase: 3 if the name starts with it, 2 if a word
/// of it does, 1 if its letters are in it in order from a word's start, else none.
fn match_score(name: &str, typed: &str) -> Option<u8> {
    let name = name.to_lowercase();
    let words: Vec<&str> = name
        .char_indices()
        .filter(|&(at, _)| at == 0 || name[..at].ends_with([' ', '/']))
        .map(|(at, _)| &name[at..])
        .collect();
    let first = typed.chars().next()?;
    let in_order = |word: &str| {
        let mut letters = word.chars();
        typed.chars().all(|letter| letters.any(|other| other == letter))
    };
    if name.starts_with(typed) {
        Some(3)
    } else if words.iter().any(|word| word.starts_with(typed)) {
        Some(2)
    } else {
        words.iter().any(|word| word.starts_with(first) && in_order(word)).then_some(1)
    }
}

/// How a menu shows the first shortcut that runs `command`, if one does.
pub fn shortcut_label(command: Command) -> Option<String> {
    let shortcut = SHORTCUTS.iter().find(|&&(_, each, _)| each == command);
    shortcut.map(|&(key, ..)| keystroke_label(key))
}

/// How a menu shows `keystroke`, as SHORTCUTS writes it: ⇧⌘K for shift-cmd-k.
fn keystroke_label(keystroke: &str) -> String {
    let mut rest = keystroke;
    let mut label = String::new();
    for (modifier, glyph) in [("ctrl-", "⌃"), ("alt-", "⌥"), ("shift-", "⇧"), ("cmd-", "⌘")] {
        if let Some(after) = rest.strip_prefix(modifier) {
            label.push_str(glyph);
            rest = after;
        }
    }
    let key = match rest {
        "enter" => "↩".to_string(),
        "escape" => "⎋".to_string(),
        "up" => "↑".to_string(),
        "down" => "↓".to_string(),
        "left" => "←".to_string(),
        "right" => "→".to_string(),
        "tab" => "⇥".to_string(),
        key => key.to_uppercase(),
    };
    label + &key
}

/// Copies `text`, a sum's result chosen in the command bar, once the bar has gone, and says so in
/// the toast.
pub fn copy_from_bar(text: &str, cx: &mut App) {
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell
            .update(cx, |shell, cx| {
                shell.command_bar_dismissed(cx);
                shell.copy(text, &format!("Copied {text}"), cx);
            })
            .ok();
    }
}

/// Runs `command`, whose row in the command bar was chosen, once the bar has gone.
pub fn run_from_bar(command: Command, cx: &mut App) {
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell
            .update(cx, |shell, cx| {
                shell.command_bar_dismissed(cx);
                shell.run(command, "command bar", cx);
            })
            .ok();
    }
}

impl Command {
    /// The command `keystroke` runs, if it's a shortcut that is `reserved` or isn't.
    pub fn for_keystroke(keystroke: &str, reserved: bool) -> Option<Command> {
        SHORTCUTS
            .iter()
            .find(|(key, _, is_reserved)| *key == keystroke && *is_reserved == reserved)
            .map(|&(_, command, _)| command)
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Command::NewTab => "new-tab",
            Command::EditUrl => "edit-url",
            Command::CloseTab => "close-tab",
            Command::ReopenClosedTab => "reopen-closed-tab",
            Command::NextTab => "next-tab",
            Command::PreviousTab => "previous-tab",
            Command::RecentTab => "recent-tab",
            Command::RecentTabBack => "recent-tab-back",
            Command::SelectTab(_) => "select-tab",
            Command::SelectLastTab => "select-last-tab",
            Command::TogglePin => "toggle-pin",
            Command::NewFolder => "new-folder",
            Command::CollapseFolders => "collapse-folders",
            Command::BackToPinnedUrl => "back-to-pinned-url",
            Command::ToggleMute => "toggle-mute",
            Command::Clear => "clear",
            Command::Undo => "undo",
            Command::CopyUrl => "copy-url",
            Command::CopyMarkdownLink => "copy-markdown-link",
            Command::Find => "find",
            Command::FindNext => "find-next",
            Command::FindPrevious => "find-previous",
            Command::JumpToSelection => "jump-to-selection",
            Command::Back => "back",
            Command::Forward => "forward",
            Command::Reload => "reload",
            Command::ForceReload => "force-reload",
            Command::ClearCacheAndRefresh => "clear-cache-and-refresh",
            Command::ClearCookiesAndRefresh => "clear-cookies-and-refresh",
            Command::Stop => "stop",
            Command::ActualSize => "actual-size",
            Command::ZoomIn => "zoom-in",
            Command::ZoomOut => "zoom-out",
            Command::ViewSource => "view-source",
            Command::SavePage => "save-page",
            Command::Print => "print",
            Command::DevTools => "devtools",
            Command::InspectElements => "inspect-elements",
            Command::JavaScriptConsole => "javascript-console",
            Command::TaskManager => "task-manager",
            Command::ManageExtensions => "manage-extensions",
            Command::NewSpace => "new-space",
            Command::RenameSpace => "rename-space",
            Command::SelectSpace(_) => "select-space",
            Command::NextSpace => "next-space",
            Command::PreviousSpace => "previous-space",
            Command::ToggleSidebar => "toggle-sidebar",
            Command::ViewLibrary => "view-library",
            Command::ViewArchive => "view-archive",
            Command::ClearArchive => "clear-archive",
            Command::ViewDownloads => "view-downloads",
            Command::AddSplit => "add-split",
            Command::CloseSplitPane => "close-split-pane",
            Command::SeparatePane => "separate-pane",
            Command::SeparateSplit => "separate-split",
            Command::FlipSplit => "flip-split",
            Command::ChangeOrientation => "change-orientation",
            Command::MovePaneLeft => "move-pane-left",
            Command::MovePaneRight => "move-pane-right",
            Command::NextPane => "next-pane",
            Command::PreviousPane => "previous-pane",
            Command::ViewHistory => "view-history",
            Command::Settings => "settings",
            Command::KeyboardShortcuts => "keyboard-shortcuts",
            Command::Quit => "quit",
        }
    }
}

#[derive(Clone, PartialEq, Action)]
#[action(namespace = browser, no_json)]
struct Run(pub(super) Command);

actions!(space_form, [CreateSpace]);
actions!(space_name, [KeepName, CancelName]);

pub(super) struct ShellHandle(pub(super) WeakEntity<Shell>);

impl Global for ShellHandle {}

/// The key context at the root of the browser's windows, where its shortcuts apply. GPUI ranks a
/// binding with no context above every other, so without one the shortcuts would take keys a
/// focused field binds for itself, such as escape and ⌘←.
pub const KEY_CONTEXT: &str = "Browser";

/// Binds the shortcuts for while the UI has the keyboard. The browser hands them over itself while
/// the page has it.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys(
        SHORTCUTS
            .iter()
            .map(|&(key, command, _)| KeyBinding::new(key, Run(command), Some(KEY_CONTEXT))),
    );
    cx.bind_keys([
        KeyBinding::new("enter", CreateSpace, Some("SpaceForm")),
        KeyBinding::new("enter", KeepName, Some("Naming")),
        KeyBinding::new("escape", CancelName, Some("Naming")),
    ]);
    cx.on_action(|run: &Run, cx| {
        // The menu's keys reach it from the dialog's window too, and the dialog has to be
        // answered first, as an alert's does.
        if crate::dialog::asking(cx) {
            return;
        }
        if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
            shell
                .update(cx, |shell, cx| shell.run(run.0, "ui", cx))
                .ok();
        }
    });
    extensions::bind(cx);
}

/// The command bar is about to show a page in the pane being added, if one is: the pages shown take
/// back its room first, so the split view shows at its place at once.
pub fn pane_added(cx: &mut App) {
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell
            .update(cx, |shell, cx| {
                if shell.adding_pane.take().is_some() {
                    shell.lay_out_page(cx);
                    cx.notify();
                }
            })
            .ok();
    }
}

/// The tab the window shows, or 0.
pub fn shown_tab(cx: &App) -> u64 {
    let shell = cx.try_global::<ShellHandle>().and_then(|handle| handle.0.upgrade());
    shell.map_or(0, |shell| shell.read(cx).selected)
}

/// A menu item that runs `command`, showing its shortcut.
pub fn menu_item(name: impl Into<SharedString>, command: Command) -> MenuItem {
    MenuItem::action(name, Run(command))
}

impl Shell {
    /// Runs a browser command and logs it. `from` says which side had the key: GPUI's bindings
    /// ("ui"), the browser handing it back ("page"), or the sidebar.
    pub fn run(&mut self, command: Command, from: &str, cx: &mut Context<Self>) {
        // check.sh waits for these lines.
        log!("browser: command {} via {from}", command.name());
        let rows = self.sidebar_rows();
        let position = rows.iter().position(|&row| row == self.row_of(self.selected));
        let count = rows.len();
        let space = self.spaces.iter().position(|space| space.id == self.space);
        // Next and previous wrap around for tabs, as in Chrome, and stop at the ends for Spaces, as
        // in Arc.
        // Escape puts back a dragged row before it stops the page (docs/ux-spec.md › Tab list).
        if matches!(command, Command::Stop) && self.cancel_drag(cx) {
            return;
        }
        // The command bar opens over the page, and the reveal's panel makes way for it.
        if matches!(command, Command::NewTab | Command::EditUrl | Command::AddSplit) {
            self.set_revealed(false, cx);
        }
        let target = match command {
            Command::NewTab => {
                self.set_new_tab(true, cx);
                return self.browser.command_bar(None, None, cx);
            }
            Command::EditUrl => {
                self.set_new_tab(false, cx);
                let editing = (self.selected != 0).then(|| (self.selected, self.url(self.selected)));
                return self.browser.command_bar(editing, self.url_field_bounds(), cx);
            }
            Command::CloseTab => {
                // From a window of Chrome's, such as its Task Manager, ⌘W closes that window, as in
                // Chrome, and leaves the tab alone.
                if !self.browser.close_other_window(cx) && self.selected != 0 {
                    self.browser.close(self.selected, false, cx);
                }
                return;
            }
            Command::TogglePin => {
                if self.selected != 0 {
                    let pinned = self.pinned().contains(&self.selected);
                    self.browser.set_pinned(self.selected, !pinned, cx);
                }
                return;
            }
            Command::NewFolder => return self.new_folder(cx),
            Command::CollapseFolders => return self.collapse_folders(cx),
            Command::BackToPinnedUrl => return self.browser.back_to_pinned_url(self.selected, cx),
            Command::ToggleMute => return self.toggle_mute(self.selected, cx),
            Command::Clear => return self.clear(cx),
            Command::Undo => {
                self.undo(cx);
                return;
            }
            Command::CopyUrl => return self.copy_url(false, cx),
            Command::CopyMarkdownLink => return self.copy_url(true, cx),
            Command::ReopenClosedTab => return self.browser.reopen_closed_tab(cx),
            Command::Back => return self.browser.go_back(self.selected, cx),
            Command::Forward => return self.browser.go_forward(self.selected, cx),
            Command::Reload => return self.browser.reload(self.selected, false, cx),
            Command::ForceReload => return self.browser.reload(self.selected, true, cx),
            Command::ClearCacheAndRefresh => {
                return self.browser.clear_site(self.selected, false, cx);
            }
            Command::ClearCookiesAndRefresh => {
                return self.browser.clear_site(self.selected, true, cx);
            }
            Command::Stop => return self.browser.stop(self.selected, cx),
            Command::ActualSize => return self.browser.zoom(self.selected, 0, cx),
            Command::ZoomIn => return self.browser.zoom(self.selected, 1, cx),
            Command::ZoomOut => return self.browser.zoom(self.selected, -1, cx),
            Command::ViewSource => return self.browser.view_source(self.selected, cx),
            Command::SavePage => return self.browser.save_page(self.selected, cx),
            Command::Print => return self.browser.print(self.selected, cx),
            Command::DevTools => return self.browser.devtools(self.selected, 0, cx),
            Command::JavaScriptConsole => return self.browser.devtools(self.selected, 1, cx),
            Command::InspectElements => return self.browser.devtools(self.selected, 2, cx),
            Command::TaskManager => return self.browser.task_manager(cx),
            Command::ManageExtensions => return self.browser.manage_extensions(None, cx),
            Command::Settings => return self.browser.settings(cx),
            Command::KeyboardShortcuts => return self.browser.shortcuts(cx),
            Command::ViewHistory => {
                // Chrome's ⌘Y shows the history page it has open rather than a second one.
                if !self.url(self.selected).starts_with("chrome://history") {
                    self.browser.view_history(cx);
                }
                return;
            }
            Command::Quit => return self.browser.quit(cx),
            Command::AddSplit => return self.add_pane(cx),
            Command::CloseSplitPane | Command::SeparatePane => {
                if self.panes(self.selected).len() > 1 {
                    match command {
                        Command::CloseSplitPane => self.browser.close(self.selected, false, cx),
                        _ => self.browser.separate(self.selected, cx),
                    }
                }
                return;
            }
            Command::SeparateSplit => {
                // From the right, so the pages keep their order in the list.
                for &tab in self.panes(self.selected)[1..].iter().rev() {
                    self.browser.separate(tab, cx);
                }
                return;
            }
            Command::FlipSplit => {
                for (index, &tab) in self.panes(self.selected).iter().rev().enumerate() {
                    self.browser.move_in_split(tab, index, cx);
                }
                return;
            }
            Command::ChangeOrientation => {
                let stacked = self.stacked(self.selected);
                return self.browser.set_split_stacked(self.selected, !stacked, cx);
            }
            Command::MovePaneLeft | Command::MovePaneRight => {
                let panes = self.panes(self.selected);
                let at = panes.iter().position(|&tab| tab == self.selected).unwrap_or_default();
                let to = match command {
                    Command::MovePaneLeft => at.checked_sub(1),
                    _ => Some(at + 1).filter(|&to| to < panes.len()),
                };
                if let Some(to) = to {
                    self.browser.move_in_split(self.selected, to, cx);
                }
                return;
            }
            Command::NextPane => return self.focus_pane(true, cx),
            Command::PreviousPane => return self.focus_pane(false, cx),
            Command::ToggleSidebar => return self.toggle_sidebar(cx),
            Command::ViewLibrary => return self.set_library(!self.library_open, cx),
            Command::ViewArchive => return self.view_section(Section::Archive, cx),
            Command::ViewDownloads => return self.view_section(Section::Downloads, cx),
            Command::ClearArchive => return self.clear_archive(cx),
            Command::NewSpace => return self.new_space(cx),
            Command::RenameSpace => return self.rename_space(self.space, cx),
            Command::SelectSpace(index) => return self.show_space_at(Some(index), cx),
            Command::NextSpace => return self.show_space_at(space.map(|index| index + 1), cx),
            Command::PreviousSpace => {
                return self.show_space_at(space.and_then(|index| index.checked_sub(1)), cx);
            }
            Command::Find => return self.browser.find(self.selected, cx),
            Command::FindNext => return self.browser.find_next(self.selected, true, cx),
            Command::FindPrevious => return self.browser.find_next(self.selected, false, cx),
            Command::JumpToSelection => return self.browser.jump_to_selection(self.selected, cx),
            Command::RecentTab => return self.walk(rows, true, cx),
            Command::RecentTabBack => return self.walk(rows, false, cx),
            Command::NextTab => position.map(|index| (index + 1) % count),
            Command::PreviousTab => position.map(|index| (index + count - 1) % count),
            Command::SelectTab(index) => (index < count).then_some(index),
            Command::SelectLastTab => count.checked_sub(1),
        };
        if let Some(index) = target {
            self.select(self.row_tab(rows[index]), cx);
        }
    }

    /// ⌃Tab, as in Arc: shows the Space's tab shown before this one at once, with no switcher.
    /// While ⌃ is held, each ⌃Tab goes on to the one shown before that, and ⌃⇧Tab back. A split's
    /// row counts once.
    fn walk(&mut self, rows: Vec<u64>, forward: bool, cx: &mut Context<Self>) {
        // Then the rows not shown yet, in the sidebar's order.
        let mut order: Vec<u64> = Vec::new();
        for row in self.recent.iter().map(|&tab| self.row_of(tab)).chain(rows.iter().copied()) {
            if rows.contains(&row) && !order.contains(&row) {
                order.push(row);
            }
        }
        let count = order.len();
        let next = match order.iter().position(|&row| row == self.row_of(self.selected)) {
            Some(at) if forward => (at + 1) % count,
            Some(at) => (at + count - 1) % count,
            // With no tab shown, the tab shown last (Arc).
            None if forward && count > 0 => 0,
            None => match count.checked_sub(1) {
                Some(last) => last,
                None => return,
            },
        };
        let tab = self.row_tab(order[next]);
        if tab == self.selected {
            return;
        }
        if self.walked.is_none() {
            self.browser.watch_control(cx);
        }
        self.walked = Some(tab);
        self.select(tab, cx);
    }

    /// Clears the Space shown, but the tab shown. Its rows go in one frame, with no toast (Arc).
    fn clear(&mut self, cx: &mut Context<Self>) {
        // A split view shown stays whole.
        let shown = self.panes(self.selected);
        self.clearing = self.today().iter().copied().filter(|tab| !shown.contains(tab)).collect();
        if self.clearing.is_empty() {
            return;
        }
        self.browser.clear(cx);
        self.undo = Some(Undo::Clear);
        // check.sh reads this line.
        log!("browser: cleared {}", self.clearing.len());
    }

    /// Undoes the last Clear or Space deleted, once, and returns whether it did.
    fn undo(&mut self, cx: &mut Context<Self>) -> bool {
        match self.undo.take() {
            Some(Undo::Clear) => self.browser.undo_clear(cx) > 0,
            Some(Undo::DeleteSpace) => self.browser.undo_delete_space(cx),
            None => false,
        }
    }

    /// Runs `command` for a key the page didn't keep, and returns whether it was used. With
    /// nothing to undo, ⌘Z goes back, for the Edit menu to give the page.
    pub fn run_for_page(&mut self, command: Command, cx: &mut Context<Self>) -> bool {
        if command != Command::Undo {
            self.run(command, "page", cx);
            return true;
        }
        let undone = self.undo(cx);
        if undone {
            // check.sh waits for this line.
            log!("browser: command undo via page");
        }
        undone
    }

    /// Copies the selected tab's URL, or a Markdown link to it, without its tracking parameters,
    /// and says so in the toast.
    fn copy_url(&mut self, markdown: bool, cx: &mut Context<Self>) {
        if let Some((text, toast)) = self.url_to_copy(markdown) {
            self.copy(&text, toast, cx);
        }
    }

    /// The text to copy and the toast to say so with.
    fn url_to_copy(&self, markdown: bool) -> Option<(String, &'static str)> {
        let url = clean_link(&self.url(self.selected));
        if url.is_empty() {
            return None;
        }
        let (text, toast) = match markdown {
            false => (url.clone(), "Copied URL"),
            true => {
                let title = markdown_text(&self.title(self.selected));
                (format!("[{title}]({url})"), "Copied URL as Markdown")
            }
        };
        Some((text, toast))
    }

    /// A click on the URL field's Copy URL button: copies without the toast, since the button
    /// shows a checkmark in the glyph's place for `COPIED_TIME`.
    pub(super) fn copy_url_from_button(&mut self, cx: &mut Context<Self>) {
        // check.sh waits for these lines.
        log!("browser: command copy-url via sidebar");
        let Some((text, _)) = self.url_to_copy(false) else {
            return;
        };
        self.browser.copy(&text, cx);
        log!("browser: copied {text}");
        self.url_copied = Some(cx.spawn(async move |shell, cx| {
            cx.background_executor().timer(COPIED_TIME).await;
            shell
                .update(cx, |shell, cx| {
                    shell.url_copied = None;
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    /// Copies `text` and shows `toast`.
    pub(super) fn copy(&mut self, text: &str, toast: &str, cx: &mut Context<Self>) {
        self.browser.copy(text, cx);
        // check.sh reads this line.
        log!("browser: copied {text}");
        self.browser.toast(toast, None, COPIED_TIME, cx);
    }

    /// Whether `command` would do something now, for the command bar to offer it.
    fn offers(&self, command: Command) -> bool {
        let page = self.page(self.selected);
        let space = self.spaces.iter().position(|space| space.id == self.space);
        match command {
            Command::Back => page.can_go_back,
            Command::Forward => page.can_go_forward,
            Command::Stop => page.loading,
            // Only a web page has a site to clear.
            Command::ClearCacheAndRefresh | Command::ClearCookiesAndRefresh => {
                page.url.starts_with("http://") || page.url.starts_with("https://")
            }
            Command::BackToPinnedUrl => page.away_from_pinned_url,
            Command::CloseSplitPane
            | Command::SeparatePane
            | Command::SeparateSplit
            | Command::FlipSplit
            | Command::ChangeOrientation => self.panes(self.selected).len() > 1,
            Command::MovePaneLeft => self.panes(self.selected).first() != Some(&self.selected),
            Command::MovePaneRight => self.panes(self.selected).last() != Some(&self.selected),
            Command::CollapseFolders => self.shown_space().is_some_and(|space| {
                space.folders.folders.keys().any(|&folder| space.is_open(folder))
            }),
            Command::Clear => !self.today().is_empty(),
            Command::NextSpace => space.is_some_and(|index| index + 1 < self.spaces.len()),
            Command::PreviousSpace => space.is_some_and(|index| index > 0),
            Command::NextTab | Command::PreviousTab => self.sidebar_rows().len() > 1,
            Command::NewFolder
            | Command::ReopenClosedTab
            | Command::ToggleSidebar
            | Command::NewSpace
            | Command::RenameSpace
            | Command::ViewLibrary
            | Command::ViewArchive
            | Command::ClearArchive
            | Command::ViewDownloads
            | Command::ManageExtensions
            | Command::ViewHistory
            | Command::KeyboardShortcuts
            | Command::Settings => true,
            _ => self.selected != 0,
        }
    }
}

/// `text` for a Markdown link's text, with what would end it escaped.
pub(super) fn markdown_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key that ran two commands would run the first of them, and the menu would show it for
    /// the other.
    #[test]
    fn no_key_is_bound_twice() {
        for (at, &(key, ..)) in SHORTCUTS.iter().enumerate() {
            assert!(SHORTCUTS[..at].iter().all(|&(each, ..)| each != key), "{key} is bound twice");
        }
    }

    /// ⌃n shows the nth Space, as the Spaces menu lists them, and the ones past the ninth have
    /// no key.
    #[test]
    fn ctrl_and_a_digit_show_the_spaces_in_order() {
        for number in 1..=9 {
            let key = format!("ctrl-{number}");
            assert_eq!(Command::for_keystroke(&key, false), Some(Command::SelectSpace(number - 1)));
        }
        assert_eq!(shortcut_label(Command::SelectSpace(0)).as_deref(), Some("⌃1"));
        assert_eq!(shortcut_label(Command::SelectSpace(9)), None);
    }

    /// ⌘Y is a page's to keep, as Chrome's is, unlike the reserved keys.
    #[test]
    fn history_is_not_a_reserved_key() {
        assert_eq!(Command::for_keystroke("cmd-y", false), Some(Command::ViewHistory));
        assert_eq!(Command::for_keystroke("cmd-y", true), None);
    }
}
