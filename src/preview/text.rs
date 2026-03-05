use std::path::Path;
use anyhow::Result;

/// Reads the first `max_lines` lines of a text file for preview.
pub fn read_preview(path: &Path, max_lines: usize) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().take(max_lines).collect();
    Ok(lines.join("\n"))
}

/// Returns true if the file appears to be valid UTF-8 text.
pub fn is_text_file(path: &Path) -> bool {
    std::fs::read(path)
        .map(|bytes| std::str::from_utf8(&bytes).is_ok())
        .unwrap_or(false)
}
