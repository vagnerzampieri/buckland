//! Single source of truth for keybindings. Screens consume `KeyAction`
//! variants, never raw `KeyEvent`s. New shortcuts go here first; this
//! makes the keymap auditable in one place and keeps the Help overlay
//! honest.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    // --- Navigation
    Down,
    Up,
    Left,
    Right,
    Top,
    Bottom,
    CycleNext, // Tab
    CyclePrev, // BackTab (Shift+Tab)

    // --- View switch
    OpenTasks,  // g
    OpenAgenda, // a
    OpenReport, // r

    // --- Task / Entry actions
    NewTask,       // n
    StartSelected, // s
    StopActive,    // S
    Done,          // d
    Archive,       // A
    Delete,        // D
    Edit,          // e
    Filter,        // /
    Confirm,       // Enter

    // --- Report actions
    CycleScope, // Tab in Report (subset of CycleNext, dispatched by screen)
    CycleGroup, // T
    Copy,       // c
    ToggleJson, // j (only in Report; conflicts handled in screen)

    // --- Global
    Help,        // ?
    Quit,        // q / esc
    DeleteCtrlD, // ^D — delete inside the Edit modal

    // --- Catch-all for free typing inside inline prompts
    Char(char),
    Backspace,

    // --- Anything we don't handle
    Ignore,
}

pub fn resolve(event: KeyEvent) -> KeyAction {
    let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
    match event.code {
        KeyCode::Char('d') if ctrl => KeyAction::DeleteCtrlD,
        KeyCode::Char('c') if ctrl => KeyAction::Quit,
        KeyCode::Char('j') => KeyAction::Down,
        KeyCode::Char('k') => KeyAction::Up,
        KeyCode::Char('h') => KeyAction::Left,
        KeyCode::Char('l') => KeyAction::Right,
        KeyCode::Char('g') => KeyAction::OpenTasks,
        KeyCode::Char('G') => KeyAction::Bottom,
        KeyCode::Char('a') => KeyAction::OpenAgenda,
        KeyCode::Char('r') => KeyAction::OpenReport,
        KeyCode::Char('n') => KeyAction::NewTask,
        KeyCode::Char('s') => KeyAction::StartSelected,
        KeyCode::Char('S') => KeyAction::StopActive,
        KeyCode::Char('d') => KeyAction::Done,
        KeyCode::Char('A') => KeyAction::Archive,
        KeyCode::Char('D') => KeyAction::Delete,
        KeyCode::Char('e') => KeyAction::Edit,
        KeyCode::Char('/') => KeyAction::Filter,
        KeyCode::Char('T') => KeyAction::CycleGroup,
        KeyCode::Char('c') => KeyAction::Copy,
        KeyCode::Char('?') => KeyAction::Help,
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Esc => KeyAction::Quit,
        KeyCode::Tab => KeyAction::CycleNext,
        KeyCode::BackTab => KeyAction::CyclePrev,
        KeyCode::Enter => KeyAction::Confirm,
        KeyCode::Backspace => KeyAction::Backspace,
        KeyCode::Up => KeyAction::Up,
        KeyCode::Down => KeyAction::Down,
        KeyCode::Left => KeyAction::Left,
        KeyCode::Right => KeyAction::Right,
        KeyCode::Home => KeyAction::Top,
        KeyCode::End => KeyAction::Bottom,
        KeyCode::Char(c) => KeyAction::Char(c),
        _ => KeyAction::Ignore,
    }
}

/// `gg` is the only multi-key sequence we honor. The App keeps a
/// `pending_g: bool` and consults this helper to decide what the *second*
/// key means once the first one was a `g`.
pub fn resolve_after_pending_g(event: KeyEvent) -> KeyAction {
    match event.code {
        KeyCode::Char('g') => KeyAction::Top,
        _ => resolve(event),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    #[test]
    fn vim_arrows_map_to_directions() {
        assert_eq!(resolve(key(KeyCode::Char('j'))), KeyAction::Down);
        assert_eq!(resolve(key(KeyCode::Char('k'))), KeyAction::Up);
        assert_eq!(resolve(key(KeyCode::Char('h'))), KeyAction::Left);
        assert_eq!(resolve(key(KeyCode::Char('l'))), KeyAction::Right);
    }

    #[test]
    fn ctrl_d_is_delete_ctrl_d() {
        assert_eq!(resolve(ctrl(KeyCode::Char('d'))), KeyAction::DeleteCtrlD);
    }

    #[test]
    fn ctrl_c_quits() {
        assert_eq!(resolve(ctrl(KeyCode::Char('c'))), KeyAction::Quit);
    }

    #[test]
    fn esc_quits() {
        assert_eq!(resolve(key(KeyCode::Esc)), KeyAction::Quit);
    }

    #[test]
    fn shifted_letters_pick_uppercase_actions() {
        assert_eq!(resolve(key(KeyCode::Char('S'))), KeyAction::StopActive);
        assert_eq!(resolve(key(KeyCode::Char('A'))), KeyAction::Archive);
        assert_eq!(resolve(key(KeyCode::Char('D'))), KeyAction::Delete);
        assert_eq!(resolve(key(KeyCode::Char('G'))), KeyAction::Bottom);
    }

    #[test]
    fn pending_g_then_g_is_top() {
        assert_eq!(
            resolve_after_pending_g(key(KeyCode::Char('g'))),
            KeyAction::Top
        );
    }

    #[test]
    fn pending_g_then_anything_else_falls_back_to_resolve() {
        assert_eq!(
            resolve_after_pending_g(key(KeyCode::Char('j'))),
            KeyAction::Down
        );
    }

    #[test]
    fn unknown_chars_are_passthrough() {
        assert_eq!(resolve(key(KeyCode::Char('x'))), KeyAction::Char('x'));
    }
}
