//! The user's keys for the browser's commands (docs/ux-spec.md › Keyboard): each command keeps its
//! keys in SHORTCUTS unless the user gave it others in Settings › Shortcuts, or none. GPUI's
//! bindings, the menus, the command bar and the keys the page hands back all ask this store, so a
//! change reaches them at once.

use gpui::{
    Action, App, Global, KeyBinding, KeyBindingContextPredicate, Keystroke, NoAction,
    OperatingSystem,
};

use super::commands::{BAR_COMMANDS, KEY_CONTEXT, Run, SHORTCUTS, ShellHandle, keystroke_label};
use super::Command;

/// Keys that run something other than a command: AppKit's Hide, Hide Others and Minimize, which
/// main.rs binds, the Edit menu's, which reach the focused page or field, and the View menu's
/// full screen.
const SYSTEM_KEYS: &[(&str, &str)] = &[
    ("cmd-h", "Hide Browser"),
    ("alt-cmd-h", "Hide Others"),
    ("cmd-m", "Minimize"),
    ("cmd-x", "Cut"),
    ("cmd-c", "Copy"),
    ("cmd-v", "Paste"),
    ("cmd-a", "Select All"),
    ("shift-cmd-z", "Redo"),
    ("ctrl-cmd-f", "Enter Full Screen"),
];

/// Characters typed with ⇧ on a US keyboard. GPUI names ⇧⌘= by the character, cmd-+, and a page
/// by the character and ⇧, shift-cmd-+.
const SHIFTED: &str = "~!@#$%^&*()_+{}|:\"<>?";

/// Who keeps a key the user asked for.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Holder {
    /// A reserved command, or one that can't be rebound.
    Command(Command),
    /// One of SYSTEM_KEYS, by its menu item's name.
    System(&'static str),
}

/// Why a key the user pressed wasn't given to the command.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Refused {
    /// Without ⌘ or ⌃ it would type, unless it's a function key.
    Plain,
    Taken(Holder),
}

/// The commands the user gave keys of their own, or none.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct UserKeys {
    set: Vec<(Command, Vec<String>)>,
}

impl Global for UserKeys {}

static DEFAULTS: UserKeys = UserKeys { set: Vec::new() };

/// The user's keys, or the defaults before they're read.
pub fn user_keys(cx: &App) -> &UserKeys {
    cx.try_global::<UserKeys>().unwrap_or(&DEFAULTS)
}

/// Whether the user can give `command` other keys. Not Quit, whose ⌘Q is the way out of every Mac
/// app; not Undo, whose ⌘Z falls back to the Edit menu's Undo for the page; not the ⌃Tab walk,
/// which lasts while ⌃ is held; and not the numbered tabs and Spaces, nine keys in one row.
pub fn rebindable(command: Command) -> bool {
    !matches!(
        command,
        Command::Quit
            | Command::Undo
            | Command::RecentTab
            | Command::RecentTabBack
            | Command::SelectTab(_)
            | Command::SelectSpace(_)
    )
}

/// Whether `command` reaches the browser before the page, on whichever key it has.
pub fn reserved(command: Command) -> bool {
    SHORTCUTS.iter().any(|&(_, each, reserved)| each == command && reserved)
}

fn defaults(command: Command) -> Vec<String> {
    (SHORTCUTS.iter())
        .filter(|&&(_, each, _)| each == command)
        .map(|&(key, ..)| key.to_string())
        .collect()
}

/// The rebindable command `name` names, as Command::name writes it.
fn named(name: &str) -> Option<Command> {
    let commands = SHORTCUTS.iter().map(|&(_, command, _)| command);
    (commands.chain(BAR_COMMANDS.iter().map(|&(_, command)| command)))
        .find(|&command| command.name() == name && rebindable(command))
}

/// Whether `key` is one keystroke GPUI can read. Only its spelling is checked, which is every
/// platform's.
fn valid(key: &str) -> bool {
    let parsed = Keystroke::parse_with_os(key, OperatingSystem::Mac);
    !key.contains(char::is_whitespace) && parsed.is_ok()
}

