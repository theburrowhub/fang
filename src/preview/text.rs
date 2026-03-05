use std::path::Path;
use crate::app::state::PreviewState;

pub async fn load_text_preview(_path: &Path, _max_lines: usize) -> PreviewState {
    PreviewState::Error("not implemented".to_string())
}
