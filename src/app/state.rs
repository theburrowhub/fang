use std::path::PathBuf;

// Re-export the canonical FileEntry from fs::metadata so all modules can use
// a single type. app::state::FileEntry == fs::metadata::FileEntry.
pub use crate::fs::metadata::FileEntry;

// ─── MakeTarget ───────────────────────────────────────────────────────────────

/// A parsed `make` target.
#[derive(Debug, Clone)]
pub struct MakeTarget {
    pub name: String,
    /// Description from a `## …` comment on the preceding line.
    pub description: Option<String>,
    #[allow(dead_code)]
    pub line_number: usize,
}

// ─── StyledLine ───────────────────────────────────────────────────────────────

/// A syntax-highlighted line: a list of (ratatui style, text) spans.
#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<(ratatui::style::Style, String)>,
}

impl StyledLine {
    #[allow(dead_code)]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            spans: vec![(ratatui::style::Style::default(), text.into())],
        }
    }
}

// ─── AI conversation ─────────────────────────────────────────────────────────

/// Role in an AI conversation turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiRole {
    User,
    Assistant,
    /// System-generated status lines ("[done]", "[error: …]").
    Status,
}

/// A single message in the AI conversation.
#[derive(Debug, Clone)]
pub struct AiMessage {
    pub role: AiRole,
    pub text: String,
}

// ─── PreviewState ─────────────────────────────────────────────────────────────

/// What is currently shown in the preview panel.
#[derive(Debug, Clone)]
pub enum PreviewState {
    None,
    Loading,
    Text {
        lines: Vec<StyledLine>,
        total_lines: usize,
    },
    Binary {
        size: u64,
        mime_hint: String,
    },
    Directory {
        entry_count: usize,
        total_size: u64,
    },
    MakeOutput {
        output: Vec<String>,
    },
    TooLarge {
        size: u64,
    },
    Error(String),
}

// ─── HeaderInfo ───────────────────────────────────────────────────────────────

/// Git branch and development environment information shown in the header bar.
#[derive(Debug, Clone, Default)]
pub struct HeaderInfo {
    /// Current git branch name, if the directory is inside a git repo.
    pub git_branch: Option<String>,
    /// Detected development environment tools: (display_name, version_string)
    /// e.g. [("py", "3.11.2"), ("go", "1.22.0"), ("node", "20.11.0")]
    pub dev_envs: Vec<(String, String)>,
}

// ─── AppMode ──────────────────────────────────────────────────────────────────

/// Current input mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    /// Fuzzy-search mode. `query` mirrors `AppState::search_query`.
    Search {
        query: String,
    },
    /// Make-target selection modal is open.
    MakeTarget,
    /// Shell command input prompt (activated with `:`).
    CommandInput {
        cmd: String,
    },
    /// External shell command prompt (activated with `;`).
    /// Runs the command in a new terminal split/pane.
    ExternalCommand {
        cmd: String,
    },
    /// Git operations menu modal.
    GitMenu {
        selected: usize,
    },
    /// Creating a new file. `from_clipboard` = true means paste clipboard content.
    NewFile {
        name: String,
        from_clipboard: bool,
    },
    /// AI prompt input (activated with `i`).
    AiPrompt {
        prompt: String,
    },
    /// AI provider selection modal (activated with `I` or first-time `i`).
    AiProviderSelect {
        selected: usize,
    },
}

// ─── FocusedPanel ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    Sidebar,
    FileList,
    Preview,
    AiChat,
}

// ─── SidebarNode ─────────────────────────────────────────────────────────────

/// One node in the sidebar breadcrumb tree.
#[derive(Debug, Clone)]
pub struct SidebarNode {
    pub path: PathBuf,
    pub depth: usize,
    pub is_expanded: bool,
    pub is_dir: bool,
}

// ─── AppState ─────────────────────────────────────────────────────────────────

/// Single source of truth for the entire application.
///
/// Owned exclusively by the main event loop — no `Arc<Mutex<_>>` needed.
#[derive(Debug)]
pub struct AppState {
    // Navigation
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub file_list_scroll: usize,

    // Sidebar
    pub sidebar_tree: Vec<SidebarNode>,
    pub sidebar_selected: usize,

    // Preview
    pub preview_state: PreviewState,
    pub preview_scroll: usize,

    // Modes & focus
    pub mode: AppMode,
    pub focused_panel: FocusedPanel,

    // Search
    /// The raw characters typed so far. `AppMode::Search { query }` mirrors this.
    pub search_query: String,
    /// Indices into `entries` that pass the current fuzzy filter, ordered by score.
    /// When `search_query` is empty this is effectively `0..entries.len()`.
    pub filtered_indices: Vec<usize>,

    // Make modal
    pub make_targets: Vec<MakeTarget>,
    pub make_target_selected: usize,
    pub make_output: Vec<String>,

    // Layout toggles
    pub sidebar_visible: bool,
    pub preview_visible: bool,

