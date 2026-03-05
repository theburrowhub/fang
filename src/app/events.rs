use std::path::PathBuf;
use crate::app::state::{PreviewState, FileEntry, HeaderInfo};

#[derive(Debug, Clone)]
pub enum Event {
    /// Periodic tick (250ms)
    Tick,
    /// Terminal key event
    Key(crossterm::event::KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Preview loading completed (from background task)
    PreviewReady(PreviewState),
    /// Line of output from make process
    MakeOutputLine(String),
    /// Make process completed
    MakeDone { exit_code: i32 },
    /// Directory loading completed (from background task)
    DirectoryLoaded { path: PathBuf, entries: Vec<FileEntry> },
    /// Header info (git branch + dev envs) loaded for current directory
    HeaderInfoReady(HeaderInfo),
    /// Shell command completed (output streamed via MakeOutputLine; this signals completion)
    CommandOutput { lines: Vec<String>, exit_code: i32 },
    /// Line of output from git process
    GitOutputLine(String),
    /// Git process completed
    GitDone { exit_code: i32 },
}
