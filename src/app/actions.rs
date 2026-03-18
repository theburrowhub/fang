#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    NavUp,
    NavDown,
    NavLeft,  // Go to parent directory
    NavRight, // Enter selected directory
    CancelMake,
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
    FocusNext, // Tab  — move focus to next  visible panel (left→right)
    FocusPrev, // Shift+Tab — move focus to prev visible panel (right→left)
    OpenCommandInput,
    CommandInputChar(char),
    CommandInputBackspace,
    RunCommand,
    CloseCommandInput,
    OpenExternalCommand,
    ExternalCommandChar(char),
    ExternalCommandBackspace,
    RunExternalCommand,
    RunExternalCommandPopup,
    CloseExternalCommand,
    // Git menu
    OpenGitMenu,
    CloseGitMenu,
    GitNavUp,
    GitNavDown,
    RunGitItem,
    // Open with system default
    OpenWithSystem,
    // Copy path
    CopyRelPath,
    CopyAbsPath,
    // New file
    OpenNewFile,
    OpenNewFileFromClipboard,
    NewFileChar(char),
    NewFileBackspace,
    CreateNewFile,
    CloseNewFile,
    // Settings editor
    OpenSettings,
    SettingsNavUp,
    SettingsNavDown,
    SettingsIncrease,
    SettingsDecrease,
    CloseSettings,
    // Git form (second screen) — opened internally; here for completeness
    #[allow(dead_code)]
    OpenGitForm,
    GitFormTabNext,
    GitFormTabPrev,
    GitFormToggle,
    GitFormChar(char),
    GitFormBackspace,
    RunGitForm,
    CloseGitForm,
    // Help panel
    OpenHelp,
    CloseHelp,
    HelpScrollUp,
    HelpScrollDown,
    // AI prompt
    OpenAiPrompt,
    AiPromptChar(char),
    AiPromptBackspace,
    RunAiPrompt,
    CloseAiPrompt,
    // AI provider selection
    OpenAiProviderSelect,
    AiProviderNavUp,
    AiProviderNavDown,
    SelectAiProvider,
    CloseAiProviderSelect,
    // AI panel
    ToggleAiPanel,
    AiScrollUp,
    AiScrollDown,
    ResetAiSession,
    Noop,
}

