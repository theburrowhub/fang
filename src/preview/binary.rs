use std::path::Path;
use anyhow::Result;
use crate::app::state::PreviewState;

/// Detect binary file and return basic info
pub async fn load(path: &Path) -> Result<PreviewState> {
    let metadata = tokio::fs::metadata(path).await?;
    let size = metadata.len();
    let mime_hint = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| format!("binary/{}", ext));

    Ok(PreviewState::Binary { size, mime_hint })
}
