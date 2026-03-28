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

// ─── Image protocol slot ──────────────────────────────────────────────────────

/// Holds a ratatui-image protocol for one image in the preview.
/// `None` until the image is first rendered (protocol is created lazily at render time).
pub struct ImageProtocolSlot {
    pub protocol: Option<ratatui_image::protocol::StatefulProtocol>,
}

impl std::fmt::Debug for ImageProtocolSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImageProtocolSlot")
    }
}

// ─── RichMarkdown ─────────────────────────────────────────────────────────────

/// A pre-rendered image embedded in a Markdown file.
/// PNG bytes were produced async (mermaid-rs-renderer → resvg, or image::open).
pub struct RenderedImage {
    pub alt: String,
    pub png: Vec<u8>,
}

impl std::fmt::Debug for RenderedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RenderedImage({} bytes, alt={:?})",
            self.png.len(),
            self.alt
        )
    }
}

impl Clone for RenderedImage {
    fn clone(&self) -> Self {
        Self {
            alt: self.alt.clone(),
            png: self.png.clone(),
        }
    }
}

/// One block in a rendered-at-draw-time Markdown preview.
#[derive(Clone)]
pub enum MarkdownItem {
    Text(Vec<StyledLine>),
    Image { png: Vec<u8>, alt: String },
}

impl std::fmt::Debug for MarkdownItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(l) => write!(f, "Text({} lines)", l.len()),
            Self::Image { alt, png } => write!(f, "Image({} bytes, {:?})", png.len(), alt),
        }
    }
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
    /// Git diff of the selected file (activated with `d`).
    GitDiff {
        lines: Vec<StyledLine>,
    },
    /// Rich Markdown: source stored for lazy text rendering at actual panel width.
    /// Text is re-rendered (fast, pure Rust) each time the panel width changes.
    /// Images are pre-rendered async and stored as PNG bytes.
    RichMarkdown {
        /// Raw Markdown source — used to re-render text at the actual panel width.
        source: String,
        /// Directory of the .md file, for resolving relative `![](path)` URLs.
        base_dir: Option<std::path::PathBuf>,
        /// Pre-rendered images in source order (mermaid blocks + embedded images).
        images: Vec<RenderedImage>,
        total_lines: usize,
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
    /// Git operations menu modal (first screen).
    GitMenu {
        selected: usize,
    },
    /// Git form (second screen — parameters for an operation).
    GitForm {
        /// Index into `git_operations()` of the selected operation.
        op_index: usize,
        /// Current values for each parameter in the form.
        values: Vec<crate::commands::git::GitParamValue>,
        /// Index of the currently focused control.
        focused: usize,
    },
    /// Creating a new file. `from_clipboard` = true means paste clipboard content.
    NewFile {
        name: String,
        from_clipboard: bool,
    },
    /// Full-screen help overlay (opened with `h`).
    Help {
        /// Vertical scroll offset.
        scroll: usize,
    },
    /// Settings editor (opened with Ctrl+S).
    Settings {
        selected: usize,
        entries: Vec<crate::config::SettingEntry>,
    },
    /// AI prompt input (activated with `i`).
    AiPrompt {
        prompt: String,
    },
    /// AI provider selection modal (activated with `I` or first-time `i`).
    AiProviderSelect {
        selected: usize,
    },
    /// Spotlight-style command palette (activated with `Ctrl+K`).
    CommandPalette {
        /// Live search query typed by the user.
        query: String,
        /// Index of the highlighted item in the filtered list.
        selected: usize,
    },
}

// ─── FocusedPanel ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    FileList,
    Preview,
    AiChat,
}

// ─── Git file status ──────────────────────────────────────────────────────────

/// Git working-tree / index status of a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFileStatus {
    /// Staged new file (A in index)
    Added,
    /// Modified in index or worktree (M)
    Modified,
    /// Deleted in index or worktree (D)
    Deleted,
    /// Renamed or copied (R / C)
    Renamed,
    /// Untracked (??)
    Untracked,
    /// Unmerged / conflict
    Conflict,
}

impl GitFileStatus {
    /// Single-char indicator shown in the file list.
    pub fn indicator(&self) -> char {
        match self {
            Self::Added => '+',
            Self::Modified => '~',
            Self::Deleted => '-',
            Self::Renamed => '»',
            Self::Untracked => '?',
            Self::Conflict => '!',
        }
    }

    /// Ratatui style for the indicator.
    pub fn style(&self) -> ratatui::style::Style {
        use ratatui::style::{Color, Style};
        match self {
            Self::Added => Style::default().fg(Color::Green),
            Self::Modified => Style::default().fg(Color::Yellow),
            Self::Deleted => Style::default().fg(Color::Red),
            Self::Renamed => Style::default().fg(Color::Cyan),
            Self::Untracked => Style::default().fg(Color::DarkGray),
            Self::Conflict => Style::default().fg(Color::Red),
        }
    }
}

