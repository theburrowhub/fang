use std::path::Path;
use crate::app::state::PreviewState;

pub async fn load_binary_preview(_path: &Path) -> PreviewState {
    PreviewState::Error("not implemented".to_string())
}

pub fn is_binary_file(_path: &Path) -> bool {
    false
}
