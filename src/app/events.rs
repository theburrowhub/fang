use std::path::PathBuf;
use crate::app::state::{PreviewState, FileEntry};

#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Key(crossterm::event::KeyEvent),
    Resize(u16, u16),
    PreviewReady(PreviewState),
    MakeOutputLine(String),
    MakeDone { exit_code: i32 },
    DirectoryLoaded { path: PathBuf, entries: Vec<FileEntry> },
}