/// GPUI's binding of `key` to `action` while the UI has the keyboard.
fn binding(key: &str, action: Box<dyn Action>, cx: &App) -> Option<KeyBinding> {
    let context = KeyBindingContextPredicate::parse(KEY_CONTEXT).ok()?.into();
    KeyBinding::load(key, action, Some(context), false, None, cx).ok()
}

/// The keys the user pressed, as SHORTCUTS writes them: the one GPUI matches first, then the one
/// a page names, where they differ.
pub fn recorded(keystroke: &Keystroke) -> Result<Vec<String>, Refused> {
    let modifiers = &keystroke.modifiers;
    let key = keystroke.key.as_str();
    let function = key.strip_prefix('f').is_some_and(|number| number.parse::<u8>().is_ok());
    if !modifiers.control && !modifiers.platform && !function {
        return Err(Refused::Plain);
    }
    let text = |shift: bool| {
        let mut text = String::new();
        for (held, name) in [
            (modifiers.control, "ctrl-"),
            (modifiers.alt, "alt-"),
            (shift, "shift-"),
            (modifiers.platform, "cmd-"),
        ] {
            if held {
                text.push_str(name);
            }
        }
        text + key
    };
    let mut keys = vec![text(modifiers.shift)];
    if !modifiers.shift && key.chars().count() == 1 && SHIFTED.contains(key) {
        keys.push(text(true));
    }
    keys.retain(|key| valid(key));
    match keys.is_empty() {
        true => Err(Refused::Plain),
        false => Ok(keys),
    }
}

impl UserKeys {
    /// The keys that run `command`, the one menus show first.
    pub fn keys(&self, command: Command) -> Vec<&str> {
        match self.set.iter().find(|(each, _)| *each == command) {
            Some((_, keys)) => keys.iter().map(String::as_str).collect(),
            None => (SHORTCUTS.iter())
                .filter(|&&(_, each, _)| each == command)
                .map(|&(key, ..)| key)
                .collect(),
        }
    }

    /// Whether the user gave `command` keys other than its defaults.
    pub fn is_set(&self, command: Command) -> bool {
        self.set.iter().any(|(each, _)| *each == command)
    }

    /// The command `keystroke` runs, if it's a shortcut that is `reserved` or isn't.
    pub fn command_for(&self, keystroke: &str, reserved: bool) -> Option<Command> {
        self.command_with(keystroke).filter(|&command| self::reserved(command) == reserved)
    }

    fn command_with(&self, keystroke: &str) -> Option<Command> {
        let user = self.set.iter().find(|(_, keys)| keys.iter().any(|key| key == keystroke));
        user.map(|&(command, _)| command).or_else(|| {
            (SHORTCUTS.iter())
                .find(|&&(key, command, _)| key == keystroke && !self.is_set(command))
                .map(|&(_, command, _)| command)
        })
    }

    /// How a menu shows the first key that runs `command`, if one does.
    pub fn label(&self, command: Command) -> Option<String> {
        self.keys(command).first().map(|&key| keystroke_label(key))
    }

    /// Each key and the command it runs.
    fn bindings(&self) -> Vec<(&str, Command)> {
        let defaults = SHORTCUTS.iter().filter(|&&(_, command, _)| !self.is_set(command));
        let user = (self.set.iter())
            .flat_map(|(command, keys)| keys.iter().map(|key| (key.as_str(), *command)));
        (defaults.map(|&(key, command, _)| (key, command))).chain(user).collect()
    }

