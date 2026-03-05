use std::path::PathBuf;

// Re-export FileEntry from fs::metadata so the rest of the codebase
// can import from app::state without a double-define.
pub use crate::fs::metadata::FileEntry;
#[allow(unused_imports)]
pub use crate::fs::metadata::FileType;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search { query: String },
    MakeTarget,
}

impl Default for AppMode {
    fn default() -> Self { AppMode::Normal }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    Sidebar,
    FileList,
    Preview,
}

impl Default for FocusedPanel {
    fn default() -> Self { FocusedPanel::FileList }
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

impl StyledLine {
    pub fn plain(text: impl Into<String>) -> Self {
        StyledLine { spans: vec![(ratatui::style::Style::default(), text.into())] }
    }
    pub fn colored(text: impl Into<String>, color: ratatui::style::Color) -> Self {
        StyledLine { spans: vec![(ratatui::style::Style::default().fg(color), text.into())] }
    }
}

#[derive(Debug, Clone, Default)]
pub enum PreviewState {
    #[default]
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
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub file_list_scroll: usize,
    pub sidebar_tree: Vec<SidebarNode>,
    pub sidebar_selected: usize,
    pub preview_state: PreviewState,
    pub preview_scroll: usize,
    pub mode: AppMode,
    pub focused_panel: FocusedPanel,
    pub search_query: String,
    pub make_targets: Vec<MakeTarget>,
    pub make_target_selected: usize,
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
            filtered_indices: vec![],
            selected_index: 0,
            file_list_scroll: 0,
            sidebar_tree: vec![],
            sidebar_selected: 0,
            preview_state: PreviewState::None,
            preview_scroll: 0,
            mode: AppMode::Normal,
            focused_panel: FocusedPanel::FileList,
            search_query: String::new(),
            make_targets: vec![],
            make_target_selected: 0,
            sidebar_visible: true,
            preview_visible: true,
            status_message: None,
            should_quit: false,
        }
    }

    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        if self.search_query.is_empty() {
            self.entries.iter().collect()
        } else {
            self.filtered_indices.iter().filter_map(|&i| self.entries.get(i)).collect()
        }
    }

    pub fn visible_count(&self) -> usize {
        if self.search_query.is_empty() { self.entries.len() }
        else { self.filtered_indices.len() }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        if self.search_query.is_empty() {
            self.entries.get(self.selected_index)
        } else {
            self.filtered_indices.get(self.selected_index).and_then(|&i| self.entries.get(i))
        }
    }

    pub fn search_query(&self) -> Option<&str> {
        match &self.mode {
            AppMode::Search { query } => Some(query.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_state() -> AppState { AppState::new(PathBuf::from("/tmp")) }

    #[test]
    fn test_default_mode() {
        let state = make_state();
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(state.focused_panel, FocusedPanel::FileList);
    }
    #[test]
    fn test_new_state_empty() {
        let state = make_state();
        assert!(state.entries.is_empty());
        assert_eq!(state.selected_index, 0);
        assert!(!state.should_quit);
    }
    #[test]
    fn test_search_query_method() {
        let mut state = make_state();
        assert!(state.search_query().is_none());
        state.mode = AppMode::Search { query: "hello".to_string() };
        assert_eq!(state.search_query(), Some("hello"));
    }
    #[test]
    fn test_visible_count_no_search() {
        let mut state = make_state();
        let cwd = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(cwd).unwrap();
        state.entries = vec![entry.clone(), entry];
        assert_eq!(state.visible_count(), 2);
    }
    #[test]
    fn test_visible_count_with_search() {
        let mut state = make_state();
        let cwd = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(cwd).unwrap();
        state.entries = vec![entry.clone(), entry];
        state.filtered_indices = vec![0];
        state.search_query = "q".to_string();
        assert_eq!(state.visible_count(), 1);
    }
    #[test]
    fn test_selected_entry_none_when_empty() {
        let state = make_state();
        assert!(state.selected_entry().is_none());
    }
}
