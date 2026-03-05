#[derive(Debug, Clone)]
pub enum Action {
    Quit, NavUp, NavDown, NavLeft, NavRight,
    ToggleSidebar, TogglePreview,
    OpenSearch, SearchInput(char), SearchBackspace, CloseSearch,
    OpenMakeModal, CloseMakeModal, MakeNavUp, MakeNavDown, RunMakeTarget,
    PreviewScrollUp, PreviewScrollDown, FocusNext, Noop,
}

pub fn map_key_to_action(key: &crossterm::event::KeyEvent, mode: &crate::app::state::AppMode) -> Action {
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
            KeyCode::Enter => Action::NavRight,
            KeyCode::Down | KeyCode::Char('j') => Action::NavDown,
            KeyCode::Up | KeyCode::Char('k') => Action::NavUp,
            KeyCode::Backspace => Action::SearchBackspace,
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
    fn key(code: KeyCode) -> KeyEvent { KeyEvent { code, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: KeyEventState::NONE } }
    #[test]
    fn test_normal_mode_quit() { assert!(matches!(map_key_to_action(&key(KeyCode::Char('q')), &AppMode::Normal), Action::Quit)); }
    #[test]
    fn test_normal_mode_navigation() {
        assert!(matches!(map_key_to_action(&key(KeyCode::Char('j')), &AppMode::Normal), Action::NavDown));
        assert!(matches!(map_key_to_action(&key(KeyCode::Char('k')), &AppMode::Normal), Action::NavUp));
    }
    #[test]
    fn test_search_mode_esc() {
        let mode = AppMode::Search { query: "test".to_string() };
        assert!(matches!(map_key_to_action(&key(KeyCode::Esc), &mode), Action::CloseSearch));
    }
    #[test]
    fn test_make_mode_enter() { assert!(matches!(map_key_to_action(&key(KeyCode::Enter), &AppMode::MakeTarget), Action::RunMakeTarget)); }
    #[test]
    fn test_ctrl_c_quit() {
        let ctrl_c = KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, kind: KeyEventKind::Press, state: KeyEventState::NONE };
        assert!(matches!(map_key_to_action(&ctrl_c, &AppMode::Normal), Action::Quit));
    }
}
