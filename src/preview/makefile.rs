use std::path::Path;
use crate::app::state::PreviewState;

pub async fn load_makefile_preview(_path: &Path) -> PreviewState {
    PreviewState::Error("not implemented".to_string())
}
