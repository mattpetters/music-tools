//! Key event → semantic input action mapping.
//!
//! Keeping this in one place means the keybinding table in spec §5.6 maps
//! directly to a `match` arm here, and we never sprinkle raw `KeyCode` checks
//! across UI renderers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(test)]
use crossterm::event::KeyEventKind;

/// Semantic input actions. UI code branches on these, never on raw `KeyCode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    /// Quit the application.
    Quit,
    /// Toggle the help overlay.
    Help,
    /// Cycle focus forward (sidebar → main → log).
    TabFocusNext,
    /// Cycle focus backward.
    TabFocusPrev,
    /// Move selection down by one. Pane-aware (sidebar vs. browser cursor).
    SidebarDown,
    /// Move selection up by one. Pane-aware.
    SidebarUp,
    /// Jump selection to the first item.
    SidebarTop,
    /// Jump selection to the last item.
    SidebarBottom,
    /// Jump directly to action N (1-indexed). Always sidebar-focused.
    JumpTo(usize),
    /// Confirm / run the focused action.
    Enter,
    /// Close a modal, or cancel a running job (M7+).
    Esc,
    /// Clear the log pane.
    ClearLog,
    /// Yank the visible log to the system clipboard (M3+).
    YankLog,
    /// Open the entry under the browser cursor (cd into dir, or no-op for files).
    OpenEntry,
    /// Go up one directory in the browser.
    GoUp,
    /// Toggle the "checked" state of the file under the browser cursor.
    ToggleCheck,
    /// Focus the path field in the browser.
    PathInput,
    /// Jump the browser to `$HOME`.
    GoHome,
    /// Refresh the current view (browser dir listing, or sidebar in future).
    Refresh,
    /// Open the recents-locations palette.
    Recents,
    /// Backspace (used in path-input mode).
    Backspace,
    /// Any printable character. Used in path-input mode.
    Char(char),
    /// Unhandled key — explicitly a no-op.
    Noop,
}

pub fn map_key(event: KeyEvent) -> InputAction {
    // Ctrl combos first.
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return match event.code {
            KeyCode::Char('c') => InputAction::Quit,
            KeyCode::Char('l') => InputAction::ClearLog,
            _ => InputAction::Noop,
        };
    }

    match event.code {
        // Quit / help
        KeyCode::Char('q') => InputAction::Quit,
        KeyCode::Char('?') => InputAction::Help,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => InputAction::SidebarDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::SidebarUp,
        KeyCode::Char('g') => InputAction::SidebarTop,
        KeyCode::Char('G') => InputAction::SidebarBottom,
        KeyCode::Char('l') | KeyCode::Right => InputAction::OpenEntry,
        KeyCode::Char('h') | KeyCode::Left => InputAction::GoUp,

        // Direct jump (sidebar)
        KeyCode::Char('1') => InputAction::JumpTo(1),
        KeyCode::Char('2') => InputAction::JumpTo(2),
        KeyCode::Char('3') => InputAction::JumpTo(3),
        KeyCode::Char('4') => InputAction::JumpTo(4),
        KeyCode::Char('5') => InputAction::JumpTo(5),
        KeyCode::Char('6') => InputAction::JumpTo(6),

        // Browser shortcuts
        KeyCode::Char(' ') => InputAction::ToggleCheck,
        KeyCode::Char('/') => InputAction::PathInput,
        KeyCode::Char('~') => InputAction::GoHome,
        KeyCode::Char('r') => InputAction::Refresh,
        KeyCode::Char('@') => InputAction::Recents,
        KeyCode::Backspace => InputAction::Backspace,

        // Log
        KeyCode::Char('y') => InputAction::YankLog,
        KeyCode::Char('c') => InputAction::ClearLog,

        // Focus
        KeyCode::Tab => InputAction::TabFocusNext,
        KeyCode::BackTab => InputAction::TabFocusPrev,

        // Run / cancel
        KeyCode::Enter => InputAction::Enter,
        KeyCode::Esc => InputAction::Esc,

        // Catch-all for path-input mode
        KeyCode::Char(c) => InputAction::Char(c),

        _ => InputAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_quits() {
        assert_eq!(
            map_key(key(KeyCode::Char('q'), KeyModifiers::NONE)),
            InputAction::Quit
        );
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(
            map_key(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            InputAction::Quit
        );
    }

    #[test]
    fn digits_jump() {
        assert_eq!(
            map_key(key(KeyCode::Char('1'), KeyModifiers::NONE)),
            InputAction::JumpTo(1)
        );
        assert_eq!(
            map_key(key(KeyCode::Char('6'), KeyModifiers::NONE)),
            InputAction::JumpTo(6)
        );
    }

    #[test]
    fn tab_cycles() {
        assert_eq!(
            map_key(key(KeyCode::Tab, KeyModifiers::NONE)),
            InputAction::TabFocusNext
        );
        assert_eq!(
            map_key(key(KeyCode::BackTab, KeyModifiers::SHIFT)),
            InputAction::TabFocusPrev
        );
    }

    #[test]
    fn arrows_navigate() {
        assert_eq!(
            map_key(key(KeyCode::Down, KeyModifiers::NONE)),
            InputAction::SidebarDown
        );
        assert_eq!(
            map_key(key(KeyCode::Up, KeyModifiers::NONE)),
            InputAction::SidebarUp
        );
        assert_eq!(
            map_key(key(KeyCode::Right, KeyModifiers::NONE)),
            InputAction::OpenEntry
        );
        assert_eq!(
            map_key(key(KeyCode::Left, KeyModifiers::NONE)),
            InputAction::GoUp
        );
    }

    #[test]
    fn browser_shortcuts() {
        assert_eq!(
            map_key(key(KeyCode::Char('l'), KeyModifiers::NONE)),
            InputAction::OpenEntry
        );
        assert_eq!(
            map_key(key(KeyCode::Char('h'), KeyModifiers::NONE)),
            InputAction::GoUp
        );
        assert_eq!(
            map_key(key(KeyCode::Char(' '), KeyModifiers::NONE)),
            InputAction::ToggleCheck
        );
        assert_eq!(
            map_key(key(KeyCode::Char('/'), KeyModifiers::NONE)),
            InputAction::PathInput
        );
        assert_eq!(
            map_key(key(KeyCode::Char('~'), KeyModifiers::NONE)),
            InputAction::GoHome
        );
        assert_eq!(
            map_key(key(KeyCode::Char('r'), KeyModifiers::NONE)),
            InputAction::Refresh
        );
    }

    #[test]
    fn unknown_keys_are_noop() {
        assert_eq!(
            map_key(key(KeyCode::F(1), KeyModifiers::NONE)),
            InputAction::Noop
        );
    }

    #[test]
    fn chars_passthrough() {
        assert_eq!(
            map_key(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            InputAction::Char('a')
        );
        assert_eq!(
            map_key(key(KeyCode::Char('Z'), KeyModifiers::NONE)),
            InputAction::Char('Z')
        );
    }

    #[test]
    fn backspace_mapped() {
        assert_eq!(
            map_key(key(KeyCode::Backspace, KeyModifiers::NONE)),
            InputAction::Backspace
        );
    }
}
