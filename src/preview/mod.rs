pub mod text;
pub mod binary;
pub mod makefile;

use crate::app::state::{FileEntry, PreviewState};

pub async fn load_preview(_entry: &FileEntry) -> PreviewState {
    PreviewState::None
}