/// Maps a crossterm key event to an Action based on the current app mode and focused panel.
pub fn map_key_to_action(
    key: &crossterm::event::KeyEvent,
    mode: &crate::app::state::AppMode,
    focused_panel: &crate::app::state::FocusedPanel,
) -> Action {
    use crate::app::state::{AppMode, FocusedPanel};
    use crossterm::event::{KeyCode, KeyModifiers};

    match mode {
        AppMode::Normal if *focused_panel == FocusedPanel::AiChat => match key.code {
            // j/k and vertical arrows scroll the AI chat panel.
            KeyCode::Char('j') | KeyCode::Down => Action::AiScrollDown,
            KeyCode::Char('k') | KeyCode::Up => Action::AiScrollUp,
            KeyCode::PageDown => Action::AiScrollDown,
            KeyCode::PageUp => Action::AiScrollUp,
            // Directory navigation always works regardless of which panel is focused.
            KeyCode::Char('u') | KeyCode::Left | KeyCode::Backspace => Action::NavLeft,
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => Action::NavRight,
            // Other universal keys
            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Esc => Action::CancelMake,
            KeyCode::Tab => Action::FocusNext,
            KeyCode::BackTab => Action::FocusPrev,
            KeyCode::Char('a') => Action::ToggleAiPanel,
            KeyCode::Char('i') => Action::OpenAiPrompt,
            KeyCode::Char('I') => Action::OpenAiProviderSelect,
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ResetAiSession
            }
            _ => Action::Noop,
        },
        AppMode::Normal => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            // Esc / Ctrl+C cancel an in-flight make target (no-op when make is idle).
            KeyCode::Esc => Action::CancelMake,
            // When preview panel has focus, j/k/arrows scroll the content.
            KeyCode::Char('j') | KeyCode::Down if *focused_panel == FocusedPanel::Preview => {
                Action::PreviewScrollDown
            }
            KeyCode::Char('k') | KeyCode::Up if *focused_panel == FocusedPanel::Preview => {
                Action::PreviewScrollUp
            }
            KeyCode::Char('j') | KeyCode::Down => Action::NavDown,
            KeyCode::Char('k') | KeyCode::Up => Action::NavUp,
            // 'h' opens Help; 'u' (and arrow/backspace) go to parent directory.
            KeyCode::Char('h') => Action::OpenHelp,
            KeyCode::Char('u') | KeyCode::Left | KeyCode::Backspace => Action::NavLeft,
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => Action::NavRight,
            KeyCode::Char('/') => Action::OpenSearch,
            KeyCode::Char('m') | KeyCode::Char('M') => Action::OpenMakeModal,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::OpenSettings
            }
            KeyCode::Char('p') => Action::TogglePreview,
            KeyCode::Tab => Action::FocusNext,
            KeyCode::BackTab => Action::FocusPrev,
            KeyCode::PageUp => Action::PreviewScrollUp,
            KeyCode::PageDown => Action::PreviewScrollDown,
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ResetAiSession
            }
            KeyCode::Char(':') => Action::OpenCommandInput,
            KeyCode::Char(';') => Action::OpenExternalCommand,
            KeyCode::Char('g') | KeyCode::Char('G') => Action::OpenGitMenu,
            KeyCode::Char('c') => Action::CopyRelPath,
            KeyCode::Char('C') => Action::CopyAbsPath,
            KeyCode::Char('o') => Action::OpenWithSystem,
            KeyCode::Char('n') => Action::OpenNewFile,
            KeyCode::Char('N') => Action::OpenNewFileFromClipboard,
            KeyCode::Char('a') => Action::ToggleAiPanel,
            KeyCode::Char('i') => Action::OpenAiPrompt,
            KeyCode::Char('I') => Action::OpenAiProviderSelect,
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
        AppMode::CommandInput { .. } => match key.code {
            KeyCode::Esc => Action::CloseCommandInput,
            KeyCode::Enter => Action::RunCommand,
            KeyCode::Backspace => Action::CommandInputBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(c) => Action::CommandInputChar(c),
            _ => Action::Noop,
        },
        AppMode::ExternalCommand { .. } => match key.code {
            KeyCode::Esc => Action::CloseExternalCommand,
            KeyCode::Enter => Action::RunExternalCommand,
            KeyCode::Backspace => Action::ExternalCommandBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::RunExternalCommandPopup
            }
            KeyCode::Char(c) => Action::ExternalCommandChar(c),
            _ => Action::Noop,
        },
        AppMode::GitMenu { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::CloseGitMenu,
            KeyCode::Enter => Action::RunGitItem,
            KeyCode::Down | KeyCode::Char('j') => Action::GitNavDown,
            KeyCode::Up | KeyCode::Char('k') => Action::GitNavUp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
        AppMode::NewFile { .. } => match key.code {
            KeyCode::Esc => Action::CloseNewFile,
            KeyCode::Enter => Action::CreateNewFile,
            KeyCode::Backspace => Action::NewFileBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(c) => Action::NewFileChar(c),
            _ => Action::Noop,
        },
        AppMode::GitForm { .. } => match key.code {
            KeyCode::Esc => Action::CloseGitForm,
            KeyCode::Enter => Action::RunGitForm,
            KeyCode::Tab => Action::GitFormTabNext,
            KeyCode::BackTab => Action::GitFormTabPrev,
            KeyCode::Char(' ') => Action::GitFormToggle,
            KeyCode::Backspace => Action::GitFormBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(ch) => Action::GitFormChar(ch),
            _ => Action::Noop,
        },
        AppMode::Help { .. } => match key.code {
            KeyCode::Char('h') | KeyCode::Esc | KeyCode::Char('q') => Action::CloseHelp,
            KeyCode::Char('j') | KeyCode::Down => Action::HelpScrollDown,
            KeyCode::Char('k') | KeyCode::Up => Action::HelpScrollUp,
            KeyCode::PageDown => Action::HelpScrollDown,
            KeyCode::PageUp => Action::HelpScrollUp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
        AppMode::Settings { .. } => match key.code {
            KeyCode::Esc | KeyCode::Enter => Action::CloseSettings,
            KeyCode::Char('j') | KeyCode::Down => Action::SettingsNavDown,
            KeyCode::Char('k') | KeyCode::Up => Action::SettingsNavUp,
            KeyCode::Char('+') | KeyCode::Right => Action::SettingsIncrease,
            KeyCode::Char('-') | KeyCode::Left => Action::SettingsDecrease,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
        AppMode::AiPrompt { .. } => match key.code {
            KeyCode::Esc => Action::CloseAiPrompt,
            KeyCode::Enter => Action::RunAiPrompt,
            KeyCode::Backspace => Action::AiPromptBackspace,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char(c) => Action::AiPromptChar(c),
            _ => Action::Noop,
        },
        AppMode::AiProviderSelect { .. } => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::CloseAiProviderSelect,
            KeyCode::Enter => Action::SelectAiProvider,
            KeyCode::Down | KeyCode::Char('j') => Action::AiProviderNavDown,
            KeyCode::Up | KeyCode::Char('k') => Action::AiProviderNavUp,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            _ => Action::Noop,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::AppMode;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
        let action = map_key_to_action(
            &key(KeyCode::Char('q')),
            &AppMode::Normal,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_normal_mode_navigation() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('j')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('k')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavUp
        ));
        // 'h' now opens Help; 'u' navigates up to parent directory
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('h')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenHelp
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('u')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavLeft
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('l')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavRight
        ));
    }

    #[test]
    fn test_normal_mode_arrow_keys() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Down),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Up),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavUp
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Left),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavLeft
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Right),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavRight
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Enter),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavRight
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Backspace),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavLeft
        ));
    }

    #[test]
    fn test_normal_mode_toggles() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('p')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::TogglePreview
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Tab),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::FocusNext
        ));
    }

    #[test]
    fn test_normal_mode_search() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('/')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenSearch
        ));
    }

    #[test]
    fn test_normal_mode_make() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('m')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenMakeModal
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('M')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenMakeModal
        ));
    }

    #[test]
    fn test_normal_mode_preview_scroll() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::PageUp),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::PreviewScrollUp
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::PageDown),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
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
            map_key_to_action(
                &ctrl_c,
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::Quit
        ));
    }

    #[test]
    fn test_normal_mode_noop() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('z')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::Noop
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::F(1)),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::Noop
        ));
    }

    #[test]
    fn test_search_mode_esc() {
        let mode = AppMode::Search {
            query: "test".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Esc),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CloseSearch));
    }

    #[test]
    fn test_search_mode_char_input() {
        let mode = AppMode::Search {
            query: String::new(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Char('a')),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::SearchInput('a')));
    }

    #[test]
    fn test_search_mode_backspace() {
        let mode = AppMode::Search {
            query: "abc".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Backspace),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::SearchBackspace));
    }

    #[test]
    fn test_search_mode_enter() {
        let mode = AppMode::Search {
            query: "foo".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Enter),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
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
        let mode = AppMode::Search {
            query: String::new(),
        };
        assert!(matches!(
            map_key_to_action(&ctrl_c, &mode, &crate::app::state::FocusedPanel::FileList),
            Action::Quit
        ));
    }

    #[test]
    fn test_search_mode_navigation() {
        let mode = AppMode::Search {
            query: String::new(),
        };
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Down),
                &mode,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Up),
                &mode,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavUp
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('j')),
                &mode,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('k')),
                &mode,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::NavUp
        ));
    }

    #[test]
    fn test_make_mode_enter() {
        let action = map_key_to_action(
            &key(KeyCode::Enter),
            &AppMode::MakeTarget,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::RunMakeTarget));
    }

    #[test]
    fn test_make_mode_esc() {
        let action = map_key_to_action(
            &key(KeyCode::Esc),
            &AppMode::MakeTarget,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CloseMakeModal));
    }

    #[test]
    fn test_make_mode_q_closes() {
        let action = map_key_to_action(
            &key(KeyCode::Char('q')),
            &AppMode::MakeTarget,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CloseMakeModal));
    }

    #[test]
    fn test_make_mode_navigation() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Down),
                &AppMode::MakeTarget,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::MakeNavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Up),
                &AppMode::MakeTarget,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::MakeNavUp
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('j')),
                &AppMode::MakeTarget,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::MakeNavDown
        ));
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('k')),
                &AppMode::MakeTarget,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::MakeNavUp
        ));
    }

    #[test]
    fn test_colon_opens_command_input() {
        let action = map_key_to_action(
            &key(KeyCode::Char(':')),
            &AppMode::Normal,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::OpenCommandInput));
    }

    #[test]
    fn test_command_input_mode_enter_runs() {
        let mode = AppMode::CommandInput {
            cmd: "ls".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Enter),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::RunCommand));
    }

    #[test]
    fn test_command_input_mode_esc_closes() {
        let mode = AppMode::CommandInput {
            cmd: "ls".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Esc),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CloseCommandInput));
    }

    #[test]
    fn test_command_input_mode_char_input() {
        let mode = AppMode::CommandInput { cmd: String::new() };
        let action = map_key_to_action(
            &key(KeyCode::Char('a')),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CommandInputChar('a')));
    }

    #[test]
    fn test_command_input_mode_backspace() {
        let mode = AppMode::CommandInput {
            cmd: "ls".to_string(),
        };
        let action = map_key_to_action(
            &key(KeyCode::Backspace),
            &mode,
            &crate::app::state::FocusedPanel::FileList,
        );
        assert!(matches!(action, Action::CommandInputBackspace));
    }

    #[test]
    fn test_command_input_mode_ctrl_c_quit() {
        let ctrl_c = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let mode = AppMode::CommandInput { cmd: String::new() };
        assert!(matches!(
            map_key_to_action(&ctrl_c, &mode, &crate::app::state::FocusedPanel::FileList),
            Action::Quit
        ));
    }

    #[test]
    fn test_i_opens_ai_in_normal_mode() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('i')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenAiPrompt
        ));
    }

    #[test]
    fn test_shift_i_opens_ai_provider_select() {
        assert!(matches!(
            map_key_to_action(
                &key(KeyCode::Char('I')),
                &AppMode::Normal,
                &crate::app::state::FocusedPanel::FileList
            ),
            Action::OpenAiProviderSelect
        ));
    }

    #[test]
    fn test_ai_prompt_mode_key_mapping() {
        let mode = AppMode::AiPrompt {
            prompt: String::new(),
        };
        let panel = crate::app::state::FocusedPanel::FileList;
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Esc), &mode, &panel),
            Action::CloseAiPrompt
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Enter), &mode, &panel),
            Action::RunAiPrompt
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Backspace), &mode, &panel),
            Action::AiPromptBackspace
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('x')), &mode, &panel),
            Action::AiPromptChar('x')
        ));
    }

    #[test]
    fn test_ai_provider_select_key_mapping() {
        let mode = AppMode::AiProviderSelect { selected: 0 };
        let panel = crate::app::state::FocusedPanel::FileList;
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Esc), &mode, &panel),
            Action::CloseAiProviderSelect
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('q')), &mode, &panel),
            Action::CloseAiProviderSelect
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Enter), &mode, &panel),
            Action::SelectAiProvider
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Down), &mode, &panel),
            Action::AiProviderNavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Up), &mode, &panel),
            Action::AiProviderNavUp
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('j')), &mode, &panel),
            Action::AiProviderNavDown
        ));
        assert!(matches!(
            map_key_to_action(&key(KeyCode::Char('k')), &mode, &panel),
            Action::AiProviderNavUp
        ));
    }
}
