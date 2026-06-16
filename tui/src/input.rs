//! Key event → semantic input action mapping.
//!
//! Keeping this in one place means the keybinding table in spec §5.6 maps
//! directly to a `match` arm here, and we never sprinkle raw `KeyCode` checks
//! across UI renderers.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Semantic input actions. UI code branches on these, never on raw `KeyCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    /// Quit the application.
    Quit,
    /// Toggle the help overlay.
    Help,
    /// Cycle focus forward (sidebar → main → log → sidebar).
    TabFocusNext,
    /// Cycle focus backward.
    TabFocusPrev,
    /// Move sidebar selection down by one.
    SidebarDown,
    /// Move sidebar selection up by one.
    SidebarUp,
    /// Jump sidebar selection to the first item.
    SidebarTop,
    /// Jump sidebar selection to the last item.
    SidebarBottom,
    /// Jump directly to action N (1-indexed).
    JumpTo(usize),
    /// Confirm / run the focused action.
    Enter,
    /// Close a modal, or cancel a running job (M7+).
    Esc,
    /// Clear the log pane.
    ClearLog,
    /// Yank the visible log to the system clipboard (M3+).
    YankLog,
    /// Unhandled key — explicitly a no-op.
    Noop,
}

pub fn map_key(event: KeyEvent) -> InputAction {
    // The modifiers-aware form: Ctrl combos first, then plain keys.
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return match event.code {
            KeyCode::Char('c') => InputAction::Quit,
            KeyCode::Char('l') => InputAction::ClearLog,
            _ => InputAction::Noop,
        };
    }

    match event.code {
        KeyCode::Char('q') => InputAction::Quit,
        KeyCode::Char('?') => InputAction::Help,
        KeyCode::Char('j') | KeyCode::Down => InputAction::SidebarDown,
        KeyCode::Char('k') | KeyCode::Up => InputAction::SidebarUp,
        KeyCode::Char('g') => InputAction::SidebarTop,
        KeyCode::Char('G') => InputAction::SidebarBottom,
        KeyCode::Char('1') => InputAction::JumpTo(1),
        KeyCode::Char('2') => InputAction::JumpTo(2),
        KeyCode::Char('3') => InputAction::JumpTo(3),
        KeyCode::Char('4') => InputAction::JumpTo(4),
        KeyCode::Char('5') => InputAction::JumpTo(5),
        KeyCode::Char('6') => InputAction::JumpTo(6),
        KeyCode::Char('y') => InputAction::YankLog,
        KeyCode::Tab => InputAction::TabFocusNext,
        KeyCode::BackTab => InputAction::TabFocusPrev,
        KeyCode::Enter => InputAction::Enter,
        KeyCode::Esc => InputAction::Esc,
        _ => InputAction::Noop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind};

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: mods,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
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
    fn unknown_keys_are_noop() {
        assert_eq!(
            map_key(key(KeyCode::F(1), KeyModifiers::NONE)),
            InputAction::Noop
        );
    }
}
