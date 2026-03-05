use std::path::Path;
use anyhow::Result;
use crate::app::state::PreviewState;

/// Loads a preview for the given path.
/// Returns a stub — full implementation with syntect highlighting is in a later unit.
pub async fn load_preview(_path: &Path) -> Result<PreviewState> {
    Ok(PreviewState::None)
}
