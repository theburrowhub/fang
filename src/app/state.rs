use std::path::PathBuf;
use ratatui::style::{Color, Style};

/// Application mode
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search { query: String },
    MakeTarget,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Normal
    }
}

/// Which panel has focus
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    Sidebar,
    FileList,
    Preview,
}

impl Default for FocusedPanel {
    fn default() -> Self {
        FocusedPanel::FileList
    }
}

/// A single file or directory entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub is_executable: bool,
    pub extension: Option<String>,
}

/// A Make target
#[derive(Debug, Clone)]
pub struct MakeTarget {
    pub name: String,
    pub description: String,
    pub line_number: usize,
}

/// A styled span: (style, text)
#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<(Style, String)>,
}

impl StyledLine {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            spans: vec![(Style::default(), text.into())],
        }
    }

    pub fn colored(text: impl Into<String>, color: Color) -> Self {
        Self {
            spans: vec![(Style::default().fg(color), text.into())],
        }
    }
}

/// Preview panel state
#[derive(Debug, Clone, Default)]
pub enum PreviewState {
    #[default]
    None,
    Loading,
    Text {
        lines: Vec<StyledLine>,
        total_lines: usize,
    },
    Binary {
        size: u64,
        mime_hint: Option<String>,
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

/// A node in the sidebar directory tree
#[derive(Debug, Clone)]
pub struct SidebarNode {
    pub path: PathBuf,
    pub depth: usize,
    pub is_expanded: bool,
    pub is_dir: bool,
}

/// Complete application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,

    pub mode: AppMode,
    pub focused_panel: FocusedPanel,

    pub sidebar_visible: bool,
    pub preview_visible: bool,

    pub preview_state: PreviewState,
    pub preview_scroll: usize,

    pub sidebar_nodes: Vec<SidebarNode>,
    pub sidebar_selected: usize,

    pub make_targets: Vec<MakeTarget>,
    pub make_selected: usize,

    pub status_message: Option<String>,
}

impl AppState {
    pub fn new(current_dir: PathBuf) -> Self {
        Self {
            current_dir,
            entries: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::FileList,
            sidebar_visible: true,
            preview_visible: true,
            preview_state: PreviewState::None,
            preview_scroll: 0,
            sidebar_nodes: Vec::new(),
            sidebar_selected: 0,
            make_targets: Vec::new(),
            make_selected: 0,
            status_message: None,
        }
    }

    /// Returns the currently visible (filtered) entries
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        self.filtered_indices
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .collect()
    }

    /// Returns the currently selected entry, if any
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        let visible = self.visible_entries();
        visible.get(self.selected_index).copied()
    }

    /// Returns the search query if in Search mode
    pub fn search_query(&self) -> Option<&str> {
        match &self.mode {
            AppMode::Search { query } => Some(query.as_str()),
            _ => None,
        }
    }
}
