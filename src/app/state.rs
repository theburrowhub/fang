use std::path::PathBuf;

// Re-export FileEntry from the canonical location so the rest of the codebase
// only needs to import from app::state or fs::metadata — both refer to the same type.
pub use crate::fs::metadata::FileEntry;
// FileType is re-exported for consumers that need to pattern-match on entry.file_type
// without importing from fs::metadata directly.
#[allow(unused_imports)]
pub use crate::fs::metadata::FileType;

// ---------------------------------------------------------------------------
// Application modes
// ---------------------------------------------------------------------------

/// Top-level mode for the application. Controls which input bindings are active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Normal file browsing.
    Normal,
    /// The fuzzy-search input is active.
    Search,
    /// A Makefile target is selected and running.
    MakeRunning,
    /// Showing help overlay.
    Help,
    /// Application is shutting down.
    Quitting,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Normal
    }
}

// ---------------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------------

/// Which panel currently has keyboard focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    /// The left-side directory tree / sidebar.
    Sidebar,
    /// The main file-list pane.
    FileList,
    /// The right-side preview pane.
    Preview,
}

impl Default for FocusedPanel {
    fn default() -> Self {
        FocusedPanel::FileList
    }
}

// ---------------------------------------------------------------------------
// Makefile targets
// ---------------------------------------------------------------------------

/// A single target discovered in a Makefile.
#[derive(Debug, Clone)]
pub struct MakeTarget {
    /// The target name (e.g. `build`, `test`).
    pub name: String,
    /// Optional doc-comment extracted from the line above the target.
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Preview types
// ---------------------------------------------------------------------------

/// A single styled line for the preview pane. The UI layer renders these with
/// ratatui `Span`s; here we store pre-computed styled segments.
#[derive(Debug, Clone)]
pub struct StyledLine {
    /// Raw text of the line (without ANSI codes).
    pub text: String,
    /// Optional foreground colour as (r, g, b) if syntax highlighting is active.
    pub fg: Option<(u8, u8, u8)>,
}

impl StyledLine {
    pub fn plain(text: impl Into<String>) -> Self {
        StyledLine {
            text: text.into(),
            fg: None,
        }
    }

    pub fn colored(text: impl Into<String>, r: u8, g: u8, b: u8) -> Self {
        StyledLine {
            text: text.into(),
            fg: Some((r, g, b)),
        }
    }
}

/// State for the preview pane.
#[derive(Debug, Clone, Default)]
pub enum PreviewState {
    /// Nothing to preview yet.
    #[default]
    Empty,
    /// Text/code file with optional syntax-highlighted lines.
    Text {
        lines: Vec<StyledLine>,
        /// Current vertical scroll offset.
        scroll: usize,
    },
    /// Binary file — shows a hex dump or a short description.
    Binary {
        size: u64,
        description: String,
    },
    /// Directory — shows a summary (item count, total size).
    Directory {
        item_count: usize,
        total_size: u64,
    },
    /// Media file (image/video/audio) — not renderable in terminal.
    Media { description: String },
    /// Loading in progress.
    Loading,
    /// An error occurred while loading the preview.
    Error(String),
}

// ---------------------------------------------------------------------------
// Sidebar / directory tree
// ---------------------------------------------------------------------------

/// A node in the collapsible sidebar directory tree.
#[derive(Debug, Clone)]
pub struct SidebarNode {
    pub path: PathBuf,
    pub name: String,
    pub is_expanded: bool,
    pub depth: usize,
    pub children: Vec<SidebarNode>,
}

impl SidebarNode {
    pub fn new(path: PathBuf, depth: usize) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        SidebarNode {
            path,
            name,
            is_expanded: false,
            depth,
            children: Vec::new(),
        }
    }

    /// Toggle the expanded state of this node.
    pub fn toggle(&mut self) {
        self.is_expanded = !self.is_expanded;
    }
}

// ---------------------------------------------------------------------------
// Search state
// ---------------------------------------------------------------------------