    /// Gives `command` `keys`, or none, taking each from the command that has it, as a user's
    /// binding beats the others (Dia). A reserved command's key, one that can't be rebound and the
    /// system's stay theirs. Returns the commands that lost a key.
    pub fn set(&mut self, command: Command, keys: Vec<String>) -> Result<Vec<Command>, Refused> {
        if !rebindable(command) {
            return Err(Refused::Taken(Holder::Command(command)));
        }
        for key in &keys {
            if let Some(&(_, name)) = SYSTEM_KEYS.iter().find(|(each, _)| each == key) {
                return Err(Refused::Taken(Holder::System(name)));
            }
            match self.command_with(key) {
                Some(holder) if holder != command && (reserved(holder) || !rebindable(holder)) => {
                    return Err(Refused::Taken(Holder::Command(holder)));
                }
                _ => {}
            }
        }
        let mut losers = Vec::new();
        for key in &keys {
            if let Some(holder) = self.command_with(key).filter(|&holder| holder != command) {
                let left = self.keys(holder).into_iter().filter(|each| each != key);
                let left = left.map(str::to_string).collect();
                self.put(holder, left);
                if !losers.contains(&holder) {
                    losers.push(holder);
                }
            }
        }
        self.put(command, keys);
        Ok(losers)
    }

    /// Gives `command` its default keys back, as set does.
    pub fn reset(&mut self, command: Command) -> Result<Vec<Command>, Refused> {
        self.set(command, defaults(command))
    }

    pub fn reset_all(&mut self) {
        self.set.clear();
    }

    /// Keeps `keys` for `command`, unless they're its defaults.
    fn put(&mut self, command: Command, keys: Vec<String>) {
        self.set.retain(|(each, _)| *each != command);
        if keys != defaults(command) {
            self.set.push((command, keys));
        }
    }

    /// As Settings saves them: a line a command, its name, then its keys, if any.
    pub fn to_text(&self) -> String {
        let lines = self.set.iter().map(|(command, keys)| {
            let words = std::iter::once(command.name()).chain(keys.iter().map(String::as_str));
            words.collect::<Vec<_>>().join(" ") + "\n"
        });
        lines.collect()
    }

    /// What to_text wrote. A line it can't read, or keys another command keeps, is left out.
    pub fn parse(text: &str) -> Self {
        let mut keys = UserKeys::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let Some(command) = words.next().and_then(named) else {
                continue;
            };
            let set = words.filter(|key| valid(key)).map(str::to_string);
            keys.set(command, set.collect()).ok();
        }
        keys
    }
}

/// Changes the user's keys with `change`, and gives GPUI's bindings and the menus the keys of the
/// commands it changed. The command bar and the page's keys ask the store each time.
pub fn change_keys<R>(cx: &mut App, change: impl FnOnce(&mut UserKeys) -> R) -> R {
    let before = user_keys(cx).clone();
    let mut after = before.clone();
    let result = change(&mut after);
    let mut changed: Vec<Command> = Vec::new();
    for &(command, _) in before.set.iter().chain(&after.set) {
        if !changed.contains(&command) && before.keys(command) != after.keys(command) {
            changed.push(command);
        }
    }
    // GPUI's keymap can't take a binding out. A later NoAction on the same key and context
    // disables the earlier ones, for keystrokes and for the menus' key equivalents, and the
    // bindings after it win again.
    let old = changed.iter().flat_map(|&command| before.keys(command));
    let old: Vec<KeyBinding> = old.filter_map(|key| binding(key, Box::new(NoAction), cx)).collect();
    let app: &App = cx;
    let new = changed.iter().flat_map(|&command| {
        let keys = after.keys(command).into_iter();
        keys.filter_map(move |key| binding(key, Box::new(Run(command)), app))
    });
    let new: Vec<KeyBinding> = new.collect();
    cx.bind_keys(old);
    cx.bind_keys(new);
    for &command in &changed {
        let keys = after.keys(command);
        // check.sh waits for these lines.
        match keys.is_empty() {
            true => log!("browser: shortcut {} none", command.name()),
            false => log!("browser: shortcut {} {}", command.name(), keys.join(" ")),
        }
    }
    cx.set_global(after);
    if let Some(shell) = cx.try_global::<ShellHandle>().map(|handle| handle.0.clone()) {
        shell.update(cx, |shell, cx| shell.refresh_menus(cx)).ok();
    }
    result
}