// ─── AppState ─────────────────────────────────────────────────────────────────

/// Single source of truth for the entire application.
///
/// Owned exclusively by the main event loop — no `Arc<Mutex<_>>` needed.
#[derive(Debug)]
pub struct AppState {
    // Navigation
    pub current_dir: PathBuf,
    /// The directory fang was opened in — navigation cannot go above this.
    pub root_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub file_list_scroll: usize,

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
    /// Cancellation sender for the in-flight `make` task.
    /// `Some` while make is running; `None` when idle.
    pub make_cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,

    // Layout toggles
    pub preview_visible: bool,
    /// When true the preview panel shows the git diff instead of file content.
    pub preview_git_diff: bool,

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

    // Configuration
    /// Loaded and live-updated application configuration.
    pub config: crate::config::Config,

    // Git file status
    /// Map from absolute path → git status, refreshed on every directory navigation.
    pub git_file_status: std::collections::HashMap<std::path::PathBuf, GitFileStatus>,

    // Image rendering (ratatui-image)
    /// Terminal-image picker — detects protocol (Kitty, iTerm2, Sixel, half-block).
    /// `None` if the terminal does not support any image protocol or init failed.
    pub image_picker: Option<ratatui_image::picker::Picker>,
    /// Per-image render state for the current `RichMarkdown` preview.
    /// Wrapped in `RefCell` so the render function can mutate state through `&AppState`.
    pub image_protocols: std::cell::RefCell<Vec<ImageProtocolSlot>>,
    /// Cached result of `render_markdown_rich` for the current `RichMarkdown` preview,
    /// keyed by panel width.  Cleared whenever `PreviewState` changes.
    /// Allows text to be re-rendered at the exact panel width without re-rendering images.
    pub markdown_text_cache: std::cell::RefCell<Option<(u16, Vec<MarkdownItem>)>>,

    // MSLP — pass --dangerously-skip-permissions to Claude CLI
    pub mslp_enabled: bool,

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
    /// Maximum scroll offset (total visual lines − view height). Updated each frame.
    pub ai_max_scroll: usize,
    /// Whether an AI response is currently streaming.
    pub ai_streaming: bool,
    /// Handle to abort the in-flight AI streaming task.
    pub ai_task_handle: Option<tokio::task::AbortHandle>,
    /// Last-known dimensions of the AI panel area (set before draw).
    /// Used by `ai_panel::update_max_scroll` so the render path stays `&AppState`.
    pub ai_panel_width: u16,
    pub ai_panel_height: u16,
}

impl AppState {
    pub fn new(
        initial_dir: PathBuf,
        ai_config: Option<crate::commands::ai::AiProviderConfig>,
    ) -> Self {
        Self {
            current_dir: initial_dir.clone(),
            root_dir: initial_dir,
            entries: vec![],
            selected_index: 0,
            file_list_scroll: 0,
            preview_state: PreviewState::None,
            preview_scroll: 0,
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::FileList,
            search_query: String::new(),
            filtered_indices: vec![],
            make_targets: vec![],
            make_target_selected: 0,
            make_output: vec![],
            make_cancel_tx: None,
            preview_visible: true,
            preview_git_diff: false,
            status_message: None,
            header_info: HeaderInfo::default(),
            should_quit: false,
            needs_terminal_clear: false,
            command_stdin: None,
            config: crate::config::Config::default(),
            git_file_status: std::collections::HashMap::new(),
            image_picker: None,
            image_protocols: std::cell::RefCell::new(vec![]),
            markdown_text_cache: std::cell::RefCell::new(None),
            mslp_enabled: false,
            ai_provider: ai_config,
            ai_providers: vec![],
            ai_conversation: vec![],
            ai_panel_visible: false,
            ai_scroll: usize::MAX,
            ai_max_scroll: 0,
            ai_streaming: false,
            ai_task_handle: None,
            ai_panel_width: 0,
            ai_panel_height: 0,
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
        let state = AppState::new(PathBuf::from("."), None);
        assert!(state.entries.is_empty());
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_visible_entries_no_filter() {
        let mut s = AppState::new(PathBuf::from("."), None);
        s.entries = vec![make_entry("a.rs", false), make_entry("b.rs", false)];
        assert_eq!(s.visible_entries().len(), 2);
        assert_eq!(s.visible_count(), 2);
    }

    #[test]
    fn test_visible_entries_with_filter() {
        let mut s = AppState::new(PathBuf::from("."), None);
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
        let mut s = AppState::new(PathBuf::from("."), None);
        s.entries = vec![make_entry("main.rs", false), make_entry("lib.rs", false)];
        s.search_query = "rs".to_string();
        s.filtered_indices = vec![0, 1];
        s.selected_index = 1;
        assert_eq!(s.selected_entry().unwrap().name, "lib.rs");
    }
}
