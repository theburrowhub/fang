#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    NavUp,
    NavDown,
    NavLeft,     // Go to parent directory
    NavRight,    // Enter selected directory
    ToggleSidebar,
    TogglePreview,
    OpenSearch,
    SearchInput(char),
    SearchBackspace,
    CloseSearch,
    OpenMakeModal,
    CloseMakeModal,
    MakeNavUp,
    MakeNavDown,
    RunMakeTarget,
    PreviewScrollUp,
    PreviewScrollDown,
    FocusNext,   // Tab between panels
    Noop,
}

/// Maps a crossterm key event to an Action based on the current app mode.
pub fn map_key_to_action(
    key: &crossterm::event::KeyEvent,
    mode: &crate::app::state::AppMode,
) -> Action {
    use crossterm::event::{KeyCode, KeyModifiers};
    use crate::app::state::AppMode;

    match mode {
        AppMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            KeyCode::Char('j') | KeyCode::Down => Action::NavDown,
            KeyCode::Char('k') | KeyCode::Up => Action::NavUp,
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => Action::NavLeft,
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => Action::NavRight,
            KeyCode::Char('/') => Action::OpenSearch,
            KeyCode::Char('m') | KeyCode::Char('M') => Action::OpenMakeModal,
            KeyCode::Char('s') => Action::ToggleSidebar,
            KeyCode::Char('p') => Action::TogglePreview,
            KeyCode::Tab => Action::FocusNext,
            KeyCode::PageUp => Action::PreviewScrollUp,
            KeyCode::PageDown => Action::PreviewScrollDown,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
        AppMode::Search { .. } => match key.code {
            KeyCode::Esc => Action::CloseSearch,
            KeyCode::Enter => Action::NavRight, // Enter selected search result
            KeyCode::Down | KeyCode::Char('j') => Action::NavDown,
            KeyCode::Up | KeyCode::Char('k') => Action::NavUp,
            KeyCode::Backspace => Action::SearchBackspace,
            // Ctrl+C must come before the unguarded Char(c) arm, or it is unreachable.
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(c) => Action::SearchInput(c),
            _ => Action::Noop,
        },
        AppMode::MakeTarget => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::CloseMakeModal,
            KeyCode::Enter => Action::RunMakeTarget,
            KeyCode::Down | KeyCode::Char('j') => Action::MakeNavDown,
            KeyCode::Up | KeyCode::Char('k') => Action::MakeNavUp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers, KeyEvent, KeyEventKind, KeyEventState};
    use crate::app::state::AppMode;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_normal_mode_quit() {
        let action = map_key_to_action(&key(KeyCode::Char('q')), &AppMode::Normal);
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_normal_mode_navigation() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('j')), &AppMode::Normal),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('k')), &AppMode::Normal),
            Action::NavUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('h')), &AppMode::Normal),
            Action::NavLeft
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('l')), &AppMode::Normal),
            Action::NavRight
        ));
    }

    #[test]
    fn test_normal_mode_arrow_keys() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Down), &AppMode::Normal),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Up), &AppMode::Normal),
            Action::NavUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Left), &AppMode::Normal),
            Action::NavLeft
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Right), &AppMode::Normal),
            Action::NavRight
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Enter), &AppMode::Normal),
            Action::NavRight
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Backspace), &AppMode::Normal),
            Action::NavLeft
        ));
    }

    #[test]
    fn test_normal_mode_toggles() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('s')), &AppMode::Normal),
            Action::ToggleSidebar
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('p')), &AppMode::Normal),
            Action::TogglePreview
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Tab), &AppMode::Normal),
            Action::FocusNext
        ));
    }

    #[test]
    fn test_normal_mode_search() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('/')), &AppMode::Normal),
            Action::OpenSearch
        ));
    }

    #[test]
    fn test_normal_mode_make() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('m')), &AppMode::Normal),
            Action::OpenMakeModal
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('M')), &AppMode::Normal),
            Action::OpenMakeModal
        ));
    }

    #[test]
    fn test_normal_mode_preview_scroll() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::PageUp), &AppMode::Normal),
            Action::PreviewScrollUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::PageDown), &AppMode::Normal),
            Action::PreviewScrollDown
        ));
    }

    #[test]
    fn test_normal_mode_ctrl_c_quit() {
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert!(matches!(
            map_key_to_action(&ctrl_c, &AppMode::Normal),
            Action::Quit
        ));
    }

    #[test]
    fn test_normal_mode_noop() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('z')), &AppMode::Normal),
            Action::Noop
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::F(1)), &AppMode::Normal),
            Action::Noop
        ));
    }

    #[test]
    fn test_search_mode_esc() {
        let mode = AppMode::Search { query: "test".to_string() };
        let action = map_key_to_action(&key(KeyCode::Esc), &mode);
        assert!(matches!(action, Action::CloseSearch));
    }

    #[test]
    fn test_search_mode_char_input() {
        let mode = AppMode::Search { query: String::new() };
        let action = map_key_to_action(&key(KeyCode::Char('a')), &mode);
        assert!(matches!(action, Action::SearchInput('a')));
    }

    #[test]
    fn test_search_mode_backspace() {
        let mode = AppMode::Search { query: "abc".to_string() };
        let action = map_key_to_action(&key(KeyCode::Backspace), &mode);
        assert!(matches!(action, Action::SearchBackspace));
    }

    #[test]
    fn test_search_mode_enter() {
        let mode = AppMode::Search { query: "foo".to_string() };
        let action = map_key_to_action(&key(KeyCode::Enter), &mode);
        assert!(matches!(action, Action::NavRight));
    }

    #[test]
    fn test_search_mode_ctrl_c_quit() {
        // Ctrl+C must be caught even in search mode (not swallowed as SearchInput).
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let mode = AppMode::Search { query: String::new() };
        assert!(matches!(map_key_to_action(&ctrl_c, &mode), Action::Quit));
    }

    #[test]
    fn test_search_mode_navigation() {
        let mode = AppMode::Search { query: String::new() };
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Down), &mode),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Up), &mode),
            Action::NavUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('j')), &mode),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('k')), &mode),
            Action::NavUp
        ));
    }

    #[test]
    fn test_make_mode_enter() {
        let action = map_key_to_action(&key(KeyCode::Enter), &AppMode::MakeTarget);
        assert!(matches!(action, Action::RunMakeTarget));
    }

    #[test]
    fn test_make_mode_esc() {
        let action = map_key_to_action(&key(KeyCode::Esc), &AppMode::MakeTarget);
        assert!(matches!(action, Action::CloseMakeModal));
    }

    #[test]
    fn test_make_mode_q_closes() {
        let action = map_key_to_action(&key(KeyCode::Char('q')), &AppMode::MakeTarget);
        assert!(matches!(action, Action::CloseMakeModal));
    }

    #[test]
    fn test_make_mode_navigation() {
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Down), &AppMode::MakeTarget),
            Action::MakeNavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Up), &AppMode::MakeTarget),
            Action::MakeNavUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('j')), &AppMode::MakeTarget),
            Action::MakeNavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('k')), &AppMode::MakeTarget),
            Action::MakeNavUp
        ));
    }
}
