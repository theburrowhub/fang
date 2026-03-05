use std::path::Path;
use anyhow::Result;
use crate::app::state::{PreviewState, StyledLine};

/// Maximum file size to preview (1 MB)
const MAX_PREVIEW_SIZE: u64 = 1024 * 1024;

/// Load a text file for preview, applying basic styling
pub async fn load(path: &Path) -> Result<PreviewState> {
    let metadata = tokio::fs::metadata(path).await?;
    let size = metadata.len();

    if size > MAX_PREVIEW_SIZE {
        return Ok(PreviewState::TooLarge { size });
    }

    let content = tokio::fs::read(path).await?;

    // Check if the file is likely binary
    if content.iter().take(8192).any(|&b| b == 0) {
        return Ok(PreviewState::Binary {
            size,
            mime_hint: infer_mime(path),
        });
    }

    let text = String::from_utf8_lossy(&content).to_string();
    let raw_lines: Vec<&str> = text.lines().collect();
    let total_lines = raw_lines.len();

    let lines: Vec<StyledLine> = raw_lines
        .iter()
        .map(|line| StyledLine::plain(*line))
        .collect();

    Ok(PreviewState::Text { lines, total_lines })
}

fn infer_mime(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| format!("application/{}", ext))
}
