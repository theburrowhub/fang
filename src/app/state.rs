use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search { query: String },
    MakeTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    Sidebar,
    FileList,
    Preview,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub is_executable: bool,
    pub extension: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MakeTarget {
    pub name: String,
    pub description: Option<String>,
    pub line_number: usize,
}

#[derive(Debug, Clone)]
pub struct StyledLine {
    pub spans: Vec<(ratatui::style::Style, String)>,
}

#[derive(Debug, Clone)]
pub enum PreviewState {
    None,
    Loading,
    Text { lines: Vec<StyledLine>, total_lines: usize },
    Binary { size: u64, mime_hint: String },
    Directory { entry_count: usize, total_size: u64 },
    MakeOutput { output: Vec<String> },
    TooLarge { size: u64 },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SidebarNode {
    pub path: PathBuf,
    pub depth: usize,
    pub is_expanded: bool,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct AppState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub file_list_scroll: usize,
    pub sidebar_tree: Vec<SidebarNode>,
    pub sidebar_selected: usize,
    pub preview_state: PreviewState,
    pub preview_scroll: usize,
    pub mode: AppMode,
    pub focused_panel: FocusedPanel,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
    pub make_targets: Vec<MakeTarget>,
    pub make_target_selected: usize,
    pub make_output: Vec<String>,
    pub sidebar_visible: bool,
    pub preview_visible: bool,
    pub status_message: Option<String>,
    pub should_quit: bool,
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
            should_quit: false,
        }
    }

    pub fn visible_entries(&self) -> Vec<(usize, &FileEntry)> {
        if self.search_query.is_empty() {
            self.entries.iter().enumerate().collect()
        } else {
            self.filtered_indices.iter().map(|&i| (i, &self.entries[i])).collect()
        }
    }

    /// Returns the number of visible entries without allocating a Vec.
    pub fn visible_count(&self) -> usize {
        if self.search_query.is_empty() {
            self.entries.len()
        } else {
            self.filtered_indices.len()
        }
    }

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