    // Status bar
    pub status_message: Option<String>,

    // Header info (git branch + dev envs)
    pub header_info: HeaderInfo,

    // Control
    pub should_quit: bool,
    /// Request a full terminal clear before the next draw (clears syntect artifact cells).
    pub needs_terminal_clear: bool,
    /// Stdin pipe for the currently-running : command.
    /// While Some, keypresses are relayed to the child process instead of navigating.
    pub command_stdin: Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>,

    // AI integration
    /// Currently configured AI provider (loaded from config or selected interactively).
    pub ai_provider: Option<crate::commands::ai::AiProviderConfig>,
    /// List of detected AI providers (populated on first detection).
    pub ai_providers: Vec<crate::commands::ai::AiProvider>,
    /// AI conversation history (persists across navigation).
    pub ai_conversation: Vec<AiMessage>,
    /// Whether the AI chat panel is visible.
    pub ai_panel_visible: bool,
    /// Scroll position within the AI chat panel.
    pub ai_scroll: usize,
    /// Total display lines in the AI chat (set by the renderer each frame).
    pub ai_total_lines: std::cell::Cell<usize>,
    /// Visible height of the AI chat inner area (set by the renderer each frame).
    pub ai_view_height: std::cell::Cell<usize>,
    /// Whether an AI response is currently streaming.
    pub ai_streaming: bool,
}

impl AppState {
    pub fn new(initial_dir: PathBuf) -> Self {
        Self {
            current_dir: initial_dir,
            entries: vec![],
            selected_index: 0,
            file_list_scroll: 0,
            sidebar_tree: vec![],
            sidebar_selected: 0,
            preview_state: PreviewState::None,
            preview_scroll: 0,
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::FileList,
            search_query: String::new(),
            filtered_indices: vec![],
            make_targets: vec![],
            make_target_selected: 0,
            make_output: vec![],
            sidebar_visible: true,
            preview_visible: true,
            status_message: None,
            header_info: HeaderInfo::default(),
            should_quit: false,
            needs_terminal_clear: false,
            command_stdin: None,
            ai_provider: crate::commands::ai::load_config(),
            ai_providers: vec![],
            ai_conversation: vec![],
            ai_panel_visible: false,
            ai_scroll: usize::MAX,
            ai_total_lines: std::cell::Cell::new(0),
            ai_view_height: std::cell::Cell::new(0),
            ai_streaming: false,
        }
    }

    // ── Visibility helpers ────────────────────────────────────────────────────

    /// Returns the entries that should be shown given the current search filter.
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        if self.search_query.is_empty() {
            self.entries.iter().collect()
        } else {
            self.filtered_indices
                .iter()
                .map(|&i| &self.entries[i])
                .collect()
        }
    }

    /// Number of visible entries without allocating a `Vec`.
    pub fn visible_count(&self) -> usize {
        if self.search_query.is_empty() {
            self.entries.len()
        } else {
            self.filtered_indices.len()
        }
    }

    /// The entry currently under the cursor, if any.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        if self.search_query.is_empty() {
            self.entries.get(self.selected_index)
        } else {
            self.filtered_indices
                .get(self.selected_index)
                .and_then(|&i| self.entries.get(i))
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, is_dir: bool) -> FileEntry {
        use crate::fs::metadata::FileType;
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            is_dir,
            is_symlink: false,
            size: 100,
            is_executable: false,
            extension: name.rsplit('.').next().map(|s| s.to_string()),
            file_type: if is_dir {
                FileType::Directory
            } else {
                FileType::Unknown
            },
            modified: None,
        }
    }

    #[test]
    fn test_new_state() {
        let state = AppState::new(PathBuf::from("."));
        assert!(state.entries.is_empty());
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_visible_entries_no_filter() {
        let mut s = AppState::new(PathBuf::from("."));
        s.entries = vec![make_entry("a.rs", false), make_entry("b.rs", false)];
        assert_eq!(s.visible_entries().len(), 2);
        assert_eq!(s.visible_count(), 2);
    }

    #[test]
    fn test_visible_entries_with_filter() {
        let mut s = AppState::new(PathBuf::from("."));
        s.entries = vec![
            make_entry("main.rs", false),
            make_entry("lib.rs", false),
            make_entry("Cargo.toml", false),
        ];
        s.search_query = "rs".to_string();
        s.filtered_indices = vec![0, 1];
        assert_eq!(s.visible_count(), 2);
        let visible = s.visible_entries();
        assert_eq!(visible[0].name, "main.rs");
        assert_eq!(visible[1].name, "lib.rs");
    }

    #[test]
    fn test_selected_entry_with_filter() {
        let mut s = AppState::new(PathBuf::from("."));
        s.entries = vec![make_entry("main.rs", false), make_entry("lib.rs", false)];
        s.search_query = "rs".to_string();
        s.filtered_indices = vec![0, 1];
        s.selected_index = 1;
        assert_eq!(s.selected_entry().unwrap().name, "lib.rs");
    }
}