/// GPUI's bindings for the keys the store has now.
pub(super) fn key_bindings(cx: &App) -> Vec<KeyBinding> {
    (user_keys(cx).bindings().into_iter())
        .filter_map(|(key, command)| binding(key, Box::new(Run(command)), cx))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{
        Context, FocusHandle, Modifiers, TestAppContext, VisualTestContext, Window, div,
        prelude::*,
    };

    use super::*;

    /// A window of the UI with the keyboard, as the shell's root has it.
    struct Root(FocusHandle);

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().key_context(KEY_CONTEXT).track_focus(&self.0).size_full()
        }
    }

    /// GPUI's bindings follow a change at once: the new key runs the command, the old one
    /// nothing, and the menus show the new one.
    #[gpui::test]
    fn gpui_follows_a_change(cx: &mut TestAppContext) {
        let ran: Rc<RefCell<Vec<Command>>> = Rc::default();
        let runs = ran.clone();
        cx.update(|cx| {
            cx.bind_keys(key_bindings(cx));
            cx.on_action(move |run: &Run, _| runs.borrow_mut().push(run.0));
        });
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Root(focus)
                })
            })
        });
        let mut cx = VisualTestContext::from_window(window.unwrap().into(), cx);
        cx.simulate_keystrokes("cmd-l");
        assert_eq!(ran.take(), [Command::EditUrl]);
        cx.update(|_, cx| change_keys(cx, |user| user.set(Command::EditUrl, keys(&["cmd-k"]))))
            .unwrap();
        cx.simulate_keystrokes("cmd-l cmd-k");
        assert_eq!(ran.take(), [Command::EditUrl]);
        let shown = cx.update(|_, cx| {
            let keymap = cx.key_bindings();
            let keymap = keymap.borrow();
            let binding = keymap.bindings_for_action(&Run(Command::EditUrl)).next();
            binding.map(|binding| binding.keystrokes()[0].inner().unparse(OperatingSystem::Mac))
        });
        assert_eq!(shown.as_deref(), Some("cmd-k"));
        // Back to ⌘L, which runs it again, and ⌘K nothing.
        cx.update(|_, cx| change_keys(cx, UserKeys::reset_all));
        cx.simulate_keystrokes("cmd-k cmd-l");
        assert_eq!(ran.take(), [Command::EditUrl]);
    }

    fn keys(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|key| key.to_string()).collect()
    }

    /// A rebound key runs its command, and the key it had runs nothing.
    #[test]
    fn a_rebound_key_runs_its_command_and_the_old_one_nothing() {
        let mut user = UserKeys::default();
        assert_eq!(user.set(Command::EditUrl, keys(&["cmd-k"])), Ok(vec![]));
        assert_eq!(user.command_for("cmd-k", false), Some(Command::EditUrl));
        assert_eq!(user.command_for("cmd-l", false), None);
        assert_eq!(user.label(Command::EditUrl).as_deref(), Some("⌘K"));
        assert_eq!(user.set(Command::EditUrl, vec![]), Ok(vec![]));
        assert_eq!(user.command_for("cmd-k", false), None);
        assert_eq!(user.label(Command::EditUrl), None);
    }

    /// A reserved command stays reserved on its new key, which the page then never sees, and the
    /// key it left is the page's.
    #[test]
    fn reserved_stays_reserved_on_its_new_key() {
        let mut user = UserKeys::default();
        user.set(Command::NewTab, keys(&["ctrl-cmd-t"])).unwrap();
        assert_eq!(user.command_for("ctrl-cmd-t", true), Some(Command::NewTab));
        assert_eq!(user.command_for("ctrl-cmd-t", false), None);
        assert_eq!(user.command_for("cmd-t", true), None);
        assert_eq!(user.command_for("cmd-t", false), None);
    }

    /// A key another command has moves to the one it's given, leaving the other without it, but a
    /// reserved command's, Quit's and the system's are refused.
    #[test]
    fn a_taken_key_moves_unless_it_is_reserved() {
        let mut user = UserKeys::default();
        assert_eq!(user.set(Command::EditUrl, keys(&["shift-cmd-k"])), Ok(vec![Command::Clear]));
        assert_eq!(user.command_for("shift-cmd-k", false), Some(Command::EditUrl));
        assert!(user.keys(Command::Clear).is_empty());
        // A command with two keys keeps the other.
        user.set(Command::Find, keys(&["cmd-["])).unwrap();
        assert_eq!(user.keys(Command::Back), ["cmd-left"]);
        let before = user.clone();
        let taken = |command| Err(Refused::Taken(Holder::Command(command)));
        assert_eq!(user.set(Command::EditUrl, keys(&["cmd-t"])), taken(Command::NewTab));
        assert_eq!(user.set(Command::EditUrl, keys(&["cmd-q"])), taken(Command::Quit));
        assert_eq!(user.set(Command::EditUrl, keys(&["ctrl-1"])), taken(Command::SelectSpace(0)));
        let system = Err(Refused::Taken(Holder::System("Copy")));
        assert_eq!(user.set(Command::EditUrl, keys(&["cmd-c"])), system);
        assert_eq!(user.set(Command::Quit, keys(&["cmd-k"])), taken(Command::Quit));
        assert_eq!(user, before);
    }

    /// Restoring a command's default takes its key back, and Restore Defaults puts every one back.
    #[test]
    fn reset_restores_the_defaults() {
        let mut user = UserKeys::default();
        user.set(Command::EditUrl, keys(&["shift-cmd-k"])).unwrap();
        assert_eq!(user.reset(Command::Clear), Ok(vec![Command::EditUrl]));
        assert_eq!(user.command_for("shift-cmd-k", false), Some(Command::Clear));
        assert!(user.is_set(Command::EditUrl) && !user.is_set(Command::Clear));
        user.set(Command::NewTab, keys(&["ctrl-cmd-t"])).unwrap();
        user.reset_all();
        assert_eq!(user, UserKeys::default());
        assert_eq!(user.command_for("cmd-t", true), Some(Command::NewTab));
    }

    /// What Settings saves reads back the same, and a line naming no command, or a command the
    /// user can't rebind, is left out.
    #[test]
    fn saved_keys_read_back() {
        let mut user = UserKeys::default();
        user.set(Command::EditUrl, keys(&["cmd-k"])).unwrap();
        user.set(Command::ToggleSidebar, vec![]).unwrap();
        user.set(Command::NewFolder, keys(&["alt-cmd-n"])).unwrap();
        assert_eq!(UserKeys::parse(&user.to_text()), user);
        assert_eq!(UserKeys::parse("quit cmd-k\nnothing cmd-j\n"), UserKeys::default());
    }

    /// A key needs ⌘ or ⌃, as one without would type, and ⇧ with a character a page names its
    /// way too.
    #[test]
    fn recorded_keys_are_written_as_the_shortcuts_are() {
        let keystroke = |control, shift, platform, key: &str| Keystroke {
            modifiers: Modifiers { control, shift, platform, ..Default::default() },
            key: key.into(),
            key_char: None,
        };
        assert_eq!(recorded(&keystroke(true, false, true, "t")), Ok(keys(&["ctrl-cmd-t"])));
        assert_eq!(recorded(&keystroke(false, true, true, "k")), Ok(keys(&["shift-cmd-k"])));
        let plus = recorded(&keystroke(false, false, true, "+"));
        assert_eq!(plus, Ok(keys(&["cmd-+", "shift-cmd-+"])));
        assert_eq!(recorded(&keystroke(false, false, false, "f5")), Ok(keys(&["f5"])));
        assert_eq!(recorded(&keystroke(false, true, false, "k")), Err(Refused::Plain));
    }
}
