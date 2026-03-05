use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct MakeTarget {
    pub name: String,
    pub description: Option<String>,
    pub line_number: usize,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub enum PreviewState {
    None,
    Text { content: String, path: PathBuf },
    Binary { path: PathBuf },
    Makefile { targets: Vec<MakeTarget>, path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub preview: PreviewState,
}

impl AppState {
    pub fn new(current_dir: PathBuf) -> Self {
        Self {
            current_dir,
            entries: Vec::new(),
            selected: 0,
            preview: PreviewState::None,
        }
    }
}