/// State for the fuzzy-search widget.
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Current query string typed by the user.
    pub query: String,
    /// Indices into the current directory listing that match the query.
    pub matches: Vec<usize>,
    /// Currently highlighted match index within `matches`.
    pub selected: usize,
}

impl SearchState {
    pub fn clear(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.selected = 0;
    }

    pub fn is_active(&self) -> bool {
        !self.query.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Main application state
// ---------------------------------------------------------------------------

/// The complete application state, owned by the top-level App struct.
#[derive(Debug)]
pub struct AppState {
    // --- Navigation ---
    /// The directory currently being listed in the file-list pane.
    pub current_dir: PathBuf,
    /// History of visited directories for backward navigation.
    pub history: Vec<PathBuf>,
    /// Stack of directories for forward navigation (populated on back-nav).
    pub forward_stack: Vec<PathBuf>,

    // --- File list ---
    /// All entries in the current directory (after sorting, before filtering).
    pub entries: Vec<FileEntry>,
    /// Whether to show hidden files (dot-files).
    pub show_hidden: bool,
    /// Index of the currently selected item in the *visible* entry list.
    pub selected_index: usize,
    /// Scroll offset for the file-list view.
    pub list_scroll: usize,

    // --- Sidebar ---
    /// Root nodes of the directory-tree sidebar.
    pub sidebar_nodes: Vec<SidebarNode>,
    /// Index of the selected node in the flat sidebar list.
    pub sidebar_selected: usize,

    // --- Preview ---
    pub preview: PreviewState,

    // --- Search ---
    pub search: SearchState,

    // --- Makefile ---
    /// Makefile targets found in the current directory (if any Makefile present).
    pub make_targets: Vec<MakeTarget>,
    /// Index of the selected Makefile target.
    pub make_selected: usize,

    // --- UI ---
    pub mode: AppMode,
    pub focused_panel: FocusedPanel,
    /// Status-bar message (shown in footer). Cleared after display.
    pub status_message: Option<String>,
}

impl AppState {
    /// Create a new AppState rooted at `start_dir`.
    pub fn new(start_dir: PathBuf) -> Self {
        AppState {
            current_dir: start_dir,
            history: Vec::new(),
            forward_stack: Vec::new(),
            entries: Vec::new(),
            show_hidden: false,
            selected_index: 0,
            list_scroll: 0,
            sidebar_nodes: Vec::new(),
            sidebar_selected: 0,
            preview: PreviewState::Empty,
            search: SearchState::default(),
            make_targets: Vec::new(),
            make_selected: 0,
            mode: AppMode::default(),
            focused_panel: FocusedPanel::default(),
            status_message: None,
        }
    }

    // -----------------------------------------------------------------------
    // Navigation helpers
    // -----------------------------------------------------------------------

    /// Reset per-directory view state after any navigation.
    /// Called by enter_dir, go_back, and go_forward to avoid repetition.
    fn reset_view(&mut self) {
        self.selected_index = 0;
        self.list_scroll = 0;
        self.preview = PreviewState::Empty;
        self.search.clear();
    }

    /// Navigate into `dir`, pushing the current directory onto the history stack.
    pub fn enter_dir(&mut self, dir: PathBuf) {
        let old = std::mem::replace(&mut self.current_dir, dir);
        self.history.push(old);
        self.forward_stack.clear();
        self.reset_view();
    }

    /// Navigate to the parent of the current directory.
    /// Returns `true` if navigation occurred, `false` if already at root.
    pub fn go_up(&mut self) -> bool {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.enter_dir(parent);
            true
        } else {
            false
        }
    }

    /// Navigate backward in history.
    /// Returns `true` if navigation occurred.
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            let current = std::mem::replace(&mut self.current_dir, prev);
            self.forward_stack.push(current);
            self.reset_view();
            true
        } else {
            false
        }
    }

    /// Navigate forward (after a back navigation).
    /// Returns `true` if navigation occurred.
    pub fn go_forward(&mut self) -> bool {
        if let Some(next) = self.forward_stack.pop() {
            let current = std::mem::replace(&mut self.current_dir, next);
            self.history.push(current);
            self.reset_view();
            true
        } else {
            false
        }
    }

    // -----------------------------------------------------------------------
    // Entry / selection helpers
    // -----------------------------------------------------------------------

    /// Returns true if `entry` should be visible given the current `show_hidden` setting.
    /// Delegates to `fs::browser::is_hidden` to keep the hidden-file definition in one place.
    #[inline]
    fn entry_is_visible(&self, entry: &FileEntry) -> bool {
        self.show_hidden || !crate::fs::browser::is_hidden(&entry.path)
    }

    /// Returns the visible entries (references), filtered by the hidden-file setting.
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        self.entries
            .iter()
            .filter(|e| self.entry_is_visible(e))
            .collect()
    }

    /// Returns the number of visible entries without allocating a Vec.
    /// Use this instead of `visible_entries().len()` when only the count is needed.
    pub fn visible_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| self.entry_is_visible(e))
            .count()
    }

    /// Returns the currently selected `FileEntry`, if any.
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        let visible = self.visible_entries();
        visible.get(self.selected_index).copied()
    }

    /// Move the selection down by `n` items.
    pub fn move_down(&mut self, n: usize) {
        let max = self.visible_count().saturating_sub(1);
        self.selected_index = (self.selected_index + n).min(max);
    }

    /// Move the selection up by `n` items.
    pub fn move_up(&mut self, n: usize) {
        self.selected_index = self.selected_index.saturating_sub(n);
    }

    /// Jump to the first item.
    pub fn jump_to_top(&mut self) {
        self.selected_index = 0;
        self.list_scroll = 0;
    }

    /// Jump to the last item.
    pub fn jump_to_bottom(&mut self) {
        self.selected_index = self.visible_count().saturating_sub(1);
    }

    // -----------------------------------------------------------------------
    // UI helpers
    // -----------------------------------------------------------------------

    /// Toggle visibility of hidden (dot) files.
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        // Re-clamp selection to avoid out-of-bounds after the filter changes.
        let max = self.visible_count().saturating_sub(1);
        self.selected_index = self.selected_index.min(max);
    }

    /// Set a transient status-bar message.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear the status-bar message.
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Toggle help overlay via AppMode.
    pub fn toggle_help(&mut self) {
        self.mode = if self.mode == AppMode::Help {
            AppMode::Normal
        } else {
            AppMode::Help
        };
    }

    /// Returns true if the help overlay is currently shown.
    pub fn show_help(&self) -> bool {
        self.mode == AppMode::Help
    }

    /// Switch focus to the next panel in rotation: FileList → Preview → Sidebar → FileList.
    pub fn cycle_focus(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::FileList => FocusedPanel::Preview,
            FocusedPanel::Preview => FocusedPanel::Sidebar,
            FocusedPanel::Sidebar => FocusedPanel::FileList,
        };
    }

    /// Returns true if we are currently in search mode.
    pub fn is_searching(&self) -> bool {
        self.mode == AppMode::Search
    }

    /// Enter search mode.
    pub fn start_search(&mut self) {
        self.mode = AppMode::Search;
        self.search.clear();
    }

    /// Exit search mode, restoring normal browsing.
    pub fn stop_search(&mut self) {
        self.mode = AppMode::Normal;
        self.search.clear();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> AppState {
        AppState::new(PathBuf::from("/tmp"))
    }

    #[test]
    fn test_default_mode() {
        let state = make_state();
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(state.focused_panel, FocusedPanel::FileList);
    }

    #[test]
    fn test_enter_dir_pushes_history() {
        let mut state = make_state();
        state.enter_dir(PathBuf::from("/tmp/sub"));
        assert_eq!(state.current_dir, PathBuf::from("/tmp/sub"));
        assert_eq!(state.history, vec![PathBuf::from("/tmp")]);
        assert!(state.forward_stack.is_empty());
    }

    #[test]
    fn test_go_back_and_forward() {
        let mut state = make_state();
        state.enter_dir(PathBuf::from("/tmp/a"));
        state.enter_dir(PathBuf::from("/tmp/a/b"));

        // Go back once.
        assert!(state.go_back());
        assert_eq!(state.current_dir, PathBuf::from("/tmp/a"));
        assert_eq!(state.forward_stack, vec![PathBuf::from("/tmp/a/b")]);

        // Go forward.
        assert!(state.go_forward());
        assert_eq!(state.current_dir, PathBuf::from("/tmp/a/b"));
        assert!(state.forward_stack.is_empty());
    }

    #[test]
    fn test_go_back_at_root_returns_false() {
        let mut state = make_state();
        // No history yet.
        assert!(!state.go_back());
    }

    #[test]
    fn test_move_up_down_clamps() {
        let mut state = make_state();
        // Inject some fake entries.
        state.entries = vec![FileEntry::from_path(std::env::current_dir().unwrap()).unwrap()];
        state.move_down(100);
        assert_eq!(state.selected_index, 0); // only one entry

        state.move_up(100);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_toggle_hidden() {
        let mut state = make_state();
        assert!(!state.show_hidden);
        state.toggle_hidden();
        assert!(state.show_hidden);
        state.toggle_hidden();
        assert!(!state.show_hidden);
    }

    #[test]
    fn test_cycle_focus() {
        let mut state = make_state();
        assert_eq!(state.focused_panel, FocusedPanel::FileList);
        state.cycle_focus();
        assert_eq!(state.focused_panel, FocusedPanel::Preview);
        state.cycle_focus();
        assert_eq!(state.focused_panel, FocusedPanel::Sidebar);
        state.cycle_focus();
        assert_eq!(state.focused_panel, FocusedPanel::FileList);
    }

    #[test]
    fn test_search_mode() {
        let mut state = make_state();
        state.start_search();
        assert!(state.is_searching());
        assert_eq!(state.mode, AppMode::Search);

        state.stop_search();
        assert!(!state.is_searching());
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_status_message() {
        let mut state = make_state();
        assert!(state.status_message.is_none());
        state.set_status("hello");
        assert_eq!(state.status_message, Some("hello".to_string()));
        state.clear_status();
        assert!(state.status_message.is_none());
    }

    #[test]
    fn test_go_up() {
        let mut state = AppState::new(PathBuf::from("/tmp/a/b"));
        assert!(state.go_up());
        assert_eq!(state.current_dir, PathBuf::from("/tmp/a"));
    }

    #[test]
    fn test_go_up_at_root() {
        let mut state = AppState::new(PathBuf::from("/"));
        assert!(!state.go_up());
    }

    #[test]
    fn test_jump_to_top_bottom() {
        let mut state = make_state();
        // Manually inject entries using the current directory entry as a stand-in.
        let cwd = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(cwd).unwrap();
        state.entries = vec![entry.clone(), entry];
        state.selected_index = 1;

        state.jump_to_top();
        assert_eq!(state.selected_index, 0);

        state.jump_to_bottom();
        // visible_count respects show_hidden; both entries have no leading dot
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_toggle_help() {
        let mut state = make_state();
        assert!(!state.show_help());
        assert_eq!(state.mode, AppMode::Normal);

        state.toggle_help();
        assert!(state.show_help());
        assert_eq!(state.mode, AppMode::Help);

        state.toggle_help();
        assert!(!state.show_help());
        assert_eq!(state.mode, AppMode::Normal);
    }

    #[test]
    fn test_visible_count_no_alloc_path() {
        let mut state = make_state();
        let cwd = std::env::current_dir().unwrap();
        let entry = FileEntry::from_path(cwd).unwrap();
        state.entries = vec![entry.clone(), entry];
        // Both entries are the cwd (not hidden), so count should be 2.
        assert_eq!(state.visible_count(), 2);
        assert_eq!(state.visible_count(), state.visible_entries().len());
    }

    #[test]
    fn test_reset_view_on_navigation() {
        let mut state = make_state();
        state.selected_index = 5;
        state.list_scroll = 3;
        state.enter_dir(PathBuf::from("/tmp/x"));
        // reset_view should have zeroed these out.
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.list_scroll, 0);
    }
}
