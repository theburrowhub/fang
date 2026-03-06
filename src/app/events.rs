use crate::app::state::{FileEntry, HeaderInfo, PreviewState};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Event {
    /// Periodic tick (250ms)
    Tick,
    /// Terminal key event
    Key(crossterm::event::KeyEvent),
    /// Terminal resize
    #[allow(dead_code)]
    Resize(u16, u16),
    /// Preview loading completed (from background task)
    PreviewReady(PreviewState),
    /// Line of output from make process
    MakeOutputLine(String),
    /// Make process completed
    MakeDone { exit_code: i32 },
    /// Directory loading completed (from background task)
    DirectoryLoaded {
        path: PathBuf,
        entries: Vec<FileEntry>,
    },
    /// Header info (git branch + dev envs) loaded for current directory
    HeaderInfoReady(HeaderInfo),
    /// Shell command completed (output streamed via MakeOutputLine; this signals completion)
    CommandOutput { exit_code: i32 },
    /// Line of output from a git operation
    GitOutputLine(String),
    /// Git operation completed
    GitDone { exit_code: i32 },
    /// Streaming text fragment from an AI response
    AiOutputLine(String),
    /// AI response completed
    AiDone,
    /// AI providers detected (from background detection task)
    AiProvidersDetected(Vec<crate::commands::ai::AiProvider>),
}
